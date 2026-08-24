//! round6-03 7 阶段集成测试.
//!
//! 覆盖:
//! 1. 完整 7 阶段 happy path (Intent → Done)
//! 2. Council hold → Rollback
//! 3. MultiSig timeout → Rollback
//! 4. Monitor failed → Rollback
//! 5. Manifest validate 拒绝 (sandbox 守门失败)
//! 6. E-layer mutation 默认拒绝
//! 7. 任意阶段手动 rollback
//! 8. 终态锁定 (Done 后无法再 rollback)

use apeireth_upgrade::{
    CouncilOpinion, CouncilReviewer, CouncilSeat, CouncilStance, DefaultSandbox,
    FiveFoldGovernance, Governance, GovernanceDecision, IntentStateMachine, IntentStatus,
    ManifestBuilder, MonitorDashboard, MonitorMetric, MonitorReport, MultiSigCollector,
    MultiSigConfig, MultiSigOutcome, OtaPipeline, OtaStage, PhysicalSignature, SandboxValidator,
    SandboxVerdict, UpgradeIntent, UpgradeKind, UpgradeManifest,
};
use uuid::Uuid;

fn sample_manifest() -> UpgradeManifest {
    ManifestBuilder::new("v1.0.0", UpgradeKind::Patch)
        .with_description("integration")
        .with_content_hash("abc")
        .build()
}

fn sample_intent() -> UpgradeIntent {
    UpgradeIntent::new(
        Uuid::new_v4(),
        "v1.1.0",
        "v1.0.0",
        UpgradeKind::Patch,
        "carrier-a",
        "fix",
    )
}

fn all_approve() -> Vec<CouncilOpinion> {
    CouncilSeat::ALL
        .iter()
        .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "ok"))
        .collect()
}

fn collect_5_of_7(intent: &UpgradeIntent) -> MultiSigOutcome {
    let hash = apeireth_upgrade::intent_payload_hash(intent);
    let cfg = MultiSigConfig::five_of_seven();
    let mut col = MultiSigCollector::new(cfg, hash.clone());
    for i in 0..5 {
        col.submit(PhysicalSignature::new(
            format!("signer-{i}"),
            hash.clone(),
            100 + i64::from(i),
            format!("sig-{i}"),
        ))
        .unwrap();
    }
    col.evaluate(200)
}

fn healthy_report() -> MonitorReport {
    let mut d = MonitorDashboard::new();
    d.record(MonitorMetric::new("a", 0.01, Some(0.05), None));
    d.record(MonitorMetric::new("b", 100.0, Some(500.0), None));
    d.report()
}

#[test]
fn integration_7_stage_happy_path_completes_done() {
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);

    // 阶段 1/7 Intent
    p.start_intent(intent.clone()).unwrap();
    assert_eq!(p.stage(), OtaStage::IntentDraft);
    assert!(p.stage().is_active());

    // 阶段 2/7 Council
    let report = CouncilReviewer::new().review(&intent, all_approve());
    p.enter_council_review(report).unwrap();
    assert_eq!(p.stage(), OtaStage::CouncilReview);
    assert!(p.stage().is_active());

    // 阶段 3/7 MultiSig
    let outcome = collect_5_of_7(&intent);
    p.enter_multisig(outcome).unwrap();
    assert_eq!(p.stage(), OtaStage::MultiSig);

    // 阶段 4/7 Sandbox (round10-01 升级: 取代 round6-03 的 Download 阶段)
    let sandbox = DefaultSandbox;
    let sandbox_manifest = ManifestBuilder::new("v1.1.0", UpgradeKind::Patch)
        .with_description("integration sandbox")
        .with_content_hash("h")
        .build();
    p.enter_sandbox(
        intent.id,
        "blue".into(),
        "green".into(),
        &sandbox_manifest,
        &sandbox,
    )
    .unwrap();
    assert_eq!(p.stage(), OtaStage::Sandbox);

    // 阶段 5/7 Switchover (蓝绿)
    p.enter_switchover().unwrap();
    assert_eq!(p.stage(), OtaStage::Switchover);

    // 阶段 6/7 Monitor
    p.enter_monitor(healthy_report()).unwrap();
    assert_eq!(p.stage(), OtaStage::Monitor);

    // 阶段 7/7 Done
    let term = p.finalize(healthy_report()).unwrap();
    assert_eq!(term, OtaStage::Done);
    assert!(p.state().is_success());
    assert!(p.state().is_terminal());
    assert!(!p.state().is_rollback());

    // OtaState::stage() 一致
    assert_eq!(p.state().stage(), OtaStage::Done);
}

#[test]
fn integration_council_hold_short_circuits_to_rollback() {
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();

    let mut ops = all_approve();
    ops[0] = CouncilOpinion::new(
        CouncilSeat::Principle,
        CouncilStance::StrongDisapprove,
        0.95,
        "violates principle",
    );
    let report = CouncilReviewer::new().review(&intent, ops);
    p.enter_council_review(report).unwrap();

    // 按住 → 直接 Rollback, 不进入 MultiSig
    assert_eq!(p.stage(), OtaStage::Rollback);
    assert!(p.state().is_rollback());
    if let apeireth_upgrade::OtaState::Rollback { reason, from_stage } = p.state() {
        assert!(reason.contains("council"));
        assert_eq!(*from_stage, OtaStage::CouncilReview);
    } else {
        panic!("expected Rollback state");
    }
}

#[test]
fn integration_multisig_timeout_short_circuits_to_rollback() {
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();

    let report = CouncilReviewer::new().review(&intent, all_approve());
    p.enter_council_review(report).unwrap();

    // 仅 1 签, deadline=100, 当前时间=500 -> Timeout
    let hash = apeireth_upgrade::intent_payload_hash(&intent);
    let cfg = MultiSigConfig::new(2, vec!["a".into(), "b".into()]).with_deadline(100);
    let mut col = MultiSigCollector::new(cfg, hash.clone());
    col.submit(PhysicalSignature::new("a", hash, 50, "sig-a"))
        .unwrap();
    let outcome = col.evaluate(500);
    assert!(matches!(outcome, MultiSigOutcome::Timeout { .. }));

    p.enter_multisig(outcome).unwrap();
    assert_eq!(p.stage(), OtaStage::Rollback);
}

#[test]
fn integration_monitor_failure_rolls_back_done() {
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    p.enter_council_review(CouncilReviewer::new().review(&intent, all_approve()))
        .unwrap();
    p.enter_multisig(collect_5_of_7(&intent)).unwrap();
    let sandbox = DefaultSandbox;
    let m = ManifestBuilder::new("v1.1.0", UpgradeKind::Patch)
        .with_content_hash("h")
        .build();
    p.enter_sandbox(intent.id, "b".into(), "g".into(), &m, &sandbox)
        .unwrap();
    p.enter_switchover().unwrap();
    p.enter_monitor(healthy_report()).unwrap();

    // 失败监控: error_rate = 0.99 远超 0.05 阈值
    let mut d = MonitorDashboard::new();
    d.record(MonitorMetric::new("err", 0.99, Some(0.05), None));
    let bad = d.report();
    assert!(bad.should_rollback());

    let term = p.finalize(bad).unwrap();
    assert_eq!(term, OtaStage::Rollback);
    assert!(p.state().is_rollback());
}

#[test]
fn integration_manifest_validation_blocks_at_entry() {
    // sandbox 拒绝空 version
    let bad = ManifestBuilder::new("", UpgradeKind::Patch)
        .with_content_hash("h")
        .build();
    let sb = DefaultSandbox;
    let v = sb.validate(&bad);
    assert!(matches!(v, SandboxVerdict::Reject(_)));

    // governance 拒绝 E-layer mutation (默认保守)
    let e = ManifestBuilder::new("v2.0.0", UpgradeKind::ELayerMutation)
        .with_content_hash("h")
        .build();
    let g = FiveFoldGovernance::default();
    let d = g.evaluate(&e);
    assert!(matches!(d, GovernanceDecision::Reject(_)));
}

#[test]
fn integration_manual_rollback_at_any_non_terminal_stage() {
    // IntentDraft 阶段手动 rollback
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent).unwrap();
    p.rollback("operator abort").unwrap();
    assert_eq!(p.stage(), OtaStage::Rollback);
}

#[test]
fn integration_done_state_blocks_further_rollback() {
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    p.enter_council_review(CouncilReviewer::new().review(&intent, all_approve()))
        .unwrap();
    p.enter_multisig(collect_5_of_7(&intent)).unwrap();
    let sandbox = DefaultSandbox;
    let m = ManifestBuilder::new("v1.1.0", UpgradeKind::Patch)
        .with_content_hash("h")
        .build();
    p.enter_sandbox(intent.id, "b".into(), "g".into(), &m, &sandbox)
        .unwrap();
    p.enter_switchover().unwrap();
    p.enter_monitor(healthy_report()).unwrap();
    p.finalize(healthy_report()).unwrap();
    assert_eq!(p.stage(), OtaStage::Done);

    // 终态 rollback 必须失败
    let err = p.rollback("post-done abort").unwrap_err();
    match err {
        apeireth_upgrade::UpgradeError::IllegalTransition(from, to) => {
            assert_eq!(from, OtaStage::Done);
            assert_eq!(to, OtaStage::Rollback);
        }
        _ => panic!("expected IllegalTransition"),
    }
}

#[test]
fn integration_intent_state_machine_rejects_illegal_transitions() {
    let intent = sample_intent();
    let mut sm = IntentStateMachine::wrap(intent);
    // 不提交直接 approve
    assert!(sm.approve().is_err());
    // 提交后 reject
    sm.submit().unwrap();
    sm.reject().unwrap();
    assert_eq!(sm.status(), IntentStatus::Rejected);
    // 终态后任何操作都失败
    assert!(sm.approve().is_err());
    assert!(sm.withdraw().is_err());
    assert!(sm.submit().is_err());
}

#[test]
fn integration_run_upgrade_public_api_succeeds_for_patch() {
    // 公开 API run_upgrade 应跑通 7 阶段 + 返回 Done
    let manifest = sample_manifest();
    let state = apeireth_upgrade::run_upgrade(&manifest).expect("run_upgrade succeeds");
    assert!(state.is_success(), "got {state:?}");
    assert!(state.is_terminal());
}

#[test]
fn integration_run_upgrade_rejects_e_layer_via_governance() {
    let e = ManifestBuilder::new("v2.0.0", UpgradeKind::ELayerMutation)
        .with_content_hash("h")
        .build();
    let res = apeireth_upgrade::run_upgrade(&e);
    assert!(res.is_err());
}
