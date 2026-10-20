//! 生物机制族 W3.3 装配面: guard 免疫升级 → quorum Conflict 源接线测试.
//!
//! 覆盖装配级契约 (不是免疫内核 / 审议内核单测的重复):
//! 1. 默认关: 门关时即使挂了免疫句柄也零信号、零门账, 记忆不被消费;
//! 2. 门开但免疫面未挂载 (None): 零外部信号, 审议照旧 `Hold`;
//! 3. 真实 guard hook 的 Deny 出口 → 高严重度免疫升级 → 共享句柄 → `Conflict`
//!    (`source == "guard.immune"`);
//! 4. 节流: 同一次 observe 只注入一次, 重复拉取不重复;
//! 5. 同签名降级再 observe (非升级) 不再注入;
//! 6. 同签名收口后冷却期外再次升级 (新 occurruence) 才再注入一次;
//! 7. 非升级观察 (低严重度 / self 耐受) 永不注入;
//! 8. 已收口记录 (即使 tier 仍是 `Tier3Lockdown`) 不算升级, 不注入;
//! 9. 默认阈值下单条 Conflict 0.4 不召开 (`Hold`), 三条 1.2 才召开且门账出现
//!    `conflict ... source=guard.immune` (不改默认阈值);
//! 10. 确定性: `now_ms` 全注入, 同输入 = 同信号账;
//! 11. OrganModule AfterTurn 用注入时钟拉取 Conflict;
//! 12. ProductionModules 的迟绑定口: 仅器官模块在场且句柄非 None 时挂上.
//!
//! **0 装诚实**: 门默认关; 免疫钩子仅 `APEIRETH_ENABLE_IMMUNE_MEMORY=1` 才挂载,
//! 本文件全部用显式句柄 / 显式门注入, 不依赖也不污染进程 env. 外部事件源只接
//! guard 免疫这一条 (IM / gateway 未接); 免疫分级不改审批 / 拒绝与审议结论.

use std::sync::Arc;

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{CapabilityId, Clock, Episode, SessionId, TraceId};
use apeireth_governance::{Decision, GovernanceRequest};
use apeireth_guard::hook::SharedImmuneMemory;
use apeireth_guard::immune_memory::{
    normalize, ImmuneMemory, IncidentKind, IncidentSignature, SeverityTier, DEFAULT_COOLDOWN_MS,
};
use apeireth_guard::BehaviorChainGuardHook;
use apeireth_orchestration::council::QuorumGate;
use apeireth_orchestration::quorum::{
    QuorumAccumulator, QuorumConfig, QuorumDecision, QuorumSignal, QuorumSignalKind,
};
use apeireth_orchestration::Council;
use apeireth_plugin::organ::{OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_protocol::canonical::NormalizedResponse;
use apeireth_runtime_assembly::canonical::module::{
    BehaviorModule, HookPoint, InvocationContext, ModuleContext, ModuleInvocationError,
    ModuleInvocationRequest, ModuleInvocationResponse, ModuleInvoker,
};
use apeireth_runtime_assembly::canonical::orchestrator::{
    immune_record_is_escalation, quorum_now_ms_from_i64, LocalOrchestratorRelationship,
    LocalSovereignty, MockCouncilInvoker, OrchestratorBoundaries, OrchestratorDecision,
    OrchestratorLoopConfig, OrganOrchestrator, OrganOrchestratorGate, OrganTickInput, QuorumTrace,
    SovereigntyGate, IMMUNE_QUORUM_SIGNAL_SOURCE,
};
use apeireth_runtime_assembly::canonical::subloop::{
    SubLoopError, SubLoopResult, SubLoopSpawner, SubLoopSpec,
};
use apeireth_runtime_assembly::{
    CognitiveBackends, CognitiveModuleConfig, OrganModule, ProductionCognitiveModules,
};
use async_trait::async_trait;
use chrono::TimeZone;

type TestOrchestrator = OrganOrchestrator<LocalOrchestratorRelationship>;
type TestMemory = Arc<parking_lot::Mutex<ImmuneMemory>>;

/// 用户原文探针: 一旦进入门账 / 信号 source 即视为泄漏.
const CANARY: &str = "CANARY-IMMUNE-USER-TEXT-9c1f";

// ============================================
// 测试替身: 9 organ mock / 固定时钟 / 免疫记忆
// ============================================

struct MockOrgan {
    kind: OrganKind,
}

#[async_trait]
impl OrganTrait for MockOrgan {
    fn name(&self) -> &'static str {
        "MockOrgan(bio_quorum_immune_wiring)"
    }

    fn organ_id(&self) -> OrganKind {
        self.kind
    }

    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        Ok(OrganOutput::NotImplemented {
            organ: self.kind,
            note: "mock organ (bio_quorum_immune_wiring)".to_string(),
        })
    }
}

fn fixed_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .single()
            .expect("fixed virtual clock"),
    ))
}

fn fixed_clock_ms() -> u64 {
    quorum_now_ms_from_i64(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .single()
            .expect("fixed virtual clock")
            .timestamp_millis(),
    )
}

fn make_episode() -> Episode {
    Episode {
        id: "ep-bio-quorum-immune".into(),
        session_id: "sess-bio-quorum-immune".into(),
        role: "user".into(),
        content: "user text that must never leak into quorum reasons".into(),
        timestamp: 0,
    }
}

fn make_tick_input(at_ms: i64) -> OrganTickInput {
    OrganTickInput {
        at_ms,
        minutes_of_day: ((at_ms.rem_euclid(86_400_000)) / 60_000) as u32,
        day_key: format!("tick-{}", at_ms.div_euclid(86_400_000)),
        context_hint: Some("你好".into()),
        episode: make_episode(),
        session_id: "sess-bio-quorum-immune".into(),
    }
}

fn proposal() -> apeireth_orchestration::Proposal {
    apeireth_orchestration::Proposal {
        id: "p-bio-quorum-immune".into(),
        proposer: "bio-quorum-immune-test".into(),
        payload: serde_json::json!({
            "action": "proactive_contact",
            "user_text": CANARY,
        }),
        submitted_at: 1_767_268_800,
        session_id: SessionId::default(),
    }
}

fn config(threshold: f32, decay_per_second: f32, cooldown_ms: u64) -> QuorumConfig {
    QuorumConfig {
        threshold,
        decay_per_second,
        cooldown_ms,
        max_accumulation: 10.0,
    }
}

fn enabled_gate(config: QuorumConfig) -> QuorumGate {
    QuorumGate::enabled(QuorumAccumulator::new(config))
}

fn build(gate: QuorumGate) -> TestOrchestrator {
    let organ = |kind: OrganKind| -> Arc<dyn OrganTrait> { Arc::new(MockOrgan { kind }) };
    let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
        Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
    OrganOrchestrator::new(
        organ(OrganKind::E4),
        organ(OrganKind::F1),
        organ(OrganKind::F4),
        organ(OrganKind::F6),
        organ(OrganKind::W1),
        organ(OrganKind::W2),
        organ(OrganKind::W3),
        organ(OrganKind::E7),
        organ(OrganKind::Memory),
        Arc::new(Council::default_allow()),
        Arc::new(MockCouncilInvoker::allow_all()),
        sovereignty,
        LocalOrchestratorRelationship::new(0.5),
        OrchestratorBoundaries::default(),
        OrchestratorLoopConfig::default(),
        fixed_clock(),
    )
    .with_quorum_gate(gate)
}

fn immune_memory() -> TestMemory {
    Arc::new(parking_lot::Mutex::new(ImmuneMemory::with_capacity(32)))
}

/// 直接 observe 一次高严重度事件 (= guard hook Deny 出口的提示级别).
///
/// 返回归一化签名, 供后续 `resolve` / 降级 observe 使用. 断言这次观察确实升级.
fn observe_escalated(
    memory: &TestMemory,
    kind: IncidentKind,
    raw_key: &str,
    now_ms: i64,
) -> IncidentSignature {
    let response = memory
        .lock()
        .observe(kind, raw_key, SeverityTier::Tier2Guard, false, now_ms);
    assert!(response.escalated, "高严重度新签名必须升级: {response:?}");
    assert_eq!(response.tier, SeverityTier::Tier3Lockdown);
    let signature = normalize(kind, raw_key);
    assert_eq!(
        signature.record_key(),
        response.record_key,
        "签名与响应记录键同源"
    );
    signature
}

fn credential_request<'a>(
    capability: &'a CapabilityId,
    arguments: &'a serde_json::Value,
) -> GovernanceRequest<'a> {
    GovernanceRequest::new(
        apeireth_governance::Action::CapabilityDispatch {
            capability,
            arguments,
        },
        SessionId::new(),
        TraceId::new(),
        1,
    )
    .with_action_id("immune-wiring-action")
}

fn conflict_signals(signals: &[QuorumSignal]) -> Vec<&QuorumSignal> {
    signals
        .iter()
        .filter(|signal| signal.kind == QuorumSignalKind::Conflict)
        .filter(|signal| signal.source == IMMUNE_QUORUM_SIGNAL_SOURCE)
        .collect()
}

// ============================================
// 1. 默认关: 挂了免疫句柄也零信号 / 零门账, 记忆不被消费
// ============================================

#[tokio::test]
async fn gate_off_ignores_immune_escalations() {
    let memory = immune_memory();
    observe_escalated(
        &memory,
        IncidentKind::CredentialLeak,
        "capability=cred.read",
        1_000,
    );

    let mut orch = build(QuorumGate::disabled());
    orch.attach_immune_memory(Some(Arc::clone(&memory)));
    assert!(orch.immune_memory_attached(), "句柄确实挂上");
    assert!(!orch.quorum_gate_enabled(), "门仍是默认关");

    assert_eq!(
        orch.record_immune_signals(1_000).await,
        0,
        "默认关不注入任何外部信号"
    );
    assert!(orch.quorum_signals().await.is_empty(), "默认关门上零信号");
    assert!(orch.last_quorum_trace().is_none(), "默认关门账为空");

    // 整 tick 同样零变化: 照常开口, 门账仍空, 免疫记忆没被消费.
    let outcome = orch.tick(make_tick_input(1_000_000)).await;
    assert!(outcome.is_some(), "默认关 = 原路径, 应照常开口");
    assert!(matches!(
        orch.last_decision(),
        Some(OrchestratorDecision::Spoke { .. })
    ));
    assert!(orch.last_quorum_trace().is_none(), "tick 后门账仍空");
    assert!(orch.quorum_signals().await.is_empty());
    assert_eq!(memory.lock().open_count(), 1, "免疫记忆状态零消费");
}

// ============================================
// 2. 门开但免疫面未挂载: 零外部信号, 审议照旧 Hold
// ============================================

#[tokio::test]
async fn gate_on_without_immune_handle_records_nothing() {
    let orch = build(enabled_gate(config(0.4, 0.0, 600_000)));
    assert!(orch.quorum_gate_enabled());
    assert!(!orch.immune_memory_attached(), "默认未挂载");
    assert_eq!(orch.record_immune_signals(1_000).await, 0);
    assert!(
        orch.quorum_signals().await.is_empty(),
        "无句柄 = 零外部信号"
    );

    assert!(
        !orch
            .council_deliberate_at(&proposal(), 1_000)
            .await
            .expect("gated path never errors"),
        "没有证据 = 未达阈值"
    );
    let trace: QuorumTrace = orch.last_quorum_trace().expect("拒绝也留门账");
    assert!(!trace.convened);
    assert!(matches!(trace.denial, Some(QuorumDecision::Hold { .. })));
}

// ============================================
// 3. 真实 guard hook Deny → 升级记录 → 共享句柄 → Conflict
// ============================================

#[tokio::test]
async fn guard_deny_escalation_feeds_one_conflict_with_stable_source() {
    let hook =
        BehaviorChainGuardHook::new().with_immune_memory(Arc::new(ImmuneMemory::with_capacity(32)));
    let capability = CapabilityId::new("credential.read").expect("valid capability id");
    let arguments = serde_json::json!({ "path": ".env" });
    let decision = hook.evaluate_detailed(&credential_request(&capability, &arguments));
    assert!(
        matches!(decision.decision, Decision::Deny { .. }),
        "credential.read 应被 Deny, got {:?}",
        decision.decision
    );

    // 组合根取回 hook 正在使用的同一份免疫句柄 (不是拷贝).
    let shared: SharedImmuneMemory = hook
        .shared_immune_memory()
        .expect("mounted hook exposes its memory");

    let orch = build(enabled_gate(config(0.4, 0.0, 600_000)));
    orch.attach_immune_memory(Some(shared));
    assert_eq!(orch.record_immune_signals(42_000).await, 1);

    let signals = orch.quorum_signals().await;
    let conflicts = conflict_signals(&signals);
    assert_eq!(conflicts.len(), 1, "一条升级 = 一条 Conflict: {signals:?}");
    assert_eq!(conflicts[0].source, IMMUNE_QUORUM_SIGNAL_SOURCE);
    assert_eq!(conflicts[0].source, "guard.immune", "source 只许稳定名");
    assert_eq!(conflicts[0].at_ms, 42_000, "时刻 = 注入的 now_ms");
    assert!(
        (conflicts[0].weight - 0.4).abs() < 1e-6,
        "权重用 Conflict 既有缺省 0.4, got {}",
        conflicts[0].weight
    );

    // source 无敏感信息: 不含 URL / 路径 / token / record key / reason 全文.
    let source = &conflicts[0].source;
    for needle in [
        "http",
        "https",
        "/",
        "\\",
        ".env",
        "credential",
        "CANARY",
        "record",
        "reason",
    ] {
        assert!(
            !source.contains(needle),
            "source 不得含敏感信息 {needle:?}: {source}"
        );
    }
}

// ============================================
// 4. 节流: 同一次 observe 只注入一次
// ============================================

#[tokio::test]
async fn same_escalation_is_injected_only_once() {
    let memory = immune_memory();
    observe_escalated(&memory, IncidentKind::PromptInjection, "injection-a", 1_000);
    let orch = build(enabled_gate(config(10.0, 0.0, 600_000)));
    orch.attach_immune_memory(Some(Arc::clone(&memory)));

    assert_eq!(orch.record_immune_signals(1_000).await, 1);
    assert_eq!(
        orch.record_immune_signals(2_000).await,
        0,
        "存量升级不得每 tick 重复注入"
    );
    assert_eq!(
        orch.record_immune_signals(3_000).await,
        0,
        "第三次拉取仍为零"
    );
    let signals = orch.quorum_signals().await;
    assert_eq!(signals.len(), 1, "同一次 observe 只有一条: {signals:?}");
    assert_eq!(
        signals[0].at_ms, 1_000,
        "at_ms 是首次注入时刻, 不会被后续拉取改写"
    );
}

// ============================================
// 5. 同签名降级再 observe (非升级) 不再注入
// ============================================

#[tokio::test]
async fn downgraded_reobserve_does_not_reinject() {
    let memory = immune_memory();
    let signature = observe_escalated(&memory, IncidentKind::ToolPoison, "tool-x", 1_000);
    let orch = build(enabled_gate(config(10.0, 0.0, 600_000)));
    orch.attach_immune_memory(Some(Arc::clone(&memory)));
    assert_eq!(orch.record_immune_signals(1_000).await, 1);

    // 已知模式: 同签名第二次 = 适应记忆降级, escalated=false.
    let response = memory.lock().observe(
        IncidentKind::ToolPoison,
        "tool-x",
        SeverityTier::Tier2Guard,
        false,
        2_000,
    );
    assert!(!response.escalated, "已知模式不得升级: {response:?}");
    assert_eq!(response.tier, SeverityTier::Tier1Note);
    assert_eq!(
        memory
            .lock()
            .record(&signature)
            .expect("record exists")
            .occurrences,
        2
    );
    assert!(!immune_record_is_escalation(
        &memory.lock().record(&signature).expect("record exists")
    ));

    assert_eq!(
        orch.record_immune_signals(2_000).await,
        0,
        "降级后的记录不算升级, 不再注入"
    );
    assert_eq!(orch.quorum_signals().await.len(), 1);
}

// ============================================
// 6. 收口后冷却期外再次升级 (新 occurrence) 才再注入一次
// ============================================

#[tokio::test]
async fn re_escalation_after_resolve_injects_once_more() {
    let memory = immune_memory();
    let signature = observe_escalated(&memory, IncidentKind::EgressAnomaly, "egress-a", 1_000);
    let orch = build(enabled_gate(config(10.0, 0.0, 600_000)));
    orch.attach_immune_memory(Some(Arc::clone(&memory)));
    assert_eq!(orch.record_immune_signals(1_000).await, 1);

    assert!(memory.lock().resolve(&signature, 1_000), "收口成功");
    assert_eq!(
        orch.record_immune_signals(1_500).await,
        0,
        "已收口记录不是升级 (即使 tier 仍是 Tier3Lockdown)"
    );

    // 冷却窗已过 + 高严重度复发 = 新一次升级 (occurrences 前进).
    let response = memory.lock().observe(
        IncidentKind::EgressAnomaly,
        "egress-a",
        SeverityTier::Tier2Guard,
        false,
        1_000 + DEFAULT_COOLDOWN_MS + 1,
    );
    assert!(response.escalated, "冷却后高严重度复发应升级: {response:?}");
    assert_eq!(
        orch.record_immune_signals((1_000 + DEFAULT_COOLDOWN_MS + 1) as u64)
            .await,
        1,
        "新一次 upgrade 再注入一次"
    );
    let signals = orch.quorum_signals().await;
    assert_eq!(
        conflict_signals(&signals).len(),
        2,
        "两次升级 = 两条: {signals:?}"
    );
}

// ============================================
// 7. 非升级观察 (低严重度 / self 耐受) 永不注入
// ============================================

#[tokio::test]
async fn non_escalated_observations_never_inject() {
    let memory = immune_memory();
    // 低严重度新签名 = Tier1Note, 不升级.
    let low = memory.lock().observe(
        IncidentKind::Unknown,
        "benign-low",
        SeverityTier::Tier1Note,
        false,
        1_000,
    );
    assert!(!low.escalated);
    // self 耐受: 不建立记录.
    let self_origin = memory.lock().observe(
        IncidentKind::Unknown,
        "self-origin",
        SeverityTier::Tier1Note,
        true,
        1_000,
    );
    assert!(!self_origin.escalated);
    assert_eq!(memory.lock().open_count(), 1, "self 耐受不写记录");

    let orch = build(enabled_gate(config(0.4, 0.0, 600_000)));
    orch.attach_immune_memory(Some(Arc::clone(&memory)));
    assert_eq!(orch.record_immune_signals(1_000).await, 0);
    assert!(orch.quorum_signals().await.is_empty());
}

// ============================================
// 8. 已收口记录不算升级 (未注入过也不注入)
// ============================================

#[tokio::test]
async fn resolved_high_record_is_not_injected() {
    let memory = immune_memory();
    let signature = observe_escalated(
        &memory,
        IncidentKind::UntrustedContentEscape,
        "escape-a",
        1_000,
    );
    // 未注入前先收口: status=Resolved, tier 仍是 Tier3Lockdown.
    assert!(memory.lock().resolve(&signature, 1_000));
    let record = memory.lock().record(&signature).expect("record exists");
    assert_eq!(record.tier, SeverityTier::Tier3Lockdown);
    assert!(!immune_record_is_escalation(&record), "Resolved 不算升级");

    let orch = build(enabled_gate(config(0.4, 0.0, 600_000)));
    orch.attach_immune_memory(Some(Arc::clone(&memory)));
    assert_eq!(orch.record_immune_signals(1_000).await, 0);
    assert!(orch.quorum_signals().await.is_empty());
}

// ============================================
// 9. 默认阈值: 单条 Conflict 不召开, 三条才召开 (门账可观测)
// ============================================

#[tokio::test]
async fn default_threshold_needs_three_conflicts_and_trace_is_bounded() {
    // 单条 Conflict 0.4 < 默认阈值 1.0 → Hold, 不得把门打成常开.
    let single_memory = immune_memory();
    observe_escalated(
        &single_memory,
        IncidentKind::CredentialLeak,
        "single-a",
        1_000,
    );
    let mut single = build(enabled_gate(config(1.0, 0.0, 600_000)));
    single.attach_immune_memory(Some(single_memory));
    assert!(single.tick(make_tick_input(1_000_000)).await.is_none());
    assert_eq!(
        single.last_decision(),
        Some(&OrchestratorDecision::Held(
            OrganOrchestratorGate::GateBlock
        )),
        "未达阈值 = 门拒绝召开, 不是 CouncilVeto"
    );
    let trace = single.last_quorum_trace().expect("拒绝留门账");
    assert!(!trace.convened);
    assert!(matches!(trace.denial, Some(QuorumDecision::Hold { .. })));

    // 三条不同签名升级 = 0.4 × 3 = 1.2 ≥ 1.0 → 本次 tick 召开.
    let memory = immune_memory();
    observe_escalated(&memory, IncidentKind::CredentialLeak, "triple-a", 1_000);
    observe_escalated(&memory, IncidentKind::PromptInjection, "triple-b", 1_000);
    observe_escalated(&memory, IncidentKind::EgressAnomaly, "triple-c", 1_000);
    let mut orch = build(enabled_gate(config(1.0, 0.0, 600_000)));
    orch.attach_immune_memory(Some(Arc::clone(&memory)));

    let outcome = orch.tick(make_tick_input(1_000_000)).await;
    assert!(outcome.is_some(), "三条 Conflict 达阈值 → 召开 → 开口");

    let trace = orch.last_quorum_trace().expect("召开留门账");
    assert!(trace.convened, "应已召开");
    assert!(trace.escalation.is_some());
    let reasons = trace.reasons();
    assert!(
        reasons.iter().any(|reason| reason.contains("conflict")),
        "门账应含 Conflict 类别: {reasons:?}"
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("source=guard.immune")),
        "门账应按稳定名标注来源: {reasons:?}"
    );

    // 门账 / 理由不含用户原文、也不含 record key / reason 全文.
    let line = trace.summary_line();
    for needle in [CANARY, "CANARY", "USER-TEXT", "record_key", "自我耐受"] {
        assert!(!line.contains(needle), "门账不得含 {needle:?}: {line}");
        for reason in reasons {
            assert!(!reason.contains(needle), "理由不得含 {needle:?}: {reason}");
        }
    }
}

// ============================================
// 10. 确定性: now_ms 全注入, 同输入 = 同信号账
// ============================================

#[tokio::test]
async fn injected_now_is_deterministic() {
    let memory_a = immune_memory();
    let memory_b = immune_memory();
    let salt = 777_000_i64;
    for memory in [&memory_a, &memory_b] {
        observe_escalated(memory, IncidentKind::ToolPoison, "tool-det", salt);
        observe_escalated(memory, IncidentKind::Unknown, "unknown-det", salt + 1);
    }

    let a = build(enabled_gate(config(1.0, 0.0, 600_000)));
    let b = build(enabled_gate(config(1.0, 0.0, 600_000)));
    a.attach_immune_memory(Some(memory_a));
    b.attach_immune_memory(Some(memory_b));

    assert_eq!(a.record_immune_signals(900_000).await, 2);
    assert_eq!(b.record_immune_signals(900_000).await, 2);
    let signals_a = a.quorum_signals().await;
    let signals_b = b.quorum_signals().await;
    assert_eq!(
        format!("{signals_a:?}"),
        format!("{signals_b:?}"),
        "同输入 + 同 now_ms → 信号账逐字节同一"
    );
    for signal in &signals_a {
        assert_eq!(signal.at_ms, 900_000, "时刻 = 注入 now_ms");
        assert_eq!(signal.kind, QuorumSignalKind::Conflict);
        assert_eq!(signal.source, IMMUNE_QUORUM_SIGNAL_SOURCE);
    }
    assert!((a.quorum_score(900_000).await - 0.8).abs() < 1e-6);
    assert!((b.quorum_score(900_000).await - 0.8).abs() < 1e-6);
}

// ============================================
// 11. OrganModule AfterTurn 用注入时钟拉取 Conflict
// ============================================

struct NoopInvoker;

#[async_trait]
impl ModuleInvoker for NoopInvoker {
    async fn invoke(
        &self,
        _request: ModuleInvocationRequest,
    ) -> Result<ModuleInvocationResponse, ModuleInvocationError> {
        Ok(ModuleInvocationResponse {
            response: NormalizedResponse::text("assistant", "test-model", ""),
            served_by: CapabilityId::new("provider.test").expect("capability id"),
        })
    }
}

struct NoSubLoop;

#[async_trait]
impl SubLoopSpawner for NoSubLoop {
    async fn spawn(&self, _spec: SubLoopSpec) -> Result<SubLoopResult, SubLoopError> {
        Err(SubLoopError::NoModel)
    }
}

static NO_SUB_LOOP: NoSubLoop = NoSubLoop;

#[tokio::test]
async fn organ_module_afterturn_pulls_conflict_with_injected_clock() {
    let memory = immune_memory();
    observe_escalated(
        &memory,
        IncidentKind::CredentialLeak,
        "capability=cred.read",
        1_000,
    );

    let module =
        OrganModule::new(fixed_clock()).with_quorum_gate(enabled_gate(config(0.4, 0.0, 600_000)));
    module.attach_immune_memory(Arc::clone(&memory));
    assert!(module.immune_memory_attached());
    assert!(module.quorum_gate_enabled());

    let session = SessionId::new();
    let invocation = InvocationContext::user_turn();
    let invoker: Arc<dyn ModuleInvoker> = Arc::new(NoopInvoker);
    let messages = vec![apeireth_protocol::canonical::NormalizedMessage::user(
        "你好",
    )];
    let candidate = NormalizedResponse::text("assistant", "test-model", "reply");
    let ctx = ModuleContext {
        session_id: &session,
        model: "test-model",
        messages: &messages,
        candidate: Some(&candidate),
        tool_call: None,
        tool_result: None,
        invocation: &invocation,
        module_id: apeireth_runtime_assembly::ORGAN_MODULE_ID,
        error: None,
        invoker: &*invoker,
        invoker_handle: Arc::clone(&invoker),
        subloop: &NO_SUB_LOOP,
    };
    module
        .on_hook(HookPoint::AfterTurn, &ctx)
        .await
        .expect("organ hook is fail-open, never errors");

    let signals = module.quorum_signals().await;
    let conflicts = conflict_signals(&signals);
    assert_eq!(
        conflicts.len(),
        1,
        "AfterTurn 应拉到一个 Conflict: {signals:?}"
    );
    assert_eq!(conflicts[0].source, IMMUNE_QUORUM_SIGNAL_SOURCE);
    assert_eq!(
        conflicts[0].at_ms,
        fixed_clock_ms(),
        "模块拉取时刻 = 注入时钟, 不读系统时钟"
    );
    assert_eq!(module.observations().len(), 1, "器官链照常只跑一次");
}

// ============================================
// 12. ProductionModules 迟绑定口: 器官模块在场 + 句柄非 None 才挂
// ============================================

fn build_modules(organs: bool) -> ProductionCognitiveModules {
    let config = CognitiveModuleConfig {
        organs,
        memory_recall: false,
        memory_writeback: false,
        preference_recall: false,
        self_assessment: false,
        filesystem: false,
        search: false,
        repo: false,
        self_status: false,
        ..CognitiveModuleConfig::default()
    };
    ProductionCognitiveModules::build(config, CognitiveBackends::default(), fixed_clock())
        .expect("minimal module set assembles")
}

#[test]
fn production_attach_requires_both_organ_module_and_handle() {
    let memory = immune_memory();
    let with_organs = build_modules(true);
    assert!(
        with_organs.attach_immune_memory(Some(memory.clone())),
        "器官模块在场 + 句柄非 None = 真挂上"
    );
    assert!(
        !with_organs.attach_immune_memory(None),
        "默认 None = 不挂, 零行为变化"
    );

    let without_organs = build_modules(false);
    assert!(
        !without_organs.attach_immune_memory(Some(memory)),
        "器官模块未注册 = 无处可挂"
    );
}
