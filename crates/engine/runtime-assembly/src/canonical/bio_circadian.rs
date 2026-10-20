//! 生物机制族 W3.3 · 节律门装配面 (默认关, fail-closed)。
//!
//! # 机制
//!
//! 内核 [`apeireth_organ::circadian`] (W1/W3.1 已备, 本批**零改动**) 把昼夜相位 /
//! 超日窗口 / 睡眠压力合成一个 [`ProactiveGate`]:
//!
//! ```text
//! 显式输入 (CircadianGateState::tick(now_ms) 或 set_inputs)
//!   phase_at(now) + window_at(now, session_start_ms) + consolidation_urgency()
//!     └─ proactive_gate(phase, window, urgency) → ProactiveGate
//!          └─ should_suppress_proactive(...) → Option<ProactiveGate>
//!               ├─ None      = Allowed (放行)
//!               └─ Some(gate) = QuietHours / RestWindow / ConsolidationDue (拦下)
//! ```
//!
//! # 时区纪律: 必须显式配置, 不得推断
//!
//! [`CircadianGateState::new`] 的 [`CircadianConfig`] 必须由调用方给出含
//! `tz_offset_minutes` 的完整配置 (**没有** `from_env` / 系统时区推断 / 默认时区捷径);
//! 小时边界、超日时长、`session_start_ms` 同样显式。同一 epoch 在不同
//! `tz_offset_minutes` 下会得到不同相位 (测试钉死) —— 时区不是猜出来的。
//!
//! # 开关 (默认关)
//!
//! 装配门 = env [`BIO_CIRCADIAN_ENABLE_ENV`] (`APEIRETH_ENABLE_BIO_CIRCADIAN`,
//! **只认 `"1"`**)。关 = 不构造门 (懒加载); [`BioCircadianGate::new`] 的
//! `enabled = false` 是第二道门: [`BioCircadianGate::decide`] **不读时钟、不 tick**,
//! 返回 [`BioCircadianDecision::Disabled`]。
//!
//! # 心跳落点
//!
//! [`super::CareHeartbeatPoll::poll`] 的**最前**咨询本门: 开关开且门不许 → 直接
//! `Ok(HeartbeatPollOutcome::Quiet)` (不新增 outcome 类型); 其余候选 (规则 / 提醒 /
//! 生物候选) 一拍都不读。「未到点」与「被节律门拦下」都是 `Quiet`, 区分靠
//! [`BioCircadianGate::last_decision`] 观测字段 (`Allowed` vs `Suppressed(_)`)。
//!
//! # 0 装诚实 (能力级别: IMPLEMENTED + 默认关; 非 PRODUCTION WIRED)
//!
//! - **真实本地时区未接**: 时区偏移必须显式配置; 不读系统时区 / 环境变量 / 用户设置。
//! - **真实睡眠压力源未接**: [`SleepPressure`] 初始为 0, 只有显式
//!   [`CircadianGateState::set_pressure`] / `accumulate_pressure` / `relieve_pressure`
//!   才会变化; 不臆造「用户很累」。
//! - **无持久化 / 无 live E2E**: 相位按注入时刻即时计算, 不落盘; `production.rs`
//!   尚未注册 (注册建议由 Lead 后续做)。

#![forbid(unsafe_code)]

use std::sync::Arc;

use apeireth_organ::circadian::{
    proactive_gate, CircadianConfig, CircadianPhase, CircadianState, ProactiveGate, SleepPressure,
    SleepUrgency, UltradianWindow,
};

/// 装配开闸 env 名 (仅显式 `"1"` 开, 其余一律关 fail-closed)。
pub const BIO_CIRCADIAN_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_CIRCADIAN";

/// 节律门时钟: 返回 epoch 毫秒。时间显式注入, 门自身不读系统时钟。
pub type CircadianGateClock = Arc<dyn Fn() -> i64 + Send + Sync>;

/// 开关解析 (纯函数): 仅显式 `"1"` 开; 未设置 / 空 / `0` / `true` / `yes` / `on` /
/// 任意其它值一律关 (默认关 fail-closed)。`trim` 后比较, 因此 `" 1 "` 算开。
#[must_use]
pub fn bio_circadian_enabled_from_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 门输入快照 (显式 `tick` 计算结果或 `set_inputs` 直接注入)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircadianGateInputs {
    /// 快照时刻 (epoch 毫秒; `set_inputs` 时由调用方给真值)。
    pub at_ms: i64,
    /// 昼夜相位。
    pub phase: CircadianPhase,
    /// 超日窗口快照。
    pub window: UltradianWindow,
    /// 睡眠压力整编紧迫度。
    pub urgency: SleepUrgency,
}

/// 节律门状态: 显式持有配置 + 会话起点 + 睡眠压力 + 最近输入快照。
///
/// 纯确定性: 只在 [`Self::tick`] 内按注入时刻计算, 不读系统时钟 / 不做 IO。
pub struct CircadianGateState {
    circadian: CircadianState,
    session_start_ms: i64,
    pressure: SleepPressure,
    inputs: CircadianGateInputs,
}

impl CircadianGateState {
    /// 显式配置构造: `config` 必须携带时区偏移 (不推断), `session_start_ms` 是超日
    /// 窗口起点, `now_ms` 是首帧快照时刻 (之后用 [`Self::tick`] 推进)。
    #[must_use]
    pub fn new(config: CircadianConfig, session_start_ms: i64, now_ms: i64) -> Self {
        let circadian = CircadianState::new(config);
        let pressure = SleepPressure::new();
        let inputs = Self::compute(&circadian, session_start_ms, &pressure, now_ms);
        Self {
            circadian,
            session_start_ms,
            pressure,
            inputs,
        }
    }

    /// 注入时刻 → 相位 / 超日窗口 / 紧迫度, 并更新快照。
    pub fn tick(&mut self, now_ms: i64) -> CircadianGateInputs {
        self.inputs = Self::compute(
            &self.circadian,
            self.session_start_ms,
            &self.pressure,
            now_ms,
        );
        self.inputs
    }

    /// 直接注入输入快照 (宿主已有相位/窗口/压力真值时用; 不读时钟)。
    ///
    /// 下一次 [`Self::tick`] 会用配置与压力重新计算并覆盖本快照。
    pub fn set_inputs(&mut self, inputs: CircadianGateInputs) {
        self.inputs = inputs;
    }

    /// 最近一次输入快照。
    #[must_use]
    pub fn inputs(&self) -> CircadianGateInputs {
        self.inputs
    }

    /// 当前节律门 (纯函数合成, 无副作用)。
    #[must_use]
    pub fn gate(&self) -> ProactiveGate {
        proactive_gate(self.inputs.phase, self.inputs.window, self.inputs.urgency)
    }

    /// 当前是否应抑制主动接触: `Some(gate)` = 拦下, `None` = 放行。
    #[must_use]
    pub fn should_suppress(&self) -> Option<ProactiveGate> {
        should_suppress_proactive(self)
    }

    /// 当前睡眠压力 (拷贝)。
    #[must_use]
    pub fn pressure(&self) -> SleepPressure {
        self.pressure
    }

    /// 显式注入睡眠压力 (真实压力源未接; 不猜)。
    pub fn set_pressure(&mut self, pressure: SleepPressure) {
        self.pressure = pressure;
    }

    /// 显式累积睡眠压力 (负荷 `0..=1`, 时长毫秒; 内核语义)。
    pub fn accumulate_pressure(&mut self, load: f64, elapsed_ms: u64) {
        self.pressure.accumulate(load, elapsed_ms);
    }

    /// 显式清账睡眠压力 (休息时长毫秒; 内核语义)。
    pub fn relieve_pressure(&mut self, rest_ms: u64) {
        self.pressure.relieve(rest_ms);
    }

    /// 显式配置 (含时区偏移)。
    #[must_use]
    pub fn config(&self) -> CircadianConfig {
        self.circadian.config()
    }

    /// 超日窗口起点 (epoch 毫秒)。
    #[must_use]
    pub fn session_start_ms(&self) -> i64 {
        self.session_start_ms
    }

    fn compute(
        circadian: &CircadianState,
        session_start_ms: i64,
        pressure: &SleepPressure,
        now_ms: i64,
    ) -> CircadianGateInputs {
        CircadianGateInputs {
            at_ms: now_ms,
            phase: circadian.phase_at(now_ms),
            window: circadian.window_at(now_ms, session_start_ms),
            urgency: pressure.consolidation_urgency(),
        }
    }
}

impl std::fmt::Debug for CircadianGateState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CircadianGateState")
            .field("config", &self.config())
            .field("session_start_ms", &self.session_start_ms)
            .field("pressure", &self.pressure)
            .field("inputs", &self.inputs)
            .finish()
    }
}

/// 节律门纯函数: 门不许 = `Some(ProactiveGate)`, 放行 = `None`。
///
/// 返回值具体是哪一档 (`QuietHours` / `RestWindow` / `ConsolidationDue`) 由内核
/// [`proactive_gate`] 决定; 本函数不重新实现优先级。
#[must_use]
pub fn should_suppress_proactive(gate_state: &CircadianGateState) -> Option<ProactiveGate> {
    let gate = gate_state.gate();
    if gate.allows_proactive() {
        None
    } else {
        Some(gate)
    }
}

/// 开关值 + 时钟 + 显式配置 → 节律门 (Lead 注册入口; 开关关 = `None` 懒加载)。
///
/// 等价于 [`BioCircadianGate::from_env_value`]; 提供自由函数便于组装根按统一
/// `..._wiring_from_env` 命名调用。
#[must_use]
pub fn bio_circadian_wiring_from_env_value(
    raw: Option<&str>,
    clock: CircadianGateClock,
    config: CircadianConfig,
    session_start_ms: i64,
) -> Option<BioCircadianGate> {
    BioCircadianGate::from_env_value(raw, clock, config, session_start_ms)
}

/// 从 env 现值装配节律门 (同 [`bio_circadian_wiring_from_env_value`] 的 fail-closed 口径)。
#[must_use]
pub fn bio_circadian_wiring_from_env(
    clock: CircadianGateClock,
    config: CircadianConfig,
    session_start_ms: i64,
) -> Option<BioCircadianGate> {
    BioCircadianGate::from_env(clock, config, session_start_ms)
}

/// 一次心跳节律门决策 (观测字段; 区分「未到点」与「被门拦下」)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BioCircadianDecision {
    /// 门关: 未咨询, 零行为变化。
    Disabled,
    /// 门开且放行 (`Allowed`)。
    Allowed,
    /// 门开且拦下 (具体档位见内层 [`ProactiveGate`])。
    Suppressed(ProactiveGate),
}

impl BioCircadianDecision {
    /// 本次是否拦下主动接触。
    #[must_use]
    pub const fn is_suppressed(self) -> bool {
        matches!(self, Self::Suppressed(_))
    }

    /// 拦下时的具体档位; 其余情况 `None`。
    #[must_use]
    pub const fn gate(self) -> Option<ProactiveGate> {
        match self {
            Self::Suppressed(gate) => Some(gate),
            Self::Disabled | Self::Allowed => None,
        }
    }
}

/// 节律门装配句柄: 开关 + 显式时钟 + 门状态 + 决策观测。
///
/// [`Self::decide`] 是心跳每拍唯一的咨询入口: 关 → `Disabled` (不读时钟);
/// 开 → `tick(now)` 后用 [`should_suppress_proactive`] 裁决并留痕 `last_decision`。
pub struct BioCircadianGate {
    enabled: bool,
    clock: CircadianGateClock,
    state: CircadianGateState,
    last_decision: Option<BioCircadianDecision>,
    ticks: u64,
}

impl BioCircadianGate {
    /// 按显式开关 + 时钟 + 门状态构造 (测试 / 宿主自带状态时用)。
    #[must_use]
    pub fn new(enabled: bool, clock: CircadianGateClock, state: CircadianGateState) -> Self {
        Self {
            enabled,
            clock,
            state,
            last_decision: None,
            ticks: 0,
        }
    }

    /// 开关值 + 时钟 + 显式配置 → 门; 关 = `None` (懒加载, 不构造)。
    ///
    /// `config` 必须携带时区偏移 (不推断); 首帧快照时刻取一次注入时钟。
    #[must_use]
    pub fn from_env_value(
        raw: Option<&str>,
        clock: CircadianGateClock,
        config: CircadianConfig,
        session_start_ms: i64,
    ) -> Option<Self> {
        if !bio_circadian_enabled_from_env_value(raw) {
            return None;
        }
        let now_ms = (clock)();
        let state = CircadianGateState::new(config, session_start_ms, now_ms);
        Some(Self::new(true, clock, state))
    }

    /// 从 env 现值装配 (同 [`Self::from_env_value`] 的 fail-closed 口径)。
    #[must_use]
    pub fn from_env(
        clock: CircadianGateClock,
        config: CircadianConfig,
        session_start_ms: i64,
    ) -> Option<Self> {
        Self::from_env_value(
            std::env::var(BIO_CIRCADIAN_ENABLE_ENV).ok().as_deref(),
            clock,
            config,
            session_start_ms,
        )
    }

    /// 开关现况。
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 门状态 (只读)。
    #[must_use]
    pub fn state(&self) -> &CircadianGateState {
        &self.state
    }

    /// 门状态 (可变; 宿主注入睡眠压力等显式输入)。
    pub fn state_mut(&mut self) -> &mut CircadianGateState {
        &mut self.state
    }

    /// 显式注入睡眠压力 (便捷口, 等价 `state_mut().set_pressure`)。
    pub fn set_pressure(&mut self, pressure: SleepPressure) {
        self.state.set_pressure(pressure);
    }

    /// 最近一次决策 (`None` = 尚未咨询); 心跳 `Quiet` 时用它区分未到点 / 被门拦下。
    #[must_use]
    pub fn last_decision(&self) -> Option<BioCircadianDecision> {
        self.last_decision
    }

    /// 已咨询次数 (开关关不计; 诊断/测试真值)。
    #[must_use]
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// 心跳每拍咨询一次: 关 → `Disabled` (零时钟读取); 开 → tick + 裁决 + 留痕。
    pub fn decide(&mut self) -> BioCircadianDecision {
        if !self.enabled {
            let decision = BioCircadianDecision::Disabled;
            self.last_decision = Some(decision);
            return decision;
        }
        let now_ms = (self.clock)();
        self.state.tick(now_ms);
        self.ticks += 1;
        let decision = match should_suppress_proactive(&self.state) {
            Some(gate) => BioCircadianDecision::Suppressed(gate),
            None => BioCircadianDecision::Allowed,
        };
        self.last_decision = Some(decision);
        decision
    }
}

impl std::fmt::Debug for BioCircadianGate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BioCircadianGate")
            .field("enabled", &self.enabled)
            .field("state", &self.state)
            .field("last_decision", &self.last_decision)
            .field("ticks", &self.ticks)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use apeireth_organ::circadian::{
        CircadianPhase, SleepUrgency, UltradianPhase, UltradianWindow,
    };

    use super::*;

    const TZ_BEIJING: i32 = 8 * 60;

    const FOCUS_WINDOW: UltradianWindow = UltradianWindow {
        phase: UltradianPhase::Focus,
        remaining_ms: 1,
    };
    const REST_WINDOW: UltradianWindow = UltradianWindow {
        phase: UltradianPhase::Rest,
        remaining_ms: 1,
    };

    fn config(tz_offset_minutes: i32) -> CircadianConfig {
        CircadianConfig::new(7, 23, tz_offset_minutes, 25 * 60_000, 5 * 60_000)
    }

    fn frozen_clock(now_ms: i64) -> CircadianGateClock {
        Arc::new(move || now_ms)
    }

    /// 本地 `hour:minute` (按给定时区) 对应的 epoch 毫秒。
    fn local_ms(hour: u8, minute: u8, tz_offset_minutes: i32) -> i64 {
        (i64::from(hour) * 60 + i64::from(minute) - i64::from(tz_offset_minutes)) * 60_000
    }

    #[test]
    fn env_switch_only_accepts_explicit_one_and_is_lazy() {
        assert_eq!(BIO_CIRCADIAN_ENABLE_ENV, "APEIRETH_ENABLE_BIO_CIRCADIAN");
        assert!(bio_circadian_enabled_from_env_value(Some("1")));
        assert!(bio_circadian_enabled_from_env_value(Some(" 1 ")));
        for raw in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("TRUE"),
            Some("yes"),
            Some("on"),
            Some("2"),
            Some("01"),
        ] {
            assert!(
                !bio_circadian_enabled_from_env_value(raw),
                "非 \"1\" 一律关 (fail-closed): {raw:?}"
            );
            assert!(
                BioCircadianGate::from_env_value(raw, frozen_clock(0), config(TZ_BEIJING), 0)
                    .is_none(),
                "关 = 不构造: {raw:?}"
            );
        }
        let gate =
            BioCircadianGate::from_env_value(Some("1"), frozen_clock(0), config(TZ_BEIJING), 0)
                .expect("开");
        assert!(gate.enabled());
        assert_eq!(gate.ticks(), 0);
    }

    #[test]
    fn should_suppress_covers_all_four_gate_states() {
        let mut state = CircadianGateState::new(config(TZ_BEIJING), 0, 0);
        let cases = [
            (CircadianPhase::Day, FOCUS_WINDOW, SleepUrgency::Low, None),
            (
                CircadianPhase::DeepNight,
                FOCUS_WINDOW,
                SleepUrgency::Low,
                Some(ProactiveGate::QuietHours),
            ),
            (
                CircadianPhase::Dawn,
                REST_WINDOW,
                SleepUrgency::High,
                Some(ProactiveGate::QuietHours),
            ),
            (
                CircadianPhase::Day,
                REST_WINDOW,
                SleepUrgency::Normal,
                Some(ProactiveGate::RestWindow),
            ),
            (
                CircadianPhase::Night,
                FOCUS_WINDOW,
                SleepUrgency::High,
                Some(ProactiveGate::ConsolidationDue),
            ),
        ];
        for (phase, window, urgency, expected) in cases {
            state.set_inputs(CircadianGateInputs {
                at_ms: 123,
                phase,
                window,
                urgency,
            });
            assert_eq!(state.inputs().at_ms, 123, "set_inputs 原样保留注入时刻");
            assert_eq!(
                should_suppress_proactive(&state),
                expected,
                "{phase:?}/{window:?}/{urgency:?}"
            );
            assert_eq!(state.gate(), expected.unwrap_or(ProactiveGate::Allowed));
        }
        assert_eq!(state.inputs().at_ms, 123);
    }

    #[test]
    fn tick_uses_the_explicitly_configured_timezone_and_never_infers_one() {
        // UTC 00:30 = 北京 08:30 (晨) / UTC 00:30 (深夜)。
        let now_ms = 30 * 60_000;
        let mut beijing = CircadianGateState::new(config(TZ_BEIJING), now_ms, now_ms);
        let mut utc = CircadianGateState::new(config(0), now_ms, now_ms);
        assert_eq!(beijing.inputs().phase, CircadianPhase::Morning);
        assert_eq!(utc.inputs().phase, CircadianPhase::DeepNight);
        assert_eq!(beijing.inputs().at_ms, now_ms);

        // 同一时刻推进: 相位随时间变化 (不是静态编造)。
        beijing.tick(local_ms(1, 0, TZ_BEIJING));
        assert_eq!(beijing.inputs().phase, CircadianPhase::DeepNight);
        assert_eq!(
            should_suppress_proactive(&beijing),
            Some(ProactiveGate::QuietHours)
        );
        utc.tick(local_ms(12, 0, 0));
        assert_eq!(utc.inputs().phase, CircadianPhase::Day);
    }

    #[test]
    fn tick_computes_the_ultradian_window_from_the_explicit_session_start() {
        let now_ms = local_ms(16, 53, TZ_BEIJING);
        let session_start_ms = now_ms - 10 * 60_000;
        let mut state = CircadianGateState::new(config(TZ_BEIJING), session_start_ms, now_ms);
        assert_eq!(state.inputs().window.phase, UltradianPhase::Focus);
        assert_eq!(
            state.inputs().window.remaining_ms,
            15 * 60_000,
            "25 分钟专注窗已过 10 分钟"
        );

        // 26 分钟经过 → 休息窗 (25 分钟专注 + 5 分钟休息)。
        state.tick(now_ms + 16 * 60_000);
        assert_eq!(state.inputs().window.phase, UltradianPhase::Rest);
        assert_eq!(
            should_suppress_proactive(&state),
            Some(ProactiveGate::RestWindow)
        );
    }

    #[test]
    fn pressure_is_explicit_and_drives_consolidation_due() {
        let now_ms = local_ms(16, 53, TZ_BEIJING);
        let mut state = CircadianGateState::new(config(TZ_BEIJING), now_ms - 60_000, now_ms);
        assert_eq!(state.pressure().level(), 0.0);
        assert_eq!(
            should_suppress_proactive(&state),
            None,
            "零压力 + 专注窗 = 放行"
        );

        state.set_pressure(SleepPressure::with_level(0.8));
        assert_eq!(state.inputs().urgency, SleepUrgency::Low, "tick 前快照未变");
        state.tick(now_ms);
        assert_eq!(state.inputs().urgency, SleepUrgency::High);
        assert_eq!(
            should_suppress_proactive(&state),
            Some(ProactiveGate::ConsolidationDue)
        );

        // 休息清账后回到放行。
        state.relieve_pressure(8 * 60 * 60 * 1000);
        assert_eq!(state.pressure().level(), 0.0);
        state.tick(now_ms);
        assert_eq!(should_suppress_proactive(&state), None);
    }

    #[test]
    fn a_disabled_gate_never_reads_the_clock_or_ticks() {
        let reads = Arc::new(AtomicU64::new(0));
        let observed = Arc::clone(&reads);
        let clock: CircadianGateClock = Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            0
        });
        let state = CircadianGateState::new(config(TZ_BEIJING), 0, 0);
        let mut gate = BioCircadianGate::new(false, clock, state);
        assert_eq!(gate.decide(), BioCircadianDecision::Disabled);
        assert_eq!(gate.decide(), BioCircadianDecision::Disabled);
        assert_eq!(reads.load(Ordering::SeqCst), 0, "门关: 零时钟读取");
        assert_eq!(gate.ticks(), 0, "门关: 零 tick");
        assert_eq!(gate.last_decision(), Some(BioCircadianDecision::Disabled));
    }

    #[test]
    fn an_enabled_gate_ticks_once_per_decision_and_records_the_suppression() {
        // 北京 01:00 = 深夜。
        let now_ms = local_ms(1, 0, TZ_BEIJING);
        let state = CircadianGateState::new(config(TZ_BEIJING), now_ms, now_ms);
        let mut gate = BioCircadianGate::new(true, frozen_clock(now_ms), state);
        let decision = gate.decide();
        assert_eq!(
            decision,
            BioCircadianDecision::Suppressed(ProactiveGate::QuietHours)
        );
        assert!(decision.is_suppressed());
        assert_eq!(decision.gate(), Some(ProactiveGate::QuietHours));
        assert_eq!(gate.ticks(), 1);
        assert_eq!(gate.last_decision(), Some(decision));

        gate.set_pressure(SleepPressure::with_level(0.0));
        assert_eq!(
            gate.state().inputs().phase,
            CircadianPhase::DeepNight,
            "状态快照可观测"
        );

        // 同日 16:53: 专注窗 + 零压力 = 放行。
        let day_ms = local_ms(16, 53, TZ_BEIJING);
        let state = CircadianGateState::new(config(TZ_BEIJING), day_ms - 60_000, day_ms);
        let mut allowed = BioCircadianGate::new(true, frozen_clock(day_ms), state);
        assert_eq!(allowed.decide(), BioCircadianDecision::Allowed);
        assert!(!allowed.decide().is_suppressed());
        assert_eq!(allowed.last_decision(), Some(BioCircadianDecision::Allowed));
        assert_eq!(allowed.ticks(), 2);
    }

    #[test]
    fn decisions_are_deterministic_for_the_same_injected_inputs() {
        let now_ms = local_ms(16, 53, TZ_BEIJING);
        let make = || {
            let state = CircadianGateState::new(config(TZ_BEIJING), now_ms - 60_000, now_ms);
            BioCircadianGate::new(true, frozen_clock(now_ms), state)
        };
        let mut a = make();
        let mut b = make();
        assert_eq!(format!("{:?}", a.decide()), format!("{:?}", b.decide()));
        assert_eq!(a.ticks(), b.ticks());
    }
}
