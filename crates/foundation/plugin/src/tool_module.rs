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
