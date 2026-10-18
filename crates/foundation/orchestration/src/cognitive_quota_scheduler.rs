//! Preemptive Multi-Level Cognitive Time-Slicing Scheduler with Cognitive Quota & PIP.
//!
//! # Mathematical & Architectural Foundations
//!
//! Unlike traditional OS schedulers which schedule CPU time ($\Delta t$), the AI Cognitive Microkernel
//! schedules **Cognitive Computation Quotas**:
//! $$\mathcal{Q} = \langle \Delta T_{\text{token}}, \Delta S_{\text{step}}, \Delta C_{\text{cost}}, \Delta D_{\text{depth}} \rangle$$
//!
//! - **Preemptive Time-Slicing**: Periodically checks step/token consumption. High-priority interrupts
//!   (`SystemEmergency`, `InteractiveUser`) requeue the running task and are dispatched on the next
//!   `schedule_next` call;
//! - **Priority Inheritance Protocol (PIP)**: If a background task holds an exclusive cognitive mutex
//!   (e.g., memory graph write-lock), its effective priority is boosted to prevent priority inversion;
//! - **Deterministic Stack Resumption**: Preempted tasks can be resumed with zero hallucination drift.
//!
//! Pure Safe Rust (`#![forbid(unsafe_code)]`).
//!
//! # 0 装诚实 — 本实现的真实边界 (M1, 2026-09-24 审计修正)
//!
//! 1. **没有"异步抢占"**: 本调度器是**同步的进程内 TCB 注册表 + 优先级队列**。
//!    `schedule_next()` 只把 TCB **克隆**返回给调用方, 由调用方自己执行; 不存在
//!    worker 线程, 不存在跑到一半被掐断的执行体。`preempt_active()` 的真实语义是
//!    "把 running 任务重新入队并标记 `is_preempted`", **不会**中止任何正在运行的
//!    代码 — 抢占是否生效完全取决于调用方是否配合轮询。
//! 2. **没有递归深度**: `CognitiveQuota` 原本带 `max_recursion_depth`, 但调度器既不
//!    跟踪调用深度也没有递归入口 (每次 `submit_task` 独立 TCB)。该字段是**未实施的
//!    空字段**, 按"删除而非留空"原则移除 (见 `CognitiveQuota` doc)。
//! 3. **`held_locks` 不是真锁**: 只是 "resource → task" 字符串记账表, 供 PIP 提升
//!    优先级; 真正的互斥由调用方持有。
//! 4. **有界性**: `tasks` 表上限 [`MAX_TRACKED_TASKS`]; 终态任务需调用方显式
//!    `complete_task()` 回收 (map 删除), 超上限时按**提交序淘汰最早提交**的条目
//!    (严格 FIFO; 正在跑的任务不淘汰, 等它回收)。

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
// cfg(kani) 证明面容器: 见 research/verification/kani/src/lib.rs kani_collections 注。
#[cfg(kani)]
use super::kani_collections::HashMap;
#[cfg(not(kani))]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Five-tier cognitive priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CognitivePriority {
    /// P0: Safety violation, emergency kill-switch, physical disconnect.
    SystemEmergency = 0,
    /// P1: Real-time user foreground interaction.
    InteractiveUser = 1,
    /// P2: Active foreground spawned worker agent.
    ActiveSubAgent = 2,
    /// P3: Background circadian dreaming, memory consolidation.
    BackgroundDreaming = 3,
    /// P4: Idle linting, vacuum, log garbage collection.
    IdleMaintenance = 4,
}

/// Cognitive computation budget & limits.
///
/// 0 装诚实 (M1): v1 设计文档里的 $\Delta D_{\text{depth}}$ 维度**未实施** —
/// 本调度器没有调用深度跟踪, 原 `max_recursion_depth` 字段是永远不被读取的空字段,
/// 已删除。若将来真要限深, 须配套 enter/exit 调用点, 而不是留一个不读的字段。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveQuota {
    pub max_tokens: usize,
    pub max_tool_steps: usize,
    pub max_cost_micros: u64,
    pub consumed_tokens: usize,
    pub consumed_tool_steps: usize,
    pub consumed_cost_micros: u64,
    /// 花费维计量覆盖计数: cost 换算不出来的计量轮数 (无单价行, 绝不编价)。
    ///
    /// 计入此列的轮**不**计入 `consumed_cost_micros`, 因此该列 > 0 时花费累计
    /// 是"部分计量"口径 (如实标注, 不是全量花费)。
    /// `#[serde(default)]` 是落库迁移口径: 旧会话行没有此列, 读回 0。
    #[serde(default)]
    pub unpriced_rounds: usize,
}

impl Default for CognitiveQuota {
    /// An unlimited, empty ledger: every ceiling unreachable, every counter
    /// zero. This is the ledger a session carries before any quota knob has
    /// been bound to it (production wiring: see the canonical runtime's
    /// session quota accounting).
    fn default() -> Self {
        Self::unlimited()
    }
}

impl CognitiveQuota {
    pub fn new(max_tokens: usize, max_tool_steps: usize, max_cost_micros: u64) -> Self {
        Self {
            max_tokens,
            max_tool_steps,
            max_cost_micros,
            consumed_tokens: 0,
            consumed_tool_steps: 0,
            consumed_cost_micros: 0,
            unpriced_rounds: 0,
        }
    }

    /// An unlimited, empty ledger. The ceilings are the type's unreachable
    /// maxima, which makes `is_exhausted` permanently false until limits are
    /// bound and consumption recorded.
    pub fn unlimited() -> Self {
        Self::new(usize::MAX, usize::MAX, u64::MAX)
    }

    /// Records one provider round's usage-metered tokens (saturating).
    ///
    /// Production metering splits across the metered dimensions because they
    /// land at different call sites: tokens and cost accumulate per provider
    /// response, tool steps per dispatched tool call. `consume_step` (below)
    /// keeps the coupled scheduler-facing accounting untouched.
    pub fn consume_tokens(&mut self, tokens: usize) {
        self.consumed_tokens = self.consumed_tokens.saturating_add(tokens);
    }

    /// Records one dispatched tool call (saturating).
    pub fn consume_tool_step(&mut self) {
        self.consumed_tool_steps = self.consumed_tool_steps.saturating_add(1);
    }

    /// Records one priced provider round's converted cost (saturating).
    ///
    /// The amount is the frozen conversion (`usage × unit price`, per-side
    /// ceil over 1 000 tokens) computed at the metering point. A round whose
    /// model carries no unit price is **not** recorded here — it lands in
    /// [`CognitiveQuota::record_unpriced_round`] instead (never invented).
    pub fn consume_cost_micros(&mut self, cost_micros: u64) {
        self.consumed_cost_micros = self.consumed_cost_micros.saturating_add(cost_micros);
    }

    /// Records one provider round whose cost could not be converted (no unit
    /// price row for the model).
    ///
    /// The round still meters tokens/steps normally; only the cost column is
    /// skipped. A non-zero count marks the cost total as "部分计量" (partial).
    pub fn record_unpriced_round(&mut self) {
        self.unpriced_rounds = self.unpriced_rounds.saturating_add(1);
    }

    /// Checks if any budget dimension has been exceeded.
    pub fn is_exhausted(&self) -> bool {
        self.consumed_tokens >= self.max_tokens
            || self.consumed_tool_steps >= self.max_tool_steps
            || self.consumed_cost_micros >= self.max_cost_micros
    }

    /// Records step consumption.
    pub fn consume_step(&mut self, tokens: usize, cost_micros: u64) -> bool {
        // saturating: 消费计数是累加账本, 溢出即 panic 属于自伤 (debug) / 回绕 (release).
        self.consumed_tokens = self.consumed_tokens.saturating_add(tokens);
        self.consumed_tool_steps = self.consumed_tool_steps.saturating_add(1);
        self.consumed_cost_micros = self.consumed_cost_micros.saturating_add(cost_micros);
        !self.is_exhausted()
    }
}

/// Serialized cognitive context frame for deterministic preemption & resumption.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveContextFrame {
    pub task_id: String,
    pub session_id: String,
    pub step_index: usize,
    pub call_stack: Vec<String>,
    pub local_transcript_snapshot: Vec<String>,
    pub world_state_hash: String,
}

/// Asynchronous cognitive interrupt signals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CognitiveInterrupt {
    EmergencyPreempt { reason: String },
    BudgetExhausted { task_id: String },
    VoluntaryYield { task_id: String },
}

/// Task Control Block (TCB) in the cognitive scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveTaskControlBlock {
    pub task_id: String,
    pub session_id: String,
    pub base_priority: CognitivePriority,
    pub effective_priority: CognitivePriority,
    pub quota: CognitiveQuota,
    pub context_frame: CognitiveContextFrame,
    pub is_preempted: bool,
}

/// `tasks` 跟踪表上限 (M1: 防无界增长). 超限淘汰最旧的未入队条目;
/// 稳态有界需调用方配合 `complete_task()` 回收终态任务。
pub const MAX_TRACKED_TASKS: usize = 1_024;

/// Multi-Level Cognitive Quota Scheduler.
#[derive(Debug, Clone)]
pub struct CognitiveQuotaScheduler {
    inner: Arc<Mutex<SchedulerInner>>,
}

#[derive(Debug)]
struct SchedulerInner {
    run_queues: BTreeMap<CognitivePriority, VecDeque<String>>,
    tasks: HashMap<String, CognitiveTaskControlBlock>,
    active_running_task: Option<String>,
    held_locks: HashMap<String, String>, // Resource -> TaskId
    /// 提交序计数 (单调递增, 只增不减; 用于有界淘汰的**严格 FIFO** 判据).
    submit_seq_counter: u64,
    /// `task_id` → 提交序 (与 `tasks` 同生命周期: 提交登记, 回收/淘汰注销).
    submit_order: HashMap<String, u64>,
}

impl Default for CognitiveQuotaScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl CognitiveQuotaScheduler {
    pub fn new() -> Self {
        let mut run_queues = BTreeMap::new();
        run_queues.insert(CognitivePriority::SystemEmergency, VecDeque::new());
        run_queues.insert(CognitivePriority::InteractiveUser, VecDeque::new());
        run_queues.insert(CognitivePriority::ActiveSubAgent, VecDeque::new());
        run_queues.insert(CognitivePriority::BackgroundDreaming, VecDeque::new());
        run_queues.insert(CognitivePriority::IdleMaintenance, VecDeque::new());

        Self {
            inner: Arc::new(Mutex::new(SchedulerInner {
                run_queues,
                tasks: HashMap::new(),
                active_running_task: None,
                held_locks: HashMap::new(),
                submit_seq_counter: 0,
                submit_order: HashMap::new(),
            })),
        }
    }

    /// Submits a new cognitive task to the scheduler.
    ///
    /// M1: 重复 `task_id` **拒绝** — 旧实现无条件 `insert`, 同一 id 二次提交会
    /// 静默覆盖 TCB 并在运行队列里留下两条同 id 记录 (一次调度, 永久重复)。
    pub fn submit_task(&self, tcb: CognitiveTaskControlBlock) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let tid = tcb.task_id.clone();
        if inner.tasks.contains_key(&tid) {
            return Err(format!("duplicate task_id: {tid}"));
        }
        // M1: tasks 表有界 — 超上限淘汰**最早提交**的条目 (严格 FIFO, D7 已清)。
        // 判据用调度器侧的 `submit_seq` (提交序), 不依赖 `task_id` 字典序; 正在跑的
        // 任务不淘汰 (它的执行上下文已交给调用方, 抽走跟踪态会让 `complete_task`
        // 谎报「从未存在」)。`submit_seq` 只增不减, 不随时钟回拨失真。
        while inner.tasks.len() >= MAX_TRACKED_TASKS {
            let victim = inner
                .submit_order
                .iter()
                .filter(|(id, _)| inner.active_running_task.as_ref() != Some(id))
                .min_by_key(|(_, seq)| **seq)
                .map(|(id, _)| id.clone());
            match victim {
                Some(id) => {
                    inner.tasks.remove(&id);
                    inner.submit_order.remove(&id);
                    // 清掉残留队列登记, 不留腐化 id (schedule_next 的 stale-skip
                    // 仍是兜底, 但淘汰时顺手清干净更省事)。
                    for queue in inner.run_queues.values_mut() {
                        queue.retain(|queued| queued != &id);
                    }
                }
                // 表已满且只剩正在跑的那一个 — 不再淘汰 (等它回收)。
                None => break,
            }
        }
        let prio = tcb.effective_priority;
        let seq = inner.submit_seq_counter;
        inner.submit_seq_counter = inner.submit_seq_counter.saturating_add(1);
        inner.tasks.insert(tid.clone(), tcb);
        inner.submit_order.insert(tid.clone(), seq);
        if let Some(queue) = inner.run_queues.get_mut(&prio) {
            queue.push_back(tid);
        }
        Ok(())
    }

    /// Dispatches the next highest-priority task to run.
    ///
    /// 返回的是 TCB 的**克隆**; 真正执行由调用方负责 (见模块 doc §0 装诚实 1)。
    /// 已回收 (`complete_task`) 或被淘汰的残留队列 id 会被跳过 (旧实现直接返 None,
    /// 一个腐化 id 就让整个调度器假死)。
    pub fn schedule_next(&self) -> Option<CognitiveTaskControlBlock> {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());

        loop {
            let mut chosen_task_id = None;
            for queue in inner.run_queues.values_mut() {
                if let Some(task_id) = queue.pop_front() {
                    chosen_task_id = Some(task_id);
                    break;
                }
            }

            let Some(task_id) = chosen_task_id else {
                inner.active_running_task = None;
                return None;
            };

            if let Some(tcb) = inner.tasks.get_mut(&task_id) {
                tcb.is_preempted = false;
                let result = tcb.clone();
                inner.active_running_task = Some(task_id);
                return Some(result);
            }
            // 残留 id (任务已回收/淘汰) → 继续扫描下一个, 不假死.
        }
    }

    /// 终态回收 (M1): 任务完成/失败/取消后从 tasks 表 + 运行队列 + active 槽位删除.
    ///
    /// 旧实现**从不删除** tasks 条目 — 长跑进程的 `tasks` map 随提交过的任务数
    /// 单调增长 (无界内存)。调用方在终态必须调用本方法回收。
    pub fn complete_task(&self, task_id: &str) -> bool {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let existed = inner.tasks.remove(task_id).is_some();
        for queue in inner.run_queues.values_mut() {
            queue.retain(|id| id != task_id);
        }
        if inner.active_running_task.as_deref() == Some(task_id) {
            inner.active_running_task = None;
        }
        // 提交序登记同生命周期注销 (回收后不再参与淘汰序).
        inner.submit_order.remove(task_id);
        // 释放该任务持有的资源记账 (PIP 权重随之失效).
        inner.held_locks.retain(|_, holder| holder != task_id);
        existed
    }

    /// 当前跟踪的任务数 (调试 / 有界性观测).
    pub fn tracked_task_count(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .tasks
            .len()
    }

    /// Signals an asynchronous preemption of the currently running task.
    ///
    /// **0 装诚实 (M1)**: 本方法只做"重新入队 + 标记 `is_preempted`", 不会
    /// 中止任何正在运行的调用方代码 (调度器不持有执行体)。原 doc 宣称的
    /// "asynchronously preempt" 名不副实, 已改为诚实描述。
    pub fn preempt_active(&self, _interrupt: CognitiveInterrupt) -> Result<Option<String>, String> {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(active_id) = inner.active_running_task.take() {
            if let Some(tcb) = inner.tasks.get_mut(&active_id) {
                tcb.is_preempted = true;
                let prio = tcb.effective_priority;
                if let Some(queue) = inner.run_queues.get_mut(&prio) {
                    queue.push_front(active_id.clone());
                }
                return Ok(Some(active_id));
            }
        }
        Ok(None)
    }

    /// Priority Inheritance Protocol (PIP): Boosts the effective priority of lock-holding task.
    pub fn boost_priority_for_lock(
        &self,
        resource_id: &str,
        requesting_priority: CognitivePriority,
    ) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(holder_id) = inner.held_locks.get(resource_id).cloned() {
            let old_prio = if let Some(holder_tcb) = inner.tasks.get_mut(&holder_id) {
                if requesting_priority < holder_tcb.effective_priority {
                    let old = holder_tcb.effective_priority;
                    holder_tcb.effective_priority = requesting_priority;
                    Some(old)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(old) = old_prio {
                if let Some(queue) = inner.run_queues.get_mut(&old) {
                    queue.retain(|id| id != &holder_id);
                }
                if let Some(queue) = inner.run_queues.get_mut(&requesting_priority) {
                    queue.push_front(holder_id);
                }
            }
        }
    }

    /// Registers a resource lock held by a task.
    ///
    /// 0 装诚实 (M1): 这**不是**真锁 — 只是 "resource → task" 字符串记账表,
    /// 供 PIP 提升优先级; 真正的互斥由调用方持有。
    pub fn register_lock(&self, resource_id: &str, task_id: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        inner
            .held_locks
            .insert(resource_id.to_string(), task_id.to_string());
    }

    /// Releases a resource lock and resets effective priority to base priority.
    pub fn release_lock(&self, resource_id: &str) {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(holder_id) = inner.held_locks.remove(resource_id) {
            let (old_prio, base_prio) = if let Some(holder_tcb) = inner.tasks.get_mut(&holder_id) {
                let current_prio = holder_tcb.effective_priority;
                holder_tcb.effective_priority = holder_tcb.base_priority;
                (current_prio, holder_tcb.base_priority)
            } else {
                return;
            };

            if old_prio != base_prio {
                if let Some(queue) = inner.run_queues.get_mut(&old_prio) {
                    queue.retain(|id| id != &holder_id);
                }
                if let Some(queue) = inner.run_queues.get_mut(&base_prio) {
                    queue.push_back(holder_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tcb(task_id: &str, prio: CognitivePriority) -> CognitiveTaskControlBlock {
        CognitiveTaskControlBlock {
            task_id: task_id.into(),
            session_id: "s1".into(),
            base_priority: prio,
            effective_priority: prio,
            quota: CognitiveQuota::new(1000, 10, 1000),
            context_frame: CognitiveContextFrame {
                task_id: task_id.into(),
                session_id: "s1".into(),
                step_index: 0,
                call_stack: vec![],
                local_transcript_snapshot: vec![],
                world_state_hash: "hash".into(),
            },
            is_preempted: false,
        }
    }

    #[test]
    fn test_cognitive_quota_budget_consumption() {
        // M1: 删除了未实施的 max_recursion_depth 参数 (3 参构造).
        let mut quota = CognitiveQuota::new(1000, 5, 5000);
        assert!(!quota.is_exhausted());

        assert!(quota.consume_step(200, 1000));
        assert_eq!(quota.consumed_tool_steps, 1);
        assert_eq!(quota.consumed_tokens, 200);

        // Exhaust step count
        for _ in 0..4 {
            quota.consume_step(100, 500);
        }
        assert!(quota.is_exhausted());
    }

    /// 花费维记账二分 (A-P2): 有价轮进 `consumed_cost_micros` (饱和), 无价轮
    /// 只进 `unpriced_rounds` (绝不编价, 不污染成本列); 旧账本行缺列读回 0。
    #[test]
    fn cost_accounting_splits_priced_and_unpriced_rounds() {
        let mut quota = CognitiveQuota::new(1000, 5, 5000);
        quota.consume_cost_micros(7_000);
        quota.record_unpriced_round();
        quota.consume_cost_micros(2);
        assert_eq!(quota.consumed_cost_micros, 7_002, "有价轮累计");
        assert_eq!(quota.unpriced_rounds, 1, "无价轮只计数, 不编价");

        // 花费列是 u64 累加账本: 到顶饱和停在 u64::MAX, 不回绕不 panic。
        let mut saturated = CognitiveQuota::unlimited();
        saturated.consume_cost_micros(u64::MAX);
        saturated.consume_cost_micros(1);
        assert_eq!(saturated.consumed_cost_micros, u64::MAX, "饱和不回绕");

        // 落库迁移口径: 旧会话行没有 `unpriced_rounds` 列, 读回 0 不是失败。
        let legacy = serde_json::from_str::<CognitiveQuota>(
            r#"{"max_tokens":1,"max_tool_steps":1,"max_cost_micros":1,
                "consumed_tokens":0,"consumed_tool_steps":0,"consumed_cost_micros":0}"#,
        )
        .expect("旧账本行必须能读回");
        assert_eq!(legacy.unpriced_rounds, 0);
    }

    /// M1 回归: 重复 task_id 必须拒绝 (旧实现静默覆盖 + 队列留重).
    #[test]
    fn m1_submit_task_rejects_duplicate_task_id() {
        let scheduler = CognitiveQuotaScheduler::new();
        assert!(scheduler
            .submit_task(tcb("task_a", CognitivePriority::ActiveSubAgent))
            .is_ok());
        let dup = scheduler.submit_task(tcb("task_a", CognitivePriority::IdleMaintenance));
        assert!(dup.is_err(), "重复 task_id 必须 Err, 得到 {dup:?}");
        assert!(dup.unwrap_err().contains("task_a"), "错误信息应含 task_id");

        // 拒绝后调度仍只出一次该任务.
        let first = scheduler.schedule_next().expect("task_a");
        assert_eq!(first.task_id, "task_a");
        assert_eq!(first.effective_priority, CognitivePriority::ActiveSubAgent);
        assert!(scheduler.schedule_next().is_none(), "不得重复出队");
    }

    /// M1 回归: complete_task 终态回收 (tasks map 删除, 队列/active 同步清理).
    #[test]
    fn m1_complete_task_reclaims_state() {
        let scheduler = CognitiveQuotaScheduler::new();
        scheduler
            .submit_task(tcb("task_done", CognitivePriority::ActiveSubAgent))
            .unwrap();
        assert_eq!(scheduler.tracked_task_count(), 1);

        let dispatched = scheduler.schedule_next().expect("dispatch");
        assert_eq!(dispatched.task_id, "task_done");

        assert!(scheduler.complete_task("task_done"), "首次回收返 true");
        assert_eq!(
            scheduler.tracked_task_count(),
            0,
            "终态后 tasks map 必须删除 (旧实现从不删 → 无界增长)"
        );
        assert!(
            !scheduler.complete_task("task_done"),
            "幂等: 二次回收 false"
        );

        // 已回收任务不会重新出队; preempt_active 也返 None.
        assert!(scheduler.schedule_next().is_none());
        assert_eq!(
            scheduler.preempt_active(CognitiveInterrupt::VoluntaryYield {
                task_id: "task_done".into(),
            }),
            Ok(None)
        );
    }

    /// M1 回归: tasks 表有界 (上限 MAX_TRACKED_TASKS, 超限淘汰**最早提交**条目).
    #[test]
    fn m1_tracked_tasks_bounded() {
        let scheduler = CognitiveQuotaScheduler::new();
        for i in 0..MAX_TRACKED_TASKS + 8 {
            scheduler
                .submit_task(tcb(&format!("t{i}"), CognitivePriority::IdleMaintenance))
                .unwrap();
        }
        assert!(
            scheduler.tracked_task_count() <= MAX_TRACKED_TASKS,
            "tasks 表超限: {}",
            scheduler.tracked_task_count()
        );
    }

    /// D7 已清回归: 有界淘汰按**提交序**(严格 FIFO), 不按 `task_id` 字典序。
    ///
    /// 构造「字典序最小 = 最后提交」: 提交序为 `t01024, t01023, ...`, 于是旧实现
    /// (字典序最小) 会淘汰**最后提交**的 `t00001` —— 恰好是反的。新实现必须淘汰
    /// 最先进来的 `t01024`。
    #[test]
    fn m1_eviction_is_strict_fifo_by_submit_order() {
        let scheduler = CognitiveQuotaScheduler::new();
        let first_submitted = format!("t{:05}", MAX_TRACKED_TASKS);
        let last_submitted = "t00001".to_string();
        for i in 0..MAX_TRACKED_TASKS {
            let id = format!("t{:05}", MAX_TRACKED_TASKS - i);
            scheduler
                .submit_task(tcb(&id, CognitivePriority::IdleMaintenance))
                .unwrap();
        }
        // 超上限一次 → 触发一次淘汰。
        scheduler
            .submit_task(tcb("t00000", CognitivePriority::IdleMaintenance))
            .unwrap();
        assert!(
            !scheduler.complete_task(&first_submitted),
            "最早提交者应被淘汰 (得到仍在跟踪表)"
        );
        assert!(
            scheduler.complete_task(&last_submitted),
            "最后提交者不该被淘汰 (得到已消失)"
        );
    }

    /// D7 已清回归: 正在跑的任务不被淘汰 (它的执行上下文已在调用方手里)。
    #[test]
    fn m1_eviction_skips_the_running_task() {
        let scheduler = CognitiveQuotaScheduler::new();
        let first_submitted = format!("t{:05}", MAX_TRACKED_TASKS);
        for i in 0..MAX_TRACKED_TASKS {
            let id = format!("t{:05}", MAX_TRACKED_TASKS - i);
            scheduler
                .submit_task(tcb(&id, CognitivePriority::IdleMaintenance))
                .unwrap();
        }
        // 派发 → `first_submitted` 成为正在跑的任务 (队列已出队)。
        let dispatched = scheduler.schedule_next().expect("应有可派发任务");
        assert_eq!(dispatched.task_id, first_submitted);
        // 超上限 → 淘汰跳到次早提交者, 不抽走正在跑的任务。
        scheduler
            .submit_task(tcb("t00000", CognitivePriority::IdleMaintenance))
            .unwrap();
        assert!(
            scheduler.complete_task(&first_submitted),
            "正在跑的任务不该被淘汰"
        );
    }

    #[test]
    fn test_preemptive_scheduling_priority_order() {
        let scheduler = CognitiveQuotaScheduler::new();

        let tcb_background = tcb("task_dream", CognitivePriority::BackgroundDreaming);
        let tcb_user = tcb("task_user", CognitivePriority::InteractiveUser);

        scheduler.submit_task(tcb_background).unwrap();
        scheduler.submit_task(tcb_user).unwrap();

        // InteractiveUser (P1) should be scheduled before BackgroundDreaming (P3)
        let first = scheduler.schedule_next().unwrap();
        assert_eq!(first.task_id, "task_user");
        assert_eq!(first.effective_priority, CognitivePriority::InteractiveUser);

        // Preempt active task (真实语义: 重新入队 + 标记, 不中止运行 — 见模块 doc)
        let preempted = scheduler
            .preempt_active(CognitiveInterrupt::EmergencyPreempt {
                reason: "User command".into(),
            })
            .unwrap();
        assert_eq!(preempted, Some("task_user".into()));
    }

    #[test]
    fn test_priority_inheritance_protocol() {
        let scheduler = CognitiveQuotaScheduler::new();

        let tcb_bg = tcb("task_bg_lock", CognitivePriority::BackgroundDreaming);

        scheduler.submit_task(tcb_bg).unwrap();
        scheduler.register_lock("memory_graph_lock", "task_bg_lock");

        // High priority user task requests memory_graph_lock
        scheduler.boost_priority_for_lock("memory_graph_lock", CognitivePriority::InteractiveUser);

        // Background task should now be at InteractiveUser priority level
        let scheduled = scheduler.schedule_next().unwrap();
        assert_eq!(scheduled.task_id, "task_bg_lock");
        assert_eq!(
            scheduled.effective_priority,
            CognitivePriority::InteractiveUser
        );

        // Once lock is released, priority resets
        scheduler.release_lock("memory_graph_lock");
    }
}
