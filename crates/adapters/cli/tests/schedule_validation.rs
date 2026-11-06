//! C2 接线验收: `validate_schedule`（**调度要不要校验 = 正确性语义**）在生产调度
//! 入口真的起作用。
//!
//! # 行为差异（接线前不可用 / 接线后可用）
//!
//! `apeireth nightwatch --watch` 的**轮询调度**（具名间隔调度）:
//!
//! - **接线前**: 轮询间隔走 `interval_secs.max(1)` ⇒ **0 秒被静默改写成 1 秒**
//!   ⇒ 用户要的那个调度（0 秒）**没有被跑**，跑的是另一个 ⇒ 「未校验的调度会跑错」。
//! - **接线后**: 调度**真正落地前**过一遍规范校验
//!   （`apeireth_orchestration::cron::validate_schedule`）⇒ 间隔非正 ⇒ **拒绝即帧**
//!   （如实报错，不静默改写；且在开库 / 进循环之前 ⇒ 不留半启动状态）。

use apeireth_cli::nightwatch_command::dispatch_nightwatch_watch;

/// **接线后不可用**: 0 秒轮询间隔在生产调度入口被拒绝（即帧）。
#[tokio::test]
async fn a_zero_poll_interval_is_refused_before_the_schedule_lands() {
    let error = dispatch_nightwatch_watch(None, 1, 900, 3600, 0)
        .await
        .expect_err("0 秒轮询间隔必须被拒绝（旧口径是静默改成 1 秒）");
    assert!(
        error.contains("拒绝调度"),
        "拒绝理由必须说清是调度被拒: {error}"
    );
    assert!(
        error.contains("interval") || error.contains("> 0"),
        "拒绝理由必须来自规范校验（间隔必须为正）: {error}"
    );
}

/// **接线后可用**: 合法间隔照常落地（校验不误伤正常调度）。
#[test]
fn the_canonical_schedule_validation_accepts_a_positive_interval() {
    let schedule = apeireth_orchestration::cron::Schedule::new("nightwatch_watch_poll", 60);
    assert!(
        apeireth_orchestration::cron::validate_schedule(&schedule).is_ok(),
        "正间隔必须通过规范校验"
    );
    assert!(
        apeireth_orchestration::cron::validate_schedule(
            &apeireth_orchestration::cron::Schedule::new("nightwatch_watch_poll", 0)
        )
        .is_err(),
        "0 间隔必须过不了规范校验（旧口径静默改成 1 秒 ⇒ 跑错调度）"
    );
}
