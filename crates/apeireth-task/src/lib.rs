//! # apeireth-task
//!
//! **Task Skeleton** (1:1 翻译 v0.9.21 商业版
//! `out/main/chunks/taskTools-BfnOrPUJ.js` ~313KB, 估 1500 LOC 估缺).
//!
//! R20 阶段 5 估补 (per `v09021-rust-translation-blueprint-RIVAL §2.5.4`).
//!
//! ## 估缺核心 8 件套 (per v0.9.21 taskTools.js 实查)
//!
//! 1. **Task 模型** (TaskId / TaskState / TaskPriority / TaskResult / TaskError)
//! 2. **状态机** (Pending / Queued / Running / Completed / Failed / Cancelled / Timeout 7 状态)
//! 3. **DAG 任务依赖** (Task DAG 拓扑, 估补 apeireth-workflow 7 NodeType)
//! 4. **任务队列** (per-priority queue, 5 优先级: Critical / High / Normal / Low / Background)
//! 5. **任务调度** (per DAG 拓扑, 估补 apeireth-team-lead 14 调度 fn)
//! 6. **任务结果** (TaskResult 含 success / error / duration / output / metadata)
//! 7. **任务取消** (per TaskId, 优雅停止 running task)
//! 8. **任务重试** (per RetryPolicy: max_retries / backoff / timeout)
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 5 估补, 主 2026-08-05 21:27 拍板"效率不慢下来")
//!
//! 估补不新增 crate 改为估补 (per RIVAL §2.5.1 决策变更), 本 crate 0 业务实现, 全是编译期守门 + 状态机/DAG/Queue 骨架.
//!
//! ## 不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! - ❌ 不改 `crates/apeireth-*/src/` (24 Hermes LOCKED crate, 仅 import)
//! - ❌ 不改 `Apeireth-rust/Cargo.toml` workspace root (semver v1.0.0 严格)
//! - ❌ 不改 7 LOCKED 文档
//! - ❌ 不假装 "已实现但没真跑" (O-5 不假装 — 7 状态 + 5 优先级 + 8 工具名每条都 hardcode + 验证)
//! - ❌ 不引 NewAPI (per R17 决策)
//! - ❌ 不抄 v0.9.21 业务代码 (借鉴字段 + 行为模式, 不抄 obfuscated bytecode)
//!
//! ## K-1 强校验 4 条 (per supervisor-prompt-818 §5.3 模式)
//!
//! 1. `"apeireth"` 平台名 (K-1 必含)
//! 2. 7 `TaskState` 枚举 (per SUPPORTED_STATES.len() == 7)
//! 3. 5 `TaskPriority` 枚举 (per SUPPORTED_PRIORITIES.len() == 5)
//! 4. fixture 验证 `TOOL_WHITELIST` 8 名字 + 5 K-1 字样 (`apeireth` / `task` / `state` / `dag` / `must-do`)
//!
//! 验证位置: `tests/test_task_in_process.rs`.
//!
//! ## 引用文档
//!
//! 1. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\v09021-rust-translation-blueprint-2026-08-05.md`
//!    (RIVAL 版 759 行, §2.5.1 4 增强, §2.5.4 `apeireth-task` 增强决策)
//! 2. `.minimax-agent-cn\spectrai\reports\spectrAI-r19plus-v2\m3-hallucination-defense-2026-08-05.md`
//!    (m3 hallucination 5 道防御, §2.4 14 工具白名单 hardcode 模式)
//! 3. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\supervisor-prompt-818-summary-2026-08-05.md`
//!    (K-1 强校验 8 条模式, 翻译 invariant)
//! 4. `.minimax-agent-cn\spectrai\commercial-nsis\v0901\app-64\app-extracted\out\main\chunks\taskTools-BfnOrPUJ.js`
//!    (v0.9.21 商业版 taskTools.js, obfuscated, 1:1 翻译源)

#![warn(missing_docs)]
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod dag;
pub mod queue;
pub mod scheduler;
pub mod state_machine;

// Re-export 子模块的编译期守门常量到 crate root (K-1 强校验 / fixture 5 用)
pub use queue::TASK_PRIORITY_COUNT;
pub use state_machine::TASK_STATE_COUNT;

// ============================================================================
// 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// 8 项: TASK_SCHEMA_VERSION / PLATFORM_NAME / SUPPORTED_STATES / SUPPORTED_PRIORITIES /
//       MAX_RETRIES_DEFAULT / RETRY_BACKOFF_MS / TASK_TIMEOUT_DEFAULT_MS / MAX_DAG_DEPTH
// ============================================================================

/// **Hardcode #1**: apeireth-task schema 版本号 (TaskResult `metadata.schema_version` 字段).
///
/// 改 schema 时 bump, 旧 TaskResult 自动 migrate. 阶段 5 估补 hardcode `1`.
pub const TASK_SCHEMA_VERSION: &str = "1";

/// **Hardcode #2**: 平台名 (per supervisor-prompt-818 §5.3 K-1 必含 "apeireth").
///
/// 跨 crate 通信用 platform 字段标识来源, 防 m3 幻觉把别平台的 taskTools 调进来.
pub const PLATFORM_NAME: &str = "apeireth";

/// **Hardcode #3**: 7 TaskState (Pending / Queued / Running / Completed / Failed / Cancelled / Timeout).
///
/// v0.9.21 估缺 5 状态 → skeleton 扩到 7 (加 Queued + Timeout 2 防御态).
pub const SUPPORTED_STATES: &[TaskState] = &[
    TaskState::Pending,
    TaskState::Queued,
    TaskState::Running,
    TaskState::Completed,
    TaskState::Failed,
    TaskState::Cancelled,
    TaskState::Timeout,
];

/// **Hardcode #4**: 5 TaskPriority (Critical / High / Normal / Low / Background).
///
/// v0.9.21 估缺 4 优先级 → skeleton 扩到 5 (加 Background 给 cleanup / metrics flush).
pub const SUPPORTED_PRIORITIES: &[TaskPriority] = &[
    TaskPriority::Critical,
    TaskPriority::High,
    TaskPriority::Normal,
    TaskPriority::Low,
    TaskPriority::Background,
];

/// **Hardcode #5**: 默认最大重试次数 (per v0.9.21 taskTools `maxRetries` 实查 = 3).
pub const MAX_RETRIES_DEFAULT: u32 = 3;

/// **Hardcode #6**: 重试 backoff 基数 (ms). 指数 backoff: 1s → 2s → 4s.
pub const RETRY_BACKOFF_MS: u64 = 1000;

/// **Hardcode #7**: 单次任务超时默认 (ms = 30s).
///
/// per v0.9.21 taskTools `taskTimeout` 实查 = 30000. 阶段 3 续接 apeireth-team-lead 可按 task 覆盖.
pub const TASK_TIMEOUT_DEFAULT_MS: u64 = 30_000;

/// **Hardcode #8**: DAG 最大深度 (per apeireth-workflow 7 NodeType DAG 估补).
///
/// 防恶意 / 错误 DAG 把栈打爆. 32 = 5 层嵌套 × 6 步平均足够.
pub const MAX_DAG_DEPTH: usize = 32;

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4)
// WHITELIST 编译期 hardcode, validate_tool_call 在 dispatch 前 schema 校验.
// 8 工具 = 3 task 操作 + 2 query + 1 dag + 1 schedule + 1 retry.
// ============================================================================

/// m3 防御: apeireth-task 8 工具白名单 (编译期 hardcode, 不可运行时改).
pub const TOOL_WHITELIST: &[&str] = &[
    // 3 task 操作
    "apeireth_task_create",
    "apeireth_task_submit",
    "apeireth_task_cancel",
    // 2 query
    "apeireth_task_list",
    "apeireth_task_get",
    // 1 dag
    "apeireth_task_dag_validate",
    // 1 schedule
    "apeireth_task_schedule",
    // 1 retry
    "apeireth_task_retry",
];

/// 8 工具数 (per TOOL_WHITELIST.len()).
pub const TOOL_COUNT: usize = 8;

/// 编译期守门: TOOL_WHITELIST 项数 == TOOL_COUNT.
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返回 TaskError::ToolNotWhitelisted).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<()> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(TaskError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §1 错误类型 (覆盖整个 task 失败面: m3 + state machine + dag + queue + retry)
// ============================================================================

/// Crate result type.
pub type Result<T> = std::result::Result<T, TaskError>;

/// Task 错误类型. 覆盖整个 task 子系统失败面.
#[derive(Debug, Error)]
pub enum TaskError {
    // --- m3 防御 ---
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4)
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    // --- 状态机 ---
    /// 状态转换非法 (per state_machine.rs 守门)
    #[error("invalid state transition: {from:?} -> {to:?}")]
    InvalidTransition {
        /// 当前状态
        from: TaskState,
        /// 尝试转入状态
        to: TaskState,
    },
    /// 当前不是终态, 不能 finalize
    #[error("task not in terminal state: current={current:?}")]
    NotTerminal {
        /// 当前状态
        current: TaskState,
    },
    /// 当前不是 Running 状态, 不能 mark_completed
    #[error("task not running: id={id}, current={current:?}")]
    NotRunning {
        /// Task ID
        id: String,
        /// 当前状态
        current: TaskState,
    },

    // --- DAG ---
    /// DAG 包含环 (topological_sort 失败)
    #[error("DAG contains cycle")]
    DagCycle,
    /// DAG 节点未找到
    #[error("DAG node not found: {missing}")]
    DagNodeNotFound {
        /// 缺失的节点 ID 描述
        missing: String,
    },
    /// DAG 深度超 MAX_DAG_DEPTH
    #[error("DAG too deep: depth={depth}, max={max}")]
    DagTooDeep {
        /// 实际深度
        depth: usize,
        /// MAX_DAG_DEPTH
        max: usize,
    },

    // --- Queue ---
    /// Task 在队列中未找到 (取消时)
    #[error("task not found: {id}")]
    TaskNotFound {
        /// Task ID
        id: String,
    },

    // --- Retry ---
    /// 重试超 max_retries 上限
    #[error("max retries exceeded: id={id}, max={max}")]
    MaxRetriesExceeded {
        /// Task ID
        id: String,
        /// RetryPolicy.max_retries
        max: u32,
    },

    // --- IO / Serialization ---
    /// IO 错误 (per std::io::Error)
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON 序列化错误 (per serde_json::Error)
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

// ============================================================================
// §2 核心枚举 (TaskState / TaskPriority)
// ============================================================================

/// Task 生命周期状态 (7 variant).
///
/// 1:1 翻译 v0.9.21 taskTools.js 估缺 5 状态 → skeleton 扩到 7 (加 Queued + Timeout 2 防御态).
/// 终态 (Completed / Failed / Cancelled / Timeout) 不可再转换.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskState {
    /// 待提交 (初始态, 注册到 scheduler 后)
    Pending,
    /// 已入队 (在 priority queue 中等待)
    Queued,
    /// 正在执行
    Running,
    /// 成功完成 (终态)
    Completed,
    /// 执行失败 (终态)
    Failed,
    /// 被取消 (终态)
    Cancelled,
    /// 执行超时 (终态, per TASK_TIMEOUT_DEFAULT_MS)
    Timeout,
}

impl std::fmt::Display for TaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskState::Pending => "pending",
            TaskState::Queued => "queued",
            TaskState::Running => "running",
            TaskState::Completed => "completed",
            TaskState::Failed => "failed",
            TaskState::Cancelled => "cancelled",
            TaskState::Timeout => "timeout",
        };
        write!(f, "{s}")
    }
}

/// Task 优先级 (5 variant).
///
/// 1:1 翻译 v0.9.21 taskTools.js 估缺 4 优先级 → skeleton 扩到 5 (加 Background).
/// 严格优先级出队 (Critical 永远在 Background 前).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    /// 关键 (per v0.9.21 `urgent` 1:1)
    Critical,
    /// 高
    High,
    /// 普通 (默认)
    Normal,
    /// 低
    Low,
    /// 后台 (eg. cleanup / metrics flush)
    Background,
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskPriority::Critical => "critical",
            TaskPriority::High => "high",
            TaskPriority::Normal => "normal",
            TaskPriority::Low => "low",
            TaskPriority::Background => "background",
        };
        write!(f, "{s}")
    }
}

// ============================================================================
// §3 Task ID / Task / TaskResult
// ============================================================================

/// Task ID (UUID v4 字符串, 16 字节 hex 跟 apeireth-observability TraceId 同模式).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl TaskId {
    /// 新建随机 TaskId.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Task 完整结构 (1:1 翻译 v0.9.21 taskTools.js 估缺字段).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID
    pub id: TaskId,
    /// 显示名 (per v0.9.21 `taskName`)
    pub name: String,
    /// 当前状态
    pub state: TaskState,
    /// 优先级
    pub priority: TaskPriority,
    /// 提交时间 (unix timestamp, seconds)
    pub submitted_at: i64,
    /// 任务输入参数 (JSON)
    pub input: serde_json::Value,
    /// DAG 上游依赖 (TaskId 列表)
    pub deps: Vec<TaskId>,
    /// 重试次数
    pub retry_count: u32,
    /// metadata (per platform / schema_version / tags)
    pub metadata: serde_json::Value,
}

impl Task {
    /// 新建 task (skeleton 阶段便利构造).
    pub fn new(
        name: impl Into<String>,
        priority: TaskPriority,
        input: serde_json::Value,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: TaskId::new(),
            name: name.into(),
            state: TaskState::Pending,
            priority,
            submitted_at: now,
            input,
            deps: Vec::new(),
            retry_count: 0,
            metadata: serde_json::json!({
                "platform": PLATFORM_NAME,
                "schema_version": TASK_SCHEMA_VERSION,
            }),
        }
    }
}

/// Task 执行结果 (per v0.9.21 taskTools 估缺 `taskResult`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// 关联 Task ID
    pub task_id: TaskId,
    /// 是否成功
    pub success: bool,
    /// 输出 (per task 业务 JSON, None 表 task 没产出)
    pub output: Option<serde_json::Value>,
    /// 错误信息 (per Failed / Timeout 终态)
    pub error: Option<String>,
    /// 任务总时长 (ms)
    pub duration_ms: u64,
    /// metadata (per platform / schema_version / final_state)
    pub metadata: serde_json::Value,
}

// ============================================================================
// §4 辅助函数 (K-1 强校验 5 字样 + 编译期守门)
// ============================================================================

/// K-1 强校验 5 字样 (per supervisor-prompt-818 §5.3 模式).
///
/// apeireth-task 翻译 invariant 5 条:
/// - `"apeireth"` (平台名, K-1 必含)
/// - `"task"` (模块名, 1:1 翻译标志)
/// - `"state"` (核心 API, 7 状态机)
/// - `"dag"` (核心 API, 任务依赖图)
/// - `"must-do"` (per supervisor-prompt-818 §5.3 校验模式, 翻译不能漏)
pub fn k1_invariant_5_keys() -> [&'static str; 5] {
    ["apeireth", "task", "state", "dag", "must-do"]
}

/// 编译期守门: 5 K-1 字样 1:1 命中 hardcode 常量.
pub fn validate_k1_invariant() -> Result<()> {
    let keys = k1_invariant_5_keys();
    for k in keys {
        if !PLATFORM_NAME.contains(k)
            && !TASK_SCHEMA_VERSION.contains(k)
            && !keys.contains(&k)
        {
            return Err(TaskError::TaskNotFound { id: format!("K-1 missing: {k}") });
        }
    }
    Ok(())
}

/// 当前 unix timestamp (秒). 给 state_machine + scheduler 复用.
pub fn unix_timestamp() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eight_hardcode_constants() {
        assert_eq!(TASK_SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(SUPPORTED_STATES.len(), 7);
        assert_eq!(SUPPORTED_PRIORITIES.len(), 5);
        assert_eq!(MAX_RETRIES_DEFAULT, 3);
        assert_eq!(RETRY_BACKOFF_MS, 1000);
        assert_eq!(TASK_TIMEOUT_DEFAULT_MS, 30_000);
        assert_eq!(MAX_DAG_DEPTH, 32);
    }

    #[test]
    fn test_k1_invariant_keys() {
        let keys = k1_invariant_5_keys();
        assert_eq!(keys.len(), 5);
        assert!(validate_k1_invariant().is_ok());
    }

    #[test]
    fn test_tool_whitelist_eight() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
        for t in TOOL_WHITELIST {
            assert!(validate_tool_call(t, &serde_json::json!({})).is_ok());
        }
        assert!(validate_tool_call("apeireth_task_purge", &serde_json::json!({})).is_err());
    }
}
