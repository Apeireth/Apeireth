//! 调度器：两条待办队列（渲染前 / 渲染后）+ 去重 + 延迟队列。
//!
//! 设计文档 §4.2：一次 flush 的顺序恒为
//! **① 脏传播 → ② 渲染（DOM 写入）→ ③ effect 执行**。
//! ⇒ effect 永远在 DOM 更新之后跑（effect 可以安全地读 DOM）。
//!
//! 队列语义：
//! - `pre` 队列 = 渲染前任务（`effect_pre`）；
//! - `post` 队列 = 渲染后任务（普通 `effect`）；
//! - 一个 flush 内被多次标脏 ⇒ **合并为一次**（集合去重）；
//! - flush 期间新建的 effect ⇒ 进延迟队列，**下一次** flush 才执行（防重入，
//!   设计文档 §4.2 规则 7）。

use std::collections::{BTreeSet, VecDeque};

use crate::graph::NodeId;

/// 待办调度器：负责合并与排序，不负责执行。
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    pre: VecDeque<NodeId>,
    post: VecDeque<NodeId>,
    deferred: VecDeque<NodeId>,
    pre_seen: BTreeSet<NodeId>,
    post_seen: BTreeSet<NodeId>,
    deferred_seen: BTreeSet<NodeId>,
    scheduled_total: usize,
}

impl Scheduler {
    /// 建一个空调度器。
    pub fn new() -> Self {
        Self::default()
    }

    /// 排入渲染前任务（幂等去重）。
    pub fn schedule_pre(&mut self, node: NodeId) {
        self.scheduled_total += 1;
        if self.pre_seen.insert(node) {
            self.pre.push_back(node);
        }
    }

    /// 排入渲染后任务（幂等去重）。
    pub fn schedule_post(&mut self, node: NodeId) {
        self.scheduled_total += 1;
        if self.post_seen.insert(node) {
            self.post.push_back(node);
        }
    }

    /// 排入延迟任务（flush 期间新建的 effect ⇒ 下一次 flush 才跑）。
    pub fn schedule_deferred(&mut self, node: NodeId) {
        self.scheduled_total += 1;
        if self.deferred_seen.insert(node) {
            self.deferred.push_back(node);
        }
    }

    /// 取走并清空渲染前队列。
    pub fn take_pre(&mut self) -> Vec<NodeId> {
        self.pre_seen.clear();
        self.pre.drain(..).collect()
    }

    /// 取走并清空渲染后队列。
    pub fn take_post(&mut self) -> Vec<NodeId> {
        self.post_seen.clear();
        self.post.drain(..).collect()
    }

    /// 把延迟队列提升为渲染后队列（flush 收尾时调用）。
    pub fn promote_deferred(&mut self) {
        let pending: Vec<NodeId> = self.deferred.drain(..).collect();
        self.deferred_seen.clear();
        for node in pending {
            self.schedule_post(node);
        }
    }

    /// 当前待办总数（pre + post + deferred）。
    pub fn pending(&self) -> usize {
        self.pre.len() + self.post.len() + self.deferred.len()
    }

    /// 累计排入次数（含被去重合并掉的重复排入）。
    pub fn scheduled_total(&self) -> usize {
        self.scheduled_total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_schedules_are_merged() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule_post(NodeId(1));
        scheduler.schedule_post(NodeId(1));
        scheduler.schedule_post(NodeId(2));

        assert_eq!(scheduler.pending(), 2, "同一节点只排一次");
        assert_eq!(
            scheduler.scheduled_total(),
            3,
            "累计排入次数保留（含被合并的）"
        );

        let batch = scheduler.take_post();
        assert_eq!(batch, vec![NodeId(1), NodeId(2)], "按排入顺序执行");
        assert_eq!(scheduler.pending(), 0);
    }

    #[test]
    fn deferred_tasks_promote_to_post_queue() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule_deferred(NodeId(5));
        assert_eq!(scheduler.pending(), 1);

        scheduler.promote_deferred();
        assert_eq!(scheduler.pending(), 1, "提升后仍在待办里");
        assert_eq!(scheduler.take_post(), vec![NodeId(5)]);
        assert_eq!(scheduler.pending(), 0);
    }

    #[test]
    fn pre_and_post_queues_are_independent() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule_pre(NodeId(1));
        scheduler.schedule_post(NodeId(2));

        assert_eq!(scheduler.take_pre(), vec![NodeId(1)]);
        assert_eq!(scheduler.take_post(), vec![NodeId(2)]);
    }
}
