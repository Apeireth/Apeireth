//! R129-6 K4 健康守护 集成测试
//!
//! per decision-61 §3.1 R129-6 — Stage 6 守护 K4 维度 (5 维度 ASI 自检)

use apeireth_pybridge::{
    stage6_health_check, stage6_health_healthy, stage6_health_summary, HealthCheck,
    HealthDimension, HealthGuard, HealthReport, HealthStatus,
};

#[test]
fn k4_5_dimensions() {
    assert_eq!(HealthDimension::N_DIMENSIONS, 5);
    assert_eq!(HealthDimension::R11Compat.name(), "R11_Compat");
    assert_eq!(HealthDimension::AsiCritical.name(), "ASI_Critical");
    assert_eq!(HealthDimension::PyBridge.name(), "PyBridge");
    assert_eq!(HealthDimension::Security.name(), "Security");
    assert_eq!(HealthDimension::Performance.name(), "Performance");
}

#[test]
fn k4_status_4_levels() {
    assert_eq!(HealthStatus::N_STATUSES, 4);
    assert_eq!(HealthStatus::Unknown.score(), 0);
    assert_eq!(HealthStatus::Crit.score(), 0);
    assert_eq!(HealthStatus::Warn.score(), 50);
    assert_eq!(HealthStatus::Ok.score(), 100);
}

#[test]
fn k4_run_all_checks() {
    let r = stage6_health_check();
    assert_eq!(r.checks.len(), 10);
    // R11 1103 严守
    assert_eq!(r.r11_module_count, 1103);
    // ASI 7 严守
    assert_eq!(r.asi_module_count, 7);
}

#[test]
fn k4_health_report_aggregate() {
    let mut r = HealthReport::new();
    r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "a", "a"));
    r.add_check(HealthCheck::ok(HealthDimension::AsiCritical, "b", "b"));
    r.add_check(HealthCheck::ok(HealthDimension::PyBridge, "c", "c"));
    r.add_check(HealthCheck::ok(HealthDimension::Security, "d", "d"));
    r.add_check(HealthCheck::ok(HealthDimension::Performance, "e", "e"));
    r.aggregate();
    assert_eq!(r.total_score, 500);
    assert_eq!(r.max_score, 500);
    assert!((r.score_percent() - 100.0).abs() < 0.01);
}

#[test]
fn k4_health_check_helpers() {
    let c1 = HealthCheck::ok(HealthDimension::R11Compat, "x", "x");
    let c2 = HealthCheck::warn(HealthDimension::R11Compat, "x", "x");
    let c3 = HealthCheck::crit(HealthDimension::R11Compat, "x", "x");
    assert_eq!(c1.status, HealthStatus::Ok);
    assert_eq!(c2.status, HealthStatus::Warn);
    assert_eq!(c3.status, HealthStatus::Crit);
}

#[test]
fn k4_health_report_healthy() {
    let mut r = HealthReport::new();
    assert!(!r.is_healthy()); // 空 = not healthy
    r.add_check(HealthCheck::ok(HealthDimension::R11Compat, "a", "a"));
    assert!(r.is_healthy());
    r.add_check(HealthCheck::crit(HealthDimension::R11Compat, "b", "b"));
    assert!(!r.is_healthy());
}

#[test]
fn k4_health_guard_run() {
    let mut g = HealthGuard::new();
    let r = g.run_all_checks();
    assert_eq!(r.checks.len(), 10);
    assert_eq!(g.check_count, 1);
}

#[test]
fn k4_stage6_summary() {
    let _ = stage6_health_check();
    let s = stage6_health_summary();
    assert!(s.contains("K4 HealthGuard"));
}

#[test]
fn k4_stage6_healthy() {
    stage6_health_check(); // 先跑一次填充
    let _ = stage6_health_healthy();
}

#[test]
fn k4_dimension_display() {
    assert_eq!(format!("{}", HealthDimension::R11Compat), "R11_Compat");
    assert_eq!(format!("{}", HealthDimension::Performance), "Performance");
}

#[test]
fn k4_check_with_chain() {
    let c = HealthCheck::ok(HealthDimension::AsiCritical, "test", "ok")
        .with_expected("7")
        .with_actual("7")
        .with_timestamp(42);
    assert_eq!(c.expected.as_deref(), Some("7"));
    assert_eq!(c.actual.as_deref(), Some("7"));
    assert_eq!(c.timestamp, 42);
}

#[test]
fn k4_health_report_score_zero() {
    let r = HealthReport::default();
    assert_eq!(r.score_percent(), 0.0);
}

#[test]
fn k4_health_report_all_ok_default() {
    let r = HealthReport::default();
    assert!(!r.all_ok);
}
