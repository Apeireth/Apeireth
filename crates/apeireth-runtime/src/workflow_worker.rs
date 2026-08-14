//! R263: apeireth-workflow AsyncWorker integration
//!
//! 让 apeireth-runtime 的 Runtime::register_worker 能接受 WorkflowWorker
//! (来自 apeireth-workflow) 作为 AsyncWorker. WorkflowRunner::run 是 sync,
//! 这里用 tokio::task::spawn_blocking 包成 async fn, 避免阻塞 runtime.

#![allow(missing_docs)]

use std::sync::Arc;

use apeireth_workflow::WorkflowWorker;
use crate::{AsyncWorker, TaskId};

#[async_trait::async_trait]
impl AsyncWorker for WorkflowWorker {
    fn name(&self) -> &str {
        WorkflowWorker::name(self)
    }

    async fn execute(&self, task_id: TaskId, params_json: String) -> Result<String, String> {
        // Sync workflow run, 在 blocking pool 跑避免阻塞 runtime.
        // WorkflowWorker: Clone (Arc<WorkflowRunner> + String), 'static safe.
        let worker: WorkflowWorker = self.clone();
        let tid = task_id;
        tokio::task::spawn_blocking(move || {
            let _ = tid; // task_id 暂不用于 workflow (workflow 内部 event_id 自管)
            worker.execute_with_input(&params_json)
        })
        .await
        .map_err(|e| format!("workflow join error: {}", e))?
    }
}
