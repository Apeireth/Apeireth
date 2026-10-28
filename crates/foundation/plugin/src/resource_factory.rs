//! Fresh per-registration resources under the existing PluginManager lifecycle.
use crate::{Plugin, PluginContext, PluginError, PluginManifest, PluginResult, ToolCapability};
use apeireth_core::kernel::{CapabilityId, ModuleInstanceId, SessionId};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Read-only authority issued by the registry. A revoked generation never reopens.
#[derive(Clone)]
pub struct ModuleActivation {
    instance: ModuleInstanceId,
    running: Arc<AtomicBool>,
}
impl ModuleActivation {
    pub(crate) fn new(instance: ModuleInstanceId) -> Self {
        Self {
            instance,
            running: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn instance(&self) -> ModuleInstanceId {
        self.instance
    }
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }
    pub(crate) fn activate(&self) {
        self.running.store(true, Ordering::Release);
    }
    pub(crate) fn revoke(&self) {
        self.running.store(false, Ordering::Release);
    }
}

/// Close must be idempotent and cancellation-safe; revoke cannot await external Arcs.
#[async_trait]
pub trait OwnedModuleResource: Send + Sync {
    fn revoke(&self);
    async fn close(&self) -> Result<(), String>;
}
struct ResourceEntry {
    key: String,
    resource: Arc<dyn OwnedModuleResource>,
    closed: bool,
    error: Option<String>,
}
/// One instance's resources. Register ownership immediately after acquisition.
#[derive(Default)]
pub struct ModuleResources {
    entries: Mutex<Vec<ResourceEntry>>,
    revoked: AtomicBool,
    closing: futures::lock::Mutex<()>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceStatus {
    pub key: String,
    pub closed: bool,
    pub error: Option<String>,
}
impl ModuleResources {
    pub fn own(
        &self,
        key: impl Into<String>,
        resource: Arc<dyn OwnedModuleResource>,
    ) -> Result<(), String> {
        let key = key.into();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if self.revoked.load(Ordering::Acquire) {
            resource.revoke();
            return Err("resource owner stopped".into());
        }
        if entries.iter().any(|entry| entry.key == key) {
            return Err(format!("duplicate resource {key}"));
        }
        entries.push(ResourceEntry {
            key,
            resource,
            closed: false,
            error: None,
        });
        Ok(())
    }
    pub fn statuses(&self) -> Vec<ResourceStatus> {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|e| ResourceStatus {
                key: e.key.clone(),
                closed: e.closed,
                error: e.error.clone(),
            })
            .collect()
    }
    pub fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
        let resources: Vec<_> = self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .map(|e| e.resource.clone())
            .collect();
        for resource in resources {
            resource.revoke();
        }
    }
    /// Reverse acquisition order. Failed closes stay owned and can be retried.
    pub async fn close(&self) -> Result<(), String> {
        self.revoke();
        let _closing = self.closing.lock().await;
        let pending: Vec<_> = self
            .entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, entry)| !entry.closed)
            .map(|(i, e)| (i, e.resource.clone()))
            .collect();
        let mut failures = Vec::new();
        for (index, resource) in pending {
            let result = resource.close().await;
            let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
            let entry = &mut entries[index];
            entry.closed = result.is_ok();
            entry.error = result.err();
            if let Some(error) = &entry.error {
                failures.push(format!("{}: {error}", entry.key));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures.join("; "))
        }
    }
}
impl Drop for ModuleResources {
    fn drop(&mut self) {
        self.revoke();
    }
}

/// A definition contains configuration, not open connections or running tasks.
#[async_trait]
pub trait ModuleDefinition: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn required_at_boot(&self) -> bool {
        true
    }
    fn dynamic_tool_namespace(&self) -> Option<&str> {
        None
    }
    async fn instantiate(
        &self,
        context: &PluginContext,
        resources: &ModuleResources,
    ) -> PluginResult<Arc<dyn Plugin>>;
}

/// Pass this ordinary plugin to existing with_plugin/register_module APIs.
/// Every call creates a separate instance adapter; only the definition is shared.
pub fn module_from_definition(definition: Arc<dyn ModuleDefinition>) -> Arc<dyn Plugin> {
    Arc::new(FactoryModule {
        definition,
        instance: Mutex::new(None),
        resources: ModuleResources::default(),
    })
}
struct Instance {
    plugin: Arc<dyn Plugin>,
    activation: ModuleActivation,
    tasks: crate::task_scope::ModuleTaskScope,
}
struct FactoryModule {
    definition: Arc<dyn ModuleDefinition>,
    instance: Mutex<Option<Instance>>,
    resources: ModuleResources,
}
impl FactoryModule {
    fn error(&self, phase: &'static str, reason: impl ToString) -> PluginError {
        PluginError::PluginFailed {
            plugin: self.manifest().id.clone(),
            phase,
            reason: reason.to_string(),
        }
    }
    fn instance(&self) -> PluginResult<Arc<dyn Plugin>> {
        self.instance
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|i| i.plugin.clone())
            .ok_or_else(|| self.error("factory", "instance not initialized"))
    }
}
#[async_trait]
impl Plugin for FactoryModule {
    fn manifest(&self) -> &PluginManifest {
        self.definition.manifest()
    }
    fn required_at_boot(&self) -> bool {
        self.definition.required_at_boot()
    }
    fn dynamic_tool_namespace(&self) -> Option<&str> {
        self.definition.dynamic_tool_namespace()
    }
    fn resource_statuses(&self) -> Vec<ResourceStatus> {
        self.resources.statuses()
    }
    fn deferred_capabilities(&self) -> bool {
        true
    }
    fn uses_tool_events_v1(&self) -> bool {
        true
    }
    fn hooks(&self) -> Vec<apeireth_protocol::canonical::ModuleHookV1> {
        self.instance().map(|i| i.hooks()).unwrap_or_default()
    }
    async fn on_hook(
        &self,
        ctx: &crate::hooks::HookContextV1,
    ) -> PluginResult<Vec<crate::hooks::HookToolRequest>> {
        self.instance()?.on_hook(ctx).await
    }
    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        if self
            .instance
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
        {
            return Err(self.error("initialize", "instance cannot be reused"));
        }
        let activation = ctx
            .activation
            .clone()
            .ok_or_else(|| self.error("initialize", "factory requires registry context"))?;
        let plugin = self.definition.instantiate(ctx, &self.resources).await?;
        // Keep even an invalid instance owned so manager rollback can release it.
        *self.instance.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instance {
            plugin: plugin.clone(),
            activation,
            tasks: ctx.tasks.clone(),
        });
        if plugin.manifest() != self.manifest() {
            return Err(self.error("initialize", "instance manifest differs from definition"));
        }
        plugin.initialize(ctx).await
    }
    async fn start(&self, ctx: &PluginContext) -> PluginResult<()> {
        self.instance()?.start(ctx).await
    }
    fn revoke_resources(&self) {
        self.resources.revoke();
        if let Ok(instance) = self.instance() {
            instance.revoke_resources();
        }
    }
    async fn shutdown(&self) -> PluginResult<()> {
        self.revoke_resources();
        let instance_result = match self.instance() {
            Ok(instance) => instance.shutdown().await,
            Err(_) => Ok(()),
        };
        let resource_result = self
            .resources
            .close()
            .await
            .map_err(|e| self.error("resources", e));
        instance_result.and(resource_result)
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        let lock = self.instance.lock().unwrap_or_else(|e| e.into_inner());
        let Some(instance) = lock.as_ref() else {
            return vec![];
        };
        instance
            .plugin
            .tools()
            .into_iter()
            .map(|tool| {
                Arc::new(InstanceTool {
                    tool,
                    activation: instance.activation.clone(),
                    tasks: instance.tasks.clone(),
                }) as Arc<dyn ToolCapability>
            })
            .collect()
    }
    fn providers(&self) -> Vec<Arc<dyn crate::provider::ProviderCapability>> {
        self.instance().map(|i| i.providers()).unwrap_or_default()
    }
    async fn refresh_tools(&self) -> PluginResult<()> {
        self.instance()?.refresh_tools().await
    }
}

struct InstanceTool {
    tool: Arc<dyn ToolCapability>,
    activation: ModuleActivation,
    tasks: crate::task_scope::ModuleTaskScope,
}
impl InstanceTool {
    async fn execute(
        &self,
        call: &ToolCall,
        work: impl std::future::Future<Output = ToolResult> + Send,
    ) -> ToolResult {
        if !self.activation.is_running() || !self.tasks.is_open() {
            return ToolResult::permanent_error(&call.id, "module instance is not running");
        }
        self.tasks
            .run(work)
            .await
            .unwrap_or_else(|_| ToolResult::permanent_error(&call.id, "module instance stopped"))
    }
}
#[async_trait]
impl ToolCapability for InstanceTool {
    fn id(&self) -> &CapabilityId {
        self.tool.id()
    }
    fn declaration(&self) -> NormalizedTool {
        self.tool.declaration()
    }
    fn model_visible(&self) -> bool {
        self.tool.model_visible()
    }
    fn module_result_delivery(
        &self,
        r: &ToolResult,
    ) -> crate::tool_execution::ModuleResultDelivery {
        self.tool.module_result_delivery(r)
    }
    fn execution_policy(&self) -> crate::ToolExecutionPolicy {
        self.tool.execution_policy()
    }
    fn freeze_invocation(
        &self,
        call: &ToolCall,
    ) -> Result<Option<crate::tool::FrozenInvocation>, ToolResult> {
        if !self.activation.is_running() || !self.tasks.is_open() {
            return Err(ToolResult::permanent_error(
                &call.id,
                "module instance is not running",
            ));
        }
        self.tool.freeze_invocation(call)
    }
    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.execute(call, self.tool.invoke(call)).await
    }
    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&crate::tool::FrozenInvocation>,
    ) -> ToolResult {
        self.execute(call, self.tool.invoke_frozen(call, frozen))
            .await
    }
    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        frozen: Option<&crate::tool::FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        self.execute(call, self.tool.invoke_in_session(call, frozen, session))
            .await
    }
    async fn invoke_with_context(
        &self,
        call: &ToolCall,
        frozen: Option<&crate::tool::FrozenInvocation>,
        context: &crate::ToolExecutionContext,
    ) -> ToolResult {
        self.execute(call, self.tool.invoke_with_context(call, frozen, context))
            .await
    }
}
