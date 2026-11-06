//! Configuration-only definition. Every registration owns fresh connections and catalog.
use super::module::McpPlugin;
use super::transport::McpChannelFactory;
use super::{McpBridgeOptions, McpServerConfig, McpServerConnection, McpToolBridge};
use apeireth_plugin::resource_factory::{ModuleDefinition, ModuleResources, OwnedModuleResource};
use apeireth_plugin::{Plugin, PluginContext, PluginError, PluginManifest, PluginResult};
use async_trait::async_trait;
use std::sync::{Arc, Mutex, Weak};

pub struct McpDefinition {
    manifest: PluginManifest,
    config: McpServerConfig,
    options: McpBridgeOptions,
    factories: Option<Vec<Arc<dyn McpChannelFactory>>>,
    current: Mutex<Weak<McpToolBridge>>,
}
impl McpDefinition {
    pub fn new(
        config: McpServerConfig,
        options: McpBridgeOptions,
    ) -> Result<Self, super::McpBridgeError> {
        // Validate without acquiring a connection or starting a task.
        let bridge = Arc::new(McpToolBridge::new(config.clone(), options.clone())?);
        Ok(Self {
            manifest: McpPlugin::new(bridge).manifest().clone(),
            config,
            options,
            factories: None,
            current: Mutex::new(Weak::new()),
        })
    }
    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }
    /// Observational reference only; stop/restart revoke old resources independently.
    pub fn current_bridge(&self) -> Option<Arc<McpToolBridge>> {
        self.current
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .upgrade()
    }
    pub fn startup_log(&self) -> Vec<String> {
        self.config.redacted_startup_log()
    }
    /// Explicit host-injected transports. Factories must return fresh owned channels.
    pub fn with_channel_factories(
        mut self,
        factories: Vec<Arc<dyn McpChannelFactory>>,
    ) -> PluginResult<Self> {
        if factories.len() != self.config.enabled_servers().count() {
            return Err(self.error("factory count differs from enabled server count"));
        }
        self.factories = Some(factories);
        Ok(self)
    }
    fn error(&self, error: impl ToString) -> PluginError {
        PluginError::PluginFailed {
            plugin: self.manifest.id.clone(),
            phase: "factory",
            reason: error.to_string(),
        }
    }
}
struct BridgeResource(Arc<McpToolBridge>);
#[async_trait]
impl OwnedModuleResource for BridgeResource {
    fn revoke(&self) {
        self.0.revoke();
    }
    async fn close(&self) -> Result<(), String> {
        self.revoke();
        let mut errors = vec![];
        for connection in self.0.connections() {
            if let Err(error) = connection.close().await {
                errors.push(error.to_string());
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}
#[async_trait]
impl ModuleDefinition for McpDefinition {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }
    fn required_at_boot(&self) -> bool {
        false
    }
    fn dynamic_tool_namespace(&self) -> Option<&str> {
        Some("tool.mcp.")
    }
    async fn instantiate(
        &self,
        _: &PluginContext,
        resources: &ModuleResources,
    ) -> PluginResult<Arc<dyn Plugin>> {
        let bridge = if let Some(factories) = &self.factories {
            let connections = self
                .config
                .enabled_servers()
                .zip(factories)
                .map(|(spec, factory)| {
                    Arc::new(McpServerConnection::new(
                        spec.clone(),
                        factory.clone(),
                        &self.options,
                    ))
                })
                .collect();
            McpToolBridge::from_connections(self.config.clone(), self.options.clone(), connections)
        } else {
            McpToolBridge::new(self.config.clone(), self.options.clone())
        }
        .map_err(|e| self.error(e))?;
        let bridge = Arc::new(bridge);
        resources
            .own("mcp.bridge", Arc::new(BridgeResource(bridge.clone())))
            .map_err(|e| self.error(e))?;
        *self.current.lock().unwrap_or_else(|e| e.into_inner()) = Arc::downgrade(&bridge);
        Ok(Arc::new(McpPlugin::new(bridge)))
    }
}
