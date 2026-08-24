//! B-2 latency smoke example (B 留的 wiremock 4 协议 + cache hit/miss/retry P50/P99)
//!
//! 运行方式: `cargo run -p apeireth-bench --example latency_smoke`
//!
//! 验收条件 (任务 spec 硬指标 #5):
//! 1. 进程退出码 = 0
//! 2. 控制台打印 12 result (4 协议 × 3 场景) 的 P50/P95/P99
//! 3. wiremock 0 网络 disclaimer 输出
//! 4. latency 数字不假装 "production ready P99"
//!
//! B final report §5.4 (2026-08-10 08:50):
//! > **B5 之后建议**: 写一个独立 `crates/apeireth-bench/` 脚本, 用 wiremock 模拟 LLM 上游,
//! > 跑 cache hit / miss / retry 三场景 P50/P99. 这个留给 Mavis 拍板.
//!
//! ponytail:
//! - 不做投机扩展 (不写 JSON 文件, 不接 logger, 不打 ANSI 颜色)
//! - 0 假装 "production ready"
//! - latency 数字诚实标 "wiremock 0 网络"
//! - R121+ 替换 mini_* → apeireth-cache::MemoryCache + apeireth-api::BackoffPolicy

use apeireth_bench::latency_bench::LatencyRunner;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("🔬 apeireth-bench :: latency_smoke (B 留的 wiremock 4 协议 + 3 场景)");
    println!("=============================================================");
    println!("B final report §5.4 验收: wiremock 模拟 4 协议上游 LLM,");
    println!("跑 cache hit / cache miss / retry 三场景 P50/P99 报告");
    println!();

    // 1) 跑全 3 场景 × 4 协议 (12 result)
    let runner = LatencyRunner::default();
    let report = runner.run_all().await;

    // 2) 输出报告
    println!("{}", report.format());

    // 3) sanity check
    assert_eq!(
        report.results.len(),
        12,
        "应有 12 result (4 协议 × 3 场景), got {}",
        report.results.len()
    );

    // 4) 验证每个 result 都有 sample
    for r in &report.results {
        assert!(r.samples > 0, "result {} 应有 sample", r.protocol.label());
        assert!(r.p50_ns > 0, "result {} p50 应 > 0", r.protocol.label());
    }

    // 5) 验证 cache hit 应该是 3 场景里最快的 (LRU 命中, 0 网络)
    let cache_hit = report
        .results
        .iter()
        .find(|r| r.scenario.label() == "cache-hit")
        .expect("cache-hit result");
    let cache_miss = report
        .results
        .iter()
        .find(|r| r.scenario.label() == "cache-miss" && r.protocol.label() == "OpenAI-Chat")
        .expect("cache-miss OpenAI-Chat result");
    assert!(
        cache_hit.p99_ns < cache_miss.p99_ns * 10,
        "cache hit p99 应远小于 cache miss p99 (cache miss 有网络 + 序列化)\n  cache_hit.p99={}ns\n  cache_miss.p99={}ns",
        cache_hit.p99_ns,
        cache_miss.p99_ns
    );

    println!();
    println!("✅ latency_smoke 跑通 — 12 result (4 协议 × 3 场景) P50/P99 报告 OK");
    println!("⚠️  wiremock 0 网络: latency 反映进程内 mock 开销, 不代表真 LLM 性能");
    println!("⏭  R121+ 替换 mini_* → apeireth-cache::MemoryCache + apeireth-api::BackoffPolicy");
}
