# R263: apeireth-workflow → apeireth-runtime AsyncWorker integration

**日期**: 2026-08-14
**作者**: 楚零
**目的**: 把 Temporal-style workflow (R152) 通过 `WorkflowWorker` adapter 接入 runtime 的 `AsyncWorker` trait

---

## §1 背景

R152 实现了 `apeireth-workflow` crate (Temporal 概念子集: Workflow + Activity + EventHistory + WorkflowRunner),
但 workflow 是 sync `pub fn run()`, 不能直接对接 runtime 的 `async fn execute()` AsyncWorker trait.
runtime 现有的 `LlmWorker` (R255) / `SimulatedWorker` 都直接 `impl AsyncWorker`, workflow 缺这一层.

---

## §2 设计

### 2.1 WorkflowWorker (apeireth-workflow 新增)

```rust
#[derive(Clone)]
pub struct WorkflowWorker {
    runner: Arc<WorkflowRunner>,
    workflow_id: String,
}

impl WorkflowWorker {
    pub fn new(runner: Arc<WorkflowRunner>, workflow_id: impl Into<String>) -> Self;
    pub fn workflow_id(&self) -> &str;
    pub fn name(&self) -> &str;  // = workflow_id (跟 AsyncWorker::name 一致)
    pub fn execute_with_input(&self, input_json: &str) -> Result<String, String>;  // sync
}
```

**关键**:
- `#[derive(Clone)]` — Arc<WorkflowRunner> 共享, Clone 安全, 满足 `'static` 要求
- `execute_with_input` 同步接口 (WorkflowRunner::run 是 sync), 上层用 `spawn_blocking` 包成 async

### 2.2 AsyncWorker impl (apeireth-runtime 新增 file `src/workflow_worker.rs`)

```rust
use apeireth_workflow::WorkflowWorker;
use crate::{AsyncWorker, TaskId};

#[async_trait::async_trait]
impl AsyncWorker for WorkflowWorker {
    fn name(&self) -> &str { WorkflowWorker::name(self) }
    async fn execute(&self, task_id: TaskId, params_json: String) -> Result<String, String> {
        let worker: WorkflowWorker = self.clone();
        let tid = task_id;
        tokio::task::spawn_blocking(move || {
            let _ = tid;
            worker.execute_with_input(&params_json)
        })
        .await
        .map_err(|e| format!("workflow join error: {}", e))?
    }
}
```

**关键**:
- `spawn_blocking` 把 sync workflow 放到 blocking pool, 不阻塞 tokio runtime worker
- `Clone` 让 worker 满足 `'static` (否则 `self` 借用无法 spawn)
- workflow event_id 自管, task_id 仅用于 tracing 关联

### 2.3 runtime Cargo.toml dep

```toml
# R263: apeireth-workflow runtime integration (AsyncWorker adapter)
apeireth-workflow = { path = "../apeireth-workflow" }
```

---

## §3 测试 (5 unit + 2 e2e = 7 cases)

### 3.1 apeireth-workflow (workflow_worker_tests mod)

- r263_01 name matches workflow_id
- r263_02 executes workflow (sum 3+5 = 8)
- r263_03 records event history (>=4 events: started + scheduled + completed + workflowCompleted)
- r263_04 bad JSON returns Err (含 "input parse")
- r263_05 unknown workflow returns Err

### 3.2 apeireth-runtime/tests/r263_workflow_e2e.rs (2 cases)

- r263_e2e_workflow_dispatched_via_runtime:
  - 注册 WorkflowWorker (workflow.double) → runtime.dispatch_async_task → 等完成 → 验证 result_json 含 `{"doubled":42}` → 验证 history events
- r263_e2e_workflow_error_marks_failed:
  - 不注册 activity (activity_not_found) → task 标 Failed + error 字段填充

**18 + 2 = 20 tests pass** (workflow 13 旧 + 5 新 R263 + runtime e2e 2 新).

---

## §4 主哲学锚对齐

- **S-1 北极星**: 借 Temporal workflow, 自实现无外部 dep
- **S-2 实事求是**: spawn_blocking 真接 sync workflow, 不假装 async
- **O-1 安全优先**: workflow runner 是 Arc<...> shared, 多线程安全
- **O-2 走在前人**: LangGraph / Temporal 概念子集, Rust 化
- **O-3 干到底**: WorkflowWorker + AsyncWorker impl + 7 tests 全过
- **O-5 不假装**: e2e 真接 runtime dispatch, 验证 result_json + history
