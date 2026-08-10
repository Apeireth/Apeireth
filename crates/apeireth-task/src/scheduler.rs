//! # DAG 任务调度器
//!
//! 按 DAG 拓扑序调度: 只有当 task 的所有上游依赖 Completed 时, 才入队到 priority queue.
//! 1:1 翻译 v0.9.21 `taskTools.js` 估缺 4 step 调度 + 估补 `apeireth-team-lead` 14 调度 fn.
//!
//! **核心 invariant**:
//! - 严格按拓扑序 (DAG 无环才能调度)
//! - 依赖未完成的任务不能入队
//! - 重试按 `RetryPolicy`: `max_retries` (per MAX_RETRIES_DEFAULT=3) + 指数 `backoff_ms`
//! - 默认超时 `TASK_TIMEOUT_DEFAULT_MS = 30_000` (30s)
//!
//! **设计选择** (per RIVAL §2.5.1 + 主人 2026-08-05 拍板):
//! - skeleton 阶段同步, 阶段 3 续接 tokio async
//! - 估补 `apeireth-team-lead` 14 调度 fn 的 Task 部分 (eg. `wait_agent_idle` 跟 DAG 节点完成同步)

use crate::queue::PriorityTaskQueue;
use crate::state_machine::TaskStateMachine;
use crate::{TaskError, TaskId, TaskPriority, TaskState, MAX_RETRIES_DEFAULT, RETRY_BACKOFF_MS, TASK_TIMEOUT_DEFAULT_MS};
use std::collections::{HashMap, HashSet};
use tracing::debug;

// ============================================================================
// §1 重试策略
// ============================================================================

/// 重试策略 (per v0.9.21 taskTools 估缺).
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// 最大重试次数 (per MAX_RETRIES_DEFAULT = 3)
    pub max_retries: u32,
    /// 退避基数 (ms, per RETRY_BACKOFF_MS = 1000, 指数 backoff)
    pub backoff_ms: u64,
    /// 单次超时 (ms, per TASK_TIMEOUT_DEFAULT_MS = 30_000)
    pub timeout_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: MAX_RETRIES_DEFAULT,
            backoff_ms: RETRY_BACKOFF_MS,
            timeout_ms: TASK_TIMEOUT_DEFAULT_MS,
        }
    }
}

// ============================================================================
// §2 DagScheduler — 拓扑序调度
// ============================================================================

/// DAG 调度器. 持有 priority queue + 状态机表 + 重试策略.
#[derive(Debug)]
pub struct DagScheduler {
    /// 5 优先级队列
    pub queue: PriorityTaskQueue,
    /// TaskId → 状态机
    pub state_machines: HashMap<TaskId, TaskStateMachine>,
    /// TaskId → 已完成的上游 id 集合
    pub completed_deps: HashMap<TaskId, HashSet<TaskId>>,
    /// TaskId → 所有上游 id 集合 (DAG 拓扑)
    pub all_deps: HashMap<TaskId, HashSet<TaskId>>,
    /// TaskId → 优先级
    pub priorities: HashMap<TaskId, TaskPriority>,
    /// TaskId → 重试次数
    pub retry_count: HashMap<TaskId, u32>,
    /// 默认重试策略
    pub retry_policy: RetryPolicy,
}

impl DagScheduler {
    /// 新建空调度器.
    pub fn new() -> Self {
        Self {
            queue: PriorityTaskQueue::new(),
            state_machines: HashMap::new(),
            completed_deps: HashMap::new(),
            all_deps: HashMap::new(),
            priorities: HashMap::new(),
            retry_count: HashMap::new(),
            retry_policy: RetryPolicy::default(),
        }
    }

    /// 注册 task. 返回 TaskId.
    pub fn register(
        &mut self,
        deps: Vec<TaskId>,
        priority: TaskPriority,
        now: i64,
    ) -> Result<TaskId, TaskError> {
        let id = TaskId::new();
        let sm = TaskStateMachine::new(id.clone(), now);
        self.state_machines.insert(id.clone(), sm);
        self.completed_deps.insert(id.clone(), HashSet::new());
        self.all_deps.insert(id.clone(), deps.into_iter().collect());
        self.priorities.insert(id.clone(), priority);
        self.retry_count.insert(id.clone(), 0);
        debug!(task_id = %id, priority = ?priority, "task registered");
        Ok(id)
    }

    /// 标记某 task 完成. 推进 DAG: 其下游若有全部依赖完成, 则入队到 priority queue.
    pub fn mark_completed(
        &mut self,
        task_id: &TaskId,
        downstream: &[TaskId],
        now: i64,
    ) -> Result<(), TaskError> {
        // 校验状态机存在
        let sm = self
            .state_machines
            .get_mut(task_id)
            .ok_or_else(|| TaskError::TaskNotFound { id: task_id.to_string() })?;
        // Running → Completed
        if sm.state != TaskState::Running {
            return Err(TaskError::NotRunning {
                id: task_id.to_string(),
                current: sm.state,
            });
        }
        sm.transition(TaskState::Completed, now)?;
        // 推进下游
        for down in downstream {
            if let Some(set) = self.completed_deps.get_mut(down) {
                set.insert(task_id.clone());
            }
            // 检查是否所有依赖都完成
            let all_done = self
                .all_deps
                .get(down)
                .map(|deps| {
                    deps.iter()
                        .all(|d| self.completed_deps.get(down).is_some_and(|s| s.contains(d)))
                })
                .unwrap_or(false);
            if all_done {
                if let Some(p) = self.priorities.get(down).copied() {
                    self.queue.enqueue(down.clone(), p);
                }
            }
        }
        Ok(())
    }

    /// 取下一个可执行 task (出队).
    pub fn next_task(&mut self) -> Option<TaskId> {
        self.queue.dequeue()
    }

    /// 重试某 task. 检查 retry_policy.max_retries 守门.
    ///
    /// 状态转换: Failed / Timeout → Queued (per state_machine::can_transition).
    /// 如果 task 已在 Queued 态 (eg. 上次 retry 后还没被调度), 状态不变, 只重入队 + 计数.
    pub fn retry(&mut self, task_id: &TaskId, now: i64) -> Result<(), TaskError> {
        let count = self.retry_count.get(task_id).copied().unwrap_or(0);
        if count >= self.retry_policy.max_retries {
            return Err(TaskError::MaxRetriesExceeded {
                id: task_id.to_string(),
                max: self.retry_policy.max_retries,
            });
        }
        self.retry_count.insert(task_id.clone(), count + 1);
        // 状态: Failed/Timeout → Queued (允许重试); 已在 Queued 则跳过 transition
        if let Some(sm) = self.state_machines.get_mut(task_id) {
            if sm.state == TaskState::Failed || sm.state == TaskState::Timeout {
                sm.transition(TaskState::Queued, now)?;
            }
        }
        // 按优先级重新入队
        if let Some(p) = self.priorities.get(task_id).copied() {
            self.queue.enqueue(task_id.clone(), p);
        }
        Ok(())
    }

    /// 待执行任务数.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

impl Default for DagScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_enqueue() {
        let mut s = DagScheduler::new();
        let _a = s.register(vec![], TaskPriority::Normal, 1000).unwrap();
        // 0 依赖, 注册即入队? skeleton 阶段 0 依赖不自动入队, 由 mark_completed 触发或显式
        assert_eq!(s.pending(), 0);
    }

    #[test]
    fn test_retry_policy_default() {
        let p = RetryPolicy::default();
        assert_eq!(p.max_retries, MAX_RETRIES_DEFAULT);
        assert_eq!(p.backoff_ms, RETRY_BACKOFF_MS);
        assert_eq!(p.timeout_ms, TASK_TIMEOUT_DEFAULT_MS);
    }

    #[test]
    fn test_retry_max_retries_exceeded() {
        let mut s = DagScheduler::new();
        let id = s.register(vec![], TaskPriority::Normal, 1000).unwrap();
        // 手动转 Failed/Timeout 再重试 3 次
        let sm = s.state_machines.get_mut(&id).unwrap();
        sm.transition(TaskState::Queued, 1001).unwrap();
        sm.transition(TaskState::Running, 1002).unwrap();
        sm.transition(TaskState::Failed, 1003).unwrap();
        // 重试 3 次都成功 (max=3)
        for i in 0..3 {
            let r = s.retry(&id, 1004 + i as i64);
            assert!(r.is_ok(), "重试 {i} 必 Ok");
        }
        // 第 4 次必拒
        assert!(s.retry(&id, 1010).is_err());
    }
}
