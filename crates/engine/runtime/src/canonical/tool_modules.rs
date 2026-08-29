//! Canonical module wrappers for builtin tool capabilities and MCP.
//!
//! Tools are owned by modules; the microkernel dispatches to them through the
//! unified module/capability registry without hardcoding tool names.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::{
    FetchConfig, FetchTool, FilesystemTool, RepoTool, SearchTool, ShellTool, TrustedShellConfig,
};

use super::module::{Module, ModuleManifest};

/// Module providing filesystem capabilities (`tool.filesystem`).
pub struct FilesystemModule {
    manifest: ModuleManifest,
    tool: Arc<FilesystemTool>,
}

impl FilesystemModule {
    /// Create a filesystem module rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            manifest: ModuleManifest::new("module.tool.filesystem", "Filesystem Tool Module"),
            tool: Arc::new(FilesystemTool::new(workspace_root)),
        }
    }

    /// Access the underlying filesystem tool.
    pub fn tool(&self) -> &Arc<FilesystemTool> {
        &self.tool
    }
}

impl Module for FilesystemModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing search capabilities (`tool.search`).
pub struct SearchModule {
    manifest: ModuleManifest,
    tool: Arc<SearchTool>,
}

impl SearchModule {
    /// Create a search module rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            manifest: ModuleManifest::new("module.tool.search", "Search Tool Module"),
            tool: Arc::new(SearchTool::new(workspace_root)),
        }
    }

    /// Access the underlying search tool.
    pub fn tool(&self) -> &Arc<SearchTool> {
        &self.tool
    }
}

impl Module for SearchModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing git repository inspection capabilities (`tool.repo`).
pub struct RepoModule {
    manifest: ModuleManifest,
    tool: Arc<RepoTool>,
}

impl RepoModule {
    /// Create a repo module rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            manifest: ModuleManifest::new("module.tool.repo", "Repository Tool Module"),
            tool: Arc::new(RepoTool::new(workspace_root)),
        }
    }

    /// Access the underlying repo tool.
    pub fn tool(&self) -> &Arc<RepoTool> {
        &self.tool
    }
}

impl Module for RepoModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing trusted shell command execution capabilities (`tool.shell`).
pub struct ShellModule {
    manifest: ModuleManifest,
    tool: Arc<ShellTool>,
}

impl ShellModule {
    /// Create a shell module with explicit configuration.
    pub fn new(config: TrustedShellConfig) -> Self {
        Self {
            manifest: ModuleManifest::new("module.tool.shell", "Trusted Shell Tool Module"),
            tool: Arc::new(ShellTool::new(config)),
        }
    }

    /// Access the underlying shell tool.
    pub fn tool(&self) -> &Arc<ShellTool> {
        &self.tool
    }
}

impl Module for ShellModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing controlled HTTP fetch capabilities (`tool.fetch`).
pub struct FetchModule {
    manifest: ModuleManifest,
    tool: Arc<FetchTool>,
}

impl FetchModule {
    /// Create a fetch module with explicit egress policy.
    pub fn new(config: FetchConfig) -> Self {
        Self {
            manifest: ModuleManifest::new("module.tool.fetch", "Controlled Fetch Tool Module"),
            tool: Arc::new(FetchTool::new(config)),
        }
    }

    /// Access the underlying fetch tool.
    pub fn tool(&self) -> &Arc<FetchTool> {
        &self.tool
    }
}

impl Module for FetchModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module that manages and dynamically contributes Model Context Protocol (MCP) tool capabilities.
pub struct McpModule {
    manifest: ModuleManifest,
    tools: std::sync::RwLock<Vec<Arc<dyn ToolCapability>>>,
}

impl McpModule {
    /// Create a new MCP module.
    pub fn new() -> Self {
        Self {
            manifest: ModuleManifest::new("module.mcp", "MCP Capability Module"),
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
    /// this module. Cross-module collisions are rejected by the runtime.
    pub fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        let mut tools = self.tools.write().expect("mcp lock poisoned");
        super::module::reject_tool_identity_collisions(&tools, &[Arc::clone(&tool)], "mcp")?;
        tools.push(tool);
        Ok(())
    }

    /// Unregister a dynamic tool capability by capability ID.
    ///
    /// Only tools whose id matches are removed; other owners are untouched.
    pub fn unregister_tool(&self, capability_id: &apeireth_core::kernel::CapabilityId) {
        self.tools
            .write()
            .expect("mcp lock poisoned")
            .retain(|t| t.id() != capability_id);
    }
}

impl Default for McpModule {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for McpModule {
    fn manifest(&self) -> &ModuleManifest {
        &self.manifest
    }

    fn tools(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools.read().expect("mcp lock poisoned").clone()
    }

    fn register_dynamic_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        self.register_tool(tool)
    }
}
