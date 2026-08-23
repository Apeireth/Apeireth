pub mod scheduler;
pub mod task_store;
pub mod supervisor;
pub mod telemetry;

use std::sync::Arc;

pub struct RuntimeEngine {
    pub scheduler: Arc<scheduler::Scheduler>,
    pub task_store: Arc<task_store::AsyncTaskStore>,
    pub supervisor: Arc<supervisor::Supervisor>,
}

impl Default for RuntimeEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeEngine {
    pub fn new() -> Self {
        Self {
            scheduler: Arc::new(scheduler::Scheduler::new()),
            task_store: Arc::new(task_store::AsyncTaskStore::new()),
            supervisor: Arc::new(supervisor::Supervisor::default()),
        }
    }
}
