//! R129-18 Stage 7 I5 G1+K2 资源+性能集成 - anyone-can-run example
//!
//! 跑 `cargo run -p apeireth-pybridge --example stage7_i5_resource_perf_run`
//!
//! 演示 I5: 4 resource dim × 5 perf kind = 20 绑定 + observe 阈值告警

use apeireth_pybridge::{
    stage7_i5_healthy, stage7_i5_summary, PerfKind, ResourceDimension, ResourcePerfCoordinator,
};

fn main() {
    println!("=== R129-18 Stage 7 I5 G1+K2 资源+性能集成 (anyone-can-run) ===\n");

    println!("[1] public API");
    println!("    summary: {}", stage7_i5_summary());
    println!("    healthy: {}", stage7_i5_healthy());

    let mut c = ResourcePerfCoordinator::new();
    println!("\n[2] observe 演示 (4 dim × 5 kind 阈值)");

    // 2.1 Rate + Bridge threshold=500us
    println!("\n    Rate + Bridge (threshold=500us):");
    let over = c.observe(1, ResourceDimension::Rate, PerfKind::Bridge, 100);
    println!("      observed=100us → over={over}");
    let over = c.observe(2, ResourceDimension::Rate, PerfKind::Bridge, 800);
    println!("      observed=800us → over={over}");

    // 2.2 Time + Import threshold=5000us
    println!("\n    Time + Import (threshold=5000us):");
    let over = c.observe(3, ResourceDimension::Time, PerfKind::Import, 1000);
    println!("      observed=1000us → over={over}");
    let over = c.observe(4, ResourceDimension::Time, PerfKind::Import, 8000);
    println!("      observed=8000us → over={over}");

    println!("\n[3] 报告统计");
    println!("    total events: {}", c.report.event_count());
    println!("    over threshold: {}", c.report.over_threshold_count());

    println!("\n=== I5 done ===");
}
