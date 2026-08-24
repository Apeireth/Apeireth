pub mod scheduler;
pub mod session_manager;
pub mod task_store;
pub mod supervisor;
pub mod telemetry;
pub mod host;
pub mod hybrid;
pub mod model_router;
pub mod event_bus_backbone;
pub mod capability_registry;
pub mod presence_hub;
pub mod lifecycle;

pub use host::{UnifiedRuntimeHost, ChatTurnOutput};
pub use session_manager::SessionState;
pub use hybrid::{HybridCognitiveRouter, HybridRoutingDecision};


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
