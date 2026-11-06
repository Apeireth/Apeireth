//! 并行工具派发的装配（`RuntimeBuilder::with_call_scheduler`）。
//!
//! # 为什么需要这一行
//!
//! 运行时**完整支持**并行派发（`plan_parallel_window` 找窗口、
//! `dispatch_parallel_window` 滚动池执行、结果按模型顺序提交），但整条链挂在
//! `with_call_scheduler` 上 —— 生产装配从不构造 `CallScheduler`
//! ⇒ 窗口恒为 1 ⇒ **并行派发在生产形同不存在**。
//!
//! # 白名单从哪来（不拍脑袋）
//!
//! 从**守卫的能力安全注册表**派生：只有
//! `CapabilitySafetyRegistry::is_provably_read_only` 判为真的能力才允许并行
//! （无持久化效果、无破坏性、无外部副作用、不联网、运行类只含 Read/Compute，
//! 且**未知能力一律不放行**）。推导逻辑与判据在
//! [`apeireth_runtime_assembly::canonical::parallel_dispatch`]，
//! 本模块只做装配根的一行调用。
//!
//! # 行为影响
//!
//! 只有**连续 ≥2 个**调用都命中白名单才走并行窗口；单调用或任一调用不在
//! 白名单 ⇒ 原串行路径不变。结果顺序按模型顺序提交，transcript 顺序不动。

use apeireth_runtime_assembly::canonical::parallel_dispatch::production_call_scheduler;
use apeireth_runtime_assembly::canonical::runtime::RuntimeBuilder;

/// 装上并行派发调度器（白名单为空时不装 —— 与接线前逐字节一致）。
pub fn attach(builder: RuntimeBuilder) -> RuntimeBuilder {
    match production_call_scheduler() {
        Some(scheduler) => builder.with_call_scheduler(scheduler),
        None => builder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_orchestration::call_scheduler::ConcurrencySafety as _;

    #[test]
    fn production_derivation_yields_a_non_empty_whitelist() {
        // 守卫注册表里已登记只读工具（tool.repo / repo_map / self_status /
        // trait_report / search）⇒ 生产应能派生出非空白名单并装上调度器。
        let scheduler = production_call_scheduler().expect("只读能力应能派生白名单");
        assert!(scheduler.is_concurrency_safe("tool.repo", &serde_json::json!({})));
        assert!(!scheduler.is_concurrency_safe("tool.apply_patch", &serde_json::json!({})));
    }

    #[test]
    fn attach_is_a_no_op_signature_wise() {
        // 装配函数的契约：拿 builder 还 builder（装上调度器不改变构建语义）。
        let _ = attach as fn(RuntimeBuilder) -> RuntimeBuilder;
    }
}
