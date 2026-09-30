//! Canonical module wrappers for builtin tool capabilities and MCP.
//!
//! Tools are owned by modules; the microkernel dispatches to them through the
//! unified module/capability registry without hardcoding tool names.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::education::EducationTool;
use apeireth_tools_canonical::{
    apply_patch_capability, authorized_file_write_policy, AroundPolicy, FetchConfig, FetchTool,
    FilesystemTool, ObservedGate, PipelinedCapability, RepoTool, SearchTool, SelfStatusSource,
    SelfStatusTool, ShellTool, ToolExecutionPipeline, TrustedShellConfig, DEFAULT_MAX_TIMEOUT_MS,
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

    /// Create a filesystem module sharing one read-observation gate with the
    /// file-write tool: reads recorded here satisfy the write side's
    /// read-before-overwrite gate.
    pub fn new_with_gate(workspace_root: impl Into<PathBuf>, gate: Arc<ObservedGate>) -> Self {
        Self {
            tool: Arc::new(FilesystemTool::new(workspace_root).with_observed_gate(gate)),
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

/// The module bag is the registration target for the external tool bridge:
/// discovered dynamic tools land here, and identity collisions are refused by
/// the same reject-on-collision check the module already enforces.
impl apeireth_tools_canonical::mcp_bridge::McpToolRegistry for McpModule {
    fn register_tool(&self, tool: Arc<dyn ToolCapability>) -> Result<(), String> {
        McpModule::register_tool(self, tool)
    }

    fn unregister_tool(&self, capability_id: &apeireth_core::kernel::CapabilityId) {
        McpModule::unregister_tool(self, capability_id);
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

/// 自述工具的执行时限 (毫秒): 自述是进程内只读采集, 到期即归 `timeout.*` 帧。
pub const SELF_STATUS_TIMEOUT_MS: u64 = 2_000;

/// Module providing the structured self-report tool (`tool.self_status`).
///
/// 自省通道: 只读结构化自述面, 与既有 5 内置工具同列注册。执行走五段流水线:
/// 输出归一合同 (`SelfStatusTool::output_schema`) 冻结自述形状, 超时归既有
/// `timeout.*` code 族, 失败即帧。
pub struct SelfStatusModule {
    tool: Arc<dyn ToolCapability>,
}

impl SelfStatusModule {
    /// Create the self-status module bound to a self-report source.
    pub fn new(source: Arc<dyn SelfStatusSource>) -> Self {
        let inner: Arc<dyn ToolCapability> = Arc::new(SelfStatusTool::new(source));
        let pipeline = Arc::new(
            ToolExecutionPipeline::new()
                .with_around(
                    AroundPolicy::new()
                        .with_timeout(SELF_STATUS_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
                )
                .with_output_schema(SelfStatusTool::output_schema()),
        );
        Self {
            tool: Arc::new(PipelinedCapability::new(inner, pipeline)),
        }
    }
}

impl CapabilityProvider for SelfStatusModule {
    fn id(&self) -> &str {
        "module.tool.self_status"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the controlled file-write tool (`tool.apply_patch`).
///
/// 受控写文件 (第七件生产工具): 补丁式创建/修改/删除 (动作必须在补丁里显式
/// 声明), 沿用读前观测门禁 (未读不得覆盖写)。写入风险档位默认 require-approval
/// 级 —— 每次写入都停在人工审批 (与本地审批面板 / IM 审批卡同链); 设置可开
/// 「自动放行已读文件的修改」档, 但删除/新建永不自动放行。
///
/// git 写边界 (件三): git 提交等写操作不提供工具, 属设计边界 (工具描述同文)。
pub struct ApplyPatchModule {
    tool: Arc<dyn ToolCapability>,
}

impl ApplyPatchModule {
    /// Create the file-write module over the shared read-observation gate.
    pub fn new(workspace_root: impl Into<PathBuf>, gate: Arc<ObservedGate>) -> Self {
        let tool = apply_patch_capability(workspace_root, gate, authorized_file_write_policy());
        Self { tool }
    }

    /// Access the underlying (pipeline-wrapped) file-write capability.
    pub fn tool(&self) -> &Arc<dyn ToolCapability> {
        &self.tool
    }
}

impl CapabilityProvider for ApplyPatchModule {
    fn id(&self) -> &str {
        "module.tool.apply_patch"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}
