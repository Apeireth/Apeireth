//! Lifecycle ownership of the existing dynamic bridge. No second tool catalog.
use super::McpToolBridge;
use apeireth_core::kernel::PluginId;
use apeireth_plugin::{
    Plugin, PluginContext, PluginError, PluginManifest, PluginResult, ToolCapability,
};
use async_trait::async_trait;
use std::sync::Arc;

pub struct McpPlugin {
    manifest: PluginManifest,
    bridge: Arc<McpToolBridge>,
}
impl McpPlugin {
    pub fn new(bridge: Arc<McpToolBridge>) -> Self {
        Self {
            manifest: PluginManifest::new(
                PluginId::new("module.mcp").expect("static id"),
                "1.0.0",
                "Owned dynamic tool bridge",
            ),
            bridge,
        }
    }
    fn error(&self, phase: &'static str, error: impl ToString) -> PluginError {
        PluginError::PluginFailed {
            plugin: self.manifest.id.clone(),
            phase,
            reason: error.to_string(),
        }
    }
}
#[async_trait]
impl Plugin for McpPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn required_at_boot(&self) -> bool {
        false
    }
    fn dynamic_tool_namespace(&self) -> Option<&str> {
        Some("tool.mcp.")
    }
    fn uses_tool_events_v1(&self) -> bool {
        true
    }
    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.bridge.tools()
    }
    async fn initialize(&self, _: &PluginContext) -> PluginResult<()> {
        self.bridge.reopen();
        Ok(())
    }
    async fn start(&self, _: &PluginContext) -> PluginResult<()> {
        self.refresh_tools().await
    }
    async fn refresh_tools(&self) -> PluginResult<()> {
        tokio::time::timeout(
            std::time::Duration::from_millis(self.bridge.options().max_timeout_ms),
            self.bridge.refresh(),
        )
        .await
        .map_err(|_| self.error("refresh", "connection/discovery deadline exceeded"))?
        .map(|_| ())
        .map_err(|e| self.error("refresh", e))
    }
    fn revoke_resources(&self) {
        self.bridge.revoke();
    }
    async fn shutdown(&self) -> PluginResult<()> {
        self.bridge.revoke();
        let mut failures = Vec::new();
        for connection in self.bridge.connections() {
            if let Err(error) = connection.close().await {
                failures.push(error.to_string());
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(self.error("shutdown", failures.join("; ")))
        }
    }
}
