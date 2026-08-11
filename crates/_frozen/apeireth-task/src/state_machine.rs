//! # Task 状态机
//!
//! 7 状态: Pending / Queued / Running / Completed / Failed / Cancelled / Timeout.
//! 1:1 翻译 v0.9.21 `taskTools.js` 估缺 5 状态 → skeleton 扩到 7 (加 Queued + Timeout 2 防御态).
//!
//! **状态转换图** (per v0.9.21 taskTools 实查 + 主人 2026-08-05 拍板):
//!
//! ```text
//!                  submit()
//!     Pending  ─────────────→  Queued
//!        │                       │  dispatch()
//!        │ cancel()              ↓
//!        ↓                    Running ──cancel()──→  Cancelled
//!    Cancelled                  │
//!                       ┌──────┼──────┐
//!                       ↓      ↓      ↓
//!                  Completed Failed Timeout
//! ```
//!
//! **核心 invariant**:
//! - 终态 (Completed / Failed / Cancelled / Timeout) 不可再转换
//! - 终态只能从 Running 转入 (除 Cancelled 可从 Pending 直转)
//! - `TaskError::InvalidTransition` 在非法转换时返回

use crate::{TaskError, TaskId, TaskResult, TaskState, SUPPORTED_STATES, TASK_SCHEMA_VERSION};
use std::time::Duration;
use tracing::trace;

// ============================================================================
// §1 编译期守门: 7 状态
// ============================================================================

/// 7 状态数 (per v0.9.21 taskTools.js 估补).
pub const TASK_STATE_COUNT: usize = 7;

/// 编译期守门: SUPPORTED_STATES.len() == TASK_STATE_COUNT.
const _: () = assert!(SUPPORTED_STATES.len() == TASK_STATE_COUNT);

// ============================================================================
// §2 TaskStateMachine — 7 状态机核心
// ============================================================================

/// Task 状态机. 持有 task_id + 当前 state + 时间戳, 提供 transition 守门.
///
/// **skeleton 阶段**: 同步实现, 无锁. 阶段 3 续接 tokio + Mutex 异步化.
#[derive(Debug, Clone)]
pub struct TaskStateMachine {
    /// Task ID (per v0.9.21 `taskId` 1:1)
    pub task_id: TaskId,
    /// 当前状态
    pub state: TaskState,
    /// 提交时间 (unix timestamp, seconds)
    pub submitted_at: i64,
    /// 进入当前状态时间
    pub state_changed_at: i64,
    /// 累计运行时间 (Running 期间累加)
    pub accumulated_runtime_ms: u64,
}

impl TaskStateMachine {
    /// 新建 task, 初始状态 = Pending.
    pub fn new(task_id: TaskId, now: i64) -> Self {
        Self {
            task_id,
            state: TaskState::Pending,
            submitted_at: now,
            state_changed_at: now,
            accumulated_runtime_ms: 0,
        }
    }

    /// 状态转换守门. 合法转换: 见模块顶状态图.
    pub fn transition(&mut self, next: TaskState, now: i64) -> Result<(), TaskError> {
        if !self.can_transition(next) {
            return Err(TaskError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        // 离开 Running 时累加运行时长
        if self.state == TaskState::Running {
            let elapsed = (now - self.state_changed_at).max(0) as u64 * 1000;
            self.accumulated_runtime_ms += elapsed;
        }
        trace!(
            task_id = %self.task_id,
            from = %self.state,
            to = %next,
            "task state transition"
        );
        self.state = next;
        self.state_changed_at = now;
        Ok(())
    }

    /// 检查转换是否合法 (不修改状态).
    pub fn can_transition(&self, next: TaskState) -> bool {
        use TaskState::*;
        match (self.state, next) {
            // 初始 → 调度
            (Pending, Queued) => true,
            // 取消可从 Pending 直转
            (Pending, Cancelled) => true,
            // 队列 → 运行
            (Queued, Running) => true,
            (Queued, Cancelled) => true,
            // 运行 → 终态 4 类
            (Running, Completed) | (Running, Failed) | (Running, Timeout) | (Running, Cancelled) => {
                true
            }
            // 重试: Failed / Timeout → Queued (per RetryPolicy 重入)
            // Cancelled 不允许重试 (user 主动取消, 不是失败)
            (Failed, Queued) | (Timeout, Queued) => true,
            // 终态不可再转 (除上述重试)
            (Completed | Failed | Cancelled | Timeout, _) => false,
            // 其他非法 (eg. Pending → Running 跳过 Queued)
            _ => false,
        }
    }

    /// 是否终态.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled | TaskState::Timeout
        )
    }

    /// 任务总时长 (从 submit 到 now).
    pub fn total_duration(&self, now: i64) -> Duration {
        Duration::from_millis(((now - self.submitted_at).max(0) as u64) * 1000)
    }
}

// ============================================================================
// §3 终态收尾: 构造 TaskResult
// ============================================================================

/// Task 收尾构造 TaskResult (从终态转 Completed/Failed/Timeout 时).
pub fn finalize(state: &TaskStateMachine, result: Option<serde_json::Value>, now: i64) -> Result<TaskResult, TaskError> {
    if !state.is_terminal() {
        return Err(TaskError::NotTerminal {
            current: state.state,
        });
    }
    let success = matches!(state.state, TaskState::Completed);
    Ok(TaskResult {
        task_id: state.task_id.clone(),
        success,
        output: result,
        error: None,
        duration_ms: state.total_duration(now).as_millis() as u64,
        metadata: serde_json::json!({
            "schema_version": TASK_SCHEMA_VERSION,
            "final_state": state.state.to_string(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seven_states_hardcoded() {
        assert_eq!(TASK_STATE_COUNT, 7);
        assert_eq!(SUPPORTED_STATES.len(), 7);
    }

    #[test]
    fn test_state_machine_happy_path() {
        let id = TaskId::new();
        let mut sm = TaskStateMachine::new(id.clone(), 1000);
        sm.transition(TaskState::Queued, 1001).unwrap();
        sm.transition(TaskState::Running, 1002).unwrap();
        sm.transition(TaskState::Completed, 1003).unwrap();
        assert!(sm.is_terminal());
    }

    #[test]
    fn test_terminal_blocks_further_transitions() {
        let id = TaskId::new();
        let mut sm = TaskStateMachine::new(id.clone(), 1000);
        sm.transition(TaskState::Queued, 1001).unwrap();
        sm.transition(TaskState::Running, 1002).unwrap();
        sm.transition(TaskState::Failed, 1003).unwrap();
        // Failed 是终态, 不能再转
        assert!(sm.transition(TaskState::Running, 1004).is_err());
    }

    #[test]
    fn test_pending_to_cancelled_direct() {
        let id = TaskId::new();
        let mut sm = TaskStateMachine::new(id, 1000);
        sm.transition(TaskState::Cancelled, 1001).unwrap();
        assert!(sm.is_terminal());
    }

    #[test]
    fn test_invalid_skip_queue_to_running() {
        let id = TaskId::new();
        let mut sm = TaskStateMachine::new(id, 1000);
        // Pending 不能直接跳 Running (必须经 Queued)
        assert!(sm.transition(TaskState::Running, 1001).is_err());
    }
}
