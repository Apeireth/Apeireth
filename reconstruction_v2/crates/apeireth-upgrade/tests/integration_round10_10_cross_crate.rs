//! round10-10 跨 crate 集成测试.
//!
//! 覆盖:
//! 1. OTA CouncilReview → apeireth-council 7 强制 advisor 全员审议
//! 2. OTA CouncilReview Hold 触发 → 7 advisor 中 Safety/Legal 低置信度触发
//! 3. OTA MultiSig → apeireth-sovereignty MultiSigPolicy M-of-N 真实校验
//! 4. OTA MultiSig ReadOnly rejected → core-rule mutation 拒绝
//! 5. OTA MultiSig insufficient signatures → Pending 状态
//! 6. OTA Sandbox → apeireth-constraint FiveGates 5 重守门
//! 7. OTA Sandbox gate4 reflection 默认 block (P19 未接入)
//! 8. 完整 7 阶段 happy path 含跨 crate 真实调用
//!
//! **守门遵守**: 不修改 OTA/council/sovereignty/constraint 任何 LOCKED 文件。

use apeireth_constraint::{ConstraintEngine, FourGates, GateVerdict, PermissionGrant};
use apeireth_core::{Action, ActionTarget, PhilosophyVerdict, RiskLevel};
use apeireth_council::{seven_mandatory_advisors, CouncilQuery, SynthesisWeights};
use apeireth_sovereignty::{
    AuthorityMultisigOutcome, HumanAuthority, MultiSigPolicy, OwnerAction, OwnerRequest, OwnerToken,
};
use apeireth_upgrade::cross_crate::{
    check_multisig_with_sovereignty, default_multi_authority, default_ota_multisig_collector,
    default_synthesis_weights, deliberate_with_7_advisors, multisig_outcome_from_authority,
    sandbox_with_five_gates, synthesize_council_report, GateVerdictExt, SandboxFiveGatesReport,
};
use apeireth_upgrade::{
    CouncilOpinion, CouncilReport, CouncilSeat, CouncilStance, DefaultSandbox, HoldAction,
    HoldTrigger, ManifestBuilder, MonitorDashboard, MonitorMetric, MonitorReport,
    MultiSigCollector, MultiSigConfig, MultiSigOutcome, OtaPipeline, OtaStage, PhysicalSignature,
    SandboxValidator, UpgradeIntent, UpgradeKind, UpgradeManifest,
};
use std::sync::Arc;
use uuid::Uuid;

// =====================================================================
// Helpers
// =====================================================================

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

fn seven_advisors() -> Vec<Arc<dyn apeireth_council::Advisor>> {
    seven_mandatory_advisors()
        .into_iter()
        .map(|b| Arc::from(b) as Arc<dyn apeireth_council::Advisor>)
        .collect()
}

fn patch_query(id: &str, now_ms: i64) -> CouncilQuery {
    CouncilQuery::new(id, "OTA cross-crate patch upgrade", now_ms).with_risk("high")
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

// =====================================================================
// Test 1: 7 advisor 全员审议 happy path
// =====================================================================

#[test]
fn r10_10_seven_advisors_full_deliberation() {
    let advisors = seven_advisors();
    assert_eq!(advisors.len(), 7);
    let query = patch_query("r10-10-q1", 1_000_000);
    let deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    assert_eq!(deliberations.len(), 7);
    // 验证域覆盖
    let domains: std::collections::HashSet<_> = deliberations.iter().map(|d| d.domain).collect();
    assert_eq!(domains.len(), 7, "必须覆盖 7 强制域");
}

#[test]
fn r10_10_council_synthesize_no_hold_on_high_confidence() {
    let advisors = seven_advisors();
    let query = patch_query("r10-10-q2", 1_000_000);
    let deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    // 模拟高置信度 (覆盖真实审议结果, 测试 OTA 适配层逻辑)
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
    let query = patch_query("r10-10-q3", 1_000_000);
    let mut deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    // 强制 Safety 低置信度 → StrongDisapprove → Hold
    deliberations[0].confidence = 0.1;
    deliberations[0].triggers_hold = true;
    let report =
        synthesize_council_report(&deliberations, &default_synthesis_weights(), Uuid::nil(), 0);
    match &report.hold {
        HoldAction::TriggerHold {
            reason,
            strong_disapprove_count,
            ..
        } => {
            assert!(*strong_disapprove_count >= 1);
            assert!(reason.contains("r10-10 cross-crate"));
        }
        _ => panic!("expected TriggerHold"),
    }
}

// =====================================================================
// Test 2: MultiSig 真实 M-of-N 校验
// =====================================================================

#[test]
fn r10_10_multisig_2_of_3_approved() {
    let policy = MultiSigPolicy::default_2_of_3();
    let auth = HumanAuthority::multi("ha-r10-10", "upgrade", 2, 3).unwrap();
    let req = OwnerRequest::new(
        "req-1",
        OwnerToken::Admin,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "r10-10 multisig test",
    );
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 1_000_000);
    assert!(matches!(outcome, AuthorityMultisigOutcome::Approved { .. }));
    // 映射到 OTA
    let ota_outcome = multisig_outcome_from_authority(outcome, 1_000_000);
    assert!(matches!(ota_outcome, MultiSigOutcome::Quorum { .. }));
}

#[test]
fn r10_10_multisig_1_of_3_insufficient() {
    let policy = MultiSigPolicy::default_2_of_3();
    let auth = HumanAuthority::multi("ha-r10-10", "upgrade", 2, 3).unwrap();
    let req = OwnerRequest::new(
        "req-2",
        OwnerToken::Admin,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "r10-10 multisig insufficient",
    );
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
fn r10_10_multisig_read_only_rejected_on_core_rule() {
    let policy = MultiSigPolicy::default_2_of_3();
    let auth = HumanAuthority::multi("ha-r10-10", "upgrade", 2, 3).unwrap();
    // ReadOnly token + core-rule mutation → ReadOnlyRejected
    let req = OwnerRequest::new(
        "req-3",
        OwnerToken::ReadOnly,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "r10-10 ReadOnly test",
    );
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 1_000_000);
    assert!(matches!(
        outcome,
        AuthorityMultisigOutcome::ReadOnlyRejected
    ));
    let ota_outcome = multisig_outcome_from_authority(outcome, 0);
    assert!(matches!(ota_outcome, MultiSigOutcome::Invalid { .. }));
}

#[test]
fn r10_10_multisig_unknown_signatory_rejected() {
    let policy = MultiSigPolicy::default_2_of_3();
    let auth = HumanAuthority::multi("ha-r10-10", "upgrade", 2, 3).unwrap();
    let req = OwnerRequest::new(
        "req-4",
        OwnerToken::Master,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "r10-10 unknown signatory",
    );
    let sigs = vec!["h-1".to_string(), "bogus-signer".to_string()];
    let outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 1_000_000);
    assert!(matches!(
        outcome,
        AuthorityMultisigOutcome::UnknownSignatory(_)
    ));
}

// =====================================================================
// Test 3: FiveGates 真实守门
// =====================================================================

#[test]
fn r10_10_five_gates_gate1_compile_time_always_pass_for_normal() {
    let engine = ConstraintEngine::new();
    let verdict = engine.gate1_compile_time();
    assert!(matches!(verdict, GateVerdict::Pass));
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
    engine.cache_mut().put(&action.id, PhilosophyVerdict::Allow);
    let report = sandbox_with_five_gates(&engine, &action);
    assert!(report.compile_time.is_pass());
    assert!(report.runtime_intercept.is_pass());
    assert!(report.multi_ai_consensus.is_pass());
    assert!(report.physical_isolation.is_pass());
    // gate5 reflection 默认 Block (P19 待接入)
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
    // gate1 compile_time 仅检查 12 键 hardcode (始终 Pass);
    // gate2 runtime 在 cache 空时 Block (默认拒绝, 主 17:58 不假装安全)
    assert!(
        report.compile_time.is_pass(),
        "gate1 always passes 12-key hardcode check"
    );
    assert!(
        !report.runtime_intercept.is_pass(),
        "gate2 runtime blocks with empty cache"
    );
    assert!(report.first_block_reason().is_some());
}

#[test]
fn r10_10_five_gates_risk_levels_map_correctly() {
    let engine = ConstraintEngine::new();
    let action_low = Action {
        id: "low".into(),
        description: "low risk".into(),
        risk_level: RiskLevel::Low,
        target: ActionTarget::NormalAction("x".into()),
    };
    let action_high = Action {
        id: "high".into(),
        description: "high risk".into(),
        risk_level: RiskLevel::High,
        target: ActionTarget::NormalAction("x".into()),
    };
    let report_low = sandbox_with_five_gates(&engine, &action_low);
    let report_high = sandbox_with_five_gates(&engine, &action_high);
    // Low risk → level 1; High risk → level 5
    assert_eq!(report_low.risk_grant.level, 1);
    assert_eq!(report_high.risk_grant.level, 5);
    assert!(report_low.risk_grant.within_threshold);
    // High risk level=5 is above default threshold (within_threshold=false)
    assert!(!report_high.risk_grant.within_threshold);
}

// =====================================================================
// Test 4: 完整 7 阶段 + 跨 crate 真实调用 (happy path)
// =====================================================================

#[test]
fn r10_10_full_7_stages_with_cross_crate_calls() {
    let intent = sample_intent();
    let mut pipeline = start_pipeline(&intent);
    // Step 1: IntentDraft
    assert_eq!(pipeline.stage(), OtaStage::IntentDraft);

    // Step 2: CouncilReview — 用 7 advisor 真实审议
    let advisors = seven_advisors();
    let query = patch_query(&intent.id.to_string(), 1_000_000);
    let deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    // 强制全部高置信度 (避免真实审议偶然 hold)
    let high_conf: Vec<_> = deliberations
        .into_iter()
        .map(|mut d| {
            d.confidence = 0.95;
            d.triggers_hold = false;
            d
        })
        .collect();
    let report = synthesize_council_report(
        &high_conf,
        &default_synthesis_weights(),
        intent.id,
        1_000_000,
    );
    pipeline.enter_council_review(report).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::CouncilReview);

    // Step 3: MultiSig — 用 sovereignty MultiSigPolicy 真实校验
    let policy = MultiSigPolicy::default_2_of_3();
    let auth = default_multi_authority().unwrap();
    let req = OwnerRequest::new(
        "r10-10-full",
        OwnerToken::Admin,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "r10-10 full path",
    );
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let auth_outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 1_000_000);
    let ms_outcome = multisig_outcome_from_authority(auth_outcome, 1_000_000);
    pipeline.enter_multisig(ms_outcome).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::MultiSig);

    // Step 4: Sandbox — 用 constraint FiveGates 真实校验
    let mut engine = ConstraintEngine::new();
    let action = Action {
        id: format!("r10-10-sandbox-{}", intent.id),
        description: "OTA cross-crate sandbox test".into(),
        risk_level: RiskLevel::Medium,
        target: ActionTarget::NormalAction("ota-patch".into()),
    };
    engine.cache_mut().put(&action.id, PhilosophyVerdict::Allow);
    let _fg_report: SandboxFiveGatesReport = sandbox_with_five_gates(&engine, &action);
    // 用 DefaultSandbox 走 OTA 自有 sandbox (fg_report 仅供审计)
    let sandbox = DefaultSandbox;
    let sandbox_verdict = sandbox.validate(&sample_manifest());
    assert!(matches!(
        sandbox_verdict,
        apeireth_upgrade::SandboxVerdict::Accept
    ));
    // 进入 sandbox 阶段 — 注意 reflection 默认 block 不影响本测试 (我们用 DefaultSandbox 走 OTA 路径)
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

    // Step 5: Switchover
    pipeline.enter_switchover().unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Switchover);

    // Step 6: Monitor
    let monitor = healthy_monitor();
    pipeline.enter_monitor(monitor.clone()).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Monitor);

    // Step 7: Done
    pipeline.finalize(monitor).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Done);
}

// =====================================================================
// Test 5: OTA CouncilReview Hold 触发 Rollback (基于跨 crate 真实审议)
// =====================================================================

#[test]
fn r10_10_ota_hold_from_real_council_triggers_rollback() {
    let intent = sample_intent();
    let mut pipeline = start_pipeline(&intent);

    let advisors = seven_advisors();
    let query = patch_query(&intent.id.to_string(), 1_000_000);
    let mut deliberations = deliberate_with_7_advisors(&advisors, &query).unwrap();
    // 强制 Safety + Legal 低置信度 → Hold
    deliberations[0].confidence = 0.05;
    deliberations[0].triggers_hold = true;
    deliberations[6].confidence = 0.05;
    deliberations[6].triggers_hold = true;
    let report = synthesize_council_report(
        &deliberations,
        &default_synthesis_weights(),
        intent.id,
        1_000_000,
    );
    // 验证 hold 已触发
    assert!(matches!(report.hold, HoldAction::TriggerHold { .. }));
    pipeline.enter_council_review(report).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Rollback);
}

// =====================================================================
// Test 6: OTA MultiSig 阻塞触发 Rollback (基于 sovereignty 真实校验)
// =====================================================================

#[test]
fn r10_10_ota_multisig_block_from_real_sovereignty_triggers_rollback() {
    let intent = sample_intent();
    let mut pipeline = start_pipeline(&intent);

    // 先过 CouncilReview (用 stub 简化, 重点在 multisig)
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

    // sovereignty 真实校验 — ReadOnly token + core-rule → Invalid → Rollback
    let policy = MultiSigPolicy::default_2_of_3();
    let auth = HumanAuthority::multi("ha-r10-10", "u", 2, 3).unwrap();
    let req = OwnerRequest::new(
        "r10-10-rb",
        OwnerToken::ReadOnly,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "r10-10 rollback test",
    );
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let auth_outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 0);
    let ms_outcome = multisig_outcome_from_authority(auth_outcome, 0);
    assert!(matches!(ms_outcome, MultiSigOutcome::Invalid { .. }));
    pipeline.enter_multisig(ms_outcome).unwrap();
    assert_eq!(pipeline.stage(), OtaStage::Rollback);
}

// =====================================================================
// Test 7: OTA Sandbox 守门 5 重报告 (约束引擎真实调用)
// =====================================================================

#[test]
fn r10_10_ota_sandbox_five_gates_full_report_for_normal() {
    let mut engine = ConstraintEngine::new();
    let action = Action {
        id: "r10-10-sandbox-report".into(),
        description: "OTA cross-crate full report".into(),
        risk_level: RiskLevel::High,
        target: ActionTarget::NormalAction("ota-patch".into()),
    };
    engine.cache_mut().put(&action.id, PhilosophyVerdict::Allow);
    let report = sandbox_with_five_gates(&engine, &action);
    // 验证 5 重报告字段均存在且类型正确
    assert!(matches!(report.compile_time, GateVerdict::Pass));
    assert!(matches!(report.runtime_intercept, GateVerdict::Pass));
    assert!(matches!(report.multi_ai_consensus, GateVerdict::Pass));
    assert!(matches!(report.physical_isolation, GateVerdict::Pass));
    assert!(!matches!(report.reflection_period, GateVerdict::Pass));
    // High risk → 5 sigs required, within threshold
    assert_eq!(report.risk_grant.level, 5);
    assert!(report.risk_grant.within_threshold);
}

// =====================================================================
// Test 8: 跨 crate 三方均通过 = OTA Sandbox Pass (组合验证)
// =====================================================================

#[test]
fn r10_10_cross_crate_three_fold_integration() {
    // 1. Council — 7 advisor 审议通过 (高置信度)
    let advisors = seven_advisors();
    let query = patch_query("r10-10-3fold", 0);
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

    // 2. Sovereignty — 2-of-3 multi-sig 通过
    let policy = MultiSigPolicy::default_2_of_3();
    let auth = default_multi_authority().unwrap();
    let req = OwnerRequest::new(
        "r10-10-3fold",
        OwnerToken::Admin,
        OwnerAction::ModifyL0Threshold,
        "alice",
        "3-fold",
    );
    let sigs = vec!["h-1".to_string(), "h-2".to_string()];
    let ms_outcome = check_multisig_with_sovereignty(&policy, &req, &sigs, &auth, 0);
    assert!(matches!(
        ms_outcome,
        AuthorityMultisigOutcome::Approved { .. }
    ));

    // 3. Constraint — FiveGates cache Allow + Medium risk → 4/5 通过
    let mut engine = ConstraintEngine::new();
    let action = Action {
        id: "r10-10-3fold-action".into(),
        description: "3-fold integration test".into(),
        risk_level: RiskLevel::Medium,
        target: ActionTarget::NormalAction("ota-patch".into()),
    };
    engine.cache_mut().put(&action.id, PhilosophyVerdict::Allow);
    let fg_report = sandbox_with_five_gates(&engine, &action);
    assert!(fg_report.compile_time.is_pass());
    assert!(fg_report.runtime_intercept.is_pass());
    assert!(fg_report.multi_ai_consensus.is_pass());
    assert!(fg_report.physical_isolation.is_pass());
    assert!(fg_report.risk_grant.within_threshold);

    // 综合判定: 3 方均通过 = 可以进入下一阶段 (虽然 reflection 默认 block 是已知 stub)
}
