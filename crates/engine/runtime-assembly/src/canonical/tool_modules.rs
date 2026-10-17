//! Canonical module wrappers for builtin tool capabilities and MCP.
//!
//! Tools are owned by modules; the microkernel dispatches to them through the
//! unified module/capability registry without hardcoding tool names.

use std::path::PathBuf;
use std::sync::Arc;

use apeireth_plugin::ToolCapability;
use apeireth_tools_canonical::education::EducationTool;
use apeireth_tools_canonical::{
    apply_patch_capability, authorized_file_write_policy, authorized_git_write_policy,
    authorized_worktree_apply_policy, content_feed_capability, git_write_capability,
    worktree_apply_capability, worktree_plan_capability, AroundPolicy, ContentFeedConfig,
    FetchConfig, FetchTool, FilesystemTool, ObservedGate, PipelinedCapability, RepoTool,
    SearchTool, SelfStatusSource, SelfStatusTool, ShellTool, ToolExecutionPipeline,
    TrustedShellConfig, DEFAULT_MAX_TIMEOUT_MS,
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

/// Module providing the life-integration content feed (`tool.content_feed`, J1).
///
/// 内容流接入: 订阅源管理 + 受控拉取 + 结构化摘要 + 入记忆流 (L2 封套披露,
/// 跨源内容不直进提示词)。治理档位在治理链上
/// ([`apeireth_tools_canonical::ContentFeedApprovalHook`]): RequireApproval 级
/// —— 拉取与入库都经审批档, 改订阅走 write_intent 同族 (改订阅 = 写文件),
/// 每次调用都停人工审批。默认关 (opt-in)。
pub struct ContentFeedModule {
    tool: Arc<dyn ToolCapability>,
}

impl ContentFeedModule {
    /// Create the content-feed module from trusted configuration.
    pub fn new(config: ContentFeedConfig) -> Self {
        Self {
            tool: content_feed_capability(config),
        }
    }

    /// Access the underlying tool capability.
    pub fn capability(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for ContentFeedModule {
    fn id(&self) -> &str {
        "module.tool.content_feed"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the proactive-care rule scheduling tool (`tool.care_schedule`, V3-a).
///
/// 双引擎主动关怀的规则面: 增删列关怀规则 (四类触发器词表), 写文件面走
/// write_intent 同族 (改规则 = 写文件)。治理档位在治理链
/// ([`apeireth_organ::care::CareScheduleApprovalHook`]): RequireApproval 级
/// —— 每次调用都停人工审批, 无自动放行档。默认关 (opt-in)。关怀发起不走
/// 本模块: 发起 = 双引擎判定 → 标准回合 (见 [`apeireth_organ::care::engine`])。
pub struct CareScheduleModule {
    tool: Arc<dyn ToolCapability>,
}

impl CareScheduleModule {
    /// Create the care-schedule module bound to the care rules data directory.
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        let tool: Arc<dyn ToolCapability> =
            Arc::new(apeireth_organ::care::CareScheduleTool::new(data_dir));
        Self { tool }
    }

    /// Access the underlying tool capability.
    pub fn capability(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for CareScheduleModule {
    fn id(&self) -> &str {
        "module.tool.care_schedule"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the backup export tool (`tool.backup_export`, G5 备份恢复).
///
/// 记忆保险箱的模型面: 全量打包七类数据面 (会话库/认知库/记忆库/技能卡/
/// 关怀规则/用户表/域文档) + manifest.json 逐文件 SHA-256 摘要 (损坏可检出);
/// **凭据不随行** (密钥/令牌面显式排除 + 测试钉死)。恢复/校验是 CLI 面
/// (`apeireth backup restore|verify`, 本地操作)。治理档位由授权层同源接线
/// (RequireApproval 级: 每次导出都停人工审批)。默认关 (opt-in)。
pub struct BackupExportModule {
    tool: Arc<dyn ToolCapability>,
}

impl BackupExportModule {
    /// Create the backup-export module bound to its data sources and hooks.
    pub fn new(config: apeireth_tools_canonical::backup::BackupExportConfig) -> Self {
        let tool: Arc<dyn ToolCapability> = Arc::new(
            apeireth_tools_canonical::backup::BackupExportTool::new(config),
        );
        Self { tool }
    }

    /// Access the underlying tool capability.
    pub fn capability(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for BackupExportModule {
    fn id(&self) -> &str {
        "module.tool.backup_export"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the durable task board (`tool.task_board`, V6-a).
///
/// 任务簿工具面: 增/列/改派/完成 over 持久任务队列 (与会话账本同库的任务表)。
/// 治理档位在治理链 ([`crate::canonical::task_board_tool::TaskBoardApprovalHook`]):
/// 写面 (增/改派/完成) = write_intent 同族 (改任务簿 = 写文件) 每次人批,
/// `task_list` 只读零审批; **无 failed/retry 工具面** (failed 只能人判)。
/// 默认关 (opt-in)。
pub struct TaskBoardModule {
    tool: Arc<dyn ToolCapability>,
}

impl TaskBoardModule {
    /// Create the task-board module bound to the durable task queue.
    pub fn new(queue: std::sync::Arc<apeireth_orchestration::task_queue::TaskQueue>) -> Self {
        Self {
            tool: Arc::new(crate::canonical::task_board_tool::TaskBoardTool::new(queue)),
        }
    }

    /// Access the underlying tool capability.
    pub fn capability(&self) -> Arc<dyn ToolCapability> {
        Arc::clone(&self.tool)
    }
}

impl CapabilityProvider for TaskBoardModule {
    fn id(&self) -> &str {
        "module.tool.task_board"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
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
/// git 写边界 (件三, 分级语义版): 仓库写操作走分级词表 (`tool.git_write`),
/// L1 可逆默认人批 / L2 出网永远人批 / L3 毁灭无工具面 (工具描述同文)。
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

/// Module providing the graded git-write tool (`tool.git_write`, 干活能力轴 P1).
///
/// 分级词表: L1 可逆 (add/commit/branch 建·切/tag 本地) 默认人批, 设置可开
/// 「自动放行 L1」档; L2 出网 (push/fetch/remote) **永远人批** (无自动放行档,
/// 会话预设也不得越人); L3 毁灭 (改史/强推/引用删除) **无工具面** —— 词法分级
/// 拒绝即帧。执行走受控通道 (结构化 argv + 工作区限域 + 超时 `timeout.*` +
/// 输出归一)。分级风险档位由治理层的分级钩子承载 (与本地审批面板 / IM 审批卡
/// 同链), 本模块只接工具面。
pub struct GitWriteModule {
    tool: Arc<dyn ToolCapability>,
}

impl GitWriteModule {
    /// Create the graded git-write module over the workspace root.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let tool = git_write_capability(workspace_root, authorized_git_write_policy());
        Self { tool }
    }

    /// Access the underlying (pipeline-wrapped) git-write capability.
    pub fn tool(&self) -> &Arc<dyn ToolCapability> {
        &self.tool
    }
}

impl CapabilityProvider for GitWriteModule {
    fn id(&self) -> &str {
        "module.tool.git_write"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the read-only change-order tool (`tool.worktree_plan`, D-P2 计划拍).
///
/// 多文件大改的「计划→改→验证→落」四拍之首: 任务目标 → 改单 (文件列表 +
/// 每文件动作 + 预期)。**计划不落盘** —— 零文件系统写; 只读档**零审批**
/// (无 ask 档钩子, `read_only` 预设不拒只读能力)。改单是
/// [`WorktreeApplyModule`] 的输入合同。
pub struct WorktreePlanModule {
    tool: Arc<dyn ToolCapability>,
}

impl WorktreePlanModule {
    /// Create the read-only change-order module rooted at `workspace_root`.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            tool: worktree_plan_capability(workspace_root),
        }
    }

    /// Access the underlying (pipeline-wrapped) change-order capability.
    pub fn tool(&self) -> &Arc<dyn ToolCapability> {
        &self.tool
    }
}

impl CapabilityProvider for WorktreePlanModule {
    fn id(&self) -> &str {
        "module.tool.worktree_plan"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the controlled worktree write tool (`tool.worktree_apply`, D-P2).
///
/// 四拍受控工作树写: 计划拍 (改单/补丁交叉一致, 零写入) → 改拍 (暂存面执行,
/// 复用事务补丁原语) → 验证拍 (纯结构校验 + 可挂校验命令走受控通道) → 落拍
/// (过读前观测门禁落真实工作区, 全有或全无, 任一失败整批回滚)。
///
/// 治理: **RequireApproval 级** —— 每次落盘都停人工审批 (审批卡载荷 = 改单
/// 摘要: 文件数/动作分布/验证命令), 无自动放行档; `read_only` 预设拒本工具。
/// **写总闸语义**: 与 `tool.apply_patch` 同受 `file_write` 唯一写总闸 ——
/// 组装根必须只在同一开关下注册本模块 (注册即闸)。git 分工: 本模块只动
/// 文件, 提交走 `tool.git_write` 的分级词表 (工具描述同文)。
pub struct WorktreeApplyModule {
    tool: Arc<dyn ToolCapability>,
}

impl WorktreeApplyModule {
    /// Create the worktree-write module over the shared read-observation gate.
    pub fn new(workspace_root: impl Into<PathBuf>, gate: Arc<ObservedGate>) -> Self {
        let tool =
            worktree_apply_capability(workspace_root, gate, authorized_worktree_apply_policy());
        Self { tool }
    }

    /// Access the underlying (pipeline-wrapped) worktree-write capability.
    pub fn tool(&self) -> &Arc<dyn ToolCapability> {
        &self.tool
    }
}

impl CapabilityProvider for WorktreeApplyModule {
    fn id(&self) -> &str {
        "module.tool.worktree_apply"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the skill distill tool (`tool.skill_distill`, 书→技能沉淀 I1).
///
/// 知识吸收的写入口: 内容输入 (正文 / 文件只读通道) + 模型侧草稿 → 结构化
/// 技能卡 → 入技能库 (数据目录 `skills/`, 每卡一 StoredDoc 拒开语义)。创建
/// 重复名拒绝; 版本演进 (`update`) 同名更新留旧版审计。
///
/// 治理: **写文件面 = file_write 总闸族 + 审批档** —— 与 `tool.apply_patch`
/// 同受 [`super::production::ProductionModulesConfig::file_write`] 唯一写总闸
/// (注册即闸), **每次沉淀都停人工审批** ( [`SkillDistillApprovalHook`],
/// 无自动放行档, 审批卡载荷 = 技能卡摘要)。
pub struct SkillDistillModule {
    tool: Arc<dyn ToolCapability>,
}

impl SkillDistillModule {
    /// Create the skill distill module over the workspace root (来源只读通道限域)
    /// and the skill library directory (数据目录 `skills/`)。
    pub fn new(workspace_root: impl Into<PathBuf>, skills_dir: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let library = apeireth_tools_canonical::SkillLibrary::new(skills_dir.into());
        Self {
            tool: apeireth_tools_canonical::skill_distill_capability(
                workspace_root,
                library,
                apeireth_tools_canonical::authorized_skill_distill_policy(),
            ),
        }
    }

    /// Access the underlying (pipeline-wrapped) distill capability.
    pub fn tool(&self) -> &Arc<dyn ToolCapability> {
        &self.tool
    }
}

impl CapabilityProvider for SkillDistillModule {
    fn id(&self) -> &str {
        "module.tool.skill_distill"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the read-only skill search tool (`tool.skill_search`, P1 调用面; 原 I1, 审计修 D13 改名).
///
/// 检索式注入 (触发词命中 / 按需拉取) 经 **L2 封套披露**进上下文 (共享引用
/// 封套 + 共享预算, 绝不无差别全量注入)。**只读档零审批** (无 ask 档钩子,
/// `read_only` 会话预设不拒只读能力)。
pub struct SkillSearchModule {
    tool: Arc<dyn ToolCapability>,
}

impl SkillSearchModule {
    /// Create the skill search module over the skill library directory
    /// (数据目录 `skills/`) with the disclosure budget (字符, 与调用方
    /// 上下文预算同参换算)。
    pub fn new(skills_dir: impl Into<PathBuf>, disclosure_budget_chars: usize) -> Self {
        let library = apeireth_tools_canonical::SkillLibrary::new(skills_dir.into());
        Self {
            tool: apeireth_tools_canonical::skill_search_capability(
                library,
                disclosure_budget_chars,
            ),
        }
    }

    /// Access the underlying (pipeline-wrapped) search capability.
    pub fn tool(&self) -> &Arc<dyn ToolCapability> {
        &self.tool
    }
}

impl CapabilityProvider for SkillSearchModule {
    fn id(&self) -> &str {
        "module.tool.skill_search"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}

/// Module providing the general-voice speech tool (`tool.voice_speak`, V3-b 通用嗓闭环).
///
/// 发声面: 文本 → 系统语音引擎合成即播 (合成与播放同一受控调用) + 语句账本
/// (`spoken.jsonl` append-only: 说了什么/何时/哪把嗓/时长) + 安静纪律 (勿扰
/// 窗口内错误即帧)。受控播放: 同时只播一条, 新话打断旧话可配 (排队串行可选),
/// 播放票据 Drop = 真停 (取消/超时不留僵尸播放)。
///
/// 治理: **发声面档** —— 注册即闸 (`APEIRETH_ENABLE_VOICE_SPEAK`, 默认关
/// fail-closed), grant 即发声 (免逐次人批); 与 shell 的「每次人批」执行档
/// **分立不借壳**, 拦截面 = 参数边界 + 勿扰窗口 + 独占播放 + 账本全责。
/// 合成执行面注入 (生产 = 系统引擎桥; 测试 = 假引擎)。
pub struct VoiceSpeakModule {
    tool: Arc<dyn ToolCapability>,
}

impl VoiceSpeakModule {
    /// Create the voice-speak module from trusted configuration + 合成执行面。
    pub fn new(
        config: apeireth_tools_canonical::VoiceSpeakConfig,
        engine: Arc<dyn apeireth_tools_canonical::SynthEngine>,
    ) -> Self {
        let tool = apeireth_tools_canonical::voice_speak_capability(
            config,
            engine,
            apeireth_tools_canonical::authorized_voice_speak_policy(),
        );
        Self { tool }
    }

    /// Access the underlying (pipeline-wrapped) speech capability.
    pub fn tool(&self) -> &Arc<dyn ToolCapability> {
        &self.tool
    }
}

impl CapabilityProvider for VoiceSpeakModule {
    fn id(&self) -> &str {
        "module.tool.voice_speak"
    }

    fn capabilities(&self) -> Vec<Arc<dyn ToolCapability>> {
        vec![Arc::clone(&self.tool)]
    }
}
