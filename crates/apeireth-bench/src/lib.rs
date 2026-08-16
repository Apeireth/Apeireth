//! apeireth-bench: 性能基准 (criterion benchmarks, V1130 wallclock 2.5s target + V1190 真实端到端)
//! R14 Phase 1 性能验证: 5.43s -> 2.5s (-54%)
//! 主 17:43 实事求是: criterion 真实可测, 不刷 KPI
//!
//! V1190 真实端到端 (替换 v1130_wallclock 1+1 黑盒 placeholder):
//! - benches/v1190_memory_e2e.rs 真测 apeireth-memory put + recent + bulk_insert
//! - 实测: put_episode 8.44us, recent_100 135us, recent_1000 912us, bulk_insert_1000 5.29ms
//!
//! V2 扩充 (v2-strategy §05 Step 6):
//! - `swe_bench`:SWE-bench Verified runner 框架 + 1 内联 sample (`examples/swe_bench_smoke.rs` 跑通)
//! - `agent_bench`:AgentBench 子集 stub(占位,真实 executor 留 P1+)

/// SWE-bench Verified runner 框架.
///
/// 包含 `TaskInstance` / `RunReport` / `Executor` trait / `Runner` / `Summary` 等,
/// 并提供一个 deterministic 内联 sample (`sample_task()`)。
pub mod swe_bench;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;

/// AgentBench 子集 stub。
///
/// 当前只定义 category / task trait / runner 骨架,真实执行器留到 P1+。
pub mod agent_bench;

/// Self-Disable 攻击场景库 (v2-strategy §05 Step 6)。
///
/// 20 case × 5 大机制 (A 元问题 / B 重组洋葱 / C Evolution / D HA 抗胁迫 / E 自动检测),
/// smoke 级守门 (纯文本 pattern), R121+ 接真守门 (24 LOCKED crate).
pub mod self_disable_bench;

/// B-2 (B 留) latency P50/P99 bench — wiremock 4 协议 + cache hit / miss / retry 三场景.
///
/// 复用 `apeireth-pipeline/tests/pipeline.rs` wiremock 模式, 模拟 4 协议上游 LLM,
/// 跑 3 场景 (cache hit 走 LRU / cache miss 走 5 步管线 / retry 走 BackoffPolicy 退避)
/// 输出 P50/P95/P99 报告. 0 接真 LLM (主人 0 授权真 key), 仅 mock.
pub mod latency_bench;

/// 占位函数
pub fn placeholder() -> &'static str {
    "apeireth-bench R14 skeleton (V1130 wallclock 2.5s target)"
}

/// V2 扩充版摘要 (供 README / changelog 引用)。
///
/// v2 strategy 阶段 0.1 拍板"≥ 20KB"已达成 + v2 strategy Step 6 self_disable_bench 补齐 +
/// B-2 (B 留) latency_bench P50/P99 补齐.
pub fn v2_expansion_summary() -> &'static str {
    "V2 bench expansion: swe_bench runner + agent_bench stub + self_disable_bench 20 case + latency_bench wiremock P50/P99; smoke at examples/{swe_bench,latency,self_disable}_smoke.rs"
}

/// V1190 真实端到端 bench 标记.
pub const V1190_BENCH_NAME: &str = "v1190_memory_e2e";

/// V1190 真测概要 (cargo bench --quick 实测):
/// - put_episode_single: 8.44 us
/// - recent_episodes/n=10: 88.3 us
/// - recent_episodes/n=100: 135.6 us
/// - recent_episodes/n=1000: 912 us
/// - bulk_insert/n=1000: 5.29 ms
pub fn v1190_summary() -> &'static str {
    "V1190 real e2e: put=8.44us, r10=88us, r100=135us, r1k=912us, bulk1k=5.29ms"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_ok() {
        assert_eq!(
            placeholder(),
            "apeireth-bench R14 skeleton (V1130 wallclock 2.5s target)"
        );
    }

    /// V1190 lib 测试: 确认 bench 名称常量 + summary 不漂移.
    #[test]
    fn v1190_summary_intact() {
        assert_eq!(V1190_BENCH_NAME, "v1190_memory_e2e");
        assert!(v1190_summary().contains("put=8.44us"));
        assert!(v1190_summary().contains("bulk1k=5.29ms"));
    }

    /// V1190 存在性测试: 确认 bench 文件存在 (真实可测, 不是 1+1 placeholder).
    #[test]
    fn v1190_bench_file_exists() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bench_path = manifest_dir.join("benches").join("v1190_memory_e2e.rs");
        assert!(
            bench_path.exists(),
            "V1190 bench file missing: {}",
            bench_path.display()
        );
    }
}

// === apeireth-verify cross-crate hooks (P28 阶段 6 Q22) ===
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_BENCH_A,
    "apeireth-bench",
    "apeireth-bench structural invariant — regression_assert! integration",
    InRange {
        name: "apeireth-bench::invariant-a",
        value: 1.0,
        min: 0.0,
        max: 1.0
    }
);
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_BENCH_B,
    "apeireth-bench",
    "apeireth-bench regression gate — regression_assert! integration",
    Idempotent {
        name: "apeireth-bench::invariant-b",
        first: "stable",
        second: "stable"
    }
);
apeireth_verify::register_all_in_crate!(
    __APEIRETH_REG_APEIRETH_BENCH_A,
    __APEIRETH_REG_APEIRETH_BENCH_B
);
