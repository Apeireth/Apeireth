//! SWE-bench smoke example (v2-strategy §05 Step 6 验收入口)
//!
//! 运行方式:`cargo run -p apeireth-bench --example swe_bench_smoke`
//!
//! 验收条件:
//! 1. 进程退出码 = 0
//! 2. 控制台打印出至少 1 个 `RunReport` + 1 个 `Summary`
//! 3. `Summary::resolved == 1`(因为 `SimulatedExecutor` 默认跑全过)
//!
//! 不依赖 docker / git / 网络 —— 用 `SimulatedExecutor` 确保 smoke 永远可重现。
//!
//! ponytail:不做任何投机扩展(不打印 ANSI 颜色、不写 json 文件、不接 logger)。
//! 真要接 telemetry 时,在 `Runner::with_executor(...)` 后面插一层即可。

use apeireth_bench::swe_bench::{format_report, format_summary, smoke_runner};

fn main() {
    println!("🔬 apeireth-bench :: swe_bench_smoke");
    println!("===================================");
    println!("v2-strategy §05 Step 6 验收: 框架能加载 + 1 sample 跑通");
    println!();

    // 1) 框架加载(空调用,确认类型 / 函数都链接得上)
    let runner = smoke_runner();
    println!(
        "[load] runner ready: tasks={} executor=SimulatedExecutor",
        runner.tasks().len()
    );

    // 2) 执行 + 评分 + 聚合
    let (reports, summary) = runner.run_and_summarize();

    // 3) 输出每个 RunReport
    println!("[run ]");
    for r in &reports {
        println!("{}", format_report(r));
    }

    // 4) 摘要(SWE-bench 官方 Resolved@N 格式)
    println!();
    println!("{}", format_summary(&summary));

    // 5) sanity check —— 任何失败都 panic,让 CI 立刻看见
    assert_eq!(summary.total, 1, "smoke 应该恰好 1 个 task");
    assert_eq!(summary.resolved, 1, "SimulatedExecutor 下 resolved 应 = 1");
    assert_eq!(summary.avg_score_x100, 100, "avg score 应 = 1.00");
    assert!(reports[0].applied, "applied 应为 true");
    assert!(reports[0].resolved, "resolved 应为 true");

    println!();
    println!("✅ swe_bench_smoke 跑通 — 框架 OK,等真实 executor 接入 (P1+)");
}
