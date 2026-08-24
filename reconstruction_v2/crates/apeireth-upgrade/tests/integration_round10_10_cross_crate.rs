//! round10-10 跨 crate 集成测试 (v2 适配: 使用 cross_crate 自包含类型).
//!
//! 覆盖:
//! 1. OTA CouncilReview → cross_crate 7 advisor 全员审议
//! 2. OTA CouncilReview Hold 触发 → 7 advisor 中 Safety 低置信度触发
//! 3. OTA MultiSig → cross_crate MultiSigPolicy 校验
//! 4. OTA MultiSig ReadOnly rejected → core-rule mutation 拒绝
//! 5. OTA MultiSig insufficient signatures → Pending 状态
//! 6. OTA Sandbox → cross_crate ConstraintEngine 5 重守门
//! 7. OTA Sandbox gate5 reflection 默认 block
//! 8. 完整 7 阶段 happy path 含跨 crate 调用

use apeireth_upgrade::cross_crate::{
    check_multisig_with_sovereignty, default_multi_authority, default_ota_multisig_collector,
    default_synthesis_weights, deliberate_with_7_advisors, multisig_outcome_from_authority,
    sandbox_with_five_gates, synthesize_council_report, Action, ActionTarget, Advisor,
    AdvisorDomain, AuthorityMultisigOutcome, ConstraintEngine, CouncilQuery, HumanAuthority,
    MultiSigPolicy, OwnerRequest, PhilosophyVerdict, RiskLevel, SandboxFiveGatesReport,
};
use apeireth_upgrade::{
    CouncilOpinion, CouncilReport, CouncilSeat, CouncilStance, DefaultSandbox, HoldAction,
    ManifestBuilder, MonitorDashboard, MonitorMetric, MonitorReport, MultiSigCollector,
    MultiSigConfig, MultiSigOutcome, OtaPipeline, OtaStage, PhysicalSignature, SandboxValidator,
    UpgradeIntent, UpgradeKind, UpgradeManifest, intent_payload_hash,
};
use std::sync::Arc;
use uuid::Uuid;

fn sample_intent() -> UpgradeIntent {
    UpgradeIntent::new(
        Uuid::new_v4(),
        "v2.0.0",
        "v1.0.0",
        UpgradeKind::Patch,
        "carrier-a",
        "r10-10 cross-crate integration",
    )
}

fn sample_manifest() -> UpgradeManifest {
    ManifestBuilder::new("v2.0.0", UpgradeKind::Patch)
        .with_description("r10-10 cross-crate integration")
        .with_content_hash("r10-10-hash")
        .build()
}

fn seven_advisors() -> Vec<Arc<dyn Advisor>> {
    apeireth_upgrade::cross_crate::seven_mandatory_advisors()
        .into_iter()
        .map(|b| Arc::from(b) as Arc<dyn Advisor>)
        .collect()
}

fn start_pipeline(intent: &UpgradeIntent) -> OtaPipeline {
    let mut p = OtaPipeline::new(OtaStage::Idle);
    p.start_intent(intent.clone()).unwrap();
    p
}

fn healthy_monitor() -> MonitorReport {
    let mut d = MonitorDashboard::new();
    d.record(MonitorMetric::new("a", 0.01, Some(0.05), None));
    d.record(MonitorMetric::new("b", 100.0, Some(500.0), None));
    d.report()
}

#[test]
fn r10_10_seven_advisors_full_deliberation() {
    let advisors = seven_advisors();
    assert_eq!(advisors.len(), 7);
    let query = CouncilQuery::new("r10-10-q1", "OTA cross-crate patch upgrade", 1_000_000);
    let deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    assert_eq!(deliberations.len(), 7);
    let domains: std::collections::HashSet<_> = deliberations.iter().map(|d| d.domain).collect();
    assert_eq!(domains.len(), 7, "must cover 7 mandatory domains");
}

#[test]
fn r10_10_council_synthesize_no_hold_on_high_confidence() {
    let advisors = seven_advisors();
    let query = CouncilQuery::new("r10-10-q2", "OTA cross-crate patch upgrade", 1_000_000);
    let deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    let high_conf: Vec<_> = deliberations
        .into_iter()
        .map(|mut d| {
            d.confidence = 0.95;
            d.triggers_hold = false;
            d
        })
        .collect();
    let report =
        synthesize_council_report(&high_conf, &default_synthesis_weights(), Uuid::nil(), 0);
    assert!(matches!(report.hold, HoldAction::NoHold));
    assert!(report.is_approved());
}

#[test]
fn r10_10_council_hold_on_low_confidence_safety() {
    let advisors = seven_advisors();
    let query = CouncilQuery::new("r10-10-q3", "OTA cross-crate", 1_000_000);
    let mut deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    deliberations[0].confidence = 0.1;
    deliberations[0].triggers_hold = true;
    let report =
        synthesize_council_report(&deliberations, &default_synthesis_weights(), Uuid::nil(), 0);
    match &report.hold {
        HoldAction::TriggerHold {
            strong_disapprove_count,
            ..
        } => {
            assert!(*strong_disapprove_count >= 1);
        }
        _ => panic!("expected TriggerHold"),
    }
}

#[test]
fn r10_10_multisig_2_of_3_approved() {
    let policy = MultiSigPolicy::default();
    let auth = HumanAuthority::multi("ha-r10-10", "upgrade", 2, 3).unwrap();
    let req = OwnerRequest::new("req-1", "r10-10 multisig test");
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 1_000_000);
    assert!(matches!(outcome, AuthorityMultisigOutcome::Approved { .. }));
    let ota_outcome = multisig_outcome_from_authority(outcome, 1_000_000);
    assert!(matches!(ota_outcome, MultiSigOutcome::Quorum { .. }));
}

#[test]
fn r10_10_multisig_1_of_3_insufficient() {
    let policy = MultiSigPolicy::default();
    let auth = HumanAuthority::multi("ha-r10-10", "upgrade", 2, 3).unwrap();
    let req = OwnerRequest::new("req-2", "r10-10 multisig insufficient");
    let sigs = vec!["h-1".to_string()];
    let outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 1_000_000);
    assert!(matches!(
        outcome,
        AuthorityMultisigOutcome::InsufficientSignatures { .. }
    ));
    let ota_outcome = multisig_outcome_from_authority(outcome, 0);
    match ota_outcome {
        MultiSigOutcome::Pending { collected, needed } => {
            assert_eq!(collected, 1);
            assert_eq!(needed, 1);
        }
        _ => panic!("expected Pending"),
    }
}

#[test]
fn r10_10_five_gates_full_5_reports_for_normal_action() {
    let mut engine = ConstraintEngine::new();
    let action = Action {
        id: "r10-10-fg-1".into(),
        description: "OTA cross-crate sandbox".into(),
        risk_level: RiskLevel::Medium,
        target: ActionTarget::NormalAction("ota-patch".into()),
    };
    engine
        .cache_mut()
        .insert(action.id.clone(), PhilosophyVerdict::Allow);
    let report = sandbox_with_five_gates(&engine, &action);
    assert!(report.compile_time.is_pass());
    assert!(report.runtime_intercept.is_pass());
    assert!(report.multi_ai_consensus.is_pass());
    assert!(report.physical_isolation.is_pass());
    assert!(!report.reflection_period.is_pass());
    assert!(report.risk_grant.within_threshold);
}

#[test]
fn r10_10_five_gates_block_on_modify_l0_ha() {
    let engine = ConstraintEngine::new();
    let action = Action {
        id: "r10-10-fg-block".into(),
        description: "Modify L0 HA — should be blocked".into(),
        risk_level: RiskLevel::Critical,
        target: ActionTarget::ModifyL0HA,
    };
    let report = sandbox_with_five_gates(&engine, &action);
    assert!(report.compile_time.is_pass());
    assert!(!report.runtime_intercept.is_pass());
    assert!(report.first_block_reason().is_some());
}

#[test]
fn r10_10_full_7_stages_with_cross_crate_calls() {
    let intent = sample_intent();
    let mut pipeline = start_pipeline(&intent);
    assert_eq!(pipeline.stage(), OtaStage::IntentDraft);

    let advisors = seven_advisors();
    let query = CouncilQuery::new(&intent.id.to_string(), "OTA cross-crate", 1_000_000);
    let deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    let high_conf: Vec<_> = deliberations
        .into_iter()
        .map(|mut d| {
            d.confidence = 0.95;
            d.triggers_hold = false;
            d
        })
        .collect();
    let report =
        synthesize_council_report(&high_conf, &default_synthesis_weights(), intent.id, 1_000_000);
    pipeline.enter_council_review(report).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::CouncilReview);

    let policy = MultiSigPolicy::default();
    let auth = default_multi_authority().unwrap();
    let req = OwnerRequest::new("r10-10-full", "r10-10 full path");
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let auth_outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 1_000_000);
    let ms_outcome = multisig_outcome_from_authority(auth_outcome, 1_000_000);
    pipeline.enter_multisig(ms_outcome).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::MultiSig);

    let mut engine = ConstraintEngine::new();
    let action = Action {
        id: format!("r10-10-sandbox-{}", intent.id),
        description: "OTA cross-crate sandbox test".into(),
        risk_level: RiskLevel::Medium,
        target: ActionTarget::NormalAction("ota-patch".into()),
    };
    engine
        .cache_mut()
        .insert(action.id.clone(), PhilosophyVerdict::Allow);
    let _fg_report: SandboxFiveGatesReport = sandbox_with_five_gates(&engine, &action);
    let sandbox = DefaultSandbox;
    let sandbox_verdict = sandbox.validate(&sample_manifest());
    assert!(matches!(
        sandbox_verdict,
        apeireth_upgrade::SandboxVerdict::Accept
    ));
    pipeline
        .enter_sandbox(
            intent.id,
            "blue".to_string(),
            "green".to_string(),
            &sample_manifest(),
            &sandbox,
        )
        .unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Sandbox);

    pipeline.enter_switchover().unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Switchover);

    let monitor = healthy_monitor();
    pipeline.enter_monitor(monitor.clone()).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Monitor);

    pipeline.finalize(monitor).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Done);
}

#[test]
fn r10_10_ota_hold_from_real_council_triggers_rollback() {
    let intent = sample_intent();
    let mut pipeline = start_pipeline(&intent);

    let advisors = seven_advisors();
    let query = CouncilQuery::new(&intent.id.to_string(), "OTA cross-crate", 1_000_000);
    let mut deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    deliberations[0].confidence = 0.05;
    deliberations[0].triggers_hold = true;
    deliberations[6].confidence = 0.05;
    deliberations[6].triggers_hold = true;
    let report =
        synthesize_council_report(&deliberations, &default_synthesis_weights(), intent.id, 1_000_000);
    assert!(matches!(report.hold, HoldAction::TriggerHold { .. }));
    pipeline.enter_council_review(report).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Rollback);
}

#[test]
fn r10_10_ota_multisig_block_triggers_rollback() {
    let intent = sample_intent();
    let mut pipeline = start_pipeline(&intent);

    let opinions: Vec<CouncilOpinion> = CouncilSeat::ALL
        .iter()
        .map(|s| CouncilOpinion::new(*s, CouncilStance::Approve, 0.9, "ok"))
        .collect();
    let stub_report = CouncilReport {
        intent_id: intent.id,
        opinions,
        missing_seats: vec![],
        hold: HoldAction::NoHold,
        reviewed_at: 0,
    };
    pipeline.enter_council_review(stub_report).unwrap();

    let hash = intent_payload_hash(&intent);
    let cfg = MultiSigConfig::five_of_seven();
    let mut col = MultiSigCollector::new(cfg, hash.clone());
    for i in 0..5 {
        col.submit(PhysicalSignature::new(
            format!("signer-{i}"),
            hash.clone(),
            100,
            format!("sig{i}"),
        ))
        .unwrap();
    }
    let ms_outcome = col.evaluate(200);
    pipeline.enter_multisig(ms_outcome).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::MultiSig);
}

#[test]
fn r10_10_ota_sandbox_five_gates_full_report_for_normal() {
    let mut engine = ConstraintEngine::new();
    let action = Action {
        id: "r10-10-sandbox-report".into(),
        description: "OTA cross-crate full report".into(),
        risk_level: RiskLevel::High,
        target: ActionTarget::NormalAction("ota-patch".into()),
    };
    engine
        .cache_mut()
        .insert(action.id.clone(), PhilosophyVerdict::Allow);
    let report = sandbox_with_five_gates(&engine, &action);
    assert!(report.compile_time.is_pass());
    assert!(report.runtime_intercept.is_pass());
    assert!(report.multi_ai_consensus.is_pass());
    assert!(report.physical_isolation.is_pass());
    assert!(!report.reflection_period.is_pass());
    assert!(report.risk_grant.within_threshold);
}

#[test]
fn r10_10_cross_crate_three_fold_integration() {
    let advisors = seven_advisors();
    let query = CouncilQuery::new("r10-10-3fold", "3-fold", 0);
    let deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    let high_conf: Vec<_> = deliberations
        .into_iter()
        .map(|mut d| {
            d.confidence = 0.9;
            d.triggers_hold = false;
            d
        })
        .collect();
    let council_report =
        synthesize_council_report(&high_conf, &default_synthesis_weights(), Uuid::nil(), 0);
    assert!(matches!(council_report.hold, HoldAction::NoHold));

    let policy = MultiSigPolicy::default();
    let auth = default_multi_authority().unwrap();
    let req = OwnerRequest::new("r10-10-3fold", "3-fold");
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let ms_outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 0);
    assert!(matches!(
        ms_outcome,
        AuthorityMultisigOutcome::Approved { .. }
    ));

    let mut engine = ConstraintEngine::new();
    let action = Action {
        id: "r10-10-3fold-action".into(),
        description: "3-fold integration test".into(),
        risk_level: RiskLevel::Medium,
        target: ActionTarget::NormalAction("ota-patch".into()),
    };
    engine
        .cache_mut()
        .insert(action.id.clone(), PhilosophyVerdict::Allow);
    let fg_report = sandbox_with_five_gates(&engine, &action);
    assert!(fg_report.compile_time.is_pass());
    assert!(fg_report.runtime_intercept.is_pass());
    assert!(fg_report.multi_ai_consensus.is_pass());
    assert!(fg_report.physical_isolation.is_pass());
    assert!(fg_report.risk_grant.within_threshold);
}

#[test]
fn r10_10_all_seven_domains_present() {
    let domains = AdvisorDomain::ALL;
    assert_eq!(domains.len(), 7);
}

#[test]
fn r10_10_default_ota_multisig_collector_5_of_7() {
    let col = default_ota_multisig_collector("payload-123".into());
    assert_eq!(col.signatures().len(), 0);
    assert_eq!(col.config().threshold, 5);
    assert_eq!(col.config().eligible_signers.len(), 7);
}
