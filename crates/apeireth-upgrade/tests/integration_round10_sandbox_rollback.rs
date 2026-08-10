//! round10-01 集成测试: Sandbox 阶段 + 反向状态机 (rollback reverse path).
//!
//! 覆盖:
//! 1. 完整 7 阶段 happy path: Idle -> IntentDraft -> CouncilReview -> MultiSig -> Sandbox -> Switchover -> Monitor -> Done
//! 2. Sandbox Reject (E-layer) -> Rollback, from_stage = Sandbox, 反向路径正确
//! 3. 反向状态机 rollback_reverse_path 各阶段 from_stage 触发点
//! 4. 任意阶段手动 rollback 都进入正确 from_stage
//! 5. 反向状态机覆盖 Monitor/Sandbox/Council/IntentDraft 四个采样点
//! 6. SEVEN_STAGES / REVERSE_STAGES 常量一致性

use apeireth_upgrade::{
    CouncilOpinion, CouncilReviewer, CouncilSeat, CouncilStance, DefaultSandbox,
    IntentStateMachine, IntentStatus, ManifestBuilder, MonitorDashboard, MonitorMetric,
    MonitorReport, MultiSigCollector, MultiSigConfig, OtaPipeline, OtaStage, PhysicalSignature,
    SandboxValidator, SandboxVerdict, UpgradeIntent, UpgradeKind, UpgradeManifest,
};
use uuid::Uuid;

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

fn sample_manifest() -> UpgradeManifest {
    ManifestBuilder::new("v1.1.0", UpgradeKind::Patch)
        .with_description("r10 sample")
        .with_content_hash("h")
        .build()
}

fn all_approve() -> Vec<CouncilOpinion> {
    CouncilSeat::ALL
        .iter()
        .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "ok"))
        .collect()
}

fn collect_5_of_7(intent: &UpgradeIntent) -> apeireth_upgrade::MultiSigOutcome {
    let hash = apeireth_upgrade::intent_payload_hash(intent);
    let cfg = MultiSigConfig::five_of_seven();
    let mut col = MultiSigCollector::new(cfg, hash.clone());
    for i in 0..5 {
        col.submit(PhysicalSignature::new(
            format!("signer-{i}"),
            hash.clone(),
            100 + i64::from(i),
            format!("sig{i}"),
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

/// 跑通 Idle -> Sandbox (默认 sandbox 接受).
fn drive_to_sandbox(p: &mut OtaPipeline, intent: &UpgradeIntent) {
    p.enter_council_review(CouncilReviewer::new().review(intent, all_approve()))
        .unwrap();
    p.enter_multisig(collect_5_of_7(intent)).unwrap();
    let sandbox = DefaultSandbox;
    p.enter_sandbox(
        intent.id,
        "blue".into(),
        "green".into(),
        &sample_manifest(),
        &sandbox,
    )
    .unwrap();
}

/// ============== 1. 完整 7 阶段 happy path ==============
#[test]
fn integration_r10_full_happy_path_intent_to_done() {
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    assert_eq!(p.stage(), OtaStage::IntentDraft);

    drive_to_sandbox(&mut p, &intent);
    assert_eq!(p.stage(), OtaStage::Sandbox);

    p.enter_switchover().unwrap();
    assert_eq!(p.stage(), OtaStage::Switchover);

    let report = healthy_report();
    p.enter_monitor(report.clone()).unwrap();
    assert_eq!(p.stage(), OtaStage::Monitor);

    let term = p.finalize(report).unwrap();
    assert_eq!(term, OtaStage::Done);
    assert!(p.state().is_success());
    assert!(p.state().is_terminal());
}

/// ============== 2. Sandbox Reject (E-layer) -> Rollback ==============
#[test]
fn integration_r10_sandbox_rejects_e_layer_triggers_rollback() {
    let intent = sample_intent();
    let e_layer_manifest = ManifestBuilder::new("v2.0.0", UpgradeKind::ELayerMutation)
        .with_content_hash("e")
        .build();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    p.enter_council_review(CouncilReviewer::new().review(&intent, all_approve()))
        .unwrap();
    p.enter_multisig(collect_5_of_7(&intent)).unwrap();

    let sandbox = DefaultSandbox;
    p.enter_sandbox(
        intent.id,
        "blue".into(),
        "green".into(),
        &e_layer_manifest,
        &sandbox,
    )
    .unwrap();
    assert_eq!(p.stage(), OtaStage::Rollback);

    // 验证 from_stage 正确
    match p.state() {
        apeireth_upgrade::OtaState::Rollback { from_stage, .. } => {
            assert_eq!(*from_stage, OtaStage::Sandbox);
        }
        _ => panic!("expected Rollback"),
    }

    // 反向路径正确
    let path = p.state().rollback_reverse_path();
    assert_eq!(
        path,
        vec![
            OtaStage::Sandbox,
            OtaStage::MultiSig,
            OtaStage::CouncilReview,
            OtaStage::IntentDraft,
            OtaStage::Idle,
        ]
    );
}

/// ============== 3. 反向状态机: 任意阶段 rollback reverse path ==============
#[test]
fn integration_r10_rollback_reverse_path_from_each_stage() {
    // Monitor → 完整 7 阶段反向
    let s = apeireth_upgrade::OtaState::Rollback {
        reason: "monitor".into(),
        from_stage: OtaStage::Monitor,
    };
    assert_eq!(
        s.rollback_reverse_path(),
        vec![
            OtaStage::Monitor,
            OtaStage::Switchover,
            OtaStage::Sandbox,
            OtaStage::MultiSig,
            OtaStage::CouncilReview,
            OtaStage::IntentDraft,
            OtaStage::Idle,
        ]
    );

    // Switchover → 6 阶段
    let s = apeireth_upgrade::OtaState::Rollback {
        reason: "switchover".into(),
        from_stage: OtaStage::Switchover,
    };
    assert_eq!(
        s.rollback_reverse_path(),
        vec![
            OtaStage::Switchover,
            OtaStage::Sandbox,
            OtaStage::MultiSig,
            OtaStage::CouncilReview,
            OtaStage::IntentDraft,
            OtaStage::Idle,
        ]
    );

    // Sandbox → 5 阶段
    let s = apeireth_upgrade::OtaState::Rollback {
        reason: "sandbox".into(),
        from_stage: OtaStage::Sandbox,
    };
    assert_eq!(
        s.rollback_reverse_path(),
        vec![
            OtaStage::Sandbox,
            OtaStage::MultiSig,
            OtaStage::CouncilReview,
            OtaStage::IntentDraft,
            OtaStage::Idle,
        ]
    );

    // MultiSig → 4 阶段
    let s = apeireth_upgrade::OtaState::Rollback {
        reason: "multisig".into(),
        from_stage: OtaStage::MultiSig,
    };
    assert_eq!(
        s.rollback_reverse_path(),
        vec![
            OtaStage::MultiSig,
            OtaStage::CouncilReview,
            OtaStage::IntentDraft,
            OtaStage::Idle,
        ]
    );

    // CouncilReview → 3 阶段
    let s = apeireth_upgrade::OtaState::Rollback {
        reason: "council".into(),
        from_stage: OtaStage::CouncilReview,
    };
    assert_eq!(
        s.rollback_reverse_path(),
        vec![
            OtaStage::CouncilReview,
            OtaStage::IntentDraft,
            OtaStage::Idle
        ]
    );

    // IntentDraft → 2 阶段
    let s = apeireth_upgrade::OtaState::Rollback {
        reason: "intent".into(),
        from_stage: OtaStage::IntentDraft,
    };
    assert_eq!(
        s.rollback_reverse_path(),
        vec![OtaStage::IntentDraft, OtaStage::Idle]
    );
}

/// ============== 4. 任意阶段手动 rollback 都进入正确 from_stage ==============
#[test]
fn integration_r10_manual_rollback_records_correct_from_stage() {
    // Sandbox 阶段手动 rollback
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    drive_to_sandbox(&mut p, &intent);
    assert_eq!(p.stage(), OtaStage::Sandbox);

    p.rollback("abort at sandbox").unwrap();
    assert_eq!(p.stage(), OtaStage::Rollback);
    match p.state() {
        apeireth_upgrade::OtaState::Rollback { from_stage, reason } => {
            assert_eq!(*from_stage, OtaStage::Sandbox);
            assert_eq!(reason, "abort at sandbox");
        }
        _ => panic!("expected Rollback"),
    }

    // MultiSig 阶段手动 rollback
    let intent2 = sample_intent();
    let mut p2 = OtaPipeline::new(OtaStage::Idle);
    p2.start_intent(intent2.clone()).unwrap();
    p2.enter_council_review(CouncilReviewer::new().review(&intent2, all_approve()))
        .unwrap();
    p2.enter_multisig(collect_5_of_7(&intent2)).unwrap();
    assert_eq!(p2.stage(), OtaStage::MultiSig);

    p2.rollback("abort at multisig").unwrap();
    match p2.state() {
        apeireth_upgrade::OtaState::Rollback { from_stage, .. } => {
            assert_eq!(*from_stage, OtaStage::MultiSig);
        }
        _ => panic!("expected Rollback from MultiSig"),
    }
}

/// ============== 5. 反向状态机 + Done 终态 vs Rollback 终态对比 ==============
#[test]
fn integration_r10_done_state_no_reverse_path_rollback_has_one() {
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    drive_to_sandbox(&mut p, &intent);
    p.enter_switchover().unwrap();
    p.enter_monitor(healthy_report()).unwrap();
    p.finalize(healthy_report()).unwrap();
    assert_eq!(p.stage(), OtaStage::Done);

    // Done 终态无反向路径
    assert!(p.state().rollback_reverse_path().is_empty());

    // 手动 rollback (强制) — 在 Done 状态下应失败
    let err = p.rollback("force").unwrap_err();
    assert!(matches!(
        err,
        apeireth_upgrade::UpgradeError::IllegalTransition(_, _)
    ));
}

/// ============== 6. SEVEN_STAGES / REVERSE_STAGES 常量一致性 ==============
#[test]
fn integration_r10_seven_stages_and_reverse_stages_invariant() {
    let seven = OtaStage::SEVEN_STAGES;
    let reverse = OtaStage::REVERSE_STAGES;

    // SEVEN_STAGES: 不含 Idle / Rollback, 含 Done
    assert_eq!(seven.len(), 7);
    assert!(seven.contains(&OtaStage::Sandbox));
    assert!(seven.contains(&OtaStage::Done));
    assert!(!seven.contains(&OtaStage::Idle));
    assert!(!seven.contains(&OtaStage::Rollback));

    // REVERSE_STAGES: 含 Idle 结尾, 不含 Done
    assert_eq!(reverse.len(), 7);
    assert_eq!(reverse[0], OtaStage::Monitor);
    assert_eq!(reverse[reverse.len() - 1], OtaStage::Idle);
    assert!(reverse.contains(&OtaStage::Sandbox));
    assert!(!reverse.contains(&OtaStage::Done));
    assert!(!reverse.contains(&OtaStage::Rollback));

    // 两者交集: 6 个阶段相同 (除了 Idle 在 reverse, Done 在 seven)
    let seven_set: std::collections::HashSet<_> = seven.iter().copied().collect();
    let reverse_set: std::collections::HashSet<_> = reverse.iter().copied().collect();
    let intersection: usize = seven_set.intersection(&reverse_set).count();
    assert_eq!(intersection, 6); // 共享 6 个阶段
}

/// ============== 7. Sandbox validator trait 多态性验证 ==============
struct AcceptAllSandbox;
impl SandboxValidator for AcceptAllSandbox {
    fn validate(&self, _manifest: &UpgradeManifest) -> SandboxVerdict {
        SandboxVerdict::Accept
    }
}

#[test]
fn integration_r10_sandbox_validator_trait_polymorphism() {
    // 用自定义 AcceptAllSandbox 替代 DefaultSandbox — 应同样通过
    let intent = sample_intent();
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    p.enter_council_review(CouncilReviewer::new().review(&intent, all_approve()))
        .unwrap();
    p.enter_multisig(collect_5_of_7(&intent)).unwrap();

    let custom = AcceptAllSandbox;
    p.enter_sandbox(
        intent.id,
        "b".into(),
        "g".into(),
        &sample_manifest(),
        &custom,
    )
    .unwrap();
    assert_eq!(p.stage(), OtaStage::Sandbox);
    match p.state() {
        apeireth_upgrade::OtaState::Sandboxed { verdict, .. } => {
            assert!(matches!(verdict, SandboxVerdict::Accept));
        }
        _ => panic!("expected Sandboxed"),
    }
}

/// ============== 8. Intent 状态机不变性 (Sandbox 升级不影响) ==============
#[test]
fn integration_r10_intent_state_machine_unchanged() {
    let intent = sample_intent();
    let mut sm = IntentStateMachine::wrap(intent);
    assert_eq!(sm.status(), IntentStatus::Drafting);
    sm.submit().unwrap();
    assert_eq!(sm.status(), IntentStatus::Submitted);
    sm.approve().unwrap();
    assert_eq!(sm.status(), IntentStatus::Approved);
}
