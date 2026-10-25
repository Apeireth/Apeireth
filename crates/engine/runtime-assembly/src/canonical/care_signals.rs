//! 关怀势能场的**信号源**（D-3，2026-10-25）：把「连续工具失败」「距上次工具成功」
//! 映射成 [`CarePotentialField::step`] 需要的 distress / fatigue / silence 因子。
//!
//! # 为什么需要这一层
//!
//! `CarePotentialField`（`apeireth_orchestration::care_potential_field`）有完整实现、
//! 有势能公式、有分级关怀动作，但**没有任何生产调用** —— 缺的不是算法，是**输入**：
//! nocturnal / distress / fatigue / silence / flow_resistance 五个因子没人算。
//!
//! 本模块按 owner 裁定（D-3）只引入**确定性、可测、不猜语义**的最小信号集：
//!
//! | 因子 | 信号 | 依据 |
//! |---|---|---|
//! | distress  | **连续**工具失败数（成功即清零） | 连续失败 = 用户在反复撞墙 |
//! | fatigue   | 距最近一次工具**成功**的时长 | 长时间无成功 = 持续消耗 |
//! | silence   | 同上（归一化到小时） | 用户没在交互 |
//! | nocturnal | **固定 0.0** | 涉及时区配置（W3.4 时区债未收），owner 裁定后置 |
//! | flow_resistance | **固定 0.0** | 需要心流检测面，本批不做 |
//!
//! **信号来源真实**：[`BioLifecycleState::consecutive_tool_failures`] 与
//! [`BioLifecycleState::last_tool_success_ms`] 由真实 `ToolOutcome` 总线回调维护
//! （`bio_lifecycle` 模块订阅），不是采样、不是估算、不编造。
//!
//! # 默认关
//!
//! 驱动器只在既有关怀总闸下工作（`APEIRETH_ENABLE_BIO_CARE_CANDIDATES`，与生物候选同门）
//! —— **不新增 env**（env 预算已满 160）。闸关时 `poll` 恒 `None`，且连信号都不读。

use std::sync::Arc;

use apeireth_orchestration::{CareAction, CarePotentialField};

use super::care_heartbeat::bio_lifecycle::BioLifecycleState;

/// distress 达到满值所需的连续失败数（3 次连续失败 = 满值）。
///
/// 取值依据：一次失败是常态（工具/网络抖动）；两次仍可能是偶发；三次连续
/// 才足以说明「用户在反复撞同一面墙」。**不随失败类型加权**（不猜严重度）。
pub const DISTRESS_FULL_FAILURES: u32 = 3;

/// fatigue 达到满值所需的「无成功」小时数（6 小时 = 满值）。
///
/// 取值依据：一次专注工作会话的上限量级；超过 6 小时无任何工具成功，
/// 说明用户要么在长时间阅读/思考，要么被困住了 —— 两种情况都值得克制关怀。
pub const FATIGUE_FULL_HOURS: f64 = 6.0;

/// 一次信号快照（纯数据，便于测试与复算）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CareSignals {
    /// `now` 时刻（epoch 毫秒）。
    pub now_ms: u64,
    /// **连续**工具失败数（真实总线回调维护）。
    pub consecutive_tool_failures: u32,
    /// 最近一次工具成功时刻；`None` = 尚无成功。
    pub last_success_ms: Option<u64>,
}

impl CareSignals {
    /// 从生命周期状态 + 当前时刻取一次快照（只读）。
    #[must_use]
    pub fn from_state(state: &BioLifecycleState, now_ms: u64) -> Self {
        Self {
            now_ms,
            consecutive_tool_failures: u32::try_from(state.consecutive_tool_failures())
                .unwrap_or(u32::MAX),
            last_success_ms: state.last_tool_success_ms(),
        }
    }

    /// distress 因子 `[0.0, 1.0]`：连续失败数 / [`DISTRESS_FULL_FAILURES`]。
    #[must_use]
    pub fn distress_factor(&self) -> f64 {
        (f64::from(self.consecutive_tool_failures) / f64::from(DISTRESS_FULL_FAILURES))
            .clamp(0.0, 1.0)
    }

    /// 距最近一次工具成功的秒数；尚无成功 = 0（不编造「一直很累」）。
    #[must_use]
    pub fn seconds_since_last_success(&self) -> f64 {
        let Some(last) = self.last_success_ms else {
            return 0.0;
        };
        // 时钟回拨（now < last）按 0 处理，不产生负因子。
        (self.now_ms.saturating_sub(last)) as f64 / 1000.0
    }

    /// fatigue 因子 `[0.0, 1.0]`：无成功小时数 / [`FATIGUE_FULL_HOURS`]。
    #[must_use]
    pub fn fatigue_factor(&self) -> f64 {
        ((self.seconds_since_last_success() / 3600.0) / FATIGUE_FULL_HOURS).clamp(0.0, 1.0)
    }
}

/// 关怀势能驱动器：持势能场 + 真实信号源，按拍推进并在突破时给出关怀动作。
///
/// - 闸关（`enabled == false`）⇒ [`poll`](Self::poll) 恒 `None`，**不读信号**
///   （与 care 家族「默认关 = 零读取」同一纪律）；
/// - 闸开 ⇒ 每拍 `step()` 一次，突破阈值时返回分级动作并记录；
/// - 记录只保留**最近一次**动作（有界，不建历史、不落盘 —— 持久化由既有
///   care 账本负责，本驱动器不另开账）。
#[derive(Debug)]
pub struct CarePotentialDriver {
    field: Arc<CarePotentialField>,
    enabled: bool,
    last_action: std::sync::Mutex<Option<CareAction>>,
    last_action_at_ms: std::sync::atomic::AtomicU64,
    ticks: std::sync::atomic::AtomicU64,
}

impl CarePotentialDriver {
    /// 构造驱动器。`enabled` = 既有关怀总闸的生效值（不新增 env）。
    #[must_use]
    pub fn new(threshold: f64, enabled: bool) -> Self {
        Self {
            field: Arc::new(CarePotentialField::new(threshold)),
            enabled,
            last_action: std::sync::Mutex::new(None),
            last_action_at_ms: std::sync::atomic::AtomicU64::new(0),
            ticks: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 开关现况。
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 当前势能值（诊断/自述用；闸关时恒 0）。
    #[must_use]
    pub fn current_potential(&self) -> f64 {
        self.field.current_potential()
    }

    /// 累计推进拍数（诊断用）。
    #[must_use]
    pub fn ticks(&self) -> u64 {
        self.ticks.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 最近一次突破产生的关怀动作（读取后仍保留，供自述/复述）。
    #[must_use]
    pub fn last_action(&self) -> Option<CareAction> {
        self.last_action
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// 最近一次关怀动作的时刻（epoch 毫秒）；`None` = 尚无。
    #[must_use]
    pub fn last_action_at_ms(&self) -> Option<u64> {
        match self
            .last_action_at_ms
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            0 => None,
            at => Some(at),
        }
    }

    /// 推进一拍：从真实状态取信号 → `step()` → 返回突破动作（若有）。
    ///
    /// `dt_seconds` = 两拍之间的真实间隔（调用方给，不猜）；非正数按 0 处理。
    pub fn poll(
        &self,
        state: &BioLifecycleState,
        now_ms: u64,
        dt_seconds: f64,
    ) -> Option<CareAction> {
        if !self.enabled {
            return None;
        }
        let signals = CareSignals::from_state(state, now_ms);
        self.ticks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let action = self.field.step(
            0.0, // nocturnal: 时区债未收，owner 裁定后置（D-3 ③）
            signals.distress_factor(),
            signals.fatigue_factor(),
            signals.seconds_since_last_success(),
            0.0, // flow_resistance: 需心流检测面，本批不做
            dt_seconds.max(0.0),
        );
        if let Some(action) = &action {
            if let Ok(mut guard) = self.last_action.lock() {
                *guard = Some(action.clone());
            }
            self.last_action_at_ms
                .store(now_ms, std::sync::atomic::Ordering::SeqCst);
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_orchestration::lifecycle::LifecyclePolicy;

    use crate::canonical::care_heartbeat::bio_lifecycle::LifecycleClock;

    fn state(now_ms: u64) -> BioLifecycleState {
        let clock_now = std::sync::Arc::new(std::sync::Mutex::new(now_ms));
        let clock: LifecycleClock = {
            let c = clock_now.clone();
            std::sync::Arc::new(move || *c.lock().expect("clock"))
        };
        BioLifecycleState::with_policy(LifecyclePolicy::default(), 8, clock)
    }

    #[test]
    fn distress_is_zero_without_failures_and_saturates_at_three() {
        let mut s = CareSignals {
            now_ms: 1_000,
            consecutive_tool_failures: 0,
            last_success_ms: None,
        };
        assert_eq!(s.distress_factor(), 0.0);
        s.consecutive_tool_failures = 1;
        assert!((s.distress_factor() - 1.0 / 3.0).abs() < 1e-12);
        s.consecutive_tool_failures = 3;
        assert_eq!(s.distress_factor(), 1.0);
        s.consecutive_tool_failures = 99;
        assert_eq!(s.distress_factor(), 1.0, "因子按文档口径钳到 [0,1]");
    }

    #[test]
    fn fatigue_uses_real_seconds_since_success_and_ignores_clock_rollback() {
        let s = CareSignals {
            now_ms: 10_000,
            consecutive_tool_failures: 0,
            last_success_ms: Some(4_000),
        };
        assert_eq!(s.seconds_since_last_success(), 6.0);
        assert!((s.fatigue_factor() - (6.0 / 3600.0) / FATIGUE_FULL_HOURS).abs() < 1e-12);
        // 时钟回拨：now < last ⇒ 0，不产负值
        let back = CareSignals {
            now_ms: 1_000,
            consecutive_tool_failures: 0,
            last_success_ms: Some(9_000),
        };
        assert_eq!(back.seconds_since_last_success(), 0.0);
        assert_eq!(back.fatigue_factor(), 0.0);
    }

    #[test]
    fn no_success_yet_means_zero_fatigue_not_infinite() {
        let s = CareSignals {
            now_ms: 10_000_000,
            consecutive_tool_failures: 0,
            last_success_ms: None,
        };
        assert_eq!(s.seconds_since_last_success(), 0.0);
        assert_eq!(s.fatigue_factor(), 0.0);
    }

    #[test]
    fn disabled_driver_never_reads_signals_or_steps() {
        let st = state(1_000);
        st.observe_tool_outcome_at("tool.a", false, 1_000)
            .expect("record");
        let driver = CarePotentialDriver::new(1.0, false);
        for i in 0..100 {
            assert!(driver.poll(&st, 1_000 + i, 300.0).is_none());
        }
        assert_eq!(driver.ticks(), 0);
        assert_eq!(driver.current_potential(), 0.0);
        assert!(driver.last_action().is_none());
    }

    #[test]
    fn consecutive_failures_raise_potential_and_emit_one_action() {
        let st = state(1_000);
        for i in 0..3 {
            st.observe_tool_outcome_at("tool.failing", false, 1_000 + i * 10)
                .expect("record failure");
        }
        assert_eq!(st.consecutive_tool_failures(), 3);
        assert_eq!(st.last_tool_success_ms(), None);
        let driver = CarePotentialDriver::new(1.0, true);
        // dt=2s：gain≈0.8 ⇒ Δu≈1.6 ≥ 阈值 1.0，突破后泄放到 30%（0.48）可断言
        let action = driver.poll(&st, 2_000, 2.0);
        assert!(action.is_some(), "3 次连续失败应突破阈值");
        assert_eq!(driver.ticks(), 1);
        assert!(driver.current_potential() < 1.0, "突破后应泄放势能");
        assert!(driver.last_action().is_some());
        assert_eq!(driver.last_action_at_ms(), Some(2_000));
    }

    #[test]
    fn success_resets_consecutive_failures_and_records_last_success() {
        let st = state(1_000);
        st.observe_tool_outcome_at("tool.a", false, 1_000)
            .expect("fail");
        st.observe_tool_outcome_at("tool.a", false, 2_000)
            .expect("fail");
        assert_eq!(st.consecutive_tool_failures(), 2);
        st.observe_tool_outcome_at("tool.a", true, 3_000)
            .expect("ok");
        assert_eq!(st.consecutive_tool_failures(), 0);
        assert_eq!(st.last_tool_success_ms(), Some(3_000));
        let s = CareSignals::from_state(&st, 10_000);
        assert_eq!(s.distress_factor(), 0.0);
        assert_eq!(s.seconds_since_last_success(), 7.0);
    }

    #[test]
    fn no_action_below_threshold() {
        let st = state(1_000);
        st.observe_tool_outcome_at("tool.a", false, 1_000)
            .expect("fail");
        let driver = CarePotentialDriver::new(50.0, true);
        assert!(driver.poll(&st, 2_000, 1.0).is_none());
        assert!(driver.last_action().is_none());
    }
}

// ============================ 提醒信号桥 (ReminderFeed) ============================

use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

use apeireth_organ::care::{CareTier, ReminderCandidate, ReminderFeed};

/// 关怀势能场 → **提醒信号源**桥（D-3 生产接线）。
///
/// # 为什么走提醒信号面而不是直接发消息
///
/// 关怀纪律四件（勿扰 / 专注 / 频率 / 预算）+ 发起闸 + 意愿判定 + 台账留痕
/// **全在 care 内核的管道里**。势能场若自己发消息，等于**绕过每一道闸** ——
/// 那正是本仓明令禁止的。走 [`ReminderFeed`] ⇒ 势能突破只产生**一条信号**，
/// 之后与天气提醒 / 守护告警**同管道同纪律**消费。
///
/// # 消费位
///
/// 产出的一条信号由内核裁决后回调 [`ReminderFeed::mark_consumed`] 清位；
/// 未被消费（被 Held / Idle 拦下）时**下一拍重投同一条**（不重复造信号）。
pub struct CarePotentialFeed {
    driver: Arc<CarePotentialDriver>,
    state: Arc<BioLifecycleState>,
    pending: Mutex<Option<ReminderCandidate>>,
    last_poll_ms: AtomicU64,
}

impl CarePotentialFeed {
    /// 构造信号桥。`state` = 真实信号源（生命周期状态句柄）。
    ///
    /// **时基统一**：拍时刻取 `state.now_ms()`（与「最近工具成功时刻」同一时钟），
    /// 不另引时钟 ⇒ 时钟回拨/注入时钟下两者永远同轴。
    #[must_use]
    pub fn new(driver: Arc<CarePotentialDriver>, state: Arc<BioLifecycleState>) -> Self {
        // 构造即锚定起始时刻 ⇒ **首拍**的 dt = 构造到首拍的真实间隔
        // (生产 = 一个心跳间隔; 不是 0)。否则首拍 dt=0、势能永远起步为 0。
        let start_ms = state.now_ms();
        Self {
            driver,
            state,
            pending: Mutex::new(None),
            last_poll_ms: AtomicU64::new(start_ms),
        }
    }

    /// 势能场驱动器句柄（诊断/自述用）。
    #[must_use]
    pub fn driver(&self) -> &Arc<CarePotentialDriver> {
        &self.driver
    }

    /// 把一条关怀动作转成提醒信号（分级 ⇒ 严重度档）。
    fn candidate_for(action: &CareAction, now_ms: u64) -> ReminderCandidate {
        let (severity, message) = match action {
            CareAction::AmbientGlowPulse { color_temp_k, .. } => (
                "info",
                format!("环境光已调暖到 {color_temp_k}K（关怀势能场 level 1）"),
            ),
            CareAction::SilentPreparation { summary, .. } => ("info", summary.clone()),
            CareAction::WhisperCare {
                suggested_speech, ..
            } => ("warn", suggested_speech.clone()),
        };
        ReminderCandidate {
            signal_id: format!("carepot-{now_ms}"),
            tier: CareTier::Reminder,
            trigger: "care_potential".to_string(),
            severity: severity.to_string(),
            message,
            evidence: "care_potential 突破阈值（连续失败/无成功时长为真实信号）".to_string(),
            source: "care_potential".to_string(),
            created_at_ms: now_ms as i64,
        }
    }
}

impl ReminderFeed for CarePotentialFeed {
    fn pending(&self) -> Result<Vec<ReminderCandidate>, String> {
        if !self.driver.enabled() {
            return Ok(Vec::new());
        }
        // 已有未消费信号 ⇒ 原样重投（不重复造、不重复推进势能）。
        if let Some(candidate) = self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
        {
            return Ok(vec![candidate]);
        }
        let now_ms = self.state.now_ms();
        let last = self
            .last_poll_ms
            .swap(now_ms, std::sync::atomic::Ordering::SeqCst);
        let dt_seconds = if last == 0 {
            0.0
        } else {
            (now_ms.saturating_sub(last)) as f64 / 1000.0
        };
        let Some(action) = self.driver.poll(&self.state, now_ms, dt_seconds) else {
            return Ok(Vec::new());
        };
        let candidate = Self::candidate_for(&action, now_ms);
        if let Ok(mut guard) = self.pending.lock() {
            *guard = Some(candidate.clone());
        }
        Ok(vec![candidate])
    }

    fn mark_consumed(&self, signal_id: &str) -> Result<(), String> {
        let mut guard = self
            .pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard
            .as_ref()
            .is_some_and(|candidate| candidate.signal_id == signal_id)
        {
            *guard = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod feed_tests {
    use super::*;
    use apeireth_organ::care::{CareEngine, ReminderFeed};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    fn state_with_clock(now: Arc<AtomicU64>) -> BioLifecycleState {
        let clock: crate::canonical::care_heartbeat::bio_lifecycle::LifecycleClock = {
            let n = now.clone();
            Arc::new(move || n.load(Ordering::SeqCst))
        };
        BioLifecycleState::with_policy(
            apeireth_orchestration::lifecycle::LifecyclePolicy::default(),
            8,
            clock,
        )
    }

    #[test]
    fn feed_is_silent_without_breach_and_produces_one_signal_after_breach() {
        let now = Arc::new(AtomicU64::new(1_000));
        let st = state_with_clock(now.clone());
        for i in 0..3 {
            st.observe_tool_outcome_at("tool.f", false, 1_000 + i * 10)
                .expect("record failure");
        }
        let driver = Arc::new(CarePotentialDriver::new(1.0, true));
        let feed = CarePotentialFeed::new(driver, Arc::new(st));

        // 构造锚定 1_000 ⇒ 首拍 dt = (now - 1_000); 推进 60s 后 dt=60s,
        // distress=1.0 ⇒ gain 0.8 ⇒ Δu=48 ≥ 阈值 1.0 ⇒ 突破 ⇒ 一条信号
        now.store(61_000, Ordering::SeqCst);
        let pending = feed.pending().expect("pending");
        assert_eq!(pending.len(), 1);
        let candidate = &pending[0];
        assert_eq!(candidate.tier, CareTier::Reminder);
        assert_eq!(candidate.source, "care_potential");
        assert!(candidate.signal_id.starts_with("carepot-"));
        assert!(!candidate.message.is_empty());
        assert!(candidate.evidence.contains("care_potential"));
    }

    #[test]
    fn unconsumed_signal_is_replayed_not_duplicated() {
        let now = Arc::new(AtomicU64::new(1_000));
        let st = state_with_clock(now.clone());
        for i in 0..3 {
            st.observe_tool_outcome_at("tool.f", false, 1_000 + i * 10)
                .expect("record failure");
        }
        let feed =
            CarePotentialFeed::new(Arc::new(CarePotentialDriver::new(1.0, true)), Arc::new(st));
        now.store(61_000, Ordering::SeqCst);
        let first = feed.pending().expect("pending");
        assert_eq!(first.len(), 1);
        let second = feed.pending().expect("pending");
        assert_eq!(second.len(), 1);
        assert_eq!(
            first[0].signal_id, second[0].signal_id,
            "同一条重投, 不造新信号"
        );
        feed.mark_consumed(&first[0].signal_id).expect("consume");
        // 消费位清空后: 势能内核在阈值以上**会再次突破**（每次泄放 70%, 属内核既有
        // 语义, 不由本面改）⇒ 下一拍会**重新评估**并可能再产一条信号。
        // 身份规则 = 时间戳派生 (`carepot-<now_ms>`): 同一毫秒内再突破得到同 id,
        // 由 care 内核按 id 去重（台账已有该 id ⇒ 不重发）—— 那是内核的职责。
        let ticks_before = feed.driver().ticks();
        let third = feed.pending().expect("pending");
        assert!(
            feed.driver().ticks() > ticks_before,
            "消费后应重新评估势能（而非复投陈旧信号）"
        );
        for candidate in &third {
            assert_eq!(
                candidate.signal_id,
                format!("carepot-{}", now.load(Ordering::SeqCst)),
                "信号身份按时间戳派生"
            );
        }
    }

    #[test]
    fn disabled_driver_yields_no_signal_forever() {
        let now = Arc::new(AtomicU64::new(1_000));
        let st = state_with_clock(now.clone());
        for i in 0..5 {
            st.observe_tool_outcome_at("tool.f", false, 1_000 + i * 10)
                .expect("record failure");
        }
        let feed =
            CarePotentialFeed::new(Arc::new(CarePotentialDriver::new(1.0, false)), Arc::new(st));
        for i in 0..20 {
            now.store(1_000 + i * 60_000, Ordering::SeqCst);
            assert!(feed.pending().expect("pending").is_empty());
        }
    }

    #[test]
    fn signal_goes_through_the_care_discipline_pipeline() {
        // 势能突破产出的信号必须能被 care 内核按**提醒档**消费 (纪律同源,
        // 不绕过勿扰/专注/频率/预算/发起闸)。
        let now = Arc::new(AtomicU64::new(1_000));
        let st = state_with_clock(now.clone());
        for i in 0..3 {
            st.observe_tool_outcome_at("tool.f", false, 1_000 + i * 10)
                .expect("record failure");
        }
        let feed =
            CarePotentialFeed::new(Arc::new(CarePotentialDriver::new(1.0, true)), Arc::new(st));
        now.store(61_000, Ordering::SeqCst);
        let signal = feed.pending().expect("pending").remove(0);

        let dir = tempfile::tempdir().expect("temp dir");
        let store = apeireth_organ::care::CareStore::new(dir.path());
        let clock = Arc::new(apeireth_organ::care::FrozenCareClock::new(
            1_700_000_000_000,
        ));
        let mut engine = CareEngine::new(
            store,
            clock,
            Arc::new(apeireth_organ::care::RuleFallbackJudge),
        );
        let outcome = engine.poll_reminder(&signal).expect("poll reminder");
        // 纪律管道要么 Idle (被纪律拦下/台账已有) 要么 Initiating —— 但**不得**是错误,
        // 且候选来源档必须是提醒档 (非 bio 档, 非越权直发)。
        assert!(
            matches!(
                outcome,
                apeireth_organ::care::CarePoll::Idle
                    | apeireth_organ::care::CarePoll::Held { .. }
                    | apeireth_organ::care::CarePoll::Declined { .. }
                    | apeireth_organ::care::CarePoll::Initiating(_)
            ),
            "势能信号必须走标准纪律管道, 不得报错或越权"
        );
    }
}
