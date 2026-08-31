//! `durable::replay` — 确定性重放引擎 (真正跳过已完成活动的副作用).
//!
//! **语义来源**: canonical/apeireth-workflow 声称 "同一 input + 相同 history → 相同 output"
//! 的 replay, 但 canonical `WorkflowRunner::run` 每次都新建空 history 并重新执行全部
//! Activity 副作用 (`r152_workflow_deliverables` 只验证了确定性函数本身)。本模块
//! **补上 canonical 未实现的真重放**:
//!
//! - 已记录 `ActivityScheduled` + `ActivityCompleted` → 返回记录的 output, **零副作用**;
//! - 已记录尝试链 `Scheduled → Failed … → Completed` → 消费失败记录后返回成功;
//! - `Scheduled` 之后无结局 (两次 journal 写之间崩溃) → at-least-once 重执行该次尝试;
//! - `Scheduled` 的 activity_id / input 与 step 请求不一致 → fail-closed [`DurableError::ReplayMismatch`];
//! - 新鲜活动经注入的 [`ActivityExecutor`] 执行并追加事件。
//!
//! **确定性方法**: 注入时间戳 + JSON 输入相等匹配 (v2 step 函数确定性, 无 rng)。
//! 不采用 canonical supervisor 的 `host_pid` / `rng_seed` DeterminismMeta。
//!
//! **不是** Main Loop / WorkflowRunner 注册表 / daemon tick: 调用方自带 step 闭包
//! 与 executor, 本类型只维护一条 journal 游标。

use serde_json::Value;

use super::history::{ActivityEvent, ActivityEventKind, DurableHistory, RUN_EVENT_ACTOR};
use super::retry::RetryPolicy;
use super::{DurableError, DurableResult};

/// 副作用执行器 (canonical `Activity` trait 的注入形态, 无全局注册表).
pub trait ActivityExecutor {
    /// 执行一次活动。`Err(String)` 视为该次尝试失败, 由 [`DurableRun`] 按 [`RetryPolicy`] 记账。
    fn execute(&self, activity_id: &str, input: &Value) -> Result<Value, String>;
}

impl<F> ActivityExecutor for F
where
    F: Fn(&str, &Value) -> Result<Value, String>,
{
    fn execute(&self, activity_id: &str, input: &Value) -> Result<Value, String> {
        self(activity_id, input)
    }
}

/// 一次持久 run: 持有追加式 journal + 重放游标。
///
/// `start` 从空日志开始; `resume` 从已有日志重放。游标只向前, 重放命中的活动
/// 不调用 executor; 游标追上日志末尾后的请求走新鲜执行并追加。
pub struct DurableRun {
    history: DurableHistory,
    /// 下一条待消费的事件下标 (0-based)。`start` 后位于 `RunStarted` 之后。
    cursor: usize,
    finished: bool,
}

impl DurableRun {
    /// 开始一次新 run, 写入 `RunStarted`。
    pub fn start(input: Value, now_ms: i64) -> Self {
        let mut history = DurableHistory::new();
        history.record(
            ActivityEventKind::RunStarted,
            RUN_EVENT_ACTOR,
            Some(input),
            None,
            None,
            0,
            now_ms,
        );
        Self {
            history,
            cursor: 1,
            finished: false,
        }
    }

    /// 从已有 journal 恢复。必须以 `run_started` 开头; 损坏则 fail-closed。
    ///
    /// 即使日志以 `run_completed` / `run_failed` 结尾, 本实例仍允许 step 函数
    /// 重放活动 (游标消费终态事件); 不把 journal 终态直接当成 "本实例已 finish",
    /// 否则重放路径会立刻 `RunAlreadyFinished`。
    pub fn resume(history: DurableHistory) -> DurableResult<Self> {
        if history.is_empty() {
            return Err(DurableError::HistoryCorrupted {
                reason: "empty history cannot be resumed".into(),
            });
        }
        let first = &history.events()[0];
        if first.kind != ActivityEventKind::RunStarted || first.seq != 1 {
            return Err(DurableError::HistoryCorrupted {
                reason: format!(
                    "history must begin with run_started seq=1, got kind={} seq={}",
                    first.kind.as_str(),
                    first.seq
                ),
            });
        }
        Ok(Self {
            history,
            cursor: 1,
            finished: false,
        })
    }

    /// 当前 journal (调用方可 serde / JSONL 持久化, 本模块不写盘)。
    pub fn history(&self) -> &DurableHistory {
        &self.history
    }

    /// 取出 journal (结束一次调用后交给存储)。
    pub fn into_history(self) -> DurableHistory {
        self.history
    }

    /// step 是否已调用 [`complete`](Self::complete) / [`fail`](Self::fail)。
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// 执行 (或重放) 一次逻辑活动。
    ///
    /// 一次逻辑调用对应 journal 上的一条尝试链:
    /// `(Scheduled → Failed)* → Scheduled → Completed` 或耗尽后的最终 `Failed`。
    /// 重试循环**不 sleep**; [`RetryPolicy::backoff_ms`] 只供调用方自行等待。
    pub fn execute_activity(
        &mut self,
        activity_id: &str,
        input: Value,
        executor: &dyn ActivityExecutor,
        policy: RetryPolicy,
        now_ms: i64,
    ) -> DurableResult<Value> {
        if self.finished {
            return Err(DurableError::RunAlreadyFinished);
        }
        if self.cursor < self.history.len() {
            self.replay_or_recover(activity_id, input, executor, policy, now_ms)
        } else {
            self.execute_fresh(activity_id, input, executor, policy, now_ms, 1)
        }
    }

    /// 标记 run 成功结束。重放命中已记录的 `RunCompleted` 时只推进游标。
    pub fn complete(&mut self, output: Value, now_ms: i64) -> DurableResult<()> {
        if self.finished {
            return Err(DurableError::RunAlreadyFinished);
        }
        if self.cursor < self.history.len() {
            let ev = &self.history.events()[self.cursor];
            match ev.kind {
                ActivityEventKind::RunCompleted => {
                    self.cursor += 1;
                    self.finished = true;
                    Ok(())
                }
                ActivityEventKind::RunFailed => Err(DurableError::ReplayMismatch {
                    activity_id: RUN_EVENT_ACTOR.into(),
                    expected_input: "run_completed".into(),
                    found: format!("kind={}", ev.kind.as_str()),
                }),
                _ => Err(DurableError::ReplayMismatch {
                    activity_id: RUN_EVENT_ACTOR.into(),
                    expected_input: "run_completed".into(),
                    found: Self::event_summary(ev),
                }),
            }
        } else {
            self.history.record(
                ActivityEventKind::RunCompleted,
                RUN_EVENT_ACTOR,
                None,
                Some(output),
                None,
                0,
                now_ms,
            );
            self.cursor = self.history.len();
            self.finished = true;
            Ok(())
        }
    }

    /// 标记 run 失败结束。重放命中已记录的 `RunFailed` 时只推进游标。
    pub fn fail(&mut self, error: impl Into<String>, now_ms: i64) -> DurableResult<()> {
        if self.finished {
            return Err(DurableError::RunAlreadyFinished);
        }
        let error = error.into();
        if self.cursor < self.history.len() {
            let ev = &self.history.events()[self.cursor];
            match ev.kind {
                ActivityEventKind::RunFailed => {
                    self.cursor += 1;
                    self.finished = true;
                    Ok(())
                }
                ActivityEventKind::RunCompleted => Err(DurableError::ReplayMismatch {
                    activity_id: RUN_EVENT_ACTOR.into(),
                    expected_input: "run_failed".into(),
                    found: format!("kind={}", ev.kind.as_str()),
                }),
                _ => Err(DurableError::ReplayMismatch {
                    activity_id: RUN_EVENT_ACTOR.into(),
                    expected_input: "run_failed".into(),
                    found: Self::event_summary(ev),
                }),
            }
        } else {
            self.history.record(
                ActivityEventKind::RunFailed,
                RUN_EVENT_ACTOR,
                None,
                None,
                Some(error),
                0,
                now_ms,
            );
            self.cursor = self.history.len();
            self.finished = true;
            Ok(())
        }
    }

    fn replay_or_recover(
        &mut self,
        activity_id: &str,
        input: Value,
        executor: &dyn ActivityExecutor,
        policy: RetryPolicy,
        now_ms: i64,
    ) -> DurableResult<Value> {
        let scheduled = self.peek().ok_or_else(|| DurableError::HistoryCorrupted {
            reason: "replay cursor past end".into(),
        })?;
        match scheduled.kind {
            ActivityEventKind::RunCompleted | ActivityEventKind::RunFailed => {
                return Err(DurableError::ReplayMismatch {
                    activity_id: activity_id.into(),
                    expected_input: Self::value_preview(&input),
                    found: format!("kind={}", scheduled.kind.as_str()),
                });
            }
            ActivityEventKind::ActivityScheduled => {}
            other => {
                return Err(DurableError::HistoryCorrupted {
                    reason: format!(
                        "replay expected activity_scheduled, found {}",
                        other.as_str()
                    ),
                });
            }
        }
        if scheduled.activity_id != activity_id || scheduled.input.as_ref() != Some(&input) {
            return Err(DurableError::ReplayMismatch {
                activity_id: activity_id.into(),
                expected_input: Self::value_preview(&input),
                found: Self::event_summary(scheduled),
            });
        }

        loop {
            let (sched_id, sched_attempt) = {
                let ev = self.peek().ok_or_else(|| DurableError::HistoryCorrupted {
                    reason: "attempt chain truncated before scheduled".into(),
                })?;
                if ev.kind != ActivityEventKind::ActivityScheduled {
                    return Err(DurableError::HistoryCorrupted {
                        reason: format!(
                            "attempt chain expected activity_scheduled, found {}",
                            ev.kind.as_str()
                        ),
                    });
                }
                (ev.activity_id.clone(), ev.attempt)
            };
            if sched_id != activity_id {
                return Err(DurableError::HistoryCorrupted {
                    reason: format!("attempt chain jumped from {activity_id:?} to {sched_id:?}"),
                });
            }
            self.cursor += 1;

            let Some(outcome) = self.peek().cloned() else {
                // 崩溃在 Scheduled 与结局之间: at-least-once 重执行该次尝试。
                return self.recover_attempt(
                    activity_id,
                    input,
                    executor,
                    policy,
                    now_ms,
                    sched_attempt,
                );
            };

            if outcome.activity_id != activity_id {
                return Err(DurableError::HistoryCorrupted {
                    reason: format!(
                        "outcome activity_id {:?} != scheduled {activity_id:?}",
                        outcome.activity_id
                    ),
                });
            }

            match outcome.kind {
                ActivityEventKind::ActivityCompleted => {
                    self.cursor += 1;
                    return Ok(outcome.output.unwrap_or(Value::Null));
                }
                ActivityEventKind::ActivityFailed => {
                    self.cursor += 1;
                    let failed_error = outcome.error.clone().unwrap_or_default();
                    let failed_attempt = outcome.attempt;
                    if let Some(next) = self.peek() {
                        if next.kind == ActivityEventKind::ActivityScheduled
                            && next.activity_id == activity_id
                            && next.input.as_ref() == Some(&input)
                        {
                            continue;
                        }
                        // 日志里这次逻辑调用以失败结束 (调用方没有继续重试)。
                        return Err(DurableError::ActivityFailed {
                            activity_id: activity_id.into(),
                            error: failed_error,
                            attempt: failed_attempt,
                        });
                    }
                    // 崩溃在 Failed 之后、下一次 Scheduled 之前。
                    if policy.should_retry(failed_attempt) {
                        return self.execute_fresh(
                            activity_id,
                            input,
                            executor,
                            policy,
                            now_ms,
                            failed_attempt.saturating_add(1),
                        );
                    }
                    return Err(DurableError::ActivityFailed {
                        activity_id: activity_id.into(),
                        error: failed_error,
                        attempt: failed_attempt,
                    });
                }
                other => {
                    return Err(DurableError::HistoryCorrupted {
                        reason: format!(
                            "expected activity_completed or activity_failed after scheduled, found {}",
                            other.as_str()
                        ),
                    });
                }
            }
        }
    }

    fn recover_attempt(
        &mut self,
        activity_id: &str,
        input: Value,
        executor: &dyn ActivityExecutor,
        policy: RetryPolicy,
        now_ms: i64,
        attempt: u32,
    ) -> DurableResult<Value> {
        match executor.execute(activity_id, &input) {
            Ok(output) => {
                self.record_completed(activity_id, output.clone(), attempt, now_ms);
                Ok(output)
            }
            Err(error) => {
                self.record_failed(activity_id, error.clone(), attempt, now_ms);
                if policy.should_retry(attempt) {
                    self.execute_fresh(
                        activity_id,
                        input,
                        executor,
                        policy,
                        now_ms,
                        attempt.saturating_add(1),
                    )
                } else {
                    Err(DurableError::ActivityFailed {
                        activity_id: activity_id.into(),
                        error,
                        attempt,
                    })
                }
            }
        }
    }

    fn execute_fresh(
        &mut self,
        activity_id: &str,
        input: Value,
        executor: &dyn ActivityExecutor,
        policy: RetryPolicy,
        now_ms: i64,
        start_attempt: u32,
    ) -> DurableResult<Value> {
        let mut attempt = start_attempt.max(1);
        loop {
            self.history.record(
                ActivityEventKind::ActivityScheduled,
                activity_id,
                Some(input.clone()),
                None,
                None,
                attempt,
                now_ms,
            );
            self.cursor = self.history.len();
            match executor.execute(activity_id, &input) {
                Ok(output) => {
                    self.record_completed(activity_id, output.clone(), attempt, now_ms);
                    return Ok(output);
                }
                Err(error) => {
                    self.record_failed(activity_id, error.clone(), attempt, now_ms);
                    if policy.should_retry(attempt) {
                        attempt = attempt.saturating_add(1);
                        continue;
                    }
                    return Err(DurableError::ActivityFailed {
                        activity_id: activity_id.into(),
                        error,
                        attempt,
                    });
                }
            }
        }
    }

    fn record_completed(&mut self, activity_id: &str, output: Value, attempt: u32, now_ms: i64) {
        self.history.record(
            ActivityEventKind::ActivityCompleted,
            activity_id,
            None,
            Some(output),
            None,
            attempt,
            now_ms,
        );
        self.cursor = self.history.len();
    }

    fn record_failed(&mut self, activity_id: &str, error: String, attempt: u32, now_ms: i64) {
        self.history.record(
            ActivityEventKind::ActivityFailed,
            activity_id,
            None,
            None,
            Some(error),
            attempt,
            now_ms,
        );
        self.cursor = self.history.len();
    }

    fn peek(&self) -> Option<&ActivityEvent> {
        self.history.events().get(self.cursor)
    }

    fn event_summary(ev: &ActivityEvent) -> String {
        format!(
            "kind={} activity_id={:?} input={}",
            ev.kind.as_str(),
            ev.activity_id,
            ev.input
                .as_ref()
                .map(Self::value_preview)
                .unwrap_or_else(|| "null".into())
        )
    }

    fn value_preview(value: &Value) -> String {
        let s = value.to_string();
        if s.len() <= 128 {
            s
        } else {
            format!("{}…", &s[..128])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct CountingEcho {
        calls: AtomicU32,
    }

    impl CountingEcho {
        fn new() -> Self {
            Self {
                calls: AtomicU32::new(0),
            }
        }
        fn hits(&self) -> u32 {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl ActivityExecutor for CountingEcho {
        fn execute(&self, _activity_id: &str, input: &Value) -> Result<Value, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(input.clone())
        }
    }

    struct FailThenSucceed {
        calls: AtomicU32,
        fail_times: u32,
    }

    impl FailThenSucceed {
        fn new(fail_times: u32) -> Self {
            Self {
                calls: AtomicU32::new(0),
                fail_times,
            }
        }
        fn hits(&self) -> u32 {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl ActivityExecutor for FailThenSucceed {
        fn execute(&self, _activity_id: &str, input: &Value) -> Result<Value, String> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            if n <= self.fail_times {
                Err(format!("transient-{n}"))
            } else {
                Ok(input.clone())
            }
        }
    }

    struct AlwaysFail;

    impl ActivityExecutor for AlwaysFail {
        fn execute(&self, _activity_id: &str, _input: &Value) -> Result<Value, String> {
            Err("intentional failure".into())
        }
    }

    fn no_retry() -> RetryPolicy {
        RetryPolicy::new(1, 1_000, 1_000)
    }

    fn policy_3() -> RetryPolicy {
        RetryPolicy::default()
    }

    fn kinds_of(h: &DurableHistory) -> Vec<ActivityEventKind> {
        h.events().iter().map(|e| e.kind).collect()
    }

    /// canonical `runner_records_event_history` / R225 end-to-end: 事件顺序。
    #[test]
    fn fresh_run_records_started_scheduled_completed() {
        let exec = CountingEcho::new();
        let mut run = DurableRun::start(json!({"k": 1}), 1000);
        let a = run
            .execute_activity("echo", json!(1), &exec, no_retry(), 1001)
            .unwrap();
        let b = run
            .execute_activity("echo", json!(2), &exec, no_retry(), 1002)
            .unwrap();
        run.complete(
            json!({"sum": a.as_i64().unwrap() + b.as_i64().unwrap()}),
            1003,
        )
        .unwrap();
        assert_eq!(exec.hits(), 2);
        assert_eq!(
            kinds_of(run.history()),
            vec![
                ActivityEventKind::RunStarted,
                ActivityEventKind::ActivityScheduled,
                ActivityEventKind::ActivityCompleted,
                ActivityEventKind::ActivityScheduled,
                ActivityEventKind::ActivityCompleted,
                ActivityEventKind::RunCompleted,
            ]
        );
        for (i, e) in run.history().events().iter().enumerate() {
            assert_eq!(e.seq, (i + 1) as u64);
        }
    }

    /// canonical `test_workflow_propagates_activity_failure` + `runner_handles_activity_failure`。
    #[test]
    fn activity_failure_is_recorded_and_propagated() {
        let mut run = DurableRun::start(json!(null), 1);
        let err = run
            .execute_activity("failing", json!(null), &AlwaysFail, no_retry(), 2)
            .unwrap_err();
        assert!(matches!(
            err,
            DurableError::ActivityFailed {
                attempt: 1,
                ref activity_id,
                ..
            } if activity_id == "failing"
        ));
        run.fail(err.to_string(), 3).unwrap();
        let failed = run
            .history()
            .filter_kind(ActivityEventKind::ActivityFailed)
            .count();
        assert_eq!(failed, 1);
        assert_eq!(
            run.history().events().last().unwrap().kind,
            ActivityEventKind::RunFailed
        );
    }

    /// canonical `r152_workflow_deliverables`: 同 input → 同 output; **并且**第二次
    /// 走 history 重放, 计数器不增加 (这是 canonical 只声称未实现的真重放)。
    #[test]
    fn replay_skips_completed_activities() {
        let exec = CountingEcho::new();
        let mut run = DurableRun::start(json!({}), 10);
        let r1 = run
            .execute_activity("echo", json!(1), &exec, no_retry(), 11)
            .unwrap();
        let r2 = run
            .execute_activity("echo", json!(2), &exec, no_retry(), 12)
            .unwrap();
        run.complete(json!({"sum": 3}), 13).unwrap();
        assert_eq!(exec.hits(), 2);
        let history = run.into_history();

        let mut replayed = DurableRun::resume(history.clone()).unwrap();
        let p1 = replayed
            .execute_activity("echo", json!(1), &exec, no_retry(), 21)
            .unwrap();
        let p2 = replayed
            .execute_activity("echo", json!(2), &exec, no_retry(), 22)
            .unwrap();
        replayed.complete(json!({"sum": 3}), 23).unwrap();
        assert_eq!(p1, r1);
        assert_eq!(p2, r2);
        assert_eq!(exec.hits(), 2, "completed activities must not re-execute");
        assert_eq!(
            replayed.history(),
            &history,
            "replay must not extend a finished journal"
        );
    }

    /// 尝试链 Scheduled→Failed→Scheduled→Completed 重放时零副作用返回成功。
    #[test]
    fn replay_consumes_recorded_retry_chain() {
        let exec = FailThenSucceed::new(2);
        let mut run = DurableRun::start(json!({}), 1);
        let out = run
            .execute_activity("flaky", json!("ok"), &exec, policy_3(), 2)
            .unwrap();
        assert_eq!(out, json!("ok"));
        assert_eq!(exec.hits(), 3);
        run.complete(out.clone(), 3).unwrap();
        let history = run.into_history();
        let attempts: Vec<u32> = history
            .filter_kind(ActivityEventKind::ActivityScheduled)
            .map(|e| e.attempt)
            .collect();
        assert_eq!(attempts, vec![1, 2, 3]);

        let exec2 = FailThenSucceed::new(99); // 若重放误执行会立刻失败
        let mut replayed = DurableRun::resume(history).unwrap();
        let again = replayed
            .execute_activity("flaky", json!("ok"), &exec2, policy_3(), 9)
            .unwrap();
        assert_eq!(again, json!("ok"));
        assert_eq!(exec2.hits(), 0, "retry chain replay must skip executor");
    }

    /// Scheduled 无结局 → at-least-once 重执行该次尝试, 然后续写同一 journal。
    #[test]
    fn crash_after_scheduled_reexecutes_at_least_once() {
        let exec = CountingEcho::new();
        let mut run = DurableRun::start(json!({}), 1);
        run.execute_activity("echo", json!(7), &exec, no_retry(), 2)
            .unwrap();
        let jsonl = run.history().to_jsonl();
        // 截到 RunStarted + ActivityScheduled (崩溃在完成事件写出前)
        let truncated: String = jsonl.lines().take(2).collect::<Vec<_>>().join("\n");
        let history = DurableHistory::from_jsonl(&truncated).unwrap();
        assert_eq!(history.len(), 2);

        let mut resumed = DurableRun::resume(history).unwrap();
        let out = resumed
            .execute_activity("echo", json!(7), &exec, no_retry(), 3)
            .unwrap();
        assert_eq!(out, json!(7));
        assert_eq!(exec.hits(), 2, "first complete + crash recovery");
        assert_eq!(
            kinds_of(resumed.history()),
            vec![
                ActivityEventKind::RunStarted,
                ActivityEventKind::ActivityScheduled,
                ActivityEventKind::ActivityCompleted,
            ]
        );
        resumed.complete(out, 4).unwrap();

        // 第二次崩溃/恢复: 续写后的 journal 被统一重放, 不再执行。
        let extended = resumed.into_history();
        let mut second = DurableRun::resume(extended).unwrap();
        let again = second
            .execute_activity("echo", json!(7), &exec, no_retry(), 5)
            .unwrap();
        assert_eq!(again, json!(7));
        assert_eq!(
            exec.hits(),
            2,
            "second resume must skip recovered completion"
        );
    }

    /// Failed 后、下一次 Scheduled 前崩溃: 按政策继续新鲜重试并追加。
    #[test]
    fn crash_after_failed_continues_retry_budget() {
        let exec = FailThenSucceed::new(2);
        let mut run = DurableRun::start(json!({}), 1);
        let _ = run.execute_activity("flaky", json!(1), &exec, policy_3(), 2);
        // 截到第一次失败 (RunStarted, Scheduled, Failed)
        let jsonl = run.history().to_jsonl();
        let truncated: String = jsonl.lines().take(3).collect::<Vec<_>>().join("\n");
        let history = DurableHistory::from_jsonl(&truncated).unwrap();

        let exec2 = FailThenSucceed::new(1); // 下一次失败一次再成功; 但 recover 从 attempt 2 起
                                             // resume 看到 Failed attempt=1, should_retry → execute_fresh attempt=2
                                             // exec2: 第一次调用 (attempt 2) 失败, 第二次成功。
        let mut resumed = DurableRun::resume(history).unwrap();
        let out = resumed
            .execute_activity("flaky", json!(1), &exec2, policy_3(), 9)
            .unwrap();
        assert_eq!(out, json!(1));
        assert_eq!(exec2.hits(), 2);
        let scheduled_attempts: Vec<u32> = resumed
            .history()
            .filter_kind(ActivityEventKind::ActivityScheduled)
            .map(|e| e.attempt)
            .collect();
        assert_eq!(scheduled_attempts, vec![1, 2, 3]);
    }

    /// activity_id / input 不一致 → ReplayMismatch, 不执行副作用。
    #[test]
    fn mismatched_schedule_is_fail_closed() {
        let exec = CountingEcho::new();
        let mut run = DurableRun::start(json!({}), 1);
        run.execute_activity("echo", json!(1), &exec, no_retry(), 2)
            .unwrap();
        let history = run.into_history();
        let before = exec.hits();

        let mut replayed = DurableRun::resume(history).unwrap();
        let err = replayed
            .execute_activity("echo", json!(99), &exec, no_retry(), 3)
            .unwrap_err();
        assert!(matches!(err, DurableError::ReplayMismatch { .. }));
        assert_eq!(exec.hits(), before);

        let mut replayed_id = DurableRun::resume(replayed.into_history()).unwrap();
        let err = replayed_id
            .execute_activity("other", json!(1), &exec, no_retry(), 4)
            .unwrap_err();
        assert!(matches!(err, DurableError::ReplayMismatch { .. }));
        assert_eq!(exec.hits(), before);
    }

    /// JSONL 往返后仍可 resume 重放 (continuation 存储路径证明, 未接线)。
    #[test]
    fn jsonl_round_trip_resume_replays() {
        let exec = CountingEcho::new();
        let mut run = DurableRun::start(json!({"k": 1}), 1);
        run.execute_activity("echo", json!("x"), &exec, no_retry(), 2)
            .unwrap();
        run.complete(json!("x"), 3).unwrap();
        let jsonl = run.history().to_jsonl();
        let restored = DurableHistory::from_jsonl(&jsonl).unwrap();
        assert_eq!(&restored, run.history());

        let mut replayed = DurableRun::resume(restored).unwrap();
        let out = replayed
            .execute_activity("echo", json!("x"), &exec, no_retry(), 9)
            .unwrap();
        replayed.complete(json!("x"), 10).unwrap();
        assert_eq!(out, json!("x"));
        assert_eq!(exec.hits(), 1);
    }

    /// 重试预算耗尽: 三次尝试均失败, 不第四次。
    #[test]
    fn retry_budget_exhausted_stops() {
        let mut run = DurableRun::start(json!({}), 1);
        let err = run
            .execute_activity("failing", json!(null), &AlwaysFail, policy_3(), 2)
            .unwrap_err();
        assert!(matches!(
            err,
            DurableError::ActivityFailed { attempt: 3, .. }
        ));
        assert_eq!(
            run.history()
                .filter_kind(ActivityEventKind::ActivityScheduled)
                .count(),
            3
        );
        assert_eq!(
            run.history()
                .filter_kind(ActivityEventKind::ActivityFailed)
                .count(),
            3
        );
    }

    /// complete 之后再执行活动 / 再 complete → RunAlreadyFinished。
    #[test]
    fn finished_run_rejects_further_work() {
        let exec = CountingEcho::new();
        let mut run = DurableRun::start(json!({}), 1);
        run.complete(json!(null), 2).unwrap();
        assert!(run.is_finished());
        assert!(matches!(
            run.execute_activity("echo", json!(1), &exec, no_retry(), 3),
            Err(DurableError::RunAlreadyFinished)
        ));
        assert!(matches!(
            run.complete(json!(null), 4),
            Err(DurableError::RunAlreadyFinished)
        ));
    }

    /// resume 空日志 / 非 RunStarted 开头 → HistoryCorrupted。
    #[test]
    fn resume_rejects_corrupt_prefix() {
        assert!(matches!(
            DurableRun::resume(DurableHistory::new()),
            Err(DurableError::HistoryCorrupted { .. })
        ));
        let mut h = DurableHistory::new();
        h.record(
            ActivityEventKind::ActivityScheduled,
            "a",
            None,
            None,
            None,
            1,
            1,
        );
        assert!(matches!(
            DurableRun::resume(h),
            Err(DurableError::HistoryCorrupted { .. })
        ));
    }

    /// 新鲜路径续跑: resume 一个尚未 complete 的 journal, 追加第二个活动。
    #[test]
    fn resume_extends_journal_with_fresh_activity() {
        let exec = CountingEcho::new();
        let mut run = DurableRun::start(json!({}), 1);
        run.execute_activity("echo", json!(1), &exec, no_retry(), 2)
            .unwrap();
        let history = run.into_history();

        let mut resumed = DurableRun::resume(history).unwrap();
        let first = resumed
            .execute_activity("echo", json!(1), &exec, no_retry(), 3)
            .unwrap();
        assert_eq!(first, json!(1));
        let second = resumed
            .execute_activity("echo", json!(2), &exec, no_retry(), 4)
            .unwrap();
        assert_eq!(second, json!(2));
        resumed.complete(json!({"sum": 3}), 5).unwrap();
        assert_eq!(
            exec.hits(),
            2,
            "first activity replayed, second executed once"
        );
        assert_eq!(resumed.history().len(), 6);
    }
}
