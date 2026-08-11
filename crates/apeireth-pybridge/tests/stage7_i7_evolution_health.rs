//! R129-18 ASI Python 整合 Stage 7 跨模块集成 — I7 G4+K4 演进+健康集成 集成测试
//!
//! **任务**: ASI Stage 7 跨模块集成 (per decision-61 §3.1 R129-18)
//! **维度**: I7 G4+K4 演进+健康 跨 stage 集成
//! **目标**: 验证 G4 演进治理跟 K4 健康守护跨 stage 集成

use apeireth_pybridge::{
    stage7_i7_healthy, stage7_i7_summary, stage7_i7_to_g4_consistency,
    stage7_i7_to_k4_consistency, EvolutionHealthAuditEvent, EvolutionHealthBinding,
    EvolutionHealthCoordinator, EvolutionHealthMatrix, EvolutionHealthReport, EvolutionKind,
    HealthDimension, STAGE7_I7_BINDING_COUNT, STAGE7_I7_DIMENSION_COUNT,
    STAGE7_I7_EVOLUTION_KIND_COUNT, STAGE7_I7_HEALTH_DIM_COUNT, STAGE7_I7_VERSION,
};

// 1. I7 编译期常数
#[test]
fn i7_01_version_constants() {
    assert_eq!(STAGE7_I7_VERSION, "0.1.0-R129-Stage7-I7");
    assert_eq!(STAGE7_I7_DIMENSION_COUNT, 2);
    assert_eq!(STAGE7_I7_EVOLUTION_KIND_COUNT, 4);
    assert_eq!(STAGE7_I7_HEALTH_DIM_COUNT, 5);
    assert_eq!(STAGE7_I7_BINDING_COUNT, 20);
}

// 2. I7 默认矩阵 4 × 5 = 20
#[test]
fn i7_02_matrix_size() {
    let m = EvolutionHealthMatrix::default_matrix();
    assert_eq!(m.len(), 20);
}

// 3. I7 4 kinds 都有 binding
#[test]
fn i7_03_all_4_kinds() {
    let m = EvolutionHealthMatrix::default_matrix();
    let kinds = [
        EvolutionKind::Add,
        EvolutionKind::Upgrade,
        EvolutionKind::Downgrade,
        EvolutionKind::Retire,
    ];
    for k in &kinds {
        assert!(m.get(*k, HealthDimension::Security).is_some(), "kind {k:?} 缺");
    }
}

// 4. I7 5 health dims 都有 binding
#[test]
fn i7_04_all_5_dims() {
    let m = EvolutionHealthMatrix::default_matrix();
    for d in &[
        HealthDimension::R11Compat,
        HealthDimension::AsiCritical,
        HealthDimension::PyBridge,
        HealthDimension::Security,
        HealthDimension::Performance,
    ] {
        assert!(m.get(EvolutionKind::Add, *d).is_some(), "dim {d:?} 缺");
    }
}

// 5. I7 Add + Security = positive impact
#[test]
fn i7_05_add_security_positive() {
    let m = EvolutionHealthMatrix::default_matrix();
    let b = m.get(EvolutionKind::Add, HealthDimension::Security).unwrap();
    assert_eq!(b.health_impact, "positive");
    assert!(b.requires_health_check);
}

// 6. I7 Downgrade + R11Compat = negative impact
#[test]
fn i7_06_downgrade_r11_negative() {
    let m = EvolutionHealthMatrix::default_matrix();
    let b = m.get(EvolutionKind::Downgrade, HealthDimension::R11Compat).unwrap();
    assert_eq!(b.health_impact, "negative");
}

// 7. I7 Retire + Performance = neutral
#[test]
fn i7_07_retire_performance_neutral() {
    let m = EvolutionHealthMatrix::default_matrix();
    let b = m.get(EvolutionKind::Retire, HealthDimension::Performance).unwrap();
    assert_eq!(b.health_impact, "neutral");
}

// 8. I7 Upgrade + AsiCritical = positive
#[test]
fn i7_08_upgrade_asi_positive() {
    let m = EvolutionHealthMatrix::default_matrix();
    let b = m.get(EvolutionKind::Upgrade, HealthDimension::AsiCritical).unwrap();
    assert_eq!(b.health_impact, "positive");
}

// 9. I7 协调器 evolve 5 health dim
#[test]
fn i7_09_coordinator_evolve_5_dims() {
    let mut c = EvolutionHealthCoordinator::new();
    let _ok = c.evolve(0, EvolutionKind::Add);
    assert_eq!(c.report.event_count(), 5);
}

// 10. I7 协调器多次 evolve 累加
#[test]
fn i7_10_multi_evolve() {
    let mut c = EvolutionHealthCoordinator::new();
    c.evolve(0, EvolutionKind::Add);
    c.evolve(0, EvolutionKind::Upgrade);
    c.evolve(0, EvolutionKind::Downgrade);
    assert_eq!(c.report.event_count(), 15); // 3 kinds × 5 dims
}

// 11. I7 AuditEvent 字段
#[test]
fn i7_11_audit_event_fields() {
    let e = EvolutionHealthAuditEvent::new(
        100,
        EvolutionKind::Upgrade,
        HealthDimension::Security,
        "positive",
        true,
    );
    assert_eq!(e.timestamp, 100);
    assert!(e.health_check_passed);
    assert_eq!(e.health_impact, "positive");
}

// 12. I7 Report 默认
#[test]
fn i7_12_report_default() {
    let r = EvolutionHealthReport::default();
    assert_eq!(r.event_count(), 0);
}

// 13. I7 公共 API summary
#[test]
fn i7_13_summary() {
    let s = stage7_i7_summary();
    assert!(s.contains("I7"));
    assert!(s.contains("G4"));
    assert!(s.contains("K4"));
}

// 14. I7 公共 API healthy
#[test]
fn i7_14_healthy() {
    assert!(stage7_i7_healthy());
}

// 15. I7 跟 G4 协同
#[test]
fn i7_15_to_g4() {
    assert!(stage7_i7_to_g4_consistency());
}

// 16. I7 跟 K4 协同
#[test]
fn i7_16_to_k4() {
    assert!(stage7_i7_to_k4_consistency());
}

// 17. I7 Binding 字段
#[test]
fn i7_17_binding_fields() {
    let b = EvolutionHealthBinding::new(
        EvolutionKind::Upgrade,
        HealthDimension::PyBridge,
        "positive",
        true,
    );
    assert_eq!(b.kind, EvolutionKind::Upgrade);
    assert_eq!(b.health_dim, HealthDimension::PyBridge);
    assert!(b.requires_health_check);
}
