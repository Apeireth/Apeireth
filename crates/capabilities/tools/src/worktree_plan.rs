//! 只读改单工具面 (`tool.worktree_plan`, 声明名 `worktree_plan`) —— 干活能力轴 P2 计划拍。
//!
//! 多文件大改走「计划→改→验证→落」四拍; 本模块承载**计划拍**:
//!
//! 1. **接收任务目标** (`goal`) 与逐文件声明 (`path` / `action` / `expect`);
//! 2. **产出改单** ([`WorktreeChangeOrder`]): 文件列表 + 每文件动作 + 预期结果,
//!    附改单身份摘要 ([`WorktreeChangeOrder::plan_id`])、文件计数与动作分布;
//! 3. **计划不落盘**: 本工具零文件系统写 —— 只做词法/语义校验 (工作区相对路径
//!    唯一、敏感面拒绝、预期结果非空), 不创建、不修改、不删除任何文件。
//!
//! 改单是 [`crate::worktree_apply`] 的输入合同: apply 的计划拍按改单逐项
//! 核对补丁声明的动作 (改单与补丁不一致 → 拒绝即帧), 其后才进暂存、验证、
//! 落地。两级工具都过五段流水线; 计划拍是**只读档零审批** (无 ask 档钩子,
//! `read_only` 预设不拒只读能力)。
//!
//! # Git 写边界 (与 `tool.git_write` 分工)
//!
//! 改单只动**文件**, 不含任何 git 写: 提交走 [`crate::git_write`] 的分级词表
//! (L1 可逆默认人批), 见 [`WORKTREE_GIT_BOUNDARY_NOTE`] (工具描述同文)。

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{ToolGuard, ToolGuardRequest};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::exec_pipeline::{AroundPolicy, OutputSchema, SchemaKind, DEFAULT_MAX_TIMEOUT_MS};
use crate::sensitive_path::{credential_surface_refusal, is_sensitive_path};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const WORKTREE_PLAN_CAPABILITY_ID: &str = "tool.worktree_plan";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const WORKTREE_PLAN_TOOL_NAME: &str = "worktree_plan";

/// 生产执行时限 (毫秒): 计划拍是纯校验, 到期即归 `timeout.*`。
pub const WORKTREE_PLAN_TIMEOUT_MS: u64 = 5_000;

/// git 写边界声明 (D-P2 与 `tool.git_write` 的分工, 工具描述与文档同文)。
///
/// worktree 两级工具只动文件: 改单与落盘都不含 git 写; 提交是仓库写, 走
/// `tool.git_write` 的分级词表 (L1 可逆默认人批 / L2 出网永远人批 / L3 毁灭
/// 无工具面)。显式写明防模型在 worktree 工具里幻觉出 git 操作。
pub const WORKTREE_GIT_BOUNDARY_NOTE: &str = "Design boundary: worktree tools \
  only move file content inside the workspace and never perform git writes - \
  neither planning nor landing runs a repository command. Committing landed \
  changes is a repository write and goes through the graded git_write tool \
  (L1 reversible local writes such as git add and git commit with a message sit \
  at human approval by default and may auto-pass only via an explicit settings \
  tier; L2 network-bound writes always need a human decision; L3 destructive \
  writes have no tool), so do not request git operations from worktree tools.";

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const WORKTREE_PLAN_TOOL_DESCRIPTION: &str = "Produce a change order for one \
  multi-file task without touching disk. Declare every target file with its \
  action (add / update / delete) and the expected outcome; the tool validates \
  the declarations (workspace-relative paths only - absolute paths, drive or UNC \
  prefixes and `..` components are refused; credential and secret surfaces are \
  refused; duplicate targets are refused; each expected outcome must be non-empty) \
  and returns the change order with a stable plan id, the file count and the \
  action distribution. This planning step is read-only: it creates, modifies and \
  deletes nothing, and it needs no approval. Feed the same change order to \
  worktree_apply together with the patch that realizes it; worktree_apply refuses \
  any patch whose declared actions drift from the change order. ";

/// 改单里的一个文件动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorktreeChangeAction {
    /// 新建文件。
    Add,
    /// 修改既有文件。
    Update,
    /// 删除既有文件。
    Delete,
}

impl WorktreeChangeAction {
    /// 稳定动作标签 (改单/补丁交叉核对、报告字段同文)。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}

/// 改单里的一条文件声明: 目标 + 动作 + 预期结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeFileChange {
    /// 工作区相对目标路径。
    pub path: String,
    /// 该文件的动作。
    pub action: WorktreeChangeAction,
    /// 预期结果 (人读描述, 必须非空 —— 没有预期的改动不进改单)。
    pub expect: String,
}

/// 动作分布 (审批卡摘要 / 报告字段同文)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionDistribution {
    /// 新建计数。
    pub add: usize,
    /// 修改计数。
    pub update: usize,
    /// 删除计数。
    pub delete: usize,
}

impl ActionDistribution {
    /// 按声明序列统计。
    pub fn of(files: &[WorktreeFileChange]) -> Self {
        let mut distribution = Self::default();
        for file in files {
            match file.action {
                WorktreeChangeAction::Add => distribution.add += 1,
                WorktreeChangeAction::Update => distribution.update += 1,
                WorktreeChangeAction::Delete => distribution.delete += 1,
            }
        }
        distribution
    }

    /// 稳定摘要文本 (`add a/update u/delete d`)。
    pub fn summary(self) -> String {
        format!(
            "add {}/update {}/delete {}",
            self.add, self.update, self.delete
        )
    }
}

/// 改单: 任务目标 + 文件动作序列 (计划拍产物, apply 的输入合同)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeChangeOrder {
    /// 任务目标 (人读, 必须非空)。
    pub goal: String,
    /// 逐文件动作声明 (顺序即改单顺序; 至少一条)。
    pub files: Vec<WorktreeFileChange>,
}

impl WorktreeChangeOrder {
    /// 文件计数。
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// 动作分布。
    pub fn action_distribution(&self) -> ActionDistribution {
        ActionDistribution::of(&self.files)
    }

    /// 改单身份摘要 (FNV-1a 64 hex, **非密码学**)。
    ///
    /// 供审批卡 / 报告 / 审计做「同一改单」配对标识, 不是安全边界: 声明序列
    /// (goal + 逐条 path|action|expect) 变则摘要必变, 序列不变则摘要稳定。
    pub fn plan_id(&self) -> String {
        let mut canonical = String::new();
        canonical.push_str(self.goal.trim());
        canonical.push('\n');
        for file in &self.files {
            canonical.push_str(&normalized_path_key(&file.path));
            canonical.push('|');
            canonical.push_str(file.action.as_str());
            canonical.push('|');
            canonical.push_str(file.expect.trim());
            canonical.push('\n');
        }
        format!("{:016x}", fnv1a64(canonical.as_bytes()))
    }

    /// 审批卡载荷的改单摘要: 文件数 + 动作分布 (+ 目标首行)。
    pub fn summary(&self) -> String {
        format!(
            "改单摘要: 文件 {} ({}); 目标: {}",
            self.file_count(),
            self.action_distribution().summary(),
            first_line(&self.goal)
        )
    }
}

/// 摘要里的目标首行 (审批卡不整段贴长目标)。
fn first_line(text: &str) -> String {
    text.trim()
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// FNV-1a 64 (稳定、零依赖; 仅身份摘要用)。
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 路径归一键 (重复声明判定 / 摘要稳定): 统一 `/` 分隔, 剥 `./` 前缀。
pub(crate) fn normalized_path_key(path: &str) -> String {
    let trimmed = path.trim().replace('\\', "/");
    let mut key = trimmed.as_str();
    while let Some(rest) = key.strip_prefix("./") {
        key = rest;
    }
    key.trim_end_matches('/').to_string()
}

/// 改单校验 (纯判定, 零文件系统**写**): 返回 `Some(拒绝即帧文案)` 即拒绝。
///
/// 逐项: ① 任务目标非空; ② 至少一条文件声明; ③ 每条: 路径非空 + 词法包含
/// (禁绝对路径 / 盘符 UNC / `..` 组件) + 敏感面 (凭据/密钥面) 拒 + 预期非空;
/// ④ 同一目标不得重复声明 (改单唯一性)。
pub fn change_order_refusal(root: &Path, order: &WorktreeChangeOrder) -> Option<String> {
    if order.goal.trim().is_empty() {
        return Some("拒绝改单: 任务目标 (goal) 不能为空".to_string());
    }
    if order.files.is_empty() {
        return Some("拒绝改单: 改单必须至少声明一个文件动作".to_string());
    }
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for file in &order.files {
        let target = file.path.trim();
        if target.is_empty() {
            return Some("拒绝改单: 改单路径为空".to_string());
        }
        let path = Path::new(target);
        if path.is_absolute() {
            return Some(format!(
                "拒绝改单 {target}: 禁止绝对路径 (必须是工作区相对路径)"
            ));
        }
        for component in path.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                other => {
                    return Some(format!("拒绝改单 {target}: 禁止的路径组件 ({other:?})"));
                }
            }
        }
        if is_sensitive_path(root, &root.join(path)) {
            return Some(credential_surface_refusal());
        }
        if file.expect.trim().is_empty() {
            return Some(format!(
                "拒绝改单 {target}: 每个文件必须声明预期结果 (expect 非空)"
            ));
        }
        let key = normalized_path_key(target);
        if let Some(previous) = seen.insert(key, file.action.as_str().to_string()) {
            return Some(format!(
                "拒绝改单 {target}: 同一目标路径重复声明 (先 {previous}, 后 {})",
                file.action.as_str()
            ));
        }
    }
    None
}

/// 从调用参数解析并校验改单 (解析失败/校验失败返回拒绝文案)。
pub fn validated_change_order(
    root: &Path,
    arguments: &serde_json::Value,
) -> Result<WorktreeChangeOrder, String> {
    let order: WorktreeChangeOrder = serde_json::from_value(arguments.clone())
        .map_err(|error| format!("invalid worktree_plan parameters: {error}"))?;
    if let Some(refusal) = change_order_refusal(root, &order) {
        return Err(refusal);
    }
    Ok(order)
}

/// 只读改单工具 (`tool.worktree_plan`, 声明名 `worktree_plan`)。
///
/// 零文件系统写: 产出改单后立即返回, 计划不落盘。
pub struct WorktreePlanTool {
    id: CapabilityId,
    root: PathBuf,
}

impl WorktreePlanTool {
    /// 绑定工作区根 (校验限域: 路径包含与敏感面判定都以它为界)。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            id: CapabilityId::new(WORKTREE_PLAN_CAPABILITY_ID).expect("valid capability id"),
            root: root.into(),
        }
    }

    /// 输出归一合同: 计划报告的形状冻结为六字段。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("status", SchemaKind::String)
            .require("plan_id", SchemaKind::String)
            .require("goal", SchemaKind::String)
            .require("file_count", SchemaKind::Number)
            .require("action_distribution", SchemaKind::Object)
            .require("files", SchemaKind::Array)
    }
}

#[async_trait]
impl ToolCapability for WorktreePlanTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "goal": {
                        "type": "string",
                        "description": "The task objective this change order realizes (human-readable)."
                    },
                    "files": {
                        "type": "array",
                        "description": "One entry per target file: {path, action, expect}.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "Workspace-relative target path."
                                },
                                "action": {
                                    "type": "string",
                                    "enum": ["add", "update", "delete"],
                                    "description": "What happens to the file."
                                },
                                "expect": {
                                    "type": "string",
                                    "description": "Expected outcome for this file (non-empty)."
                                }
                            },
                            "required": ["path", "action", "expect"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["goal", "files"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(WORKTREE_PLAN_TOOL_NAME)
            .with_description(format!(
                "{WORKTREE_PLAN_TOOL_DESCRIPTION}{WORKTREE_GIT_BOUNDARY_NOTE}"
            ))
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = WORKTREE_PLAN_TOOL_NAME;
        let order = match validated_change_order(&self.root, &call.arguments) {
            Ok(order) => order,
            Err(refusal) => return ToolResult::permanent_error(&call.id, refusal).with_name(name),
        };
        ToolResult::ok(
            &call.id,
            serde_json::json!({
                "status": "planned",
                "plan_id": order.plan_id(),
                "goal": order.goal,
                "file_count": order.file_count(),
                "action_distribution": {
                    "add": order.action_distribution().add,
                    "update": order.action_distribution().update,
                    "delete": order.action_distribution().delete,
                },
                "files": order.files,
            }),
        )
        .with_name(name)
    }
}

/// 单调边界 Guard (只拒不放): 改单校验不过即拒 —— 工作区外 / 敏感面 /
/// 重复声明 / 空目标, 任何后续阶段都不能翻回放行。
pub struct WorktreePlanBoundaryGuard {
    root: PathBuf,
}

impl WorktreePlanBoundaryGuard {
    /// 绑定工作区根的改单边界 Guard。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl ToolGuard for WorktreePlanBoundaryGuard {
    fn name(&self) -> &str {
        "worktree_plan_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != WORKTREE_PLAN_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判改单边界。
        let Ok(order) = serde_json::from_value::<WorktreeChangeOrder>(request.arguments.clone())
        else {
            return None;
        };
        change_order_refusal(&self.root, &order)
    }
}

/// 组装计划拍五段流水线: 只读档零审批 (无 ask 档钩子) → 单调边界 Guard →
/// 超时 → 输出归一合同。
pub fn worktree_plan_pipeline(
    root: impl Into<PathBuf>,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_guard(Arc::new(WorktreePlanBoundaryGuard::new(root)))
            .with_around(
                AroundPolicy::new().with_timeout(WORKTREE_PLAN_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(WorktreePlanTool::output_schema()),
    )
}

/// 组装只读改单工具面 (模型侧一个能力)。
pub fn worktree_plan_capability(root: impl Into<PathBuf>) -> Arc<dyn ToolCapability> {
    let root = root.into();
    let tool: Arc<dyn ToolCapability> = Arc::new(WorktreePlanTool::new(root.clone()));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        worktree_plan_pipeline(root),
    ))
}
