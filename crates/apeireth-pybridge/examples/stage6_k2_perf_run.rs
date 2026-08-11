//! R129-6 Stage 6 K2 性能守护 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage6_k2_perf_run`
//!
//! 演示 K2 性能守护: 5 kind 性能监控 + p95 阈值告警

use apeireth_pybridge::{
    stage6_perf_alerts, stage6_perf_healthy, stage6_perf_summary, stage6_record_perf, PerfKind,
    PerfSample, PerfStats,
};
use std::time::Duration;

fn main() {
    println!("=== R129-6 Stage 6 K2 性能守护 (anyone-can-run) ===\n");

    // 1. 演示 5 kind 性能记录
    for (kind, us) in [
        (PerfKind::Bridge, 150),
        (PerfKind::Eval, 300),
        (PerfKind::Import, 800),
        (PerfKind::Convert, 50),
        (PerfKind::Call, 250),
    ] {
        let s = stage6_record_perf(kind, Duration::from_micros(us), true);
        println!(
            "  recorded: {} {}μs (threshold={}μs) over={}",
            kind, us, s.threshold_us, s.over_threshold
        );
    }

    // 2. 演示 1 个 kind 100 样本聚合
    let samples: Vec<PerfSample> = (0..100)
        .map(|i| PerfSample::from_us(PerfKind::Bridge, i * 5, true))
        .collect();
    let stats = PerfStats::from_samples(&samples);
    println!("\n  Bridge 100 samples stats:\n{stats}");

    // 3. 摘要 + 告警
    println!("\n  summary: {}", stage6_perf_summary());
    let alerts = stage6_perf_alerts();
    println!("  alerts ({}):", alerts.len());
    for (k, p95, th) in &alerts {
        println!("    {k} p95={p95}μs > {th}μs");
    }
    println!("  healthy: {}", stage6_perf_healthy());

    println!("\n=== K2 done ===");
}
