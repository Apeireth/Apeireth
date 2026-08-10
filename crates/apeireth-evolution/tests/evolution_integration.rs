//! `apeireth-evolution` integration tests
//!
//! 覆盖端到端场景: state machine + fail-6 + council 集成 + L0 防护。

use apeireth_council::CouncilEvent;
use apeireth_evolution::council_bridge::{
    all_approve_report, build_verdict_from_report, held_report, reject_report, CouncilAdapter,
    CouncilIntegrationConfig, EvolutionOutcome, EvolutionProposal,
};
use apeireth_evolution::engine::{EvolutionEngine, EvolutionStep};
use apeireth_evolution::fail::{FailKind, FailOutcome, StrictFailPolicy};
use apeireth_evolution::state::{EvolutionState, TransitionReason};
use apeireth_evolution::{
    Abstraction, BasicEvolution, Episode, EvolutionStateMachine, Extension, FailPolicy, FailRecord,
    Learning, MockPlugin, Plugin, PluginKind, SelfModification, SystemState, L0_ANCHOR,
};

fn now() -> i64 {
    apeireth_evolution::current_time_ms()
}

/// Integration #1: 6 状态机 happy path
#[test]
fn integration_six_state_happy_path() {
    let mut m = EvolutionStateMachine::new();
    assert_eq!(m.current, EvolutionState::Idle);

    m.transition(EvolutionState::Draft, TransitionReason::Start, now())
        .unwrap();
    m.transition(EvolutionState::Proposed, TransitionReason::Submit, now())
        .unwrap();
    m.transition(
        EvolutionState::Ratified,
        TransitionReason::CouncilApprove,
        now(),
    )
    .unwrap();
    m.transition(EvolutionState::Active, TransitionReason::Activate, now())
        .unwrap();

    assert_eq!(m.current, EvolutionState::Active);
    assert_eq!(m.history.len(), 4);
    // 4 个转换都合法
    assert!(m
        .history
        .iter()
        .all(|t| EvolutionStateMachine::is_legal(t.from, t.to)));
}

/// Integration #2: L0 防护触发的全路径
#[test]
fn integration_l0_guard_protects() {
    let mut e: EvolutionEngine = EvolutionEngine::new("p-l0", StrictFailPolicy);
    e.start(now()).unwrap();
    assert_eq!(e.current_state(), EvolutionState::Draft);

    e.guard_l0("L0", now()).unwrap();
    assert_eq!(e.current_state(), EvolutionState::Retired);

    // 验证 log 记录 L0Guard step
    let log = e.log();
    let has_l0 = log
        .steps
        .iter()
        .any(|s| matches!(s, EvolutionStep::L0GuardTriggered { .. }));
    assert!(has_l0);
}

/// Integration #3: trait fail-6 全部 6 类可触发
#[test]
fn integration_fail_six_all_kinds() {
    let policy = StrictFailPolicy;

    // ReflectionFailure (Draft → Retired)
    let mut m1 = EvolutionStateMachine::new();
    m1.transition(EvolutionState::Draft, TransitionReason::Start, now())
        .unwrap();
    let rec = FailRecord::new(FailKind::ReflectionFailure, "x", now());
    assert_eq!(
        policy
            .apply_fail(&mut m1, FailKind::ReflectionFailure, &rec, now())
            .unwrap(),
        FailOutcome::Retired
    );

    // CouncilRejectFailure (Proposed → Retired)
    let mut m2 = EvolutionStateMachine::new();
    m2.transition(EvolutionState::Draft, TransitionReason::Start, now())
        .unwrap();
    m2.transition(EvolutionState::Proposed, TransitionReason::Submit, now())
        .unwrap();
    let rec = FailRecord::new(FailKind::CouncilRejectFailure, "x", now());
    assert_eq!(
        policy
            .apply_fail(&mut m2, FailKind::CouncilRejectFailure, &rec, now())
            .unwrap(),
        FailOutcome::Retired
    );

    // CouncilHoldFailure (Proposed → Draft, retry)
    let mut m3 = EvolutionStateMachine::new();
    m3.transition(EvolutionState::Draft, TransitionReason::Start, now())
        .unwrap();
    m3.transition(EvolutionState::Proposed, TransitionReason::Submit, now())
        .unwrap();
    let rec = FailRecord::new(FailKind::CouncilHoldFailure, "x", now()).with_retry(1);
    let outcome = policy
        .apply_fail(&mut m3, FailKind::CouncilHoldFailure, &rec, now())
        .unwrap();
    assert!(matches!(
        outcome,
        FailOutcome::RetriedToDraft { attempt: 1 }
    ));
    assert_eq!(m3.current, EvolutionState::Draft);

    // ActivationTimeoutFailure (Ratified → Retired)
    let mut m4 = EvolutionStateMachine::new();
    m4.transition(EvolutionState::Draft, TransitionReason::Start, now())
        .unwrap();
    m4.transition(EvolutionState::Proposed, TransitionReason::Submit, now())
        .unwrap();
    m4.transition(
        EvolutionState::Ratified,
        TransitionReason::CouncilApprove,
        now(),
    )
    .unwrap();
    let rec = FailRecord::new(FailKind::ActivationTimeoutFailure, "x", now());
    assert_eq!(
        policy
            .apply_fail(&mut m4, FailKind::ActivationTimeoutFailure, &rec, now())
            .unwrap(),
        FailOutcome::Retired
    );

    // OutOfReflectionWindowFailure (任何 → Retired)
    let mut m5 = EvolutionStateMachine::new();
    m5.transition(EvolutionState::Draft, TransitionReason::Start, now())
        .unwrap();
    let rec = FailRecord::new(FailKind::OutOfReflectionWindowFailure, "x", now());
    assert_eq!(
        policy
            .apply_fail(&mut m5, FailKind::OutOfReflectionWindowFailure, &rec, now())
            .unwrap(),
        FailOutcome::Retired
    );

    // IntegrityCheckFailure (Draft → Retired)
    let mut m6 = EvolutionStateMachine::new();
    m6.transition(EvolutionState::Draft, TransitionReason::Start, now())
        .unwrap();
    let rec = FailRecord::new(FailKind::IntegrityCheckFailure, "x", now());
    assert_eq!(
        policy
            .apply_fail(&mut m6, FailKind::IntegrityCheckFailure, &rec, now())
            .unwrap(),
        FailOutcome::Retired
    );
}

/// Integration #4: 多轮 CouncilHold retry 直到 budget 耗尽
#[test]
fn integration_retry_budget_exhaustion() {
    let mut e: EvolutionEngine = apeireth_evolution::EvolutionEngine::with_config(
        "p-retry",
        apeireth_evolution::engine::EngineConfig {
            reflection_window_ms: 60_000,
            max_retry: 2,
        },
        StrictFailPolicy,
    );

    e.start(now()).unwrap();
    e.submit(now()).unwrap();
    assert_eq!(e.current_state(), EvolutionState::Proposed);

    // 第 1 次 retry
    e.apply_fail(FailKind::CouncilHoldFailure, "h1", now())
        .unwrap();
    assert_eq!(e.current_state(), EvolutionState::Draft);
    assert_eq!(e.retry_count(), 1);

    e.submit(now()).unwrap();
    // 第 2 次 retry
    e.apply_fail(FailKind::CouncilHoldFailure, "h2", now())
        .unwrap();
    assert_eq!(e.current_state(), EvolutionState::Draft);
    assert_eq!(e.retry_count(), 2);

    e.submit(now()).unwrap();
    // 第 3 次 — 预算耗尽 → Retired
    let outcome = e
        .apply_fail(FailKind::CouncilHoldFailure, "h3", now())
        .unwrap();
    assert_eq!(outcome, FailOutcome::Retired);
    assert!(e.current_state().is_terminal());
}

/// Integration #5: CouncilAdapter 端到端 (allowed verdict)
#[test]
fn integration_council_adapter_allowed() {
    let mut e: EvolutionEngine = EvolutionEngine::new("p-council", StrictFailPolicy);
    e.start(now()).unwrap();
    e.submit(now()).unwrap();

    let report = all_approve_report(now());
    let verdict = build_verdict_from_report(
        &CouncilEvent::DeliberationCompleted {
            session_id: "s".into(),
            report: report.clone(),
            elapsed_ms: 50,
        },
        &report,
    );

    let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
    let outcome = adapter.handle_council_verdict(&verdict, now()).unwrap();
    assert_eq!(outcome, EvolutionOutcome::Ratified);
    assert_eq!(e.current_state(), EvolutionState::Ratified);

    // 继续激活
    e.activate(now()).unwrap();
    assert_eq!(e.current_state(), EvolutionState::Active);
    assert!(e.log().succeeded);
}

/// Integration #6: CouncilAdapter held verdict → retry
#[test]
fn integration_council_adapter_held_retries() {
    let mut e: EvolutionEngine = EvolutionEngine::new("p-council-hold", StrictFailPolicy);
    e.start(now()).unwrap();
    e.submit(now()).unwrap();

    let report = held_report(now());
    let verdict = build_verdict_from_report(
        &CouncilEvent::DeliberationCompleted {
            session_id: "s".into(),
            report: report.clone(),
            elapsed_ms: 50,
        },
        &report,
    );

    // 第 1 次 hold → retry
    {
        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let outcome = adapter.handle_council_verdict(&verdict, now()).unwrap();
        assert!(matches!(outcome, EvolutionOutcome::Retried { .. }));
    }
    assert_eq!(e.current_state(), EvolutionState::Draft);
    assert_eq!(e.retry_count(), 1);

    // 重新提交 → 第二次 hold → retry_count=2
    e.submit(now()).unwrap();
    {
        let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
        let outcome2 = adapter.handle_council_verdict(&verdict, now()).unwrap();
        assert!(matches!(outcome2, EvolutionOutcome::Retried { .. }));
    }
    assert_eq!(e.retry_count(), 2);
}

/// Integration #7: CouncilAdapter rejected verdict → Retired
#[test]
fn integration_council_adapter_rejected() {
    let mut e: EvolutionEngine = EvolutionEngine::new("p-reject", StrictFailPolicy);
    e.start(now()).unwrap();
    e.submit(now()).unwrap();

    let report = reject_report(now());
    let verdict = build_verdict_from_report(
        &CouncilEvent::DeliberationCompleted {
            session_id: "s".into(),
            report: report.clone(),
            elapsed_ms: 50,
        },
        &report,
    );

    let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
    let outcome = adapter.handle_council_verdict(&verdict, now()).unwrap();
    assert!(matches!(outcome, EvolutionOutcome::Rejected { .. }));
    assert!(e.current_state().is_terminal());
}

/// Integration #8: L0 proposal 立即 Retired
#[test]
fn integration_l0_proposal_blocked_at_guard() {
    let mut e: EvolutionEngine = EvolutionEngine::new("p-l0-block", StrictFailPolicy);
    e.start(now()).unwrap();

    let mut adapter = CouncilAdapter::new(&mut e, CouncilIntegrationConfig::default());
    let proposal = EvolutionProposal::new("p-l0-block", "try touching L0", "L0", "nuclear");
    let ok = adapter.guard_proposal(&proposal, now()).unwrap();
    assert!(!ok, "L0 proposal must be blocked");
    assert_eq!(e.current_state(), EvolutionState::Retired);

    // Engine 的 l0_anchor 应等于常量
    assert_eq!(e.l0_anchor(), L0_ANCHOR);
}

/// Integration #9: BasicEvolution 4 trait 协同
#[test]
fn integration_basic_evolution_four_traits() {
    let mut e = BasicEvolution::new();

    // 1. Learning
    let episodes = vec![
        Episode::new("1", "auth.login", "check", "ok", now()),
        Episode::new("2", "auth.refresh", "check", "ok", now()),
        Episode::new("3", "auth.refresh", "check", "ok", now()),
        Episode::new("4", "auth.refresh", "check", "ok", now()),
    ];
    for ep in &episodes {
        e.learn(ep).unwrap();
    }
    let score = e.knowledge_score();
    assert!(score > 0.0);

    // 2. Abstraction
    let concept = e.abstract_concept(&episodes).unwrap();
    assert!(concept.name.starts_with("auth"));
    assert_eq!(concept.example_count, 4);

    // 3. SelfModification
    let sys = SystemState::new("L3", 85);
    let patch = e.propose_patch(&sys);
    assert_eq!(patch.target_layer, "L3");
    assert_eq!(patch.risk, "low");
    assert!(!patch.targets_l0());

    // 4. Extension
    let p1: Box<dyn Plugin> = Box::new(MockPlugin::new("p1", PluginKind::Tool, "x"));
    let p2: Box<dyn Plugin> = Box::new(MockPlugin::new("s1", PluginKind::Sensor, "y"));
    let p3: Box<dyn Plugin> = Box::new(MockPlugin::new("a1", PluginKind::Action, "z"));
    e.extend_capability(p1).unwrap();
    e.extend_capability(p2).unwrap();
    e.extend_capability(p3).unwrap();
    assert_eq!(e.plugin_count(), 3);

    // reset
    e.reset();
    assert_eq!(e.knowledge_score(), 0.0);
}

/// Integration #10: EvolutionEngine 端到端 happy path
#[test]
fn integration_engine_end_to_end_active() {
    let mut e: EvolutionEngine = EvolutionEngine::new("p-end", StrictFailPolicy);
    e.start(now()).unwrap();
    e.submit(now()).unwrap();
    e.mark_ratified(now()).unwrap();
    e.activate(now()).unwrap();

    assert_eq!(e.current_state(), EvolutionState::Active);
    let log = e.log();
    assert!(log.succeeded);
    assert!(log.ended_at_ms.is_some());
    assert!(log
        .steps
        .iter()
        .any(|s| matches!(s, EvolutionStep::Activated { .. })));
}
