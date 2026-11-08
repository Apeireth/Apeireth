//! `tool.process_tree` —— **进程树快照面**（模型可调用工具面 · 能力创造线 W1 C-B1）。
//!
//! # 定位
//!
//! 把库级内核 [`crate::process`] 的进程树能力暴露成**模型可调用的一条工具面**：
//! 动作词表**闭合**（`snapshot` / `cancel`），**表外动作拒绝即帧**（不猜、不静默放行）。
//!
//! | 动作 | 语义 | 治理档 |
//! |---|---|---|
//! | `snapshot` | **自根树**快照（根 = **本进程自己的 pid** ⇒ 覆盖我们 spawn 出来的全部子进程，含 shell 的子进程） | **R0 零审批**（纯只读） |
//! | `cancel` | 按 pid **取消**一棵**已证明属于本进程**的进程树 | **R3 逐次人批**（每次调用都停人工审批） |
//!
//! 本面**只复用** [`crate::process`] 的公共 API（[`process_tree_snapshot`] /
//! [`cancel_process_tree`] / [`process_tree_support`]），**不重写**进程枚举与终止逻辑；
//! 平台机制名只落在平台后端模块 ⇒ **公共契约面零平台机制名**。
//!
//! # 越权防线（fail-closed · 宁漏勿越权）
//!
//! `cancel` 必须**证明**目标 pid 是**本进程的后代**：
//!
//! * **证据 = 父进程链上溯**：从目标 pid 沿父链上溯，链上出现**本进程 pid** 即命中；
//!   一路上溯到会话初始进程仍未命中 ⇒ **证不出来**。
//! * **证不出来 ⇒ 拒绝即帧**（永久错误帧，不发任何终止动作）。
//! * **本进程自身**与 **pid 0** 一律拒绝（不许自我了断、不许把 0 当根）。
//! * 归属证明在**冻结载荷生成时**（审批前）与**执行时**（审批后）各做一次 ⇒
//!   审批绑定的是**确切目标**，执行时再复验一次（防 pid 复用）。
//!
//! # 平台口径（0 装诚实）
//!
//! * 平台**未实现** ⇒ **显式 `Unsupported` 错误帧**（真因如实），
//!   **绝不返回空快照冒充成功**。
//! * 平台**可枚举但不可判定收容组归属** ⇒ 归属记 `undetermined`（**不谎报**未逃逸）。
//!
//! # 结果面口径
//!
//! * 快照帧：`action` / `status` / `root_pid` / `count` / `escaped_count` / `entries` /
//!   `platform_support`；每条成员带 `pid` / `parent_pid` / `command`（有界）/ `status` /
//!   `group_containment`。
//! * 取消帧：`action` / `status` / `root_pid` / `terminated` / `already_exited` / `failed` /
//!   `caused_termination`。
//! * **超长命令行截断并显式标记**（`command_truncated`）—— 输出侧可截，**输入侧不截**
//!   （词表外动作、非法 pid 一律拒绝即帧）。
//!
//! # 接线口径（本线只实现，不越界接线）
//!
//! 本文件 = **实现态（IMPLEMENTED）**；**未接线**：生产装配注册、治理链挂
//! [`ProcessTreeApprovalHook`]、模型面工具登记 ⇒ 接线归生产装配线（不在本线写域）。

use std::time::Duration;

use apeireth_core::kernel::CapabilityId;
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::exec_pipeline::{OutputSchema, SchemaKind};
use crate::process::{
    cancel_process_tree, process_tree_snapshot, process_tree_support, run_native, GroupContainment,
    PlatformKind, ProcessTreeCancel, ProcessTreeError, ProcessTreeSnapshot, ProcessTreeStatus,
    ProcessTreeSupport,
};

// ---------------------------------------------------------------------------
// 常量（身份 / 词表 / 上限）
// ---------------------------------------------------------------------------

/// 稳定能力标识（注册表 / 治理授权 / 审批档共用）。
pub const PROCESS_TREE_CAPABILITY_ID: &str = "tool.process_tree";
/// 模型面工具名（模型在 `ToolCall.name` 里用的名字）。
pub const PROCESS_TREE_TOOL_NAME: &str = "process_tree";
/// 冻结调用载荷版本。
pub const PROCESS_TREE_FROZEN_VERSION: u32 = 1;
/// 生产执行时限（毫秒）：进程表枚举 + 逐 pid 终止，到期即归 `timeout.*`。
pub const PROCESS_TREE_TIMEOUT_MS: u64 = 5_000;

/// 动作：只读快照（**零审批**）。
pub const ACTION_SNAPSHOT: &str = "snapshot";
/// 动作：取消一棵受控进程树（**逐次人批**）。
pub const ACTION_CANCEL: &str = "cancel";
/// 闭合动作词表（`snapshot` / `cancel`）。
pub const PROCESS_TREE_ACTIONS: &[&str] = &[ACTION_SNAPSHOT, ACTION_CANCEL];

/// 单条命令行在结果面里的字符上限（超长 ⇒ 截断 + 显式标记 `command_truncated`）。
pub const PROCESS_TREE_COMMAND_MAX_CHARS: usize = 512;

/// 主体边界声明（工具描述与结果帧同文）。
pub const PROCESS_TREE_SCOPE_NOTE: &str =
    "本面只看得见**本进程自己的后代进程树**: 快照根默认是本进程自身, 取消只作用于本进程的后代; \
     证不出后代关系的 pid 一律拒绝 (不越权动别人的进程)。";

// ---------------------------------------------------------------------------
// 写面归类 + 审批理由
// ---------------------------------------------------------------------------

/// 动作的写面归类（未知动作无写面 ⇒ 不猜）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessTreeWriteFace {
    /// 纯读（快照）：零审批、零写。
    ReadOnly,
    /// 终止进程树（写面）：逐次人批。
    ProcessTermination,
}

impl ProcessTreeWriteFace {
    /// 稳定标签（审计 / 测试用）。
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::ProcessTermination => "process_termination",
        }
    }
}

/// 动作的写面归类（未知动作无写面 ⇒ 不猜）。
pub fn process_tree_write_face_for_action(action: &str) -> Option<ProcessTreeWriteFace> {
    match action {
        ACTION_SNAPSHOT => Some(ProcessTreeWriteFace::ReadOnly),
        ACTION_CANCEL => Some(ProcessTreeWriteFace::ProcessTermination),
        _ => None,
    }
}

/// 审批档位理由（审批卡载荷同文）。按动作如实标注写面。
pub fn process_tree_approval_reason(action: Option<&str>) -> String {
    let face = match action {
        Some(ACTION_SNAPSHOT) => "snapshot = 只读进程树快照, 零审批",
        Some(ACTION_CANCEL) => "cancel = 终止本进程后代的一棵进程树",
        _ => "动作未识别, fail-closed 拒绝即帧",
    };
    format!(
        "process_tree ({face}) —— 写面逐次人批: 每次调用都停人工审批, 无自动放行档; \
         取消只作用于本进程的后代, 证不出后代关系即拒绝 (宁漏勿越权)"
    )
}

/// 平台种类的稳定标签（**平台无关**的公开措辞，不带任何平台机制名）。
pub fn process_tree_platform_label(platform: PlatformKind) -> &'static str {
    match platform {
        PlatformKind::Windows => "windows",
        PlatformKind::Linux => "linux",
        PlatformKind::MacOs => "macos",
        PlatformKind::Other => "other",
    }
}

/// 平台支持档的稳定标签。
pub fn process_tree_support_label(support: ProcessTreeSupport) -> &'static str {
    match support {
        ProcessTreeSupport::Enumerable => "enumerable",
        ProcessTreeSupport::Unsupported => "unsupported",
    }
}

/// 库级错误 → **模型面**错误文案（真因如实；平台未实现显式点出，绝不装作成功）。
pub fn process_tree_error_message(error: &ProcessTreeError, action: &str) -> String {
    match error {
        ProcessTreeError::Unsupported { platform } => format!(
            "拒绝{action}: 该平台未实现进程树面 (platform={}); 显式未实现, \
             不返回空快照冒充成功",
            process_tree_platform_label(*platform)
        ),
        ProcessTreeError::NotControlled { pid } => format!(
            "拒绝{action}: pid {pid} 证不出是本进程的后代 (归属证明失败 ⇒ \
             fail-closed 拒绝即帧, 未对任何进程发动作)"
        ),
        ProcessTreeError::NotFound { pid } => format!(
            "拒绝{action}: pid {pid} 不存在 (可能已退出或已被取消); \
             拒绝即帧, 未对任何进程发动作"
        ),
        ProcessTreeError::Refused { pid, reason } => {
            format!("拒绝{action}: pid {pid} 被拒: {reason}")
        }
        ProcessTreeError::Io { message } => {
            format!("{action} 失败: 进程表查询或终止未成功: {message}")
        }
    }
}

// ---------------------------------------------------------------------------
// 治理（写面逐次人批 · 读面零审批）
// ---------------------------------------------------------------------------

/// 治理档位钩子：`tool.process_tree` 的**取消动作每次**停
/// [`Decision::RequireApproval`]（无自动放行档），**快照动作零审批**。
///
/// 与本地审批面板 / IM 审批卡同一条审批链（照既有范式）：档位在治理层的钩子上，
/// 其它能力一律放行（本钩子只管自己的面）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessTreeApprovalHook;

impl ProcessTreeApprovalHook {
    /// 空构造（无状态钩子）。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl GovernanceHook for ProcessTreeApprovalHook {
    fn name(&self) -> &str {
        "process_tree_approval_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != PROCESS_TREE_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments.get("action").and_then(Value::as_str);
        match action {
            // 纯读面：零审批。
            Some(ACTION_SNAPSHOT) => Decision::Allow,
            // 写面：逐次人批（无自动放行档）。
            Some(ACTION_CANCEL) => Decision::require_approval(process_tree_approval_reason(action)),
            // 词表外 / 动作缺失：fail-closed 拒绝即帧（不静默放行，不猜动作）。
            other => Decision::deny(format!(
                "进程树动作 {other:?} 不在闭合词表 (snapshot / cancel): fail-closed 拒绝即帧"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// 工具面
// ---------------------------------------------------------------------------

/// 工具调用参数（未知字段拒绝）。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcessTreeParams {
    action: Option<String>,
    pid: Option<u32>,
}

/// 冻结的调用载荷：**版本化的确切执行形状**。
///
/// ⚠️ 取消面的载荷里装的是**确切的目标 pid** ⇒ 人工批准的那一次操作与实际执行的
/// 那一次操作**逐字节同形**（模型事后改参数也改不动已批准的载荷）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProcessTreeFrozenInvocation {
    version: u32,
    /// 动作（`snapshot` / `cancel`）。
    action: String,
    /// 目标 pid：`snapshot` = 快照根；`cancel` = 待取消的树根。
    pid: u32,
}

/// `tool.process_tree` —— 进程树快照面。
pub struct ProcessTreeTool {
    id: CapabilityId,
}

impl std::fmt::Debug for ProcessTreeTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProcessTreeTool")
            .field("id", &self.id)
            .finish()
    }
}

impl Default for ProcessTreeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessTreeTool {
    /// 构造进程树工具面（**零配置**：本面不需要任何外部端点或凭据）。
    pub fn new() -> Self {
        Self {
            id: CapabilityId::new(PROCESS_TREE_CAPABILITY_ID).expect("capability id is valid"),
        }
    }

    /// 输出归一合同：结构化输出必须是对象且 `action` / `status` 恒在场。
    pub fn output_schema() -> OutputSchema {
        OutputSchema::new()
            .require("action", SchemaKind::String)
            .require("status", SchemaKind::String)
    }

    fn declaration_parameters() -> ToolParameters {
        let parameters = json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [ACTION_SNAPSHOT, ACTION_CANCEL],
                    "description": "snapshot / cancel (closed vocabulary)."
                },
                "pid": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Target process id. cancel requires it; snapshot defaults to the current process and may narrow to one of its descendants."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        });
        let mut params = ToolParameters::new();
        params.extend(parameters.as_object().cloned().unwrap_or_default());
        params
    }

    fn parse_params(&self, call: &ToolCall) -> Result<ProcessTreeParams, ToolResult> {
        let object = call.arguments.as_object().ok_or_else(|| {
            ToolResult::permanent_error(&call.id, "进程树参数必须是 JSON 对象")
                .with_name(PROCESS_TREE_TOOL_NAME)
        })?;

        for key in object.keys() {
            if key != "action" && key != "pid" {
                return Err(ToolResult::permanent_error(
                    &call.id,
                    format!("未知进程树参数 {key:?}; 只支持 \"action\" 与 \"pid\""),
                )
                .with_name(PROCESS_TREE_TOOL_NAME));
            }
        }

        serde_json::from_value(call.arguments.clone()).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("进程树参数非法: {e}"))
                .with_name(PROCESS_TREE_TOOL_NAME)
        })
    }

    /// 拒绝即帧（真因如实，不静默降级）。
    fn refuse(&self, call: &ToolCall, reason: impl Into<String>) -> ToolResult {
        ToolResult::permanent_error(&call.id, reason.into()).with_name(PROCESS_TREE_TOOL_NAME)
    }

    /// 库级错误 → 模型面错误帧。
    ///
    /// **只有「瞬时 IO 失败」且「本次调用没有副作用」才可重试**；其余（平台未实现 /
    /// 证不出后代 / 目标不存在 / 被拒）一律**永久错误帧** ⇒ 拒绝即帧，且不制造重试风暴。
    ///
    /// `side_effect` = 本次调用可能已经产生终止副作用 ⇒ 失败**不可重试**
    /// （重复调用可能重复动作）。
    fn map_error(
        &self,
        call: &ToolCall,
        error: ProcessTreeError,
        action: &str,
        side_effect: bool,
    ) -> ToolResult {
        let message = process_tree_error_message(&error, action);
        let retryable = matches!(&error, ProcessTreeError::Io { .. }) && !side_effect;
        if retryable {
            ToolResult::retryable_error(&call.id, message).with_name(PROCESS_TREE_TOOL_NAME)
        } else {
            self.refuse(call, message)
        }
    }

    /// 归属证明（**fail-closed**）：证明目标 pid 属于**本进程**（自身或其后代）。
    ///
    /// 证据 = **父进程链上溯**（链上出现本进程 pid 即命中）。证不出 ⇒ 拒绝即帧。
    /// 本函数只调库级公共 API，**不重写**归属判定逻辑。
    fn prove_ownership(&self, call: &ToolCall, pid: u32, action: &str) -> Result<(), ToolResult> {
        match process_tree_snapshot(pid) {
            Ok(_) => Ok(()),
            Err(error) => Err(self.map_error(call, error, action, false)),
        }
    }

    /// 校验 + 冻结（**审批前**就完成归属证明 ⇒ 非法操作不会生成待审批项）。
    fn build_frozen(&self, call: &ToolCall) -> Result<ProcessTreeFrozenInvocation, ToolResult> {
        let params = self.parse_params(call)?;
        let Some(action) = params.action.as_deref() else {
            return Err(self.refuse(
                call,
                format!("拒绝进程树调用: 必须显式给 action ({ACTION_SNAPSHOT} / {ACTION_CANCEL})"),
            ));
        };

        match action {
            ACTION_SNAPSHOT => {
                // 根默认 = 本进程自身 ⇒ 覆盖我们 spawn 出来的全部子进程（含 shell 的子进程）。
                let root_pid = params.pid.unwrap_or_else(std::process::id);
                if root_pid == 0 {
                    return Err(self.refuse(call, "拒绝快照: pid 0 不是进程根"));
                }
                // 归属证明：根必须是本进程自身或其后代 ⇒ 证不出即拒。
                self.prove_ownership(call, root_pid, ACTION_SNAPSHOT)?;
                Ok(ProcessTreeFrozenInvocation {
                    version: PROCESS_TREE_FROZEN_VERSION,
                    action: ACTION_SNAPSHOT.to_string(),
                    pid: root_pid,
                })
            }
            ACTION_CANCEL => {
                let Some(pid) = params.pid else {
                    return Err(self.refuse(call, "拒绝取消: cancel 必须给 pid"));
                };
                if pid == 0 {
                    return Err(self.refuse(call, "拒绝取消: pid 0 不是进程根"));
                }
                if pid == std::process::id() {
                    return Err(self.refuse(call, "拒绝取消: 目标是本进程自身 (不许自我了断)"));
                }
                // 归属证明：**证不出后代关系 ⇒ 拒绝即帧**（宁漏勿越权）。
                self.prove_ownership(call, pid, ACTION_CANCEL)?;
                Ok(ProcessTreeFrozenInvocation {
                    version: PROCESS_TREE_FROZEN_VERSION,
                    action: ACTION_CANCEL.to_string(),
                    pid,
                })
            }
            other => Err(self.refuse(
                call,
                format!("未知进程树动作 {other:?}: 词表闭合 ({ACTION_SNAPSHOT} / {ACTION_CANCEL})"),
            )),
        }
    }

    /// 审批视图（**给人看的**）：只有动作与**确切目标 pid**。
    fn display_invocation(frozen: &ProcessTreeFrozenInvocation) -> Value {
        json!({
            "version": frozen.version,
            "action": frozen.action,
            "target_pid": frozen.pid,
            "scope": PROCESS_TREE_SCOPE_NOTE,
        })
    }

    async fn execute_frozen(
        &self,
        call: &ToolCall,
        frozen: &ProcessTreeFrozenInvocation,
    ) -> ToolResult {
        if frozen.version != PROCESS_TREE_FROZEN_VERSION {
            return self.refuse(
                call,
                format!(
                    "冻结载荷版本 {} 不受支持 (期望 {})",
                    frozen.version, PROCESS_TREE_FROZEN_VERSION
                ),
            );
        }
        match frozen.action.as_str() {
            ACTION_SNAPSHOT => self.execute_snapshot(call, frozen.pid).await,
            ACTION_CANCEL => self.execute_cancel(call, frozen.pid).await,
            other => self.refuse(
                call,
                format!(
                    "冻结载荷的动作 {other:?} 不在闭合词表 ({ACTION_SNAPSHOT} / {ACTION_CANCEL})"
                ),
            ),
        }
    }

    /// 快照执行（只读面；进程表枚举是阻塞 IO ⇒ 走原生任务线程，不占异步工作线程）。
    async fn execute_snapshot(&self, call: &ToolCall, root_pid: u32) -> ToolResult {
        let outcome = run_native(move || process_tree_snapshot(root_pid)).await;
        let snapshot = match outcome {
            Ok(Ok(snapshot)) => snapshot,
            Ok(Err(error)) => {
                return self.map_error(call, error, ACTION_SNAPSHOT, false);
            }
            Err(join_error) => {
                return ToolResult::retryable_error(
                    &call.id,
                    format!("进程树快照任务失败: {join_error}"),
                )
                .with_name(PROCESS_TREE_TOOL_NAME);
            }
        };
        ToolResult::ok(&call.id, Self::snapshot_frame(&snapshot)).with_name(PROCESS_TREE_TOOL_NAME)
    }

    /// 取消执行（写面；逐 pid 终止是阻塞动作 ⇒ 走原生任务线程）。
    async fn execute_cancel(&self, call: &ToolCall, pid: u32) -> ToolResult {
        let outcome = run_native(move || cancel_process_tree(pid)).await;
        match outcome {
            Ok(Ok(result)) => ToolResult::ok(&call.id, Self::cancel_frame(&result))
                .with_name(PROCESS_TREE_TOOL_NAME),
            Ok(Err(error)) => self.map_error(call, error, ACTION_CANCEL, true),
            Err(join_error) => self.refuse(call, format!("进程树取消任务失败: {join_error}")),
        }
    }

    /// 快照结果帧（模型面措辞：状态与归属用小写稳定标签）。
    fn snapshot_frame(snapshot: &ProcessTreeSnapshot) -> Value {
        let entries: Vec<Value> = snapshot
            .entries
            .iter()
            .map(|entry| {
                let (command, truncated) = bound_command(&entry.command_line);
                json!({
                    "pid": entry.pid,
                    "parent_pid": entry.parent_pid,
                    "command": command,
                    "command_truncated": truncated,
                    "status": status_label(entry.status),
                    "group_containment": containment_label(entry.group_containment),
                })
            })
            .collect();

        json!({
            "action": ACTION_SNAPSHOT,
            "status": "ok",
            "root_pid": snapshot.root_pid,
            "count": snapshot.len(),
            "escaped_count": snapshot.escaped().count(),
            "entries": entries,
            "platform_support": process_tree_support_label(process_tree_support()),
            "scope": PROCESS_TREE_SCOPE_NOTE,
        })
    }

    /// 取消结果帧（逐 pid 如实回报：终止 / 已退出 / 失败）。
    fn cancel_frame(result: &ProcessTreeCancel) -> Value {
        match result {
            ProcessTreeCancel::Cancelled {
                root_pid,
                terminated,
                already_exited,
                failed,
            } => json!({
                "action": ACTION_CANCEL,
                "status": "cancelled",
                "root_pid": root_pid,
                "terminated": terminated,
                "already_exited": already_exited,
                "failed": failed,
                "caused_termination": true,
                "scope": PROCESS_TREE_SCOPE_NOTE,
            }),
            ProcessTreeCancel::AlreadyCancelled {
                root_pid,
                terminated,
            } => json!({
                "action": ACTION_CANCEL,
                "status": "already_cancelled",
                "root_pid": root_pid,
                "terminated": terminated,
                "already_exited": [],
                "failed": [],
                "caused_termination": false,
                "scope": PROCESS_TREE_SCOPE_NOTE,
            }),
            ProcessTreeCancel::NotRunning { root_pid } => json!({
                "action": ACTION_CANCEL,
                "status": "not_running",
                "root_pid": root_pid,
                "terminated": [],
                "already_exited": [],
                "failed": [],
                "caused_termination": false,
                "scope": PROCESS_TREE_SCOPE_NOTE,
            }),
        }
    }
}

#[async_trait]
impl ToolCapability for ProcessTreeTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(PROCESS_TREE_TOOL_NAME)
            .with_description(
                "Lists the process tree rooted at the current process (pid / parent pid / command / \
                 state / containment membership) and cancels one process tree by pid. The \
                 vocabulary is closed: snapshot, cancel. Governance: snapshot is a pure read with \
                 zero approval; cancel requires a human decision on each call and only ever \
                 touches a tree proven to belong to this process — a pid whose descent cannot be \
                 proven is refused outright, never guessed.",
            )
            .with_parameters(Self::declaration_parameters())
    }

    fn execution_policy(&self) -> apeireth_plugin::ToolExecutionPolicy {
        apeireth_plugin::ToolExecutionPolicy {
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string" },
                    "status": { "type": "string" }
                },
                "required": ["action", "status"]
            })),
            timeout: Some(Duration::from_millis(PROCESS_TREE_TIMEOUT_MS)),
            // 取消面会改进程状态 ⇒ 同一时刻只放行一次（串行化）。
            concurrency: apeireth_plugin::ToolConcurrency::Serial,
            // 本面只在桌面平台上有实现；其它平台显式未实现。
            platform: apeireth_plugin::ToolPlatform::Desktop,
            ..Default::default()
        }
    }

    fn freeze_invocation(&self, call: &ToolCall) -> Result<Option<FrozenInvocation>, ToolResult> {
        let frozen = self.build_frozen(call)?;
        let payload = serde_json::to_value(&frozen).map_err(|e| {
            ToolResult::permanent_error(&call.id, format!("冻结进程树调用载荷失败: {e}"))
                .with_name(PROCESS_TREE_TOOL_NAME)
        })?;
        let display = Self::display_invocation(&frozen);
        Ok(Some(FrozenInvocation::new(payload, display)))
    }

    async fn invoke_frozen(
        &self,
        call: &ToolCall,
        frozen: Option<&FrozenInvocation>,
    ) -> ToolResult {
        let Some(frozen) = frozen else {
            return self.invoke(call).await;
        };
        let parsed: ProcessTreeFrozenInvocation =
            match serde_json::from_value(frozen.payload.clone()) {
                Ok(parsed) => parsed,
                Err(e) => {
                    return self.refuse(call, format!("冻结的进程树调用载荷非法: {e}"));
                }
            };
        self.execute_frozen(call, &parsed).await
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        let frozen = match self.build_frozen(call) {
            Ok(frozen) => frozen,
            Err(result) => return result,
        };
        self.execute_frozen(call, &frozen).await
    }
}

// ---------------------------------------------------------------------------
// 小工具
// ---------------------------------------------------------------------------

/// 命令行有界化（超长 ⇒ 截断 + 显式标记）。
fn bound_command(raw: &str) -> (String, bool) {
    if raw.chars().count() <= PROCESS_TREE_COMMAND_MAX_CHARS {
        return (raw.to_string(), false);
    }
    let bounded: String = raw
        .chars()
        .take(PROCESS_TREE_COMMAND_MAX_CHARS)
        .collect::<String>();
    (bounded, true)
}

/// 进程状态的模型面标签。
fn status_label(status: ProcessTreeStatus) -> &'static str {
    match status {
        ProcessTreeStatus::Running => "running",
        ProcessTreeStatus::Sleeping => "sleeping",
        ProcessTreeStatus::Blocked => "blocked",
        ProcessTreeStatus::Stopped => "stopped",
        ProcessTreeStatus::Zombie => "zombie",
        ProcessTreeStatus::Exited => "exited",
        ProcessTreeStatus::Unknown => "unknown",
    }
}

/// 收容组归属的模型面标签（不可判定时如实记 `undetermined`，**不谎报**）。
fn containment_label(containment: GroupContainment) -> &'static str {
    match containment {
        GroupContainment::Contained => "contained",
        GroupContainment::Escaped => "escaped",
        GroupContainment::Undetermined => "undetermined",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_vocabulary_is_closed_and_write_faces_are_pinned() {
        assert_eq!(PROCESS_TREE_ACTIONS, [ACTION_SNAPSHOT, ACTION_CANCEL]);
        assert_eq!(
            process_tree_write_face_for_action(ACTION_SNAPSHOT),
            Some(ProcessTreeWriteFace::ReadOnly)
        );
        assert_eq!(
            process_tree_write_face_for_action(ACTION_CANCEL),
            Some(ProcessTreeWriteFace::ProcessTermination)
        );
        // 词表外动作没有写面（不猜）。
        assert_eq!(process_tree_write_face_for_action("kill"), None);
        assert_eq!(process_tree_write_face_for_action("list"), None);
    }

    #[test]
    fn unsupported_platform_is_reported_explicitly() {
        let message = process_tree_error_message(
            &ProcessTreeError::Unsupported {
                platform: PlatformKind::MacOs,
            },
            ACTION_SNAPSHOT,
        );
        assert!(message.contains("未实现"), "{message}");
        assert!(message.contains("macos"), "{message}");
        assert!(
            !message.contains("ok"),
            "未实现平台不得被写成成功: {message}"
        );
    }

    #[test]
    fn not_controlled_is_refused_with_the_reason() {
        let message = process_tree_error_message(
            &ProcessTreeError::NotControlled { pid: 4242 },
            ACTION_CANCEL,
        );
        assert!(message.contains("4242"), "{message}");
        assert!(message.contains("后代"), "{message}");
    }
}
