//! AI 自驱心跳调度器 (HeartbeatScheduler).
//!
//! 统一管理智能体的自主唤醒、环境定时感知、异步后台任务轮询与用户交互事件，
//! 支持 5 大触发源、5 级优先级抢占式二叉堆队列与“心流锁”保护机制.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// 心跳事件触发源类型.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatTriggerSource {
    /// 定时器时钟滴答 (Cron / Interval)
    Timer,
    /// 外部环境感知事件 (屏幕/窗口变化/文件变动)
    EnvironmentEvent,
    /// 内部子 Agent 或器官协同请求
    InternalAgent,
    /// 用户主动开口或交互输入 (最高优先级)
    UserInteraction,
    /// 异步后台任务完成回调
    AsyncTaskCallback,
}

/// 心跳任务项 (实现优先级反向排序以支持 BinaryHeap 作为最大堆).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatTask {
    pub id: String,
    pub source: HeartbeatTriggerSource,
    /// 优先级 (0 ~ 255, 数值越高优先级越高)
    pub priority: u8,
    pub payload: String,
    pub scheduled_at_ms: u64,
}

impl Ord for HeartbeatTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.scheduled_at_ms.cmp(&self.scheduled_at_ms))
    }
}

impl PartialOrd for HeartbeatTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// 心流锁状态 (FlowLock).
///
/// 当智能体处于深度思考或关键连贯任务时，心流锁会阻止低优先级的闲散事件打断执行.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FlowLock {
    pub is_locked: bool,
    pub minimum_preemption_priority: u8,
    pub locked_reason: String,
}

/// 心跳调度器.
#[derive(Debug, Clone, Default)]
pub struct HeartbeatScheduler {
    task_queue: BinaryHeap<HeartbeatTask>,
    flow_lock: FlowLock,
}

impl HeartbeatScheduler {
    pub fn new() -> Self {
        Self {
            task_queue: BinaryHeap::new(),
            flow_lock: FlowLock::default(),
        }
    }

    /// 启用心流锁，保护当前任务不被低于 `min_priority` 的低优先级事件打断.
    pub fn acquire_flow_lock(&mut self, min_priority: u8, reason: &str) {
        self.flow_lock = FlowLock {
            is_locked: true,
            minimum_preemption_priority: min_priority,
            locked_reason: reason.to_string(),
        };
    }

    /// 释放心流锁.
    pub fn release_flow_lock(&mut self) {
        self.flow_lock = FlowLock::default();
    }

    /// 调度一个新心跳任务入队.
    pub fn schedule_task(&mut self, task: HeartbeatTask) {
        self.task_queue.push(task);
    }

    /// 弹出下一个应执行的高优先级任务 (受心流锁与时间戳约束).
    pub fn poll_next_task(&mut self, now_ms: u64) -> Option<HeartbeatTask> {
        if let Some(top) = self.task_queue.peek() {
            // 尚未到达预定执行时间
            if top.scheduled_at_ms > now_ms {
                return None;
            }

            // 若心流锁已锁定且任务优先级不足以抢占，则暂不弹出
            if self.flow_lock.is_locked && top.priority < self.flow_lock.minimum_preemption_priority
            {
                return None;
            }
        }

        self.task_queue.pop()
    }

    /// 当前队列待处理任务数.
    pub fn pending_count(&self) -> usize {
        self.task_queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_priority_preemption_and_flow_lock() {
        let mut scheduler = HeartbeatScheduler::new();

        let low_task = HeartbeatTask {
            id: "cron_1".to_string(),
            source: HeartbeatTriggerSource::Timer,
            priority: 10,
            payload: "检查天气".to_string(),
            scheduled_at_ms: 1000,
        };

        let high_task = HeartbeatTask {
            id: "user_1".to_string(),
            source: HeartbeatTriggerSource::UserInteraction,
            priority: 100,
            payload: "主人叫我".to_string(),
            scheduled_at_ms: 1000,
        };

        scheduler.schedule_task(low_task);
        scheduler.schedule_task(high_task);

        // 1. 高优先级任务优先被弹出
        let first = scheduler.poll_next_task(1000).unwrap();
        assert_eq!(first.id, "user_1");

        // 2. 模拟进入深度思考并获取心流锁 (要求优先级 >= 50 才能打断)
        scheduler.acquire_flow_lock(50, "正在进行代码重构思考");

        // 此时队列中只剩 priority=10 的低优先级任务，心流锁生效，返回 None
        assert!(scheduler.poll_next_task(1000).is_none());
        assert_eq!(scheduler.pending_count(), 1);

        // 3. 释放心流锁后，低优先级任务正常弹出
        scheduler.release_flow_lock();
        let second = scheduler.poll_next_task(1000).unwrap();
        assert_eq!(second.id, "cron_1");
    }
}
