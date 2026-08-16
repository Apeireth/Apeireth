//! R263: e2e test - register a WorkflowWorker with runtime, dispatch task,
//! wait for completion, verify result.

#![allow(missing_docs)]

use apeireth_runtime::Runtime;
use apeireth_tool_registry::TaskStatus;
use apeireth_workflow::{
    Activity, ActivityInput, ActivityOutput, Workflow, WorkflowContext, WorkflowResult,
    WorkflowRunner, WorkflowWorker,
};
use std::sync::Arc;
use std::time::Duration;

struct DoubleActivity;
impl Activity for DoubleActivity {
    fn execute(&self, input: &ActivityInput) -> Result<ActivityOutput, String> {
        let n = input.get("n").and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(serde_json::json!({"doubled": n * 2}))
    }
}

struct DoubleWorkflow;
impl Workflow for DoubleWorkflow {
    fn id(&self) -> &str {
        "double"
    }
    fn run(
        &self,
        ctx: &WorkflowContext,
        input: &serde_json::Value,
    ) -> WorkflowResult<serde_json::Value> {
        let result = ctx.execute_activity("double", input.clone())?;
        Ok(result)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn r263_e2e_workflow_dispatched_via_runtime() {
    let mut runner = WorkflowRunner::new();
    runner.register_activity("double", Arc::new(DoubleActivity));
    runner.register_workflow(Arc::new(DoubleWorkflow));
    let runner = Arc::new(runner);

    let worker = WorkflowWorker::new(runner.clone(), "double");

    let runtime = Runtime::new();
    runtime.register_worker("workflow.double", Arc::new(worker));

    let task_id = runtime
        .dispatch_async_task("workflow.double", r#"{"n":21}"#)
        .await;

    let rec = runtime
        .task_store
        .wait_for_completion(task_id, Duration::from_secs(10))
        .await
        .expect("wait_for_completion");

    assert_eq!(
        rec.status,
        TaskStatus::Completed,
        "expected Completed, got {:?}",
        rec.status
    );
    let result_json = rec.result_json.expect("result_json populated on Completed");
    let v: serde_json::Value = serde_json::from_str(&result_json).expect("output json");
    assert_eq!(v["doubled"], 42, "expected 42, got {}", v);

    let hist = runner.get_history("double").expect("history");
    assert!(
        hist.len() >= 4,
        "expected at least 4 events, got {}",
        hist.len()
    );
    assert_eq!(hist[0].kind, apeireth_workflow::EventKind::WorkflowStarted);
    assert_eq!(
        hist.last().unwrap().kind,
        apeireth_workflow::EventKind::WorkflowCompleted
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn r263_e2e_workflow_error_marks_failed() {
    let mut runner = WorkflowRunner::new();
    // No activity registered -> "activity_not_found" error
    runner.register_workflow(Arc::new(DoubleWorkflow));
    let runner = Arc::new(runner);
    let worker = WorkflowWorker::new(runner, "double");

    let runtime = Runtime::new();
    runtime.register_worker("workflow.double", Arc::new(worker));

    let task_id = runtime
        .dispatch_async_task("workflow.double", r#"{"n":1}"#)
        .await;
    let rec = runtime
        .task_store
        .wait_for_completion(task_id, Duration::from_secs(10))
        .await
        .expect("wait_for_completion");
    assert_eq!(
        rec.status,
        TaskStatus::Failed,
        "expected Failed, got {:?}",
        rec.status
    );
    assert!(rec.error.is_some(), "error message populated");
}
