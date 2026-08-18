//! **R225 — in-process Workflow 验证调用** (per 蓝图 §3.7 缺口 5)
//!
//! **背景**: R152 的早期测试引用了尚未落地的 API (`DefaultWorkflowValidator` 等),
//!   长期未跑, 编译失败. R225 把它改成调用真实存在的 API:
//!   `WorkflowRunner::register_workflow` + `register_activity` + `run` + `get_history`.
//!
//! **3 测试**:
//! 1. `WorkflowRunner::new()` 初始为空 (list_workflows == 0, total_runs == 0)
//! 2. 注册 workflow + activity + run → history 记录 EventKind::WorkflowStarted
//! 3. Activity 错误传播到 workflow run 输出
//!
//! **不假装**: 用 lib 现有方法, 0 编造 API.

use apeireth_workflow::{
    Activity, ActivityInput, ActivityOutput, Workflow, WorkflowContext, WorkflowResult,
    WorkflowRunner,
};
use serde_json::json;
use std::sync::Arc;

// ============================================================
// §1 测试活动 — 真实现, 不 stub
// ============================================================

struct EchoActivity;

impl Activity for EchoActivity {
    fn execute(&self, input: &ActivityInput) -> Result<ActivityOutput, String> {
        Ok(input.clone())
    }
}

struct FailingActivity;

impl Activity for FailingActivity {
    fn execute(&self, _input: &ActivityInput) -> Result<ActivityOutput, String> {
        Err("activity failed by design".to_string())
    }
}

// ============================================================
// §2 测试 workflow — 调 Activity 后返 output
// ============================================================

struct EchoWorkflow;

impl Workflow for EchoWorkflow {
    fn id(&self) -> &str {
        "EchoWorkflow"
    }
    fn run(
        &self,
        ctx: &WorkflowContext,
        input: &serde_json::Value,
    ) -> WorkflowResult<serde_json::Value> {
        // 调一次 echo activity
        let out = ctx.execute_activity("echo", input.clone())?;
        Ok(out)
    }
}

struct FailingWorkflow;

impl Workflow for FailingWorkflow {
    fn id(&self) -> &str {
        "FailingWorkflow"
    }
    fn run(
        &self,
        ctx: &WorkflowContext,
        input: &serde_json::Value,
    ) -> WorkflowResult<serde_json::Value> {
        // 调一次 failing activity, 错误传播
        let _ = ctx.execute_activity("failing", input.clone())?;
        Ok(json!({"unreachable": true}))
    }
}

// ============================================================
// §3 测试用例 (3 cases, R225)
// ============================================================

/// **`WorkflowRunner::new()` 初始为空**
#[test]
fn test_workflow_runner_starts_empty() {
    let runner = WorkflowRunner::new();
    assert_eq!(
        runner.list_workflows().len(),
        0,
        "新建 runner 应无 workflow"
    );
    assert_eq!(runner.total_runs(), 0, "新建 runner 应无 run 历史");
}

/// **register workflow + activity + run 端到端**
#[test]
fn test_workflow_runner_run_end_to_end() {
    let mut runner = WorkflowRunner::new();
    runner.register_activity("echo", Arc::new(EchoActivity));
    runner.register_workflow(Arc::new(EchoWorkflow));

    // run with input
    let input = json!({"hello": "world"});
    let result = runner.run("EchoWorkflow", &input).expect("run failed");
    assert_eq!(result, json!({"hello": "world"}));

    // history 应有 WorkflowStarted + ActivityScheduled + ActivityCompleted + WorkflowCompleted
    let history = runner.get_history("EchoWorkflow").expect("history missing");
    assert!(
        history
            .iter()
            .any(|e| format!("{:?}", e.kind).contains("WorkflowStarted")),
        "history 应有 WorkflowStarted: {history:?}"
    );
    assert!(
        history
            .iter()
            .any(|e| format!("{:?}", e.kind).contains("ActivityCompleted")),
        "history 应有 ActivityCompleted: {history:?}"
    );
    assert!(
        history
            .iter()
            .any(|e| format!("{:?}", e.kind).contains("WorkflowCompleted")),
        "history 应有 WorkflowCompleted: {history:?}"
    );

    assert_eq!(runner.total_runs(), 1);
}

/// **activity 失败传播到 workflow run 输出**
#[test]
fn test_workflow_propagates_activity_failure() {
    let mut runner = WorkflowRunner::new();
    runner.register_activity("failing", Arc::new(FailingActivity));
    runner.register_workflow(Arc::new(FailingWorkflow));

    let input = json!({"x": 1});
    let res = runner.run("FailingWorkflow", &input);
    assert!(
        res.is_err(),
        "workflow 应 propagate activity 错误, got: {res:?}"
    );
    let err = res.unwrap_err();
    let err_str = format!("{err}");
    assert!(
        err_str.contains("activity failed"),
        "错误应含 activity failed, got: {err_str}"
    );

    // history 应有 ActivityFailed
    let history = runner
        .get_history("FailingWorkflow")
        .expect("history missing");
    assert!(
        history
            .iter()
            .any(|e| format!("{:?}", e.kind).contains("ActivityFailed")),
        "history 应有 ActivityFailed: {history:?}"
    );
}
