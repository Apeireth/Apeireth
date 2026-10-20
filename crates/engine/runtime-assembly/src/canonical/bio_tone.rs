//! 生物机制族 W3.3 · 调质 → 语气装配面 (默认关, fail-closed; W3.5/W3.7 补真实事件源)。
//!
//! # 机制
//!
//! 内核 [`apeireth_organ::neuromodulation`] 的六维慢变量状态机 +
//! [`apeireth_organ::tone`] 的调质语气桥 (两者 W1/W3.1 已备, 本批**零改动**):
//!
//! ```text
//! 真实事件源 (既有运行时钩子, 见下节) ──┐
//! 显式注入口 (ToneState::apply_feedback) ─┤
//!                                         └─ ToneFeedbackKind → NeuromodulatorEvent
//!   → NeuromodulatorState::apply(event, now_ms)
//!      └─ behavior_params() → tone_hint_with_params(bond, params)
//!           └─ BioToneModule (HookPoint::TurnStart)
//!                └─ 一个独立 PromptOverlay::system (装配尾部, 不碰既有前缀)
//! ```
//!
//! # 真实事件源 (本批接线; 复用既有钩子面, 不新建总线)
//!
//! 事件源 = [`BioToneModule`] 在既有模块钩子上的旁路观察, **仅开关开时消费**
//! (关 = 模块不注册且 `enabled = false` 双闸, 零观察零状态变化):
//!
//! | 既有真实事件 | 确定性判定 | 反馈档 | 说明 |
//! |---|---|---|---|
//! | [`HookPoint::AfterToolResult`] + [`ToolResult`] | `ToolResult::is_ok()` | [`ToneFeedbackKind::TaskSuccess`] | 协议层显式结算为成功 |
//! | [`HookPoint::AfterToolResult`] + [`ToolResult`] | `ToolOutcome::Error { .. }` | [`ToneFeedbackKind::TaskFailure`] | 协议层显式结算为失败 (可重试/不可重试同档) |
//! | [`HookPoint::OnError`] + 错误文本 | `error` 存在 | [`ToneFeedbackKind::TaskFailure`] | 运行时 / 模块失败的 best-effort 观察钩 |
//! | [`HookPoint::AfterToolResult`] + [`ToolCall`] | 工具名在**本进程内首次出现** (模块持有首见集, 重复 / 空名不注入) | [`ToneFeedbackKind::Novelty`] | 新工具面首次触达 = 确定性新颖信号, 不做内容推断 |
//!
//! 映射规则: 缺载荷 (`tool_call` / `tool_result` / `error` 为 `None`) = 无事件 = 不注入; 其它钩子
//! (含 `TurnStart`) 不产生反馈。不读用户文本、不做情感 / 意图推断 —— 只把协议里
//! **已有显式判决**的结局与显式载荷映射成事件, 不替任何一档编造信号。
//! 首见集口径: 进程生命周期内按工具名去重 (工具名来自名册, 有限集合); 不区分会话、
//! 无 TTL / 无淘汰, 重启归零。
//!
//! 粒度为**工具调用**级: 一次结算 = 一次脉冲, 无节流 / 无去重; 高频工具面会更快把
//! 六维推离基线 (已知量化缺口, 窗口未标定)。状态仍在基线档时不发 overlay。
//!
//! **装配面社会反馈源 (W3.7)**: CLI 审批落账 (`apeireth approve` / `apeireth reject`)
//! 在 resolve 成功后, 经装配根交付的共享状态句柄 (`ProductionModules::bio_tone_state`)
//! 旁路注入一次社会反馈 —— 人批 → [`ToneFeedbackKind::SocialPositive`], 人拒 →
//! [`ToneFeedbackKind::SocialNegative`], 取消 (`cancel`) = 收回请求、不构成社会结论
//! = 不注入。该源在装配根层, 不经模块钩子; **不改审批结论、不参与权限判定**, 只把
//! 已经发生的人机社会结果记成调质脉冲。开关关时装配根不交付句柄 = 零观察零变化。
//!
//! **未接的真实源 (诚实边界)**: 用户显式好评 / 差评的采集入口在装配面不存在 ——
//! `self_tuning::TuningEvent::ToneFeedback`、`value_cases::Feedback`、
//! `online_calibration::UserFeedback`、`orchestrator::OrchestratorFeedback` 四份接口
//! 当前均无生产调用方 (接口已备待接)。故 [`ToneFeedbackKind`] 其余四档
//! (RewardPrediction / Threat / Quiet / UserCorrection) 仍只经显式注入口
//! [`ToneState::apply_feedback`] 进入。
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
//! # 0 装诚实 (能力级别: PRODUCTION WIRED, 默认关; 非 DEFAULT ENABLED, 无 live E2E)
//!
//! - **事件源 (W3.7 半程)**: 已接 = 工具结算成功 / 失败 + 运行时错误 + 新工具首见
//!   Novelty (上表四行, 复用既有钩子面) + 审批面人批 / 人拒 (装配根句柄, 见上节);
//!   未接 = 用户显式好评 / 差评采集 (四处接口无生产调用方), 其余**四档**
//!   (RewardPrediction / Threat / Quiet / UserCorrection) 只有显式注入口。不读文本、
//!   不猜情绪、不做情感识别。
//! - **关系快照未接**: [`BondCharacterSnapshot`] 缺省为 `default()`; 真实羁绊存储
//!   (`BondCharacterStore`) 未读取, 需宿主经 [`ToneState::set_bond_snapshot`] 显式注入。
//! - **时间注入**: 逻辑时刻由构造时注入的 [`ToneClock`] 给出, 状态机自身不读系统时钟;
//!   生产 = 系统时钟, 测试 = 冻结时钟。
//! - **无持久化 / 无遥测 / 无 live E2E**: 重启丢调质状态; `production.rs` 已注册
//!   本模块 (`APEIRETH_ENABLE_BIO_TONE=1` 才构造) 并保存共享状态句柄
//!   ([`crate::canonical::production::ProductionModules::bio_tone_state`]), 但无
//!   live E2E 实测。
//! - **反馈注入点**: 句柄已有真实消费方 (CLI 审批面 `approval_tone_feedback`); 装配根
//!   另有 [`crate::canonical::production::ProductionModules::attach_tone_feedback_sink`]
//!   供其它既有反馈点 (例如未来的用户好评 / 差评采集) 持句柄旁路注入, 不复制总线。
//! - 调质是工程标签, 不宣称主观情绪、意识或生理真实性。

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_organ::neuromodulation::{
    BehaviorParams, NeuromodulatorConfig, NeuromodulatorEvent, NeuromodulatorState,
    NeuromodulatorVector,
};
use apeireth_organ::tone::{
    behavior_tone_style, tone_hint, tone_hint_with_params, BehaviorToneStyle, BondCharacterSnapshot,
};
use apeireth_protocol::canonical::{ToolCall, ToolResult};
use apeireth_runtime::ApprovalDecision;

use crate::canonical::module::{
    BehaviorModule, HookPoint, ModuleContext, ModuleError, ModuleManifest, ModuleOutcome,
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

/// 真实事件 → 反馈档的确定性映射 (既有钩子面, 零情感推断; 映射表见模块文档)。
///
/// - [`HookPoint::AfterToolResult`] + [`ToolResult`]: 成功 →
///   [`ToneFeedbackKind::TaskSuccess`], 失败 → [`ToneFeedbackKind::TaskFailure`];
/// - [`HookPoint::OnError`] + 错误文本 → [`ToneFeedbackKind::TaskFailure`];
/// - 缺载荷 / 其它钩子 → `None` (无事件 = 不注入; 不从缺失数据编信号)。
#[must_use]
pub fn tone_feedback_from_hook(
    hook: HookPoint,
    tool_result: Option<&ToolResult>,
    error: Option<&str>,
) -> Option<ToneFeedbackKind> {
    match hook {
        HookPoint::AfterToolResult => tool_result.map(|result| {
            if result.is_ok() {
                ToneFeedbackKind::TaskSuccess
            } else {
                ToneFeedbackKind::TaskFailure
            }
        }),
        HookPoint::OnError => error.map(|_| ToneFeedbackKind::TaskFailure),
        _ => None,
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
/// - 开关关: 所有钩子 → `ModuleOutcome::continue_()` (不观察事件、不注入状态、
///   零 overlay; defense-in-depth 第二道门);
/// - 开关开: 非 `TurnStart` 钩子只做**真实事件旁路** (工具结算 / 运行时错误 →
///   [`tone_feedback_from_hook`]), 不产 overlay、不改指令; `TurnStart` 只读装配:
///   基线档零 overlay, 非基线档 exactly one `PromptOverlay::system`。
pub struct BioToneModule {
    manifest: ModuleManifest,
    enabled: bool,
    state: Arc<ToneState>,
    seen_tools: Mutex<BTreeSet<String>>,
}

impl BioToneModule {
    /// 按显式开关 + 共享调质状态构造。
    #[must_use]
    pub fn new(enabled: bool, state: Arc<ToneState>) -> Self {
        Self {
            manifest: ModuleManifest::new(BIO_TONE_MODULE_ID, "Bio Tone Modulation"),
            enabled,
            state,
            seen_tools: Mutex::new(BTreeSet::new()),
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

    /// 旁路消费一次既有钩子上的真实事件 (只推进调质状态, 不产 overlay)。
    ///
    /// - 开关关 → `None` (零观察、状态不动);
    /// - 无事件载荷 / 非事件钩子 → `None` (无事件 = 不注入);
    /// - 有事件 → 一次 [`ToneState::apply_feedback`] 回执 (可诊断 / 可断言)。
    ///
    /// 本入口不观察工具名 (即不含 Novelty 源); 钩子面请用
    /// [`Self::record_hook_observation`]。
    pub fn record_hook_feedback(
        &self,
        hook: HookPoint,
        tool_result: Option<&ToolResult>,
        error: Option<&str>,
    ) -> Option<ToneFeedbackReceipt> {
        self.record_hook_observation(hook, None, tool_result, error)
            .into_iter()
            .next()
    }

    /// 旁路消费一次既有钩子上的全部真实事件 (只推进调质状态, 不产 overlay)。
    ///
    /// 事件顺序固定 (确定性): ①工具结算 / 运行时错误 (见
    /// [`tone_feedback_from_hook`]) → ②`AfterToolResult` 上工具名首见 → Novelty。
    /// 开关关 = **零观察**: 不写首见集、不回执 (关时逐字节零变化)。
    pub fn record_hook_observation(
        &self,
        hook: HookPoint,
        tool_call: Option<&ToolCall>,
        tool_result: Option<&ToolResult>,
        error: Option<&str>,
    ) -> Vec<ToneFeedbackReceipt> {
        if !self.enabled {
            return Vec::new();
        }
        let mut receipts = Vec::new();
        if let Some(feedback) = tone_feedback_from_hook(hook, tool_result, error) {
            receipts.push(self.state.apply_feedback(feedback));
        }
        if hook == HookPoint::AfterToolResult {
            let name = tool_call
                .map(|call| call.name.trim())
                .filter(|n| !n.is_empty());
            if let Some(name) = name {
                let mut seen = self.lock_seen_tools();
                if seen.insert(name.to_owned()) {
                    receipts.push(self.state.apply_feedback(ToneFeedbackKind::Novelty));
                }
            }
        }
        receipts
    }

    fn lock_seen_tools(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.seen_tools
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[async_trait::async_trait]
impl BehaviorModule for BioToneModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if !self.enabled {
            return Ok(ModuleOutcome::continue_());
        }
        if hook != HookPoint::TurnStart {
            // 真实事件源 (既有钩子面): 工具结算 / 运行时错误 / 工具名首见旁路推进
            // 调质状态。不产 overlay (overlay 只在回合起点装配), 指令恒为 Continue。
            self.record_hook_observation(hook, ctx.tool_call(), ctx.tool_result(), ctx.error);
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
    module: Arc<dyn BehaviorModule>,
}

impl BioToneWiring {
    /// 共享调质状态 (真实反馈事件源接上后从这条口注入)。
    #[must_use]
    pub fn state(&self) -> &Arc<ToneState> {
        &self.state
    }

    /// 待注册模块 (`RuntimeBuilder::with_module` / `ProductionModules` 同口)。
    #[must_use]
    pub fn module(&self) -> Arc<dyn BehaviorModule> {
        Arc::clone(&self.module)
    }
}

/// 开关值 → 模块 (开关关 = `None`, 不构造; 供宿主自带 [`ToneState`] 时使用)。
#[must_use]
pub fn bio_tone_module_from_env_value(
    raw: Option<&str>,
    state: Arc<ToneState>,
) -> Option<Arc<dyn BehaviorModule>> {
    bio_tone_enabled_from_env_value(raw)
        .then(|| Arc::new(BioToneModule::new(true, state)) as Arc<dyn BehaviorModule>)
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
    let module: Arc<dyn BehaviorModule> = Arc::new(BioToneModule::new(true, Arc::clone(&state)));
    Some(BioToneWiring { state, module })
}

/// 从 env 现值装配 (同 [`bio_tone_wiring_from_env_value`] 的 fail-closed 口径)。
#[must_use]
pub fn bio_tone_wiring_from_env(clock: ToneClock) -> Option<BioToneWiring> {
    bio_tone_wiring_from_env_value(std::env::var(BIO_TONE_ENABLE_ENV).ok().as_deref(), clock)
}

/// 审批面社会反馈映射 (W3.7 装配面真实源, 纯映射): 人批 → 社会正结果,
/// 人拒 → 社会负结果; 取消 = 收回请求, 不构成社会结论 (返回 `None`, 不编事件)。
///
/// 只做枚举映射, 不读文本 / 不判意图; 反馈档是工程标签, 不宣称主观情绪。
#[must_use]
pub fn approval_social_feedback(decision: &ApprovalDecision) -> Option<ToneFeedbackKind> {
    match decision {
        ApprovalDecision::Approve => Some(ToneFeedbackKind::SocialPositive),
        ApprovalDecision::Reject { .. } => Some(ToneFeedbackKind::SocialNegative),
        ApprovalDecision::Cancel { .. } => None,
    }
}

/// 把一次审批面社会结论推进共享调质状态 (句柄与结论都真存在时才推进; 返回是否推进)。
///
/// 开关关 = `state == None` = 不回调、不观察、零变化 (与未接线逐字节一致)。
pub fn apply_approval_social_feedback(
    state: Option<&Arc<ToneState>>,
    feedback: Option<ToneFeedbackKind>,
) -> bool {
    match (state, feedback) {
        (Some(state), Some(feedback)) => {
            state.apply_feedback(feedback);
            true
        }
        _ => false,
    }
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

    #[test]
    fn real_hook_events_map_to_the_documented_feedback_kinds_and_ignore_missing_payloads() {
        let ok = ToolResult::ok("call_ok", serde_json::Value::Null);
        let retryable = ToolResult::retryable_error("call_retry", "timeout");
        let permanent = ToolResult::permanent_error("call_perm", "bad argument");

        assert_eq!(
            tone_feedback_from_hook(HookPoint::AfterToolResult, Some(&ok), None),
            Some(ToneFeedbackKind::TaskSuccess),
            "工具成功 = TaskSuccess"
        );
        for failed in [&retryable, &permanent] {
            assert_eq!(
                tone_feedback_from_hook(HookPoint::AfterToolResult, Some(failed), None),
                Some(ToneFeedbackKind::TaskFailure),
                "工具失败 (可重试/不可重试) 同档 = TaskFailure"
            );
        }
        assert_eq!(
            tone_feedback_from_hook(HookPoint::OnError, None, Some("module hook failed")),
            Some(ToneFeedbackKind::TaskFailure),
            "运行时/模块错误 = TaskFailure"
        );

        // 缺载荷 = 无事件 (不从缺失数据编信号); 其它钩子不产生反馈。
        assert_eq!(
            tone_feedback_from_hook(HookPoint::AfterToolResult, None, None),
            None
        );
        assert_eq!(
            tone_feedback_from_hook(HookPoint::OnError, None, None),
            None
        );
        for hook in [
            HookPoint::TurnStart,
            HookPoint::BeforeModelCall,
            HookPoint::AfterModelResponse,
            HookPoint::BeforeToolCall,
            HookPoint::BeforeFinalCommit,
            HookPoint::AfterTurn,
        ] {
            assert_eq!(
                tone_feedback_from_hook(hook, Some(&ok), Some("ignored")),
                None,
                "{hook:?} 不是反馈事件源"
            );
        }

        // 开关关 = 零观察: 同一真实事件不推进状态。
        let state = baseline_state();
        let disabled = BioToneModule::new(false, Arc::clone(&state));
        assert!(disabled
            .record_hook_feedback(HookPoint::AfterToolResult, Some(&ok), None)
            .is_none());
        assert!(state.is_baseline(), "开关关 = 真实事件也不观察");

        // 开关开 = 真实事件一次投递一次回执 (单次事件未必移动离散档位:
        // 状态已变、overlay 仍等档位真的移动, 见零 overlay 语义)。
        let baseline_vector = state.vector();
        let enabled = BioToneModule::new(true, Arc::clone(&state));
        let receipt = enabled
            .record_hook_feedback(HookPoint::AfterToolResult, Some(&permanent), None)
            .expect("开关开 + 真实事件");
        assert_eq!(receipt.feedback, ToneFeedbackKind::TaskFailure);
        assert!(receipt.changed());
        assert_ne!(state.vector(), baseline_vector, "事件真的进了状态机");
    }

    #[test]
    fn first_sighting_of_a_tool_name_is_novelty_and_repeats_are_not() {
        let state = baseline_state();
        let module = BioToneModule::new(true, Arc::clone(&state));
        let call = ToolCall {
            id: "call_1".into(),
            name: "tool.weather".into(),
            arguments: serde_json::Value::Null,
        };
        let ok = ToolResult::ok("call_1", serde_json::Value::Null);

        let first = module.record_hook_observation(
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&ok),
            None,
        );
        assert_eq!(first.len(), 2, "结算 + 首见各一次脉冲");
        assert_eq!(first[0].feedback, ToneFeedbackKind::TaskSuccess);
        assert_eq!(first[1].feedback, ToneFeedbackKind::Novelty);
        assert!(first[1].changed(), "Novelty 事件真的进了状态机");

        let repeat = module.record_hook_observation(
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&ok),
            None,
        );
        assert_eq!(repeat.len(), 1, "重复工具名不再产生 Novelty");
        assert_eq!(repeat[0].feedback, ToneFeedbackKind::TaskSuccess);

        // 空名 = 无事件 (不从缺失数据编信号); 非 AfterToolResult 钩子即便带 tool_call 也不看名。
        let mut blank = call.clone();
        blank.name = "   ".into();
        let blank_receipts = module.record_hook_observation(
            HookPoint::AfterToolResult,
            Some(&blank),
            Some(&ok),
            None,
        );
        assert!(blank_receipts
            .iter()
            .all(|receipt| receipt.feedback != ToneFeedbackKind::Novelty));
        let before_hook =
            module.record_hook_observation(HookPoint::BeforeToolCall, Some(&call), None, None);
        assert!(before_hook.is_empty(), "非事件钩子不产生反馈");
    }

    #[test]
    fn novelty_source_is_zero_change_when_the_switch_is_off() {
        let state = baseline_state();
        let module = BioToneModule::new(false, Arc::clone(&state));
        let call = ToolCall {
            id: "call_1".into(),
            name: "tool.weather".into(),
            arguments: serde_json::Value::Null,
        };
        let ok = ToolResult::ok("call_1", serde_json::Value::Null);
        let receipts = module.record_hook_observation(
            HookPoint::AfterToolResult,
            Some(&call),
            Some(&ok),
            None,
        );
        assert!(receipts.is_empty());
        assert!(state.is_baseline(), "开关关 = 首见集不写、状态不动");
    }

    #[test]
    fn repeated_identical_events_are_not_deduplicated_and_are_bounded_by_the_clamp() {
        let state = baseline_state();
        let first = state.apply_feedback(ToneFeedbackKind::TaskFailure);
        assert!(first.changed(), "首次同向事件推进状态 (非幂等)");
        let second = state.apply_feedback(ToneFeedbackKind::TaskFailure);
        assert!(second.changed(), "重复投递 = 重复脉冲 (不去重)");

        let mut saturated = None;
        for _ in 0..256 {
            let receipt = state.apply_feedback(ToneFeedbackKind::TaskFailure);
            if !receipt.changed() {
                saturated = Some(receipt);
                break;
            }
        }
        let receipt = saturated.expect("持续同向事件必须到达 clamp 界");
        assert_eq!(
            state.vector(),
            receipt.after,
            "clamp 后同向事件不再改变状态 (有界)"
        );
        assert!(!state.is_baseline());
    }
}
