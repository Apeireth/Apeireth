//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I5 G1+K2 资源+性能集成 集成测试
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **维度**: I5 G1+K2 资源+性能 跨 stage 集成
//! **目标**: 验证 G1 资源治理跟 K2 性能守护跨 stage 集成

use apeireth_pybridge::{
    stage7_i5_healthy, stage7_i5_summary, stage7_i5_to_g1_consistency,
    stage7_i5_to_k2_consistency, ResourcePerfAuditEvent, ResourcePerfBinding,
    ResourcePerfCoordinator, ResourcePerfMatrix, ResourcePerfReport, PerfKind, ResourceDimension,
    STAGE7_I5_BINDING_COUNT, STAGE7_I5_DIMENSION_COUNT, STAGE7_I5_PERF_KIND_COUNT,
    STAGE7_I5_RESOURCE_DIM_COUNT, STAGE7_I5_VERSION,
};

// 1. I5 编译期常数
#[test]
fn i5_01_version_constants() {
    assert_eq!(STAGE7_I5_VERSION, "0.1.0-R129-Stage7-I5");
    assert_eq!(STAGE7_I5_DIMENSION_COUNT, 2);
    assert_eq!(STAGE7_I5_PERF_KIND_COUNT, 5);
    assert_eq!(STAGE7_I5_RESOURCE_DIM_COUNT, 4);
    assert_eq!(STAGE7_I5_BINDING_COUNT, 20);
}

// 2. I5 默认矩阵 4 × 5 = 20
#[test]
fn i5_02_matrix_size() {
    let m = ResourcePerfMatrix::default_matrix();
    assert_eq!(m.len(), 20);
}

// 3. I5 Rate + Bridge = 500us
#[test]
fn i5_03_rate_bridge_threshold() {
    let m = ResourcePerfMatrix::default_matrix();
    let b = m.get(ResourceDimension::Rate, PerfKind::Bridge).unwrap();
    assert_eq!(b.threshold_us, 500);
}

// 4. I5 Memory + Eval = 1000us
#[test]
fn i5_04_memory_eval_threshold() {
    let m = ResourcePerfMatrix::default_matrix();
    let b = m.get(ResourceDimension::Memory, PerfKind::Eval).unwrap();
    assert_eq!(b.threshold_us, 1000);
}

// 5. I5 Time + Import = 5000us
#[test]
fn i5_05_time_import_threshold() {
    let m = ResourcePerfMatrix::default_matrix();
    let b = m.get(ResourceDimension::Time, PerfKind::Import).unwrap();
    assert_eq!(b.threshold_us, 5000);
}

// 6. I5 Count + Call = 800us
#[test]
fn i5_06_count_call_threshold() {
    let m = ResourcePerfMatrix::default_matrix();
    let b = m.get(ResourceDimension::Count, PerfKind::Call).unwrap();
    assert_eq!(b.threshold_us, 800);
}

// 7. I5 协调器 under threshold
#[test]
fn i5_07_under_threshold() {
    let mut c = ResourcePerfCoordinator::new();
    let over = c.observe(0, ResourceDimension::Rate, PerfKind::Bridge, 100);
    assert!(!over);
    assert_eq!(c.report.over_threshold_count(), 0);
}

// 8. I5 协调器 over threshold
#[test]
fn i5_08_over_threshold() {
    let mut c = ResourcePerfCoordinator::new();
    let over = c.observe(0, ResourceDimension::Rate, PerfKind::Bridge, 1000);
    assert!(over);
    assert_eq!(c.report.over_threshold_count(), 1);
}

// 9. I5 5 perf kinds 都查得到
#[test]
fn i5_09_all_5_kinds() {
    let m = ResourcePerfMatrix::default_matrix();
    let kinds = [
        PerfKind::Bridge,
        PerfKind::Eval,
        PerfKind::Import,
        PerfKind::Convert,
        PerfKind::Call,
    ];
    for k in &kinds {
        assert!(m.get(ResourceDimension::Rate, *k).is_some(), "kind {k:?} 缺");
    }
}

// 10. I5 4 dims 都查得到
#[test]
fn i5_10_all_4_dims() {
    let m = ResourcePerfMatrix::default_matrix();
    for d in &ResourceDimension::ALL {
        assert!(m.get(*d, PerfKind::Bridge).is_some(), "dim {d:?} 缺");
    }
}

// 11. I5 AuditEvent 字段
#[test]
fn i5_11_audit_event_fields() {
    let e = ResourcePerfAuditEvent::new(
        100,
        ResourceDimension::Count,
        PerfKind::Call,
        800,
        1500,
        true,
    );
    assert_eq!(e.threshold_us, 800);
    assert_eq!(e.observed_us, 1500);
    assert!(e.over_threshold);
}

// 12. I5 Report 默认
#[test]
fn i5_12_report_default() {
    let r = ResourcePerfReport::default();
    assert_eq!(r.event_count(), 0);
}

// 13. I5 公共 API summary
#[test]
fn i5_13_summary() {
    let s = stage7_i5_summary();
    assert!(s.contains("I5"));
    assert!(s.contains("G1"));
    assert!(s.contains("K2"));
}

// 14. I5 公共 API healthy
#[test]
fn i5_14_healthy() {
    assert!(stage7_i5_healthy());
}

// 15. I5 跟 G1 协同
#[test]
fn i5_15_to_g1() {
    assert!(stage7_i5_to_g1_consistency());
}

// 16. I5 跟 K2 协同
#[test]
fn i5_16_to_k2() {
    assert!(stage7_i5_to_k2_consistency());
}

// 17. I5 Binding 字段
#[test]
fn i5_17_binding_fields() {
    let b = ResourcePerfBinding::new(ResourceDimension::Time, PerfKind::Import, 5000);
    assert_eq!(b.dimension, ResourceDimension::Time);
    assert_eq!(b.perf_kind, PerfKind::Import);
    assert_eq!(b.threshold_us, 5000);
}
