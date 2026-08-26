//! # Task 优先级队列
//!
//! 5 优先级: Critical / High / Normal / Low / Background.
//! 1:1 翻译 v0.9.21 `taskTools.js` 估缺 4 优先级 → skeleton 扩到 5 (加 Background).
//!
//! **核心 invariant**:
//! - 严格优先级出队 (Critical 永远在 Background 前)
//! - 同优先级 FIFO (per v0.9.21 taskTools `enqueue` 时序)
//! - 5 优先级 hardcode 编译期守门
//!
//! **设计选择** (per RIVAL §2.5.1):
//! - skeleton 阶段用 5 个 `VecDeque<TaskId>` (per-priority FIFO 队列)
//! - 阶段 3 续接 tokio `mpsc` channel 异步化
//! - 估补 `apeireth-team-lead` 的 `ORCHESTRATOR_TOOL_COUNT=14` 调度优先级

use crate::{TaskError, TaskId, TaskPriority, SUPPORTED_PRIORITIES};

// ============================================================================
// §1 编译期守门: 5 优先级
// ============================================================================

/// 5 优先级数 (per v0.9.21 taskTools.js 估补).
pub const TASK_PRIORITY_COUNT: usize = 5;

/// 编译期守门: SUPPORTED_PRIORITIES.len() == TASK_PRIORITY_COUNT.
const _: () = assert!(SUPPORTED_PRIORITIES.len() == TASK_PRIORITY_COUNT);

// ============================================================================
// §2 PriorityTaskQueue — per-priority FIFO
// ============================================================================

/// 5 优先级 per-priority FIFO 队列. 严格优先级出队, 同优先级 FIFO.
///
/// skeleton 阶段用 5 `VecDeque`, 无锁. 阶段 3 续接 tokio `mpsc`.
#[derive(Debug, Default)]
pub struct PriorityTaskQueue {
    /// Critical 队列 (最高优先级, 估 v0.9.21 `urgent` 1:1)
    pub critical: std::collections::VecDeque<TaskId>,
    /// High 队列
    pub high: std::collections::VecDeque<TaskId>,
    /// Normal 队列 (默认优先级)
    pub normal: std::collections::VecDeque<TaskId>,
    /// Low 队列
    pub low: std::collections::VecDeque<TaskId>,
    /// Background 队列 (最低优先级, eg. cleanup / metrics flush)
    pub background: std::collections::VecDeque<TaskId>,
}

impl PriorityTaskQueue {
    /// 新建空队列.
    pub fn new() -> Self {
        Self::default()
    }

    /// 入队. 按 priority 选择对应子队列.
    pub fn enqueue(&mut self, task_id: TaskId, priority: TaskPriority) {
        let q = self.queue_mut(priority);
        q.push_back(task_id);
    }

    /// 出队. 严格优先级 (Critical → Background), 同优先级 FIFO.
    ///
    /// 返回 `None` 表 5 队列全空.
    pub fn dequeue(&mut self) -> Option<TaskId> {
        for p in [
            TaskPriority::Critical,
            TaskPriority::High,
            TaskPriority::Normal,
            TaskPriority::Low,
            TaskPriority::Background,
        ] {
            let q = self.queue_mut(p);
            if let Some(t) = q.pop_front() {
                return Some(t);
            }
        }
        None
    }

    /// 总任务数.
    pub fn len(&self) -> usize {
        self.critical.len()
            + self.high.len()
            + self.normal.len()
            + self.low.len()
            + self.background.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 取消某 task (per TaskId). 遍历 5 队列移除.
    pub fn cancel(&mut self, task_id: &TaskId) -> Result<(), TaskError> {
        for p in [
            TaskPriority::Critical,
            TaskPriority::High,
            TaskPriority::Normal,
            TaskPriority::Low,
            TaskPriority::Background,
        ] {
            let q = self.queue_mut(p);
            if let Some(pos) = q.iter().position(|x| x == task_id) {
                q.remove(pos);
                return Ok(());
            }
        }
        Err(TaskError::TaskNotFound { id: task_id.to_string() })
    }

    fn queue_mut(&mut self, p: TaskPriority) -> &mut std::collections::VecDeque<TaskId> {
        match p {
            TaskPriority::Critical => &mut self.critical,
            TaskPriority::High => &mut self.high,
            TaskPriority::Normal => &mut self.normal,
            TaskPriority::Low => &mut self.low,
            TaskPriority::Background => &mut self.background,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_five_priorities_hardcoded() {
        assert_eq!(TASK_PRIORITY_COUNT, 5);
        assert_eq!(SUPPORTED_PRIORITIES.len(), 5);
    }

    #[test]
    fn test_strict_priority_order() {
        let mut q = PriorityTaskQueue::new();
        let bg = TaskId::new();
        let cr = TaskId::new();
        let nm = TaskId::new();
        // 反序入队: background 先, critical 后
        q.enqueue(bg.clone(), TaskPriority::Background);
        q.enqueue(nm.clone(), TaskPriority::Normal);
        q.enqueue(cr.clone(), TaskPriority::Critical);
        // 出队顺序: critical → normal → background
        assert_eq!(q.dequeue(), Some(cr));
        assert_eq!(q.dequeue(), Some(nm));
        assert_eq!(q.dequeue(), Some(bg));
        assert!(q.is_empty());
    }

    #[test]
    fn test_same_priority_fifo() {
        let mut q = PriorityTaskQueue::new();
        let a = TaskId::new();
        let b = TaskId::new();
        q.enqueue(a.clone(), TaskPriority::Normal);
        q.enqueue(b.clone(), TaskPriority::Normal);
        assert_eq!(q.dequeue(), Some(a));
        assert_eq!(q.dequeue(), Some(b));
    }

    #[test]
    fn test_cancel_by_id() {
        let mut q = PriorityTaskQueue::new();
        let a = TaskId::new();
        let b = TaskId::new();
        q.enqueue(a.clone(), TaskPriority::Normal);
        q.enqueue(b.clone(), TaskPriority::High);
        q.cancel(&a).unwrap();
        assert_eq!(q.len(), 1);
        assert_eq!(q.dequeue(), Some(b));
    }
}
