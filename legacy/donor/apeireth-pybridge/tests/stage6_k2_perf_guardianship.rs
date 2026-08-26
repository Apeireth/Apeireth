//! R129-6 K2 性能守护 集成测试
//!
//! per decision-61 §3.1 R129-6 — Stage 6 守护 K2 维度

use apeireth_pybridge::{
    stage6_perf_alerts, stage6_perf_healthy, stage6_perf_summary, stage6_record_perf, PerfKind,
    PerfMonitor, PerfSample, PerfStats,
};
use std::time::Duration;

#[test]
fn k2_record_and_summary() {
    let s = stage6_record_perf(PerfKind::Bridge, Duration::from_micros(100), true);
    assert_eq!(s.kind, PerfKind::Bridge);
    let sum = stage6_perf_summary();
    assert!(sum.contains("K2 PerfMonitor"));
}

#[test]
fn k2_perf_kind_5_thresholds() {
    assert_eq!(PerfKind::N_KINDS, 5);
    assert_eq!(PerfKind::Bridge.default_threshold_us(), 500);
    assert_eq!(PerfKind::Eval.default_threshold_us(), 1000);
    assert_eq!(PerfKind::Import.default_threshold_us(), 5000);
    assert_eq!(PerfKind::Convert.default_threshold_us(), 100);
    assert_eq!(PerfKind::Call.default_threshold_us(), 800);
}

#[test]
fn k2_perf_stats_aggregate() {
    let samples: Vec<PerfSample> = (0..50)
        .map(|i| PerfSample::from_us(PerfKind::Bridge, i as u128, true))
        .collect();
    let s = PerfStats::from_samples(&samples);
    assert_eq!(s.count, 50);
    assert_eq!(s.min_us, 0);
    assert_eq!(s.max_us, 49);
    // p95 (95th percentile) > p50 (50th percentile) for non-constant samples
    assert!(s.p95_us > s.p50_us);
}

#[test]
fn k2_perf_monitor_5_kind_isolation() {
    let mut m = PerfMonitor::default();
    for k in [
        PerfKind::Bridge,
        PerfKind::Eval,
        PerfKind::Import,
        PerfKind::Convert,
        PerfKind::Call,
    ] {
        m.record(PerfSample::from_us(k, 100, true));
    }
    for k in [
        PerfKind::Bridge,
        PerfKind::Eval,
        PerfKind::Import,
        PerfKind::Convert,
        PerfKind::Call,
    ] {
        assert_eq!(m.samples[k.idx()].len(), 1);
    }
    assert_eq!(m.total_recorded, 5);
}

#[test]
fn k2_perf_monitor_lru() {
    let mut m = PerfMonitor::new(3);
    for i in 0..5 {
        m.record(PerfSample::from_us(PerfKind::Bridge, i, true));
    }
    assert_eq!(m.samples[PerfKind::Bridge.idx()].len(), 3);
    assert_eq!(m.total_dropped, 2);
}

#[test]
fn k2_perf_alerts() {
    let _ = stage6_perf_alerts();
}

#[test]
fn k2_healthy_no_alerts() {
    let _ = stage6_perf_healthy();
}

#[test]
fn k2_sample_from_duration() {
    let s = PerfSample::from_duration(PerfKind::Call, Duration::from_micros(200), true);
    assert_eq!(s.latency_us, 200);
    assert!(s.success);
    assert!(!s.over_threshold); // 200 < 800
}

#[test]
fn k2_sample_over_threshold() {
    let s = PerfSample::from_us(PerfKind::Convert, 200, true);
    assert!(s.over_threshold); // 200 > 100
}

#[test]
fn k2_summary_contains_recorded() {
    stage6_record_perf(PerfKind::Eval, Duration::from_micros(50), true);
    let s = stage6_perf_summary();
    assert!(s.contains("K2 PerfMonitor"));
    assert!(s.contains("recorded="));
}
