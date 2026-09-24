//! Canonical module wrappers for builtin tool capabilities and MCP.
//!
//! Tools are owned by modules; the microkernel dispatches to them through the
//! unified module/capability registry without hardcoding tool names.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::education::EducationTool;
use apeireth_tools_canonical::{
    FetchConfig, FetchTool, FilesystemTool, RepoTool, SearchTool, ShellTool, TrustedShellConfig,
};

use super::capability::CapabilityProvider;

/// Module providing filesystem capabilities (`tool.filesystem`).
pub struct FilesystemModule {
    tool: Arc<FilesystemTool>,
}

impl FilesystemModule {
    /// Create a filesystem module rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            tool: Arc::new(FilesystemTool::new(workspace_root)),
        }
    }

    /// Access the underlying filesystem tool.
    pub fn tool(&self) -> &Arc<FilesystemTool> {
        &self.tool
    }
}

impl CapabilityProvider for FilesystemModule {
    fn id(&self) -> &str {
        "module.tool.filesystem"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing search capabilities (`tool.search`).
pub struct SearchModule {
    tool: Arc<SearchTool>,
}

impl SearchModule {
    /// Create a search module rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            tool: Arc::new(SearchTool::new(workspace_root)),
        }
    }

    /// Access the underlying search tool.
    pub fn tool(&self) -> &Arc<SearchTool> {
        &self.tool
    }
}

impl CapabilityProvider for SearchModule {
    fn id(&self) -> &str {
        "module.tool.search"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing git repository inspection capabilities (`tool.repo`).
pub struct RepoModule {
    tool: Arc<RepoTool>,
}

impl RepoModule {
    /// Create a repo module rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            tool: Arc::new(RepoTool::new(workspace_root)),
        }
    }

    /// Access the underlying repo tool.
    pub fn tool(&self) -> &Arc<RepoTool> {
        &self.tool
    }
}

impl CapabilityProvider for RepoModule {
    fn id(&self) -> &str {
        "module.tool.repo"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing trusted shell command execution capabilities (`tool.shell`).
pub struct ShellModule {
    tool: Arc<ShellTool>,
}

impl ShellModule {
    /// Create a shell module with explicit configuration.
    pub fn new(config: TrustedShellConfig) -> Self {
        Self {
            tool: Arc::new(ShellTool::new(config)),
        }
    }

    /// Access the underlying shell tool.
    pub fn tool(&self) -> &Arc<ShellTool> {
        &self.tool
    }
}

impl CapabilityProvider for ShellModule {
    fn id(&self) -> &str {
        "module.tool.shell"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module providing controlled HTTP fetch capabilities (`tool.fetch`).
pub struct FetchModule {
    tool: Arc<FetchTool>,
}

impl FetchModule {
    /// Create a fetch module with explicit egress policy.
    pub fn new(config: FetchConfig) -> Self {
        Self {
            tool: Arc::new(FetchTool::new(config)),
        }
    }

    /// Access the underlying fetch tool.
    pub fn tool(&self) -> &Arc<FetchTool> {
        &self.tool
    }
}

impl CapabilityProvider for FetchModule {
    fn id(&self) -> &str {
        "module.tool.fetch"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}

/// Module that manages and dynamically contributes Model Context Protocol (MCP) tool capabilities.
pub struct McpModule {
    tools: std::sync::RwLock<Vec<Arc<dyn ToolCapability>>>,
}

impl McpModule {
    /// Create a new MCP module.
    pub fn new() -> Self {
        Self {
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
        // poison 容错 (2026-09-24 修复轮补漏): 锁保护的是可重建的工具注册表,
        // 持锁线程 panic 不应让后续注册/查询连锁 panic (与 G 组 poison 修复同模式)。
        let mut tools = self
            .tools
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|t| t.id() != capability_id);
    }
}

impl Default for McpModule {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityProvider for McpModule {
    fn id(&self) -> &str {
        "module.mcp"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        self.tools
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// Module providing education Dx-Check substitution verification (`tool.education`).
///
/// W2 §4.3 (2026-10-10): 包装 `DxCheckTool` 纯确定性换元检查 (微分标记一致性 /
/// 残留原变量 / 经典根号三角代换模式) —— 0 副作用, 默认关注册。
pub struct EducationModule {
    tool: Arc<EducationTool>,
}

impl EducationModule {
    /// Create the education module (无状态工具)。
    pub fn new() -> Self {
        Self {
            tool: Arc::new(EducationTool::new()),
        }
    }

    /// Access the underlying education tool.
    pub fn tool(&self) -> &Arc<EducationTool> {
        &self.tool
    }
}

impl Default for EducationModule {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityProvider for EducationModule {
    fn id(&self) -> &str {
        "module.tool.education"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![self.tool.clone()]
    }
}
