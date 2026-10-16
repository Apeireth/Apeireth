//! 受控工作树写工具面 (`tool.worktree_apply`, 声明名 `worktree_apply`) —— 干活能力轴 P2。
//!
//! 多文件大改走**四拍受控写** (计划→改→验证→落), 每拍可审、任一失败整批
//! 不落地:
//!
//! ```text
//! 计划拍   改单校验 + 补丁解析 + 交叉一致 + 边界预检   (零写入)
//! 改拍     暂存面物化 + 事务补丁原语执行               (工作区零写入)
//! 验证拍   纯结构校验 (恒) + 校验命令 (可挂, 受控通道)  (工作区零写入)
//! 落拍     事务补丁原语过观测门禁落真实工作区           (全有或全无)
//! ```
//!
//! 语义要点:
//!
//! 1. **按改单执行**: [`crate::worktree_plan`] 产的改单是输入合同; 计划拍把
//!    补丁声明的动作与改单逐项核对 (文件集 + 动作标签), 不一致拒绝即帧
//!    ([`REFUSAL_DRIFT_PREFIX`]) —— 改单之外的写无处混入。
//! 2. **暂存面执行** ([`crate::apply_patch`] 原语复用): 改拍把改单涉及的既有
//!    文件内容物化进系统临时目录下的工作树式暂存面, 再用事务补丁原语在暂存面
//!    上执行; 补丁错误 (上下文不匹配等) 在暂存面即爆, **工作区零写入**。
//! 3. **验证拍**: 纯结构校验恒有 (暂存面文件集与改单逐项相符), 可挂校验命令
//!    走受控通道 (结构化 argv、无 shell 解析、工作目录限域暂存面、
//!    `ProcessLimits` 超时); 任一验证不过 → 整批未落地 ([`REFUSAL_VERIFY_PREFIX`])。
//! 4. **落拍全有或全无**: 同一补丁过读前观测门禁 + 持久档原子写落真实工作区
//!    (事务语义复用: 任一失败整批回滚)。观测门禁语义不变: 未读不得覆盖写/删。
//! 5. **写总闸语义**: 与 `tool.apply_patch` 同受 `file_write` 唯一写总闸
//!    (注册即闸, 组装根同一开关); 工具不在册 = 写闸关。
//!
//! # 治理 (两级工具均过五段流水线)
//!
//! - 计划拍 [`crate::worktree_plan`]: 只读档**零审批** (无 ask 档钩子);
//! - 本工具: **RequireApproval 级** —— 每次落盘都停人工审批, 无自动放行档;
//!   审批卡载荷 = 改单摘要 (文件数 / 动作分布 / 验证命令), 见
//!   [`WorktreeApplyApprovalHook`]; `read_only` 预设拒本工具;
//! - [`WorktreeApplyRiskMappingHook`] (授权映射) + [`WorktreeApplyBoundaryGuard`]
//!   (单调边界, 只拒不放) + 超时 + 输出归一合同。
//!
//! # Git 写边界 (与 `tool.git_write` 分工)
//!
//! 本工具**不含 git 写**: 改单只动文件; 提交走 [`crate::git_write`] 的分级
//! 词表 (L1 可逆默认人批), 见 [`crate::worktree_plan::WORKTREE_GIT_BOUNDARY_NOTE`]。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use apeireth_core::deadline::TimeoutErrorCode;
use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{
    redact_secret_surfaces, Action, Decision, GovernanceHook, GovernanceRequest, PermissionPolicy,
    ToolGuard, ToolGuardRequest,
};
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::apply_patch::{FilePatchAction, PatchReport, TransactionalPatchApplier};
use crate::apply_patch_tool::boundary_refusal;
use crate::exec_pipeline::{
    AroundPolicy, OutputSchema, PreExecuteHook, PreExecuteRequest, PreVerdict, SchemaKind,
    DEFAULT_MAX_TIMEOUT_MS,
};
use crate::observed_gate::ObservedGate;
use crate::process::{ProcessExecutor, ProcessLimits, ProcessRequest};
use crate::worktree_plan::{
    change_order_refusal, normalized_path_key, ActionDistribution, WorktreeChangeAction,
    WorktreeChangeOrder, WORKTREE_GIT_BOUNDARY_NOTE,
};

/// 稳定能力标识 (注册表 / 治理授权 / 风险映射共用)。
pub const WORKTREE_APPLY_CAPABILITY_ID: &str = "tool.worktree_apply";

/// 模型侧声明名 (短名, 每次请求都进上下文)。
pub const WORKTREE_APPLY_TOOL_NAME: &str = "worktree_apply";

/// 生产执行时限 (毫秒): 改拍 + 验证拍 + 落拍合计, 到期归 `timeout.*`。
/// 校验命令另有各自的 `ProcessLimits` 时限 (默认档)。
pub const WORKTREE_APPLY_TIMEOUT_MS: u64 = 120_000;

/// 改单与补丁不一致拒绝帧的稳定前缀 (计划拍, 零写入)。
pub const REFUSAL_DRIFT_PREFIX: &str = "改单与补丁不一致拒绝即帧";

/// 验证拍失败拒绝帧的稳定前缀 (整批未落地)。
pub const REFUSAL_VERIFY_PREFIX: &str = "验证拍失败即帧";

/// 工具描述全文 (注册声明与文档引用同一份文本)。
pub const WORKTREE_APPLY_TOOL_DESCRIPTION: &str = "Land one approved multi-file \
  change order through four controlled beats. Plan check: the patch text's \
  declared actions must match the change order exactly - same target files and \
  the same add / update / delete actions; any drift is refused before anything \
  runs. Staging: the patch is applied to a staging surface with the \
  transactional patch primitive, so patch errors never touch the workspace. \
  Verification: pure structural validation always runs (the staged files match \
  the change order item by item), and optional verification commands run as \
  structured arguments inside the staging surface under a timeout - a failing \
  verification aborts the whole batch with nothing landed. Landing: the same \
  patch lands in the workspace through the transactional patch primitive with \
  the read-before-overwrite observation gate and durable atomic writes - the \
  batch is all-or-nothing and any failure rolls every file back. Paths must be \
  workspace-relative; absolute paths, drive or UNC prefixes and `..` components \
  are refused; credential and secret surfaces are refused; a file must have been \
  read in this session before it can be overwritten or deleted. Every apply \
  requires a human decision (require-approval level, no auto-pass tier): the \
  approval payload carries the change order summary - file count, action \
  distribution and verification commands. The read_only permission preset \
  refuses this tool. ";

/// 校验命令 (验证拍可挂面): 结构化 argv, 不经 shell 解析。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyCommand {
    /// 可执行程序 (非空; 由受控通道按程序名解析, 无 PATH 外壳展开)。
    pub program: String,
    /// 参数面 (逐字传递)。
    #[serde(default)]
    pub args: Vec<String>,
}

impl VerifyCommand {
    /// 人读命令面 (已脱敏 —— 凭据不入日志/审批卡)。
    pub fn display(&self) -> String {
        let mut text = self.program.clone();
        for arg in &self.args {
            text.push(' ');
            text.push_str(arg);
        }
        redact_secret_surfaces(text.trim())
    }
}

/// 工具调用参数 (未知字段拒绝)。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeApplyParams {
    /// 改单 (与 `worktree_plan` 产物同形: goal + 逐文件 path/action/expect)。
    pub plan: WorktreeChangeOrder,
    /// 实现改单的补丁文本 (`*** Begin Patch` ... `*** End Patch`)。
    pub patch: String,
    /// 验证拍可挂的校验命令 (缺省 = 纯结构校验)。
    #[serde(default)]
    pub verify: Vec<VerifyCommand>,
}

/// 已通过计划拍校验的执行件 (改单 + 补丁动作 + 补丁文本 + 校验命令)。
#[derive(Debug, Clone)]
pub struct PreparedApply {
    /// 改单。
    pub order: WorktreeChangeOrder,
    /// 解析出的补丁动作 (与改单逐项相符)。
    pub actions: Vec<FilePatchAction>,
    /// 补丁文本。
    pub patch: String,
    /// 校验命令。
    pub verify: Vec<VerifyCommand>,
}

/// 审批卡载荷: 改单摘要 (文件数 / 动作分布 / 验证命令) + 档位声明。
pub fn apply_summary(order: &WorktreeChangeOrder, verify: &[VerifyCommand]) -> String {
    let verify_text = if verify.is_empty() {
        "验证命令 0 条 (纯结构校验)".to_string()
    } else {
        let displays = verify
            .iter()
            .map(VerifyCommand::display)
            .collect::<Vec<_>>()
            .join(" ; ");
        format!("验证命令 {} 条: {displays}", verify.len())
    };
    format!(
        "worktree apply ({WORKTREE_APPLY_CAPABILITY_ID}) 审批级落盘, 每次都要人批 \
         (无自动放行档)。{}; {}",
        order.summary(),
        verify_text
    )
}

/// 改单动作标签 → 补丁动作标签 (交叉核对同文)。
fn action_label(action: &FilePatchAction) -> &'static str {
    match action {
        FilePatchAction::Add { .. } => "add",
        FilePatchAction::Update { .. } => "update",
        FilePatchAction::Delete { .. } => "delete",
    }
}

/// 补丁动作的目标路径。
fn action_target(action: &FilePatchAction) -> &Path {
    match action {
        FilePatchAction::Add { path, .. }
        | FilePatchAction::Delete { path }
        | FilePatchAction::Update { path, .. } => path,
    }
}

/// 计划拍: 改单与补丁的交叉一致核对 (文件集 + 动作标签逐项相等)。
///
/// 三个拒绝面: 补丁动了改单之外的目标 / 改单有补丁未动的目标 / 同一目标
/// 动作标签不一致 (含补丁内同一目标重复声明)。文件集之外的写无处混入。
pub fn cross_check_change_order(
    order: &WorktreeChangeOrder,
    actions: &[FilePatchAction],
) -> Result<(), String> {
    let mut order_map: BTreeMap<String, (String, &'static str)> = BTreeMap::new();
    for file in &order.files {
        order_map.insert(
            normalized_path_key(&file.path),
            (file.path.trim().to_string(), file.action.as_str()),
        );
    }
    let mut patch_map: BTreeMap<String, (String, &'static str)> = BTreeMap::new();
    for action in actions {
        let target = action_target(action).to_string_lossy().into_owned();
        let key = normalized_path_key(&target);
        if let Some((previous_path, previous_label)) =
            patch_map.insert(key, (target.clone(), action_label(action)))
        {
            return Err(format!(
                "{REFUSAL_DRIFT_PREFIX}: 补丁重复声明同一目标 {previous_path} ({previous_label}) \
                 与 {target} ({})",
                action_label(action)
            ));
        }
    }
    for (key, (path, label)) in &patch_map {
        match order_map.get(key) {
            None => {
                return Err(format!(
                    "{REFUSAL_DRIFT_PREFIX}: 补丁声明了改单之外的目标 {path} ({label})"
                ))
            }
            Some((_, order_label)) if *order_label != *label => {
                return Err(format!(
                    "{REFUSAL_DRIFT_PREFIX}: 目标 {path} 动作不一致 (改单 {order_label}, 补丁 {label})"
                ));
            }
            Some(_) => {}
        }
    }
    for (key, (path, label)) in &order_map {
        if !patch_map.contains_key(key) {
            return Err(format!(
                "{REFUSAL_DRIFT_PREFIX}: 改单声明了补丁未动的目标 {path} ({label})"
            ));
        }
    }
    Ok(())
}

/// 校验命令合法性: 程序名非空 (参数面逐字传递, 不做 shell 解析)。
fn validate_verify_commands(verify: &[VerifyCommand]) -> Result<(), String> {
    for command in verify {
        if command.program.trim().is_empty() {
            return Err(format!("{REFUSAL_VERIFY_PREFIX}: 校验命令的程序名不能为空"));
        }
    }
    Ok(())
}

/// 计划拍校验 (不含读前观测门禁 —— 门禁是会话态, 归边界预检/Guard):
/// 参数解析 → 改单校验 → 补丁解析 → 交叉一致 → 校验命令合法。
pub fn prepare_apply(root: &Path, arguments: &serde_json::Value) -> Result<PreparedApply, String> {
    let params: WorktreeApplyParams = serde_json::from_value(arguments.clone())
        .map_err(|error| format!("invalid worktree_apply parameters: {error}"))?;
    if let Some(refusal) = change_order_refusal(root, &params.plan) {
        return Err(refusal);
    }
    let actions =
        TransactionalPatchApplier::parse_patch(&params.patch).map_err(|error| error.to_string())?;
    cross_check_change_order(&params.plan, &actions)?;
    validate_verify_commands(&params.verify)?;
    Ok(PreparedApply {
        order: params.plan,
        actions,
        patch: params.patch,
        verify: params.verify,
    })
}

/// 边界预检: 计划拍校验 + 逐动作硬边界 (工作区包含 / 敏感面 / 未读不改)。
///
/// 任一撞边界即返回拒绝文案 (错误即帧); 与 [`WorktreeApplyBoundaryGuard`]
/// 同一判定函数, 预检与 Guard 不各说各话。
pub fn apply_boundary_refusal(
    root: &Path,
    gate: &ObservedGate,
    arguments: &serde_json::Value,
) -> Option<String> {
    let prepared = match prepare_apply(root, arguments) {
        Ok(prepared) => prepared,
        Err(refusal) => return Some(refusal),
    };
    prepared
        .actions
        .iter()
        .find_map(|action| boundary_refusal(root, gate, action))
}

/// 落盘报告 (输出归一合同: 形状冻结)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorktreeApplyReport {
    /// 改单身份摘要 (与审批卡/计划产物同文配对)。
    pub plan_id: String,
    /// 改单文件计数。
    pub file_count: usize,
    /// 动作分布。
    pub action_distribution: ActionDistribution,
    /// 验证拍账目 (结构校验 + 逐条校验命令)。
    pub verification: Vec<String>,
    /// 落地的新建文件。
    pub files_added: Vec<String>,
    /// 落地的修改文件。
    pub files_updated: Vec<String>,
    /// 落地的删除文件。
    pub files_deleted: Vec<String>,
}

impl WorktreeApplyReport {
    /// 从改单与验证账目组装报告。
    fn new(order: &WorktreeChangeOrder, verification: Vec<String>, landed: PatchReport) -> Self {
        Self {
            plan_id: order.plan_id(),
            file_count: order.file_count(),
            action_distribution: order.action_distribution(),
            verification,
            files_added: landed.files_added,
            files_updated: landed.files_updated,
            files_deleted: landed.files_deleted,
        }
    }
}

/// 工作树式暂存面: 系统临时目录下的批次暂存区 (改拍/验证拍的执行面)。
///
/// 只承载本批改单涉及的文件内容 (字节级物化, 不搬权限属性 —— 属性差异属落拍
/// 阶段的现实, 不在暂存面伪造); 生命周期随批次, [`Drop`] 即清理 (半成品不弃尸)。
struct StageSurface {
    root: PathBuf,
}

impl StageSurface {
    fn create() -> Result<Self, String> {
        static NONCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "apeireth-worktree-stage-{}-{}",
            std::process::id(),
            NONCE.fetch_add(1, Ordering::Relaxed)
        ));
        // 同进程序号不复用残留 (崩溃残留也不带入新批次)。
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)
            .map_err(|error| format!("暂存面创建失败: {error} ({})", root.display()))?;
        Ok(Self { root })
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for StageSurface {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// 改拍物化: 把改单涉及的既有文件 (Update/Delete 目标) 内容搬进暂存面。
///
/// Add 目标不物化 —— 暂存面与真面同语义 (目标必须缺席)。路径已经过计划拍的
/// 词法包含/敏感面校验; 暂存面与落拍各自的 symlink 现实校验由事务原语兜住。
fn materialize(stage_root: &Path, root: &Path, actions: &[FilePatchAction]) -> Result<(), String> {
    for action in actions {
        let path = match action {
            FilePatchAction::Add { .. } => continue,
            FilePatchAction::Update { path, .. } | FilePatchAction::Delete { path } => path,
        };
        let source = root.join(path);
        let target = stage_root.join(path);
        let bytes = std::fs::read(&source)
            .map_err(|error| format!("暂存面物化失败 {}: {error}", path.display()))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("暂存面物化失败 {}: {error}", path.display()))?;
        }
        std::fs::write(&target, bytes)
            .map_err(|error| format!("暂存面物化失败 {}: {error}", path.display()))?;
    }
    Ok(())
}

/// 验证拍·纯结构校验 (恒有): 暂存面文件集与改单逐项相符。
///
/// 三个断言: ① add/update 目标全部在暂存面; ② delete 目标全部已离场;
/// ③ 暂存面无改单之外的残留文件。结构校验不过 = 验证拍失败即帧。
fn structural_verify(stage_root: &Path, order: &WorktreeChangeOrder) -> Result<(), String> {
    let mut expected_present: BTreeSet<String> = BTreeSet::new();
    let mut expected_absent: BTreeSet<String> = BTreeSet::new();
    for file in &order.files {
        let key = normalized_path_key(&file.path);
        match file.action {
            WorktreeChangeAction::Add | WorktreeChangeAction::Update => {
                expected_present.insert(key);
            }
            WorktreeChangeAction::Delete => {
                expected_absent.insert(key);
            }
        }
    }
    let staged = walk_file_keys(stage_root, stage_root).map_err(|error| {
        format!("{REFUSAL_VERIFY_PREFIX} (纯结构校验): 暂存面遍历失败: {error}")
    })?;
    for key in &expected_present {
        if !staged.contains(key) {
            return Err(format!(
                "{REFUSAL_VERIFY_PREFIX} (纯结构校验): 暂存面缺少预期文件 {key}"
            ));
        }
    }
    for key in &expected_absent {
        if staged.contains(key) {
            return Err(format!(
                "{REFUSAL_VERIFY_PREFIX} (纯结构校验): 暂存面仍存在应删除文件 {key}"
            ));
        }
    }
    for key in &staged {
        if !expected_present.contains(key) {
            return Err(format!(
                "{REFUSAL_VERIFY_PREFIX} (纯结构校验): 暂存面存在改单之外的文件 {key}"
            ));
        }
    }
    Ok(())
}

/// 递归收集相对文件键 (`/` 分隔, 与 [`normalized_path_key`] 同文)。
fn walk_file_keys(root: &Path, dir: &Path) -> Result<BTreeSet<String>, String> {
    let mut keys = BTreeSet::new();
    let entries = std::fs::read_dir(dir).map_err(|error| error.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            keys.extend(walk_file_keys(root, &path)?);
        } else {
            let relative = path
                .strip_prefix(root)
                .map_err(|error| error.to_string())?
                .to_string_lossy()
                .replace('\\', "/");
            keys.insert(normalized_path_key(&relative));
        }
    }
    Ok(keys)
}

/// 验证拍·校验命令 (受控通道): 结构化 argv、无 shell 解析、工作目录限域
/// 暂存面、`ProcessLimits` 超时; 无法启动/超时/非零退出一律验证拍失败即帧。
fn run_verification_commands(
    stage_root: &Path,
    verify: &[VerifyCommand],
) -> Result<Vec<String>, String> {
    let executor = ProcessExecutor::new();
    let mut entries = Vec::new();
    for command in verify {
        let display = command.display();
        let request = ProcessRequest::new(&command.program)
            .with_args(command.args.clone())
            .with_working_directory(stage_root)
            .with_limits(ProcessLimits::default());
        let result = executor.execute(&request).map_err(|error| {
            format!("{REFUSAL_VERIFY_PREFIX} (校验命令无法启动, 整批未落地): {display} — {error}")
        })?;
        let mut output = String::new();
        if !result.stdout.is_empty() {
            output.push_str(&String::from_utf8_lossy(&result.stdout));
        }
        if !result.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&String::from_utf8_lossy(&result.stderr));
        }
        let output = redact_secret_surfaces(&output);
        if result.timed_out() {
            return Err(format!(
                "{REFUSAL_VERIFY_PREFIX} (校验命令超时, 整批未落地): {display} — {} ({}ms)",
                TimeoutErrorCode::DeadlineExpired.as_str(),
                ProcessLimits::default().max_runtime.as_millis()
            ));
        }
        if !result.success() {
            let code = result
                .exit_code()
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".into());
            let detail = if output.is_empty() {
                "(无输出)"
            } else {
                output.as_str()
            };
            return Err(format!(
                "{REFUSAL_VERIFY_PREFIX} (校验命令非零退出, 整批未落地): {display} — \
                 exit {code}: {detail}"
            ));
        }
        entries.push(format!("$ {display}"));
    }
    Ok(entries)
}

/// 四拍执行 (阻塞池): 改拍 → 验证拍 → 落拍; 计划拍已在调用侧完成。
///
/// 落拍前工作区**零写入**; 落拍复用 [`TransactionalPatchApplier`] 的事务
/// 语义 (预演 + 观测门禁 CAS + 持久档原子写 + 任一失败整批回滚)。
fn execute_four_beats(
    root: &Path,
    gate: &ObservedGate,
    prepared: &PreparedApply,
) -> Result<WorktreeApplyReport, String> {
    let order = &prepared.order;

    // 改拍: 暂存面物化 + 事务补丁原语执行。
    let stage = StageSurface::create()?;
    materialize(stage.root(), root, &prepared.actions)?;
    let staged = TransactionalPatchApplier::apply(stage.root(), &prepared.patch)
        .map_err(|error| format!("改拍失败 (暂存面执行, 工作区零写入): {error}"))?;

    // 验证拍: 纯结构校验 (恒) + 校验命令 (可挂)。
    structural_verify(stage.root(), order)?;
    let mut verification = vec![format!(
        "结构校验通过: 暂存面与改单逐项相符 (改拍落地 {} 动作)",
        staged.total_actions
    )];
    verification.extend(run_verification_commands(stage.root(), &prepared.verify)?);

    // 落拍: 全有或全无, 任一失败整批回滚。
    let landed = TransactionalPatchApplier::apply_with_gate(root, &prepared.patch, gate)
        .map_err(|error| format!("落拍失败 (全有或全无, 整批回滚): {error}"))?;
    Ok(WorktreeApplyReport::new(order, verification, landed))
}

/// 受控工作树写工具 (`tool.worktree_apply`, 声明名 `worktree_apply`)。
pub struct WorktreeApplyTool {
    id: CapabilityId,
    root: PathBuf,
    gate: Arc<ObservedGate>,
}

impl WorktreeApplyTool {
    /// 绑定工作区根与共享读前观测门禁 (与文件读工具共用一张观测表)。
    pub fn new(root: impl Into<PathBuf>, gate: Arc<ObservedGate>) -> Self {
        Self {
            id: CapabilityId::new(WORKTREE_APPLY_CAPABILITY_ID).expect("valid capability id"),
            root: root.into(),
            gate,
        }
    }

    /// 本工具写入观测的门禁 (供装配/测试共享)。
    pub fn observed_gate(&self) -> &Arc<ObservedGate> {
        &self.gate
    }

    /// 输出归一合同: 落盘报告的形状冻结为八字段。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("status", SchemaKind::String)
            .require("plan_id", SchemaKind::String)
            .require("file_count", SchemaKind::Number)
            .require("action_distribution", SchemaKind::Object)
            .require("verification", SchemaKind::Array)
            .require("files_added", SchemaKind::Array)
            .require("files_updated", SchemaKind::Array)
            .require("files_deleted", SchemaKind::Array)
    }
}

#[async_trait]
impl ToolCapability for WorktreeApplyTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut parameters = ToolParameters::new();
        parameters.extend(
            serde_json::json!({
                "type": "object",
                "properties": {
                    "plan": {
                        "type": "object",
                        "description": "The change order to land: {goal, files: [{path, action, expect}]}, exactly as worktree_plan produced it.",
                        "properties": {
                            "goal": { "type": "string" },
                            "files": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "path": { "type": "string" },
                                        "action": { "type": "string", "enum": ["add", "update", "delete"] },
                                        "expect": { "type": "string" }
                                    },
                                    "required": ["path", "action", "expect"],
                                    "additionalProperties": false
                                }
                            }
                        },
                        "required": ["goal", "files"],
                        "additionalProperties": false
                    },
                    "patch": {
                        "type": "string",
                        "description": "The patch that realizes the change order, from *** Begin Patch to *** End Patch; its declared actions must match the change order exactly."
                    },
                    "verify": {
                        "type": "array",
                        "description": "Optional verification commands for the verification beat: [{program, args}]. They run as structured arguments in the staging surface; omit for pure structural validation.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "program": { "type": "string" },
                                "args": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["program"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["plan", "patch"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(WORKTREE_APPLY_TOOL_NAME)
            .with_description(format!(
                "{WORKTREE_APPLY_TOOL_DESCRIPTION}{WORKTREE_GIT_BOUNDARY_NOTE}"
            ))
            .with_parameters(parameters)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let name = WORKTREE_APPLY_TOOL_NAME;
        // 计划拍: 改单/补丁/校验命令校验 + 逐动作硬边界 (零写入)。
        if let Some(refusal) = apply_boundary_refusal(&self.root, &self.gate, &call.arguments) {
            return ToolResult::permanent_error(&call.id, refusal).with_name(name);
        }
        let prepared = match prepare_apply(&self.root, &call.arguments) {
            Ok(prepared) => prepared,
            Err(refusal) => return ToolResult::permanent_error(&call.id, refusal).with_name(name),
        };

        // 改拍 + 验证拍 + 落拍走阻塞池 (物化/子进程/持久档原子写不占执行器);
        // 到期计时器仍能触发, 超时归既有 `timeout.*` code 族。
        let root = self.root.clone();
        let gate = Arc::clone(&self.gate);
        match tokio::task::spawn_blocking(move || execute_four_beats(&root, &gate, &prepared)).await
        {
            Ok(Ok(report)) => ToolResult::ok(
                &call.id,
                serde_json::json!({
                    "status": "applied",
                    "plan_id": report.plan_id,
                    "file_count": report.file_count,
                    "action_distribution": {
                        "add": report.action_distribution.add,
                        "update": report.action_distribution.update,
                        "delete": report.action_distribution.delete,
                    },
                    "verification": report.verification,
                    "files_added": report.files_added,
                    "files_updated": report.files_updated,
                    "files_deleted": report.files_deleted,
                }),
            )
            .with_name(name),
            Ok(Err(frame)) => ToolResult::permanent_error(&call.id, frame).with_name(name),
            Err(error) => ToolResult::permanent_error(
                &call.id,
                format!("worktree_apply task failed: {error}"),
            )
            .with_name(name),
        }
    }
}

// ---------------------------------------------------------------------------
// 治理面: 授权映射 / 单调边界 / 审批级风险映射
// ---------------------------------------------------------------------------

/// 授权映射 (pre-execute 瀑布): 未显式授权 = require-approval 档。
///
/// 与文件写入的风险映射同一语义: 本钩子只回答「这次调用是否有授权」,
/// 审批级档位由 [`WorktreeApplyApprovalHook`] 承载 (同一条审批链)。
pub struct WorktreeApplyRiskMappingHook {
    policy: Arc<Mutex<PermissionPolicy>>,
}

impl WorktreeApplyRiskMappingHook {
    /// 挂在共享授权策略上的风险映射钩子。
    pub fn new(policy: Arc<Mutex<PermissionPolicy>>) -> Self {
        Self { policy }
    }
}

impl PreExecuteHook for WorktreeApplyRiskMappingHook {
    fn name(&self) -> &str {
        "worktree_apply_risk_mapping"
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
                    "worktree-write tool {} sits at the require-approval level: \
                     no explicit authorization for it",
                    request.capability.as_str()
                ),
            },
        }
    }
}

/// 单调边界 Guard (只拒不放): 计划拍校验不过 / 工作区外 / 敏感面 / 未读
/// 覆盖写一律拒绝; 自动放行档放行的调用也照穿本 Guard。
pub struct WorktreeApplyBoundaryGuard {
    root: PathBuf,
    gate: Arc<ObservedGate>,
}

impl WorktreeApplyBoundaryGuard {
    /// 绑定工作区根与共享读前观测门禁的边界 Guard。
    pub fn new(root: impl Into<PathBuf>, gate: Arc<ObservedGate>) -> Self {
        Self {
            root: root.into(),
            gate,
        }
    }
}

impl ToolGuard for WorktreeApplyBoundaryGuard {
    fn name(&self) -> &str {
        "worktree_apply_boundary"
    }

    fn deny(&self, request: &ToolGuardRequest<'_>) -> Option<String> {
        if request.capability.as_str() != WORKTREE_APPLY_CAPABILITY_ID {
            return None;
        }
        apply_boundary_refusal(&self.root, &self.gate, request.arguments)
    }
}

/// 审批级风险映射 (治理层, 与本地审批面板 / IM 审批卡同链)。
///
/// **每次落盘都停人工审批** (`RequireApproval` 级, 无自动放行档): 理由载荷 =
/// 改单摘要 (文件数 / 动作分布 / 验证命令)。计划拍校验不过的调用拒绝即帧
/// (错误即帧, 不进审批)。
pub struct WorktreeApplyApprovalHook {
    root: PathBuf,
}

impl WorktreeApplyApprovalHook {
    /// 绑定工作区根 (改单校验限域) 的审批级风险映射。
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

#[async_trait]
impl GovernanceHook for WorktreeApplyApprovalHook {
    fn name(&self) -> &str {
        "worktree_apply_write_risk"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != WORKTREE_APPLY_CAPABILITY_ID {
            return Decision::Allow;
        }
        match prepare_apply(&self.root, arguments) {
            Ok(prepared) => {
                Decision::require_approval(apply_summary(&prepared.order, &prepared.verify))
            }
            Err(refusal) => Decision::deny(refusal),
        }
    }
}

/// 生产授权播种: 已注册的工作树写工具即获显式授权 (授权 = 「这次调用有人拍板」)。
///
/// 审批级档位不在工具策略里 —— 由 [`WorktreeApplyApprovalHook`] 承载,
/// 与本地审批面板 / IM 审批卡同链。
pub fn authorized_worktree_apply_policy() -> Arc<Mutex<PermissionPolicy>> {
    let policy = Arc::new(Mutex::new(PermissionPolicy::new()));
    policy
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .grant(apeireth_governance::Permission::ExecuteTool(
            WORKTREE_APPLY_CAPABILITY_ID.to_string(),
        ));
    policy
}

/// 组装落盘五段流水线: 风险映射 → 单调边界 Guard → 超时 → 输出归一合同。
pub fn worktree_apply_pipeline(
    root: impl Into<PathBuf>,
    gate: Arc<ObservedGate>,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<crate::exec_pipeline::ToolExecutionPipeline> {
    let root = root.into();
    let guard: Arc<dyn ToolGuard> = Arc::new(WorktreeApplyBoundaryGuard::new(
        root.clone(),
        Arc::clone(&gate),
    ));
    Arc::new(
        crate::exec_pipeline::ToolExecutionPipeline::new()
            .with_pre_hook(Arc::new(WorktreeApplyRiskMappingHook::new(policy)))
            .with_guard(guard)
            .with_around(
                AroundPolicy::new().with_timeout(WORKTREE_APPLY_TIMEOUT_MS, DEFAULT_MAX_TIMEOUT_MS),
            )
            .with_output_schema(WorktreeApplyTool::output_schema()),
    )
}

/// 组装生产工具面: 四拍受控写 + 五段流水线 (模型侧一个能力)。
pub fn worktree_apply_capability(
    root: impl Into<PathBuf>,
    gate: Arc<ObservedGate>,
    policy: Arc<Mutex<PermissionPolicy>>,
) -> Arc<dyn ToolCapability> {
    let root = root.into();
    let tool: Arc<dyn ToolCapability> =
        Arc::new(WorktreeApplyTool::new(root.clone(), Arc::clone(&gate)));
    Arc::new(crate::exec_pipeline::PipelinedCapability::new(
        tool,
        worktree_apply_pipeline(root, gate, policy),
    ))
}
