//! Compatibility accessors over the production resource factory.
use super::{McpModule, ProductionModules};
use apeireth_tools_canonical::mcp_bridge::{McpDefinition, McpToolBridge};
use std::sync::Arc;
impl ProductionModules {
    /// Configuration is available before Runtime initializes any resources.
    pub fn mcp_definition(&self) -> Option<&Arc<McpDefinition>> {
        self.mcp_definition.as_ref()
    }
    /// Available after Runtime initialization. Holding this Arc never grants lifecycle authority.
    /// The former pre-initialize configuration view is mcp_definition().config().
    pub fn mcp_bridge(&self) -> Option<Arc<McpToolBridge>> {
        self.mcp_definition
            .as_ref()
            .and_then(|definition| definition.current_bridge())
    }
    /// Legacy construction-only bag; production discovery uses the managed registry.
    pub fn mcp_module(&self) -> Option<&Arc<McpModule>> {
        self.mcp_module.as_ref()
    }
}
