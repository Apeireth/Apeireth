//! Authorized recovery runs before correction and recall on the next turn.
//! The writer is shared with AfterTurn; there is no detached worker or second store.
use super::cognitive::MemoryWritebackModule;
use apeireth_plugin::ToolExecutionContext;
use apeireth_runtime::canonical::module::{
    BehaviorModule, CommittedEvidenceAccess, HookPoint, ModuleContext, ModuleError, ModuleManifest,
    ModuleOutcome,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct MemoryRecoveryModule {
    manifest: ModuleManifest,
    writer: Arc<MemoryWritebackModule>,
}
impl MemoryRecoveryModule {
    pub fn new(writer: Arc<MemoryWritebackModule>) -> Self {
        Self {
            manifest: ModuleManifest::new("cognitive.memory_recovery", "Durable memory recovery"),
            writer,
        }
    }
}
#[async_trait]
impl BehaviorModule for MemoryRecoveryModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }
    fn committed_evidence_access(&self) -> CommittedEvidenceAccess {
        CommittedEvidenceAccess::OwnedHistory
    }
    async fn on_hook(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::TurnStart {
            let outcome = self.writer.drain_recovery(ctx, None).await?;
            self.writer
                .ensure_corrections_settled(ctx.session_id)
                .map_err(|error| ModuleError::Message(error.to_string()))?;
            Ok(outcome)
        } else {
            Ok(ModuleOutcome::continue_())
        }
    }
    async fn on_hook_authenticated(
        &self,
        hook: HookPoint,
        ctx: &ModuleContext<'_>,
        execution: &ToolExecutionContext,
    ) -> Result<ModuleOutcome, ModuleError> {
        if hook == HookPoint::TurnStart {
            let outcome = self
                .writer
                .drain_recovery_authenticated(ctx, execution)
                .await?;
            self.writer
                .ensure_corrections_settled(ctx.session_id)
                .map_err(|error| ModuleError::Message(error.to_string()))?;
            Ok(outcome)
        } else {
            Ok(ModuleOutcome::continue_())
        }
    }
}
