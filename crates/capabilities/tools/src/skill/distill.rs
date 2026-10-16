//! 沉淀管线工具面 (`tool.skill_distill`, 声明名 `skill_distill`) —— 书→技能
//! 沉淀 (P 轴吸收自动化 P1) 的写入口。
//!
//! 管线分工 (命名即承诺): **提炼是模型侧的活** (调用方给 prompt 模板, 产出
//! 草稿 [`SkillCardDraft`]: 触发词 / 要点 / 操作步骤), **管线与落盘是工具侧
//! 的活** ([`distill_into`] 三拍):
//!
//! 1. **来源采集** (内容输入, 二选一): 正文 (`text`) 或文件路径 (`path`,
//!    **只读通道**: 词法限域 + 敏感面拒 + 超限拒, 只读不写);
//! 2. **结构化**: 坏卡拒 + 触发词归一 + 派生稳定身份与沉淀时刻
//!    ([`crate::skill::card::structure_card`]);
//! 3. **入技能库**: 创建**重复名拒绝**; 版本演进 (`update`) = 同名更新,
//!    旧版先留审计再原子写新版 ([`crate::skill::library`])。
//!
//! # 治理 (写文件面: file_write 总闸族 + 审批档)
//!
//! - **file_write 总闸族**: 组装根只在文件写总闸 (`file_write`) 同开时注册
//!    本工具 (注册即闸, 与 `tool.apply_patch` / `tool.worktree_apply` 同一闸);
//! - **审批档**: **每次沉淀都停人工审批** ([`SkillDistillApprovalHook`],
//!    无自动放行档), 审批卡载荷 = 技能卡摘要 (卡名 / 来源 / 触发词数 / 摘要);
//! - **单调边界 Guard** ([`SkillDistillBoundaryGuard`]): 坏卡 / 来源越界 /
//!    来源缺省一律拒绝即帧, 任何后续阶段不得翻回。

use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{
    Action, Decision, GovernanceHook, GovernanceRequest, PermissionPolicy, ToolGuard,
    ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, PreExecuteHook, PreExecuteRequest, PreVerdict, SchemaKind,
    DEFAULT_MAX_TIMEOUT_MS,
};
use crate::sensitive_path::{credential_surface_refusal, is_sensitive_path};
use crate::skill::card::{card_draft_refusal, SkillCardDraft, SkillSourceKind};
use crate::skill::library::{SkillIntake, SkillLibrary};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const SKILL_DISTILL_CAPABILITY_ID: &str = "tool.skill_distill";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const SKILL_DISTILL_TOOL_NAME: &str = "skill_distill";

/// 生产执行时限 (毫秒): 沉淀走阻塞池做持久档原子写, 到期即归 `timeout.*`。
pub const SKILL_DISTILL_TIMEOUT_MS: u64 = 30_000;

/// 内容输入的字节上限 (超过拒绝即帧, 不静默截断 —— 大书请分章沉淀)。
pub const SKILL_SOURCE_MAX_BYTES: usize = 512 * 1024;

/// 内联正文来源在报告里的稳定标签。
pub const SKILL_SOURCE_INLINE_REF: &str = "inline:text";

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const SKILL_DISTILL_TOOL_DESCRIPTION: &str = "Distill one skill card from \
  content input into the skill library. The distillation itself (trigger words, \
  key points, operation steps) is produced by the model-side prompt template of \
  the caller and passed as `card`; this tool owns the pipeline and persistence: \
  source acquisition (exactly one of `text` or `path` - `path` is read through a \
  read-only workspace channel), structuring (bad cards are refused, triggers are \
  normalized, identity and timestamp are derived), and library intake. Creating a \
  card whose name already exists is refused; version evolution is explicit: pass \
  `update: true` to evolve the same-name card, which archives the old version to \
  the audit log before the new version lands atomically. This is a file-write \
  face behind the file_write master gate and every distillation waits for a \
  human decision.";

/// 工具调用参数 (未知字段拒绝)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillDistillParams {
    /// 结构化草稿 (模型侧 prompt 模板产物)。
    pub card: SkillCardDraft,
    /// 内容输入正文 (与 `path` 二选一)。
    #[serde(default)]
    pub text: Option<String>,
    /// 内容输入文件路径 (与 `text` 二选一; 只读通道, 工作区相对路径)。
    #[serde(default)]
    pub path: Option<String>,
    /// 版本演进开关: `true` = 同名更新 (留旧版审计); 缺省 = 创建 (重复名拒绝)。
    #[serde(default)]
    pub update: bool,
}

/// 采集到的内容输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSourceInput {
    /// 内容输入文本。
    pub text: String,
    /// 来源引用 (报告字段: 文件路径或 [`SKILL_SOURCE_INLINE_REF`])。
    pub reference: String,
}

/// 内容输入校验 (纯判定, 零 IO): 返回 `Some(拒绝即帧文案)` 即拒绝。
///
/// 逐项: ① `text` / `path` **恰给一个**且非空; ② 内联正文限字节;
/// ③ 文件路径词法限域 (禁绝对路径 / 盘符 UNC / `..` 组件)。
pub fn source_request_refusal(params: &SkillDistillParams) -> Option<String> {
    let text = params.text.as_deref().map(str::trim);
    let path = params.path.as_deref().map(str::trim);
    let text = text.filter(|value| !value.is_empty());
    let path = path.filter(|value| !value.is_empty());
    match (text, path) {
        (None, None) => {
            return Some("拒绝沉淀: 内容输入必须恰给一个 (text 或 path)".to_string());
        }
        (Some(_), Some(_)) => {
            return Some("拒绝沉淀: text 与 path 只能给一个 (内容输入二选一)".to_string());
        }
        (Some(text), None) => {
            if text.len() > SKILL_SOURCE_MAX_BYTES {
                return Some(format!(
                    "拒绝沉淀: 内容输入超限 (上限 {SKILL_SOURCE_MAX_BYTES} 字节; 大书请分章沉淀)"
                ));
            }
        }
        (None, Some(path)) => {
            let target = Path::new(path);
            if target.is_absolute() {
                return Some(format!(
                    "拒绝沉淀 {path}: 禁止绝对路径 (必须是工作区相对路径)"
                ));
            }
            for component in target.components() {
                match component {
                    Component::Normal(_) | Component::CurDir => {}
                    other => {
                        return Some(format!("拒绝沉淀 {path}: 禁止的路径组件 ({other:?})"));
                    }
                }
            }
        }
    }
    None
}

/// 沉淀请求校验 (纯判定): 来源输入 + 坏卡拒。返回 `Some(拒绝即帧文案)` 即拒。
pub fn distill_request_refusal(params: &SkillDistillParams) -> Option<String> {
    source_request_refusal(params).or_else(|| card_draft_refusal(&params.card))
}

/// 只读通道采集内容输入: 内联正文直取; 文件路径过敏感面判定后**只读**。
pub fn read_source(root: &Path, params: &SkillDistillParams) -> Result<SkillSourceInput, String> {
    if let Some(text) = params
        .text
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
    {
        return Ok(SkillSourceInput {
            text: text.to_string(),
            reference: SKILL_SOURCE_INLINE_REF.to_string(),
        });
    }
    let path = params
        .path
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or_else(|| "拒绝沉淀: 内容输入必须恰给一个 (text 或 path)".to_string())?;
    let joined = root.join(path);
    if is_sensitive_path(root, &joined) {
        return Err(credential_surface_refusal());
    }
    let bytes = std::fs::read(&joined)
        .map_err(|error| format!("拒绝沉淀: 来源文件读取失败 {path}: {error}"))?;
    if bytes.len() > SKILL_SOURCE_MAX_BYTES {
        return Err(format!(
            "拒绝沉淀: 来源文件超限 ({path}, 上限 {SKILL_SOURCE_MAX_BYTES} 字节; 大书请分章沉淀)"
        ));
    }
    let text = String::from_utf8(bytes)
        .map_err(|error| format!("拒绝沉淀: 来源文件不是 UTF-8 文本 {path}: {error}"))?;
    Ok(SkillSourceInput {
        text,
        reference: path.to_string(),
    })
}

/// 沉淀报告 (输出归一合同的字段来源)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillDistillReport {
    /// `distilled` (创建) / `evolved` (版本演进)。
    pub status: &'static str,
    /// 稳定身份。
    pub id: String,
    /// 卡名。
    pub name: String,
    /// 当前版本序号 (创建 = 1; 每次演进 +1)。
    pub revision: u32,
    /// 归一后的触发词。
    pub triggers: Vec<String>,
    /// 来源种类标签 (`document` / `book` / `article`)。
    pub source_kind: SkillSourceKind,
    /// 来源标题。
    pub source_title: String,
    /// 内容输入引用 (文件路径或内联标签)。
    pub source_ref: String,
    /// 内容输入字符数。
    pub source_chars: usize,
    /// 已存档的旧版本数 (创建 = 0)。
    pub archived_count: u32,
}

impl SkillDistillReport {
    /// 从入库结果 + 内容输入组装报告。
    pub fn from_intake(intake: &SkillIntake, source: &SkillSourceInput) -> Self {
        let card = &intake.card.body;
        Self {
            status: if intake.archived.is_some() {
                "evolved"
            } else {
                "distilled"
            },
            id: card.id.clone(),
            name: card.name.clone(),
            revision: intake.revision,
            triggers: card.triggers.clone(),
            source_kind: card.source.kind,
            source_title: card.source.title.clone(),
            source_ref: source.reference.clone(),
            source_chars: source.text.chars().count(),
            archived_count: intake.revision.saturating_sub(1),
        }
    }
}

/// 沉淀管线 (工具侧三拍): 来源采集 → 结构化 → 入技能库。`now_ms` 由调用方
/// 给 (测试可钉死)。
pub fn distill_into(
    library: &SkillLibrary,
    root: &Path,
    params: &SkillDistillParams,
    now_ms: u64,
) -> Result<SkillDistillReport, String> {
    if let Some(refusal) = source_request_refusal(params) {
        return Err(refusal);
    }
    let source = read_source(root, params)?;
    let intake = if params.update {
        library.update(&params.card, now_ms)
    } else {
        library.create(&params.card, now_ms)
    }
    .map_err(|error| error.to_string())?;
    Ok(SkillDistillReport::from_intake(&intake, &source))
}

/// epoch 毫秒 (沉淀时刻)。
fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 审批卡载荷 (沉淀摘要): 卡名 / 模式 / 来源 / 触发词数 / 摘要。
///
/// 请求本身坏 (来源缺失 / 坏卡) 时返回 `Err` —— 拒绝即帧, 不进审批。
pub fn skill_distill_summary(params: &SkillDistillParams) -> Result<String, String> {
    if let Some(refusal) = distill_request_refusal(params) {
        return Err(refusal);
    }
    let mode = if params.update {
        "版本演进 (同名更新, 留旧版审计)"
    } else {
        "创建 (重复名拒绝)"
    };
    Ok(format!(
        "技能卡沉淀 [{}]: {} ({}·{}); 触发词 {} 个; 摘要: {}",
        mode,
        params.card.name.trim(),
        params.card.source.kind.as_str(),
        params.card.source.title.trim(),
        crate::skill::card::normalize_triggers(&params.card.triggers).len(),
        params.card.summary.trim()
    ))
}

/// 沉淀工具 (`tool.skill_distill`, 声明名 `skill_distill`)。
pub struct SkillDistillTool {
    id: CapabilityId,
    root: PathBuf,
    library: SkillLibrary,
}

impl SkillDistillTool {
    /// 绑定工作区根 (来源只读通道限域) 与技能库。
    pub fn new(root: impl Into<PathBuf>, library: SkillLibrary) -> Self {
        Self {
            id: CapabilityId::new(SKILL_DISTILL_CAPABILITY_ID).expect("valid capability id"),
            root: root.into(),
            library,
        }
    }

    /// 输出归一合同: 沉淀报告的形状冻结为九字段。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("status", SchemaKind::String)
            .require("id", SchemaKind::String)
            .require("name", SchemaKind::String)
            .require("revision", SchemaKind::Number)
            .require("triggers", SchemaKind::Array)
            .require("source_kind", SchemaKind::String)
            .require("source_ref", SchemaKind::String)
            .require("source_chars", SchemaKind::Number)
            .require("archived_count", SchemaKind::Number)
    }
}

#[async_trait]
impl ToolCapability for SkillDistillTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "card": {
                        "type": "object",
                        "description": "The structured skill card draft from the caller's model-side prompt template: {name, summary, triggers, body, source:{kind, title}}."
                    },
                    "text": {
                        "type": "string",
                        "description": "Inline content input (mutually exclusive with path)."
                    },
                    "path": {
                        "type": "string",
                        "description": "Workspace-relative source file, read through a read-only channel (mutually exclusive with text)."
                    },
                    "update": {
                        "type": "boolean",
                        "description": "true = evolve the same-name card (archives the old version); default = create (duplicate names are refused)."
                    }
                },
                "required": ["card"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(SKILL_DISTILL_TOOL_NAME)
            .with_description(SKILL_DISTILL_TOOL_DESCRIPTION.to_string())
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = SKILL_DISTILL_TOOL_NAME;
        let params: SkillDistillParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(params) => params,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid skill_distill parameters: {error}"),
                )
                .with_name(name)
            }
        };
        if let Some(refusal) = distill_request_refusal(&params) {
            return ToolResult::permanent_error(&call.id, refusal).with_name(name);
        }
        // 落盘走阻塞池: 持久档原子写 + fsync 不占执行器; 到期计时器照常触发。
        let root = self.root.clone();
        let library = self.library.clone();
        match tokio::task::spawn_blocking(move || {
            distill_into(&library, &root, &params, epoch_ms())
        })
        .await
        {
            Ok(Ok(report)) => ToolResult::ok(
                &call.id,
                serde_json::json!({
                    "status": report.status,
                    "id": report.id,
                    "name": report.name,
                    "revision": report.revision,
                    "triggers": report.triggers,
                    "source_kind": report.source_kind.as_str(),
                    "source_ref": report.source_ref,
                    "source_chars": report.source_chars,
                    "archived_count": report.archived_count,
                }),
            )
            .with_name(name),
            Ok(Err(refusal)) => ToolResult::permanent_error(&call.id, refusal).with_name(name),
            Err(error) => {
                ToolResult::permanent_error(&call.id, format!("skill_distill task failed: {error}"))
                    .with_name(name)
            }
        }
    }
}

/// 授权映射 (pre-execute 瀑布): 未显式授权 = require-approval 档。
///
/// 与文件写入的风险映射同一语义: 只回答「这次调用是否有授权」, 审批档由
/// [`SkillDistillApprovalHook`] 唯一承载 (同一审批链)。
pub struct SkillDistillRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl SkillDistillRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子。
    pub fn new(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self { policy }
    }
}

impl PreExecuteHook for SkillDistillRiskMappingHook {
    fn name(&self) -> &str {
        "skill_distill_risk_mapping"
    }

    fn pre_verdict(&self, request: &PreExecuteRequest<'_>) -> PreVerdict {
        let decision = self
            .policy
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .decision_for_capability(request.capability.as_str());
        match decision {
            Decision::Allow => PreVerdict::Allow,
            Decision::RequireApproval { reason } => PreVerdict::Ask { reason },
            Decision::Deny { .. } => PreVerdict::Ask {
                reason: format!(
                    "skill-distill tool {} sits at the require-approval level: \
                     no explicit authorization for it",
                    request.capability.as_str()
                ),
            },
        }
    }
}

/// 单调边界 Guard (只拒不放): 来源越界 / 来源缺省 / 坏卡一律拒绝即帧。
pub struct SkillDistillBoundaryGuard {
    root: PathBuf,
}

impl SkillDistillBoundaryGuard {
    /// 绑定工作区根 (来源路径词法限域) 的边界 Guard。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl ToolGuard for SkillDistillBoundaryGuard {
    fn name(&self) -> &str {
        "skill_distill_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != SKILL_DISTILL_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判来源与坏卡。
        let Ok(params) = serde_json::from_value::<SkillDistillParams>(request.arguments.clone())
        else {
            return None;
        };
        if let Some(refusal) = distill_request_refusal(&params) {
            return Some(refusal);
        }
        if let Some(path) = params
            .path
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
        {
            let joined = self.root.join(path);
            if is_sensitive_path(&self.root, &joined) {
                return Some(credential_surface_refusal());
            }
        }
        None
    }
}

/// 审批档风险映射 (治理层, 与本地审批面板 / IM 审批卡同链)。
///
/// **每次沉淀都停人工审批** (RequireApproval 级, 无自动放行档): 理由载荷 =
/// 技能卡摘要。坏请求拒绝即帧 (错误即帧, 不进审批)。
#[derive(Debug, Clone, Copy, Default)]
pub struct SkillDistillApprovalHook;

impl SkillDistillApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for SkillDistillApprovalHook {
    fn name(&self) -> &str {
        "skill_distill_write_risk"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != SKILL_DISTILL_CAPABILITY_ID {
            return Decision::Allow;
        }
        let Ok(params) = serde_json::from_value::<SkillDistillParams>((*arguments).clone()) else {
            return Decision::Allow;
        };
        match skill_distill_summary(&params) {
            Ok(summary) => Decision::require_approval(summary),
            Err(refusal) => Decision::deny(refusal),
        }
    }
}

/// 生产授权播种: 已注册的沉淀工具即获显式授权 (授权 = 「这次调用有人拍板」)。
///
/// 审批档不在工具策略里 —— 由 [`SkillDistillApprovalHook`] 唯一承载,
/// 与本地审批面板 / IM 审批卡同链。
pub fn authorized_skill_distill_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(apeireth_governance::Permission::ExecuteTool(
            SKILL_DISTILL_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 组装沉淀五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn skill_distill_pipeline(
    root: impl Into<PathBuf>,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    let root = root.into();
    let guard: Arc<dyn ToolGuard> = Arc::new(SkillDistillBoundaryGuard::new(root.clone()));
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(SkillDistillRiskMappingHook::new(policy)))
            .with_guard(guard)
            .with_around(
                AroundPolicy::new().with_timeout(SKILL_DISTILL_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(SkillDistillTool::output_schema()),
    )
}

/// 组装生产工具面: 沉淀管线 + 五段流水线 (模型侧一个能力)。
pub fn skill_distill_capability(
    root: impl Into<PathBuf>,
    library: SkillLibrary,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<dyn ToolCapability> {
    let root = root.into();
    let tool: Arc<dyn ToolCapability> = Arc::new(SkillDistillTool::new(root.clone(), library));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        skill_distill_pipeline(root, policy),
    ))
}
