//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I2 D2+K1 反思+错误集成 集成测试
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **维度**: I2 D2+K1 反思+错误 跨 stage 集成
//! **目标**: 验证 D2 反思自循环跟 K1 错误守护跨 stage 集成

use apeireth_pybridge::{
    stage7_i2_healthy, stage7_i2_summary, stage7_i2_to_d2_consistency,
    stage7_i2_to_k1_consistency, ReflectionErrorAuditEvent, ReflectionErrorBinding,
    ReflectionErrorCoordinator, ReflectionErrorMatrix, ReflectionErrorReport, ErrorKind,
    STAGE7_I2_BINDING_COUNT, STAGE7_I2_DIMENSION_COUNT, STAGE7_I2_ERROR_KIND_COUNT,
    STAGE7_I2_NODE_COUNT, STAGE7_I2_VERSION,
};

// 1. I2 编译期常数
#[test]
fn i2_01_version_constants() {
    assert_eq!(STAGE7_I2_VERSION, "0.1.0-R129-Stage7-I2");
    assert_eq!(STAGE7_I2_DIMENSION_COUNT, 2);
    assert_eq!(STAGE7_I2_NODE_COUNT, 8);
    assert_eq!(STAGE7_I2_ERROR_KIND_COUNT, 4);
    assert_eq!(STAGE7_I2_BINDING_COUNT, 32);
}

// 2. I2 默认矩阵 8 × 4 = 32
#[test]
fn i2_02_matrix_size() {
    let m = ReflectionErrorMatrix::default_matrix();
    assert_eq!(m.len(), 32);
}

// 3. I2 8 nodes 都有 binding (per node)
#[test]
fn i2_03_all_8_nodes() {
    let m = ReflectionErrorMatrix::default_matrix();
    let nodes = [
        "observe",
        "analyze",
        "reflect",
        "refine",
        "finalize",
        "internal_audit",
        "internal_ceiling",
        "internal_harness",
    ];
    for n in &nodes {
        assert!(m.get(n, ErrorKind::Bridge).is_some(), "node {n} 缺");
    }
}

// 4. I2 4 kinds 都有 binding
#[test]
fn i2_04_all_4_kinds() {
    let m = ReflectionErrorMatrix::default_matrix();
    let kinds = [
        ErrorKind::Transport,
        ErrorKind::Conversion,
        ErrorKind::Bridge,
        ErrorKind::Contract,
    ];
    for k in &kinds {
        assert!(m.get("observe", *k).is_some(), "kind {:?} 缺", k);
    }
}

// 5. I2 get 缺 → None
#[test]
fn i2_05_get_missing() {
    let m = ReflectionErrorMatrix::default_matrix();
    assert!(m.get("nonexistent", ErrorKind::Bridge).is_none());
}

// 6. I2 default binding auto_retry
#[test]
fn i2_06_default_binding_strategy() {
    let b = ReflectionErrorBinding::default_binding("observe", ErrorKind::Bridge);
    assert_eq!(b.recovery_strategy, "auto_retry");
}

// 7. I2 协调器 auto_retry success
#[test]
fn i2_07_coordinator_success() {
    let mut c = ReflectionErrorCoordinator::new();
    let ok = c.reflect_and_recover(0, "observe", ErrorKind::Bridge);
    assert!(ok);
    assert_eq!(c.report.event_count(), 1);
    assert_eq!(c.report.recovery_success_count(), 1);
}

// 8. I2 协调器 no_binding fail
#[test]
fn i2_08_coordinator_fail() {
    let mut c = ReflectionErrorCoordinator::new();
    let ok = c.reflect_and_recover(0, "nonexistent", ErrorKind::Bridge);
    assert!(!ok);
    assert_eq!(c.report.recovery_failed_count(), 1);
}

// 9. I2 4 kinds 都能 recover
#[test]
fn i2_09_all_4_kinds_recover() {
    let mut c = ReflectionErrorCoordinator::new();
    for k in [
        ErrorKind::Transport,
        ErrorKind::Conversion,
        ErrorKind::Bridge,
        ErrorKind::Contract,
    ] {
        let ok = c.reflect_and_recover(0, "observe", k);
        assert!(ok, "kind {k:?} 恢复失败");
    }
    assert_eq!(c.report.event_count(), 4);
}

// 10. I2 AuditEvent 字段
#[test]
fn i2_10_audit_event_fields() {
    let e = ReflectionErrorAuditEvent::new(123, "analyze", ErrorKind::Contract, "auto_retry", true);
    assert_eq!(e.timestamp, 123);
    assert_eq!(e.node_id, "analyze");
    assert!(e.recovery_success);
}

// 11. I2 Report 默认
#[test]
fn i2_11_report_default() {
    let r = ReflectionErrorReport::default();
    assert_eq!(r.event_count(), 0);
}

// 12. I2 公共 API summary
#[test]
fn i2_12_summary() {
    let s = stage7_i2_summary();
    assert!(s.contains("I2"));
    assert!(s.contains("D2"));
    assert!(s.contains("K1"));
}

// 13. I2 公共 API healthy
#[test]
fn i2_13_healthy() {
    assert!(stage7_i2_healthy());
}

// 14. I2 跟 D2 协同
#[test]
fn i2_14_to_d2() {
    assert!(stage7_i2_to_d2_consistency());
}

// 15. I2 跟 K1 协同
#[test]
fn i2_15_to_k1() {
    assert!(stage7_i2_to_k1_consistency());
}
