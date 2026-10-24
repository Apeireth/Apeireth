//! 并行工具派发在**生产装配**里真的装上了（不是只有库级能力）。
//!
//! # 这条测试钉什么
//!
//! 运行时完整支持并行派发（`plan_parallel_window` + `dispatch_parallel_window`），
//! 但整条链挂在 `RuntimeBuilder::with_call_scheduler` 上。此前生产装配
//! **从不构造 `CallScheduler`** ⇒ 窗口恒为 1 ⇒ 并行派发形同不存在。
//!
//! 本测试从**生产装配**出发，断言：
//! 1. 生产运行时里**装上了**调度器（`Runtime::call_scheduler()` 为 `Some`）；
//! 2. 该调度器的白名单只放行**可证明只读**的工具；
//! 3. 有副作用的工具（写/执行/联网）一律不放行。
//!
//! 白名单不是手写清单：它从守卫的能力安全注册表**派生**
//! （`is_provably_read_only`），所以这里的断言同时在守卫登记正确性。

use apeireth_orchestration::call_scheduler::ConcurrencySafety as _;

#[tokio::test]
async fn production_runtime_installs_a_conservative_parallel_scheduler() {
    // 生产装配（与 CLI 同一入口；keyless 也能构建，不触网）。
    let runtime = apeireth_cli::build_canonical_runtime_from_env()
        .await
        .expect("keyless production runtime builds without network");

    let scheduler = runtime
        .call_scheduler()
        .expect("生产装配必须装上并发调度器，否则并行派发永远不发生");

    // ① 只读工具应被放行（白名单从守卫注册表派生）
    for capability in [
        "tool.repo",
        "tool.repo_map",
        "tool.self_status",
        "tool.trait_report",
        "tool.search",
    ] {
        assert!(
            scheduler.is_concurrency_safe(capability, &serde_json::json!({})),
            "{capability} 是可证明只读，应允许并行"
        );
    }

    // ② 有副作用 / 未登记的一律不放行（未知不等于安全）
    for capability in [
        "tool.apply_patch",
        "tool.shell",
        "tool.git_write",
        "tool.fetch",
        "tool.never-registered",
    ] {
        assert!(
            !scheduler.is_concurrency_safe(capability, &serde_json::json!({})),
            "{capability} 绝不能进并发白名单"
        );
    }
}
