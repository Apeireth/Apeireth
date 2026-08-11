//! Stage 3 性能基准 runnable example
//!
//! 跑 5 target 性能基准 + 打印完整报告
//!
//! ```bash
//! cargo run -p apeireth-pybridge --example stage3_bench_run
//! ```

use apeireth_pybridge::{stage3_bench_run_default, stage3_bench_targets, BenchRunner, BenchConfig};

fn main() {
    println!("================================================================");
    println!("Stage 3 性能基准 (per decision-58 §2.1 P10-3)");
    println!("================================================================");
    println!();

    // 5 target 列表
    println!("5 内置 target (借鉴 superpowers 234 skill execution 模式):");
    for t in stage3_bench_targets() {
        println!("  - {:<22}  {}", t.id(), t.when_to_use());
    }
    println!();

    // 默认跑 (N=100, warmup=true)
    println!("--- 默认 5 target × 100 iter + warmup ---");
    let report = stage3_bench_run_default();
    println!("{report}");

    // 详细报告 (per target mean/p95)
    println!();
    println!("--- 详细 per-target mean/p95 ---");
    for tr in &report.target_reports {
        println!("  [{}] mean={:.2}μs p95={:.2}μs", tr.target_id, tr.stats.mean.as_nanos() as f64 / 1000.0, tr.stats.p95.as_nanos() as f64 / 1000.0);
    }

    // 性能总结
    println!();
    println!("--- 性能总结 ---");
    println!("总 wallclock: {:.2}ms", report.total_wallclock.as_micros() as f64 / 1000.0);
    let total_iters: usize = report.target_reports.iter().map(|t| t.stats.n).sum();
    println!("总迭代次数: {total_iters} (5 target × 100 iter)");
    println!(
        "吞吐: {:.0} iter/sec",
        total_iters as f64 / report.total_wallclock.as_secs_f64()
    );

    // 跑 1 个 target 详细
    println!();
    println!("--- BenchRunner::run_one 详细 (1 target) ---");
    let runner = BenchRunner::with_config(BenchConfig { iterations: 50, warmup: true });
    let r = runner.run_one(&apeireth_pybridge::BenchR11ModuleCount);
    println!("{r}");

    println!();
    println!("================================================================");
    println!("Stage 3 性能基准 done");
    println!("================================================================");
}
