//! Immutable turn catalogs over the one lifecycle owner. Shared services remain shared.
use super::Runtime;
impl Runtime {
    pub(super) fn execution_view(&self) -> Self {
        Self {
            instances: self.instances.clone(),
            instance_host: None,
            task: self.task.clone(),
            plugins: self.plugins.execution_snapshot(),
            v1_protocol: self.v1_protocol,
            providers: self.providers.clone(),
            sessions: self.sessions.clone(),
            governance: self.governance.clone(),
            clock: self.clock.clone(),
            config: self.config.clone(),
            modules: self.modules.clone(),
            registered_behaviors: self.registered_behaviors.clone(),
            capabilities: self.capabilities.execution_snapshot(),
            dispatcher: self.dispatcher.clone(),
            session_locks: self.session_locks.clone(),
            event_sink: self.event_sink.clone(),
            context_projector: self.context_projector.clone(),
            repetition_detectors: self.repetition_detectors.clone(),
            invariant_auditor: self.invariant_auditor.clone(),
            context_spill: self.context_spill.clone(),
            compaction: self.compaction.clone(),
            call_scheduler: self.call_scheduler.clone(),
            metrics: self.metrics.clone(),
            tool_completion_observer: self.tool_completion_observer.clone(),
        }
    }
}
