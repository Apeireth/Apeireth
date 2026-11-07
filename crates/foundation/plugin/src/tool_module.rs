//! One ordinary lifecycle module owning a selectable set of tools.
use crate::{CapabilityKind, Plugin, PluginContext, PluginManifest, PluginResult, ToolCapability};
use apeireth_core::kernel::PluginId;
use async_trait::async_trait;
use std::sync::Arc;

pub struct ToolModule {
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
    events_v1: bool,
}
impl ToolModule {
    pub fn new(id: PluginId, tools: Vec<Arc<dyn ToolCapability>>) -> PluginResult<Self> {
        let mut manifest = PluginManifest::new(id, "1.0.0", "Registered tool capability module");
        for tool in &tools {
            manifest = manifest.declare_capability(
                tool.id().clone(),
                CapabilityKind::Tool,
                tool.declaration().description.unwrap_or_default(),
            )?;
        }
        for tool in &tools {
            let contract = tool.module_contract();
            if contract != crate::module_contract::ModuleToolContract::default() {
                let descriptor = manifest
                    .capabilities
                    .iter_mut()
                    .find(|d| &d.id == tool.id())
                    .expect("declared tool");
                *descriptor = descriptor
                    .clone()
                    .with_module_tool_contract(&contract)
                    .map_err(|reason| {
                        crate::PluginError::init_failed(manifest.id.clone(), reason)
                    })?;
            }
        }
        Ok(Self {
            manifest,
            tools,
            events_v1: true,
        })
    }
    /// Opt into a logical surface backed by the existing native executor.
    /// The host must explicitly select this contract; old modules keep their ABI.
    pub fn with_public_contract(
        self,
        mode: crate::module_contract::ExecutionMode,
    ) -> PluginResult<Self> {
        use crate::module_contract::{EffectClass, ExecutionMode, ModuleToolContract};
        let effect = if matches!(mode, ExecutionMode::Agentic) {
            EffectClass::Orchestration
        } else {
            EffectClass::Protected
        };
        let tools = self
            .tools
            .into_iter()
            .map(|tool| {
                crate::module_contract::ContractTool::wrap(
                    tool,
                    ModuleToolContract::logical(mode, effect),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|reason| crate::PluginError::init_failed(self.manifest.id.clone(), reason))?;
        let mut module = Self::new(self.manifest.id, tools)?;
        module.events_v1 = self.events_v1;
        Ok(module)
    }

    /// Preserve old durable-log ABI only for explicit legacy host adapters.
    pub fn with_legacy_events(mut self) -> Self {
        self.events_v1 = false;
        self
    }
}

/// A module whose tools were elected on to an explicit contract at assembly time.
///
/// Lifecycle, wire name, frozen payload and every executor decision stay the
/// inner module's; only the declared/executable contract is replaced. Election
/// restricts a surface — it never grants authority.
struct ElectedContractModule {
    inner: Arc<dyn Plugin>,
    manifest: PluginManifest,
    tools: Vec<Arc<dyn ToolCapability>>,
}

/// Elect one explicit contract for every tool `inner` implements.
///
/// Fails closed when the module did not declare a tool it implements (a manifest
/// must never silently disagree with the executable surface), when the contract
/// is invalid, or when the module owns a dynamic tool namespace (its descriptor
/// set can change later, so a static election would go stale).
pub fn elect_module_contract(
    inner: Arc<dyn Plugin>,
    contract: &crate::module_contract::ModuleToolContract,
) -> Result<Arc<dyn Plugin>, String> {
    use crate::module_contract::ContractTool;
    contract.validate()?;
    if inner.dynamic_tool_namespace().is_some() {
        return Err(format!(
            "module {} owns a dynamic tool namespace; its contract cannot be elected statically",
            inner.manifest().id
        ));
    }
    let tools = inner.tools();
    if tools.is_empty() {
        return Ok(inner);
    }
    let mut manifest = inner.manifest().clone();
    let mut elected = Vec::with_capacity(tools.len());
    for tool in tools {
        let descriptor = manifest
            .capabilities
            .iter_mut()
            .find(|descriptor| &descriptor.id == tool.id())
            .ok_or_else(|| format!("module {} did not declare {}", manifest.id, tool.id()))?;
        *descriptor = descriptor.clone().with_module_tool_contract(contract)?;
        elected.push(ContractTool::wrap(tool, contract.clone())?);
    }
    Ok(Arc::new(ElectedContractModule {
        inner,
        manifest,
        tools: elected,
    }))
}

#[async_trait]
impl Plugin for ElectedContractModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }

    fn providers(&self) -> Vec<Arc<dyn crate::provider::ProviderCapability>> {
        self.inner.providers()
    }

    fn hooks(&self) -> Vec<apeireth_protocol::canonical::ModuleHookV1> {
        self.inner.hooks()
    }

    fn uses_tool_events_v1(&self) -> bool {
        self.inner.uses_tool_events_v1()
    }

    fn required_at_boot(&self) -> bool {
        self.inner.required_at_boot()
    }

    fn deferred_capabilities(&self) -> bool {
        self.inner.deferred_capabilities()
    }

    fn dynamic_tool_namespace(&self) -> Option<&str> {
        self.inner.dynamic_tool_namespace()
    }

    fn revoke_resources(&self) {
        self.inner.revoke_resources();
    }

    fn resource_statuses(&self) -> Vec<crate::resource_factory::ResourceStatus> {
        self.inner.resource_statuses()
    }

    async fn initialize(&self, ctx: &PluginContext) -> PluginResult<()> {
        self.inner.initialize(ctx).await
    }

    async fn start(&self, ctx: &PluginContext) -> PluginResult<()> {
        self.inner.start(ctx).await
    }

    async fn refresh_tools(&self) -> PluginResult<()> {
        self.inner.refresh_tools().await
    }

    async fn on_hook(
        &self,
        ctx: &crate::hooks::HookContextV1,
    ) -> PluginResult<Vec<crate::hooks::HookToolRequest>> {
        self.inner.on_hook(ctx).await
    }

    async fn shutdown(&self) -> PluginResult<()> {
        self.inner.shutdown().await
    }
}
#[async_trait]
impl Plugin for ToolModule {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.clone()
    }
    fn uses_tool_events_v1(&self) -> bool {
        self.events_v1
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        Ok(())
    }
    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }
}
