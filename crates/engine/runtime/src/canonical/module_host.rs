//! Registry lifecycle facade and dispatcher catalog views. No second registry.
use super::error::{RuntimeError, RuntimeResult};
use super::runtime::Runtime;
use apeireth_core::kernel::{CapabilityId, PluginId, SessionId};
use apeireth_plugin::{Plugin, PluginContext, ToolCapability};
use std::sync::Arc;

impl Runtime {
    /// Policy view, not another catalog. A session scope only restricts authority.
    pub fn tool_dispatcher(&self) -> &Arc<super::dispatcher::ToolDispatcher> {
        &self.dispatcher
    }

    pub fn tools_for_session(&self, session: SessionId) -> Vec<Arc<dyn ToolCapability>> {
        self.tools()
            .into_iter()
            .filter(|tool| self.dispatcher.allows(session, tool.id()))
            .collect()
    }

    pub fn tool_declarations_for_session(
        &self,
        session: SessionId,
    ) -> Vec<apeireth_protocol::canonical::NormalizedTool> {
        self.tool_declarations()
            .into_iter()
            .filter(|tool| {
                self.capabilities
                    .find_control_by_name(&tool.name)
                    .map(|control| self.dispatcher.allows(session, control.id()))
                    .or_else(|| {
                        self.tools()
                            .into_iter()
                            .find(|candidate| candidate.declaration().name == tool.name)
                            .map(|candidate| self.dispatcher.allows(session, candidate.id()))
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    pub(super) fn tool_is_legacy(&self, id: &CapabilityId) -> bool {
        self.plugins
            .record(id)
            .ok()
            .and_then(|record| self.plugins.plugins().get(record.owner).ok())
            .is_none_or(|plugin| !plugin.uses_tool_events_v1())
    }

    pub(super) fn resolve_tool(&self, name: &str) -> Option<Arc<dyn ToolCapability>> {
        let mut matches = self
            .tools()
            .into_iter()
            .filter(|tool| tool.declaration().name == name);
        let first = matches.next()?;
        if matches.next().is_some() {
            None
        } else {
            Some(first)
        }
    }

    /// Register a tool/hook module in the existing PluginManager. Provider routing
    /// is immutable for a Runtime; provider membership changes require a rebuild.
    pub async fn register_module(
        &mut self,
        plugin: Arc<dyn Plugin>,
        ctx: &PluginContext,
    ) -> RuntimeResult<()> {
        if !plugin.providers().is_empty() {
            return Err(RuntimeError::misconfigured(
                "provider membership changes require rebuilding the Runtime",
            ));
        }
        let id = plugin.manifest().id.clone();
        super::module::reject_tool_identity_collisions(
            &self.capabilities.capabilities(),
            &plugin.tools(),
            id.as_str(),
        )
        .map_err(RuntimeError::misconfigured)?;
        super::control::reject_control_tool_collisions(
            &self.capabilities.controls(),
            &plugin.tools(),
        )
        .map_err(RuntimeError::misconfigured)?;
        self.plugins.register(plugin)?;
        let result = match self.plugins.initialize(&id, ctx).await {
            Ok(()) => self.plugins.start(&id, ctx).await,
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            let _ = self.plugins.stop(&id).await;
            return Err(error.into());
        }
        Ok(())
    }

    /// Cancels work immediately; exclusive stop subsequently drains and cleans up.
    pub fn cancel_module(&self, id: &PluginId) -> RuntimeResult<()> {
        self.plugins.cancel(id)?;
        Ok(())
    }

    pub async fn stop_module(&mut self, id: &PluginId) -> RuntimeResult<()> {
        if !self.plugins.plugins().get(id)?.providers().is_empty() {
            return Err(RuntimeError::misconfigured(
                "stop provider modules through Runtime shutdown",
            ));
        }
        self.plugins.stop(id).await?;
        Ok(())
    }

    pub fn unregister_module(&mut self, id: &PluginId) -> RuntimeResult<()> {
        self.plugins.unregister(id)?;
        Ok(())
    }
}
