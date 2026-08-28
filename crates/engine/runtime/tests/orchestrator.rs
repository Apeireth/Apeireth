//! P-arch (2026-08-28): OrganOrchestrator 类似 AwakeCompanion 集成测试 (子代理 R12 真实施).
//!
//! 3 测试 (per 子代理 R12 任务说明):
//! 1. `orchestrator_tick_9_organ_process_serial` — 9 organ process 串联, 1:1 翻译 v1
//! 2. `orchestrator_8_gates_real` — 8 重 gate 真实存在, 13 种 InitiativeGate 全列
//! 3. `orchestrator_5_state_machine_transitions` — Idle → Draft → Proposed → Ratified → Active
//!
//! **0 装诚实真账 (子代理 R12 独立判断)**:
//! - 本测试验证 Orchestrator spec 完整骨架 (9 organ + 8 gate + 5 state machine).
//! - 真实 integration 路径 (cognitive module 接入 + governance 13 键 + git tag)
//!   仍待 v2.0.0 release 后启动 (per R11 spec §8.4 + 子代理 R12 估 1-3 周).
//! - 0 引新外部 dep (Cargo.lock 0 行 diff), 0 触碰 LOCKED 5 项.

use std::sync::Arc;

use apeireth_core::clock::VirtualClock;
use apeireth_core::kernel::{Clock, Episode};
use apeireth_orchestration::Council;
use apeireth_plugin::organ::{OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait};
use apeireth_runtime::canonical::orchestrator::{
    LocalOrchestratorRelationship, LocalSovereignty, OrchestratorBoundaries,
    OrchestratorLoopConfig, OrganChainOutputs, OrganOrchestrator, OrganOrchestratorGate,
    OrganTickInput, PolicyStage, PolicyTransitionReason, RatificationChain, RelationshipState,
    SovereigntyGate,
};
use chrono::TimeZone;

// ============================================
// 测试用 mock organ
// ============================================

/// Mock organ (per R12 测试: runtime crate 不依赖 apeireth-organ).
struct MockOrgan {
    kind: OrganKind,
    process_count: std::sync::atomic::AtomicU64,
}

impl MockOrgan {
    fn new(kind: OrganKind) -> Self {
        Self {
            kind,
            process_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn call_count(&self) -> u64 {
        self.process_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl OrganTrait for MockOrgan {
    fn name(&self) -> &'static str {
        "MockOrgan"
    }
    fn organ_id(&self) -> OrganKind {
        self.kind
    }
    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        self.process_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(OrganOutput::NotImplemented {
            organ: self.kind,
            note: "mock organ (orchestrator integration test)".to_string(),
        })
    }
}

/// F1 emotion organ mock — 返 `OrganOutput::Emotion` with configurable pleasure (Stage 2 测试).
struct F1EmotionMock {
    pleasure: f32,
}

#[async_trait::async_trait]
impl OrganTrait for F1EmotionMock {
    fn name(&self) -> &'static str {
        "F1EmotionMock"
    }
    fn organ_id(&self) -> OrganKind {
        OrganKind::F1
    }
    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        Ok(OrganOutput::Emotion {
            pleasure: self.pleasure,
            arousal: 0.5,
            dominance: 0.5,
            trend: apeireth_plugin::organ::EmotionTrend::Stable,
        })
    }
}

/// 构造带可配置 F1 pleasure 的 orchestrator (Stage 2 测试 helper)
fn build_orchestrator_with_f1_pleasure(
    pleasure: f32,
) -> OrganOrchestrator<LocalOrchestratorRelationship> {
    use apeireth_plugin::organ::EmotionTrend;
    let _ = EmotionTrend::Stable; // suppress unused import

    let organ_e4: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::E4));
    let organ_f1: Arc<dyn OrganTrait> = Arc::new(F1EmotionMock { pleasure });
    let organ_f4: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::F4));
    let organ_f6: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::F6));
    let organ_w1: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W1));
    let organ_w2: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W2));
    let organ_w3: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W3));
    let organ_e7: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::E7));
    let organ_memory: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::Memory));

    let council = Arc::new(Council::default_allow());
    let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
        Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
    let rel = LocalOrchestratorRelationship::new(0.5);

    OrganOrchestrator::new(
        organ_e4,
        organ_f1,
        organ_f4,
        organ_f6,
        organ_w1,
        organ_w2,
        organ_w3,
        organ_e7,
        organ_memory,
        council,
        sovereignty,
        rel,
        OrchestratorBoundaries::default(),
        OrchestratorLoopConfig::default(),
        clock(),
    )
}

/// E7 emergence organ mock — 返 `OrganOutput::Emergence` with configurable gate (Stage 3 测试).
struct E7GateMock {
    gate: Option<apeireth_plugin::organ::InitiativeGate>,
}

#[async_trait::async_trait]
impl OrganTrait for E7GateMock {
    fn name(&self) -> &'static str {
        "E7GateMock"
    }
    fn organ_id(&self) -> OrganKind {
        OrganKind::E7
    }
    async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
        Ok(OrganOutput::Emergence {
            action: String::new(),
            spoke: false,
            gate: self.gate,
        })
    }
}

/// 构造带可配置 E7 gate 的 orchestrator (Stage 3 测试 helper)
fn build_orchestrator_with_e7_gate(
    gate: Option<apeireth_plugin::organ::InitiativeGate>,
) -> OrganOrchestrator<LocalOrchestratorRelationship> {
    let organ_e4: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::E4));
    let organ_f1: Arc<dyn OrganTrait> = Arc::new(F1EmotionMock { pleasure: 0.6 }); // 高 pleasure 跳过 EmotionLow
    let organ_f4: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::F4));
    let organ_f6: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::F6));
    let organ_w1: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W1));
    let organ_w2: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W2));
    let organ_w3: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W3));
    let organ_e7: Arc<dyn OrganTrait> = Arc::new(E7GateMock { gate });
    let organ_memory: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::Memory));

    let council = Arc::new(Council::default_allow());
    let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
        Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
    let rel = LocalOrchestratorRelationship::new(0.5);

    OrganOrchestrator::new(
        organ_e4,
        organ_f1,
        organ_f4,
        organ_f6,
        organ_w1,
        organ_w2,
        organ_w3,
        organ_e7,
        organ_memory,
        council,
        sovereignty,
        rel,
        OrchestratorBoundaries::default(),
        OrchestratorLoopConfig::default(),
        clock(),
    )
}

fn clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 12, 0, 0)
            .single()
            .unwrap(),
    ))
}

fn build_orchestrator() -> OrganOrchestrator<LocalOrchestratorRelationship> {
    let organ_e4: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::E4));
    let organ_f1: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::F1));
    let organ_f4: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::F4));
    let organ_f6: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::F6));
    let organ_w1: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W1));
    let organ_w2: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W2));
    let organ_w3: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::W3));
    let organ_e7: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::E7));
    let organ_memory: Arc<dyn OrganTrait> = Arc::new(MockOrgan::new(OrganKind::Memory));

    let council = Arc::new(Council::default_allow());
    let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
        Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
    let rel = LocalOrchestratorRelationship::new(0.5);

    OrganOrchestrator::new(
        organ_e4,
        organ_f1,
        organ_f4,
        organ_f6,
        organ_w1,
        organ_w2,
        organ_w3,
        organ_e7,
        organ_memory,
        council,
        sovereignty,
        rel,
        OrchestratorBoundaries::default(),
        OrchestratorLoopConfig::default(),
        clock(),
    )
}

fn make_episode() -> Episode {
    Episode {
        id: "ep-1".into(),
        session_id: "sess-1".into(),
        role: "user".into(),
        content: "test".into(),
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
        session_id: "sess-1".into(),
    }
}

// ============================================
// 测试 1: orchestrator_tick_9_organ_process_serial
// ============================================

/// 9 organ process 串联 (per R11 spec §4.1: E4 → F1 → F4 → F6 → W1 → W2 → W3 → E7 → Memory).
///
/// 验证:
/// - `chain_9_organs()` 调 9 organ 全部
/// - 输出按 §4.1 顺序累积 (e4/f1/f4/f6/w1/w2/w3/e7/memory 字段全有)
/// - 顺序真实: 9 organ 全调, `all_present()` = true
#[tokio::test]
async fn orchestrator_tick_9_organ_process_serial() {
    let orch = build_orchestrator();
    let organ_input = OrganInput::new(make_episode(), vec!["你好".to_string()]);

    // 直接调 chain_9_organs (内部 helper, 9 organ 串联)
    let outputs: OrganChainOutputs = orch.chain_9_organs(organ_input).await;

    // 9 organ 全有输出 (mock 返 NotImplemented, 算"有")
    assert!(
        outputs.all_present(),
        "9 organ 全有输出 (NotImplemented 也算有)"
    );

    // 按 §4.1 顺序累积到 9 字段
    assert!(outputs.e4.is_some(), "1. E4 curiosity 应有输出");
    assert!(outputs.f1.is_some(), "2. F1 emotion 应有输出");
    assert!(outputs.f4.is_some(), "3. F4 hypothesis 应有输出");
    assert!(outputs.f6.is_some(), "4. F6 value_cases 应有输出");
    assert!(outputs.w1.is_some(), "5. W1 world_model 应有输出");
    assert!(outputs.w2.is_some(), "6. W2 causal_world_model 应有输出");
    assert!(
        outputs.w3.is_some(),
        "7. W3 causal_world_model_edges 应有输出"
    );
    assert!(outputs.e7.is_some(), "8. E7 emergence 应有输出");
    assert!(outputs.memory.is_some(), "9. Memory memory 应有输出");

    // 9 organ handle 顺序 (per R11 spec §4.1)
    let handles = orch.organ_handles();
    assert_eq!(handles.len(), 9);
    assert_eq!(handles[0].0, "E4 curiosity");
    assert_eq!(handles[1].0, "F1 emotion_memory");
    assert_eq!(handles[2].0, "F4 hypothesis");
    assert_eq!(handles[3].0, "F6 value_cases");
    assert_eq!(handles[4].0, "W1 world_model");
    assert_eq!(handles[5].0, "W2 causal_world_model");
    assert_eq!(handles[6].0, "W3 causal_world_model_edges");
    assert_eq!(handles[7].0, "E7 emergence");
    assert_eq!(handles[8].0, "Memory merger");
}

// ============================================
// 测试 2: orchestrator_8_gates_real
// ============================================

/// 8 重 gate 真实存在 + 13 种 InitiativeGate 全列.
///
/// 验证:
/// - `OrganOrchestratorGate::ALL_13.len() == 13` (emergence 8 + organs 5)
/// - `check_8_gates()` 真实返回 InitiativeGate (user_quiet / quiet_hours / daily_limit /
///   llm_budget / min_depth 5 重真实存在; rhythm_unknown / rhythm_veto / drive_low 由 E7
///   organ 真算法给出)
/// - SovereigntyFrozen / EmotionLow / CouncilVeto / PolicyInactive / GateBlock 5 重
///   也真实存在 (orchestrator 上层)
#[tokio::test]
async fn orchestrator_8_gates_real() {
    // 13 种 InitiativeGate 全列 (per R11 spec §5: emergence 8 + organs 5 = 13)
    assert_eq!(
        OrganOrchestratorGate::ALL_13.len(),
        13,
        "8 重 gate + organs 5 重 = 13 种 InitiativeGate"
    );

    // emergence 8 重 (per v1 `emergence.rs:460-503` 1:1)
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::UserQuiet));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::QuietHours));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::DailyLimit));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::LlmBudget));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::DepthLow));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::RhythmUnknown));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::RhythmVeto));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::DriveLow));

    // organs 5 重 (per v1 `AwakeCompanion::tick` 上层 1:1)
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::SovereigntyFrozen));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::EmotionLow));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::CouncilVeto));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::PolicyInactive));
    assert!(OrganOrchestratorGate::ALL_13.contains(&OrganOrchestratorGate::GateBlock));

    // check_8_gates() 真实路径 — user_quiet (per R12 真实施 orchestrator.rs)
    {
        let mut orch = build_orchestrator();
        orch.boundaries_mut_for_test().user_quiet = true;
        let gate = orch.check_8_gates_for_test(720, 0, 0, None, &OrganChainOutputs::default());
        assert_eq!(
            gate,
            Some(OrganOrchestratorGate::UserQuiet),
            "user_quiet 应真实生效"
        );
    }

    // quiet_hours (per R12 真实施 early_gate_block)
    {
        let mut orch = build_orchestrator();
        orch.boundaries_mut_for_test().quiet_start_minutes = Some(22 * 60);
        orch.boundaries_mut_for_test().quiet_end_minutes = Some(6 * 60);
        let gate = orch.check_8_gates_for_test(23 * 60, 0, 0, None, &OrganChainOutputs::default());
        assert_eq!(
            gate,
            Some(OrganOrchestratorGate::QuietHours),
            "quiet_hours 应真实生效"
        );
    }

    // daily_limit
    {
        let orch = build_orchestrator();
        let gate = orch.check_8_gates_for_test(720, 5, 0, None, &OrganChainOutputs::default());
        assert_eq!(
            gate,
            Some(OrganOrchestratorGate::DailyLimit),
            "daily_limit 应真实生效"
        );
    }

    // llm_budget (距上次主动不足 min_llm_interval_ms)
    {
        let orch = build_orchestrator();
        // last_initiative_ms = 1000, at_ms = 1100, min_llm_interval_ms = 60_000 (default)
        // diff = 100 < 60_000 → LlmBudget 触发
        let gate = orch.check_8_gates_for_test(720, 0, 1100, Some(1000), &OrganChainOutputs::default());
        assert_eq!(
            gate,
            Some(OrganOrchestratorGate::LlmBudget),
            "llm_budget 应真实生效"
        );
    }

    // depth_low
    {
        let mut orch = build_orchestrator();
        orch.relationship_mut_for_test().adjust(-1.0); // depth → 0.0
        let gate = orch.check_8_gates_for_test(720, 0, 0, None, &OrganChainOutputs::default());
        assert_eq!(
            gate,
            Some(OrganOrchestratorGate::DepthLow),
            "depth_low 应真实生效 (depth=0.0 < min_depth=0.3)"
        );
    }

    // sovereignty_frozen
    {
        let mut orch = build_orchestrator();
        if let Some(local) = orch.sovereignty_mut_for_test().as_any_mut() {
            if let Some(local_sovereignty) = local.downcast_mut::<LocalSovereignty>() {
                local_sovereignty.freeze();
            }
        }
        let gate = orch.check_8_gates_for_test(720, 0, 0, None, &OrganChainOutputs::default());
        assert_eq!(
            gate,
            Some(OrganOrchestratorGate::SovereigntyFrozen),
            "sovereignty_frozen 应真实生效"
        );
    }

    // 默认全 pass (无任何 gate 触发) — 返 None
    {
        let orch = build_orchestrator();
        let gate = orch.check_8_gates_for_test(720, 0, 1_000_000, Some(0), &OrganChainOutputs::default());
        assert_eq!(gate, None, "默认 8 重 gate 全 pass");
    }
}

// ============================================
// 测试 3: orchestrator_5_state_machine_transitions
// ============================================

/// 5 状态机 transition 路径 (per R11 spec §6.2: Idle → Draft → Proposed → Ratified → Active).
///
/// 验证:
/// - `PolicyStage` 5 variant 全列
/// - `allowed_next()` 按 §6.2 表推进 (Idle → Draft → Proposed → Ratified → Active)
/// - `is_active()`: Ratified / Active = true, Idle / Draft / Proposed = false
/// - Orchestrator `transition_policy()` 真实推进 + 不允许 transition 返 Err
/// - `ratify_fresh_policy()` 走完整 5 状态链路终点 Active
#[tokio::test]
async fn orchestrator_5_state_machine_transitions() {
    // 5 variant (per R11 spec §6.1 + 子代理 R7 独立判断: forward-declared)
    let stages = [
        PolicyStage::Idle,
        PolicyStage::Draft,
        PolicyStage::Proposed,
        PolicyStage::Ratified,
        PolicyStage::Active,
    ];
    assert_eq!(stages.len(), 5, "5 状态机 variant");

    // §6.2 transition 路径: Idle → Draft → Proposed → Ratified → Active
    assert_eq!(PolicyStage::Idle.allowed_next(), Some(PolicyStage::Draft));
    assert_eq!(
        PolicyStage::Draft.allowed_next(),
        Some(PolicyStage::Proposed)
    );
    assert_eq!(
        PolicyStage::Proposed.allowed_next(),
        Some(PolicyStage::Ratified)
    );
    assert_eq!(
        PolicyStage::Ratified.allowed_next(),
        Some(PolicyStage::Active)
    );
    assert_eq!(PolicyStage::Active.allowed_next(), None); // 终态 (Retired 在 evolution crate)

    // is_active() (per v1 `EvolutionState::is_active` 1:1)
    assert!(!PolicyStage::Idle.is_active());
    assert!(!PolicyStage::Draft.is_active());
    assert!(!PolicyStage::Proposed.is_active());
    assert!(PolicyStage::Ratified.is_active()); // 已通过审议可发声
    assert!(PolicyStage::Active.is_active());

    // transition_policy() 真实路径 (per R12 真实施)
    let mut orch = build_orchestrator();
    assert_eq!(orch.policy_stage(), PolicyStage::Active); // 默认 Active (per ratify_fresh_policy 终点)

    // 不允许的 transition 返 Err (Active → Draft, Active 已是终态)
    let bad = orch.transition_policy(PolicyStage::Draft, PolicyTransitionReason::Revoke);
    assert!(bad.is_err(), "Active → Draft 应返 Err (Active 是终态)");

    // ratify_fresh_policy() 走完整 5 状态链路终点 Active
    assert!(orch.ratify_fresh_policy().is_ok());
    assert_eq!(orch.policy_stage(), PolicyStage::Active);

    // ratify_fresh_policy() 真实路径: 4 transition 走链 + RatificationChain 留痕
    // (per Stage 1 完整化, v1 `AwakeCompanion::ratify_fresh_policy` 1:1)
    let mut orch = build_orchestrator();
    let chain = orch.ratify_fresh_policy().expect("ratify 第一次成功");
    assert_eq!(
        chain.len(),
        4,
        "4 transitions: Draft→Proposed→Ratified→Active"
    );
    assert!(chain.all_ok(), "all 4 transitions 应 ok");
    // 4 transition 顺序核对
    assert_eq!(chain.steps[0].0, PolicyStage::Draft);
    assert_eq!(chain.steps[1].0, PolicyStage::Proposed);
    assert_eq!(chain.steps[2].0, PolicyStage::Ratified);
    assert_eq!(chain.steps[3].0, PolicyStage::Active);
    // 终态 = Active
    assert_eq!(orch.policy_stage(), PolicyStage::Active);

    // 重复 ratify_fresh_policy() = idempotent (v1 semantics: *evolution = new())
    let chain2 = orch.ratify_fresh_policy().expect("ratify 第二次也成功");
    assert_eq!(chain2.len(), 4);
    assert!(chain2.all_ok());
    assert_eq!(orch.policy_stage(), PolicyStage::Active);

    // L0-L5 集成 (per `v2-architecture-reflection.md` §6 + 子代理 R12 真实施)
    use apeireth_runtime::canonical::orchestrator::UpgradeLayer;
    assert_eq!(UpgradeLayer::ALL.len(), 6, "L0-L5 6 layers");
    assert_eq!(UpgradeLayer::L0HumanApproval.as_str(), "L0_human_approval");
    assert_eq!(UpgradeLayer::L5RuntimePatch.as_str(), "L5_runtime_patch");

    // tick 真实路径 (per R12 真实施 tick 6 步骤):
    // 1. 主权闸 → 2. 9 organ 串联 + 8 gate → 3. 情绪调制 → 4. 智囊团审议 →
    // 5. 演化闸 → 6. 洋葱门
    let mut orch = build_orchestrator();
    let outcome = orch.tick(make_tick_input(1_000_000)).await;
    // 默认 8 重 gate pass + Council.default_allow() pass + 演化 Active + 主权未冻
    // → Spoke { action: "问候" } (per R12 真实施 orchestrator.rs tick 步骤 7)
    assert!(outcome.is_some(), "默认条件全部 pass → 决定开口");
    let outcome = outcome.unwrap();
    assert_eq!(outcome.action_label, "问候");
    assert!(matches!(
        orch.last_decision(),
        Some(apeireth_runtime::canonical::orchestrator::OrchestratorDecision::Spoke { .. })
    ));

    // sovereignty frozen → tick 返 None + last_decision = Held(SovereigntyFrozen)
    let mut orch = build_orchestrator();
    if let Some(local) = orch.sovereignty_mut_for_test().as_any_mut() {
        if let Some(local_sovereignty) = local.downcast_mut::<LocalSovereignty>() {
            local_sovereignty.freeze();
        }
    }
    let outcome = orch.tick(make_tick_input(1_000_000)).await;
    assert!(outcome.is_none(), "主权熔断 → tick 返 None");
    assert_eq!(
        orch.last_decision(),
        Some(
            &apeireth_runtime::canonical::orchestrator::OrchestratorDecision::Held(
                OrganOrchestratorGate::SovereigntyFrozen
            )
        )
    );

    // user_quiet → tick 返 None + last_decision = Held(UserQuiet)
    let mut orch = build_orchestrator();
    orch.boundaries_mut_for_test().user_quiet = true;
    let outcome = orch.tick(make_tick_input(1_000_000)).await;
    assert!(outcome.is_none(), "user_quiet → tick 返 None");
    assert_eq!(
        orch.last_decision(),
        Some(
            &apeireth_runtime::canonical::orchestrator::OrchestratorDecision::Held(
                OrganOrchestratorGate::UserQuiet
            )
        )
    );
}

// ============================================
// 测试 4: orchestrator_step3_f1_emotion_real (Stage 2 完整化)
// ============================================

/// tick 步骤 3: F1 emotion organ 真路径 → 算 mood → 比 mood_floor (per v1 organs.rs:108-114 1:1).
///
/// 验证:
/// - F1 organ 返 `OrganOutput::Emotion { pleasure, .. }` → extract_emotion_mood 算 mood
///   = (pleasure + 1.0) / 2.0
/// - mood < mood_floor (default 0.3) → tick 返 None + last_decision = Held(EmotionLow)
/// - mood >= mood_floor → tick 正常走完 (Spoke)
/// - F1 organ 返 `NotImplemented` (Mock) → extract_emotion_mood 返 None → step 3 skip
#[tokio::test]
async fn orchestrator_step3_f1_emotion_real() {
    use apeireth_plugin::organ::{EmotionTrend, OrganOutput};
    use apeireth_runtime::canonical::orchestrator::OrganChainOutputs;

    // Case 1: pleasure = -0.8 → mood = (-0.8 + 1.0) / 2.0 = 0.1 < 0.3 (mood_floor)
    {
        let orch = build_orchestrator_with_f1_pleasure(-0.8_f32);
        let organ_input = OrganInput::new(make_episode(), vec!["你好".to_string()]);
        let chain: OrganChainOutputs = orch.chain_9_organs(organ_input).await;
        let mood_low = orch.extract_emotion_mood(&chain);
        assert!(
            mood_low.is_some(),
            "F1 emotion 输出存在 → extract 应返 Some"
        );
        let mood_low = mood_low.unwrap();
        assert!(
            (mood_low - 0.1).abs() < 1e-5,
            "mood_low 应 = 0.1, got {mood_low}"
        );
        assert!(
            mood_low < OrchestratorLoopConfig::default().mood_floor,
            "mood_low < mood_floor → 应触发 EmotionLow"
        );
    }

    // Case 2: pleasure = 0.6 → mood = (0.6 + 1.0) / 2.0 = 0.8 > 0.3
    {
        let orch = build_orchestrator_with_f1_pleasure(0.6_f32);
        let organ_input = OrganInput::new(make_episode(), vec!["你好".to_string()]);
        let chain: OrganChainOutputs = orch.chain_9_organs(organ_input).await;
        let mood_high = orch.extract_emotion_mood(&chain);
        assert!(mood_high.is_some());
        let mood_high = mood_high.unwrap();
        assert!(
            (mood_high - 0.8).abs() < 1e-5,
            "mood_high 应 = 0.8, got {mood_high}"
        );
        assert!(
            mood_high >= OrchestratorLoopConfig::default().mood_floor,
            "mood_high >= mood_floor → tick 不应在 step 3 拦下"
        );
    }

    // Case 3: F1 organ 返 NotImplemented (Mock) → extract_emotion_mood 返 None
    // (Orchestrator 不假装"有情绪数据"; tick step 3 skip)
    {
        let orch_mock = build_orchestrator();
        let organ_input = OrganInput::new(make_episode(), vec![]);
        let chain_mock = orch_mock.chain_9_organs(organ_input).await;
        let mood_mock = orch_mock.extract_emotion_mood(&chain_mock);
        assert!(
            mood_mock.is_none(),
            "F1 organ 返 NotImplemented → extract_emotion_mood 应 None, got {mood_mock:?}"
        );
        // 确认 f1 真是 NotImplemented (sanity)
        if let Some(OrganOutput::NotImplemented { organ, .. }) = chain_mock.f1.as_ref() {
            assert_eq!(*organ, OrganKind::F1);
        } else {
            panic!("F1 应返 NotImplemented, got {:?}", chain_mock.f1);
        }
    }

    // Case 4: EmotionLow 真实触发路径 — tick + 低 pleasure F1 → 返 None + last_decision = Held(EmotionLow)
    // 注意: tick 步骤顺序 = 主权闸 → 9 organ 串联 + 8 gate → 情绪调制 (step 3) → ...
    // 低 pleasure F1 → mood = 0.1 < 0.3 → step 3 EmotionLow → tick 返 None
    // (必须先过主权闸 + 8 重 gate; 默认条件满足)
    {
        let mut orch_low = build_orchestrator_with_f1_pleasure(-0.8_f32);
        let outcome = orch_low.tick(make_tick_input(1_000_000)).await;
        assert!(
            outcome.is_none(),
            "低 pleasure F1 → tick 应在 step 3 EmotionLow 拦下"
        );
        assert_eq!(
            orch_low.last_decision(),
            Some(
                &apeireth_runtime::canonical::orchestrator::OrchestratorDecision::Held(
                    OrganOrchestratorGate::EmotionLow
                )
            ),
            "last_decision 应 = Held(EmotionLow)"
        );
    }

    // Case 5: 高 pleasure F1 → tick 正常走完 (Spoke)
    {
        let mut orch_high = build_orchestrator_with_f1_pleasure(0.6_f32);
        let outcome = orch_high.tick(make_tick_input(1_000_000)).await;
        assert!(
            outcome.is_some(),
            "高 pleasure F1 → tick 应通过 step 3, 走完正常路径"
        );
        let outcome = outcome.unwrap();
        assert_eq!(outcome.action_label, "问候");
        assert!(matches!(
            orch_high.last_decision(),
            Some(apeireth_runtime::canonical::orchestrator::OrchestratorDecision::Spoke { .. })
        ));
    }
}

// ============================================
// 测试 5: orchestrator_check_8_gates_e7_real (Stage 3 完整化)
// ============================================

/// check_8_gates() 接 E7 organ 真 gate (per v1 `EmergenceLoop::last_hold()` 1:1).
///
/// 验证:
/// - E7 organ 返 `OrganOutput::Emergence { gate: Some(InitiativeGate::RhythmUnknown), .. }`
///   → `extract_e7_gate()` 返 `Some(RhythmUnknown)`, `check_8_gates()` 返 `Some(RhythmUnknown)`.
/// - 同理 RhythmVeto / DriveLow 3 重都从 E7 chain 真实拿.
/// - E7 organ 返 `NotImplemented` (Mock) → `extract_e7_gate()` 返 None, `check_8_gates()` 不返 RhythmXxx.
/// - tick() 真实路径: E7 gate = RhythmVeto → tick 返 None + last_decision = Held(RhythmVeto)
#[tokio::test]
async fn orchestrator_check_8_gates_e7_real() {
    use apeireth_plugin::organ::InitiativeGate;
    use apeireth_runtime::canonical::orchestrator::OrganChainOutputs;
    use apeireth_plugin::organ::OrganOutput;

    // Case 1: E7 organ 返 RhythmUnknown → extract_e7_gate 返 Some → check_8_gates 返 RhythmUnknown
    {
        let orch = build_orchestrator_with_e7_gate(Some(InitiativeGate::RhythmUnknown));
        let organ_input = OrganInput::new(make_episode(), vec![]);
        let chain: OrganChainOutputs = orch.chain_9_organs(organ_input).await;
        let gate = orch.extract_e7_gate(&chain);
        assert_eq!(
            gate,
            Some(InitiativeGate::RhythmUnknown),
            "extract_e7_gate 应 = RhythmUnknown"
        );
        let check = orch.check_8_gates_for_test(720, 0, 0, None, &chain);
        assert_eq!(
            check,
            Some(OrganOrchestratorGate::RhythmUnknown),
            "check_8_gates 应从 e7.gate 返 RhythmUnknown"
        );
    }

    // Case 2: E7 organ 返 RhythmVeto → check_8_gates 返 RhythmVeto
    {
        let orch = build_orchestrator_with_e7_gate(Some(InitiativeGate::RhythmVeto));
        let organ_input = OrganInput::new(make_episode(), vec![]);
        let chain: OrganChainOutputs = orch.chain_9_organs(organ_input).await;
        let check = orch.check_8_gates_for_test(720, 0, 0, None, &chain);
        assert_eq!(check, Some(OrganOrchestratorGate::RhythmVeto));
    }

    // Case 3: E7 organ 返 DriveLow → check_8_gates 返 DriveLow
    {
        let orch = build_orchestrator_with_e7_gate(Some(InitiativeGate::DriveLow));
        let organ_input = OrganInput::new(make_episode(), vec![]);
        let chain: OrganChainOutputs = orch.chain_9_organs(organ_input).await;
        let check = orch.check_8_gates_for_test(720, 0, 0, None, &chain);
        assert_eq!(check, Some(OrganOrchestratorGate::DriveLow));
    }

    // Case 4: E7 organ 返 None gate (spoke=true 路径) → extract 返 None → check_8_gates 不返 RhythmXxx
    // (其他 5 重 gate 也 pass 默认 → 返 None)
    {
        let orch = build_orchestrator_with_e7_gate(None);
        let organ_input = OrganInput::new(make_episode(), vec![]);
        let chain: OrganChainOutputs = orch.chain_9_organs(organ_input).await;
        let gate = orch.extract_e7_gate(&chain);
        assert!(
            gate.is_none(),
            "E7 gate = None (spoke=true) → extract 应 None, got {gate:?}"
        );
        let check = orch.check_8_gates_for_test(720, 0, 1_000_000, Some(0), &chain);
        assert!(
            check.is_none(),
            "8 重 gate 全 pass + E7 gate None → check 应 None, got {check:?}"
        );
    }

    // Case 5: E7 organ 返 NotImplemented (Mock organ) → extract 返 None → check_8_gates 不返 RhythmXxx
    {
        let orch = build_orchestrator(); // 默认 Mock 9 organ, E7 返 NotImplemented
        let organ_input = OrganInput::new(make_episode(), vec![]);
        let chain: OrganChainOutputs = orch.chain_9_organs(organ_input).await;
        // 确认 e7 真是 NotImplemented
        if let Some(OrganOutput::NotImplemented { organ, .. }) = chain.e7.as_ref() {
            assert_eq!(*organ, OrganKind::E7);
        } else {
            panic!("E7 应返 NotImplemented, got {:?}", chain.e7);
        }
        let gate = orch.extract_e7_gate(&chain);
        assert!(gate.is_none(), "E7 = NotImplemented → extract None");
    }

    // Case 6: tick() 真实路径 — E7 gate = RhythmVeto → tick 返 None + last_decision = Held(RhythmVeto)
    // 注意: tick 顺序 = 主权闸 → chain_9_organs + check_8_gates → step 3 emotion → ...
    // 默认 OrchestratorBoundaries + LoopConfig (无 quiet hours / 不超 daily_limit / 不超 llm_interval /
    // depth=0.5 > min_depth=0.3). step 3 F1 = high pleasure (0.6), mood=0.8 > 0.3, 通过.
    // 故 RhythmVeto (E7 真算法) 是唯一拦下原因.
    {
        let mut orch_veto = build_orchestrator_with_e7_gate(Some(InitiativeGate::RhythmVeto));
        let outcome = orch_veto.tick(make_tick_input(1_000_000)).await;
        assert!(
            outcome.is_none(),
            "RhythmVeto gate → tick 应返 None"
        );
        assert_eq!(
            orch_veto.last_decision(),
            Some(
                &apeireth_runtime::canonical::orchestrator::OrchestratorDecision::Held(
                    OrganOrchestratorGate::RhythmVeto
                )
            ),
            "last_decision 应 = Held(RhythmVeto)"
        );
    }

    // Case 7: tick() 真实路径 — E7 gate = None + 所有 organ Mock (NotImplemented)
    // → check_8_gates 不返 RhythmXxx → step 3 F1 = NotImplemented skip → step 4 Council pass →
    // step 5 Policy Active → step 6 Sovereignty 未熔断 → Spoke
    // (此 case 类似 orchestrator_5_state_machine_transitions 默认场景, 加 Stage 3 E7 gate 接入验证)
    {
        let mut orch = build_orchestrator(); // E7 = NotImplemented, F1 = NotImplemented
        let outcome = orch.tick(make_tick_input(1_000_000)).await;
        assert!(
            outcome.is_some(),
            "Mock 9 organ + E7 None gate → tick 应正常走完, 返 Some"
        );
        assert!(matches!(
            orch.last_decision(),
            Some(apeireth_runtime::canonical::orchestrator::OrchestratorDecision::Spoke { .. })
        ));
    }
}
