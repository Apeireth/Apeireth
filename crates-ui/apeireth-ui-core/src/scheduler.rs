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
//!
//! ## 阶段必须随任务一起流转（顺序保证的前提）
//!
//! 一个任务的**阶段**（渲染前 / 渲染后）是任务自身的属性 ⇒ 任何一次「重新排入」
//! 都必须**回投到同一个队列**，否则「渲染前任务先于渲染后任务」这条硬保证会被
//! 重排路径悄悄破坏（`effect_pre` 的第二次重跑曾被排进 post 队列 ⇒ pre 跑在
//! post 之后）。延迟队列的提升同理 ⇒ 延迟条目自带 [`Phase`]。

use std::collections::{BTreeSet, VecDeque};

use crate::graph::NodeId;

/// 任务阶段：渲染前 / 渲染后。
///
/// 为什么需要独立类型：任务被**重新排入**时必须回投到**同一个队列**
/// （顺序保证 ⇒ ADR-0021 把「脏传播 → 渲染前任务 → effect」列为硬保证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// 渲染前任务（`effect_pre`）。
    Pre,
    /// 渲染后任务（普通 `effect`）。
    Post,
}

/// 待办调度器：负责合并与排序，不负责执行。
#[derive(Debug, Clone, Default)]
pub struct Scheduler {
    pre: VecDeque<NodeId>,
    post: VecDeque<NodeId>,
    deferred: VecDeque<(NodeId, Phase)>,
    pre_seen: BTreeSet<NodeId>,
    post_seen: BTreeSet<NodeId>,
    deferred_seen: BTreeSet<NodeId>,
    scheduled_total: usize,
    /// 被去重**合并掉**的重复排入次数（可观测：合并率异常高说明排入逻辑有 bug）。
    merged_total: usize,
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
        } else {
            self.merged_total += 1;
        }
    }

    /// 排入渲染后任务（幂等去重）。
    pub fn schedule_post(&mut self, node: NodeId) {
        self.scheduled_total += 1;
        if self.post_seen.insert(node) {
            self.post.push_back(node);
        } else {
            self.merged_total += 1;
        }
    }

    /// 排入延迟任务（flush 期间新建的 effect ⇒ 下一次 flush 才跑）。
    ///
    /// **阶段随任务一起存**：提升时按阶段回投对应队列 ⇒ `effect_pre` 不会因为
    /// 「被延迟过一次」就变成渲染后任务（顺序保证不破）。
    pub fn schedule_deferred(&mut self, node: NodeId, phase: Phase) {
        self.scheduled_total += 1;
        if self.deferred_seen.insert(node) {
            self.deferred.push_back((node, phase));
        } else {
            self.merged_total += 1;
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

    /// 把延迟队列提升为正式队列（flush 收尾时调用）。
    ///
    /// **按各自的阶段回投**：渲染前任务回投 pre 队列、渲染后任务回投 post 队列
    /// ⇒ 「渲染前先于渲染后」的顺序保证在延迟路径上同样成立。
    pub fn promote_deferred(&mut self) {
        let pending: Vec<(NodeId, Phase)> = self.deferred.drain(..).collect();
        self.deferred_seen.clear();
        for (node, phase) in pending {
            match phase {
                Phase::Pre => self.schedule_pre(node),
                Phase::Post => self.schedule_post(node),
            }
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

    /// 累计被去重**合并掉**的重复排入次数。
    pub fn merged_total(&self) -> usize {
        self.merged_total
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
        assert_eq!(scheduler.merged_total(), 1, "被合并掉的重复排入可观测");

        let batch = scheduler.take_post();
        assert_eq!(batch, vec![NodeId(1), NodeId(2)], "按排入顺序执行");
        assert_eq!(scheduler.pending(), 0);
    }

    #[test]
    fn deferred_tasks_promote_to_post_queue() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule_deferred(NodeId(5), Phase::Post);
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

    // ── 判别测试（钉行为不钉字面）──────────────────────────────────────────

    /// 延迟的**渲染前**任务必须回投 pre 队列（顺序保证 ⇒ pre 先于 post）。
    #[test]
    fn deferred_pre_task_is_promoted_back_to_the_pre_queue() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule_deferred(NodeId(7), Phase::Pre);
        scheduler.promote_deferred();

        assert_eq!(
            scheduler.take_pre(),
            vec![NodeId(7)],
            "渲染前任务不得被提升成渲染后任务"
        );
        assert!(scheduler.take_post().is_empty(), "渲染后队列不得被污染");
    }

    /// 同一节点重复排入延迟队列只保留**第一次**的阶段。
    #[test]
    fn deferred_dedupe_keeps_the_first_phase() {
        let mut scheduler = Scheduler::new();
        scheduler.schedule_deferred(NodeId(9), Phase::Pre);
        scheduler.schedule_deferred(NodeId(9), Phase::Post);
        scheduler.promote_deferred();

        assert_eq!(scheduler.take_pre(), vec![NodeId(9)]);
        assert!(scheduler.take_post().is_empty());
    }
}
