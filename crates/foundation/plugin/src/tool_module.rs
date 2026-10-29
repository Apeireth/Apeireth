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
        Ok(Self {
            manifest,
            tools,
            events_v1: true,
        })
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
