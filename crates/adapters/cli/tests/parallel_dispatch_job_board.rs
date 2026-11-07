//! C3 接线验收: `remove_job`（**任务板整族生产无入口**）在并行工具派发的生产
//! 批次生命周期里真的被调用。
//!
//! # 行为差异（接线前不可用 / 接线后可用）
//!
//! - **接线前**: 生产装配从不构造 `JobBoard` ⇒ 并行派发的一批调用**在板上没有
//!   任何记录** ⇒ 没有身份、没有归属方、没有进展，**也没有释放路径**（"板上
//!   不留死记录"这件事根本不存在，因为板上什么都没有）。
//! - **接线后**: 每一批并行派发在板上登记为一个**长任务**（身份 + 归属方 +
//!   逐调用进展）⇒ 批次**取消**或**全部交付**时结算并**释放**记录
//!   ⇒ ⇒ 板上不留死记录（`remove_job` 的真实生产调用点）。
//!
//! # 为什么这个消费点是真的
//!
//! 并行派发的一批调用**就是一次长任务**：多段、并发、部分先完成、可能整批被取消
//! ⇒ 板上给这一批一个身份与归属方才有「这批跑到哪儿了」的落点；而取消路径
//! 必须有释放口，否则板上会攒下一堆死记录。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use apeireth_cli::parallel_dispatch_wiring::{production_job_board, production_scheduler};
use apeireth_orchestration::call_scheduler::{
    CallRunner, CallScheduler, SafetyWhitelist, ScheduledCall, SchedulerConfig,
};
use apeireth_orchestration::job_board::JobStatus;
use serde_json::Value;

/// 一批只读调用（生产并行窗口里的典型形态）。
fn batch_calls() -> Vec<ScheduledCall> {
    vec![
        ScheduledCall::new("call-1", "tool.repo", Value::Null),
        ScheduledCall::new("call-2", "tool.repo_map", Value::Null),
    ]
}

/// 立即完成的执行体。
fn runner() -> Arc<dyn CallRunner<usize>> {
    Arc::new(
        |index: usize, _call: &ScheduledCall| -> Pin<Box<dyn Future<Output = usize> + Send>> {
            Box::pin(async move { index })
        },
    )
}

/// **接线后**: 生产调度器**装上了任务板**。
#[test]
fn the_production_scheduler_carries_the_task_board() {
    let scheduler = production_scheduler().expect("只读白名单非空 ⇒ 生产调度器应存在");
    assert!(
        scheduler.job_board().is_some(),
        "生产调度器必须装上任务板（并行派发批次要有身份与释放口）"
    );
}

/// **接线后不可用 / 接线后可用**: 批次在飞时板上**有**记录 ⇒ 取消后记录**被释放**。
#[tokio::test]
async fn a_cancelled_parallel_batch_releases_its_task_board_record() {
    let scheduler = production_scheduler().expect("生产调度器应存在");
    let board = production_job_board();
    let mut batch = scheduler.start_batch(batch_calls(), runner());

    let job_id = batch
        .board_job_id()
        .expect("装了任务板 ⇒ 批次必须在板上登记");
    assert_eq!(
        board.status(job_id).expect("批次在飞时板上必须有记录"),
        JobStatus::Running,
        "批次在飞 ⇒ 板上记录是运行态"
    );

    batch.abort();

    assert!(
        board.status(job_id).is_err(),
        "取消后必须释放板上记录（remove_job 的真实调用点 ⇒ 板上不留死记录）"
    );
}

/// **全部交付后同样释放**（正常收尾路径也不能攒死记录）。
#[tokio::test]
async fn a_fully_delivered_parallel_batch_releases_its_task_board_record() {
    let scheduler = production_scheduler().expect("生产调度器应存在");
    let board = production_job_board();
    let mut batch = scheduler.start_batch(batch_calls(), runner());
    let job_id = batch
        .board_job_id()
        .expect("装了任务板 ⇒ 批次必须在板上登记");

    while batch.next_outcome().await.is_some() {}

    assert!(board.status(job_id).is_err(), "全部交付后必须释放板上记录");
}

/// **接线前的形状**: 不装板 ⇒ 批次在板上**根本没有记录**（连身份都没有）。
#[tokio::test]
async fn without_the_task_board_the_parallel_batch_leaves_no_record() {
    let scheduler =
        CallScheduler::new(Arc::new(SafetyWhitelist::new()), SchedulerConfig::default())
            .expect("调度器装配");
    let mut batch = scheduler.start_batch(batch_calls(), runner());
    assert!(
        batch.board_job_id().is_none(),
        "未装任务板 ⇒ 批次在板上没有记录（= 接线前的生产形状）"
    );
}
