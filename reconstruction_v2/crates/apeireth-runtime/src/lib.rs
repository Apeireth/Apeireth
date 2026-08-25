pub mod agent;
pub mod bench;
pub mod capability_registry;
pub mod central;
pub mod context_fold;
pub mod council;
pub mod cron;
pub mod environment;
pub mod eval;
pub mod event_bus_backbone;
pub mod evolution;
pub mod extension;
pub mod host;
pub mod hybrid;
pub mod life_force;
pub mod lifecycle;
pub mod model_router;
pub mod orchestrator;
pub mod pipeline;
pub mod pipeline_g5;
pub mod presence_hub;
pub mod rate_limiter;
pub mod scheduler;
pub mod session_manager;
pub mod state;
pub mod supervisor;
pub mod task_store;
pub mod team_lead;
pub mod telemetry;
pub mod upgrade;
pub mod v1agent;
pub mod v1bench;
pub mod v1central;
pub mod v1host;
pub mod v1integration_e2e;
pub mod v1state;
pub mod v1supervisor;
pub mod v1tui_e2e;
pub mod verify;

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
