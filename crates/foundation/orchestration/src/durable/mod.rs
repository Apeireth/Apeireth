//! `apeireth-orchestration::durable` — 持久执行原语 (durable execution primitives).
//!
//! **语义来源 (semantic salvage)**:
//! - `legacy/canonical/apeireth-workflow` (Temporal-lite `EventHistory` + Activity 语义)
//! - `legacy/canonical/apeireth-supervisor/src/journal_entry.rs` (chidori 式 host-call
//!   journal: 单调 seq + JSONL 行序列化 + 确定性重放元数据)
//! - `legacy/canonical/apeireth-bus/src/event_log.rs` (replay 查询语义: filter / since / last_n)
//! - `legacy/frozen/apeireth-task` (7 状态守门状态机 + 重试策略常量: max_retries=3,
//!   backoff 1s→2s→4s)
//!
//! **本模块提供的四个正交原语**:
//! 1. `DurableHistory` — 追加式活动事件日志 (typed events, 单调 seq, JSONL 往返,
//!    按 activity / 时间 / last_n 查询)
//! 2. `DurableRun` — 确定性重放引擎: 同一 input + 同一 history 前缀 → 同一 output,
//!    且**已记录完成的活动零副作用重放** (这是 legacy canonical 只声称而未实现的真重放:
//!    canonical 的 "replay" 只是整段重跑并重新执行全部副作用)
//! 3. `RetryPolicy` — 尝试预算 + 确定性退避计划 (尝试号作为 failure/retry 元数据
//!    记入事件流)
//! 4. `ActivityStateMachine` — 活动调用守门状态机 (Pending/Scheduled/Running/
//!    Succeeded/Failed/Cancelled/TimedOut; Failed/TimedOut 可重入, Cancelled 不可)
//!
//! **架构边界 (frozen v2 — 非谈判项)**:
//! - 这里的 `Run` **不是** Main Loop, 也不是第二个 agent 循环。它是一个纯库级
//!   step-function 原语: 调用方自带确定性 step 闭包与 `ActivityExecutor`, 本模块
//!   不拥有执行权、不注册工作流、不 spawn 任务、不 tick 定时器。
//! - 0 LLM 调用: 本模块纯确定性, 不经过 ProviderRouter (LLM 调用若需要持久语义,
//!   由 ModuleContext → invoker_handle() → canonical governance 路径自行组合本原语)。
//! - 未生产接线 (NOT production-wired, NOT default enabled):
//!   - **SubLoop 映射**: 未来一个 Module 可以在 bounded SubLoop 内驱动 durable step,
//!     并把每次受治理的工具调用记为 activity event, 使中断的 SubLoop 通过重放恢复
//!     而不重付副作用。本 wave 只交付原语, 不 spawn 任何循环。
//!   - **Continuation 映射**: `DurableHistory` 可 serde (整段或 JSONL), 可由未来适配器
//!     放入 `ContinuationSnapshot` / 经任意 `ContinuationStore` 持久化; 测试验证了
//!     JSONL 往返, 存储路径已证明但未接线。
//!   - **Future scheduler 映射**: `RetryPolicy::backoff_ms` 只返回时长, 异步 sleep
//!     由调用方拥有; 本模块不创建任何定时任务 (无 daemon ticking)。

#![forbid(unsafe_code)]

mod history;
mod replay;
mod retry;

pub use history::{
    now_epoch_ms, ActivityEvent, ActivityEventKind, DurableHistory, RUN_EVENT_ACTOR,
};
pub use replay::{ActivityExecutor, DurableRun};
pub use retry::{
    ActivityState, ActivityStateError, ActivityStateMachine, RetryPolicy, ACTIVITY_STATE_COUNT,
    SUPPORTED_ACTIVITY_STATES,
};

/// 持久执行原语的可判别错误。
///
/// 所有变体均为 fail-closed: 重放不一致与损坏日志显式报错, 绝不静默重跑副作用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DurableError {
    /// 日志非法 (seq 不单调 / 事件序列不符合追加序不变量)。
    HistoryCorrupted {
        /// 人读原因
        reason: String,
    },
    /// 重放分歧: step 函数请求的活动与日志中下一个已记录的调度不一致
    /// (确定性被破坏的显式信号, 对应 canonical `WorkflowError::HistoryCorrupted` 的升级)。
    ReplayMismatch {
        /// 请求的活动 ID
        activity_id: String,
        /// 期望输入
        expected_input: String,
        /// 日志中实际记录的 (activity_id, input) 摘要
        found: String,
    },
    /// 活动在耗尽重试预算后仍失败 (或重放命中已记录的最终失败)。
    ActivityFailed {
        /// 活动 ID
        activity_id: String,
        /// 底层错误
        error: String,
        /// 失败时的尝试号 (failure/retry 元数据)
        attempt: u32,
    },
    /// Run 已终结 (complete/fail 之后不得再执行活动)。
    RunAlreadyFinished,
}

impl std::fmt::Display for DurableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HistoryCorrupted { reason } => write!(f, "durable history corrupted: {reason}"),
            Self::ReplayMismatch {
                activity_id,
                expected_input,
                found,
            } => write!(
                f,
                "replay mismatch for activity {activity_id:?}: expected input {expected_input}, journal has {found}"
            ),
            Self::ActivityFailed {
                activity_id,
                error,
                attempt,
            } => write!(
                f,
                "activity {activity_id:?} failed (attempt {attempt}): {error}"
            ),
            Self::RunAlreadyFinished => write!(f, "durable run already finished"),
        }
    }
}

impl std::error::Error for DurableError {}

/// 持久执行 result 类型。
pub type DurableResult<T> = Result<T, DurableError>;
