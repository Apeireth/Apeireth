//! round8-06 integration tests — HA 部署模式自适应端到端
//!
//! 覆盖 3 模式 (Single / Multi / Dynamic) 在真实部署上下文中的差异化行为:
//! - Single: 1-of-1 + Windows Hello + low/medium 风险
//! - Multi: M-of-N 严格阈值
//! - Dynamic: 上下文自适应阈值 (E 层 +20, 紧急 -20, 反思 +30)
//!
//! **守 7 项不修改承诺**: 不修改 ha.rs / sovereign.rs 已实装类型。

use apeireth_sovereignty::{
    AuthorityMode, DeploymentContext, DeploymentMode, DeploymentOutcome, HAAuthentication,
    HADeploymentEnforcer, HumanApproval, HumanAuthority, MultiSigPolicy, Signatory,
};

const NOW: i64 = 1_700_000_000_000;

#[test]
fn integration_single_mode_full_pipeline_low_risk() {
    // 真实场景: 单人部署 + Windows Hello + low risk
    let mut ha = HumanAuthority::single("h-1", "Alice");
    ha.record_approval(HumanApproval::new("ap-1", "h-1", "Alice", 1000, "test"));
    let bio = apeireth_sovereignty::MockBiometric::new();
    let enforcer = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
    let outcome = enforcer.enforce(&["h-1".into()], "low", NOW);
    assert!(outcome.is_approved());
    if let DeploymentOutcome::ApprovedSingle {
        signature_id,
        confidence,
        ..
    } = outcome
    {
        assert_eq!(signature_id, "h-1");
        assert!(confidence >= 0.0 && confidence <= 1.0);
    } else {
        panic!("应为 ApprovedSingle");
    }
}

#[test]
fn integration_single_mode_rejects_high_risk() {
    // 真实场景: 单人部署试图处理 high risk 任务 → 必须升级到 Multi 模式
    let mut ha = HumanAuthority::single("h-1", "Alice");
    ha.record_approval(HumanApproval::new("ap-1", "h-1", "Alice", 1000, "test"));
    let bio = apeireth_sovereignty::MockBiometric::new();
    let enforcer = HADeploymentEnforcer::single(&ha, &bio, DeploymentContext::NormalLayer);
    let outcome = enforcer.enforce(&["h-1".into()], "high", NOW);
    assert!(outcome.is_rejected());
    if let DeploymentOutcome::RejectedSingleHighRisk { risk, max_allowed } = outcome {
        assert_eq!(risk, "high");
        assert_eq!(max_allowed, "medium");
    }
}

#[test]
fn integration_multi_mode_2_of_3_approved() {
    // 真实场景: 多人部署 + 2-of-3 阈值 + E 层上下文
    let mut ha = HumanAuthority::multi("ha-1", "team", 2, 3).unwrap();
    ha.record_approval(HumanApproval::new("ap-1", "h-1", "Alice", 1000, "x"));
    ha.record_approval(HumanApproval::new("ap-2", "h-2", "Bob", 1000, "x"));
    let policy = MultiSigPolicy::default_2_of_3();
    let enforcer = HADeploymentEnforcer::multi(&ha, &policy, DeploymentContext::ExistenceLayer);
    let outcome = enforcer.enforce(&["h-1".into(), "h-2".into()], "critical", NOW);
    assert!(outcome.is_approved());
    if let DeploymentOutcome::ApprovedMulti {
        valid_signatures,
        required,
        effective_threshold,
        ..
    } = outcome
    {
        assert_eq!(valid_signatures, 2);
        assert_eq!(required, 2);
        assert_eq!(effective_threshold, 66);
    }
}

#[test]
fn integration_dynamic_mode_e_layer_raises_threshold() {
    // 真实场景: E 层请求 + Dynamic 模式 → 阈值从 50% 提到 70%
    let mut ha = HumanAuthority::dynamic("d-1", "E-layer-ctx", 3, 50, 5);
    for i in 0..5 {
        ha.record_approval(HumanApproval::new(
            format!("ap-{i}"),
            format!("h-{i}"),
            format!("S{i}"),
            1000,
            "x",
        ));
    }
    let enforcer = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::ExistenceLayer);
    let outcome = enforcer.enforce(&["h-0".into(), "h-1".into(), "h-2".into()], "nuclear", NOW);
    assert!(outcome.is_approved());
    if let DeploymentOutcome::ApprovedDynamic {
        base_threshold,
        adjusted_threshold,
        ..
    } = outcome
    {
        assert_eq!(base_threshold, 50);
        assert_eq!(adjusted_threshold, 70, "E 层 +20% 应为 70");
    }
}

#[test]
fn integration_dynamic_mode_emergency_lowers_threshold() {
    // 真实场景: 紧急层请求 → 阈值从 50% 降到 30%
    let mut ha = HumanAuthority::dynamic("d-1", "emergency-ctx", 3, 50, 5);
    for i in 0..5 {
        ha.record_approval(HumanApproval::new(
            format!("ap-{i}"),
            format!("h-{i}"),
            format!("S{i}"),
            1000,
            "x",
        ));
    }
    let enforcer = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::EmergencyLayer);
    let outcome = enforcer.enforce(&[], "low", NOW);
    assert!(outcome.is_approved());
    if let DeploymentOutcome::ApprovedDynamic {
        adjusted_threshold, ..
    } = outcome
    {
        assert_eq!(adjusted_threshold, 30);
    }
}

#[test]
fn integration_dynamic_mode_reflection_layer_plus_30() {
    // 真实场景: 反思层请求 → 阈值从 50% 提到 80%
    let mut ha = HumanAuthority::dynamic("d-1", "reflection-ctx", 3, 50, 5);
    for i in 0..5 {
        ha.record_approval(HumanApproval::new(
            format!("ap-{i}"),
            format!("h-{i}"),
            format!("S{i}"),
            1000,
            "x",
        ));
    }
    let enforcer = HADeploymentEnforcer::dynamic(&ha, DeploymentContext::ReflectionLayer);
    let outcome = enforcer.enforce(&[], "low", NOW);
    assert!(outcome.is_approved());
    if let DeploymentOutcome::ApprovedDynamic {
        adjusted_threshold,
        context,
        ..
    } = outcome
    {
        assert_eq!(adjusted_threshold, 80);
        assert!(context.requires_reflection());
    }
}

#[test]
fn integration_deployment_mode_select_for_context() {
    // 真实场景: 根据上下文选择模式
    assert_eq!(
        DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 0),
        DeploymentMode::Dynamic
    );
    assert_eq!(
        DeploymentMode::select_for_context(DeploymentContext::NormalLayer, 1),
        DeploymentMode::Single
    );
    assert_eq!(
        DeploymentMode::select_for_context(DeploymentContext::ExistenceLayer, 5),
        DeploymentMode::Multi
    );
    assert_eq!(
        DeploymentMode::select_for_context(DeploymentContext::ReflectionLayer, 3),
        DeploymentMode::Multi
    );
}

#[test]
fn integration_reflection_tracker_blocks_within_window() {
    // 真实场景: 反思期内所有审批被拒绝
    let tracker = apeireth_sovereignty::DeploymentReflectionTracker::new(NOW, 7 * 86_400_000);
    assert!(tracker.is_in_reflection(NOW + 86_400_000));
    assert!(!tracker.is_in_reflection(NOW + 8 * 86_400_000));
    let remaining = tracker.remaining_ms(NOW);
    assert_eq!(remaining, 7 * 86_400_000);
}
