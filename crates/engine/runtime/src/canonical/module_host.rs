//! Registry lifecycle facade and dispatcher catalog views. No second registry.
use super::error::{RuntimeError, RuntimeResult};
use super::runtime::{Runtime, RuntimeBuilder};
use apeireth_core::kernel::{CapabilityId, PluginId, SessionId};
use apeireth_plugin::{Plugin, PluginContext, ToolCapability};
use std::sync::Arc;

impl Runtime {
    /// Every active tool is projected from the one executable catalog.
    pub fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.capabilities
            .capabilities()
            .into_iter()
            .filter(|tool| tool.model_visible())
            .map(|tool| {
                let registration = self
                    .capabilities
                    .registration(tool.id())
                    .expect("catalog entry");
                self.dispatcher.adapt_owned(
                    tool,
                    registration.scope,
                    registration.legacy,
                    registration
                        .module
                        .unwrap_or_else(|| PluginId::new("host.legacy").expect("static id")),
                )
            })
            .collect()
    }

    pub(super) fn uses_v1_tool_events(&self) -> bool {
        self.v1_protocol
            || self.plugins.plugins().ids().any(|id| {
                self.plugins
                    .plugins()
                    .get(id)
                    .is_ok_and(|plugin| plugin.uses_tool_events_v1())
            })
    }

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
        self.capabilities
            .registration(id)
            .is_none_or(|entry| entry.legacy)
    }

    pub(super) fn resolve_tool(&self, name: &str) -> Option<Arc<dyn ToolCapability>> {
        self.capabilities.find_by_name(name)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn bind_admitted_tool(
        &self,
        tool: Arc<dyn ToolCapability>,
        call: &apeireth_protocol::canonical::ToolCall,
        session: SessionId,
        request: apeireth_core::kernel::RequestId,
        trace: apeireth_core::kernel::TraceId,
        security: Option<&apeireth_governance::TurnSecurityContext>,
        approved: bool,
        automatic: bool,
    ) -> Result<Arc<dyn ToolCapability>, apeireth_protocol::canonical::ToolResult> {
        let registration = self.capabilities.registration(tool.id()).ok_or_else(|| {
            super::dispatcher::failure(
                call,
                super::dispatcher::DispatchErrorCode::Unregistered,
                "tool was unregistered before execution",
            )
        })?;
        let module = registration
            .module
            .clone()
            .unwrap_or_else(|| PluginId::new("host.legacy").expect("static id"));
        let timeout = tool
            .execution_policy()
            .timeout
            .unwrap_or(self.dispatcher.limits.timeout)
            .min(self.dispatcher.limits.timeout);
        let context = apeireth_plugin::ToolExecutionContext {
            agent: self.dispatcher.agent(session),
            principal: self.dispatcher.principal(session),
            session: Some(session),
            turn: Some(request),
            call_id: call.id.clone(),
            caller: if automatic {
                apeireth_plugin::ToolCaller::Module(module.clone())
            } else {
                apeireth_plugin::ToolCaller::Model
            },
            module,
            tool_scope: self.dispatcher.scope_snapshot(session),
            authorization: apeireth_plugin::ToolAuthorization {
                intent_id: security.map(|s| s.intent_id.clone()),
                authorization_context: security.and_then(|s| s.authorization_context.clone()),
                approved,
            },
            cancellation: Default::default(),
            deadline: std::time::Instant::now()
                .checked_add(timeout)
                .unwrap_or_else(std::time::Instant::now),
            trace: Some(trace),
            module_tasks: registration.scope.clone(),
            resource_lease: None,
        };
        Ok(self
            .dispatcher
            .adapt_context(tool, registration.scope, registration.legacy, context))
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
        if let Err(error) = self.plugins.validate_current_implementations(&id) {
            let _ = self.plugins.stop(&id).await;
            return Err(error.into());
        }
        let plugin = self.plugins.plugins().get(&id)?;
        if let Err(error) = self.capabilities.register_module(
            &id,
            plugin.tools(),
            self.plugins.task_scope(&id)?,
            !plugin.uses_tool_events_v1(),
        ) {
            let _ = self.plugins.stop(&id).await;
            return Err(RuntimeError::misconfigured(error));
        }
        Ok(())
    }

    /// Refresh a live module without replacing the registry or permission gate.
    pub async fn refresh_module_tools(&self, id: &PluginId) -> RuntimeResult<()> {
        let plugin = self.plugins.plugins().get(id)?.clone();
        if plugin.dynamic_tool_namespace().is_none() {
            return Err(RuntimeError::misconfigured(
                "module has no dynamic tool namespace",
            ));
        }
        let scope = self.plugins.task_scope(id)?;
        tokio::time::timeout(
            self.dispatcher.limits.timeout,
            scope.run(plugin.refresh_tools()),
        )
        .await
        .map_err(|_| RuntimeError::misconfigured("dynamic refresh deadline exceeded"))?
        .map_err(|_| RuntimeError::misconfigured("module stopping"))??;
        self.plugins.validate_current_implementations(id)?;
        self.capabilities
            .replace_module_tools(id, plugin.tools(), scope)
            .map_err(RuntimeError::misconfigured)
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
        self.plugins
            .stop_until(id, tokio::time::sleep(std::time::Duration::from_secs(30)))
            .await?;
        Ok(())
    }

    pub fn unregister_module(&mut self, id: &PluginId) -> RuntimeResult<()> {
        self.plugins.unregister(id)?;
        self.capabilities.remove_module(id);
        Ok(())
    }
}

impl RuntimeBuilder {
    /// Host-selected capabilities use the ordinary Plugin lifecycle.
    pub fn with_tool_module(
        self,
        provider: &dyn super::capability::CapabilityProvider,
    ) -> RuntimeResult<Self> {
        let module = provider
            .tool_module()
            .map_err(RuntimeError::misconfigured)?;
        Ok(self.with_plugin(Arc::new(module)))
    }
}

pub(super) async fn populate_tool_catalog(
    catalog: &super::capability::CapabilityRegistry,
    manager: &mut apeireth_plugin::PluginManager,
) -> RuntimeResult<()> {
    let ids = manager.plugins().ids().cloned().collect::<Vec<_>>();
    for id in ids {
        if manager.state(&id)? != apeireth_core::kernel::Lifecycle::Active {
            continue;
        }
        let result = (|| -> RuntimeResult<()> {
            manager.validate_current_implementations(&id)?;
            let plugin = manager.plugins().get(&id)?;
            catalog
                .register_module(
                    &id,
                    plugin.tools(),
                    manager.task_scope(&id)?,
                    !plugin.uses_tool_events_v1(),
                )
                .map_err(RuntimeError::misconfigured)
        })();
        if let Err(error) = result {
            manager.shutdown_all().await;
            return Err(error);
        }
    }
    Ok(())
}
