//! Serialized control mutations; model turns use immutable membership snapshots.
use super::*;
use apeireth_plugin::{Plugin, PluginContext};
impl AgentRuntime {
    pub async fn register_module(
        &self,
        plugin: Arc<dyn Plugin>,
        context: PluginContext,
    ) -> AgentResult<()> {
        if !self.inner.scope.is_open() {
            return Err(error(AgentErrorCode::OwnerStopped, "runtime stopped"));
        }
        let mut kernel =
            tokio::time::timeout(self.inner.policy.stop_timeout, self.inner.kernel.write())
                .await
                .map_err(|_| error(AgentErrorCode::NotReady, "module control deadline exceeded"))?;
        if !self.inner.scope.is_open() {
            return Err(error(
                AgentErrorCode::OwnerStopped,
                "runtime stopped while awaiting control",
            ));
        }
        // Context gets only the factory's per-module binding, never global Host authority.
        let context = context.with_agent_factory(self.inner.factory.clone());
        tokio::time::timeout(
            self.inner.policy.stop_timeout,
            kernel.register_module(plugin, &context),
        )
        .await
        .map_err(|_| {
            error(
                AgentErrorCode::NotReady,
                "module initialization deadline exceeded; owner retained",
            )
        })?
        .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))
    }
    pub async fn refresh_module_tools(&self, id: &PluginId) -> AgentResult<()> {
        if !self.inner.scope.is_open() {
            return Err(error(AgentErrorCode::OwnerStopped, "runtime stopped"));
        }
        let kernel =
            tokio::time::timeout(self.inner.policy.stop_timeout, self.inner.kernel.write())
                .await
                .map_err(|_| error(AgentErrorCode::NotReady, "module control deadline exceeded"))?;
        if !self.inner.scope.is_open() {
            return Err(error(
                AgentErrorCode::OwnerStopped,
                "runtime stopped while awaiting control",
            ));
        }
        tokio::time::timeout(
            self.inner.policy.stop_timeout,
            kernel.refresh_module_tools(id),
        )
        .await
        .map_err(|_| error(AgentErrorCode::NotReady, "module refresh deadline exceeded"))?
        .map_err(|e| error(AgentErrorCode::Provider, e.to_string()))
    }
}
