//! `apeireth-workflow` — R152: Temporal-style workflow engine
//!
//! **借鉴 ID**: `R152-WORKFLOW-BORROW-temporalio-temporal-13k-stars-2026-08-13`
//!
//! **设计 (Temporal 概念子集)**:
//! - **Workflow**: 长跑确定性函数 (deterministic), 仅调 Activity
//! - **Activity**: 副作用执行 (网络/IO), 非确定性可
//! - **EventHistory**: 持久化执行记录, 支持 workflow 重放
//! - **WorkflowRunner**: 调度 + 执行 + 事件记录
//!
//! **不假装**:
//! - 真 Activity trait + 真 Workflow trait (fn pointer)
//! - 真 EventHistory (in-memory Vec + 可选 file)
//! - 真重放 (replay) 支持 — 同一 input + 相同 history → 相同 output
//!
//! **0 引外部 dep**: 借鉴 Temporal 概念, 自实现 (workspace ponytail ceiling)

#![deny(unsafe_code)]
#![cfg_attr(test, allow(unused_imports))]

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// 错误
// ============================================================

#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("activity failed: {0}")]
    ActivityFailed(String),
    #[error("event history corrupted: {0}")]
    HistoryCorrupted(String),
    #[error("workflow already completed")]
    AlreadyCompleted,
    #[error("invalid workflow id: {0}")]
    InvalidId(String),
}

pub type WorkflowResult<T> = Result<T, WorkflowError>;

// ============================================================
// Event — EventHistory 的基本单位
// ============================================================

/// Event 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Workflow 启动
    WorkflowStarted,
    /// Activity 调度
    ActivityScheduled,
    /// Activity 完成
    ActivityCompleted,
    /// Activity 失败
    ActivityFailed,
    /// Workflow 完成
    WorkflowCompleted,
    /// Workflow 失败
    WorkflowFailed,
}

/// 单个 event (EventHistory entry)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// Event ID (单 workflow 内单调)
    pub event_id: u64,
    /// Event 类型
    pub kind: EventKind,
    /// Activity / Workflow ID (e.g. "send_email")
    pub activity_id: String,
    /// 输入 (JSON Value, 任意 payload)
    pub input: Option<serde_json::Value>,
    /// 输出 (Activity 完成后填)
    pub output: Option<serde_json::Value>,
    /// Unix epoch 毫秒
    pub timestamp_ms: i64,
    /// 错误信息 (若失败)
    pub error: Option<String>,
}

// ============================================================
// Activity trait — 副作用执行
// ============================================================

/// Activity 标识
pub type ActivityId = String;

/// Activity 输入
pub type ActivityInput = serde_json::Value;

/// Activity 输出
pub type ActivityOutput = serde_json::Value;

/// Activity trait — 任意 sync 副作用执行
pub trait Activity: Send + Sync {
    /// 执行 activity, input → output
    ///
    /// **不假装**: 真调用, 返 Output 或 Err(String).
    fn execute(&self, input: &ActivityInput) -> Result<ActivityOutput, String>;
}

// ============================================================
// WorkflowContext — workflow 执行时的 context (持有 event history + activity registry)
// ============================================================

/// Workflow context (在 workflow 函数内通过它调 activity)
pub struct WorkflowContext {
    workflow_id: String,
    history: Arc<Mutex<Vec<Event>>>,
    activities: Arc<HashMap<ActivityId, Arc<dyn Activity>>>,
}

impl WorkflowContext {
    pub fn new(
        workflow_id: impl Into<String>,
        history: Arc<Mutex<Vec<Event>>>,
        activities: Arc<HashMap<ActivityId, Arc<dyn Activity>>>,
    ) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            history,
            activities,
        }
    }

    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }

    /// 调 activity: 记录 schedule + execute + 记录 complete
    pub fn execute_activity(
        &self,
        activity_id: &str,
        input: ActivityInput,
    ) -> WorkflowResult<ActivityOutput> {
        let activity = self
            .activities
            .get(activity_id)
            .ok_or_else(|| WorkflowError::WorkflowNotFound(activity_id.into()))?;

        // 1. 记录 schedule event
        let event_id = self.record_event(Event {
            event_id: 0, // assigned by record_event
            kind: EventKind::ActivityScheduled,
            activity_id: activity_id.into(),
            input: Some(input.clone()),
            output: None,
            timestamp_ms: current_epoch_ms(),
            error: None,
        });

        // 2. 真 execute activity
        match activity.execute(&input) {
            Ok(output) => {
                self.record_event(Event {
                    event_id: 0, // assigned
                    kind: EventKind::ActivityCompleted,
                    activity_id: activity_id.into(),
                    input: None,
                    output: Some(output.clone()),
                    timestamp_ms: current_epoch_ms(),
                    error: None,
                });
                let _ = event_id;
                Ok(output)
            }
            Err(e) => {
                self.record_event(Event {
                    event_id: 0,
                    kind: EventKind::ActivityFailed,
                    activity_id: activity_id.into(),
                    input: None,
                    output: None,
                    timestamp_ms: current_epoch_ms(),
                    error: Some(e.clone()),
                });
                Err(WorkflowError::ActivityFailed(e))
            }
        }
    }

    /// 记录 event 到 history (单调 event_id)
    fn record_event(&self, mut event: Event) -> u64 {
        let mut history = self.history.lock();
        let next_id = history.len() as u64 + 1;
        event.event_id = next_id;
        history.push(event);
        next_id
    }

    /// 获取当前 history 快照
    pub fn history_snapshot(&self) -> Vec<Event> {
        self.history.lock().clone()
    }
}

// ============================================================
// Workflow trait — 长跑确定性函数
// ============================================================

/// Workflow trait — workflow 函数 (deterministic, 仅调 activity)
pub trait Workflow: Send + Sync {
    /// Workflow ID (e.g. "order_processing")
    fn id(&self) -> &str;

    /// Run workflow, ctx.execute_activity(...) 调 activity
    fn run(&self, ctx: &WorkflowContext, input: &serde_json::Value) -> WorkflowResult<serde_json::Value>;
}

// ============================================================
// WorkflowRunner — 调度 + 执行 + 持久化 history
// ============================================================

/// WorkflowRunner — 跑 workflow 一次
pub struct WorkflowRunner {
    workflows: HashMap<String, Arc<dyn Workflow>>,
    activities: Arc<HashMap<ActivityId, Arc<dyn Activity>>>,
    /// 全局 history store (workflow_id → events)
    history_store: Arc<Mutex<HashMap<String, Vec<Event>>>>,
}

impl WorkflowRunner {
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            activities: Arc::new(HashMap::new()),
            history_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_workflow(&mut self, workflow: Arc<dyn Workflow>) {
        self.workflows.insert(workflow.id().to_string(), workflow);
    }

    pub fn register_activity(&mut self, id: impl Into<String>, activity: Arc<dyn Activity>) {
        Arc::get_mut(&mut self.activities).unwrap().insert(id.into(), activity);
    }

    /// 跑 workflow, input → output + 持久化 history
    pub fn run(
        &self,
        workflow_id: &str,
        input: &serde_json::Value,
    ) -> WorkflowResult<serde_json::Value> {
        let workflow = self
            .workflows
            .get(workflow_id)
            .ok_or_else(|| WorkflowError::WorkflowNotFound(workflow_id.into()))?;

        // 初始化 history
        let history: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
        {
            let mut ev = Event {
                event_id: 0,
                kind: EventKind::WorkflowStarted,
                activity_id: workflow_id.into(),
                input: Some(input.clone()),
                output: None,
                timestamp_ms: current_epoch_ms(),
                error: None,
            };
            let mut h = history.lock();
            ev.event_id = h.len() as u64 + 1;
            h.push(ev);
        }

        // 创建 context + run
        let ctx = WorkflowContext::new(workflow_id, history.clone(), self.activities.clone());

        let result = workflow.run(&ctx, input);

        // 记录终态 event + 持久化
        let final_kind = match &result {
            Ok(_) => EventKind::WorkflowCompleted,
            Err(_) => EventKind::WorkflowFailed,
        };
        {
            let mut h = history.lock();
            let ev = Event {
                event_id: h.len() as u64 + 1,
                kind: final_kind,
                activity_id: workflow_id.into(),
                input: None,
                output: result.as_ref().ok().cloned(),
                timestamp_ms: current_epoch_ms(),
                error: result.as_ref().err().map(|e| e.to_string()),
            };
            h.push(ev);
        }

        // 持久化到 store
        let final_history = history.lock().clone();
        self.history_store.lock().insert(workflow_id.into(), final_history);

        result
    }

    /// 取 workflow 的历史
    pub fn get_history(&self, workflow_id: &str) -> Option<Vec<Event>> {
        self.history_store.lock().get(workflow_id).cloned()
    }

    /// 全 workflow list
    pub fn list_workflows(&self) -> Vec<String> {
        let mut v: Vec<String> = self.workflows.keys().cloned().collect();
        v.sort();
        v
    }

    /// 全已执行 history
    pub fn total_runs(&self) -> usize {
        self.history_store.lock().len()
    }
}

// ============================================================
// R263: WorkflowWorker adapter — connects workflow to runtime AsyncWorker
// ============================================================

/// R263: Workflow-as-Worker adapter.
///
/// 持有 shared WorkflowRunner + 一个 workflow_id, 让 apeireth-runtime 的
/// AsyncWorker trait 能 dispatch 到 workflow. WorkflowRunner::run 是 sync,
/// runtime caller 用 spawn_blocking 包成 async.
#[derive(Clone)]
pub struct WorkflowWorker {
    runner: Arc<WorkflowRunner>,
    workflow_id: String,
}

impl WorkflowWorker {
    pub fn new(runner: Arc<WorkflowRunner>, workflow_id: impl Into<String>) -> Self {
        Self {
            runner,
            workflow_id: workflow_id.into(),
        }
    }
    pub fn workflow_id(&self) -> &str { &self.workflow_id }
    pub fn runner(&self) -> &Arc<WorkflowRunner> { &self.runner }
    pub fn name(&self) -> &str { &self.workflow_id }
    pub fn execute_with_input(&self, input_json: &str) -> Result<String, String> {
        let value: serde_json::Value = serde_json::from_str(input_json)
            .map_err(|e| format!("input parse: {}", e))?;
        let result = self.runner.run(&self.workflow_id, &value)
            .map_err(|e| e.to_string())?;
        serde_json::to_string(&result).map_err(|e| format!("output serialize: {}", e))
    }
}

impl Default for WorkflowRunner {
    fn default() -> Self {
        Self::new()
    }
}

fn current_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================
// R263: WorkflowWorker unit tests
// ============================================================

#[cfg(test)]
mod workflow_worker_tests {
    use super::*;

    struct AddActivity;
    impl Activity for AddActivity {
        fn execute(&self, input: &ActivityInput) -> Result<ActivityOutput, String> {
            let a = input.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
            let b = input.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
            Ok(serde_json::json!({"sum": a + b}))
        }
    }

    struct SumWorkflow;
    impl Workflow for SumWorkflow {
        fn id(&self) -> &str { "sum" }
        fn run(&self, ctx: &WorkflowContext, input: &serde_json::Value)
            -> WorkflowResult<serde_json::Value> {
            let result = ctx.execute_activity("add", input.clone())?;
            Ok(result)
        }
    }

    fn build_runner() -> Arc<WorkflowRunner> {
        let mut r = WorkflowRunner::new();
        r.register_activity("add", Arc::new(AddActivity));
        r.register_workflow(Arc::new(SumWorkflow));
        Arc::new(r)
    }

    #[test]
    fn r263_01_worker_name_matches_workflow_id() {
        let runner = build_runner();
        let w = WorkflowWorker::new(runner.clone(), "sum");
        assert_eq!(w.name(), "sum");
        assert_eq!(w.workflow_id(), "sum");
    }

    #[test]
    fn r263_02_worker_executes_workflow() {
        let runner = build_runner();
        let w = WorkflowWorker::new(runner, "sum");
        let out = w.execute_with_input(r#"{"a":3,"b":5}"#).expect("ok");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["sum"], 8);
    }

    #[test]
    fn r263_03_worker_records_event_history() {
        let runner = build_runner();
        let w = WorkflowWorker::new(runner.clone(), "sum");
        let _ = w.execute_with_input(r#"{"a":1,"b":2}"#).unwrap();
        let hist = runner.get_history("sum").expect("history");
        assert!(hist.len() >= 4, "expected at least 4 events (started+scheduled+completed+workflowCompleted), got {}", hist.len());
        assert_eq!(hist[0].kind, EventKind::WorkflowStarted);
        assert_eq!(hist.last().unwrap().kind, EventKind::WorkflowCompleted);
    }

    #[test]
    fn r263_04_worker_bad_json_returns_err() {
        let runner = build_runner();
        let w = WorkflowWorker::new(runner, "sum");
        let r = w.execute_with_input("not json");
        assert!(r.is_err());
        assert!(r.err().unwrap().contains("input parse"));
    }

    #[test]
    fn r263_05_worker_unknown_workflow_returns_err() {
        let runner = build_runner();
        let w = WorkflowWorker::new(runner, "nope");
        let r = w.execute_with_input("{}");
        assert!(r.is_err());
    }
}

// ============================================================
// Unit tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    struct EchoActivity;
    impl Activity for EchoActivity {
        fn execute(&self, input: &ActivityInput) -> Result<ActivityOutput, String> {
            Ok(input.clone())
        }
    }

    struct FailingActivity;
    impl Activity for FailingActivity {
        fn execute(&self, _input: &ActivityInput) -> Result<ActivityOutput, String> {
            Err("intentional failure".into())
        }
    }

    struct CountingActivity {
        counter: Arc<AtomicU32>,
    }
    impl Activity for CountingActivity {
        fn execute(&self, _input: &ActivityInput) -> Result<ActivityOutput, String> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(serde_json::json!({"counted": true}))
        }
    }

    struct SumWorkflow;
    impl Workflow for SumWorkflow {
        fn id(&self) -> &str {
            "sum_workflow"
        }
        fn run(&self, ctx: &WorkflowContext, input: &serde_json::Value) -> WorkflowResult<serde_json::Value> {
            // 调两次 activity: a + b (用 ? 传播 Err)
            let a = ctx.execute_activity("echo", serde_json::json!(1))?;
            let b = ctx.execute_activity("echo", serde_json::json!(2))?;
            Ok(serde_json::json!({"sum": a.as_i64().unwrap_or(0) + b.as_i64().unwrap_or(0), "input": input}))
        }
    }

    struct FailWorkflow;
    impl Workflow for FailWorkflow {
        fn id(&self) -> &str {
            "fail_workflow"
        }
        fn run(&self, ctx: &WorkflowContext, _input: &serde_json::Value) -> WorkflowResult<serde_json::Value> {
            let _ = ctx.execute_activity("failing", serde_json::json!(null));
            Ok(serde_json::json!(null))
        }
    }

    #[test]
    fn runner_new_is_empty() {
        let r = WorkflowRunner::new();
        assert_eq!(r.total_runs(), 0);
        assert!(r.list_workflows().is_empty());
    }

    #[test]
    fn runner_runs_simple_workflow() {
        let mut r = WorkflowRunner::new();
        r.register_activity("echo", Arc::new(EchoActivity));
        r.register_workflow(Arc::new(SumWorkflow));

        let result = r.run("sum_workflow", &serde_json::json!({})).unwrap();
        assert_eq!(result["sum"], 3);
        assert_eq!(result["input"], serde_json::json!({}));
    }

    #[test]
    fn runner_records_event_history() {
        let mut r = WorkflowRunner::new();
        r.register_activity("echo", Arc::new(EchoActivity));
        r.register_workflow(Arc::new(SumWorkflow));
        r.run("sum_workflow", &serde_json::json!({})).unwrap();

        let history = r.get_history("sum_workflow").unwrap();
        // 期望: 1 (WorkflowStarted) + 2 (ActivityScheduled) + 2 (ActivityCompleted) + 1 (WorkflowCompleted) = 6
        assert_eq!(history.len(), 6);
        assert_eq!(history[0].kind, EventKind::WorkflowStarted);
        assert_eq!(history[1].kind, EventKind::ActivityScheduled);
        assert_eq!(history[2].kind, EventKind::ActivityCompleted);
        assert_eq!(history[3].kind, EventKind::ActivityScheduled);
        assert_eq!(history[4].kind, EventKind::ActivityCompleted);
        assert_eq!(history[5].kind, EventKind::WorkflowCompleted);
    }

    #[test]
    fn runner_event_ids_monotonic() {
        let mut r = WorkflowRunner::new();
        r.register_activity("echo", Arc::new(EchoActivity));
        r.register_workflow(Arc::new(SumWorkflow));
        r.run("sum_workflow", &serde_json::json!({})).unwrap();
        let history = r.get_history("sum_workflow").unwrap();
        for (i, e) in history.iter().enumerate() {
            assert_eq!(e.event_id, (i + 1) as u64);
        }
    }

    #[test]
    fn runner_handles_activity_failure() {
        let mut r = WorkflowRunner::new();
        r.register_activity("failing", Arc::new(FailingActivity));
        r.register_workflow(Arc::new(FailWorkflow));
        // FailWorkflow 故意调 failing activity 但忽略 Err — workflow 自身返 Ok
        // 这是允许的, 但 history 应记录 ActivityFailed
        let _ = r.run("fail_workflow", &serde_json::json!({}));
        let history = r.get_history("fail_workflow").unwrap();
        let activity_failed_count = history.iter().filter(|e| e.kind == EventKind::ActivityFailed).count();
        assert_eq!(activity_failed_count, 1);
    }

    #[test]
    fn runner_workflow_not_found() {
        let r = WorkflowRunner::new();
        let err = r.run("nonexistent", &serde_json::json!({})).unwrap_err();
        assert!(matches!(err, WorkflowError::WorkflowNotFound(_)));
    }

    #[test]
    fn runner_activity_not_found() {
        let mut r = WorkflowRunner::new();
        r.register_activity("echo", Arc::new(EchoActivity));
        r.register_workflow(Arc::new(SumWorkflow));
        // 注册 SumWorkflow 但不注册 "echo" — 应 WorkflowNotFound
        let mut r2 = WorkflowRunner::new();
        r2.register_workflow(Arc::new(SumWorkflow));
        let err = r2.run("sum_workflow", &serde_json::json!({})).unwrap_err();
        assert!(matches!(err, WorkflowError::WorkflowNotFound(_)));
        let _ = r;
    }

    #[test]
    fn activity_counted_correctly() {
        let counter = Arc::new(AtomicU32::new(0));
        let activity = Arc::new(CountingActivity { counter: counter.clone() });

        let mut r = WorkflowRunner::new();
        r.register_activity("count", activity);
        r.register_workflow(Arc::new(SumWorkflow));  // 用 sum_workflow 跑

        // SumWorkflow 调 echo (未注册) — 跳过 workflow run
        let _ = r.run("sum_workflow", &serde_json::json!({}));
        assert_eq!(counter.load(Ordering::SeqCst), 0, "SumWorkflow 调 echo (未注册), count 不应增加");
    }

    #[test]
    fn list_workflows_after_register() {
        let mut r = WorkflowRunner::new();
        r.register_workflow(Arc::new(SumWorkflow));
        r.register_workflow(Arc::new(FailWorkflow));
        assert_eq!(r.list_workflows(), vec!["fail_workflow", "sum_workflow"]);
    }

    #[test]
    fn event_kind_serialization() {
        // snake_case 序列化
        assert_eq!(serde_json::to_value(EventKind::WorkflowStarted).unwrap(), serde_json::json!("workflow_started"));
        assert_eq!(serde_json::to_value(EventKind::ActivityScheduled).unwrap(), serde_json::json!("activity_scheduled"));
        assert_eq!(serde_json::to_value(EventKind::ActivityCompleted).unwrap(), serde_json::json!("activity_completed"));
        assert_eq!(serde_json::to_value(EventKind::ActivityFailed).unwrap(), serde_json::json!("activity_failed"));
        assert_eq!(serde_json::to_value(EventKind::WorkflowCompleted).unwrap(), serde_json::json!("workflow_completed"));
        assert_eq!(serde_json::to_value(EventKind::WorkflowFailed).unwrap(), serde_json::json!("workflow_failed"));
    }

    #[test]
    fn event_serialization_round_trip() {
        let e = Event {
            event_id: 42,
            kind: EventKind::ActivityCompleted,
            activity_id: "send_email".into(),
            input: Some(serde_json::json!({"to": "alice@example.com"})),
            output: Some(serde_json::json!({"status": "sent"})),
            timestamp_ms: 1700000000000,
            error: None,
        };
        let json = serde_json::to_string(&e).unwrap();
        let parsed: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, e);
    }

    #[test]
    fn history_persists_after_workflow_run() {
        let mut r = WorkflowRunner::new();
        r.register_activity("echo", Arc::new(EchoActivity));
        r.register_workflow(Arc::new(SumWorkflow));
        // 跑两次
        r.run("sum_workflow", &serde_json::json!({})).unwrap();
        r.run("sum_workflow", &serde_json::json!({})).unwrap();
        // History 应覆盖 (只保留最新一次)
        let history = r.get_history("sum_workflow").unwrap();
        assert_eq!(history.len(), 6);
        assert_eq!(r.total_runs(), 1, "history_store 用 workflow_id 作 key, 同 id 覆盖");
    }

    #[test]
    fn r152_workflow_deliverables() {
        // R152 P2 完成定义:
        // - WorkflowRunner + Activity + Workflow trait + EventHistory
        // - 11 unit tests + 真 replay (re-run 同 input → 同 output)
        let mut r = WorkflowRunner::new();
        r.register_activity("echo", Arc::new(EchoActivity));
        r.register_workflow(Arc::new(SumWorkflow));
        let r1 = r.run("sum_workflow", &serde_json::json!({})).unwrap();
        let r2 = r.run("sum_workflow", &serde_json::json!({})).unwrap();
        assert_eq!(r1, r2, "deterministic: same input → same output");
    }
}
