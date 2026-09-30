//! 受控文件写入的生产工具面 (`tool.apply_patch`, 声明名 `apply_patch`)。
//!
//! 本模块只做**接线**:`crate::apply_patch` 的事务补丁原语与
//! `crate::observed_gate` 的读前观测门禁原样沿用, 这里把它们组装成一个模型
//! 可调用的生产工具, 并补齐生产工具面应有的三件套:
//!
//! 1. **写入风险映射** ([`ApplyPatchRiskMappingHook`]): 未显式授权的写入调用
//!    停在 require-approval 档 (`pipeline.pre_ask` 冻结, 即帧不落盘);
//!    显式授权后放行。风险档位 (每次写入都要人批 / 自动放行已读文件的修改)
//!    由治理层的授权标记与写入风险钩子承载, 与本地审批面板 / IM 审批卡同一条
//!    审批链。
//! 2. **单调边界 Guard** ([`ApplyPatchWriteBoundaryGuard`]): 只拒不放的硬边界
//!    —— 工作区外路径、凭据/密钥面、未读覆盖写一律拒绝; 自动放行档放行的调用
//!    也照穿这道 Guard (自动放行从不绕过硬边界)。
//! 3. **五段流水线**: pre(风险映射) → Guard(单调边界) → around(超时归
//!    `timeout.*`) → post(默认透传) → 输出归一合同 (报告形状冻结)。
//!
//! 创建 / 修改 / 删除都必须在补丁文本里显式声明 (Add / Update / Delete 三类
//! 动作)。删除与新建**永不**进入自动放行档: 任何含创建/删除动作的补丁都停在
//! 人工审批档。
//!
//! # Git 写边界 (件三)
//! git 提交等写操作**不提供工具**, 属设计边界 —— 仓库工具维持只读合同, 见
//! [`GIT_WRITE_BOUNDARY_NOTE`] (工具描述与文档同文, 防模型幻觉工具存在)。

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
use serde::Deserialize;

use crate::apply_patch::{FilePatchAction, TransactionalPatchApplier};
use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, PreExecuteHook, PreExecuteRequest, PreVerdict, SchemaKind,
    DEFAULT_MAX_TIMEOUT_MS,
};
use crate::observed_gate::{GateDenial, ObservationState, ObservedGate};
use crate::sensitive_path::{credential_surface_refusal, is_sensitive_path};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const APPLY_PATCH_CAPABILITY_ID: &str = "tool.apply_patch";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const APPLY_PATCH_TOOL_NAME: &str = "apply_patch";

/// 生产执行时限 (毫秒): 补丁走阻塞池做持久档原子写, 到期即归 `timeout.*`。
pub const APPLY_PATCH_TIMEOUT_MS: u64 = 30_000;

/// git 写边界声明 (件三): 工具描述与文档同文。
///
/// 显式写明「git 提交等写操作不提供工具, 属设计边界」, 避免模型幻觉出一个
/// 并不存在的 git 写工具。
pub const GIT_WRITE_BOUNDARY_NOTE: &str = "Design boundary: git write operations are \
 deliberately not provided as tools — committing, pushing, branch changes, history \
 rewrites and other repository write operations have no tool, and the repository \
 tool stays read-only on purpose. Do not request a git write tool; none exists.";

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const APPLY_PATCH_TOOL_DESCRIPTION: &str = "Apply one transactional multi-file \
 patch inside the workspace root. Every change must be declared in the patch text: \
 *** Add File creates a file, *** Update File edits one with strict-unique \
 search/replace hunks (zero or multiple matches refuse the hunk), and *** Delete \
 File removes one. The patch is all-or-nothing: any failing hunk rolls every file in \
 the patch back. Paths must be workspace-relative; absolute paths, drive or UNC \
 prefixes and `..` components are refused, and symlink escapes are refused at commit \
 time. Credential and secret surfaces (.env family, key material, credential stores) \
 are refused. The read-before-overwrite observation gate applies: a file must have \
 been read in this session before it can be overwritten or deleted. Every write \
 requires human approval by default; a settings switch can auto-pass modifications of \
 files already read, but deletes and creates never auto-pass. ";

/// 工具调用参数 (未知字段拒绝)。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyPatchParams {
    /// 补丁文本 (`*** Begin Patch` ... `*** End Patch`)。
    pub patch: String,
}

/// 自动放行档的放行类别: 只看补丁声明的**动作种类**。
///
/// 「已读文件」这一半由读前观测门禁承担 (未读覆盖写直接拒绝), 两层叠加才是
/// 「自动放行已读文件的修改」。解析失败按最保守档处理 (走人工审批)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteReleaseClass {
    /// 只含 Update 动作 (纯修改类)。
    ModificationOnly,
    /// 含 Add / Delete 动作 (创建/删除永不自动放行)。
    ContainsCreateOrDelete,
    /// 补丁文本无法解析为声明动作 (fail-closed 向人工审批)。
    Unparseable,
}

/// 从调用参数判定放行类别。
pub fn write_release_class_for_arguments(arguments: &serde_json::Value) -> WriteReleaseClass {
    let patch = arguments
        .get("patch")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    write_release_class_for_patch(patch)
}

/// 从补丁文本判定放行类别。
pub fn write_release_class_for_patch(patch: &str) -> WriteReleaseClass {
    match TransactionalPatchApplier::parse_patch(patch) {
        Ok(actions) if actions.iter().all(is_modification) => WriteReleaseClass::ModificationOnly,
        Ok(_) => WriteReleaseClass::ContainsCreateOrDelete,
        Err(_) => WriteReleaseClass::Unparseable,
    }
}

fn is_modification(action: &FilePatchAction) -> bool {
    matches!(action, FilePatchAction::Update { .. })
}

/// 补丁动作的目标路径。
fn action_target(action: &FilePatchAction) -> &Path {
    match action {
        FilePatchAction::Add { path, .. }
        | FilePatchAction::Delete { path }
        | FilePatchAction::Update { path, .. } => path,
    }
}

/// 该动作是否覆盖/删除既有文件 (需先读后写)。
fn action_touches_existing(action: &FilePatchAction) -> bool {
    matches!(
        action,
        FilePatchAction::Delete { .. } | FilePatchAction::Update { .. }
    )
}

/// 目标在共享读前观测门禁里是否「已读」(已观测为存在)。
///
/// 两个拼写都查: 请求拼写 (`root` 直接 join) 与 canonical 根拼写, 跨工具
/// 拼写差异不误伤 (读工具两种拼写都记观测)。
fn observed_present(gate: &ObservedGate, root: &Path, target: &Path) -> bool {
    if matches!(gate.state(&root.join(target)), ObservationState::Present(_)) {
        return true;
    }
    if let Ok(canonical_root) = std::fs::canonicalize(root) {
        return matches!(
            gate.state(&canonical_root.join(target)),
            ObservationState::Present(_)
        );
    }
    false
}

/// 硬边界判定 (纯函数, 只拒不放): 返回 `Some(拒绝即帧文案)` 即拒绝。
///
/// 三层: ① 词法包含 (绝对路径 / 盘符 UNC 前缀 / `..` 组件); ② 敏感面
/// (复用 `sensitive_path` 的凭据/密钥面清单); ③ 读前观测门禁 (未读不得
/// 覆盖/删除)。现实层的 symlink 逃逸校验由补丁原语在提交阶段兜住。
pub(crate) fn boundary_refusal(
    root: &Path,
    gate: &ObservedGate,
    action: &FilePatchAction,
) -> Option<String> {
    let target = action_target(action);
    if target.as_os_str().is_empty() {
        return Some("拒绝写入: 补丁路径为空".to_string());
    }
    if target.is_absolute() {
        return Some(format!(
            "拒绝写入 {}: 禁止绝对路径 (必须是工作区相对路径)",
            target.display()
        ));
    }
    for component in target.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            other => {
                return Some(format!(
                    "拒绝写入 {}: 禁止的路径组件 ({other:?})",
                    target.display()
                ))
            }
        }
    }
    let joined = root.join(target);
    if is_sensitive_path(root, &joined) {
        return Some(credential_surface_refusal());
    }
    if action_touches_existing(action) && !observed_present(gate, root, target) {
        return Some(GateDenial::NotObserved(joined).message());
    }
    None
}

/// 从调用参数解析补丁动作 (解析失败返回错误文本, 由调用方决定如何闭合)。
fn parse_actions(arguments: &serde_json::Value) -> Result<Vec<FilePatchAction>, String> {
    let params: ApplyPatchParams = serde_json::from_value(arguments.clone())
        .map_err(|error| format!("invalid apply_patch parameters: {error}"))?;
    TransactionalPatchApplier::parse_patch(&params.patch).map_err(|error| error.to_string())
}

/// 受控文件写入工具 (`tool.apply_patch`, 声明名 `apply_patch`)。
///
/// 事务补丁原语 (创建/修改/删除显式声明, 严格唯一上下文匹配, 全有或全无) 与
/// 读前观测门禁原样沿用; 本工具补齐生产工具面: 边界即帧、输出归一、错误即帧。
pub struct ApplyPatchTool {
    id: CapabilityId,
    root: PathBuf,
    gate: Arc<ObservedGate>,
}

impl ApplyPatchTool {
    /// 绑定工作区根与共享的读前观测门禁 (与文件读工具共用一张观测表)。
    pub fn new(root: impl Into<PathBuf>, gate: Arc<ObservedGate>) -> Self {
        Self {
            id: CapabilityId::new(APPLY_PATCH_CAPABILITY_ID).expect("valid capability id"),
            root: root.into(),
            gate,
        }
    }

    /// 本工具写入观测的门禁 (供装配/测试共享)。
    pub fn observed_gate(&self) -> &Arc<ObservedGate> {
        &self.gate
    }

    /// 输出归一合同: 成功报告的形状冻结为五字段。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("status", SchemaKind::String)
            .require("files_added", SchemaKind::Array)
            .require("files_updated", SchemaKind::Array)
            .require("files_deleted", SchemaKind::Array)
            .require("total_actions", SchemaKind::Number)
    }

    /// 预演边界检查: 任一动作撞硬边界即返回拒绝文案 (错误即帧)。
    fn precheck(&self, actions: &[FilePatchAction]) -> Result<(), String> {
        for action in actions {
            if let Some(refusal) = boundary_refusal(&self.root, &self.gate, action) {
                return Err(refusal);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl ToolCapability for ApplyPatchTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "patch": {
                        "type": "string",
                        "description": "The whole patch text, from *** Begin Patch to *** End Patch; declare every create/update/delete in it."
                    }
                },
                "required": ["patch"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(APPLY_PATCH_TOOL_NAME)
            .with_description(format!(
                "{APPLY_PATCH_TOOL_DESCRIPTION}{GIT_WRITE_BOUNDARY_NOTE}"
            ))
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = APPLY_PATCH_TOOL_NAME;
        let params: ApplyPatchParams = match serde_json::from_value(call.arguments.clone()) {
            Ok(params) => params,
            Err(error) => {
                return ToolResult::permanent_error(
                    &call.id,
                    format!("invalid apply_patch parameters: {error}"),
                )
                .with_name(name)
            }
        };
        let actions = match TransactionalPatchApplier::parse_patch(&params.patch) {
            Ok(actions) => actions,
            Err(error) => {
                return ToolResult::permanent_error(&call.id, error.to_string()).with_name(name)
            }
        };
        if let Err(refusal) = self.precheck(&actions) {
            return ToolResult::permanent_error(&call.id, refusal).with_name(name);
        }

        // 提交走阻塞池: 持久档原子写 + fsync 不应占住执行器; 到期计时器仍能
        // 触发, 超时归既有 `timeout.*` code 族 (可重试帧)。
        let root = self.root.clone();
        let gate = Arc::clone(&self.gate);
        let patch = params.patch;
        match tokio::task::spawn_blocking(move || {
            TransactionalPatchApplier::apply_with_gate(&root, &patch, &gate)
        })
        .await
        {
            Ok(Ok(report)) => ToolResult::ok(
                &call.id,
                serde_json::json!({
                    "status": "applied",
                    "files_added": report.files_added,
                    "files_updated": report.files_updated,
                    "files_deleted": report.files_deleted,
                    "total_actions": report.total_actions,
                }),
            )
            .with_name(name),
            Ok(Err(error)) => {
                ToolResult::permanent_error(&call.id, error.to_string()).with_name(name)
            }
            Err(error) => {
                ToolResult::permanent_error(&call.id, format!("apply_patch task failed: {error}"))
                    .with_name(name)
            }
        }
    }
}

/// 写入风险映射 (pre-execute 瀑布): 未显式授权 = require-approval 档。
///
/// 判定顺序 (与外部工具风险映射同一语义):
///
/// 1. 共享 [`PermissionPolicy`] 里有显式授权 (grant 且未标审批) → 放行;
/// 2. 授权被标了人工审批 → require-approval 档 (`ask`);
/// 3. 无授权 → 同样停在 require-approval 档 (`ask`), 拒绝即帧
///    (`pipeline.pre_ask`)、不落盘。
///
/// 这里只回答「这次调用是否有人的授权」; 风险档位 (每次写入都要人批 /
/// 自动放行已读文件的修改) 由治理层承载, 与本地审批面板 / IM 审批卡同链。
pub struct ApplyPatchRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl ApplyPatchRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子。
    pub fn new(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self { policy }
    }
}

impl PreExecuteHook for ApplyPatchRiskMappingHook {
    fn name(&self) -> &str {
        "apply_patch_risk_mapping"
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
                    "file-write tool {} sits at the require-approval level: \
                     no explicit authorization for it",
                    request.capability.as_str()
                ),
            },
        }
    }
}

/// 单调边界 Guard (只拒不放): 工作区外 / 敏感面 / 未读覆盖写。
///
/// 自动放行档放行的调用也照穿本 Guard —— 自动放行从不绕过硬边界; 任何后续
/// 阶段都无法把这里的拒绝翻回放行 (单调性)。
pub struct ApplyPatchWriteBoundaryGuard {
    root: PathBuf,
    gate: Arc<ObservedGate>,
}

impl ApplyPatchWriteBoundaryGuard {
    /// 绑定工作区根与共享读前观测门禁的边界 Guard。
    pub fn new(root: impl Into<PathBuf>, gate: Arc<ObservedGate>) -> Self {
        Self {
            root: root.into(),
            gate,
        }
    }
}

impl ToolGuard for ApplyPatchWriteBoundaryGuard {
    fn name(&self) -> &str {
        "apply_patch_write_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != APPLY_PATCH_CAPABILITY_ID {
            return None;
        }
        // 解析失败不归边界 Guard (由工具即帧报解析错误), 只判硬边界。
        let Ok(actions) = parse_actions(request.arguments) else {
            return None;
        };
        actions
            .iter()
            .find_map(|action| boundary_refusal(&self.root, &self.gate, action))
    }
}

/// 写入风险档位钩子 (治理层, 与本地审批面板 / IM 审批卡同链)。
///
/// **自动放行档**的放行规则: 只含修改动作 (Update) 的补丁放行; 含创建/删除
/// 动作的补丁停在 [`Decision::RequireApproval`] —— 「删除/新建永不自动放行」
/// 单调生效, 每次此类写入都进人工审批 (审批四态闭合 / IM 审批卡)。「只放行
/// 已读文件的修改」的另一半由读前观测门禁承担 (未读覆盖写直接拒绝)。
///
/// 默认档不挂本钩子: 授权标记 (`require_approval_for`) 让每次写入都停在
/// 人工审批档。
#[derive(Debug, Clone, Copy, Default)]
pub struct ApplyPatchWriteApprovalHook;

impl ApplyPatchWriteApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for ApplyPatchWriteApprovalHook {
    fn name(&self) -> &str {
        "apply_patch_write_risk"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != APPLY_PATCH_CAPABILITY_ID {
            return Decision::Allow;
        }
        match write_release_class_for_arguments(arguments) {
            WriteReleaseClass::ModificationOnly => Decision::Allow,
            WriteReleaseClass::ContainsCreateOrDelete => Decision::require_approval(format!(
                "file-write patch declares create/delete actions ({APPLY_PATCH_CAPABILITY_ID}): \
                 creates and deletes never auto-pass and wait for a human decision"
            )),
            WriteReleaseClass::Unparseable => Decision::require_approval(format!(
                "file-write patch text could not be parsed into declared actions \
                 ({APPLY_PATCH_CAPABILITY_ID}): fail closed toward human approval"
            )),
        }
    }
}

/// 生产授权播种: 已注册的写入工具即获显式授权 (授权 = 「这次调用有人拍板」)。
///
/// 风险档位 (每次写入都要人批 / 自动放行已读文件的修改) 不在工具策略里 ——
/// 由治理层的授权标记与写入风险钩子承载, 与本地审批面板 / IM 审批卡同链。
pub fn authorized_file_write_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(apeireth_governance::Permission::ExecuteTool(
            APPLY_PATCH_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 组装生产五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn apply_patch_pipeline(
    root: impl Into<PathBuf>,
    gate: Arc<ObservedGate>,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    let root = root.into();
    let guard: Arc<dyn ToolGuard> =
        Arc::new(ApplyPatchWriteBoundaryGuard::new(root, Arc::clone(&gate)));
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(ApplyPatchRiskMappingHook::new(policy)))
            .with_guard(guard)
            .with_around(
                AroundPolicy::new().with_timeout(APPLY_PATCH_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(ApplyPatchTool::output_schema()),
    )
}

/// 组装生产工具面: 补丁原语 + 五段流水线 (模型侧一个能力)。
pub fn apply_patch_capability(
    root: impl Into<PathBuf>,
    gate: Arc<ObservedGate>,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<dyn ToolCapability> {
    let root = root.into();
    let tool = Arc::new(ApplyPatchTool::new(root.clone(), Arc::clone(&gate)));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        apply_patch_pipeline(root, gate, policy),
    ))
}
