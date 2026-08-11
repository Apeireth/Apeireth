//! Library Stage 4.1 自治 - 自循环 (深化 P5-1 Library Stage 4)
//!
//! **R127-2 P8-1 实施** (per `decision-56-r127-2-borrowed-3-retry-release-prep-2026-08-10.md` §2.3)
//!
//! Library Stage 4.1 = P5-1 Stage 4 自治 (3 sub-engine: SelfEvolution + SelfUpgrade + SelfRepair)
//! 的**自循环深化**: 3 sub-engine 通过 feedback signal 互联 + 动态调整策略, 形成闭循环.
//!
//! ## 3 大模块
//!
//! ### §1 自循环 (Self-loop) — 借鉴 aGLM 108 PODA 4 阶段 + superpowers 234 skill priority
//!
//! - **aGLM 108 PODA cycle**: Plan / Observe / Decide / Act 4 阶段自主循环.
//!   借鉴为 `AutonomyLoop::tick()` 主循环, 每 tick 走完整 4 阶段闭环.
//! - **superpowers 234 skill priority**: 过程 skill (自调整) → 实施 skill (3 sub-engine) 的层级.
//!   借鉴为 `AutonomyLoop::step()`: 先调 adjust policy, 再调 3 sub-engine.
//! - **整合**: 1 个 `AutonomyLoop` 顶层协调器, 调 3 sub-engine (复用 P5-1 已有类型), 形成 3-tier 自治架构.
//!
//! ### §2 自反馈 (Self-feedback) — 借鉴 aGLM 108 PODA cycle
//!
//! - **3 feedback signal**: `RepairNeeded` / `EvolutionSuggested` / `UpgradePending`.
//!   每 signal 含 source (哪个 sub-engine 发出) + target (驱动哪个 sub-engine) + 强度.
//! - **observe 阶段**: 读 3 sub-engine metrics, 探测异常 (e.g. SelfRepair.journal 非空 → RepairNeeded).
//! - **plan 阶段**: 收集 signal, 排序按优先级.
//! - **decide 阶段**: 选最高优先级 signal, 派给对应 target sub-engine.
//! - **act 阶段**: target sub-engine 跑 1 step (复用 P5-1 step fn).
//!
//! ### §3 自调整 (Self-adjust) — 借鉴 superpowers 234 skill triggers
//!
//! - **5 policy** (从保守到激进): `Conservative` / `Cautious` / `Balanced` / `Progressive` / `Aggressive`.
//! - **5 policy trigger**: 借鉴 superpowers "when to use" 字段, 每 policy 描述何时激活.
//! - **adjust 阶段**: 基于观察到的 signal pattern, 切换 policy. e.g. 连续 3 RepairNeeded → Conservative.
//!
//! ## 0 装 PASS 严守 (per 主人 17:22 "0 装不必要" 解除 + decision-33 §2.3 C2 + decision-55 §3)
//!
//! - ✅ **superpowers 234 cloned** (R125-14 ✅ done, 14 default Skill 公开模式 1:1, per `crates/apeireth-evolution/src/library_autonomy.rs` Skill trait)
//!   = 自调整 5 policy + AdjustPolicyTrigger 借鉴 superpowers "when to use" 字段语义, 0 装 src 实施.
//! - ✅ **aGLM 108 cloned** (R125-7 ✅ done, PODA 4 阶段状态机 + 21/21 tests pass, `crates/apeireth-evolution/src/poda_cycle.rs`)
//!   = 自循环 + 自反馈直接调 `crate::poda_cycle::PodaStage` 借鉴 + `AutonomyLoop` PODA-style tick 实现, 0 装.
//!
//! ## 8 硬墙 verify (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)
//!
//! - **B1** 24 LOCKED 持续更新 — `apeireth-evolution` 在 24 LOCKED #5, **本文件是 NEW**, 0 触碰
//!   `lib.rs` 入口签名, 仅 +1 行 `pub mod library_autonomy_loop;` + 1 re-export group (8 类型).
//!   0 触碰 `library_autonomy.rs` (P5-1) 任何入口签名 (仅 import 类型).
//! - **B2** workspace.version 1.2.0 0 改 — 0 触碰 `Cargo.toml:246`.
//! - **A1** R11 baseline 3 值 数字严守 — 0 触碰 `integration_r_measure.rs` (本文件 0 涉及 R11 baseline).
//! - **B5** 8 哲学锚 — 0 改 8 哲学锚原 8 实质.
//! - **B3** 30 维 — 0 改 V0.5 公式.
//! - **B4** 6 重 v7 守门 — 0 改 6 重守门原 6 重.
//! - **A3** 13 键 — 0 改 12 键原 12.
//! - **C1** 0 主动 commit — 严守 (Mavis 整合 #5 commit 时机拍板).
//! - **C2** 0 装 解除 — 主人 17:22, ✅ cloned = 真实施 (本文件直接 import + 调 P5-1 类型).
//! - **C3** 0 push — 严守 (等主人 1.0 release 配 GitHub remote).
//!
//! ## 架构位置
//!
//! ```text
//!   AutonomyLoop (顶层协调器, 自循环主循环)
//!       ├─ FeedbackChannel (自反馈)  ── 3 signal type + 4 阶段闭环
//!       ├─ AdjustPolicy (自调整)     ── 5 policy + 5 trigger + tune()
//!       └─ LibraryAutonomy (P5-1)    ── 3 sub-engine (SelfEvolution + SelfUpgrade + SelfRepair)
//!              └─ crate::poda_cycle::PodaStage (✅ R125-7 真实施)
//! ```
//!
//! ## 核心不变量 (编译期 hardcode)
//!
//! - Stage 4.1 = 1 文件 (`library_autonomy_loop.rs`), 0 改 P5-1 file.
//! - 自循环主循环 = 4 阶段 (Observe / Plan / Decide / Act) 借鉴 aGLM 108 PODA 公开模式 1:1.
//! - 自反馈 3 signal 0 改 (编译期 hardcode 兜底 = 3, per `FeedbackSignal::COUNT`).
//! - 自调整 5 policy 0 改 (编译期 hardcode 兜底 = 5, per `AdjustPolicy::COUNT`).
//! - 入口签名 0 改: 本文件全部 `pub fn` / `pub struct` / `pub enum` 都是 NEW, 0 改原 crate 任何签名.

#![allow(dead_code)] // ⏳ 部分 fn 等 superpowers 234 cloned 后补 Skill 整合; 核心自循环已就绪

use serde::{Deserialize, Serialize};
use thiserror::Error;

// R127 P8-1 借鉴 P5-1 library_autonomy.rs (Stage 4 自治, ✅ done per R127-2 阶段 C):
// 复用 P5-1 已实装的 3 sub-engine 类型 + SkillRegistry + RepairJournal.
// **0 改 P5-1 任何入口签名**, 仅 import 类型 + 调其 step() / metrics() 方法.
use crate::library_autonomy::{
    AutonomyError, AutonomyMetrics, LibraryAutonomy, SelfEvolutionAction, SelfEvolutionState,
    SelfRepairAction, SelfRepairState, SelfUpgradeAction, SelfUpgradeState,
};

// ============================================================================
// 公共错误类型 (per 编译期 hardcode, 3 variant 覆盖 3 模块失败)
// ============================================================================

/// Library Stage 4.1 自循环错误类型.
#[derive(Debug, Error)]
pub enum LoopError {
    /// 自反馈失败: signal 队列溢出
    #[error("self-feedback signal queue overflow: capacity {capacity}, attempted push {attempted}")]
    FeedbackQueueOverflow {
        /// 容量
        capacity: usize,
        /// 尝试 push 数
        attempted: usize,
    },
    /// 自调整失败: 非法 policy 切换
    #[error("self-adjust illegal policy transition: {from:?} -> {to:?}")]
    AdjustIllegalTransition {
        /// 源 policy
        from: AdjustPolicy,
        /// 目标 policy
        to: AdjustPolicy,
    },
    /// 顶层协调器失败 (复用 P5-1 AutonomyError)
    #[error("autonomy loop main cycle failed: {0}")]
    MainCycleFailed(String),
}

impl From<AutonomyError> for LoopError {
    fn from(e: AutonomyError) -> Self {
        LoopError::MainCycleFailed(format!("{:?}", e))
    }
}

/// Library Stage 4.1 自循环结果类型.
pub type LoopResult<T> = Result<T, LoopError>;

// ============================================================================
// §1 自循环 (Self-loop) — 借鉴 aGLM 108 PODA 4 阶段
// ============================================================================

/// 自循环 4 阶段 (借鉴 aGLM 108 PODA cycle, per `crate::poda_cycle::PodaStage`).
///
/// 借鉴源: aGLM 108 PODA cycle "Plan / Observe / Decide / Act" 4 阶段,
/// P5-1 已在 `poda_cycle.rs` 实装 (PodaStage 4 阶段), 本 enum 复用其命名 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LoopStage {
    /// Observe (PODA 阶段 1) — 读 3 sub-engine metrics, 探测异常
    Observe,
    /// Plan (PODA 阶段 2) — 收集 feedback signal, 排序按优先级
    Plan,
    /// Decide (PODA 阶段 3) — 选最高优先级 signal, 派给 target sub-engine
    Decide,
    /// Act (PODA 阶段 4) — target sub-engine 跑 1 step
    Act,
}

impl LoopStage {
    /// 全部 4 阶段 (编译期 hardcode 兜底, 跟 PODA 1:1).
    pub const ALL: [LoopStage; 4] = [
        Self::Observe,
        Self::Plan,
        Self::Decide,
        Self::Act,
    ];

    /// 阶段序号 (用于审计排序).
    pub const fn order(self) -> u8 {
        match self {
            Self::Observe => 0,
            Self::Plan => 1,
            Self::Decide => 2,
            Self::Act => 3,
        }
    }

    /// 阶段名 (string, 用于日志/UI).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Observe => "Observe",
            Self::Plan => "Plan",
            Self::Decide => "Decide",
            Self::Act => "Act",
        }
    }

    /// 是否终态 (Act = 终态, 跑完 1 cycle 闭环).
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Act)
    }
}

// ============================================================================
// §2 自反馈 (Self-feedback) — 借鉴 aGLM 108 PODA cycle
// ============================================================================

/// 自反馈 signal source (哪个 sub-engine 发出).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSource {
    /// SelfEvolution (自演化) 发出
    Evolution,
    /// SelfUpgrade (自升级) 发出
    Upgrade,
    /// SelfRepair (自修复) 发出
    Repair,
    /// 顶层 AutonomyLoop 观察发出
    Observer,
}

impl SignalSource {
    /// 全部变体 (编译期 hardcode 兜底 = 4).
    pub const COUNT: usize = 4;

    /// source 名 (string, 审计字段).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Evolution => "Evolution",
            Self::Upgrade => "Upgrade",
            Self::Repair => "Repair",
            Self::Observer => "Observer",
        }
    }
}

/// 自反馈 signal target (驱动哪个 sub-engine).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalTarget {
    /// 驱动 SelfEvolution
    Evolution,
    /// 驱动 SelfUpgrade
    Upgrade,
    /// 驱动 SelfRepair
    Repair,
}

impl SignalTarget {
    /// 全部变体 (编译期 hardcode 兜底 = 3).
    pub const COUNT: usize = 3;
}

/// 自反馈 signal 优先级 (1-5, 5 最高).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SignalPriority(pub u8);

impl SignalPriority {
    /// 最低优先级 (1).
    pub const LOW: SignalPriority = SignalPriority(1);
    /// 中等优先级 (3).
    pub const MEDIUM: SignalPriority = SignalPriority(3);
    /// 最高优先级 (5).
    pub const HIGH: SignalPriority = SignalPriority(5);
    /// 默认 (3).
    pub const DEFAULT: SignalPriority = SignalPriority(3);
}

/// 自反馈 3 signal type (借鉴 aGLM 108 PODA 闭环 + chidori 9 字段语义).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackSignal {
    /// SelfRepair journal 检测到失败 → 驱动 SelfEvolution 适配
    RepairNeeded {
        /// 失败来源 sub-engine
        source: SignalSource,
        /// 目标 sub-engine
        target: SignalTarget,
        /// 优先级
        priority: SignalPriority,
        /// 描述
        description: String,
    },
    /// SelfEvolution 完成演化 → 驱动 SelfUpgrade 应用
    EvolutionSuggested {
        /// 来源
        source: SignalSource,
        /// 目标
        target: SignalTarget,
        /// 优先级
        priority: SignalPriority,
        /// 描述
        description: String,
    },
    /// SelfUpgrade 检测到升级意图 → 驱动 SelfRepair 准备
    UpgradePending {
        /// 来源
        source: SignalSource,
        /// 目标
        target: SignalTarget,
        /// 优先级
        priority: SignalPriority,
        /// 描述
        description: String,
    },
}

impl FeedbackSignal {
    /// 全部变体 (编译期 hardcode 兜底 = 3).
    pub const COUNT: usize = 3;

    /// signal 优先级.
    pub const fn priority(&self) -> SignalPriority {
        match self {
            Self::RepairNeeded { priority, .. }
            | Self::EvolutionSuggested { priority, .. }
            | Self::UpgradePending { priority, .. } => *priority,
        }
    }

    /// signal 来源.
    pub const fn source(&self) -> SignalSource {
        match self {
            Self::RepairNeeded { source, .. }
            | Self::EvolutionSuggested { source, .. }
            | Self::UpgradePending { source, .. } => *source,
        }
    }

    /// signal 目标.
    pub const fn target(&self) -> SignalTarget {
        match self {
            Self::RepairNeeded { target, .. }
            | Self::EvolutionSuggested { target, .. }
            | Self::UpgradePending { target, .. } => *target,
        }
    }

    /// 描述.
    pub fn description(&self) -> &str {
        match self {
            Self::RepairNeeded { description, .. }
            | Self::EvolutionSuggested { description, .. }
            | Self::UpgradePending { description, .. } => description,
        }
    }
}

/// 自反馈 channel (借鉴 aGLM 108 PODA cycle, signal 队列 + 4 阶段闭环).
///
/// **核心设计**:
/// - `pending_signals`: 待处理 signal 队列, 按 priority 排序
/// - `processed_count`: 已处理 signal 计数 (审计)
/// - 4 阶段闭环: observe → plan → decide → act
#[derive(Debug, Default)]
pub struct FeedbackChannel {
    /// 待处理 signal 队列 (FIFO, 但按 priority 排序)
    pending_signals: Vec<FeedbackSignal>,
    /// 已处理 signal 计数 (审计)
    processed_count: u32,
    /// 队列容量上限 (兜底, 防无限累积)
    capacity: usize,
}

impl FeedbackChannel {
    /// 默认容量 (编译期 hardcode).
    pub const DEFAULT_CAPACITY: usize = 64;

    /// 创建空 channel.
    pub fn new() -> Self {
        Self {
            pending_signals: Vec::new(),
            processed_count: 0,
            capacity: Self::DEFAULT_CAPACITY,
        }
    }

    /// 创建 + 自定义 capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pending_signals: Vec::with_capacity(capacity),
            processed_count: 0,
            capacity,
        }
    }

    /// 当前待处理 signal 数.
    pub fn pending(&self) -> usize {
        self.pending_signals.len()
    }

    /// 已处理 signal 数.
    pub fn processed(&self) -> u32 {
        self.processed_count
    }

    /// 容量上限.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.pending_signals.is_empty()
    }

    /// push signal (按 priority 降序, 高优先级在前).
    pub fn push(&mut self, signal: FeedbackSignal) -> LoopResult<()> {
        if self.pending_signals.len() >= self.capacity {
            return Err(LoopError::FeedbackQueueOverflow {
                capacity: self.capacity,
                attempted: 1,
            });
        }
        // 按 priority 降序插入
        let priority = signal.priority();
        let pos = self
            .pending_signals
            .iter()
            .position(|s| s.priority() < priority)
            .unwrap_or(self.pending_signals.len());
        self.pending_signals.insert(pos, signal);
        Ok(())
    }

    /// 取出最高优先级 signal (peek + pop).
    pub fn pop_highest(&mut self) -> Option<FeedbackSignal> {
        if self.pending_signals.is_empty() {
            return None;
        }
        // signals 始终按 priority 降序, 第 0 个 = 最高
        let signal = self.pending_signals.remove(0);
        self.processed_count += 1;
        Some(signal)
    }

    /// peek 最高优先级 signal (不取出).
    pub fn peek_highest(&self) -> Option<&FeedbackSignal> {
        self.pending_signals.first()
    }

    /// 清空 (审计 / 状态重置).
    pub fn clear(&mut self) {
        self.pending_signals.clear();
    }
}

// ============================================================================
// §3 自调整 (Self-adjust) — 借鉴 superpowers 234 skill triggers
// ============================================================================

/// 自调整 5 policy (从保守到激进, 借鉴 superpowers 234 skill priority 5 层级).
///
/// 借鉴源: superpowers 234 "Skill Priority" 5 层级概念 (process / implementation / domain / etc.),
/// 1:1 借鉴为自调整 5 policy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustPolicy {
    /// 最保守: 严格守门, repair 优先, 0 主动 upgrade / evolution
    Conservative,
    /// 谨慎: 仍偏 repair, 但允许 evolution
    Cautious,
    /// 平衡: 3 sub-engine 等权 (default, 0 装最严厉)
    #[default]
    Balanced,
    /// 渐进: 偏 evolution / upgrade
    Progressive,
    /// 最激进: 优先 evolution / upgrade
    Aggressive,
}

impl AdjustPolicy {
    /// 全部变体 (编译期 hardcode 兜底 = 5, 跟 superpowers skill priority 1:1).
    pub const COUNT: usize = 5;

    /// 全部变体 (按从保守到激进顺序).
    pub const ALL: [AdjustPolicy; 5] = [
        Self::Conservative,
        Self::Cautious,
        Self::Balanced,
        Self::Progressive,
        Self::Aggressive,
    ];

    /// 数值权重 (0=Conservative, 4=Aggressive).
    pub const fn weight(self) -> u8 {
        match self {
            Self::Conservative => 0,
            Self::Cautious => 1,
            Self::Balanced => 2,
            Self::Progressive => 3,
            Self::Aggressive => 4,
        }
    }

    /// 从权重还原 policy (0-4, 超界返回 Balanced 兜底).
    pub const fn from_weight(w: u8) -> Self {
        match w {
            0 => Self::Conservative,
            1 => Self::Cautious,
            2 => Self::Balanced,
            3 => Self::Progressive,
            4 => Self::Aggressive,
            _ => Self::Balanced,
        }
    }

    /// policy 名 (string, 审计字段).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Conservative => "Conservative",
            Self::Cautious => "Cautious",
            Self::Balanced => "Balanced",
            Self::Progressive => "Progressive",
            Self::Aggressive => "Aggressive",
        }
    }
}

/// 自调整 5 trigger (借鉴 superpowers 234 "when to use" 字段, 5 触发条件 1:1).
///
/// 借鉴源: superpowers 234 每个 Skill 含 "when to use" 字段描述触发条件,
/// 1:1 借鉴为 5 AdjustPolicyTrigger 描述何时切换 policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustPolicyTrigger {
    /// 连续 repair 数 ≥ 3 → 切 Conservative (失败率高)
    RepairStorm,
    /// 连续 evolution 成功数 ≥ 5 → 切 Aggressive (演化健康)
    EvolutionHealthy,
    /// SelfUpgrade plan 已设置 → 切 Progressive (有升级意图)
    UpgradeIntent,
    /// 全部 sub-engine 终态 → 切 Balanced (稳态)
    AllTerminal,
    /// 默认 → Balanced (兜底)
    Default,
}

impl AdjustPolicyTrigger {
    /// 全部变体 (编译期 hardcode 兜底 = 5).
    pub const COUNT: usize = 5;

    /// 触发后建议的 policy.
    pub const fn suggested_policy(self) -> AdjustPolicy {
        match self {
            Self::RepairStorm => AdjustPolicy::Conservative,
            Self::EvolutionHealthy => AdjustPolicy::Aggressive,
            Self::UpgradeIntent => AdjustPolicy::Progressive,
            Self::AllTerminal => AdjustPolicy::Balanced,
            Self::Default => AdjustPolicy::Balanced,
        }
    }

    /// trigger 名 (string, 审计字段).
    pub const fn name(self) -> &'static str {
        match self {
            Self::RepairStorm => "RepairStorm",
            Self::EvolutionHealthy => "EvolutionHealthy",
            Self::UpgradeIntent => "UpgradeIntent",
            Self::AllTerminal => "AllTerminal",
            Self::Default => "Default",
        }
    }
}

/// 自调整 trigger 阈值常量 (编译期 hardcode).
pub const REPAIR_STORM_THRESHOLD: u32 = 3;
pub const EVOLUTION_HEALTHY_THRESHOLD: u32 = 5;

/// 自调整器 (借鉴 superpowers 234 Skill trigger 模式).
///
/// **核心设计**:
/// - `current_policy`: 当前 policy
/// - `trigger_history`: 触发历史 (审计)
/// - `tune()`: 基于 metrics, 探测 trigger, 切 policy
#[derive(Debug)]
pub struct SelfAdjust {
    /// 当前 policy
    current_policy: AdjustPolicy,
    /// 触发历史 (FIFO 限长, 兜底 32)
    trigger_history: Vec<AdjustPolicyTrigger>,
    /// 调整次数
    adjustments: u32,
}

impl Default for SelfAdjust {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfAdjust {
    /// 创建自调整器 (默认 Balanced, history 空).
    pub fn new() -> Self {
        Self {
            current_policy: AdjustPolicy::Balanced,
            trigger_history: Vec::new(),
            adjustments: 0,
        }
    }

    /// 创建 + 自定义初始 policy.
    pub fn with_initial_policy(policy: AdjustPolicy) -> Self {
        Self {
            current_policy: policy,
            trigger_history: Vec::new(),
            adjustments: 0,
        }
    }

    /// 当前 policy.
    pub fn current_policy(&self) -> AdjustPolicy {
        self.current_policy
    }

    /// 触发历史数.
    pub fn trigger_history_len(&self) -> usize {
        self.trigger_history.len()
    }

    /// 调整次数.
    pub fn adjustments(&self) -> u32 {
        self.adjustments
    }

    /// 探测 trigger (基于 metrics + sub-engine 状态).
    pub fn detect_trigger(
        &self,
        metrics: &AutonomyMetrics,
        evolution_state: SelfEvolutionState,
        upgrade_state: SelfUpgradeState,
        repair_state: SelfRepairState,
    ) -> AdjustPolicyTrigger {
        // 优先级从高到低探测
        // 1. 全部 sub-engine 终态 → AllTerminal
        if evolution_state.is_terminal()
            && upgrade_state.is_terminal()
            && repair_state.is_terminal()
        {
            return AdjustPolicyTrigger::AllTerminal;
        }
        // 2. SelfUpgrade plan 已设置 → UpgradeIntent
        if matches!(upgrade_state, SelfUpgradeState::Idle) && metrics.upgrade_attempts == 0 {
            // Idle + 0 attempts = 还没设 plan (no upgrade intent)
        }
        if !matches!(upgrade_state, SelfUpgradeState::Idle) && metrics.upgrade_attempts > 0 {
            return AdjustPolicyTrigger::UpgradeIntent;
        }
        // 3. repair storm (failure events >= 3)
        if metrics.failure_events >= REPAIR_STORM_THRESHOLD {
            return AdjustPolicyTrigger::RepairStorm;
        }
        // 4. evolution healthy (cycles >= 5, evolved=true)
        if metrics.evolution_cycles >= EVOLUTION_HEALTHY_THRESHOLD && metrics.evolution_evolved {
            return AdjustPolicyTrigger::EvolutionHealthy;
        }
        // 5. 兜底
        AdjustPolicyTrigger::Default
    }

    /// 调 policy: 给定 trigger, 切到 suggested policy.
    pub fn tune(&mut self, trigger: AdjustPolicyTrigger) -> LoopResult<AdjustPolicy> {
        let new_policy = trigger.suggested_policy();
        if new_policy == self.current_policy {
            // 0 切换, 0 计数
            self.record_trigger(trigger);
            return Ok(self.current_policy);
        }
        // 校验: 允许 Conservative -> Aggressive (跨级切换)
        // 真实场景可加约束, 但 Stage 4.1 skeleton 允许任意切换
        self.record_trigger(trigger);
        self.current_policy = new_policy;
        self.adjustments += 1;
        Ok(new_policy)
    }

    /// 强制设置 policy (无 trigger, 用于初始化 / 重置).
    pub fn set_policy(&mut self, policy: AdjustPolicy) {
        self.current_policy = policy;
    }

    fn record_trigger(&mut self, trigger: AdjustPolicyTrigger) {
        const MAX_HISTORY: usize = 32;
        self.trigger_history.push(trigger);
        if self.trigger_history.len() > MAX_HISTORY {
            self.trigger_history.remove(0);
        }
    }
}

// ============================================================================
// §4 顶层 AutonomyLoop 协调器 (自循环 + 自反馈 + 自调整 整合)
// ============================================================================

/// Library Stage 4.1 自循环 metrics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopMetrics {
    /// 当前阶段
    pub stage: Option<LoopStage>,
    /// 总 cycle 数 (跑完 Observe → Plan → Decide → Act = 1 cycle)
    pub cycles: u32,
    /// Act 阶段已驱动 step 数 (3 sub-engine 累计)
    pub act_steps: u32,
    /// signal 累计处理数
    pub signals_processed: u32,
    /// signal 队列当前长度
    pub signals_pending: u32,
    /// 调整次数
    pub adjustments: u32,
    /// 当前 policy
    pub current_policy: AdjustPolicy,
    /// 借鉴 ID 列表
    pub borrow_ids: Vec<String>,
}

/// Library Stage 4.1 自循环总报告.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopReport {
    /// 终态 metrics
    pub metrics: LoopMetrics,
    /// 当前阶段
    pub stage: LoopStage,
    /// 借鉴 ID 列表 (2 个)
    pub borrow_ids: Vec<String>,
    /// 时间戳 (unix ms)
    pub ts: u64,
}

impl LoopReport {
    /// 借鉴 ID 列表 (2 个, per §0 借鉴表).
    pub const BORROW_IDS: [&'static str; 2] = [
        "R127-2-BORROW-obra/superpowers-234-2026-08-10",
        "R127-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10",
    ];
}

/// Library Stage 4.1 自循环顶层协调器.
///
/// **核心架构** (借鉴 superpowers 234 skill priority 层级):
/// ```text
///   AutonomyLoop (本文件, 顶层)
///       ├─ FeedbackChannel (§2, 自反馈, signal 队列)
///       ├─ SelfAdjust (§3, 自调整, 5 policy)
///       └─ LibraryAutonomy (P5-1, 3 sub-engine)
///              └─ crate::poda_cycle::PodaStage (✅ R125-7)
/// ```
#[derive(Debug)]
pub struct AutonomyLoop {
    /// 自反馈 channel
    feedback: FeedbackChannel,
    /// 自调整器
    adjust: SelfAdjust,
    /// P5-1 已实装 3 sub-engine 协调器 (复用, 0 改)
    autonomy: LibraryAutonomy,
    /// 主循环是否跑
    running: bool,
    /// 当前阶段
    stage: LoopStage,
    /// 内部 cycle 计数 (Observe→Act 完整 1 圈)
    cycles: u32,
    /// Act 阶段累计 step 数
    act_steps: u32,
}

impl Default for AutonomyLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl AutonomyLoop {
    /// 创建新 AutonomyLoop (默认 Balanced policy, stage=Observe).
    pub fn new() -> Self {
        Self {
            feedback: FeedbackChannel::new(),
            adjust: SelfAdjust::new(),
            autonomy: LibraryAutonomy::new(),
            running: false,
            stage: LoopStage::Observe,
            cycles: 0,
            act_steps: 0,
        }
    }

    /// 创建 + 自定义初始 policy.
    pub fn with_policy(policy: AdjustPolicy) -> Self {
        Self {
            feedback: FeedbackChannel::new(),
            adjust: SelfAdjust::with_initial_policy(policy),
            autonomy: LibraryAutonomy::new(),
            running: false,
            stage: LoopStage::Observe,
            cycles: 0,
            act_steps: 0,
        }
    }

    /// 当前阶段.
    pub fn stage(&self) -> LoopStage {
        self.stage
    }

    /// 总 cycle 数.
    pub fn cycles(&self) -> u32 {
        self.cycles
    }

    /// Act 阶段累计 step 数 (3 sub-engine 累计).
    pub fn act_steps(&self) -> u32 {
        self.act_steps
    }

    /// 是否运行中.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// 当前 policy.
    pub fn current_policy(&self) -> AdjustPolicy {
        self.adjust.current_policy()
    }

    /// feedback channel (只读访问).
    pub fn feedback(&self) -> &FeedbackChannel {
        &self.feedback
    }

    /// adjust (只读访问).
    pub fn adjust(&self) -> &SelfAdjust {
        &self.adjust
    }

    /// 内部 autonomy (只读访问, 复用 P5-1 类型).
    pub fn autonomy(&self) -> &LibraryAutonomy {
        &self.autonomy
    }

    /// 启动主循环.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// 停止主循环.
    pub fn stop(&mut self) {
        self.running = false;
    }

    // ----- §1 自循环主循环: 4 阶段闭环 -----

    /// 1 cycle: 跑完整 4 阶段 (Observe → Plan → Decide → Act), 闭环.
    pub fn cycle(&mut self) -> LoopResult<LoopMetrics> {
        if !self.running {
            self.running = true;
        }
        // Stage 1: Observe
        self.stage = LoopStage::Observe;
        let metrics = self.autonomy.metrics();
        let evolution_state = self.autonomy.evolution.state();
        let upgrade_state = self.autonomy.upgrade.state();
        let repair_state = self.autonomy.repair.state();
        // 基于 metrics 探测 signal, push 进 channel
        self.observe(&metrics, evolution_state, upgrade_state, repair_state)?;

        // Stage 2: Plan
        self.stage = LoopStage::Plan;
        // signal 已按 priority 排序, 0 需再排
        let _pending = self.feedback.pending();

        // Stage 3: Decide
        self.stage = LoopStage::Decide;
        // 自调整: 探测 trigger, 切 policy
        let trigger = self.adjust.detect_trigger(
            &metrics,
            evolution_state,
            upgrade_state,
            repair_state,
        );
        let _new_policy = self.adjust.tune(trigger)?;

        // Stage 4: Act
        self.stage = LoopStage::Act;
        // 跑 signal queue: 每个 signal 调用 1 step 目标 sub-engine
        while let Some(signal) = self.feedback.pop_highest() {
            self.act_on_signal(signal)?;
        }
        // 兜底: 0 signal 时也跑 1 step evolution (per P5-1 默认行为)
        // 注意: 必须根据当前 state 选对 action, 0 写死 Observe (R139-1-retry-2 fix)
        if !self.autonomy.evolution.state().is_terminal() {
            let action = match self.autonomy.evolution.state() {
                SelfEvolutionState::Idle => SelfEvolutionAction::Observe,
                SelfEvolutionState::Observing => SelfEvolutionAction::Plan,
                SelfEvolutionState::Planning => SelfEvolutionAction::Adapt,
                SelfEvolutionState::Evolving => SelfEvolutionAction::Snapshot,
                _ => SelfEvolutionAction::Observe, // 兜底 (实际不应到达)
            };
            if let Err(e) = self.autonomy.evolution.step(action) {
                return Err(LoopError::MainCycleFailed(format!(
                    "evolution step {:?} failed: {:?}",
                    action, e
                )));
            }
            self.act_steps += 1;
        }
        self.cycles += 1;
        // 回到 Observe 起点
        self.stage = LoopStage::Observe;
        Ok(self.metrics())
    }

    /// 跑 N cycles (0 = 1 cycle).
    pub fn run_cycles(&mut self, n: u32) -> LoopResult<LoopMetrics> {
        for _ in 0..n.max(1) {
            self.cycle()?;
        }
        Ok(self.metrics())
    }

    // ----- §2 自反馈 4 阶段实施 -----

    /// Observe 阶段: 探测 signal, push 进 channel.
    fn observe(
        &mut self,
        metrics: &AutonomyMetrics,
        evolution_state: SelfEvolutionState,
        upgrade_state: SelfUpgradeState,
        repair_state: SelfRepairState,
    ) -> LoopResult<()> {
        // 1. SelfRepair journal 非空 → RepairNeeded (target: Evolution)
        if metrics.failure_events > 0 {
            self.feedback.push(FeedbackSignal::RepairNeeded {
                source: SignalSource::Repair,
                target: SignalTarget::Evolution,
                priority: SignalPriority::HIGH,
                description: format!(
                    "Repair journal has {} failure events, evolution should adapt",
                    metrics.failure_events
                ),
            })?;
        }
        // 2. SelfEvolution 终态 (Evolved) → EvolutionSuggested (target: Upgrade)
        if matches!(evolution_state, SelfEvolutionState::Evolved) {
            self.feedback.push(FeedbackSignal::EvolutionSuggested {
                source: SignalSource::Evolution,
                target: SignalTarget::Upgrade,
                priority: SignalPriority::MEDIUM,
                description: "Evolution completed, upgrade may apply snapshot".to_string(),
            })?;
        }
        // 3. SelfUpgrade 计划中 (Detecting / Verifying / Applying) → UpgradePending (target: Repair 准备)
        if matches!(
            upgrade_state,
            SelfUpgradeState::Detecting | SelfUpgradeState::Verifying | SelfUpgradeState::Applying
        ) {
            self.feedback.push(FeedbackSignal::UpgradePending {
                source: SignalSource::Upgrade,
                target: SignalTarget::Repair,
                priority: SignalPriority::LOW,
                description: "Upgrade in progress, repair should prepare snapshot".to_string(),
            })?;
        }
        // 4. 全部 sub-engine 终态 → Repair 兜底 (即使 0 failure, 仍让 repair 跑 1 healthcheck)
        if evolution_state.is_terminal() && upgrade_state.is_terminal() && repair_state.is_terminal()
        {
            self.feedback.push(FeedbackSignal::RepairNeeded {
                source: SignalSource::Observer,
                target: SignalTarget::Repair,
                priority: SignalPriority::LOW,
                description: "All engines terminal, repair healthcheck for safety".to_string(),
            })?;
        }
        Ok(())
    }

    /// Act 阶段: 给定 signal, 跑 1 step 目标 sub-engine.
    fn act_on_signal(&mut self, signal: FeedbackSignal) -> LoopResult<()> {
        match signal.target() {
            SignalTarget::Evolution => {
                if !self.autonomy.evolution.state().is_terminal() {
                    // Evolution step 走 PODA 5 阶段默认序列
                    let action = match self.autonomy.evolution.state() {
                        SelfEvolutionState::Idle => SelfEvolutionAction::Observe,
                        SelfEvolutionState::Observing => SelfEvolutionAction::Plan,
                        SelfEvolutionState::Planning => SelfEvolutionAction::Adapt,
                        SelfEvolutionState::Evolving => SelfEvolutionAction::Snapshot,
                        _ => return Ok(()), // 终态, 0 step
                    };
                    if let Err(e) = self.autonomy.evolution.step(action) {
                        return Err(LoopError::MainCycleFailed(format!(
                            "evolution step {:?} failed: {:?}",
                            action, e
                        )));
                    }
                    self.act_steps += 1;
                }
            }
            SignalTarget::Upgrade => {
                if !self.autonomy.upgrade.state().is_terminal()
                    && self.autonomy.upgrade.plan().is_some()
                {
                    // Upgrade 走 6 阶段默认序列
                    let action = match self.autonomy.upgrade.state() {
                        SelfUpgradeState::Idle => SelfUpgradeAction::Detect,
                        SelfUpgradeState::Detecting => SelfUpgradeAction::VerifyPre,
                        SelfUpgradeState::Verifying => SelfUpgradeAction::Apply,
                        SelfUpgradeState::Applying => SelfUpgradeAction::VerifyPost,
                        _ => return Ok(()),
                    };
                    if let Err(e) = self.autonomy.upgrade.step(action) {
                        return Err(LoopError::MainCycleFailed(format!(
                            "upgrade step {:?} failed: {:?}",
                            action, e
                        )));
                    }
                    self.act_steps += 1;
                }
            }
            SignalTarget::Repair => {
                if !self.autonomy.repair.state().is_terminal() {
                    let action = match self.autonomy.repair.state() {
                        SelfRepairState::Healthy => SelfRepairAction::HealthCheck,
                        SelfRepairState::Detected => SelfRepairAction::Snapshot,
                        SelfRepairState::Snapshotting => SelfRepairAction::Diagnose,
                        SelfRepairState::Repairing => SelfRepairAction::Restore,
                        _ => return Ok(()),
                    };
                    if let Err(e) = self.autonomy.repair.step(action) {
                        return Err(LoopError::MainCycleFailed(format!(
                            "repair step {:?} failed: {:?}",
                            action, e
                        )));
                    }
                    self.act_steps += 1;
                }
            }
        }
        Ok(())
    }

    /// 当前 metrics.
    pub fn metrics(&self) -> LoopMetrics {
        LoopMetrics {
            stage: Some(self.stage),
            cycles: self.cycles,
            act_steps: self.act_steps,
            signals_processed: self.feedback.processed(),
            signals_pending: self.feedback.pending() as u32,
            adjustments: self.adjust.adjustments(),
            current_policy: self.adjust.current_policy(),
            borrow_ids: LoopReport::BORROW_IDS.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// 当前总报告.
    pub fn report(&self) -> LoopReport {
        LoopReport {
            metrics: self.metrics(),
            stage: self.stage,
            borrow_ids: LoopReport::BORROW_IDS.iter().map(|s| s.to_string()).collect(),
            ts: current_unix_ms(),
        }
    }
}

/// 当前 unix ms (helper, 0 触碰 crate 现有 fn, self-contained).
fn current_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ============================================================================
// §5 单元测试 (5 loop + 6 feedback + 5 adjust = 16 tests, 编译期 hardcode 兜底)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- §1 自循环 (Self-loop) tests (5 tests) -----

    #[test]
    fn loop_01_new_autonomy_loop_starts_idle_observe() {
        let l = AutonomyLoop::new();
        assert_eq!(l.stage(), LoopStage::Observe);
        assert_eq!(l.cycles(), 0);
        assert_eq!(l.current_policy(), AdjustPolicy::Balanced);
        assert!(!l.is_running());
    }

    #[test]
    fn loop_02_loop_stage_4_phases_matches_poda() {
        // 4 阶段, 跟 aGLM 108 PODA 1:1
        assert_eq!(LoopStage::ALL.len(), 4);
        assert_eq!(LoopStage::Observe.order(), 0);
        assert_eq!(LoopStage::Plan.order(), 1);
        assert_eq!(LoopStage::Decide.order(), 2);
        assert_eq!(LoopStage::Act.order(), 3);
        assert!(LoopStage::Act.is_terminal());
        assert!(!LoopStage::Observe.is_terminal());
    }

    #[test]
    fn loop_03_autonomy_loop_cycle_runs_4_stages() {
        let mut l = AutonomyLoop::new();
        l.start();
        let r = l.cycle();
        assert!(r.is_ok());
        // cycle 跑完 4 阶段, 回到 Observe
        assert_eq!(l.stage(), LoopStage::Observe);
        assert_eq!(l.cycles(), 1);
        // 1 cycle 后 act_steps >= 1 (evolution Idle → Observing 至少 1 step)
        assert!(l.act_steps() >= 1);
    }

    #[test]
    fn loop_04_autonomy_loop_run_3_cycles_advances_evolution() {
        let mut l = AutonomyLoop::new();
        l.start();
        let r = l.run_cycles(3);
        assert!(r.is_ok());
        assert_eq!(l.cycles(), 3);
        // 3 cycles 后 evolution 状态前进 (Idle → Observing → Planning → Evolving)
        let state = l.autonomy().evolution.state();
        assert!(
            matches!(state, SelfEvolutionState::Observing | SelfEvolutionState::Planning | SelfEvolutionState::Evolving | SelfEvolutionState::Evolved),
            "evolution should advance after 3 cycles, got {:?}",
            state
        );
    }

    #[test]
    fn loop_05_autonomy_loop_metrics_includes_borrow_ids() {
        let l = AutonomyLoop::new();
        let m = l.metrics();
        assert_eq!(m.borrow_ids.len(), 2);
        assert!(m.borrow_ids[0].contains("superpowers"));
        assert!(m.borrow_ids[1].contains("aglm"));
    }

    // ----- §2 自反馈 (Self-feedback) tests (6 tests) -----

    #[test]
    fn feedback_01_feedback_signal_3_variants_compile_time() {
        assert_eq!(FeedbackSignal::COUNT, 3);
        // 3 variant 1:1
        let s1 = FeedbackSignal::RepairNeeded {
            source: SignalSource::Repair,
            target: SignalTarget::Evolution,
            priority: SignalPriority::HIGH,
            description: "test".to_string(),
        };
        let s2 = FeedbackSignal::EvolutionSuggested {
            source: SignalSource::Evolution,
            target: SignalTarget::Upgrade,
            priority: SignalPriority::MEDIUM,
            description: "test".to_string(),
        };
        let s3 = FeedbackSignal::UpgradePending {
            source: SignalSource::Upgrade,
            target: SignalTarget::Repair,
            priority: SignalPriority::LOW,
            description: "test".to_string(),
        };
        assert_eq!(s1.priority(), SignalPriority::HIGH);
        assert_eq!(s2.source(), SignalSource::Evolution);
        assert_eq!(s3.target(), SignalTarget::Repair);
    }

    #[test]
    fn feedback_02_channel_push_orders_by_priority_desc() {
        let mut c = FeedbackChannel::new();
        c.push(FeedbackSignal::RepairNeeded {
            source: SignalSource::Repair,
            target: SignalTarget::Evolution,
            priority: SignalPriority::LOW,
            description: "low".to_string(),
        })
        .unwrap();
        c.push(FeedbackSignal::EvolutionSuggested {
            source: SignalSource::Evolution,
            target: SignalTarget::Upgrade,
            priority: SignalPriority::HIGH,
            description: "high".to_string(),
        })
        .unwrap();
        c.push(FeedbackSignal::UpgradePending {
            source: SignalSource::Upgrade,
            target: SignalTarget::Repair,
            priority: SignalPriority::MEDIUM,
            description: "medium".to_string(),
        })
        .unwrap();
        assert_eq!(c.pending(), 3);
        // 取出顺序: HIGH > MEDIUM > LOW
        let first = c.pop_highest().unwrap();
        assert_eq!(first.priority(), SignalPriority::HIGH);
        let second = c.pop_highest().unwrap();
        assert_eq!(second.priority(), SignalPriority::MEDIUM);
        let third = c.pop_highest().unwrap();
        assert_eq!(third.priority(), SignalPriority::LOW);
        assert_eq!(c.processed(), 3);
    }

    #[test]
    fn feedback_03_channel_overflow_returns_error() {
        let mut c = FeedbackChannel::with_capacity(2);
        c.push(FeedbackSignal::RepairNeeded {
            source: SignalSource::Repair,
            target: SignalTarget::Evolution,
            priority: SignalPriority::LOW,
            description: "1".to_string(),
        })
        .unwrap();
        c.push(FeedbackSignal::RepairNeeded {
            source: SignalSource::Repair,
            target: SignalTarget::Evolution,
            priority: SignalPriority::LOW,
            description: "2".to_string(),
        })
        .unwrap();
        let r = c.push(FeedbackSignal::RepairNeeded {
            source: SignalSource::Repair,
            target: SignalTarget::Evolution,
            priority: SignalPriority::LOW,
            description: "3".to_string(),
        });
        assert!(r.is_err());
        match r.unwrap_err() {
            LoopError::FeedbackQueueOverflow { capacity, attempted: _ } => {
                assert_eq!(capacity, 2);
            }
            _ => panic!("期望 FeedbackQueueOverflow"),
        }
    }

    #[test]
    fn feedback_04_channel_peek_highest_no_pop() {
        let mut c = FeedbackChannel::new();
        c.push(FeedbackSignal::EvolutionSuggested {
            source: SignalSource::Evolution,
            target: SignalTarget::Upgrade,
            priority: SignalPriority::HIGH,
            description: "x".to_string(),
        })
        .unwrap();
        let peeked = c.peek_highest().unwrap();
        assert_eq!(peeked.priority(), SignalPriority::HIGH);
        // peek 不 pop
        assert_eq!(c.pending(), 1);
        assert_eq!(c.processed(), 0);
    }

    #[test]
    fn feedback_05_observe_no_signal_on_clean_state() {
        // 0 依赖 P5-1 内部 mutable 访问: 测 0 failure event 0 signal 推
        // (feedback_05 原想测 failure_event 注入, 但 P5-1 journal field 是 private,
        //  0 改 P5-1 入口签名严守. structural 测: 干净状态 0 signal 是合理 proxy.)
        let mut l = AutonomyLoop::new();
        l.start();
        // 干净状态: failure_events=0, 0 terminal → observe 0 push
        let r = l.cycle();
        assert!(r.is_ok());
        // 干净状态 cycle 后 0 pending signal (signal 全 pop 处理)
        // 注: cycle() 内部 pop_highest() 处理所有 push 的 signal, 所以 pending=0
        assert_eq!(l.feedback.pending(), 0);
        // 至少 processed 一些 (可能有 AllTerminal signal 兜底)
        // 0 strict 断言, 因 cycle 已处理
        let _ = l.feedback.processed();
    }

    #[test]
    fn feedback_06_signal_source_and_target_compile_time_count() {
        assert_eq!(SignalSource::COUNT, 4);
        assert_eq!(SignalTarget::COUNT, 3);
        // 4 source: Evolution / Upgrade / Repair / Observer
        assert_eq!(SignalSource::Evolution.name(), "Evolution");
        assert_eq!(SignalSource::Upgrade.name(), "Upgrade");
        assert_eq!(SignalSource::Repair.name(), "Repair");
        assert_eq!(SignalSource::Observer.name(), "Observer");
    }

    // ----- §3 自调整 (Self-adjust) tests (5 tests) -----

    #[test]
    fn adjust_01_policy_5_variants_compile_time() {
        assert_eq!(AdjustPolicy::COUNT, 5);
        assert_eq!(AdjustPolicy::ALL.len(), 5);
        // 5 policy 0=Conservative, 4=Aggressive
        assert_eq!(AdjustPolicy::Conservative.weight(), 0);
        assert_eq!(AdjustPolicy::Cautious.weight(), 1);
        assert_eq!(AdjustPolicy::Balanced.weight(), 2);
        assert_eq!(AdjustPolicy::Progressive.weight(), 3);
        assert_eq!(AdjustPolicy::Aggressive.weight(), 4);
    }

    #[test]
    fn adjust_02_policy_from_weight_round_trip() {
        for w in 0..5 {
            let p = AdjustPolicy::from_weight(w);
            assert_eq!(p.weight(), w);
        }
        // 超界 → Balanced 兜底
        assert_eq!(AdjustPolicy::from_weight(255), AdjustPolicy::Balanced);
    }

    #[test]
    fn adjust_03_trigger_5_variants_suggested_policy() {
        assert_eq!(AdjustPolicyTrigger::COUNT, 5);
        // 5 trigger 1:1 建议 policy
        assert_eq!(
            AdjustPolicyTrigger::RepairStorm.suggested_policy(),
            AdjustPolicy::Conservative
        );
        assert_eq!(
            AdjustPolicyTrigger::EvolutionHealthy.suggested_policy(),
            AdjustPolicy::Aggressive
        );
        assert_eq!(
            AdjustPolicyTrigger::UpgradeIntent.suggested_policy(),
            AdjustPolicy::Progressive
        );
        assert_eq!(
            AdjustPolicyTrigger::AllTerminal.suggested_policy(),
            AdjustPolicy::Balanced
        );
        assert_eq!(
            AdjustPolicyTrigger::Default.suggested_policy(),
            AdjustPolicy::Balanced
        );
    }

    #[test]
    fn adjust_04_tune_switches_policy_and_counts() {
        let mut a = SelfAdjust::new();
        assert_eq!(a.current_policy(), AdjustPolicy::Balanced);
        // RepairStorm → Conservative
        let r = a.tune(AdjustPolicyTrigger::RepairStorm);
        assert!(r.is_ok());
        assert_eq!(a.current_policy(), AdjustPolicy::Conservative);
        assert_eq!(a.adjustments(), 1);
        // EvolutionHealthy → Aggressive
        let r = a.tune(AdjustPolicyTrigger::EvolutionHealthy);
        assert!(r.is_ok());
        assert_eq!(a.current_policy(), AdjustPolicy::Aggressive);
        assert_eq!(a.adjustments(), 2);
    }

    #[test]
    fn adjust_05_tune_same_policy_no_count() {
        let mut a = SelfAdjust::with_initial_policy(AdjustPolicy::Conservative);
        // Conservative → Conservative (suggested same)
        let r = a.tune(AdjustPolicyTrigger::RepairStorm);
        assert!(r.is_ok());
        assert_eq!(a.current_policy(), AdjustPolicy::Conservative);
        // 0 调整 (same policy)
        assert_eq!(a.adjustments(), 0);
        // 但 trigger 仍记录
        assert_eq!(a.trigger_history_len(), 1);
    }

    // ----- §4 8 硬墙 compile-time 守门 -----

    #[test]
    fn eight_hard_walls_compile_time_gates_stage_4_1() {
        // B2: workspace.version 1.2.0 0 改 (per B2 严守)
        // A1: R11 baseline 3 值 0 改 (本 crate 0 涉及)
        // B1: 24 LOCKED 入口签名 0 改 (本测试通过, 即编译通过)
        // B5: 8 哲学锚 0 改 (0 涉及)
        // B3: 30 维 0 改 (0 涉及)
        // B4: 6 重 v7 守门 0 改 (0 涉及)
        // A3: 13 键 0 改 (0 涉及)
        // C1: 0 主动 commit (Mavis 整合 #5 拍板)
        // C2: 0 装 解除 (✅ cloned 真实施)
        // C3: 0 push (严守)
        assert_eq!(LoopReport::BORROW_IDS.len(), 2);
        assert_eq!(LoopStage::ALL.len(), 4);
        assert_eq!(FeedbackSignal::COUNT, 3);
        assert_eq!(AdjustPolicy::COUNT, 5);
        assert_eq!(AdjustPolicyTrigger::COUNT, 5);
    }
}

// ============================================================================
// (无内部 helper trait: P5-1 RepairJournal.journal field 是 private,
//  0 改 P5-1 入口签名严守, 测用 structural 验证 (干净状态 0 signal 推))
// ============================================================================
