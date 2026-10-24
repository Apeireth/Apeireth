//! Governance hook implementation for Two-Stage Behavior-Chain Safety Classifier.
//!
//! # 免疫记忆接线（生物机制族 W3.1，默认关）
//!
//! [`BehaviorChainGuardHook::with_immune_memory`] 把 [`crate::immune_memory::ImmuneMemory`]
//! 挂到 hook 判定出口。**未挂载（字段默认 `None`）时判定逐字节不变**：不观察、不写记忆、
//! 不追加 evidence、不追加 verdict metadata。挂载后只在 `Deny` / `RequireApproval` 出口
//! 观察一次，并把 [`ImmuneResponse`] 作为**附加分级信号**写入三个旁路输出：
//!
//! - `GuardDecision.evidence` 追加一行
//!   `immune: kind=<kind> tier=<tier> escalated=<bool> record=<key>`；
//! - `GovernanceVerdict.metadata` 追加 `immune_kind` / `immune_tier` /
//!   `immune_escalated` / `immune_record_key` / `immune_reason`；
//! - [`BehaviorChainGuardHook::last_immune_signal`] 可取回本 trace 最近一次的
//!   [`ImmuneSignal`]（含抗原签名，便于装配层 / 运维执行 `resolve`）。
//!
//! **主结论不变**：`allow` / `deny` / `require_approval` 以及 `risk_score`、`stage`、
//! `reasons` 都不被免疫分级改写；`Tier0Quiet` 不等于 allow，`Tier3Lockdown` 也不会自行
//! 升级为 deny。免疫 tier 只回答「这是同类事件的第几次、是否值得加重审计」，放行 / 拦截
//! 仍由既有 fast / chain / intent / fusion 决定。`Allow` 出口不观察（避免正常流量预置记忆，
//! 使后续真实事件被误判成「已知模式」而降级）；`dry_run` 也不观察（试算不得污染记忆）。
//!
//! ## incident kind 映射表（按优先级，首个命中）
//!
//! | 优先级 | hook 既有特征 | `IncidentKind` |
//! | --- | --- | --- |
//! | 1 | `may_access_credentials`，或 `data_sensitivity ∈ {Credential, Secret}`，或 source 含 `CredentialStore` | `CredentialLeak` |
//! | 2 | 绑定了 turn intent 且 `alignment.score >= 0.65`（动作与可信意图相悖） | `PromptInjection` |
//! | 3 | sink 含 `ExternalNetwork`，或 operation 含 `NetworkSend` / `Publish` | `EgressAnomaly` |
//! | 4 | source 含 `ExternalNetwork` / `ToolOutput` 且 sink 含 `MemoryWrite` / `SystemFile` / `ModelCompletion` / `InternalNetwork` | `UntrustedContentEscape` |
//! | 5 | `descriptor_source ∈ {FallbackHeuristic, AdapterInferred, Unknown}`（工具语义面不可信） | `ToolPoison` |
//! | 6 | 其余 | `Unknown` |
//!
//! 映射是**粗粒度启发式**：本 hook 没有独立的注入 / 投毒 / 出网检测器，只能从既有
//! 归一化观察特征近似。装配层若接入更强检测器，应在 W3.2 直接给出 kind，而不是依赖本表。
//!
//! ## raw_key（签名输入）
//!
//! `capability=<capability_id> effect=<effect_fingerprint> sensitivity=<DataSensitivity>`。
//! 三项都是 hook 既有稳定特征：能力 id 来自 dispatcher；`effect_fingerprint` 由
//! `semantics.rs` 从能力描述符生成（与具体文件 / URL 参数无关）；敏感级来自描述符。
//! 因此签名粒度是「能力族 × 效应形态」，不是单次实例；`normalize` 会再消掉数字、路径、
//! 零宽字符与大小写噪声。
//!
//! ## is_self_origin
//!
//! hook 内没有 self 来源信息，生产路径一律传 `false`（self 耐受不生效）。装配层可从本机 /
//! 自身进程 / 已授权内部路径判定 self 来源并在调用 `observe` 前注入；本 crate 把该参数留在
//! 内部接缝 [`observe_incident`] 上，供 W3.2 装配时替换。
//!
//! ## 装配边界（诚实口径）
//!
//! 本改动是 **IMPLEMENTED + crate 内接线**：`apeireth-guard` 内部可构造、可测试、可调用；
//! 但 `runtime-assembly` 尚未创建 / 注入 `Arc<ImmuneMemory>`（W3.2 装配点），默认开关
//! 仍是关。对外能力级别是 **IMPLEMENTED（默认关、未装配）**，不是 PRODUCTION WIRED，
//! 更不是 DEFAULT ENABLED。

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use apeireth_core::kernel::SessionId;
use apeireth_governance::{
    Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict, OperationClass,
    TaskIntentEnvelopeV1,
};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::chain::{ActionStatus, BehaviorChain};
use crate::chain_guard::ChainGuard;
use crate::classifier::{ChainRiskClassifier, NoClassifier};
use crate::dataset::DatasetRecorder;
use crate::decision::GuardDecision;
use crate::fast_guard::FastGuard;
use crate::features_v2::AgentChainFeatureV2;
use crate::fusion::DecisionFusion;
use crate::immune_memory::{
    normalize, ImmuneMemory, ImmuneResponse, IncidentKind, IncidentSignature, SeverityTier,
};
use crate::intent::{AlignmentAssessment, IntentAlignmentGuard};
use crate::introspection::{
    GuardDryRunRequest, GuardDryRunResponse, GuardEventDto, GuardStatusDto,
};
use crate::observation::{DataSensitivity, SafetyObservation, SinkClass, SourceClass};
use crate::semantics::{
    CapabilitySafetyMetadataProvider, CapabilitySafetyRegistry, DescriptorSource,
};
use crate::session::{SessionBehaviorHistory, TurnBehaviorSummary};
use crate::snapshot::FeatureSnapshot;

const MAX_RECENT_EVENTS: usize = 200;
/// 同时驻留的行为链上限 (按 trace 计)。超出后按插入顺序 LRU 驱逐最久
/// 未命中的链; 正在评估的链由回写阶段重新插回, 不会丢本轮的 action。
const MAX_ACTIVE_CHAINS: usize = 256;
/// 同时驻留的**特征快照**上限 (按 (会话, 动作) 计)。
///
/// 快照每次评估都写一份完整的 `FeatureSnapshot`，而 guard hook 是
/// **长生命周期单例** ⇒ 没有上限就按「动作数」无界增长（一次长会话就能吃掉
/// 大量内存）。这里给一个有界上限：写新快照时若超量，按**插入顺序**淘汰最旧的。
///
/// 注：`clear_session` 也会清这份表（见其实现），但**会话永不结束**时
/// 仍要有上限兜底 —— 两者缺一不可。
const MAX_LAST_SNAPSHOTS: usize = 1024;

/// 共享、可内部修改的免疫记忆句柄（hook 与装配层 / 运维共用同一份）。
pub type SharedImmuneMemory = Arc<Mutex<ImmuneMemory>>;

/// 一次免疫观察的分级信号：挂在 guard hook 出口的附加字段。
///
/// 它不参与 [`GuardDecision::decision`] 的构造，只作为分级 / 审计旁路信息存在：
/// [`Self::tier`] 是免疫记忆给出的响应强度，[`Self::escalated`] 表示本条是否为
/// 「新签名 + 高严重度」的升级观察，[`Self::signature`] 可用于后续 `resolve`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImmuneSignal {
    /// 映射后的事件类别。
    pub kind: IncidentKind,
    /// 归一化后的抗原签名（与 `response.record_key` 一致）。
    pub signature: IncidentSignature,
    /// 免疫内核返回的原始响应。
    pub response: ImmuneResponse,
}

impl ImmuneSignal {
    /// 建议的响应分级。
    pub fn tier(&self) -> SeverityTier {
        self.response.tier
    }

    /// 是否为「新签名 + 高严重度」的升级观察。
    pub fn escalated(&self) -> bool {
        self.response.escalated
    }

    /// 记录键，形如 `credential_leak:1a2b3c4d5e6f7a8b`。
    pub fn record_key(&self) -> &str {
        &self.response.record_key
    }

    /// 分级理由（中文，直接用于审计 / 可观测）。
    pub fn reason(&self) -> &str {
        &self.response.reason
    }

    /// 追加到 `GuardDecision.evidence` 的稳定单行表示。
    fn evidence_line(&self) -> String {
        format!(
            "immune: kind={} tier={} escalated={} record={}",
            self.kind.as_str(),
            tier_label(self.response.tier),
            self.response.escalated,
            self.response.record_key
        )
    }

    /// 追加到 `GovernanceVerdict.metadata` 的旁路字段。
    fn annotate_verdict(&self, verdict: &mut GovernanceVerdict) {
        verdict.metadata.insert("immune_kind", self.kind.as_str());
        verdict
            .metadata
            .insert("immune_tier", tier_label(self.response.tier));
        verdict
            .metadata
            .insert("immune_escalated", self.response.escalated.to_string());
        verdict
            .metadata
            .insert("immune_record_key", self.response.record_key.clone());
        verdict
            .metadata
            .insert("immune_reason", self.response.reason.clone());
    }
}

/// 稳定的 tier 字符串标识（用于 evidence / verdict，不随 Debug 表示漂移）。
const fn tier_label(tier: SeverityTier) -> &'static str {
    match tier {
        SeverityTier::Tier0Quiet => "tier0_quiet",
        SeverityTier::Tier1Note => "tier1_note",
        SeverityTier::Tier2Guard => "tier2_guard",
        SeverityTier::Tier3Lockdown => "tier3_lockdown",
    }
}

/// hook 既有特征 → 免疫事件类别的映射（实现模块文档中的优先级表）。
fn classify_incident(
    obs: &SafetyObservation,
    alignment: &AlignmentAssessment,
    has_intent: bool,
) -> IncidentKind {
    let credential_origin = obs.may_access_credentials
        || matches!(
            obs.data_sensitivity,
            DataSensitivity::Credential | DataSensitivity::Secret
        )
        || obs
            .source_classes
            .iter()
            .any(|source| matches!(source, SourceClass::CredentialStore));
    if credential_origin {
        return IncidentKind::CredentialLeak;
    }

    if has_intent && alignment.score >= 0.65 {
        return IncidentKind::PromptInjection;
    }

    let network_sink = obs
        .sink_classes
        .iter()
        .any(|sink| matches!(sink, SinkClass::ExternalNetwork))
        || obs.operation_classes.iter().any(|operation| {
            matches!(
                operation,
                OperationClass::NetworkSend | OperationClass::Publish
            )
        });
    if network_sink {
        return IncidentKind::EgressAnomaly;
    }

    let untrusted_source = obs.source_classes.iter().any(|source| {
        matches!(
            source,
            SourceClass::ExternalNetwork | SourceClass::ToolOutput
        )
    });
    let trusted_sink = obs.sink_classes.iter().any(|sink| {
        matches!(
            sink,
            SinkClass::MemoryWrite
                | SinkClass::SystemFile
                | SinkClass::ModelCompletion
                | SinkClass::InternalNetwork
        )
    });
    if untrusted_source && trusted_sink {
        return IncidentKind::UntrustedContentEscape;
    }

    if matches!(
        obs.descriptor_source,
        DescriptorSource::FallbackHeuristic
            | DescriptorSource::AdapterInferred
            | DescriptorSource::Unknown
    ) {
        return IncidentKind::ToolPoison;
    }

    IncidentKind::Unknown
}

/// 免疫签名输入：不含参数正文、不含实例噪声的稳定特征三元组。
fn immune_raw_key(obs: &SafetyObservation) -> String {
    format!(
        "capability={} effect={} sensitivity={:?}",
        obs.capability_id, obs.effect_fingerprint, obs.data_sensitivity
    )
}

/// 主结论 → 免疫严重度提示：拒绝 = 高（新签名会再升一级），审批 = 低。
///
/// `Allow` 出口不观察，`Tier0Quiet` 只是让函数保持全序、不作为生产观察值。
fn severity_hint(decision: &Decision) -> SeverityTier {
    match decision {
        Decision::Deny { .. } => SeverityTier::Tier2Guard,
        Decision::RequireApproval { .. } => SeverityTier::Tier1Note,
        Decision::Allow => SeverityTier::Tier0Quiet,
    }
}

/// 判定出口的免疫观察接缝。
///
/// 生产路径由 [`BehaviorChainGuardHook::evaluate_internal`] 调用并传
/// `is_self_origin = false`（hook 内没有 self 来源信息）；装配层将来若能判定
/// self 来源，可在 W3.2 改为传入 `true`，无需改动免疫内核。
fn observe_incident(
    memory: &mut ImmuneMemory,
    obs: &SafetyObservation,
    decision: &GuardDecision,
    alignment: &AlignmentAssessment,
    has_intent: bool,
    is_self_origin: bool,
    now_ms: i64,
) -> ImmuneSignal {
    let kind = classify_incident(obs, alignment, has_intent);
    let raw_key = immune_raw_key(obs);
    let signature = normalize(kind, &raw_key);
    let response = memory.observe(
        kind,
        &raw_key,
        severity_hint(&decision.decision),
        is_self_origin,
        now_ms,
    );
    ImmuneSignal {
        kind,
        signature,
        response,
    }
}

fn apply_alignment(mut decision: GuardDecision, alignment: &AlignmentAssessment) -> GuardDecision {
    decision.risk_score = decision.risk_score.max(alignment.score);
    for reason in &alignment.reasons {
        if !decision.reasons.contains(reason) {
            decision.reasons.push(reason.clone());
        }
    }
    if !alignment.reasons.is_empty() {
        decision.evidence.push(format!(
            "alignment={:?} score={:.2}",
            alignment.class, alignment.score
        ));
    }
    if matches!(decision.decision, Decision::Deny { .. }) {
        return decision;
    }
    if alignment.score >= 0.85 {
        decision.decision = Decision::deny("intent and action alignment is high risk");
        decision.stage = crate::decision::GuardStage::ChainGuard;
    } else if alignment.score >= 0.65 {
        decision.decision = Decision::require_approval("action expands the trusted task intent");
        decision.stage = crate::decision::GuardStage::ChainGuard;
    }
    decision
}

#[derive(Debug, Default, Clone)]
struct GuardCounters {
    total_evaluations: u64,
    total_allowed: u64,
    total_denied: u64,
    total_approval_required: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SessionBehaviorSummary {
    pub recent_turns: u32,
    pub denied_action_count: u32,
    pub approval_rejection_count: u32,
    pub credential_probe_count: u32,
    pub sensitive_read_count: u32,
    pub network_egress_count: u32,
    pub repeated_scope_expansion_count: u32,
    pub repeated_alternate_tool_count: u32,
    pub risk_trend: f64,
}

/// Summary of turn-level risk evaluation preserved across turns for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnRiskSummary {
    pub trace_id: String,
    pub max_risk_score: f64,
    pub denied: bool,
    pub timestamp_ms: i64,
}

/// Bounded risk history across turns for a session (max 10 items).
#[derive(Debug, Clone, Default)]
pub struct SessionRiskHistory {
    pub summaries: VecDeque<TurnRiskSummary>,
}

impl SessionRiskHistory {
    pub fn push(&mut self, summary: TurnRiskSummary) {
        if self.summaries.len() >= 10 {
            self.summaries.pop_front();
        }
        self.summaries.push_back(summary);
    }
}

/// Canonical governance hook evaluating agent behavior chains across two stages.
pub struct BehaviorChainGuardHook {
    fast_guard: FastGuard,
    chain_guard: ChainGuard,
    chains: Mutex<HashMap<(SessionId, String), BehaviorChain>>,
    session_scopes: Mutex<HashMap<SessionId, String>>,
    session_risk_history: Mutex<HashMap<SessionId, SessionRiskHistory>>,
    session_behavior_summary: Mutex<HashMap<SessionId, SessionBehaviorSummary>>,
    session_history: Mutex<HashMap<SessionId, SessionBehaviorHistory>>,
    turn_intents: Mutex<HashMap<(SessionId, String), TaskIntentEnvelopeV1>>,
    last_snapshots: Mutex<HashMap<(SessionId, String), FeatureSnapshot>>,
    /// `last_snapshots` 的**插入序**队列（先进先出）。
    ///
    /// 快照本身没有时间戳字段，所以「最旧」记在这里 —— 与 `chain_order`
    /// 同一套路。队列长度与 map 同步，超量时从队首淘汰。
    last_snapshot_order: Mutex<std::collections::VecDeque<(SessionId, String)>>,
    /// 插入顺序 LRU 队列, 与 `chains` 同源维护: 每次命中把 key 移到队尾,
    /// 超限时从队首驱逐, 使驱逐语义 (最久未使用) 与注释一致 —— 旧实现
    /// 直接 `chains.keys().next()` (`HashMap` 任意键序), 注释说的顺序驱逐
    /// 并未成立。
    chain_order: Mutex<VecDeque<(SessionId, String)>>,
    dataset_recorder: Option<Arc<DatasetRecorder>>,
    classifier: Arc<dyn ChainRiskClassifier>,
    safety_provider: Arc<dyn CapabilitySafetyMetadataProvider>,
    recent_events: Mutex<VecDeque<GuardEventDto>>,
    counters: Mutex<GuardCounters>,
    /// 免疫记忆（默认 `None` = 关闭）。挂载后只在风险 / 升级出口观察并追加分级信号。
    immune_memory: Option<SharedImmuneMemory>,
    /// 每个 `(session, trace)` 最近一次的免疫信号，供可观测 / 测试取回。
    last_immune_signals: Mutex<HashMap<(SessionId, String), ImmuneSignal>>,
}

impl Default for BehaviorChainGuardHook {
    fn default() -> Self {
        Self::new()
    }
}

impl BehaviorChainGuardHook {
    /// Create a new behavior chain guard hook.
    pub fn new() -> Self {
        Self {
            fast_guard: FastGuard::new(),
            chain_guard: ChainGuard::new(),
            chains: Mutex::new(HashMap::new()),
            session_scopes: Mutex::new(HashMap::new()),
            session_risk_history: Mutex::new(HashMap::new()),
            session_behavior_summary: Mutex::new(HashMap::new()),
            session_history: Mutex::new(HashMap::new()),
            turn_intents: Mutex::new(HashMap::new()),
            last_snapshots: Mutex::new(HashMap::new()),
            last_snapshot_order: Mutex::new(std::collections::VecDeque::new()),
            chain_order: Mutex::new(VecDeque::new()),
            dataset_recorder: None,
            classifier: Arc::new(NoClassifier),
            safety_provider: Arc::new(CapabilitySafetyRegistry::canonical()),
            recent_events: Mutex::new(VecDeque::with_capacity(MAX_RECENT_EVENTS)),
            counters: Mutex::new(GuardCounters::default()),
            immune_memory: None,
            last_immune_signals: Mutex::new(HashMap::new()),
        }
    }

    /// 挂载免疫记忆（默认关）。
    ///
    /// 传入的 [`Arc<ImmuneMemory>`] 在唯一持有时被移入 hook；若调用方仍持有其它
    /// clone，则移入一份快照（之后的外部变更不影响 hook）。需要在 hook 外执行
    /// `resolve` / `reopen` 时，用 [`Self::shared_immune_memory`] 取回同一个句柄。
    ///
    /// 挂载后行为只增不改：allow/deny 主结论、risk_score、reasons、stage 保持不变，
    /// 仅在 `Deny` / `RequireApproval` 出口追加 `immune:` evidence 与 verdict 字段。
    pub fn with_immune_memory(mut self, memory: Arc<ImmuneMemory>) -> Self {
        let owned = Arc::try_unwrap(memory).unwrap_or_else(|shared| (*shared).clone());
        self.immune_memory = Some(Arc::new(Mutex::new(owned)));
        self
    }

    /// 是否已挂载免疫记忆（默认 `false`）。
    pub fn immune_memory_enabled(&self) -> bool {
        self.immune_memory.is_some()
    }

    /// 取回 hook 正在使用的免疫记忆句柄（未挂载时为 `None`）。
    ///
    /// 装配层 / 运维用它执行 [`ImmuneMemory::resolve`] / [`ImmuneMemory::reopen`]，
    /// 与 hook 内的观察共享同一份状态。
    pub fn shared_immune_memory(&self) -> Option<SharedImmuneMemory> {
        self.immune_memory.clone()
    }

    /// 取回某 session + trace 最近一次的免疫信号（未挂载 / 未观察时为 `None`）。
    pub fn last_immune_signal(
        &self,
        session_id: &SessionId,
        trace_id: &str,
    ) -> Option<ImmuneSignal> {
        self.last_immune_signals
            .lock()
            .get(&(*session_id, trace_id.to_string()))
            .cloned()
    }

    /// Set an optional dataset recorder for offline ML dataset collection.
    pub fn with_dataset_recorder(mut self, recorder: Arc<DatasetRecorder>) -> Self {
        self.dataset_recorder = Some(recorder);
        self
    }

    /// Return the composition-owned recorder so a runtime event observer can
    /// be attached to the same dataset sink without creating a second file
    /// writer.
    pub fn dataset_recorder(&self) -> Option<Arc<DatasetRecorder>> {
        self.dataset_recorder.clone()
    }

    /// Install an optional local classifier. The default remains unavailable,
    /// preserving the deterministic Fast/Chain Guard path.
    pub fn with_classifier(mut self, classifier: Arc<dyn ChainRiskClassifier>) -> Self {
        self.classifier = classifier;
        self
    }

    pub fn with_safety_provider(
        mut self,
        provider: Arc<dyn CapabilitySafetyMetadataProvider>,
    ) -> Self {
        self.safety_provider = provider;
        self
    }

    pub fn last_feature_snapshot(
        &self,
        session_id: &SessionId,
        action_id: &str,
    ) -> Option<FeatureSnapshot> {
        self.last_snapshots
            .lock()
            .get(&(*session_id, action_id.to_string()))
            .cloned()
    }

    pub fn session_history(&self, session_id: &SessionId) -> Vec<TurnBehaviorSummary> {
        self.session_history
            .lock()
            .get(session_id)
            .map(|history| history.turns.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn update_action_execution(
        &self,
        session_id: &SessionId,
        trace_id: &str,
        action_id: &str,
        succeeded: bool,
    ) {
        if let Some(chain) = self
            .chains
            .lock()
            .get_mut(&(*session_id, trace_id.to_string()))
        {
            chain.update_action_status(
                action_id,
                if succeeded {
                    ActionStatus::Succeeded
                } else {
                    ActionStatus::Failed
                },
            );
        }
    }

    /// Evaluate one request and retain the structured guard decision for
    /// callers that need an auditable action-level result.
    pub fn evaluate_detailed(&self, request: &GovernanceRequest<'_>) -> GuardDecision {
        let (decision, _) = self.evaluate_internal(request);
        decision
    }

    /// Set the declared task scope for a given session.
    pub fn set_declared_scope(&self, session_id: &SessionId, scope: impl Into<String>) {
        let scope_str = scope.into();
        self.session_scopes.lock().insert(*session_id, scope_str);
    }

    /// Bind a trusted envelope before the provider starts tool use.
    pub fn bind_turn_intent(
        &self,
        session_id: SessionId,
        trace_id: impl Into<String>,
        intent: TaskIntentEnvelopeV1,
    ) {
        self.turn_intents
            .lock()
            .insert((session_id, trace_id.into()), intent);
    }

    pub fn turn_intent(
        &self,
        session_id: &SessionId,
        trace_id: &str,
    ) -> Option<TaskIntentEnvelopeV1> {
        self.turn_intents
            .lock()
            .get(&(*session_id, trace_id.to_string()))
            .cloned()
    }

    pub fn session_behavior_summary(&self, session_id: &SessionId) -> SessionBehaviorSummary {
        self.session_behavior_summary
            .lock()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Retrieve a cloned snapshot of a behavior chain for a specific session and trace.
    pub fn chain_for_trace(&self, session_id: &SessionId, trace_id: &str) -> Option<BehaviorChain> {
        self.chains
            .lock()
            .get(&(*session_id, trace_id.to_string()))
            .cloned()
    }

    /// Retrieve bounded risk history for a session.
    pub fn session_risk_history(&self, session_id: &SessionId) -> Vec<TurnRiskSummary> {
        self.session_risk_history
            .lock()
            .get(session_id)
            .map(|h| h.summaries.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Retrieve the current status summary for desktop telemetry and gateway queries.
    pub fn status(&self) -> GuardStatusDto {
        let chains = self.chains.lock();
        let counters = self.counters.lock();
        let rec_enabled = self
            .dataset_recorder
            .as_ref()
            .map(|r| r.is_enabled())
            .unwrap_or(false);

        GuardStatusDto {
            enabled: true,
            fast_guard_active: true,
            chain_guard_active: true,
            intent_guard_active: true,
            cross_turn_monitoring_active: true,
            active_chains: chains.len(),
            total_evaluations: counters.total_evaluations,
            total_allowed: counters.total_allowed,
            total_denied: counters.total_denied,
            total_approval_required: counters.total_approval_required,
            dataset_recording_enabled: rec_enabled,
            ml_classifier_available: self.classifier.available(),
            ml_model_version: self.classifier.model_version(),
            ml_mode: self.classifier.enforcement_mode().as_str().to_string(),
            ml_reason: self.classifier.model_reason(),
            feature_schema_version: crate::features_v2::AGENT_CHAIN_FEATURE_V2.to_string(),
            dataset_version: crate::dataset::GUARD_DATASET_V3.to_string(),
        }
    }

    /// Retrieve the recent evaluation events (newest first).
    pub fn recent_events(&self, limit: Option<usize>) -> Vec<GuardEventDto> {
        let events = self.recent_events.lock();
        let limit = limit.unwrap_or(50).min(events.len());
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Clear memory for a completed session.
    pub fn clear_session(&self, session_id: &SessionId) {
        let mut chains = self.chains.lock();
        chains.retain(|(s, _), _| s != session_id);
        // LRU 队列与 chains 同源: 一并清理, 否则队列里留下永不命中的幽灵
        // key, 会挤占 MAX_ACTIVE_CHAINS 额度并驱逐活跃链。
        self.chain_order.lock().retain(|(s, _)| s != session_id);
        self.session_scopes.lock().remove(session_id);
        self.session_risk_history.lock().remove(session_id);
        self.session_behavior_summary.lock().remove(session_id);
        self.session_history.lock().remove(session_id);
        self.turn_intents
            .lock()
            .retain(|(session, _), _| session != session_id);
        self.last_immune_signals
            .lock()
            .retain(|(session, _), _| session != session_id);
        // 特征快照同样按会话持有 —— 不清的话，会话结束后它的快照仍然驻留
        // （而 `last_snapshots` 的键是 (会话, 动作)，容量上限只是兜底）。
        self.last_snapshots
            .lock()
            .retain(|(session, _), _| session != session_id);
        self.last_snapshot_order
            .lock()
            .retain(|(session, _)| session != session_id);
    }

    /// Dry-run evaluate an action without permanently recording it in the chain.
    pub fn dry_run(&self, req: &GuardDryRunRequest) -> GuardDryRunResponse {
        let obs = SafetyObservation {
            trace_id: "dry_run".to_string(),
            request_id: "dry_run".to_string(),
            session_id: req
                .session_id
                .clone()
                .unwrap_or_else(|| "dry_run".to_string()),
            stage: "capability_dispatch".to_string(),
            capability_id: req.capability_id.clone(),
            tool_name: req.capability_id.clone(),
            resource_classes: Vec::new(),
            source_classes: Vec::new(),
            sink_classes: Vec::new(),
            permission_scope: req.capability_id.clone(),
            approval_state: "dry_run".to_string(),
            argument_shape: "dry_run".to_string(),
            redacted_argument_features: req.arguments.clone(),
            result_class: None,
            result_size_bucket: None,
            retry_count: 0,
            denied_before: false,
            prior_actions: Vec::new(),
            external_effect: false,
            operation_class: apeireth_governance::OperationClass::Unknown,
            operation_classes: vec![apeireth_governance::OperationClass::Unknown],
            descriptor_source: crate::semantics::DescriptorSource::Unknown,
            data_sensitivity: crate::observation::DataSensitivity::Unknown,
            persistent_effect: false,
            destructive_effect: false,
            requires_network: false,
            may_access_credentials: false,
            effect_fingerprint: "dry_run".to_string(),
        };

        let mut temp_chain = BehaviorChain::new("dry_run", "dry_run");
        if let Some(intent) = req.intent.clone() {
            temp_chain.set_intent(intent);
        }
        let fast_res = self
            .fast_guard
            .evaluate(&obs, req.declared_scope.as_deref());
        let decision = if fast_res.clear {
            GuardDecision::allow_fast()
        } else {
            self.chain_guard.evaluate(&temp_chain, &obs, &fast_res)
        };
        let features = AgentChainFeatureV2::from_chain(&temp_chain);
        let prediction = self.classifier.classify_v2(&features);
        let decision = DecisionFusion::apply(
            self.classifier.enforcement_mode(),
            &decision,
            &fast_res,
            &prediction,
            &features,
        );

        GuardDryRunResponse {
            decision: decision.decision.label().to_string(),
            stage: decision.stage,
            risk_score: decision.risk_score,
            reasons: decision.reasons,
            evidence: decision.evidence,
        }
    }

    fn evaluate_internal(
        &self,
        request: &GovernanceRequest<'_>,
    ) -> (GuardDecision, GovernanceVerdict) {
        let trace_id = request.trace.to_string();
        let key = (request.session, trace_id.clone());

        // 快照阶段 (持锁): 全局 chains 锁内只做"取链 / 建链"与 LRU 记账。
        // 旧实现在持锁状态下跑完 fast guard、chain guard、fusion 与全部
        // 记账 —— 锁把不同会话的 guard 评估串行化, 且与 update_action_execution
        // 等链写者相互阻塞。评估改在锁外对快照进行, 结束后再回写。
        let mut chain = {
            let mut chains = self.chains.lock();
            let intent = request
                .security_context
                .and_then(|context| {
                    if context.trace_id.is_empty() || context.trace_id == trace_id {
                        context.intent.clone().map(|mut intent| {
                            intent.trace_id = trace_id.clone();
                            intent.session_id = request.session.to_string();
                            intent
                        })
                    } else {
                        None
                    }
                })
                .or_else(|| self.turn_intent(&request.session, &trace_id));
            if let Some(intent) = intent.clone() {
                self.turn_intents
                    .lock()
                    .insert((request.session, trace_id.clone()), intent);
            }
            // 插入顺序 LRU: 命中即移到队尾; 超限从队首驱逐最久未使用的链。
            // 被驱逐链可能是并发评估中的目标, 回写阶段会重新插回。
            {
                let mut order = self.chain_order.lock();
                if let Some(position) = order.iter().position(|entry| *entry == key) {
                    order.remove(position);
                }
                order.push_back(key.clone());
                while order.len() > MAX_ACTIVE_CHAINS {
                    match order.pop_front() {
                        Some(evicted) => {
                            chains.remove(&evicted);
                        }
                        None => break,
                    }
                }
            }
            let chain = chains.entry(key.clone()).or_insert_with(|| {
                let mut c = BehaviorChain::new(request.session.to_string(), trace_id.clone());
                if let Some(scope) = self.session_scopes.lock().get(&request.session) {
                    c.set_declared_scope(scope.clone());
                }
                if let Some(intent) = intent.clone() {
                    c.set_intent(intent);
                }
                c
            });
            if chain.intent.is_none() {
                if let Some(intent) = intent {
                    chain.set_intent(intent);
                }
            }
            chain.clone()
        };

        // Calculate retry stats and prior actions
        let actions = chain.actions();
        let denied_before = actions.iter().any(|a| a.denied);
        let retry_count = actions.iter().rev().take_while(|a| a.denied).count() as u32;
        let prior_actions = actions.iter().map(|a| a.capability_id.clone()).collect();

        // Extract normalized safety observation
        let obs = SafetyObservation::from_governance_request_with_provider(
            request,
            retry_count,
            denied_before,
            prior_actions,
            Some(self.safety_provider.as_ref()),
        );

        // Add action to behavior chain
        let action_id = chain.add_action_with_id(&obs, request.round, request.action_id);

        let alignment = IntentAlignmentGuard.evaluate(chain.intent.as_ref(), &obs);
        chain.set_action_alignment(&action_id, alignment.class);

        // Stage A: Fast Guard
        let fast_res = self
            .fast_guard
            .evaluate(&obs, chain.declared_task_scope.as_deref());

        // Stage B: Chain Guard (or early allow)
        let mut guard_decision = if fast_res.clear {
            GuardDecision::allow_fast()
        } else {
            self.chain_guard.evaluate(&chain, &obs, &fast_res)
        };
        guard_decision = apply_alignment(guard_decision, &alignment);
        let cross_turn = self
            .session_history
            .lock()
            .entry(request.session)
            .or_default()
            .cross_turn_features(&trace_id);
        if cross_turn.sensitive_probe_turn_count >= 2
            && (obs.may_access_credentials
                || matches!(
                    obs.data_sensitivity,
                    crate::observation::DataSensitivity::Credential
                        | crate::observation::DataSensitivity::Secret
                ))
        {
            guard_decision.decision = Decision::deny("repeated sensitive probing across turns");
            guard_decision.risk_score = guard_decision.risk_score.max(0.95);
            guard_decision
                .reasons
                .push("cross_turn_sensitive_probing".to_string());
            guard_decision.evidence.push(
                "bounded session summary observed repeated credential/environment probes"
                    .to_string(),
            );
            guard_decision.stage = crate::decision::GuardStage::ChainGuard;
        }
        let features = AgentChainFeatureV2::from_chain_with_cross_turn(&chain, cross_turn);
        let snapshot = FeatureSnapshot::capture(&trace_id, &action_id, features.clone());
        let prediction = self.classifier.classify_v2(&snapshot.features);
        let mut guard_decision = DecisionFusion::apply(
            self.classifier.enforcement_mode(),
            &guard_decision,
            &fast_res,
            &prediction,
            &snapshot.features,
        );

        // 免疫记忆接缝（默认关）：只在非 Allow 的风险 / 升级结论上观察一次。
        // 观察结果只写入旁路（evidence / last_immune_signals / verdict metadata），
        // 不参与 Decision 构造 —— 免疫 tier 不能放行或阻断动作。
        let immune_signal = match &self.immune_memory {
            Some(memory) if !guard_decision.decision.is_allowed() => {
                let signal = {
                    let mut memory = memory.lock();
                    // hook 内无 self 来源信息，生产路径固定传 false；
                    // 装配层将来可在此注入 self 判定（见模块文档）。
                    observe_incident(
                        &mut memory,
                        &obs,
                        &guard_decision,
                        &alignment,
                        chain.intent.is_some(),
                        false,
                        chrono::Utc::now().timestamp_millis(),
                    )
                };
                self.last_immune_signals
                    .lock()
                    .insert(key.clone(), signal.clone());
                Some(signal)
            }
            _ => None,
        };
        if let Some(signal) = &immune_signal {
            guard_decision.evidence.push(signal.evidence_line());
        }

        // Update action status in chain
        let status = match &guard_decision.decision {
            Decision::Allow => ActionStatus::Allowed,
            Decision::Deny { .. } => ActionStatus::Denied,
            Decision::RequireApproval { .. } => ActionStatus::RequireApproval,
        };
        chain.update_action_status(&action_id, status);

        // Record turn risk summary for session continuity (without leaking full DAG)
        {
            let mut risk_histories = self.session_risk_history.lock();
            let hist = risk_histories.entry(request.session).or_default();
            hist.push(TurnRiskSummary {
                trace_id: request.trace.to_string(),
                max_risk_score: guard_decision.risk_score,
                denied: matches!(guard_decision.decision, Decision::Deny { .. }),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
            });
        }

        {
            let mut histories = self.session_history.lock();
            let history = histories.entry(request.session).or_default();
            history.upsert(TurnBehaviorSummary::from_chain(
                &chain,
                &obs,
                guard_decision.risk_score,
                matches!(guard_decision.decision, Decision::Deny { .. }),
                matches!(guard_decision.decision, Decision::RequireApproval { .. }),
                chrono::Utc::now().timestamp_millis(),
            ));
            let mut summary = self.session_behavior_summary.lock();
            let state = summary.entry(request.session).or_default();
            let cross = history.cross_turn_features(&trace_id);
            state.recent_turns = history.recent_turn_count();
            state.denied_action_count = cross.denied_action_count
                + u32::from(matches!(guard_decision.decision, Decision::Deny { .. }));
            state.approval_rejection_count += u32::from(matches!(
                guard_decision.decision,
                Decision::RequireApproval { .. }
            ));
            state.credential_probe_count = history
                .turns
                .iter()
                .map(|turn| turn.credential_probe_count)
                .sum();
            state.sensitive_read_count = history
                .turns
                .iter()
                .map(|turn| turn.sensitive_read_count)
                .sum();
            state.network_egress_count = history
                .turns
                .iter()
                .map(|turn| turn.network_egress_count)
                .sum();
            state.repeated_scope_expansion_count = cross.repeated_scope_expansion_count;
            state.repeated_alternate_tool_count = cross.repeated_alternate_tool_count;
            state.risk_trend = cross.risk_trend;
        }

        // Update counters
        {
            let mut c = self.counters.lock();
            c.total_evaluations += 1;
            match &guard_decision.decision {
                Decision::Allow => c.total_allowed += 1,
                Decision::Deny { .. } => c.total_denied += 1,
                Decision::RequireApproval { .. } => c.total_approval_required += 1,
            }
        }

        // Record recent event for desktop observability
        {
            let event = GuardEventDto {
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                session_id: request.session.to_string(),
                trace_id: request.trace.to_string(),
                round: request.round,
                capability_id: obs.capability_id.clone(),
                stage: guard_decision.stage,
                decision: guard_decision.decision.label().to_string(),
                risk_score: guard_decision.risk_score,
                reasons: guard_decision.reasons.clone(),
                evidence: guard_decision.evidence.clone(),
            };

            let mut events = self.recent_events.lock();
            if events.len() >= MAX_RECENT_EVENTS {
                events.pop_front();
            }
            events.push_back(event);
        }

        // Record dataset record if recorder is configured
        if let Some(recorder) = &self.dataset_recorder {
            recorder.record_classification(
                &action_id,
                &obs,
                &chain,
                &fast_res,
                &guard_decision,
                &snapshot,
            );
        }
        {
            let key = (request.session, action_id.clone());
            let mut snapshots = self.last_snapshots.lock();
            snapshots.insert(key.clone(), snapshot);
            let mut order = self.last_snapshot_order.lock();
            order.push_back(key);
            // 有界：超量时按**插入顺序**淘汰最旧的。
            // 队首可能是同一 key 的旧记录（覆盖写），跳过它继续找。
            while order.len() > MAX_LAST_SNAPSHOTS {
                let Some(oldest) = order.pop_front() else {
                    break;
                };
                snapshots.remove(&oldest);
            }
        }

        // 回写阶段 (持锁): 把评估后的链快照写回表。评估期间该 trace 若被
        // 并发驱逐 (> MAX_ACTIVE_CHAINS 个 trace 同时活跃时可能发生), 重新
        // 插回, 保证本轮的 action/mutation 不丢。同一会话的 turn 由 runtime
        // 的 per-session 锁串行化, 正常情况下这里是同一条链的顺序更新。
        {
            let mut chains = self.chains.lock();
            match chains.get_mut(&key) {
                Some(entry) => *entry = chain,
                None => {
                    chains.insert(key, chain);
                }
            }
        }

        let mut verdict = guard_decision.to_verdict(self.name());
        if let Some(signal) = &immune_signal {
            signal.annotate_verdict(&mut verdict);
        }
        (guard_decision, verdict)
    }
}

#[async_trait]
impl GovernanceHook for BehaviorChainGuardHook {
    fn name(&self) -> &str {
        "behavior_chain_guard"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let (decision, _) = self.evaluate_internal(request);
        decision.decision
    }

    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let (_, verdict) = self.evaluate_internal(request);
        verdict
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, TraceId};
    use apeireth_governance::{Action, Decision};

    use crate::classifier::RiskPrediction;
    use crate::decision::GuardStage;
    use crate::intent::AlignmentClass;

    /// 构造一次能力派发请求；参数引用由调用方持有。
    fn dispatch<'a>(
        capability: &'a CapabilityId,
        arguments: &'a serde_json::Value,
        session: SessionId,
        trace: TraceId,
        round: u32,
        action_id: &'a str,
    ) -> GovernanceRequest<'a> {
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            session,
            trace,
            round,
        )
        .with_action_id(action_id)
    }

    /// 走真实语义描述符生成归一化观察（不手搓 40 个字段）。
    fn observation_for(capability_id: &str, arguments: &serde_json::Value) -> SafetyObservation {
        let capability =
            CapabilityId::new(capability_id).expect("test capability id must be valid");
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        let registry = CapabilitySafetyRegistry::canonical();
        SafetyObservation::from_governance_request_with_provider(
            &request,
            0,
            false,
            Vec::new(),
            Some(&registry),
        )
    }

    fn credential_arguments() -> serde_json::Value {
        serde_json::json!({ "path": ".env" })
    }

    fn credential_request<'a>(
        capability: &'a CapabilityId,
        arguments: &'a serde_json::Value,
        session: SessionId,
        trace: TraceId,
        round: u32,
        action_id: &'a str,
    ) -> GovernanceRequest<'a> {
        dispatch(capability, arguments, session, trace, round, action_id)
    }

    fn mounted_hook() -> BehaviorChainGuardHook {
        BehaviorChainGuardHook::new().with_immune_memory(Arc::new(ImmuneMemory::with_capacity(32)))
    }

    fn aligned_assessment() -> AlignmentAssessment {
        AlignmentAssessment {
            class: AlignmentClass::Aligned,
            score: 0.0,
            reasons: Vec::new(),
        }
    }

    /// 特征快照表**有界**：连续评估超过上限后，最旧的被淘汰
    /// （guard hook 是长生命周期单例 ⇒ 没有上限就按「动作数」无界增长）。
    #[test]
    fn feature_snapshot_store_is_bounded() {
        let hook = BehaviorChainGuardHook::new();
        let session = SessionId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();
        let trace = TraceId::new();

        // 超过上限 + 余量：上限是 1024，跑 1100 次足够触发淘汰。
        for index in 0..(MAX_LAST_SNAPSHOTS + 76) {
            let action_id = format!("action-{index}");
            let request =
                credential_request(&capability, &arguments, session, trace, 1, &action_id);
            let _ = hook.evaluate_detailed(&request);
        }

        let stored = hook.last_snapshots.lock();
        assert!(
            stored.len() <= MAX_LAST_SNAPSHOTS,
            "快照表必须有界，实际 {} 条（上限 {MAX_LAST_SNAPSHOTS}）",
            stored.len()
        );
        // 最早的应该已被淘汰，最新的还在。
        assert!(
            !stored.contains_key(&(session, "action-0".to_string())),
            "最旧的快照应被淘汰"
        );
        assert!(
            stored.contains_key(&(session, format!("action-{}", MAX_LAST_SNAPSHOTS + 75))),
            "最新的快照应保留"
        );
    }

    /// `clear_session` 也要清特征快照 —— 否则会话结束后它的快照仍然驻留。
    #[test]
    fn clear_session_drops_feature_snapshots() {
        let hook = BehaviorChainGuardHook::new();
        let session = SessionId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();
        let trace = TraceId::new();
        let request = credential_request(&capability, &arguments, session, trace, 1, "a1");
        let _ = hook.evaluate_detailed(&request);
        assert!(
            hook.last_feature_snapshot(&session, "a1").is_some(),
            "评估后应留下快照"
        );

        hook.clear_session(&session);
        assert!(
            hook.last_feature_snapshot(&session, "a1").is_none(),
            "会话结束后快照必须清掉"
        );
    }

    #[test]
    fn default_off_decision_matches_pinned_baseline() {
        let hook = BehaviorChainGuardHook::new();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();
        let request = credential_request(
            &capability,
            &arguments,
            session,
            trace,
            1,
            "baseline-action",
        );

        let decision = hook.evaluate_detailed(&request);
        let expected = GuardDecision {
            decision: Decision::deny("guard chain denied: dotfile_or_hidden_resource_access"),
            risk_score: 0.85,
            reasons: vec![
                "credential_store_access".to_string(),
                "dotfile_or_hidden_resource_access".to_string(),
                "turn_intent_unavailable".to_string(),
            ],
            evidence: vec!["alignment=ScopeExpansion score=0.75".to_string()],
            stage: GuardStage::ChainGuard,
            classifier_prediction: Some(RiskPrediction::unavailable()),
        };
        assert_eq!(
            serde_json::to_string(&decision).expect("decision serializes"),
            serde_json::to_string(&expected).expect("baseline serializes"),
            "默认关路径必须与接线前的 pinned 判定逐字节一致"
        );
        assert!(
            decision
                .evidence
                .iter()
                .all(|line| !line.starts_with("immune:")),
            "默认关不得追加 immune evidence"
        );
        assert!(hook
            .last_immune_signal(&session, &trace.to_string())
            .is_none());
        assert!(!hook.immune_memory_enabled());
    }

    #[test]
    fn default_off_never_touches_immune_outputs() {
        let hook = BehaviorChainGuardHook::new();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();
        let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

        let (decision, verdict) = hook.evaluate_internal(&request);
        assert!(matches!(decision.decision, Decision::Deny { .. }));
        assert!(decision
            .evidence
            .iter()
            .all(|line| !line.starts_with("immune:")));
        assert!(verdict
            .metadata
            .iter()
            .all(|(key, _)| !key.starts_with("immune_")));
        assert!(hook.shared_immune_memory().is_none());
        assert!(hook
            .last_immune_signal(&session, &trace.to_string())
            .is_none());
    }

    #[test]
    fn mounted_memory_only_appends_immune_signal() {
        let plain = BehaviorChainGuardHook::new();
        let immune = mounted_hook();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();
        let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

        let (baseline_decision, baseline_verdict) = plain.evaluate_internal(&request);
        let (observed_decision, observed_verdict) = immune.evaluate_internal(&request);

        assert_eq!(observed_decision.decision, baseline_decision.decision);
        assert_eq!(observed_decision.risk_score, baseline_decision.risk_score);
        assert_eq!(observed_decision.reasons, baseline_decision.reasons);
        assert_eq!(observed_decision.stage, baseline_decision.stage);
        assert_eq!(
            observed_decision.classifier_prediction,
            baseline_decision.classifier_prediction
        );

        let added: Vec<&String> = observed_decision
            .evidence
            .iter()
            .filter(|line| line.starts_with("immune:"))
            .collect();
        assert_eq!(added.len(), 1, "每次风险出口恰好追加一条 immune evidence");
        assert_eq!(
            observed_decision.evidence.len(),
            baseline_decision.evidence.len() + 1
        );

        let mut stripped_decision = observed_decision.clone();
        stripped_decision
            .evidence
            .retain(|line| !line.starts_with("immune:"));
        assert_eq!(
            serde_json::to_string(&stripped_decision).expect("stripped decision serializes"),
            serde_json::to_string(&baseline_decision).expect("baseline decision serializes"),
            "挂载后除 immune 旁路外主判定逐字节不变"
        );

        let mut stripped_verdict = observed_verdict.clone();
        for key in [
            "immune_kind",
            "immune_tier",
            "immune_escalated",
            "immune_record_key",
            "immune_reason",
        ] {
            stripped_verdict.metadata.remove(key);
        }
        // `guard_evidence` 是 evidence 的拼接结果，需把追加上去的 immune 段一并去掉。
        if let Some(joined) = stripped_verdict.metadata.remove("guard_evidence") {
            let cleaned = joined
                .split("; ")
                .filter(|part| !part.starts_with("immune:"))
                .collect::<Vec<_>>()
                .join("; ");
            if !cleaned.is_empty() {
                stripped_verdict.metadata.insert("guard_evidence", cleaned);
            }
        }
        assert_eq!(stripped_verdict, baseline_verdict);
        assert!(observed_verdict.metadata.contains_key("immune_tier"));
        assert!(immune
            .last_immune_signal(&session, &trace.to_string())
            .is_some());
        assert!(plain
            .last_immune_signal(&session, &trace.to_string())
            .is_none());
    }

    #[test]
    fn first_high_severity_incident_escalates_to_lockdown() {
        let hook = mounted_hook();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();
        let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

        let (decision, _) = hook.evaluate_internal(&request);
        assert!(matches!(decision.decision, Decision::Deny { .. }));
        let signal = hook
            .last_immune_signal(&session, &trace.to_string())
            .expect("mounted hook must record an immune signal");
        assert_eq!(signal.kind, IncidentKind::CredentialLeak);
        assert_eq!(signal.tier(), SeverityTier::Tier3Lockdown);
        assert!(signal.escalated());
        assert!(signal.evidence_line().contains("tier=tier3_lockdown"));

        let shared = hook
            .shared_immune_memory()
            .expect("mounted hook exposes its memory");
        let record = shared
            .lock()
            .record(&signal.signature)
            .expect("escalated incident must be recorded");
        assert_eq!(record.occurrences, 1);
        assert_eq!(record.tolerance_level, 0);
    }

    #[test]
    fn second_identical_signature_downgrades_without_releasing_deny() {
        let hook = mounted_hook();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();

        let first_request = credential_request(&capability, &arguments, session, trace, 1, "a1");
        let (first_decision, _) = hook.evaluate_internal(&first_request);
        let first = hook
            .last_immune_signal(&session, &trace.to_string())
            .expect("first observation");

        let second_request = credential_request(&capability, &arguments, session, trace, 2, "a2");
        let (second_decision, _) = hook.evaluate_internal(&second_request);
        let second = hook
            .last_immune_signal(&session, &trace.to_string())
            .expect("second observation");

        assert_eq!(first.signature, second.signature, "同签名才能命中适应记忆");
        assert_eq!(first.tier(), SeverityTier::Tier3Lockdown);
        assert_eq!(second.tier(), SeverityTier::Tier1Note);
        assert!(second.tier() < first.tier(), "二次暴露必须降级");
        assert!(!second.escalated(), "已知模式不得再升级");
        assert!(matches!(first_decision.decision, Decision::Deny { .. }));
        assert!(
            matches!(second_decision.decision, Decision::Deny { .. }),
            "免疫降级不允许把 Deny 改成 Allow（轻 != 允许）"
        );

        let shared = hook
            .shared_immune_memory()
            .expect("mounted hook exposes its memory");
        let record = shared
            .lock()
            .record(&second.signature)
            .expect("known pattern stays recorded");
        assert_eq!(record.occurrences, 2);
        assert_eq!(record.tolerance_level, 1);
    }

    #[test]
    fn self_origin_tolerance_is_quiet_and_records_nothing() {
        let observation = observation_for("credential.read", &credential_arguments());
        let decision = GuardDecision {
            decision: Decision::require_approval("human confirmation required"),
            risk_score: 0.5,
            reasons: Vec::new(),
            evidence: Vec::new(),
            stage: GuardStage::ChainGuard,
            classifier_prediction: None,
        };
        let mut memory = ImmuneMemory::with_capacity(8);
        let signal = observe_incident(
            &mut memory,
            &observation,
            &decision,
            &aligned_assessment(),
            false,
            true,
            1_000,
        );

        assert_eq!(signal.tier(), SeverityTier::Tier0Quiet);
        assert!(!signal.escalated());
        assert_eq!(memory.open_count(), 0);
        assert_eq!(memory.known_patterns(), 0);
    }

    #[test]
    fn resolved_incident_is_quiet_inside_cooldown() {
        let hook = mounted_hook();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();

        let first_request = credential_request(&capability, &arguments, session, trace, 1, "a1");
        let (_, _) = hook.evaluate_internal(&first_request);
        let first = hook
            .last_immune_signal(&session, &trace.to_string())
            .expect("first observation");
        assert_eq!(first.tier(), SeverityTier::Tier3Lockdown);

        let shared = hook
            .shared_immune_memory()
            .expect("mounted hook exposes its memory");
        assert!(shared
            .lock()
            .resolve(&first.signature, chrono::Utc::now().timestamp_millis()));

        let second_request = credential_request(&capability, &arguments, session, trace, 2, "a2");
        let (second_decision, _) = hook.evaluate_internal(&second_request);
        let second = hook
            .last_immune_signal(&session, &trace.to_string())
            .expect("second observation");

        assert_eq!(second.signature, first.signature);
        assert_eq!(
            second.tier(),
            SeverityTier::Tier0Quiet,
            "冷却窗内只静默计数"
        );
        assert!(!second.escalated());
        assert!(matches!(second_decision.decision, Decision::Deny { .. }));
        assert_eq!(shared.lock().open_count(), 0);
    }

    #[test]
    fn allowed_actions_are_never_observed() {
        let hook = mounted_hook();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("fs.read").expect("valid id");
        let arguments = serde_json::json!({ "path": "README.md" });
        let request = dispatch(&capability, &arguments, session, trace, 1, "a1");

        let (decision, verdict) = hook.evaluate_internal(&request);
        assert!(decision.is_allowed(), "纯读路径应保持 allow");
        assert!(
            hook.last_immune_signal(&session, &trace.to_string())
                .is_none(),
            "Allow 出口不得观察，避免正常流量预置记忆"
        );
        assert_eq!(
            hook.shared_immune_memory()
                .expect("mounted hook exposes its memory")
                .lock()
                .known_patterns(),
            0
        );
        assert!(verdict
            .metadata
            .iter()
            .all(|(key, _)| !key.starts_with("immune_")));
    }

    #[test]
    fn incident_kind_mapping_matches_documented_table() {
        let aligned = aligned_assessment();
        let injection = AlignmentAssessment {
            class: AlignmentClass::HighRiskMismatch,
            score: 0.95,
            reasons: vec!["unrequested_credential_access".to_string()],
        };

        assert_eq!(
            classify_incident(
                &observation_for("credential.read", &credential_arguments()),
                &aligned,
                false
            ),
            IncidentKind::CredentialLeak
        );
        assert_eq!(
            classify_incident(
                &observation_for(
                    "http.send",
                    &serde_json::json!({ "url": "https://example.com" })
                ),
                &aligned,
                false
            ),
            IncidentKind::EgressAnomaly
        );
        assert_eq!(
            classify_incident(
                &observation_for(
                    "tool.content_feed",
                    &serde_json::json!({ "url": "https://example.com" })
                ),
                &aligned,
                false
            ),
            IncidentKind::UntrustedContentEscape
        );
        assert_eq!(
            classify_incident(
                &observation_for("tool.mystery", &serde_json::json!({})),
                &aligned,
                false
            ),
            IncidentKind::ToolPoison
        );
        assert_eq!(
            classify_incident(
                &observation_for("fs.read", &serde_json::json!({ "path": "README.md" })),
                &injection,
                true
            ),
            IncidentKind::PromptInjection
        );
        assert_eq!(
            classify_incident(
                &observation_for("fs.read", &serde_json::json!({ "path": "README.md" })),
                &injection,
                false
            ),
            IncidentKind::Unknown,
            "未绑定 intent 时不得把对齐缺失当成注入"
        );
    }

    #[test]
    fn verdict_metadata_carries_immune_tier_without_changing_decision() {
        let hook = mounted_hook();
        let session = SessionId::new();
        let trace = TraceId::new();
        let capability = CapabilityId::new("credential.read").expect("valid id");
        let arguments = credential_arguments();
        let request = credential_request(&capability, &arguments, session, trace, 1, "a1");

        let (decision, verdict) = hook.evaluate_internal(&request);
        assert!(matches!(decision.decision, Decision::Deny { .. }));
        assert_eq!(verdict.decision, decision.decision, "主结论不得被免疫改写");
        assert_eq!(verdict.metadata.get("immune_kind"), Some("credential_leak"));
        assert_eq!(verdict.metadata.get("immune_tier"), Some("tier3_lockdown"));
        assert_eq!(verdict.metadata.get("immune_escalated"), Some("true"));
        assert!(verdict
            .metadata
            .get("immune_record_key")
            .is_some_and(|key| key.starts_with("credential_leak:")));
        assert!(verdict
            .metadata
            .get("immune_reason")
            .is_some_and(|reason| !reason.is_empty()));
        assert!(verdict
            .metadata
            .get("guard_evidence")
            .is_some_and(|evidence| evidence.contains("immune:")));
    }
}
