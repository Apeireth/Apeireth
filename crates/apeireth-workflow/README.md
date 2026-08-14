# apeireth-workflow - R152 Temporal-style workflow engine

> 借鉴 ID: `R152-WORKFLOW-BORROW-temporalio-temporal-13k-stars-2026-08-13`
> 0 引外部 dep (workspace ponytail ceiling: 能不引就不引)

## 核心组件

- `Workflow` trait - 长跑确定性函数, 仅调 activity
- `Activity` trait - 副作用执行 (网络/IO, 非确定性可)
- `EventHistory` - 持久化执行记录, 支持 workflow 重放
- `WorkflowRunner` - 调度 + 执行 + 事件记录

## 公开 API

\`\`\`rust
use apeireth_workflow::{WorkflowRunner, Workflow, Activity, WorkflowContext, WorkflowResult};

// 1. 定义 Activity
struct EchoActivity;
impl Activity for EchoActivity {
    fn execute(&self, input: &serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(input.clone())
    }
}

// 2. 定义 Workflow
struct AddWorkflow;
impl Workflow for AddWorkflow {
    fn id(&self) -> &str { "add" }
    fn run(&self, ctx: &WorkflowContext, input: &serde_json::Value) -> WorkflowResult<serde_json::Value> {
        let a = ctx.execute_activity("echo", serde_json::json!(input["a"]))?;
        let b = ctx.execute_activity("echo", serde_json::json!(input["b"]))?;
        Ok(serde_json::json!({"result": a.as_i64().unwrap_or(0) + b.as_i64().unwrap_or(0)}))
    }
}

// 3. 注册 + 跑
let mut runner = WorkflowRunner::new();
runner.register_activity("echo", Arc::new(EchoActivity));
runner.register_workflow(Arc::new(AddWorkflow));
let result = runner.run("add", &serde_json::json!({"a": 10, "b": 20})).unwrap();
assert_eq!(result["result"], 30);
\`\`\`

## 事件类型

\`\`\`rust
enum EventKind {
    WorkflowStarted,
    ActivityScheduled,
    ActivityCompleted,
    ActivityFailed,
    WorkflowCompleted,
    WorkflowFailed,
}
\`\`\`

## R263: WorkflowWorker — 接到 runtime AsyncWorker

`WorkflowWorker` adapter 让 `apeireth-runtime` 的 `AsyncWorker` trait 能 dispatch workflow.
WorkflowRunner::run 是 sync, 用 `tokio::task::spawn_blocking` 包成 async.

```rust
use apeireth_workflow::WorkflowWorker;
use std::sync::Arc;

let mut runner = WorkflowRunner::new();
runner.register_activity("add", Arc::new(AddActivity));
runner.register_workflow(Arc::new(AddWorkflow));
let runner = Arc::new(runner);

let worker = WorkflowWorker::new(runner.clone(), "add");
// 在 apeireth-runtime 侧: runtime.register_worker("workflow.add", Arc::new(worker));
// 然后: runtime.dispatch_async_task("workflow.add", r#"{"a":3,"b":5}"#).await
```

`#[derive(Clone)]` 让 `WorkflowWorker` 满足 `'static` (Arc<WorkflowRunner> + String),
满足 `AsyncWorker::execute` 在 spawn_blocking 里的 `'static` 要求.

## 借鉴来源

| ID | 来源 | 模式 |
|---|---|---|
| R152-WORKFLOW-BORROW-temporalio-temporal-13k | temporalio/temporal | Workflow + Activity + EventHistory |

## 0 触碰

- 0 触碰 apeireth-pipeline (chat 5 步管线), 独立 crate
- 0 触碰 3 不可变脊柱 (Self-Disable / L0 HA / 13 键 verdict cache)

## 13 unit tests + 1 example

- runner_new_is_empty / runs_simple_workflow / records_event_history /
  event_ids_monotonic / handles_activity_failure / workflow_not_found /
  activity_not_found / counted_correctly / list_workflows_after_register /
  event_kind_serialization / event_serialization_round_trip /
  history_persists_after_workflow_run / r152_workflow_deliverables
- examples/workflow_demo.rs - AddWorkflow 跑 a=10 + b=20

详见: docs/r150/r150-p1-six-modules.md §1 (R150 跳过 #7 的原因 + R152 续做)
