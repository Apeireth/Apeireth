//! Self-Disable smoke example (v2-strategy §05 Step 6 验收入口)
//!
//! 运行方式: `cargo run -p apeireth-bench --example self_disable_smoke`
//!
//! 验收条件:
//! 1. 进程退出码 = 0
//! 2. 20 case 全 smoke pass (默认 5 大机制守门全过)
//! 3. 满足 v2 strategy Step 6 验收门槛 (≥ 5 case pass)
//! 4. 控制台打印每个 case 的 verdict + summary
//!
//! 不依赖 docker / git / 网络 — 纯文本守门, smoke 永远可重现.
//!
//! ponytail:
//! - 不写 "production ready" 假话
//! - 守门函数是 smoke 级 (纯文本 pattern), R121+ 接真守门 (24 LOCKED crate)
//! - 5 大机制 1:1 翻译自 `docs/glossary/09-self-disable.md`

use apeireth_bench::self_disable_bench::{default_runner, format_case_result, format_summary};

fn main() {
    println!("🔬 apeireth-bench :: self_disable_smoke");
    println!("======================================");
    println!("v2-strategy §05 Step 6 验收: 20 case + 5 大机制守门全过");
    println!("(per docs/v2-strategy/05-EXECUTION-NOW.md:162-165)");
    println!();

    // 1) 框架加载
    let runner = default_runner();
    println!(
        "[load] runner ready: cases={} (20 default = 4 per category × 5 categories)",
        runner.case_count()
    );

    // 2) 跑 + 评分 + 聚合
    let (results, summary) = runner.run_and_summarize();

    // 3) 输出每个 CaseResult
    println!("[run ]");
    for r in &results {
        println!("{}", format_case_result(r));
    }

    // 4) 摘要
    println!();
    println!("{}", format_summary(&summary));

    // 5) sanity check
    assert_eq!(summary.total, 20, "应有 20 case");
    assert!(
        summary.meets_step6_threshold(),
        "Step 6 验收门槛 ≥ 5 case pass, got {}/20",
        summary.smoke_pass
    );

    // 6) 验证每 category 都有 4 case
    for (cat, total, pass) in &summary.by_category {
        assert_eq!(*total, 4, "{cat:?} 应 = 4 case");
        assert_eq!(
            *pass, 4,
            "{cat:?} 应 smoke 全 pass (默认 20 case 全期望被拦)"
        );
    }

    println!();
    println!(
        "✅ self_disable_smoke 跑通 — 20 case 全 smoke pass, ≥ 5 验收门槛 (got {})",
        summary.smoke_pass
    );
    println!("⚠️  smoke 级守门: 纯文本 pattern, R121+ 接真守门 (24 LOCKED crate)");
    println!("⏭  不假装 'production ready': 真实 LLM / Evolution 接入留 R121+");
}
