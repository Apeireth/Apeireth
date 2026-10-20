//! 生物机制族 W3.3 · 调质 → 语气装配面 (默认关, fail-closed)。
//!
//! # 机制
//!
//! 内核 [`apeireth_organ::neuromodulation`] 的六维慢变量状态机 +
//! [`apeireth_organ::tone`] 的调质语气桥 (两者 W1/W3.1 已备, 本批**零改动**):
//!
//! ```text
//! 显式反馈事件 (ToneState::apply_feedback)
//!   └─ ToneFeedbackKind → NeuromodulatorEvent → NeuromodulatorState::apply(event, now_ms)
//!        └─ behavior_params() → tone_hint_with_params(bond, params)
//!             └─ BioToneModule (HookPoint::TurnStart)
//!                  └─ 一个独立 PromptOverlay::system (追加在装配尾部, 不碰既有前缀)
//! ```
//!
//! # 开关 (默认关)
//!
//! 装配门 = env [`BIO_TONE_ENABLE_ENV`] (`APEIRETH_ENABLE_BIO_TONE`, **只认 `"1"`**;
//! 未设置 / 空 / `0` / `true` / `on` / 其它一律关 = fail-closed)。
//! 关 = 不构造模块 (懒加载), 且 [`BioToneModule::new`] 的 `enabled = false` 是第二道门
//! (defense-in-depth): 钩子直接 `ModuleOutcome::continue_()`, 零 overlay。
//!
//! # 零变化语义 (两层)
//!
//! 1. **开关关**: 零 overlay, 与未接线逐字节一致;
//! 2. **开关开但调质仍在基线档**: [`BioToneModule::assemble`] 也不发 overlay —— 没有
//!    显式反馈事件就没有语气调制 (不拿默认关系快照编造「调质语气」)。只有
//!    [`ToneState::apply_feedback`] 把六维推离基线、离散语气档位真的移动后, 才在
//!    `TurnStart` 追加**一个**系统块。
//!
//! # 缓存前缀纪律
//!
//! overlay 经 [`ModuleOutcome::with_prompt_overlay`] 追加: 独立 system 块, 不改写会话
//! 历史 / 既有 system 前缀 / provider 缓存前缀; 同一回合最多一块, 文本来自固定词表
//! (`tone_hint_with_params` 的基线句 + 【调质语气】固定片段), 不含用户原文。
//!
//! # 0 装诚实 (能力级别: IMPLEMENTED + 默认关; 非 PRODUCTION WIRED)
//!
//! - **事件源未接**: 真实反馈事件源 (工具结果 / 用户纠错检测 / 社交反馈识别) **未接**;
//!   本批只有显式注入口 [`ToneState::apply_feedback`] (测试/宿主可调), 不读事件流、
//!   **不编事件**。
//! - **关系快照未接**: [`BondCharacterSnapshot`] 缺省为 `default()`; 真实羁绊存储
//!   (`BondCharacterStore`) 未读取, 需宿主经 [`ToneState::set_bond_snapshot`] 显式注入。
//! - **时间注入**: 逻辑时刻由构造时注入的 [`ToneClock`] 给出, 状态机自身不读系统时钟;
//!   生产 = 系统时钟, 测试 = 冻结时钟。
//! - **无持久化 / 无遥测 / 无 live E2E**: 重启丢调质状态; `production.rs` 尚未注册
//!   本模块 —— 注册由 Lead 后续做 ([`bio_tone_wiring_from_env`] 是可调 helper)。
//! - 调质是工程标签, 不宣称主观情绪、意识或生理真实性。

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_organ::neuromodulation::{
    BehaviorParams, NeuromodulatorConfig, NeuromodulatorEvent, NeuromodulatorState,
    NeuromodulatorVector,
};
use apeireth_organ::tone::{
    behavior_tone_style, tone_hint, tone_hint_with_params, BehaviorToneStyle, BondCharacterSnapshot,
};

use crate::canonical::module::{
    AgentModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
    PromptOverlay,
};

/// 装配开闸 env 名 (仅显式 `"1"` 开, 其余一律关 fail-closed)。
pub const BIO_TONE_ENABLE_ENV: &str = "APEIRETH_ENABLE_BIO_TONE";

/// 模块稳定标识 (自述 / 注册去重 / 测试断言共用)。
pub const BIO_TONE_MODULE_ID: &str = "bio_tone";

/// 语气覆盖块里导出的稳定文案标签 (与 `tone_hint_with_params` 的既有片段一致)。
pub const TONE_CLAUSE_LABEL: &str = "【调质语气】";

/// 开关解析 (纯函数): 仅显式 `"1"` 开; 未设置 / 空 / `0` / `true` / `yes` / `on` /
/// 任意其它值一律关 (默认关 fail-closed)。`trim` 后比较, 因此 `" 1 "` 算开。
#[must_use]
pub fn bio_tone_enabled_from_env_value(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim() == "1")
}

/// 调质状态时钟: 返回 epoch 毫秒 (`u64`)。时间显式注入, 状态机不读系统时钟。
pub type ToneClock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// 显式反馈事件 (装配面事件注入口的闭合词表)。
///
/// 每个档位一一映射到 [`NeuromodulatorEvent`] (九类事件全覆盖), 不做任何额外解释:
/// 装配面只负责「谁在什么时候报告了哪一类反馈」, 状态变化全部由内核确定性计算。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneFeedbackKind {
    /// 奖赏预测误差: `actual - expected` (有符号脉冲)。
    RewardPrediction {
        /// 预期值 (内核按非有限 → 0 安全化)。
        expected: f32,
        /// 实际值 (内核按非有限 → 0 安全化)。
        actual: f32,
    },
    /// 任务成功。
    TaskSuccess,
    /// 任务失败 (失败也是学习信号)。
    TaskFailure,
    /// 社交正反馈。
    SocialPositive,
    /// 社交负反馈。
    SocialNegative,
    /// 威胁 / 危险信号。
    Threat,
    /// 新异刺激。
    Novelty,
    /// 安静期。
    Quiet,
    /// 用户纠错 (注意重定向)。
    UserCorrection,
}

impl ToneFeedbackKind {
    /// 映射到内核事件 (纯函数, 无 IO)。
    #[must_use]
    pub const fn to_event(self) -> NeuromodulatorEvent {
        match self {
            Self::RewardPrediction { expected, actual } => {
                NeuromodulatorEvent::RewardPredictionError { expected, actual }
            }
            Self::TaskSuccess => NeuromodulatorEvent::TaskSuccess,
            Self::TaskFailure => NeuromodulatorEvent::TaskFailure,
            Self::SocialPositive => NeuromodulatorEvent::SocialPositive,
            Self::SocialNegative => NeuromodulatorEvent::SocialNegative,
            Self::Threat => NeuromodulatorEvent::Threat,
            Self::Novelty => NeuromodulatorEvent::Novelty,
            Self::Quiet => NeuromodulatorEvent::Quiet,
            Self::UserCorrection => NeuromodulatorEvent::UserCorrection,
        }
    }

    /// 中性 ASCII 标签 (日志 / 遥测 / 自述用, 不含用户原文)。
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.to_event().code()
    }
}

/// 一次显式反馈注入的回执 (前后快照, 测试 / 诊断真值)。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToneFeedbackReceipt {
    /// 注入的反馈档。
    pub feedback: ToneFeedbackKind,
    /// 映射到的事件稳定标签。
    pub event_code: &'static str,
    /// 注入时刻 (注入时钟真值, epoch 毫秒)。
    pub at_ms: u64,
    /// 注入前六维向量。
    pub before: NeuromodulatorVector,
    /// 注入后六维向量。
    pub after: NeuromodulatorVector,
}

impl ToneFeedbackReceipt {
    /// 本次注入是否真的改变了六维状态 (事件增益为 0 / 已在 clamp 界时可能不变)。
    #[must_use]
    pub fn changed(&self) -> bool {
        self.before != self.after
    }
}

/// 调质状态 (装配面唯一持有者): `Arc<Mutex<NeuromodulatorState>>` + 关系快照 + 注入时钟。
///
/// 内核状态机是纯确定性的; 本类型只做三件事: 持有共享状态、把显式反馈映射成内核事件、
/// 把状态读成语气文本。**不读事件流、不读时钟 (除注入时钟)、不编事件、不落盘**。
pub struct ToneState {
    neuromodulators: Arc<Mutex<NeuromodulatorState>>,
    bond: Mutex<BondCharacterSnapshot>,
    clock: ToneClock,
}

impl ToneState {
    /// 以基线调质状态 + 默认关系快照构造; 逻辑时刻由 `clock` 注入。
    #[must_use]
    pub fn new(clock: ToneClock) -> Self {
        Self {
            neuromodulators: Arc::new(Mutex::new(NeuromodulatorState::new(
                NeuromodulatorConfig::default(),
            ))),
            bond: Mutex::new(BondCharacterSnapshot::default()),
            clock,
        }
    }

    /// 共享内核状态句柄 (宿主可在同一实例上直接调用内核 API; 本面不复制状态)。
    #[must_use]
    pub fn neuromodulator_state(&self) -> Arc<Mutex<NeuromodulatorState>> {
        Arc::clone(&self.neuromodulators)
    }

    /// 当前六维向量 (只读快照)。
    #[must_use]
    pub fn vector(&self) -> NeuromodulatorVector {
        self.lock_neuromodulators().vector()
    }

    /// 当前行为参数 (唯一内核映射出口)。
    #[must_use]
    pub fn behavior_params(&self) -> BehaviorParams {
        self.lock_neuromodulators().behavior_params()
    }

    /// 当前离散语气档位 (六维 + 有限片段预算)。
    #[must_use]
    pub fn tone_style(&self) -> BehaviorToneStyle {
        behavior_tone_style(&self.behavior_params())
    }

    /// 当前是否仍在基线语气档 (六维全部中间档 = 无调质调制)。
    #[must_use]
    pub fn is_baseline(&self) -> bool {
        self.tone_style().is_baseline()
    }

    /// 显式事件注入口: 反馈档 → 内核事件 → 状态 (时刻取注入时钟)。
    ///
    /// **不编事件**: 本函数只接受调用方显式给出的反馈档; 不读任何事件流 / 文本。
    /// 返回前后快照回执; 事件增益为 0 或状态已在 clamp 界时 `changed() == false`。
    pub fn apply_feedback(&self, feedback: ToneFeedbackKind) -> ToneFeedbackReceipt {
        let at_ms = (self.clock)();
        let mut state = self.lock_neuromodulators();
        let before = state.vector();
        let event = feedback.to_event();
        let event_code = event.code();
        state.apply(&event, at_ms);
        let after = state.vector();
        ToneFeedbackReceipt {
            feedback,
            event_code,
            at_ms,
            before,
            after,
        }
    }

    /// 当前关系快照 (拷贝)。
    #[must_use]
    pub fn bond_snapshot(&self) -> BondCharacterSnapshot {
        *self.lock_bond()
    }

    /// 显式注入关系快照 (真实羁绊存储未接; 宿主/测试可给出自己的快照)。
    pub fn set_bond_snapshot(&self, snapshot: BondCharacterSnapshot) {
        *self.lock_bond() = snapshot;
    }

    /// 当前语气提示文本 (基线档逐字等于 [`tone_hint`]; 非基线档追加固定片段)。
    #[must_use]
    pub fn current_tone_hint(&self) -> String {
        tone_hint_with_params(&self.bond_snapshot(), &self.behavior_params())
    }

    fn lock_neuromodulators(&self) -> MutexGuard<'_, NeuromodulatorState> {
        self.neuromodulators
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn lock_bond(&self) -> MutexGuard<'_, BondCharacterSnapshot> {
        self.bond
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl std::fmt::Debug for ToneState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ToneState")
            .field("vector", &self.vector())
            .field("bond", &self.bond_snapshot())
            .finish_non_exhaustive()
    }
}

/// 一次 `TurnStart` 的装配结果 (纯函数输出, 可观测; 不含用户原文)。
#[derive(Debug, Clone, PartialEq)]
pub struct BioToneTurn {
    /// 模块开关现况。
    pub enabled: bool,
    /// 本次读到的行为参数。
    pub params: BehaviorParams,
    /// 本次读到的离散语气档位。
    pub style: BehaviorToneStyle,
    /// 是否仍在基线档 (true = 本次零 overlay)。
    pub baseline: bool,
    /// 尾部独立 system 块; `None` = 本次零输出。
    pub overlay: Option<PromptOverlay>,
}

/// 调质 → 语气装配模块 (`HookPoint::TurnStart` 尾部独立 system 块)。
///
/// - 开关关 / 非 `TurnStart` 钩子 → `ModuleOutcome::continue_()` (零 overlay);
/// - 开关开但调质仍在基线档 → 同样零 overlay (不编「调质语气」);
/// - 开关开 + 显式反馈事件把语气档推离基线 → exactly one `PromptOverlay::system`。
pub struct BioToneModule {
    manifest: ModuleManifest,
    enabled: bool,
    state: Arc<ToneState>,
}

impl BioToneModule {
    /// 按显式开关 + 共享调质状态构造。
    #[must_use]
    pub fn new(enabled: bool, state: Arc<ToneState>) -> Self {
        Self {
            manifest: ModuleManifest::new(BIO_TONE_MODULE_ID, "Bio Tone Modulation"),
            enabled,
            state,
        }
    }

    /// 开关现况。
    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// 共享调质状态 (事件注入面)。
    #[must_use]
    pub fn state(&self) -> &Arc<ToneState> {
        &self.state
    }

    /// 装配一次 (纯读: 不加锁写状态、不产生副作用)。
    #[must_use]
    pub fn assemble(&self) -> BioToneTurn {
        let params = self.state.behavior_params();
        let style = behavior_tone_style(&params);
        let baseline = style.is_baseline();
        let overlay = (self.enabled && !baseline)
            .then(|| PromptOverlay::system(self.state.current_tone_hint()));
        BioToneTurn {
            enabled: self.enabled,
            params,
            style,
            baseline,
            overlay,
        }
    }
}

#[async_trait::async_trait]
impl AgentModule for BioToneModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        _ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook != HookPoint::TurnStart || !self.enabled {
            return Ok(ModuleOutcome::continue_());
        }
        let Some(overlay) = self.assemble().overlay else {
            return Ok(ModuleOutcome::continue_());
        };
        Ok(ModuleOutcome::continue_().with_prompt_overlay(overlay))
    }
}

/// 装配结果: 共享调质状态 + 一个模块 (`module()` 交给 `ProductionModules` 注册)。
pub struct BioToneWiring {
    state: Arc<ToneState>,
    module: Arc<dyn AgentModule>,
}

impl BioToneWiring {
    /// 共享调质状态 (真实反馈事件源接上后从这条口注入)。
    #[must_use]
    pub fn state(&self) -> &Arc<ToneState> {
        &self.state
    }

    /// 待注册模块 (`RuntimeBuilder::with_module` / `ProductionModules` 同口)。
    #[must_use]
    pub fn module(&self) -> Arc<dyn AgentModule> {
        Arc::clone(&self.module)
    }
}

/// 开关值 → 模块 (开关关 = `None`, 不构造; 供宿主自带 [`ToneState`] 时使用)。
#[must_use]
pub fn bio_tone_module_from_env_value(
    raw: Option<&str>,
    state: Arc<ToneState>,
) -> Option<Arc<dyn AgentModule>> {
    bio_tone_enabled_from_env_value(raw)
        .then(|| Arc::new(BioToneModule::new(true, state)) as Arc<dyn AgentModule>)
}

/// 开关值 → (共享调质状态 + 模块); 开关关 = `None` (懒加载)。
///
/// `clock` 是调质状态时钟 (生产 = 系统时钟, 测试 = 冻结时钟): 本 helper 不自行读时钟,
/// 时间源由组装根显式给出。
#[must_use]
pub fn bio_tone_wiring_from_env_value(
    raw: Option<&str>,
    clock: ToneClock,
) -> Option<BioToneWiring> {
    if !bio_tone_enabled_from_env_value(raw) {
        return None;
    }
    let state = Arc::new(ToneState::new(clock));
    let module: Arc<dyn AgentModule> = Arc::new(BioToneModule::new(true, Arc::clone(&state)));
    Some(BioToneWiring { state, module })
}

/// 从 env 现值装配 (同 [`bio_tone_wiring_from_env_value`] 的 fail-closed 口径)。
#[must_use]
pub fn bio_tone_wiring_from_env(clock: ToneClock) -> Option<BioToneWiring> {
    bio_tone_wiring_from_env_value(std::env::var(BIO_TONE_ENABLE_ENV).ok().as_deref(), clock)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use apeireth_organ::neuromodulation::NeuromodulatorKind;

    use super::*;

    const NOW_MS: u64 = 1_760_000_000_000;

    fn frozen_clock() -> ToneClock {
        Arc::new(|| NOW_MS)
    }

    fn baseline_state() -> Arc<ToneState> {
        Arc::new(ToneState::new(frozen_clock()))
    }

    #[test]
    fn env_switch_only_accepts_explicit_one_and_is_lazy() {
        assert_eq!(BIO_TONE_ENABLE_ENV, "APEIRETH_ENABLE_BIO_TONE");
        assert!(bio_tone_enabled_from_env_value(Some("1")));
        assert!(bio_tone_enabled_from_env_value(Some(" 1 ")));
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
            Some("1.0"),
        ] {
            assert!(
                !bio_tone_enabled_from_env_value(raw),
                "非 \"1\" 一律关 (fail-closed): {raw:?}"
            );
            assert!(
                bio_tone_wiring_from_env_value(raw, frozen_clock()).is_none(),
                "关 = 不构造: {raw:?}"
            );
        }
        let wiring = bio_tone_wiring_from_env_value(Some("1"), frozen_clock()).expect("开");
        assert_eq!(wiring.module().manifest().id, BIO_TONE_MODULE_ID);
        assert!(wiring.state().is_baseline());
    }

    #[test]
    fn every_feedback_kind_maps_to_its_documented_neuromodulator_event() {
        let cases = [
            (
                ToneFeedbackKind::RewardPrediction {
                    expected: 0.2,
                    actual: 0.8,
                },
                "reward_prediction_error",
            ),
            (ToneFeedbackKind::TaskSuccess, "task_success"),
            (ToneFeedbackKind::TaskFailure, "task_failure"),
            (ToneFeedbackKind::SocialPositive, "social_positive"),
            (ToneFeedbackKind::SocialNegative, "social_negative"),
            (ToneFeedbackKind::Threat, "threat"),
            (ToneFeedbackKind::Novelty, "novelty"),
            (ToneFeedbackKind::Quiet, "quiet"),
            (ToneFeedbackKind::UserCorrection, "user_correction"),
        ];
        for (feedback, code) in cases {
            assert_eq!(feedback.code(), code);
            assert_eq!(feedback.to_event().code(), code);
        }
        assert_eq!(
            ToneFeedbackKind::RewardPrediction {
                expected: 0.2,
                actual: 0.8,
            }
            .to_event()
            .reward_prediction_error(),
            Some(0.6)
        );
    }

    #[test]
    fn injected_feedback_moves_the_state_in_the_documented_direction() {
        let state = baseline_state();
        let baseline = state.vector();

        let correction = state.apply_feedback(ToneFeedbackKind::UserCorrection);
        assert!(correction.changed());
        assert!(correction.after.acetylcholine > baseline.acetylcholine);
        assert!(correction.after.cortisol > baseline.cortisol);

        let threat = state.apply_feedback(ToneFeedbackKind::Threat);
        assert!(threat.changed());
        assert!(threat.after.cortisol > correction.after.cortisol);
        assert!(threat.after.noradrenaline > correction.after.noradrenaline);

        assert_eq!(threat.event_code, "threat");
        assert_eq!(threat.at_ms, NOW_MS, "时刻来自注入时钟");
        assert_eq!(
            state.neuromodulator_state().lock().expect("lock").vector(),
            threat.after,
            "共享 Arc<Mutex<_>> 是唯一状态真值"
        );
    }

    #[test]
    fn baseline_params_fall_back_to_the_existing_tone_hint_verbatim() {
        let state = baseline_state();
        let baseline = BehaviorParams::baseline();
        let snapshots = [
            BondCharacterSnapshot::default(),
            BondCharacterSnapshot {
                trust: 0.5,
                ..BondCharacterSnapshot::default()
            },
            BondCharacterSnapshot {
                trust: 0.8,
                resonance: 0.7,
                ..BondCharacterSnapshot::default()
            },
            BondCharacterSnapshot {
                resonance: 0.5,
                ..BondCharacterSnapshot::default()
            },
        ];
        for snapshot in snapshots {
            assert_eq!(
                tone_hint_with_params(&snapshot, &baseline),
                tone_hint(&snapshot),
                "基线参数必须逐字回退既有 tone_hint"
            );
        }
        assert_eq!(
            state.current_tone_hint(),
            tone_hint(&state.bond_snapshot()),
            "装配面基线档逐字回退"
        );
        assert!(state.is_baseline());
    }

    #[test]
    fn the_enabled_module_stays_silent_until_an_event_moves_the_tone_style() {
        let state = baseline_state();
        let module = BioToneModule::new(true, Arc::clone(&state));
        let baseline = module.assemble();
        assert!(baseline.baseline);
        assert!(
            baseline.overlay.is_none(),
            "开关开但基线档 = 零 overlay (不编调质语气)"
        );

        // 同类事件累计到档位真的移动 (社交正反馈 → 更暖)。
        for _ in 0..3 {
            state.apply_feedback(ToneFeedbackKind::SocialPositive);
        }
        let modulated = module.assemble();
        assert!(!modulated.baseline);
        let overlay = modulated.overlay.expect("非基线档必须出现独立块");
        let text = apeireth_protocol::canonical::ContentPart::join_text(&overlay.message().content);
        assert!(
            text.starts_with(tone_hint(&state.bond_snapshot())),
            "保留既有基线前缀: {text}"
        );
        assert!(text.contains(TONE_CLAUSE_LABEL), "固定标签: {text}");
        assert!(text.contains("更温暖"), "方向: {text}");
    }

    #[test]
    fn a_disabled_module_never_assembles_an_overlay_even_with_moved_state() {
        let state = baseline_state();
        for _ in 0..2 {
            state.apply_feedback(ToneFeedbackKind::Threat);
        }
        let module = BioToneModule::new(false, state);
        let turn = module.assemble();
        assert!(!turn.baseline, "状态确实已非基线");
        assert!(
            turn.overlay.is_none(),
            "开关关 = defense-in-depth 零 overlay"
        );
        assert!(!module.enabled());
    }

    #[test]
    fn injected_bond_snapshot_changes_the_base_prefix_but_not_the_state() {
        let state = baseline_state();
        let module = BioToneModule::new(true, Arc::clone(&state));
        state.set_bond_snapshot(BondCharacterSnapshot {
            trust: 0.8,
            resonance: 0.7,
            ..BondCharacterSnapshot::default()
        });
        state.apply_feedback(ToneFeedbackKind::SocialPositive);
        state.apply_feedback(ToneFeedbackKind::SocialPositive);
        state.apply_feedback(ToneFeedbackKind::SocialPositive);
        let overlay = module.assemble().overlay.expect("非基线档");
        let text = apeireth_protocol::canonical::ContentPart::join_text(&overlay.message().content);
        assert!(
            text.starts_with(tone_hint(&state.bond_snapshot())),
            "基底句来自注入了关系快照的既有 tone_hint: {text}"
        );
        assert!(text.starts_with("轻松亲切"), "信任/共鸣 ≥ 0.6 档: {text}");
        assert_eq!(
            state.tone_style().warmth,
            apeireth_organ::tone::WarmthLevel::Affectionate
        );
        assert_eq!(
            state.vector().get(NeuromodulatorKind::Dopamine),
            state.vector().dopamine,
            "六维下标访问与字段一致 (状态未被关系快照改写)"
        );
    }

    #[test]
    fn the_clock_is_the_only_time_source_and_is_injected() {
        let ticks = Arc::new(AtomicU64::new(0));
        let observed = Arc::clone(&ticks);
        let clock: ToneClock = Arc::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            7
        });
        let state = ToneState::new(clock);
        assert_eq!(ticks.load(Ordering::SeqCst), 0, "构造不读时钟");
        let receipt = state.apply_feedback(ToneFeedbackKind::Quiet);
        assert_eq!(receipt.at_ms, 7);
        assert_eq!(ticks.load(Ordering::SeqCst), 1, "每个事件恰好读一次");
    }
}
