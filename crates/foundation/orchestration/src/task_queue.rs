//! 持久任务队列 (V6-a): 状态机 + 幂等唤醒 + 崩溃恢复 + 环路熔断 + 结算接账。
//!
//! # 为什么有这一页
//!
//! 事件溯源日志管「发生过什么」, 任务队列管「还有什么要干、归谁干」: 跨重启
//! 可审计的任务交接 (入队/指派/唤醒/重投递) 全部落在持久存储上, 状态推进永远
//! 是显式动作 —— 没有任何隐式自动重试。
//!
//! # 状态机 (六态, 全部转移显式)
//!
//! ```text
//!                    ┌─────────────── 人判 fail (human_judge_fail)  ←──────┐
//!                    ↓                                                    │
//!  enqueue ──→ Pending ──assign(唤醒)──→ Assigned ──begin──→ Running      │
//!    (可带 assignee=Assigned)   ↑           │                  │  │        │
//!                    │          │           │ block            │  │complete│
//!                    │          │           ↓                  │  ↓        │
//!                    │          └──assign── Blocked ←──block───┘  Done    │
//!                    │                          │                          │
//!                    │   崩溃恢复 (open):       │ 人判 retry               │
//!                    └── running/assigned ──────┤ (human_judge_retry)      │
//!                        超 max_attempts        ↓                          │
//!                        ──→ Blocked      Pending ─────────────────────────┘
//!                                                    Failed (只能人判)
//! ```
//!
//! - `Pending` — 入队待派; `Assigned` — 已指派并已唤醒; `Running` — 执行段在跑;
//!   `Done` — 完成 (终态); `Failed` — 失败 (**只能人判**, 见下); `Blocked` —
//!   外部等待/重投递熔断。
//! - 唯一的失败入口是 [`TaskQueue::human_judge_fail`] (人判, 拒 agent 调用);
//!   **失败绝不自动重试** —— 回 `Pending` 的唯一路径是显式人判
//!   [`TaskQueue::human_judge_retry`] (重试 = 显式动作, 防雪崩)。
//!
//! # 崩溃安全 (重启即恢复)
//!
//! [`TaskQueue::open`] 是恢复点: `Running`/`Assigned` 的任务回 `Pending` 并派一次
//! `attempt` (执行段计数); 超过 [`TaskQueueConfig::max_attempts`] 的不再回队,
//! 熔断为 `Blocked` (防崩溃死循环)。未投递的唤醒信号原样留在唤醒队列里等重投
//! (`redelivered` 计数 + 审计), 重启不丢任务、不丢唤醒。
//!
//! # 幂等唤醒 (assign 即唤醒)
//!
//! - **唤醒信号与状态事件同汇**: [`TaskEvent::WakeEmitted`] 与其余状态事件写进
//!   **同一本** 事件账 ([`TaskQueueState::journal`]), 唤醒体本身进持久唤醒队列
//!   ([`TaskQueueState::wakes`]), [`TaskQueue::drain_wakes`] 取走即标已投递。
//! - **幂等去重**: 携带同一 `idempotency_key` 的重复投递**不产生第二个任务**
//!   (任何状态下命中都算) —— 同 key 双投递只执行一次, 重复那次只把既有任务的
//!   `redelivered` 加一并留审计 ([`TaskEvent::DeliveryDeduplicated`])。
//!
//! # 结算接账 (只接账不重复记)
//!
//! 一个回合 = 一个任务的执行段: [`TaskQueue::link_settlement`] 把回合结算挂到
//! 任务账上 ([`TaskSettlement`]), **同一回合只挂一次** (重复挂账不重复记,
//! [`TaskEvent::SettlementDuplicateIgnored`])。任务账只存轮次与账本引用
//! (`ledger_ref`), **不存任何金额** —— 「算不算花过钱」沿三维账本既有口径
//! ([`crate::work_consumption`]: 事件日志单趟折叠, 取消/中断同答案), 本模块
//! 只接账、不重复记。
//!
//! # 环路熔断 (任务链深度上限)
//!
//! [`TaskSpec::parent_task_id`] 形成任务链 (spawn 链); 链深超过
//! [`TaskQueueConfig::max_chain_depth`] 的入队**拒绝即帧**并留审计
//! ([`TaskEvent::ChainRefused`]) —— 涌现级失控在结构上不可能, 不寄望个体自省。
//!
//! # 认领授权 (自报必须是本人)
//!
//! [`TaskCaller`] 区分 agent 与人: 自报 `begin`/`complete`/`block` 必须
//! `caller.id == assignee` (委托者不能伪造工人状态); `fail`/`retry` 是人判面
//! ([`TaskQueueError::HumanOnly`] 结构性拒绝 agent)。指派/改派面不在这里叠
//! 第二把锁 —— 它的把关在审批档 (任务簿写面 = 写文件同族, 每次人批), 队列层
//! 只留痕 ([`TaskEvent::Assigned`] 的 `by`)。
//!
//! # 存储
//!
//! 队列本身不落盘: 持久化收敛在 [`TaskStore`] 端口的**原子读改写**
//! ([`TaskStore::update`]), 实现负责「一次 update = 一次原子落盘」——
//! 并发入队因此原子 (同店多手互不覆盖)。本模块给内存镜像
//! ([`InMemoryTaskStore`]); 生产 sqlite 表与会话账本同库, 落在装配层
//! (与会话账本同一存储族)。事件账只增不删, 长期归档/轮转留后续阶段。

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

// ============================================================================
// 错误
// ============================================================================

/// 任务队列域错误的稳定 code 族。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskQueueErrorCode {
    /// 入参不合法。
    InvalidInput,
    /// 任务不存在。
    NotFound,
    /// 状态机不允许该转移。
    InvalidTransition,
    /// 自报者不是执行者 (认领授权)。
    NotAssignee,
    /// 该动作只能由人发起 (failed 只能人判)。
    HumanOnly,
    /// 任务链深度熔断 (环路检测)。
    ChainDepthExceeded,
    /// 持久存储读写失败。
    Store,
}

impl TaskQueueErrorCode {
    /// 稳定 code 字符串。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "task_queue_invalid_input",
            Self::NotFound => "task_queue_not_found",
            Self::InvalidTransition => "task_queue_invalid_transition",
            Self::NotAssignee => "task_queue_not_assignee",
            Self::HumanOnly => "task_queue_human_only",
            Self::ChainDepthExceeded => "task_queue_chain_depth_exceeded",
            Self::Store => "task_queue_store",
        }
    }
}

impl std::fmt::Display for TaskQueueErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 任务队列域的失败。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskQueueError {
    /// 入参不合法。
    InvalidInput {
        /// 人读原因。
        reason: String,
    },
    /// 任务不存在。
    NotFound {
        /// 任务 id。
        task_id: String,
    },
    /// 状态机不允许该转移 (错误即帧, 不静默吞)。
    InvalidTransition {
        /// 任务 id。
        task_id: String,
        /// 当前态。
        from: TaskState,
        /// 被拒的动作名。
        action: &'static str,
    },
    /// 自报者不是执行者 (委托者不能伪造工人状态)。
    NotAssignee {
        /// 任务 id。
        task_id: String,
        /// 自报者 id。
        caller: String,
    },
    /// 该动作只能由人发起 (failed 只能人判 / 重试是显式人判动作)。
    HumanOnly {
        /// 被拒的动作名。
        action: &'static str,
    },
    /// 任务链深度熔断 (spawn 链 N 层熔断)。
    ChainDepthExceeded {
        /// 父任务 id。
        parent_task_id: String,
        /// 将形成的链深。
        depth: u32,
        /// 配置上限。
        max_depth: u32,
    },
    /// 持久存储读写失败。
    Store {
        /// 失败的操作。
        operation: &'static str,
        /// 人读原因。
        reason: String,
    },
}

impl TaskQueueError {
    /// 错误归属 code。
    pub const fn code(&self) -> TaskQueueErrorCode {
        match self {
            Self::InvalidInput { .. } => TaskQueueErrorCode::InvalidInput,
            Self::NotFound { .. } => TaskQueueErrorCode::NotFound,
            Self::InvalidTransition { .. } => TaskQueueErrorCode::InvalidTransition,
            Self::NotAssignee { .. } => TaskQueueErrorCode::NotAssignee,
            Self::HumanOnly { .. } => TaskQueueErrorCode::HumanOnly,
            Self::ChainDepthExceeded { .. } => TaskQueueErrorCode::ChainDepthExceeded,
            Self::Store { .. } => TaskQueueErrorCode::Store,
        }
    }
}

impl std::fmt::Display for TaskQueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput { reason } => write!(f, "invalid task queue input: {reason}"),
            Self::NotFound { task_id } => write!(f, "task not found: {task_id}"),
            Self::InvalidTransition {
                task_id,
                from,
                action,
            } => write!(f, "task {task_id} cannot {action} from state {from}"),
            Self::NotAssignee { task_id, caller } => {
                write!(f, "caller {caller} is not the assignee of task {task_id}")
            }
            Self::HumanOnly { action } => write!(f, "{action} is human-judged only"),
            Self::ChainDepthExceeded {
                parent_task_id,
                depth,
                max_depth,
            } => write!(
                f,
                "task chain depth {depth} under parent {parent_task_id} exceeds the fuse {max_depth}"
            ),
            Self::Store { operation, reason } => write!(f, "task queue store {operation}: {reason}"),
        }
    }
}

impl std::error::Error for TaskQueueError {}

/// 任务队列域 result 别名。
pub type TaskQueueResult<T> = Result<T, TaskQueueError>;

// ============================================================================
// 状态机 / 记录
// ============================================================================

/// 任务状态 (六态, 转移永远显式)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// 入队待派。
    Pending,
    /// 已指派并已唤醒。
    Assigned,
    /// 执行段在跑。
    Running,
    /// 完成 (终态)。
    Done,
    /// 失败 (只能人判进入; 重试只能人判)。
    Failed,
    /// 外部等待 / 重投递熔断。
    Blocked,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Pending => "pending",
            Self::Assigned => "assigned",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        };
        f.write_str(name)
    }
}

/// 回合结算挂账 (一个回合 = 一个任务的执行段)。
///
/// **只接账不重复记**: 这里只有「哪个回合、引用哪本账」, 没有任何金额字段 ——
/// 「算不算花过钱」沿三维账本既有口径读
/// ([`crate::work_consumption::fold_consumed_work`]: 事件日志单趟折叠, 取消/
/// 中断同答案), 任务账不产生第二份花费记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskSettlement {
    /// 轮次号 (任务账里一回合一段)。
    pub turn: u64,
    /// 账本引用 (金额所在账本的身份, 只作指针)。
    pub ledger_ref: String,
    /// 挂账时刻 (epoch 毫秒)。
    pub linked_at_ms: i64,
}

/// 一条任务记录 (持久档的行语义)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskRecord {
    /// 任务 id (`task-<写派号>` 派号)。
    pub id: String,
    /// 任务标题。
    pub title: String,
    /// 不透明载荷 (队列不解释)。
    pub payload: serde_json::Value,
    /// 当前状态。
    pub state: TaskState,
    /// 入队时刻 (epoch 毫秒)。
    pub created_at_ms: i64,
    /// 最近状态推进时刻 (epoch 毫秒)。
    pub updated_at_ms: i64,
    /// 当前执行者 (可空)。
    pub assignee: Option<String>,
    /// 幂等键 (同键重复投递不重复执行)。
    pub idempotency_key: Option<String>,
    /// 父任务 id (任务链/spawn 链; 根任务为空)。
    pub parent_task_id: Option<String>,
    /// 链深 (根 = 0; 超上限的入队被熔断)。
    pub chain_depth: u32,
    /// 派发执行次数 (进入 Running + 崩溃恢复重派; 防崩溃死循环的计数)。
    pub attempts: u32,
    /// 重复投递计数 (同键重复投递 + 未投递唤醒重投)。
    pub redelivered: u32,
    /// 回合结算挂账 (按回合幂等, 一回合一段)。
    pub settlements: Vec<TaskSettlement>,
}

/// 唤醒信号 (assign 即唤醒; 与状态事件同汇, 体在持久唤醒队列)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskWake {
    /// 被唤醒的任务 id。
    pub task_id: String,
    /// 被唤醒的执行者。
    pub assignee: String,
    /// 唤醒时刻 (epoch 毫秒)。
    pub at_ms: i64,
    /// 是否已投递 ([`TaskQueue::drain_wakes`] 取走即 true; 重启后未投递的重投)。
    pub delivered: bool,
}

/// 任务队列事件 (事件账只增; 唤醒信号与状态事件**同汇**)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskEvent {
    /// 入队。
    Enqueued {
        /// 任务 id。
        task_id: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 同键重复投递被幂等去重 (不重复执行)。
    DeliveryDeduplicated {
        /// 既有任务 id。
        task_id: String,
        /// 幂等键。
        key: String,
        /// 时刻。
        at_ms: i64,
        /// 累计重复投递数。
        redelivered: u32,
    },
    /// 指派。
    Assigned {
        /// 任务 id。
        task_id: String,
        /// 执行者。
        assignee: String,
        /// 指派人 (留痕; 入队即派为 `<enqueue>`)。
        by: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 唤醒信号发出 (与状态事件同汇)。
    WakeEmitted {
        /// 任务 id。
        task_id: String,
        /// 被唤醒的执行者。
        assignee: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 执行段开始。
    ExecutionStarted {
        /// 任务 id。
        task_id: String,
        /// 执行者。
        assignee: String,
        /// 时刻。
        at_ms: i64,
        /// 累计派发次数。
        attempts: u32,
    },
    /// 完成。
    ExecutionCompleted {
        /// 任务 id。
        task_id: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 自报阻塞 (外部等待)。
    ExecutionBlocked {
        /// 任务 id。
        task_id: String,
        /// 阻塞原因。
        reason: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 人判失败 (failed 只能人判)。
    FailureJudged {
        /// 任务 id。
        task_id: String,
        /// 人判者。
        judge: String,
        /// 失败原因。
        reason: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 人判重试 (显式动作, 失败不自动重试)。
    RetryJudged {
        /// 任务 id。
        task_id: String,
        /// 人判者。
        judge: String,
        /// 重试理由。
        reason: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 崩溃恢复: 执行段未收场, 回 Pending 重派一次。
    RecoveredToPending {
        /// 任务 id。
        task_id: String,
        /// 时刻。
        at_ms: i64,
        /// 恢复后累计派发次数。
        attempts: u32,
    },
    /// 派发熔断: 恢复次数超上限, 转 Blocked (防崩溃死循环)。
    AttemptFused {
        /// 任务 id。
        task_id: String,
        /// 时刻。
        at_ms: i64,
        /// 累计派发次数。
        attempts: u32,
    },
    /// 任务链深度熔断审计 (环路检测)。
    ChainRefused {
        /// 父任务 id。
        parent_task_id: String,
        /// 将形成的链深。
        depth: u32,
        /// 配置上限。
        max_depth: u32,
        /// 时刻。
        at_ms: i64,
    },
    /// 结算挂账 (一回合一段)。
    SettlementLinked {
        /// 任务 id。
        task_id: String,
        /// 轮次号。
        turn: u64,
        /// 账本引用。
        ledger_ref: String,
        /// 时刻。
        at_ms: i64,
    },
    /// 重复挂账被忽略 (无重复记账)。
    SettlementDuplicateIgnored {
        /// 任务 id。
        task_id: String,
        /// 轮次号。
        turn: u64,
        /// 时刻。
        at_ms: i64,
    },
    /// 未投递唤醒在重启后重投 (redelivered 计数)。
    WakeRedelivered {
        /// 任务 id。
        task_id: String,
        /// 被唤醒的执行者。
        assignee: String,
        /// 时刻。
        at_ms: i64,
    },
}

/// 任务队列持久档 (内存镜像; 每次原子读改写整档落盘)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TaskQueueState {
    /// 已应用的最大写派号 (跨重启单调, 派任务 id 用)。
    pub write_seq: u64,
    /// 任务表 (id → 记录)。
    pub tasks: BTreeMap<String, TaskRecord>,
    /// 唤醒队列 (assign 即唤醒; 未投递的重启后重投)。
    pub wakes: Vec<TaskWake>,
    /// 事件账 (只增; 唤醒信号与状态事件同汇)。
    pub journal: Vec<TaskEvent>,
}

// ============================================================================
// 存储端口
// ============================================================================

/// 任务队列持久化端口: **一次 [`TaskStore::update`] = 一次原子读改写落盘**。
///
/// 实现负责原子性 (同店多手/多进程互不覆盖)。闭包的变更**一律落盘** ——
/// 连拒绝路径的审计事件也要落盘, 因此闭包须自律「要么全改, 要么只写审计」,
/// 判定结果经闭包捕获通道返回 ([`TaskQueue`] 的各方法即是这么写的)。
pub trait TaskStore: Send + Sync {
    /// 读当前整档。
    fn read(&self) -> TaskQueueResult<TaskQueueState>;
    /// 原子读改写: 闭包对内存镜像的变更一次性落盘。
    fn update(&self, update: &mut dyn FnMut(&mut TaskQueueState)) -> TaskQueueResult<()>;
}

/// 内存镜像存储 (进程内; 测试/无持久面的组合用)。
#[derive(Debug, Default)]
pub struct InMemoryTaskStore {
    state: Mutex<TaskQueueState>,
}

impl InMemoryTaskStore {
    /// 空镜像。
    pub fn new() -> Self {
        Self::default()
    }
}

impl TaskStore for InMemoryTaskStore {
    fn read(&self) -> TaskQueueResult<TaskQueueState> {
        let guard = self.state.lock().map_err(|error| TaskQueueError::Store {
            operation: "read",
            reason: error.to_string(),
        })?;
        Ok(guard.clone())
    }

    fn update(&self, update: &mut dyn FnMut(&mut TaskQueueState)) -> TaskQueueResult<()> {
        let mut guard = self.state.lock().map_err(|error| TaskQueueError::Store {
            operation: "update",
            reason: error.to_string(),
        })?;
        update(&mut guard);
        Ok(())
    }
}

// ============================================================================
// 入参 / 出参
// ============================================================================

/// 调用者身份 (认领授权: 自报必须是本人; fail/retry 只能人判)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCaller {
    /// 调用者 id (会话/代理/人的稳定标识)。
    pub id: String,
    /// 是否人 (人判面的准入)。
    pub is_human: bool,
}

impl TaskCaller {
    /// agent 调用者 (不能进人判面)。
    pub fn agent(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            is_human: false,
        }
    }

    /// 人调用者 (人判面: fail / retry)。
    pub fn human(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            is_human: true,
        }
    }
}

/// 入队规格。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskSpec {
    /// 任务标题 (必填, 非空)。
    pub title: String,
    /// 不透明载荷。
    pub payload: serde_json::Value,
    /// 幂等键 (可空; 同键重复投递不重复执行)。
    pub idempotency_key: Option<String>,
    /// 初始执行者 (可空; 给了即 Assigned + 唤醒)。
    pub assignee: Option<String>,
    /// 父任务 id (可空; 形成任务链, 深度超限熔断)。
    pub parent_task_id: Option<String>,
}

impl TaskSpec {
    /// 标题 + 载荷 (其余字段可选)。
    pub fn new(title: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            title: title.into(),
            payload,
            idempotency_key: None,
            assignee: None,
            parent_task_id: None,
        }
    }

    /// 设幂等键。
    #[must_use]
    pub fn with_idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// 设初始执行者 (入队即 Assigned + 唤醒)。
    #[must_use]
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// 设父任务 (任务链)。
    #[must_use]
    pub fn with_parent(mut self, parent_task_id: impl Into<String>) -> Self {
        self.parent_task_id = Some(parent_task_id.into());
        self
    }
}

/// 入队结果 (幂等去重时回既有任务, 不产生第二个任务)。
#[derive(Debug, Clone, PartialEq)]
pub enum EnqueueOutcome {
    /// 新任务已入队。
    Enqueued {
        /// 新任务记录。
        task: TaskRecord,
        /// 随入队发出的唤醒 (带初始执行者时有)。
        wake: Option<TaskWake>,
    },
    /// 同键重复投递被幂等去重 (单执行)。
    Deduplicated {
        /// 既有任务 id。
        task_id: String,
        /// 既有任务累计重复投递数。
        redelivered: u32,
    },
}

/// 结算挂账结果 (一回合一段, 重复挂账不重复记)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementOutcome {
    /// 已挂上 (新段)。
    Linked {
        /// 任务 id。
        task_id: String,
        /// 轮次号。
        turn: u64,
    },
    /// 该回合已挂过 (无重复记账)。
    AlreadyLinked {
        /// 任务 id。
        task_id: String,
        /// 轮次号。
        turn: u64,
    },
}

/// 一次快照读的完整结果 (面板读面: 任务表 + 分态计数 + 事件账**同一刻**)。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskSnapshot {
    /// 全队任务 (稳定排序: 入队时刻, 同刻按 id)。
    pub tasks: Vec<TaskRecord>,
    /// 分态计数 (与 [`TaskQueue::counts`] 同口径)。
    pub counts: TaskBoardCounts,
    /// 事件账 (只增; 唤醒信号与状态事件同汇)。
    pub journal: Vec<TaskEvent>,
}

/// 自述任务节计数 (真值; `tool.self_status` 任务节的来源)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBoardCounts {
    /// `Pending` 计数。
    pub pending: u64,
    /// `Assigned` 计数。
    pub assigned: u64,
    /// `Running` 计数。
    pub running: u64,
    /// `Done` 计数。
    pub done: u64,
    /// `Failed` 计数。
    pub failed: u64,
    /// `Blocked` 计数。
    pub blocked: u64,
}

// ============================================================================
// 配置
// ============================================================================

/// 默认派发次数上限 (崩溃恢复重派超此数熔断为 Blocked)。
pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;

/// 默认任务链深度上限 (spawn 链 N 层熔断) = 5 层 (设计裁定 R1⑳: 环路上限深度 5)。
pub const DEFAULT_MAX_CHAIN_DEPTH: u32 = 5;

/// 队列配置 (两个熔断: 崩溃死循环 / 环路)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskQueueConfig {
    /// 派发次数上限 (进入 Running + 崩溃恢复重派的合计; 超限熔断为 Blocked)。
    pub max_attempts: u32,
    /// 任务链深度上限 (根 = 0; 超限拒绝即帧 + 审计)。
    pub max_chain_depth: u32,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            max_chain_depth: DEFAULT_MAX_CHAIN_DEPTH,
        }
    }
}

// ============================================================================
// 队列
// ============================================================================

/// 队列时钟 (epoch 毫秒; 测试可注入定值)。
pub type TaskClockMs = Arc<dyn Fn() -> i64 + Send + Sync>;

fn system_clock_ms() -> TaskClockMs {
    Arc::new(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(i64::MAX)
    })
}

/// 持久任务队列: 状态机 + 幂等唤醒 + 崩溃恢复 + 环路熔断 + 结算接账。
///
/// 全部方法 `&self`: 并发安全由 [`TaskStore::update`] 的原子读改写承载
/// (同店多手互不覆盖)。
pub struct TaskQueue {
    store: Arc<dyn TaskStore>,
    clock: TaskClockMs,
    config: TaskQueueConfig,
}

impl std::fmt::Debug for TaskQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskQueue")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl TaskQueue {
    /// 打开队列并执行崩溃恢复 ([`TaskQueue::open`] 即恢复点)。
    ///
    /// 恢复语义: `Running`/`Assigned` 回 `Pending` 重派一次 (`attempts` +1),
    /// 超 [`TaskQueueConfig::max_attempts`] 的熔断为 `Blocked`; 未投递唤醒留在
    /// 唤醒队列重投 (`redelivered` +1)。`Failed` **不被恢复触碰** (失败不自动
    /// 重试), `Done` 是终态。
    pub fn open(store: Arc<dyn TaskStore>, config: TaskQueueConfig) -> TaskQueueResult<Self> {
        Self::open_with_clock(store, config, system_clock_ms())
    }

    /// 打开队列 (注入时钟; 测试用)。
    pub fn open_with_clock(
        store: Arc<dyn TaskStore>,
        config: TaskQueueConfig,
        clock: TaskClockMs,
    ) -> TaskQueueResult<Self> {
        let queue = Self {
            store,
            clock,
            config,
        };
        queue.recover_on_open()?;
        Ok(queue)
    }

    /// 附从句柄: 同店第二把句柄, **不跑崩溃恢复** (开失败面也没有假帧)。
    ///
    /// 崩溃恢复 ([`TaskQueue::open`] = 恢复点) 是主句柄的专属语义 —— 每店恰
    /// 一个主句柄; 附从句柄给同店旁路面板 (读面 + 人判面) 并用。重复恢复会
    /// 假记 `WakeRedelivered` 审计、把在跑任务重派回 `Pending`, 绝不允许。
    pub fn attach(store: Arc<dyn TaskStore>, config: TaskQueueConfig) -> Self {
        Self::attach_with_clock(store, config, system_clock_ms())
    }

    /// 附从句柄 (注入时钟; 测试用)。
    pub fn attach_with_clock(
        store: Arc<dyn TaskStore>,
        config: TaskQueueConfig,
        clock: TaskClockMs,
    ) -> Self {
        Self {
            store,
            clock,
            config,
        }
    }

    /// 已应用的最大写派号 (跨重启单调)。
    pub fn write_seq(&self) -> TaskQueueResult<u64> {
        Ok(self.store.read()?.write_seq)
    }

    // ------------------------------------------------------------------
    // 入队 / 指派
    // ------------------------------------------------------------------

    /// 耐久入队 (幂等键去重 + 任务链深度熔断)。
    ///
    /// - 同 `idempotency_key` 重复投递 = 不重复执行 (回既有任务, `redelivered`+1);
    /// - 带 `assignee` 的入队即 [`TaskQueue::assign`] 语义: `Assigned` + 唤醒;
    /// - 带 `parent_task_id` 的链深超限 = 拒绝即帧 + 审计。
    pub fn enqueue(&self, spec: TaskSpec) -> TaskQueueResult<EnqueueOutcome> {
        self.validate_spec(&spec)?;
        let now = (self.clock)();
        let config = self.config;
        let mut result: TaskQueueResult<EnqueueOutcome> = Err(TaskQueueError::Store {
            operation: "update",
            reason: "enqueue closure did not run".to_string(),
        });
        self.store.update(&mut |state| {
            state.write_seq = state.write_seq.saturating_add(1);
            // 幂等去重: 同键在任何状态下都不产生第二个可执行任务。
            if let Some(key) = spec.idempotency_key.as_deref() {
                if let Some(existing) = state
                    .tasks
                    .values_mut()
                    .find(|task| task.idempotency_key.as_deref() == Some(key))
                {
                    existing.redelivered = existing.redelivered.saturating_add(1);
                    existing.updated_at_ms = now;
                    let task_id = existing.id.clone();
                    let redelivered = existing.redelivered;
                    state.journal.push(TaskEvent::DeliveryDeduplicated {
                        task_id: task_id.clone(),
                        key: key.to_string(),
                        at_ms: now,
                        redelivered,
                    });
                    result = Ok(EnqueueOutcome::Deduplicated {
                        task_id,
                        redelivered,
                    });
                    return;
                }
            }
            // 任务链深度 (环路熔断): 超限拒绝即帧, 审计照样落盘。
            let (parent_task_id, chain_depth) = match spec.parent_task_id.as_deref() {
                None => (None, 0u32),
                Some(parent_id) => match state.tasks.get(parent_id) {
                    Some(parent) => (
                        Some(parent_id.to_string()),
                        parent.chain_depth.saturating_add(1),
                    ),
                    None => {
                        result = Err(TaskQueueError::NotFound {
                            task_id: parent_id.to_string(),
                        });
                        return;
                    }
                },
            };
            if chain_depth > config.max_chain_depth {
                let parent_task_id = parent_task_id.unwrap_or_else(|| "<root>".to_string());
                state.journal.push(TaskEvent::ChainRefused {
                    parent_task_id: parent_task_id.clone(),
                    depth: chain_depth,
                    max_depth: config.max_chain_depth,
                    at_ms: now,
                });
                result = Err(TaskQueueError::ChainDepthExceeded {
                    parent_task_id,
                    depth: chain_depth,
                    max_depth: config.max_chain_depth,
                });
                return;
            }
            let id = format!("task-{}", state.write_seq);
            let assignee = spec.assignee.clone();
            let record = TaskRecord {
                id: id.clone(),
                title: spec.title.clone(),
                payload: spec.payload.clone(),
                state: if assignee.is_some() {
                    TaskState::Assigned
                } else {
                    TaskState::Pending
                },
                created_at_ms: now,
                updated_at_ms: now,
                assignee: assignee.clone(),
                idempotency_key: spec.idempotency_key.clone(),
                parent_task_id,
                chain_depth,
                attempts: 0,
                redelivered: 0,
                settlements: Vec::new(),
            };
            state.journal.push(TaskEvent::Enqueued {
                task_id: id.clone(),
                at_ms: now,
            });
            // 带执行者的入队 = assign 即唤醒 (唤醒信号与状态事件同汇)。
            let wake = assignee.map(|assignee| {
                state.journal.push(TaskEvent::Assigned {
                    task_id: id.clone(),
                    assignee: assignee.clone(),
                    by: "<enqueue>".to_string(),
                    at_ms: now,
                });
                state.journal.push(TaskEvent::WakeEmitted {
                    task_id: id.clone(),
                    assignee: assignee.clone(),
                    at_ms: now,
                });
                TaskWake {
                    task_id: id.clone(),
                    assignee,
                    at_ms: now,
                    delivered: false,
                }
            });
            if let Some(wake) = wake.clone() {
                state.wakes.push(wake);
            }
            state.tasks.insert(id, record.clone());
            result = Ok(EnqueueOutcome::Enqueued { task: record, wake });
        })?;
        result
    }

    /// 指派/改派 (assign 即唤醒: 唤醒信号与状态事件同汇)。
    ///
    /// 目标态须 `Pending`/`Assigned`/`Blocked` (状态机显式, 非法转移拒绝即帧);
    /// 指派人留痕 ([`TaskEvent::Assigned`]), 授权把关在审批档 (任务簿写面每次
    /// 人批), 队列层不叠第二把锁 —— 自报面的认领授权见 [`TaskQueue::complete`]。
    pub fn assign(
        &self,
        task_id: &str,
        assignee: &str,
        caller: &TaskCaller,
    ) -> TaskQueueResult<TaskRecord> {
        let now = (self.clock)();
        let task_id = task_id.to_string();
        let assignee = assignee.to_string();
        let caller = caller.clone();
        let mut result: TaskQueueResult<TaskRecord> = Err(TaskQueueError::Store {
            operation: "update",
            reason: "assign closure did not run".to_string(),
        });
        self.store.update(&mut |state| {
            let Some(record) = state.tasks.get(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            if !matches!(
                record.state,
                TaskState::Pending | TaskState::Assigned | TaskState::Blocked
            ) {
                result = Err(TaskQueueError::InvalidTransition {
                    task_id: task_id.clone(),
                    from: record.state,
                    action: "assign",
                });
                return;
            }
            let Some(record) = state.tasks.get_mut(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            record.state = TaskState::Assigned;
            record.assignee = Some(assignee.clone());
            record.updated_at_ms = now;
            state.journal.push(TaskEvent::Assigned {
                task_id: task_id.clone(),
                assignee: assignee.clone(),
                by: caller.id.clone(),
                at_ms: now,
            });
            // assign 即唤醒 (唤醒信号=事件汇同源)。
            state.journal.push(TaskEvent::WakeEmitted {
                task_id: task_id.clone(),
                assignee: assignee.clone(),
                at_ms: now,
            });
            state.wakes.push(TaskWake {
                task_id: task_id.clone(),
                assignee: assignee.clone(),
                at_ms: now,
                delivered: false,
            });
            result = Ok(record.clone());
        })?;
        result
    }

    /// 取走待投递的唤醒 (取走即标已投递; 未取走的重启后重投)。
    pub fn drain_wakes(&self) -> TaskQueueResult<Vec<TaskWake>> {
        let mut result: TaskQueueResult<Vec<TaskWake>> = Err(TaskQueueError::Store {
            operation: "update",
            reason: "drain closure did not run".to_string(),
        });
        self.store.update(&mut |state| {
            let mut taken = Vec::new();
            for wake in &mut state.wakes {
                if !wake.delivered {
                    wake.delivered = true;
                    taken.push(wake.clone());
                }
            }
            result = Ok(taken);
        })?;
        result
    }

    // ------------------------------------------------------------------
    // 执行段 (自报必须是本人)
    // ------------------------------------------------------------------

    /// 执行段开始 (`Assigned` → `Running`, `attempts`+1)。
    ///
    /// 自报必须是本人: `caller.id == assignee` (委托者不能伪造工人状态);
    /// 人可代任何执行者推进。
    pub fn begin_execution(
        &self,
        task_id: &str,
        caller: &TaskCaller,
    ) -> TaskQueueResult<TaskRecord> {
        let now = (self.clock)();
        let task_id = task_id.to_string();
        let caller = caller.clone();
        let mut result: TaskQueueResult<TaskRecord> = Err(TaskQueueError::Store {
            operation: "update",
            reason: "begin closure did not run".to_string(),
        });
        self.store.update(&mut |state| {
            let Some(record) = state.tasks.get(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            if record.state != TaskState::Assigned {
                result = Err(TaskQueueError::InvalidTransition {
                    task_id: task_id.clone(),
                    from: record.state,
                    action: "begin_execution",
                });
                return;
            }
            if !caller.is_human && record.assignee.as_deref() != Some(caller.id.as_str()) {
                result = Err(TaskQueueError::NotAssignee {
                    task_id: task_id.clone(),
                    caller: caller.id.clone(),
                });
                return;
            }
            let Some(record) = state.tasks.get_mut(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            record.state = TaskState::Running;
            record.attempts = record.attempts.saturating_add(1);
            record.updated_at_ms = now;
            state.journal.push(TaskEvent::ExecutionStarted {
                task_id: task_id.clone(),
                assignee: record.assignee.clone().unwrap_or_default(),
                at_ms: now,
                attempts: record.attempts,
            });
            result = Ok(record.clone());
        })?;
        result
    }

    /// 完成 (`Assigned`/`Running` → `Done`)。自报必须是本人。
    pub fn complete(&self, task_id: &str, caller: &TaskCaller) -> TaskQueueResult<TaskRecord> {
        self.self_report_to(task_id, caller, TaskState::Done, "complete", None)
    }

    /// 自报阻塞 (`Assigned`/`Running` → `Blocked`, 外部等待)。自报必须是本人。
    pub fn block(
        &self,
        task_id: &str,
        caller: &TaskCaller,
        reason: &str,
    ) -> TaskQueueResult<TaskRecord> {
        self.self_report_to(task_id, caller, TaskState::Blocked, "block", Some(reason))
    }

    fn self_report_to(
        &self,
        task_id: &str,
        caller: &TaskCaller,
        to: TaskState,
        action: &'static str,
        reason: Option<&str>,
    ) -> TaskQueueResult<TaskRecord> {
        let now = (self.clock)();
        let task_id = task_id.to_string();
        let caller = caller.clone();
        let reason = reason.map(str::to_string);
        let mut result: TaskQueueResult<TaskRecord> = Err(TaskQueueError::Store {
            operation: "update",
            reason: "self-report closure did not run".to_string(),
        });
        self.store.update(&mut |state| {
            let Some(record) = state.tasks.get(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            if !matches!(record.state, TaskState::Assigned | TaskState::Running) {
                result = Err(TaskQueueError::InvalidTransition {
                    task_id: task_id.clone(),
                    from: record.state,
                    action,
                });
                return;
            }
            // 自报必须是本人: caller == assignee (人可代推进)。
            if !caller.is_human && record.assignee.as_deref() != Some(caller.id.as_str()) {
                result = Err(TaskQueueError::NotAssignee {
                    task_id: task_id.clone(),
                    caller: caller.id.clone(),
                });
                return;
            }
            let Some(record) = state.tasks.get_mut(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            record.state = to;
            record.updated_at_ms = now;
            match to {
                TaskState::Done => state.journal.push(TaskEvent::ExecutionCompleted {
                    task_id: task_id.clone(),
                    at_ms: now,
                }),
                TaskState::Blocked => state.journal.push(TaskEvent::ExecutionBlocked {
                    task_id: task_id.clone(),
                    reason: reason.clone().unwrap_or_default(),
                    at_ms: now,
                }),
                _ => {}
            }
            result = Ok(record.clone());
        })?;
        result
    }

    // ------------------------------------------------------------------
    // 人判面 (failed 只能人判; 重试是显式动作)
    // ------------------------------------------------------------------

    /// 人判失败 (唯一失败入口; agent 调用结构性拒绝)。
    ///
    /// 进入 `Failed` 后**不自动重试**: 崩溃恢复不触碰 `Failed`, 回 `Pending`
    /// 的唯一路径是 [`TaskQueue::human_judge_retry`]。
    pub fn human_judge_fail(
        &self,
        task_id: &str,
        judge: &TaskCaller,
        reason: &str,
    ) -> TaskQueueResult<TaskRecord> {
        self.human_judge_move(
            task_id,
            judge,
            TaskState::Failed,
            "human_judge_fail",
            reason,
        )
    }

    /// 人判重试 (显式动作, 防雪崩: 失败绝不自动重试)。
    ///
    /// `Failed` → `Pending` (回队待派, 不自动唤醒 —— 再派走 [`TaskQueue::assign`])。
    pub fn human_judge_retry(
        &self,
        task_id: &str,
        judge: &TaskCaller,
        reason: &str,
    ) -> TaskQueueResult<TaskRecord> {
        self.human_judge_move(
            task_id,
            judge,
            TaskState::Pending,
            "human_judge_retry",
            reason,
        )
    }

    fn human_judge_move(
        &self,
        task_id: &str,
        judge: &TaskCaller,
        to: TaskState,
        action: &'static str,
        reason: &str,
    ) -> TaskQueueResult<TaskRecord> {
        if !judge.is_human {
            return Err(TaskQueueError::HumanOnly { action });
        }
        let now = (self.clock)();
        let task_id = task_id.to_string();
        let judge = judge.id.clone();
        let reason = reason.to_string();
        let mut result: TaskQueueResult<TaskRecord> = Err(TaskQueueError::Store {
            operation: "update",
            reason: "human-judge closure did not run".to_string(),
        });
        self.store.update(&mut |state| {
            let Some(record) = state.tasks.get(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            let allowed = match to {
                TaskState::Failed => !matches!(record.state, TaskState::Done | TaskState::Failed),
                TaskState::Pending => record.state == TaskState::Failed,
                _ => false,
            };
            if !allowed {
                result = Err(TaskQueueError::InvalidTransition {
                    task_id: task_id.clone(),
                    from: record.state,
                    action,
                });
                return;
            }
            let Some(record) = state.tasks.get_mut(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            record.state = to;
            record.updated_at_ms = now;
            match to {
                TaskState::Failed => state.journal.push(TaskEvent::FailureJudged {
                    task_id: task_id.clone(),
                    judge: judge.clone(),
                    reason: reason.clone(),
                    at_ms: now,
                }),
                TaskState::Pending => state.journal.push(TaskEvent::RetryJudged {
                    task_id: task_id.clone(),
                    judge: judge.clone(),
                    reason: reason.clone(),
                    at_ms: now,
                }),
                _ => {}
            }
            result = Ok(record.clone());
        })?;
        result
    }

    // ------------------------------------------------------------------
    // 结算接账 (只接账不重复记)
    // ------------------------------------------------------------------

    /// 把回合结算挂到任务账 (一个回合 = 一个任务的执行段)。
    ///
    /// 同一回合只挂一次: 重复挂账回 [`SettlementOutcome::AlreadyLinked`] 并留
    /// 「无重复记账」审计。挂账**只存轮次与账本引用**, 金额沿三维账本既有口径
    /// ([`crate::work_consumption`]) 读, 本动作不产生第二份花费记录。
    pub fn link_settlement(
        &self,
        task_id: &str,
        settlement: TaskSettlement,
    ) -> TaskQueueResult<SettlementOutcome> {
        let now = (self.clock)();
        let task_id = task_id.to_string();
        let mut result: TaskQueueResult<SettlementOutcome> = Err(TaskQueueError::Store {
            operation: "update",
            reason: "settlement closure did not run".to_string(),
        });
        self.store.update(&mut |state| {
            let Some(record) = state.tasks.get_mut(&task_id) else {
                result = Err(TaskQueueError::NotFound {
                    task_id: task_id.clone(),
                });
                return;
            };
            if record
                .settlements
                .iter()
                .any(|settled| settled.turn == settlement.turn)
            {
                state.journal.push(TaskEvent::SettlementDuplicateIgnored {
                    task_id: task_id.clone(),
                    turn: settlement.turn,
                    at_ms: now,
                });
                result = Ok(SettlementOutcome::AlreadyLinked {
                    task_id: task_id.clone(),
                    turn: settlement.turn,
                });
                return;
            }
            record.settlements.push(settlement.clone());
            record.updated_at_ms = now;
            state.journal.push(TaskEvent::SettlementLinked {
                task_id: task_id.clone(),
                turn: settlement.turn,
                ledger_ref: settlement.ledger_ref.clone(),
                at_ms: now,
            });
            result = Ok(SettlementOutcome::Linked {
                task_id: task_id.clone(),
                turn: settlement.turn,
            });
        })?;
        result
    }

    // ------------------------------------------------------------------
    // 读面
    // ------------------------------------------------------------------

    /// 读一条任务。
    pub fn get(&self, task_id: &str) -> TaskQueueResult<TaskRecord> {
        self.store
            .read()?
            .tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| TaskQueueError::NotFound {
                task_id: task_id.to_string(),
            })
    }

    /// 全队快照 (稳定排序: 入队时刻, 同刻按 id)。
    pub fn list(&self) -> TaskQueueResult<Vec<TaskRecord>> {
        let mut tasks: Vec<TaskRecord> = self.store.read()?.tasks.into_values().collect();
        tasks.sort_by(|left, right| {
            (left.created_at_ms, &left.id).cmp(&(right.created_at_ms, &right.id))
        });
        Ok(tasks)
    }

    /// 面板一次快照读: 任务表 + 分态计数 + 事件账**一次**读出 (同一时刻,
    /// 绝不拼两个时刻的状态); 排序与 [`TaskQueue::list`] 同口径, 计数与
    /// [`TaskQueue::counts`] 同口径。
    pub fn snapshot(&self) -> TaskQueueResult<TaskSnapshot> {
        let state = self.store.read()?;
        let mut tasks: Vec<TaskRecord> = state.tasks.into_values().collect();
        tasks.sort_by(|left, right| {
            (left.created_at_ms, &left.id).cmp(&(right.created_at_ms, &right.id))
        });
        let mut counts = TaskBoardCounts::default();
        for task in &tasks {
            match task.state {
                TaskState::Pending => counts.pending += 1,
                TaskState::Assigned => counts.assigned += 1,
                TaskState::Running => counts.running += 1,
                TaskState::Done => counts.done += 1,
                TaskState::Failed => counts.failed += 1,
                TaskState::Blocked => counts.blocked += 1,
            }
        }
        Ok(TaskSnapshot {
            tasks,
            counts,
            journal: state.journal,
        })
    }

    /// 分态计数 (自述任务节真值来源)。
    pub fn counts(&self) -> TaskQueueResult<TaskBoardCounts> {
        let mut counts = TaskBoardCounts::default();
        for task in self.store.read()?.tasks.values() {
            match task.state {
                TaskState::Pending => counts.pending += 1,
                TaskState::Assigned => counts.assigned += 1,
                TaskState::Running => counts.running += 1,
                TaskState::Done => counts.done += 1,
                TaskState::Failed => counts.failed += 1,
                TaskState::Blocked => counts.blocked += 1,
            }
        }
        Ok(counts)
    }

    /// 事件账 (只增; 唤醒信号与状态事件同汇)。
    pub fn journal(&self) -> TaskQueueResult<Vec<TaskEvent>> {
        Ok(self.store.read()?.journal)
    }

    // ------------------------------------------------------------------
    // 内部
    // ------------------------------------------------------------------

    fn validate_spec(&self, spec: &TaskSpec) -> TaskQueueResult<()> {
        if spec.title.trim().is_empty() {
            return Err(TaskQueueError::InvalidInput {
                reason: "task title must not be empty".to_string(),
            });
        }
        if spec
            .idempotency_key
            .as_deref()
            .is_some_and(|key| key.trim().is_empty())
        {
            return Err(TaskQueueError::InvalidInput {
                reason: "idempotency key must not be blank".to_string(),
            });
        }
        if spec
            .assignee
            .as_deref()
            .is_some_and(|who| who.trim().is_empty())
        {
            return Err(TaskQueueError::InvalidInput {
                reason: "assignee must not be blank".to_string(),
            });
        }
        Ok(())
    }

    /// 崩溃恢复 (open 即恢复点): 见 [`TaskQueue::open`]。
    fn recover_on_open(&self) -> TaskQueueResult<()> {
        let now = (self.clock)();
        let max_attempts = self.config.max_attempts;
        self.store.update(&mut |state| {
            // 未投递唤醒: 留在唤醒队列重投, 重投递计数 +1 + 审计。
            for wake in &state.wakes {
                if wake.delivered {
                    continue;
                }
                if let Some(record) = state.tasks.get_mut(&wake.task_id) {
                    record.redelivered = record.redelivered.saturating_add(1);
                }
                state.journal.push(TaskEvent::WakeRedelivered {
                    task_id: wake.task_id.clone(),
                    assignee: wake.assignee.clone(),
                    at_ms: now,
                });
            }
            // 执行段未收场 (Running/Assigned): 回 Pending 重派一次;
            // 超派发上限熔断为 Blocked (防崩溃死循环)。
            let unfinished: Vec<String> = state
                .tasks
                .values()
                .filter(|task| matches!(task.state, TaskState::Running | TaskState::Assigned))
                .map(|task| task.id.clone())
                .collect();
            for task_id in unfinished {
                let Some(record) = state.tasks.get_mut(&task_id) else {
                    continue;
                };
                record.attempts = record.attempts.saturating_add(1);
                record.updated_at_ms = now;
                if record.attempts > max_attempts {
                    record.state = TaskState::Blocked;
                    state.journal.push(TaskEvent::AttemptFused {
                        task_id,
                        at_ms: now,
                        attempts: record.attempts,
                    });
                } else {
                    record.state = TaskState::Pending;
                    state.journal.push(TaskEvent::RecoveredToPending {
                        task_id,
                        at_ms: now,
                        attempts: record.attempts,
                    });
                }
            }
        })
    }
}
