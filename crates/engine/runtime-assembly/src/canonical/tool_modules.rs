//! Canonical module wrappers for builtin tool capabilities and MCP.
//!
//! After the Module/Plugin convergence there are exactly two kinds of module
//! here:
//!
//! - [`BuiltinToolsModule`] — one owner for every builtin tool capability
//!   (`tool.filesystem`, `tool.search`, `tool.repo`, and the opt-in
//!   `tool.shell` / `tool.fetch` / `tool.education`). It is the aggregate the
//!   tools crate already owned as a "plugin": one unit of lifecycle with many
//!   capabilities, rather than one module per tool.
//! - [`McpModule`] — the one module with genuine *dynamic* capability
//!   ownership. An MCP server discovers tools at runtime, so this module can
//!   grow its export set after boot, through the same
//!   [`Module::add_tool`] path every other module is checked against.
//!
//! Both register through the ordinary module path: same manifest schema, same
//! lifecycle, same capability index, same duplicate-identity discipline. There
//! is no capability registry beside the module registry.

use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::{
    CapabilityOwnership, Module, ModuleManifest, ModuleSystemError, PluginResult, ToolCapability,
};

pub use apeireth_tools_canonical::BuiltinToolsModule;

/// Module id of the MCP capability owner (matches the historical `module.mcp`
/// owner string so existing admin panels and configuration keep resolving).
pub const MCP_MODULE_ID: &str = "module.mcp";

/// Module that manages and dynamically contributes Model Context Protocol (MCP)
/// tool capabilities.
///
/// The only module in the runtime whose capability set can grow after boot.
/// Dynamic tools are still admitted by the module *manager*, which checks the
/// capability id and the model-facing name against every module's exports
/// first; this module's own bag check is the second, module-local line of
/// defence. A tool that is in the bag but was never admitted by the manager is
/// not dispatchable and does not appear in any listing.
pub struct McpModule {
    manifest: ModuleManifest,
    tools: std::sync::RwLock<Vec<Arc<dyn ToolCapability>>>,
}

impl McpModule {
    /// Create a new MCP module.
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest::system(
                MCP_MODULE_ID,
                env!("CARGO_PKG_VERSION"),
                "Model Context Protocol tools discovered at runtime",
            )
            .with_name("mcp")
            // Dynamic ownership is declared in the manifest — the one place the
            // manager reads it from. This is the only module that declares it.
            .with_capability_ownership(CapabilityOwnership::Dynamic),
            tools: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Add a tool capability provided by an MCP source during initialization.
    pub fn with_tool(self, tool: Arc<dyn ToolCapability>) -> Result<Self, String> {
        self.register_tool(tool)?;
        Ok(self)
    }

    /// Register a dynamic tool capability provided by an MCP source.
    ///
    /// Rejects duplicate capability ids and duplicate model-facing names inside
    /// this module. Cross-module collisions are rejected by the module manager,
    /// which owns the single capability index and checks every module.
    pub fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        self.admit_tool(tool).map_err(|error| error.to_string())
    }

    /// Typed admission into the module-local bag.
    ///
    /// The manager performs the same two checks across every module before it
    /// asks; this is the module-local half, so a bag cannot disagree with itself
    /// even if it is filled directly.
    fn admit_tool(&self, tool: Arc<dyn ToolCapability>) -> PluginResult<()> {
        // poison 容错 (2026-09-24 修复轮): 锁保护的是可重建的工具注册表,
        // 持锁线程 panic 不应让后续注册/查询连锁 panic。
        let mut tools = self
            .tools
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if tools.iter().any(|t| t.id() == tool.id()) {
            return Err(ModuleSystemError::DuplicateCapability {
                capability: tool.id().clone(),
                // Both claimants are this module: the bag collided with itself,
                // which the manager would have caught first anyway.
                incumbent: self.manifest.id.clone(),
                challenger: self.manifest.id.clone(),
            });
        }
        if let Some(incumbent) = tools
            .iter()
            .find(|t| t.declaration().name == tool.declaration().name)
        {
            return Err(ModuleSystemError::AmbiguousToolName {
                name: tool.declaration().name,
                incumbent: incumbent.id().clone(),
                challenger: tool.id().clone(),
            });
        }
        tools.push(tool);
        Ok(())
    }

    /// Unregister a dynamic tool capability by capability ID.
    ///
    /// Only tools whose id matches are removed; other owners are untouched.
    pub fn unregister_tool(&self, capability_id: &CapabilityId) {
        self.tools
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|t| t.id() != capability_id);
    }

    /// Tools this module currently holds, admitted or not.
    pub fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl Default for McpModule {
    fn default() -> Self {
        Self::new()
    }
}

/// The unified module surface: manifest, lifecycle, and the live export set.
///
/// `tools()` is read live rather than snapshotted at construction, which is
/// what lets the manager's index resolve a tool admitted after boot.
#[async_trait::async_trait]
impl Module for McpModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools()
    }

    async fn add_tool(&self, tool: Arc<dyn ToolCapability>) -> PluginResult<()> {
        self.admit_tool(tool)
    }

    async fn remove_tool(&self, id: &CapabilityId) -> PluginResult<bool> {
        let mut tools = self.tools.write().expect("mcp lock poisoned");
        let before = tools.len();
        tools.retain(|tool| tool.id() != id);
        Ok(tools.len() != before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::ModuleInitContext;
    use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
    use async_trait::async_trait;

    struct FakeTool {
        id: CapabilityId,
        name: &'static str,
    }

    #[async_trait]
    impl ToolCapability for FakeTool {
        fn id(&self) -> &CapabilityId {
            &self.id
        }
        fn declaration(&self) -> NormalizedTool {
            NormalizedTool {
                name: self.name.into(),
                description: Some("fake".into()),
                parameters: ToolParameters::new(),
                strict: false,
            }
        }
        async fn invoke(&self, call: &ToolCall) -> ToolResult {
            ToolResult::ok(&call.id, serde_json::json!({}))
        }
    }

    fn tool(id: &str, name: &'static str) -> Arc<dyn ToolCapability> {
        Arc::new(FakeTool {
            id: CapabilityId::new(id).unwrap(),
            name,
        })
    }

    #[tokio::test]
    async fn a_dynamic_tool_is_visible_through_the_module_surface() {
        let module = McpModule::new();
        assert!(module.tools().is_empty());
        module
            .add_tool(tool("tool.mcp.custom_search", "mcp_custom_search"))
            .await
            .expect("dynamic tool admitted");
        assert_eq!(module.tools().len(), 1);
        assert_eq!(module.manifest().id.as_str(), MCP_MODULE_ID);

        // A refused duplicate leaves the bag untouched.
        assert!(module
            .add_tool(tool("tool.mcp.custom_search", "other_name"))
            .await
            .is_err());
        assert!(module
            .add_tool(tool("tool.mcp.imposter", "mcp_custom_search"))
            .await
            .is_err());
        assert_eq!(module.tools().len(), 1);

        assert!(module
            .remove_tool(&CapabilityId::new("tool.mcp.custom_search").unwrap())
            .await
            .unwrap());
        assert!(module.tools().is_empty());
        assert!(!module
            .remove_tool(&CapabilityId::new("tool.mcp.custom_search").unwrap())
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn the_module_declares_no_static_capabilities() {
        // Nothing is declared up front: every capability arrives dynamically, so
        // the manifest starts empty and the index follows the live bag.
        let module = McpModule::new();
        assert!(module.manifest().capabilities.is_empty());
        let _ = ModuleInitContext::new(
            apeireth_core::kernel::system_clock(),
            Arc::new(apeireth_plugin::NoCredentials),
            apeireth_core::kernel::TraceId::new(),
        );
    }
}
