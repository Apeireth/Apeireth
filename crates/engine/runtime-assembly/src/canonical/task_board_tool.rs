//! 任务簿工具面 (`tool.task_board`, V6-a) + 治理档钩子。
//!
//! # 工具面 (闭合动作词表)
//!
//! | 动作 | 语义 | 写面归类 |
//! |---|---|---|
//! | `task_add` | 入队 (可带幂等键 / 初始执行者 / 父任务) | 任务簿写面 = 写文件 (write_intent 同族) |
//! | `task_list` | 列任务 (只读) | 只读 |
//! | `task_reassign` | 指派/改派 (assign 即唤醒) | 任务簿写面 = 写文件 (write_intent 同族) |
//! | `task_complete` | 自报完成 (必须是执行者本人) | 任务簿写面 = 写文件 (write_intent 同族) |
//!
//! **没有 `failed` / `retry` 动作** —— 失败只能人判 (人判面在队列的
//! `human_judge_*`, 不给 agent 工具面), 重试是显式人判动作, 防雪崩。
//!
//! # 治理档 (任务工具治理档)
//!
//! 写面 (增/改派/完成) 走 write_intent 同族 (任务簿写面 = 写文件): 每次调用
//! 都停人工审批 ([`TaskBoardApprovalHook`] 唯一承载, 无自动放行档); `task_list`
//! 是自家任务态的只读面 (与 `tool.self_status` 同类, 无外部内容注入面) 零审批;
//! 未知动作 fail-closed 停审批 (不静默放行)。
//!
//! # 认领授权 (自报必须是本人)
//!
//! `task_complete` 是自报动作: 调用方 = 派发本调用的会话, 队列层要求
//! `caller == assignee` (委托者不能伪造工人状态) —— 拒绝即帧, 不静默改态。
//! 状态机、幂等唤醒、崩溃恢复、结算接账全在 [`crate::task_queue`], 本模块
//! 只做工具面投影。

#![forbid(unsafe_code)]

use std::sync::Arc;

use apeireth_core::kernel::{CapabilityId, SessionId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use apeireth_orchestration::task_queue::{
    EnqueueOutcome, TaskCaller, TaskQueue, TaskQueueError, TaskRecord, TaskSpec,
};
use apeireth_plugin::{FrozenInvocation, ToolCapability};
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolParameters, ToolResult};
use async_trait::async_trait;
use serde_json::{json, Value};

/// 稳定能力标识 (注册表 / 治理授权 / 审批档共用)。
pub const TASK_BOARD_CAPABILITY_ID: &str = "tool.task_board";
/// 模型侧声明名。
pub const TASK_BOARD_TOOL_NAME: &str = "task_board";

/// 动作: 入队。
pub const ACTION_TASK_ADD: &str = "task_add";
/// 动作: 列任务 (纯读)。
pub const ACTION_TASK_LIST: &str = "task_list";
/// 动作: 指派/改派 (assign 即唤醒)。
pub const ACTION_TASK_REASSIGN: &str = "task_reassign";
/// 动作: 自报完成 (必须是执行者本人)。
pub const ACTION_TASK_COMPLETE: &str = "task_complete";

/// 闭合动作词表 (增/列/改派/完成; **无 failed/retry 面**)。
pub const TASK_BOARD_ACTIONS: &[&str] = &[
    ACTION_TASK_ADD,
    ACTION_TASK_LIST,
    ACTION_TASK_REASSIGN,
    ACTION_TASK_COMPLETE,
];

/// 动作的写面归类 (write_intent 同族口径; 未知动作无写面)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskBoardWriteFace {
    /// 任务簿写面 = 写文件 (write_intent 同族)。
    TaskBookWrite,
    /// 只读列任务。
    ReadOnly,
}

impl TaskBoardWriteFace {
    /// 稳定标签 (输出帧 `write_face` 列)。
    pub const fn label(self) -> &'static str {
        match self {
            Self::TaskBookWrite => "task_book_write (write_intent 同族)",
            Self::ReadOnly => "read_only",
        }
    }
}

/// 动作的写面归类 (未知动作无写面)。
pub fn task_board_write_face_for_action(action: &str) -> Option<TaskBoardWriteFace> {
    match action {
        ACTION_TASK_ADD | ACTION_TASK_REASSIGN | ACTION_TASK_COMPLETE => {
            Some(TaskBoardWriteFace::TaskBookWrite)
        }
        ACTION_TASK_LIST => Some(TaskBoardWriteFace::ReadOnly),
        _ => None,
    }
}

/// 审批档位理由 (审批卡载荷同文)。按动作如实标注写面: 改任务簿 = 写文件
/// (write_intent 同族) / 只读列自家任务态。
pub fn task_board_approval_reason(action: Option<&str>) -> String {
    let face = match action.map(str::as_ref) {
        Some(ACTION_TASK_ADD) | Some(ACTION_TASK_REASSIGN) | Some(ACTION_TASK_COMPLETE) => {
            "task_add/task_reassign/task_complete = 改任务簿 = 写文件 (write_intent 同族)"
        }
        Some(ACTION_TASK_LIST) => "task_list = 只读列自家任务态 (无外部内容注入面), 零审批",
        _ => "动作未识别, fail-closed 停人工审批",
    };
    format!(
        "task_board ({face}) —— 写面 RequireApproval 级: 每次写面都停人工审批, 无自动放行档; \
         failed 只能人判 (本工具无 failed/retry 面), 状态推进永远显式"
    )
}

/// 治理档位钩子: `tool.task_board` 的**写面**每次调用停
/// [`Decision::RequireApproval`] (无自动放行档, 任何预设/设置都不得越人);
/// `task_list` 只读零审批; 未知动作 fail-closed 停审批。
///
/// 与内容流/技能沉淀/关怀规则同一承载方式: 档位在治理层的钩子上, 与本地审批
/// 面板 / IM 审批卡同一条审批链。其它能力一律放行 (本钩子只管自己的面)。
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskBoardApprovalHook;

impl TaskBoardApprovalHook {
    /// 空构造 (无状态钩子)。
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GovernanceHook for TaskBoardApprovalHook {
    fn name(&self) -> &str {
        "task_board_approval_tier"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return Decision::Allow;
        };
        if capability.as_str() != TASK_BOARD_CAPABILITY_ID {
            return Decision::Allow;
        }
        let action = arguments.get("action").and_then(Value::as_str);
        match action {
            // 只读列自家任务态: 与 `tool.self_status` 同类只读面, 零审批。
            Some(ACTION_TASK_LIST) => Decision::Allow,
            // 写面 = write_intent 同族 (改任务簿 = 写文件), 每次人批;
            // 动作缺失/未知 fail-closed 同样停审批 (不静默放行)。
            _ => Decision::require_approval(task_board_approval_reason(action)),
        }
    }
}

/// `tool.task_board`: 任务簿 (增/列/改派/完成) 的工具面投影。
pub struct TaskBoardTool {
    id: CapabilityId,
    queue: Arc<TaskQueue>,
}

impl TaskBoardTool {
    /// 绑定持久任务队列 (存储在装配层: 与会话账本同库的任务表)。
    pub fn new(queue: Arc<TaskQueue>) -> Self {
        Self {
            id: CapabilityId::new(TASK_BOARD_CAPABILITY_ID).unwrap(),
            queue,
        }
    }

    fn parse_params(&self, call: &ToolCall) -> Result<TaskBoardParams, ToolResult> {
        let arguments = match &call.arguments {
            Value::Null => json!({}),
            other => other.clone(),
        };
        if !arguments.is_object() {
            return Err(permanent(
                call,
                "task_board arguments must be a JSON object",
            ));
        }
        serde_json::from_value::<TaskBoardParams>(arguments)
            .map_err(|error| permanent(call, format!("invalid task_board parameters: {error}")))
    }

    /// 解析并闭合校验一次调用 (非法参数拒绝即帧, 不进审批)。
    fn prepare(&self, call: &ToolCall) -> Result<PreparedTaskBoardCall, ToolResult> {
        let params = self.parse_params(call)?;
        if !TASK_BOARD_ACTIONS.contains(&params.action.as_str()) {
            return Err(permanent(
                call,
                format!(
                    "unknown task_board action {:?}; the closed vocabulary is task_add / \
                     task_list / task_reassign / task_complete (failed/retry are human-judged \
                     only and have no tool face)",
                    params.action
                ),
            ));
        }
        match params.action.as_str() {
            ACTION_TASK_ADD => {
                let title = params
                    .title
                    .filter(|title| !title.trim().is_empty())
                    .ok_or_else(|| permanent(call, "task_add requires a non-empty \"title\""))?;
                let mut spec = TaskSpec::new(title, params.payload.unwrap_or(Value::Null));
                if let Some(key) = params.idempotency_key {
                    spec = spec.with_idempotency_key(key);
                }
                if let Some(assignee) = params.assignee {
                    spec = spec.with_assignee(assignee);
                }
                if let Some(parent) = params.parent_task_id {
                    spec = spec.with_parent(parent);
                }
                Ok(PreparedTaskBoardCall::Add { spec })
            }
            ACTION_TASK_LIST => Ok(PreparedTaskBoardCall::List),
            ACTION_TASK_REASSIGN => {
                let task_id = params
                    .task_id
                    .ok_or_else(|| permanent(call, "task_reassign requires \"task_id\""))?;
                let assignee = params
                    .assignee
                    .filter(|assignee| !assignee.trim().is_empty())
                    .ok_or_else(|| {
                        permanent(call, "task_reassign requires a non-empty \"assignee\"")
                    })?;
                Ok(PreparedTaskBoardCall::Reassign { task_id, assignee })
            }
            _ => {
                let task_id = params
                    .task_id
                    .ok_or_else(|| permanent(call, "task_complete requires \"task_id\""))?;
                Ok(PreparedTaskBoardCall::Complete { task_id })
            }
        }
    }

    fn invoke_scoped(&self, call: &ToolCall, caller: Option<SessionId>) -> ToolResult {
        let prepared = match self.prepare(call) {
            Ok(prepared) => prepared,
            Err(result) => return result,
        };
        // 自报者 = 派发本调用的会话 (认领授权: 自报必须是本人)。
        let caller_id = caller
            .map(|session| session.to_string())
            .unwrap_or_default();
        match prepared {
            PreparedTaskBoardCall::Add { spec } => match self.queue.enqueue(spec) {
                Ok(EnqueueOutcome::Enqueued { task, wake }) => ToolResult::ok(
                    &call.id,
                    frame(
                        ACTION_TASK_ADD,
                        true,
                        TaskBoardWriteFace::TaskBookWrite,
                        false,
                        vec![task_json(&task)],
                        match wake {
                            Some(wake) => format!("assign 即唤醒: {}", wake.assignee),
                            None => String::new(),
                        },
                    ),
                )
                .with_name(TASK_BOARD_TOOL_NAME),
                Ok(EnqueueOutcome::Deduplicated {
                    task_id,
                    redelivered,
                }) => match self.queue.get(&task_id) {
                    Ok(task) => ToolResult::ok(
                        &call.id,
                        frame(
                            ACTION_TASK_ADD,
                            true,
                            TaskBoardWriteFace::TaskBookWrite,
                            true,
                            vec![task_json(&task)],
                            format!(
                                "同键重复投递被幂等去重 (redelivered={redelivered}), 不重复执行"
                            ),
                        ),
                    )
                    .with_name(TASK_BOARD_TOOL_NAME),
                    Err(error) => queue_error(call, error),
                },
                Err(error) => queue_error(call, error),
            },
            PreparedTaskBoardCall::List => match self.queue.list() {
                Ok(tasks) => {
                    let rows = tasks.iter().map(task_json).collect::<Vec<_>>();
                    ToolResult::ok(
                        &call.id,
                        frame(
                            ACTION_TASK_LIST,
                            true,
                            TaskBoardWriteFace::ReadOnly,
                            false,
                            rows,
                            String::new(),
                        ),
                    )
                    .with_name(TASK_BOARD_TOOL_NAME)
                }
                Err(error) => queue_error(call, error),
            },
            PreparedTaskBoardCall::Reassign { task_id, assignee } => {
                match self
                    .queue
                    .assign(&task_id, &assignee, &TaskCaller::agent(caller_id))
                {
                    Ok(task) => ToolResult::ok(
                        &call.id,
                        frame(
                            ACTION_TASK_REASSIGN,
                            true,
                            TaskBoardWriteFace::TaskBookWrite,
                            false,
                            vec![task_json(&task)],
                            format!("assign 即唤醒: {assignee}"),
                        ),
                    )
                    .with_name(TASK_BOARD_TOOL_NAME),
                    Err(error) => queue_error(call, error),
                }
            }
            PreparedTaskBoardCall::Complete { task_id } => {
                match self.queue.complete(&task_id, &TaskCaller::agent(caller_id)) {
                    Ok(task) => ToolResult::ok(
                        &call.id,
                        frame(
                            ACTION_TASK_COMPLETE,
                            true,
                            TaskBoardWriteFace::TaskBookWrite,
                            false,
                            vec![task_json(&task)],
                            String::new(),
                        ),
                    )
                    .with_name(TASK_BOARD_TOOL_NAME),
                    Err(error) => queue_error(call, error),
                }
            }
        }
    }
}

fn permanent(call: &ToolCall, message: impl Into<String>) -> ToolResult {
    ToolResult::permanent_error(&call.id, message).with_name(TASK_BOARD_TOOL_NAME)
}

fn queue_error(call: &ToolCall, error: TaskQueueError) -> ToolResult {
    permanent(call, format!("task queue refused: {error}"))
}

/// 输出帧 (字段合同: 六字段恒在场)。
fn frame(
    action: &str,
    ok: bool,
    write_face: TaskBoardWriteFace,
    deduplicated: bool,
    tasks: Vec<Value>,
    reason: String,
) -> Value {
    json!({
        "action": action,
        "ok": ok,
        "write_face": write_face.label(),
        "deduplicated": deduplicated,
        "tasks": tasks,
        "reason": reason,
    })
}

/// 任务行投影 (真值; 结算只回轮次引用 —— 金额沿三维账本既有口径, 不在此重复)。
fn task_json(record: &TaskRecord) -> Value {
    json!({
        "id": record.id,
        "title": record.title,
        "state": record.state.to_string(),
        "assignee": record.assignee,
        "idempotency_key": record.idempotency_key,
        "parent_task_id": record.parent_task_id,
        "chain_depth": record.chain_depth,
        "attempts": record.attempts,
        "redelivered": record.redelivered,
        "created_at_ms": record.created_at_ms,
        "updated_at_ms": record.updated_at_ms,
        "settled_turns": record.settlements.iter().map(|settled| settled.turn).collect::<Vec<_>>(),
    })
}

enum PreparedTaskBoardCall {
    Add { spec: TaskSpec },
    List,
    Reassign { task_id: String, assignee: String },
    Complete { task_id: String },
}

/// 工具参数面 (动作 + 可选 title/payload/idempotency_key/assignee/parent_task_id/task_id)。
#[derive(Debug, serde::Deserialize)]
struct TaskBoardParams {
    action: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    payload: Option<Value>,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    parent_task_id: Option<String>,
    #[serde(default)]
    task_id: Option<String>,
}

#[async_trait]
impl ToolCapability for TaskBoardTool {
    fn id(&self) -> &CapabilityId {
        &self.id
    }

    fn declaration(&self) -> NormalizedTool {
        let mut params = ToolParameters::new();
        params.extend(
            json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": TASK_BOARD_ACTIONS,
                        "description": "task_add / task_list / task_reassign / task_complete (closed vocabulary; failed/retry are human-judged only)."
                    },
                    "title": {
                        "type": "string",
                        "description": "Task title for task_add (required there)."
                    },
                    "payload": {
                        "type": "object",
                        "description": "Opaque task payload for task_add (the queue does not interpret it)."
                    },
                    "idempotency_key": {
                        "type": "string",
                        "description": "Idempotency key for task_add: redelivering the same key never creates a second task and never runs twice."
                    },
                    "assignee": {
                        "type": "string",
                        "description": "Assignee (worker session id) for task_add / task_reassign. Assigning wakes the assignee; self-reports (task_complete) must come from this assignee."
                    },
                    "parent_task_id": {
                        "type": "string",
                        "description": "Parent task id for task_add (task chain); chains deeper than the configured fuse are refused with an audit event."
                    },
                    "task_id": {
                        "type": "string",
                        "description": "Task id for task_reassign / task_complete."
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
        NormalizedTool::new(TASK_BOARD_TOOL_NAME)
            .with_description(
                "Durable task board (V6-a persistent task queue): task_add / task_list / \
                 task_reassign / task_complete over a crash-safe queue persisted beside the \
                 session ledger. Adding a task with an idempotency key is duplicate-safe \
                 (redelivering the same key never runs the task twice); assigning emits a wake \
                 signal to the assignee; task_complete is a self-report and only the assignee \
                 may file it. Failed tasks are human-judged only — this tool deliberately has \
                 no failed/retry face, so failures never retry automatically. Governance: task \
                 book writes (add / reassign / complete) are file-write-family writes and every \
                 write call stops at human approval (no auto-pass tier); task_list reads own \
                 task state only and needs no approval.",
            )
            .with_parameters(params)
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        self.invoke_scoped(call, None)
    }

    /// 会话作用域入口: 自报面 (task_complete) 的 caller = 派发本调用的会话。
    async fn invoke_in_session(
        &self,
        call: &ToolCall,
        _frozen: Option<&FrozenInvocation>,
        session: Option<SessionId>,
    ) -> ToolResult {
        self.invoke_scoped(call, session)
    }
}
