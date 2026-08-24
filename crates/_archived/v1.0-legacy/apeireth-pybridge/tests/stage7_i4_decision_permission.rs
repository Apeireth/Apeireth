//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I4 D4+G2 决策+权限集成 集成测试
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **维度**: I4 D4+G2 决策+权限 跨 stage 集成
//! **目标**: 验证 D4 决策自循环跟 G2 权限治理 (1:1 跟 B4 6 重 v7 严守) 跨 stage 集成

use apeireth_pybridge::{
    stage7_i4_healthy, stage7_i4_summary, stage7_i4_to_d4_consistency, stage7_i4_to_g2_consistency,
    DecisionPermissionAuditEvent, DecisionPermissionBinding, DecisionPermissionCoordinator,
    DecisionPermissionMatrix, DecisionPermissionReport, DecisionPolicy, PermissionContext,
    PermissionLayer, STAGE7_I4_BINDING_COUNT, STAGE7_I4_DIMENSION_COUNT, STAGE7_I4_LAYER_COUNT,
    STAGE7_I4_POLICY_COUNT, STAGE7_I4_VERSION,
};

// 1. I4 编译期常数
#[test]
fn i4_01_version_constants() {
    assert_eq!(STAGE7_I4_VERSION, "0.1.0-R129-Stage7-I4");
    assert_eq!(STAGE7_I4_DIMENSION_COUNT, 2);
    assert_eq!(STAGE7_I4_POLICY_COUNT, 5);
    assert_eq!(STAGE7_I4_LAYER_COUNT, 6);
    assert_eq!(STAGE7_I4_BINDING_COUNT, 30);
}

// 2. I4 默认矩阵 5 × 6 = 30
#[test]
fn i4_02_matrix_size() {
    let m = DecisionPermissionMatrix::default_matrix();
    assert_eq!(m.len(), 30);
}

// 3. I4 5 policies 都有 binding
#[test]
fn i4_03_all_5_policies() {
    let m = DecisionPermissionMatrix::default_matrix();
    let policies = [
        DecisionPolicy::Conservative,
        DecisionPolicy::Cautious,
        DecisionPolicy::Balanced,
        DecisionPolicy::Progressive,
        DecisionPolicy::Aggressive,
    ];
    for p in &policies {
        assert!(
            m.get(*p, PermissionLayer::L1TypeCheck).is_some(),
            "policy {p:?} 缺"
        );
    }
}

// 4. I4 6 layers 都有 binding (1:1 跟 B4 6 重 v7 严守)
#[test]
fn i4_04_all_6_layers() {
    let m = DecisionPermissionMatrix::default_matrix();
    let layers = [
        PermissionLayer::L1TypeCheck,
        PermissionLayer::L2ScopeCheck,
        PermissionLayer::L3RateCheck,
        PermissionLayer::L4GuardCheck,
        PermissionLayer::L5AuditCheck,
        PermissionLayer::L6ProvenanceCheck,
    ];
    for l in &layers {
        assert!(
            m.get(DecisionPolicy::Balanced, *l).is_some(),
            "layer {l:?} 缺"
        );
    }
}

// 5. I4 Conservative → deny
#[test]
fn i4_05_conservative_deny() {
    let m = DecisionPermissionMatrix::default_matrix();
    for l in &[
        PermissionLayer::L1TypeCheck,
        PermissionLayer::L2ScopeCheck,
        PermissionLayer::L3RateCheck,
        PermissionLayer::L4GuardCheck,
        PermissionLayer::L5AuditCheck,
        PermissionLayer::L6ProvenanceCheck,
    ] {
        let b = m.get(DecisionPolicy::Conservative, *l).unwrap();
        assert_eq!(b.decision_rule, "deny", "conservative + {l:?} 应 deny");
    }
}

// 6. I4 Balanced → allow
#[test]
fn i4_06_balanced_allow() {
    let m = DecisionPermissionMatrix::default_matrix();
    for l in &[
        PermissionLayer::L1TypeCheck,
        PermissionLayer::L2ScopeCheck,
        PermissionLayer::L3RateCheck,
        PermissionLayer::L4GuardCheck,
        PermissionLayer::L5AuditCheck,
        PermissionLayer::L6ProvenanceCheck,
    ] {
        let b = m.get(DecisionPolicy::Balanced, *l).unwrap();
        assert_eq!(b.decision_rule, "allow", "balanced + {l:?} 应 allow");
    }
}

// 7. I4 Cautious → audit_required
#[test]
fn i4_07_cautious_audit() {
    let m = DecisionPermissionMatrix::default_matrix();
    let b = m
        .get(DecisionPolicy::Cautious, PermissionLayer::L1TypeCheck)
        .unwrap();
    assert_eq!(b.decision_rule, "audit_required");
}

// 8. I4 协调器 Conservative deny
#[test]
fn i4_08_coordinator_deny() {
    let mut c = DecisionPermissionCoordinator::new();
    let ctx = PermissionContext::safe_default();
    let rule = c.decide(
        0,
        DecisionPolicy::Conservative,
        PermissionLayer::L1TypeCheck,
        &ctx,
    );
    assert_eq!(rule, "deny");
    assert_eq!(c.report.deny_count(), 1);
}

// 9. I4 协调器 Balanced allow
#[test]
fn i4_09_coordinator_allow() {
    let mut c = DecisionPermissionCoordinator::new();
    let ctx = PermissionContext::safe_default();
    let rule = c.decide(
        0,
        DecisionPolicy::Balanced,
        PermissionLayer::L3RateCheck,
        &ctx,
    );
    assert_eq!(rule, "allow");
    assert_eq!(c.report.allow_count(), 1);
}

// 10. I4 协调器 多次 decide 累加
#[test]
fn i4_10_multi_decide() {
    let mut c = DecisionPermissionCoordinator::new();
    let ctx = PermissionContext::safe_default();
    c.decide(
        0,
        DecisionPolicy::Conservative,
        PermissionLayer::L1TypeCheck,
        &ctx,
    );
    c.decide(
        0,
        DecisionPolicy::Balanced,
        PermissionLayer::L1TypeCheck,
        &ctx,
    );
    c.decide(
        0,
        DecisionPolicy::Aggressive,
        PermissionLayer::L1TypeCheck,
        &ctx,
    );
    assert_eq!(c.report.event_count(), 3);
}

// 11. I4 AuditEvent 字段 + v7 baseline intact
#[test]
fn i4_11_audit_event_v7_intact() {
    let e = DecisionPermissionAuditEvent::new(
        0,
        DecisionPolicy::Balanced,
        PermissionLayer::L1TypeCheck,
        "allow",
        true,
    );
    assert!(e.v7_baseline_intact);
    assert_eq!(e.decision_rule, "allow");
}

// 12. I4 Report 默认
#[test]
fn i4_12_report_default() {
    let r = DecisionPermissionReport::default();
    assert_eq!(r.event_count(), 0);
}

// 13. I4 公共 API summary
#[test]
fn i4_13_summary() {
    let s = stage7_i4_summary();
    assert!(s.contains("I4"));
    assert!(s.contains("D4"));
    assert!(s.contains("G2"));
}

// 14. I4 公共 API healthy (B4 6 重 v7 严守)
#[test]
fn i4_14_healthy() {
    assert!(stage7_i4_healthy());
}

// 15. I4 跟 D4 协同
#[test]
fn i4_15_to_d4() {
    assert!(stage7_i4_to_d4_consistency());
}

// 16. I4 跟 G2 协同 (B4 6 重 v7 严守)
#[test]
fn i4_16_to_g2() {
    assert!(stage7_i4_to_g2_consistency());
}

// 17. I4 Binding 字段
#[test]
fn i4_17_binding_fields() {
    let b = DecisionPermissionBinding::new(
        DecisionPolicy::Aggressive,
        PermissionLayer::L6ProvenanceCheck,
        "allow",
    );
    assert_eq!(b.policy, DecisionPolicy::Aggressive);
    assert_eq!(b.layer, PermissionLayer::L6ProvenanceCheck);
    assert_eq!(b.decision_rule, "allow");
}
