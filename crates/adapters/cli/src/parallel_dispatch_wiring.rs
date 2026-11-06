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

use std::sync::{Arc, OnceLock};

use apeireth_orchestration::call_scheduler::CallScheduler;
use apeireth_orchestration::job_board::{BoardConfig, JobBoard};
use apeireth_runtime_assembly::canonical::parallel_dispatch::production_call_scheduler;
use apeireth_runtime_assembly::canonical::runtime::RuntimeBuilder;

/// **生产任务板**（并行派发批次的长任务身份 + 终局释放）。
///
/// # 为什么生产要有这块板
///
/// 并行派发的一批调用就是一次**长任务**：多段、并发、部分先完成、可能整批被取消。
/// 板上给这一批一个**身份 + 归属方 + 逐调用进展**，并在**取消 / 全部交付**时
/// 结算并**释放**记录 ⇒ ⇒ 板上不留死记录，且「这批跑到哪儿了」可查。
///
/// **进程内单例**（与 `governance_chain::shared_audit_sink` 同口径）：同一进程里
/// 装配多个 runtime 时共用一块板 ⇒ 批次记录在同一本账上。
pub fn production_job_board() -> Arc<JobBoard> {
    static BOARD: OnceLock<Arc<JobBoard>> = OnceLock::new();
    Arc::clone(BOARD.get_or_init(|| Arc::new(JobBoard::new(BoardConfig::default()))))
}

/// **生产并行派发调度器**（白名单派生 + 任务板接线）。
///
/// 白名单为空 ⇒ `None`（不装调度器 ⇒ 与接线前逐字节一致）；否则调度器**装上
/// 生产任务板** ⇒ 每一批并行派发在板上登记为长任务 ⇒ 取消 / 全部交付时结算并
/// 释放记录（板上不留死记录）。
pub fn production_scheduler() -> Option<Arc<CallScheduler>> {
    production_call_scheduler().map(|scheduler| {
        Arc::new(CallScheduler::with_job_board(
            (*scheduler).clone(),
            production_job_board(),
        ))
    })
}

/// 装上并行派发调度器（白名单为空时不装 —— 与接线前逐字节一致）。
///
/// **任务板接线**：调度器装上板 ⇒ 每一批并行派发在板上登记为长任务 ⇒
/// 取消 / 全部交付时结算并释放记录（板上不留死记录）。
pub fn attach(builder: RuntimeBuilder) -> RuntimeBuilder {
    match production_scheduler() {
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
