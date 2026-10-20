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

#[cfg(test)]
mod tests {
    use super::*;

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
}
