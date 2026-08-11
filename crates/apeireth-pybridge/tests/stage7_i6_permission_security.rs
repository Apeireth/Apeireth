//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I6 G2+K3 权限+安全集成 集成测试
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **维度**: I6 G2+K3 权限+安全 跨 stage 集成
//! **目标**: 验证 G2 权限治理 (1:1 跟 B4 6 重 v7) 跟 K3 安全守护 (6+1 重门) 跨 stage 集成

use apeireth_pybridge::{
    stage7_i6_healthy, stage7_i6_summary, stage7_i6_to_g2_consistency,
    stage7_i6_to_k3_consistency, PermissionSecurityAuditEvent, PermissionSecurityBinding,
    PermissionSecurityCoordinator, PermissionSecurityMatrix, PermissionSecurityReport,
    PermissionLayer, SecurityGate, SecurityVerdict, STAGE7_I6_BINDING_COUNT,
    STAGE7_I6_DIMENSION_COUNT, STAGE7_I6_PERMISSION_LAYER_COUNT, STAGE7_I6_SECURITY_GATE_COUNT,
    STAGE7_I6_VERSION,
};

// 1. I6 编译期常数
#[test]
fn i6_01_version_constants() {
    assert_eq!(STAGE7_I6_VERSION, "0.1.0-R129-Stage7-I6");
    assert_eq!(STAGE7_I6_DIMENSION_COUNT, 2);
    assert_eq!(STAGE7_I6_PERMISSION_LAYER_COUNT, 6);
    assert_eq!(STAGE7_I6_SECURITY_GATE_COUNT, 7);
    assert_eq!(STAGE7_I6_BINDING_COUNT, 42);
}

// 2. I6 默认矩阵 6 × 7 = 42
#[test]
fn i6_02_matrix_size() {
    let m = PermissionSecurityMatrix::default_matrix();
    assert_eq!(m.len(), 42);
}

// 3. I6 v7 baseline 6 绑定 (L1-G1 ... L6-G6)
#[test]
fn i6_03_v7_baseline_count() {
    let m = PermissionSecurityMatrix::default_matrix();
    assert_eq!(m.v7_baseline_count(), 6);
}

// 4. I6 G7 跨语言 6 绑定 (任意 layer × G7)
#[test]
fn i6_04_g7_extension_count() {
    let m = PermissionSecurityMatrix::default_matrix();
    assert_eq!(m.g7_extension_count(), 6);
}

// 5. I6 L1TypeCheck + G1Identity = v7 baseline
#[test]
fn i6_05_l1_g1_v7_baseline() {
    let m = PermissionSecurityMatrix::default_matrix();
    let b = m.get(PermissionLayer::L1TypeCheck, SecurityGate::G1Identity).unwrap();
    assert!(b.is_v7_baseline);
    assert!(!b.is_g7_extension);
}

// 6. I6 任意 layer + G7CrossLanguage = G7 跨语言
#[test]
fn i6_06_g7_extension_binding() {
    let m = PermissionSecurityMatrix::default_matrix();
    for l in &[
        PermissionLayer::L1TypeCheck,
        PermissionLayer::L2ScopeCheck,
        PermissionLayer::L3RateCheck,
        PermissionLayer::L4GuardCheck,
        PermissionLayer::L5AuditCheck,
        PermissionLayer::L6ProvenanceCheck,
    ] {
        let b = m.get(*l, SecurityGate::G7CrossLanguage).unwrap();
        assert!(b.is_g7_extension, "{l:?} + G7 应是 G7 扩展");
    }
}

// 7. I6 非对齐组合 = neither
#[test]
fn i6_07_other_combination() {
    let m = PermissionSecurityMatrix::default_matrix();
    let b = m.get(PermissionLayer::L3RateCheck, SecurityGate::G1Identity).unwrap();
    assert!(!b.is_v7_baseline);
    assert!(!b.is_g7_extension);
}

// 8. I6 协调器 v7 baseline → Allow
#[test]
fn i6_08_coordinator_v7_allow() {
    let mut c = PermissionSecurityCoordinator::new();
    let v = c.check(0, PermissionLayer::L1TypeCheck, SecurityGate::G1Identity);
    assert!(matches!(v, SecurityVerdict::Allow));
    assert_eq!(c.report.allow_count(), 1);
}

// 9. I6 协调器 G7 跨语言 → Audit
#[test]
fn i6_09_coordinator_g7_audit() {
    let mut c = PermissionSecurityCoordinator::new();
    let v = c.check(0, PermissionLayer::L3RateCheck, SecurityGate::G7CrossLanguage);
    assert!(matches!(v, SecurityVerdict::Audit));
}

// 10. I6 协调器 非对齐 → Warn
#[test]
fn i6_10_coordinator_other_warn() {
    let mut c = PermissionSecurityCoordinator::new();
    let v = c.check(0, PermissionLayer::L3RateCheck, SecurityGate::G1Identity);
    assert!(matches!(v, SecurityVerdict::Warn));
}

// 11. I6 6 v7 baseline → 6 个 Allow
#[test]
fn i6_11_all_6_v7_allow() {
    let mut c = PermissionSecurityCoordinator::new();
    for (l, g) in [
        (PermissionLayer::L1TypeCheck, SecurityGate::G1Identity),
        (PermissionLayer::L2ScopeCheck, SecurityGate::G2Goal),
        (PermissionLayer::L3RateCheck, SecurityGate::G3Capability),
        (PermissionLayer::L4GuardCheck, SecurityGate::G4Compliance),
        (PermissionLayer::L5AuditCheck, SecurityGate::G5Resource),
        (PermissionLayer::L6ProvenanceCheck, SecurityGate::G6Audit),
    ] {
        let v = c.check(0, l, g);
        assert!(matches!(v, SecurityVerdict::Allow), "{l:?}+{g:?} 应 Allow");
    }
    assert_eq!(c.report.allow_count(), 6);
}

// 12. I6 AuditEvent 字段 + v7 baseline intact
#[test]
fn i6_12_audit_event_v7_intact() {
    let e = PermissionSecurityAuditEvent::new(
        100,
        PermissionLayer::L6ProvenanceCheck,
        SecurityGate::G6Audit,
        SecurityVerdict::Allow,
        true,
    );
    assert!(e.v7_baseline_intact);
}

// 13. I6 Report 默认
#[test]
fn i6_13_report_default() {
    let r = PermissionSecurityReport::default();
    assert_eq!(r.event_count(), 0);
}

// 14. I6 公共 API summary
#[test]
fn i6_14_summary() {
    let s = stage7_i6_summary();
    assert!(s.contains("I6"));
    assert!(s.contains("G2"));
    assert!(s.contains("K3"));
}

// 15. I6 公共 API healthy (B4 6 重 v7 + G7 6 严守)
#[test]
fn i6_15_healthy() {
    assert!(stage7_i6_healthy());
}

// 16. I6 跟 G2 协同
#[test]
fn i6_16_to_g2() {
    assert!(stage7_i6_to_g2_consistency());
}

// 17. I6 跟 K3 协同
#[test]
fn i6_17_to_k3() {
    assert!(stage7_i6_to_k3_consistency());
}

// 18. I6 Binding 字段
#[test]
fn i6_18_binding_fields() {
    let b = PermissionSecurityBinding::new(
        PermissionLayer::L1TypeCheck,
        SecurityGate::G1Identity,
    );
    assert_eq!(b.layer, PermissionLayer::L1TypeCheck);
    assert_eq!(b.gate, SecurityGate::G1Identity);
    assert!(b.is_v7_baseline);
}
