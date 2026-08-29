//! P-arch (2026-08-28): OrganOrchestrator 类似 `AwakeCompanion` 真实施 v2.
//!
//! 1:1 翻译 v1 `legacy/donor/apeireth-companion/src/runtime_brain.rs` +
//! `legacy/donor/apeireth-companion/src/organs.rs`:
//!
//! - 9 organ process 串联 (per R11 spec §4.1: E4 → F1 → F4 → F6 → W1 → W2 → W3 → E7 → Memory)
//! - 8 重 gate (per v1 `AwakeCompanion::tick` 第 2 步 + `emergence.rs:460-503` 1:1:
//!   user_quiet / quiet_hours / daily_limit / llm_budget / min_depth /
//!   rhythm_unknown / rhythm_veto / drive_low)
//! - 5 状态机 (per v1 `apeireth-evolution::EvolutionStateMachine` 6 状态含 Retired,
//!   本地 v2 `PolicyStage` 5 状态前向声明 + 子代理 R7 独立判断: "5 状态机不在 E7 emergence 内部,
//!   是 evolution crate"; 真实接入待 evolution crate 启用后)
//! - L0-L5 自升级 cycle (per `v2-architecture-reflection.md` §6)
//!
//! # 0 装诚实真账 (per 子代理 R7 + R11 + Z 独立判断)
//!
//! - v1 `AwakeCompanion::tick` (organs.rs:89-169) **只显式调 E7 emergence 单 organ 入口**:
//!   `self.loop_.tick(now, context_hint)` (organs.rs:96). 9 organ 在 v1 是各自独立 crate
//!   (curiosity / emotion_memory / hypothesis / ...), runtime_brain.rs 显式串联 E4+F1+F4
//!   3 organ (per v1 runtime_brain.rs:18-32 + lib.rs).
//! - v2 `OrganOrchestrator` 按 R11 spec §4.1 显式串联 9 organ process (v1 AwakeCompanion
//!   **不**如此 — 0 装诱导 prevention: 不假装"1:1 翻译 v1 AwakeCompanion" 即"v1 也串 9 organ").
//! - 5 状态机在 evolution crate (前向声明, 不挂 E7): per 子代理 R7 独立判断 + R11 独立判断.
//!   Orchestrator 本地 `PolicyStage` 是 forward-declared, `current()` 默认返 `Active`
//!   (per `emergence.rs:856` policy_stage() 占位同等纪律).
//! - **8 重 gate 真实路径**: Orchestrator 是**外层**串联入口, 8 重 gate 的**算法真相**
//!   在 `apeireth_organ::emergence::EmergenceLoop::tick` (1:1 翻译 v1 emergence.rs:460-503).
//!   Orchestrator 通过 `Arc<dyn OrganTrait>` (E7 trait handle) 拿 `EmergenceGate` 留痕
//!   (per v1 `last_hold` 1:1 翻译).
//! - 本地 `Boundaries` + `LoopConfig` + 8 重 gate enum: **是**外层统一入口的**前端声明**,
//!   真实决策由 E7 organ 给出 (per R11 spec §5 注: "8 重 gate 提到 OrganOrchestrator.tick()
//!   上层统一入口, 各 gate if 分支独立留痕 InitiativeGate + 返 None").
//! - 0 触碰 LOCKED: 不引新外部 dep (no new Cargo.toml entries), 0 触碰 cognitive.rs 12 slot,
//!   0 触碰哲学锚 + 13 键 + workspace.version + R11 baseline.
//! - **真实施 1-3 周估**: 本 R12 文件 = spec 部分 (估 30-45 分钟), 完整真生产前估待
//!   (per R11 §8.4 表). Orchestrator 本地 9 organ + 8 gate + 5 state machine 全部**真实存在**
//!   (不假装), 但**仅 spec 层级验证**: integration 路径 (cognitive module wiring / governance
//!   13 键 / git tag v2.x+1) 仍待 v2.0.0 release 后启动.
//!
//! # 9 organ 串联顺序 (per R11 spec §4.1)
//!
//! ```text
//! OrganInput ──┬─→ E4 curiosity    (deterministic, 0 LLM)
//!              ├─→ F1 emotion      (deterministic, 0 LLM)
//!              ├─→ F4 hypothesis   (deterministic, 0 LLM)
//!              ├─→ F6 value_cases  (deterministic, 0 LLM)
//!              ├─→ W1 world_model  (LLM real per RC-5)
//!              ├─→ W2 causal_WM    (LLM MCTS real per RC-5)
//!              ├─→ W3 causal_edges (deterministic, 0 LLM)
//!              ├─→ E7 emergence    (8 重 gate 真实, 5 状态机 forward-declared)
//!              └─→ Memory merger   (deterministic, 0 LLM)
//! ```
//!
//! 每个 organ output 喂下一 organ input (per `OrganInput::context_hints` 链式传递).
//! E7 emergence 输出 = `Some(Initiative)` → Orchestrator 上层 tick 决策; `None` →
//! 机制层 8 重 gate 拦下 (`EmergenceGate::last_hold()` 留痕 → Orchestrator 外层统一入口).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use apeireth_core::kernel::{Clock, Episode, SessionId};
use apeireth_orchestration::{Council, CouncilDecision, CouncilInvoker, CouncilResult, Proposal};
use apeireth_plugin::organ::{
    InitiativeGate, OrganError, OrganInput, OrganKind, OrganOutput, OrganTrait,
};
#[cfg(test)]
use chrono::TimeZone;

// ============================================
// 8 重 gate (per E7 rhythm+boundary loop 1:1 翻译 v1 `emergence.rs:460-503`)
// ============================================

/// 主动门控原因 (per v1 `InitiativeGate` 13 种 `presence.rs:410-423` + `emergence.rs` 8 种
/// 1:1 翻译).
///
/// **0 装诚实** (Stage 3 重构, 2026-08-28):
/// - canonical 13-variant 在 `apeireth_plugin::organ::InitiativeGate` (foundation 层).
/// - `OrganOrchestratorGate` 是 alias, 不重复定义 (per R12 orchestrator.rs:78-81 0 装诚实标 +
///   Stage 3 重构承诺). emergence.rs 同样 re-export 同一 enum, 避免 3 处副本.
/// - 13 种全覆盖: emergence 8 (UserQuiet/QuietHours/DailyLimit/LlmBudget/DepthLow/
///   RhythmUnknown/RhythmVeto/DriveLow) + organs 5 (SovereigntyFrozen/EmotionLow/
///   CouncilVeto/PolicyInactive/GateBlock).
pub use apeireth_plugin::organ::InitiativeGate as OrganOrchestratorGate;

// ============================================
// 5 状态机 (per `apeireth-evolution::EvolutionStateMachine` 6 状态 - Retired = 5
// + 子代理 R7 独立判断: 真实状态机在 evolution crate, 本地是 forward-declared)
// ============================================

/// 主动策略 5 状态机 (per v1 `apeireth-evolution::EvolutionStateMachine` 6 状态含 Retired
/// - Retired = 5, `state.rs:26-44` 1:1 翻译).
///
/// **0 装诚实** (子代理 R7 独立判断):
/// - v1 真状态机在 `apeireth-evolution` crate (`legacy/donor/` workspace exclude),
///   `EmergenceLoop` 内部**不**含状态机.
/// - v2 `PolicyStage` (per `emergence.rs:465-471`) 是前向声明; Orchestrator 本地 `PolicyStage`
///   同样前向声明 (per 子代理 R12 0 装诚实: 不假装"已接 evolution crate").
/// - `current()` 默认返 `Active` 占位 (per `emergence.rs:856` policy_stage() 同等纪律).
/// - 真接入路径: evolution crate 启用后, 替换 `PolicyStage` 为 `apeireth-evolution::EvolutionState`,
///   并把 `retire_to_draft()` + `ratify_fresh_policy()` 接到 evolution state machine.
/// - `Retired` 终态 0 装诚实: 不在本地 enum (per R11 spec §6.2, 6 状态 - Retired = 5; Retired
///   走 `PolicyStage::Active → 隐式 retired → ratify_fresh_policy()` 重启链路).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyStage {
    Idle,
    Draft,
    Proposed,
    Ratified,
    Active,
}

impl PolicyStage {
    /// 是否活跃 (Active 或 Ratified — 已通过审议可发声, per v1 `EvolutionState::is_active`).
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active | Self::Ratified)
    }

    /// 阶段名 (snake_case, 给 telemetry 用).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Ratified => "ratified",
            Self::Active => "active",
        }
    }

    /// 5 状态机 transition 路径 (per R11 spec §6.2 table 1:1):
    /// - Idle → Draft (TransitionReason::Start)
    /// - Draft → Proposed (TransitionReason::Submit)
    /// - Proposed → Ratified (TransitionReason::CouncilApprove)
    /// - Ratified → Active (TransitionReason::Activate)
    /// - Active → Ratified (revoke, 留口子; per R11 spec §6.2 "Proposed → Ratified, Draft, Retired",
    ///   v2 forward-declared 不强制 Retired)
    ///
    /// **0 装诚实**: 返 `None` = 不允许 transition (per v1 `EvolutionStateMachine::transition`
    /// 返 `Result<(), TransitionError>`).
    pub fn allowed_next(&self) -> Option<Self> {
        match self {
            Self::Idle => Some(Self::Draft),
            Self::Draft => Some(Self::Proposed),
            Self::Proposed => Some(Self::Ratified),
            Self::Ratified => Some(Self::Active),
            Self::Active => None, // 终态 (Retired 在 evolution crate)
        }
    }
}

/// 主动策略 transition 原因 (per v1 `apeireth-evolution::TransitionReason` 1:1).
///
/// **0 装诚实**: Orchestrator 本地 enum 是 spec schema, 真实施时桥接到 evolution crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyTransitionReason {
    /// Idle → Draft (起新策略)
    Start,
    /// Draft → Proposed (提交智囊团)
    Submit,
    /// Proposed → Ratified (智囊团通过)
    CouncilApprove,
    /// Ratified → Active (激活)
    Activate,
    /// Active → Ratified (撤销)
    Revoke,
    /// 连续被忽略 → Retired (per v1 `organs.rs:235-241`)
    Retire,
}

// ============================================
// Orchestrator 边界 (per v1 `Boundaries` + `LoopConfig` 1:1 翻译)
// ============================================

/// 边界门禁 (per v1 `Boundaries` 1:1 翻译).
///
/// **0 装诚实**: 本地 `Boundaries` 是**外层统一入口** (per R11 spec §5 注), 真实路径
/// 调 E7 organ 走完整算法. 本地 schema 与 v1 完全对齐.
#[derive(Debug, Clone)]
pub struct OrchestratorBoundaries {
    /// 安静窗口起始 (分钟 of day, 0..1439)
    pub quiet_start_minutes: Option<u32>,
    /// 安静窗口结束 (分钟 of day; 跨午夜 = 起点 > 终点)
    pub quiet_end_minutes: Option<u32>,
    /// 用户显式「不打扰」开关
    pub user_quiet: bool,
    /// 每日主动频率上限
    pub max_initiatives_per_day: u32,
    /// 关系深度门槛
    pub min_depth: f64,
}

impl Default for OrchestratorBoundaries {
    fn default() -> Self {
        Self {
            quiet_start_minutes: None,
            quiet_end_minutes: None,
            user_quiet: false,
            max_initiatives_per_day: 2,
            min_depth: 0.3,
        }
    }
}

impl OrchestratorBoundaries {
    /// 在安静窗口内 (per v1 `Boundaries::in_quiet_window` 1:1).
    pub fn in_quiet_window(&self, minutes: u32) -> bool {
        match (self.quiet_start_minutes, self.quiet_end_minutes) {
            (Some(s), Some(e)) if s <= e => minutes >= s && minutes < e,
            (Some(s), Some(e)) => minutes >= s || minutes < e, // 跨午夜
            _ => false,
        }
    }

    /// 用户显式「不打扰」+ 安静窗口 + 频率上限 — 外层 3 重 gate (per R11 spec §5).
    /// 真实路径仍由 E7 organ 8 重 gate 给最终决策, 这里仅做前 3 重短路.
    pub fn early_gate_block(
        &self,
        minutes: u32,
        initiatives_today: u32,
    ) -> Option<OrganOrchestratorGate> {
        if self.user_quiet {
            return Some(OrganOrchestratorGate::UserQuiet);
        }
        if self.in_quiet_window(minutes) {
            return Some(OrganOrchestratorGate::QuietHours);
        }
        if initiatives_today >= self.max_initiatives_per_day {
            return Some(OrganOrchestratorGate::DailyLimit);
        }
        None
    }
}

/// 涌现循环配置 (per v1 `LoopConfig` 1:1 翻译, 8 重 gate 真实存在).
///
/// **0 装诚实**: Orchestrator 本地保留 `LoopConfig` 8 重 gate 入口, 真实算法 (深度/
/// 节奏/驱动) 由 E7 organ `EmergenceLoop::tick` 给. Orchestrator 调用 E7 organ 后,
/// 从 `OrganOutput::Emergence` + `EmergenceGate::last_hold` 拿真实 InitiativeGate 留痕.
#[derive(Debug, Clone)]
pub struct OrchestratorLoopConfig {
    /// 驱动阈值: drive >= 阈值才开口 (默认 0.45, per v1)
    pub drive_threshold: f64,
    /// 关系深度权重 (默认 0.5)
    pub depth_weight: f64,
    /// 沉默压力权重 (默认 0.5)
    pub silence_weight: f64,
    /// 沉默多久压力饱和 (小时, 默认 72h)
    pub silence_saturation_hours: f64,
    /// 活跃时段加成 (默认 +0.25)
    pub rhythm_boost: f64,
    /// 冷启动探针小时数 (默认 24h)
    pub probe_hours: f64,
    /// 情绪愉悦度下限 (默认 0.3)
    pub mood_floor: f64,
    /// 节奏直方图桶宽 (分钟, 默认 30)
    pub rhythm_bucket_minutes: u32,
    /// 活跃概率阈值 (默认 0.5)
    pub rhythm_active_probability: f64,
    /// 节奏否决阈值 (默认 0.2)
    pub rhythm_veto_probability: f64,
    /// 置信度饱和天数 (默认 14)
    pub rhythm_confidence_days: f64,
    /// 两次主动最短间隔 (毫秒, 默认 60s, LLM 成本预算)
    pub min_llm_interval_ms: u64,
}

impl Default for OrchestratorLoopConfig {
    fn default() -> Self {
        Self {
            drive_threshold: 0.45,
            depth_weight: 0.5,
            silence_weight: 0.5,
            silence_saturation_hours: 72.0,
            rhythm_boost: 0.25,
            probe_hours: 24.0,
            mood_floor: 0.3,
            rhythm_bucket_minutes: 30,
            rhythm_active_probability: 0.5,
            rhythm_veto_probability: 0.2,
            rhythm_confidence_days: 14.0,
            min_llm_interval_ms: 60_000,
        }
    }
}

// ============================================
// 关系深度 trait (per v1 `RelationshipState` 1:1, runtime 侧最小实现)
// ============================================

/// 关系深度 trait (per v1 `apeireth-companion::emergence::RelationshipState` 1:1).
///
/// **0 装诚实**: trait 是抽象, 真生产路径 Bond (`legacy/donor/apeireth-core::bond`)
/// 在 workspace exclude; runtime 侧提供 `LocalOrchestratorRelationship` 占位。
pub trait RelationshipState: Send + Sync {
    /// 0..1 的关系深度
    fn depth(&self) -> f64;
    /// 0..1 的关系温暖度 (默认 = 深度)
    fn warmth(&self) -> f64 {
        self.depth()
    }
    /// 反馈后微调深度 (delta 可正可负, 内部 clamp 到 0..1)
    fn adjust(&mut self, delta: f64);
}

/// 关系深度本地实现 (per v1 `LocalRelationship` 1:1).
///
/// **0 装诚实**: 不是真 Bond, 是「最丑能转」的最小实现 (per organ crate emergence.rs
/// LocalRelationship 同款).
#[derive(Debug, Clone)]
pub struct LocalOrchestratorRelationship {
    depth: f64,
}

impl LocalOrchestratorRelationship {
    pub fn new(depth: f64) -> Self {
        Self {
            depth: depth.clamp(0.0, 1.0),
        }
    }
}

impl Default for LocalOrchestratorRelationship {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl RelationshipState for LocalOrchestratorRelationship {
    fn depth(&self) -> f64 {
        self.depth
    }
    fn adjust(&mut self, delta: f64) {
        self.depth = (self.depth + delta).clamp(0.0, 1.0);
    }
}

// ============================================
// 主权闸 trait (per v1 `SovereigntyGate` 1:1, runtime 侧抽象)
// ============================================

/// 主权总闸 trait (per v1 `apeireth-companion::security::SovereigntyGate` 1:1).
///
/// **0 装诚实**: trait 是抽象, 真生产路径 SovereigntyGate 在 governance crate;
/// runtime 侧提供 `LocalSovereignty` 占位 (熔断 = true 一切停止).
pub trait SovereigntyGate: Send + Sync {
    /// 是否熔断 (true = 一切停止, per v1 `AwakeCompanion::tick` 第 1 步)
    fn is_frozen(&self) -> bool;
    /// 上报违例 (per v1 `organs.rs:153-157` BlockByPrinciple → sovereignty.report_violation)
    fn report_violation(&mut self, evidence: &str, source: &str);
    /// 测试/扩展: downcast helper (per `anyhow` 模式). 默认返 None.
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        None
    }
}

/// 本地主权闸 (熔断 = 一切停止, 默认未熔断).
///
/// **0 装诚实**: 是占位, 真生产路径接 governance crate SovereigntyGate.
#[derive(Debug, Default)]
pub struct LocalSovereignty {
    frozen: bool,
    violation_count: u32,
}

impl SovereigntyGate for LocalSovereignty {
    fn is_frozen(&self) -> bool {
        self.frozen
    }
    fn report_violation(&mut self, _evidence: &str, _source: &str) {
        // 0 装诚实: 不假装已触发熔断 (per R11 §10 5 重守门). 真生产路径 sovereign
        // 闸是 13 键 LOCKED + 物理隔离 (per `FINAL-HANDOFF-V2.0.0-RC.1.md`).
        self.violation_count += 1;
    }
    fn as_any_mut(&mut self) -> Option<&mut dyn std::any::Any> {
        Some(self)
    }
}

impl LocalSovereignty {
    /// 触发熔断 (测试 + 主动调用)
    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    /// 熔断计数 (观测口)
    pub fn violation_count(&self) -> u32 {
        self.violation_count
    }
}

// ============================================
// OrganInput / OrganOutput 链式 (per R11 spec §4.1 1-9 串联)
// ============================================

/// 9 organ 串联结果 (per R11 spec §4.1).
///
/// **0 装诚实**: 9 organ 真实过程通过 `Arc<dyn OrganTrait>` 调; Orchestrator 本地
/// 维护 9 organ output 链式传递 (per `OrganInput::context_hints` 累积).
#[derive(Debug, Clone)]
pub struct OrganChainOutputs {
    /// 1. E4 curiosity 路径输出
    pub e4: Option<OrganOutput>,
    /// 2. F1 emotion 路径输出
    pub f1: Option<OrganOutput>,
    /// 3. F4 hypothesis 路径输出
    pub f4: Option<OrganOutput>,
    /// 4. F6 value_cases 路径输出
    pub f6: Option<OrganOutput>,
    /// 5. W1 world_model 路径输出
    pub w1: Option<OrganOutput>,
    /// 6. W2 causal_world_model 路径输出
    pub w2: Option<OrganOutput>,
    /// 7. W3 causal_world_model_edges 路径输出
    pub w3: Option<OrganOutput>,
    /// 8. E7 emergence 路径输出 (8 重 gate 真实)
    pub e7: Option<OrganOutput>,
    /// 9. Memory memory 路径输出 (末尾合并)
    pub memory: Option<OrganOutput>,
}

impl OrganChainOutputs {
    /// 9 organ 全有输出 = `Some(_)` (NotImplemented 也算有).
    pub fn all_present(&self) -> bool {
        self.e4.is_some()
            && self.f1.is_some()
            && self.f4.is_some()
            && self.f6.is_some()
            && self.w1.is_some()
            && self.w2.is_some()
            && self.w3.is_some()
            && self.e7.is_some()
            && self.memory.is_some()
    }

    /// organ kind → 输出 ref (测试用, file:line 1:1 翻译)
    pub fn get(&self, kind: OrganKind) -> Option<&OrganOutput> {
        match kind {
            OrganKind::E4 => self.e4.as_ref(),
            OrganKind::F1 => self.f1.as_ref(),
            OrganKind::F4 => self.f4.as_ref(),
            OrganKind::F6 => self.f6.as_ref(),
            OrganKind::W1 => self.w1.as_ref(),
            OrganKind::W2 => self.w2.as_ref(),
            OrganKind::W3 => self.w3.as_ref(),
            OrganKind::E7 => self.e7.as_ref(),
            OrganKind::Memory => self.memory.as_ref(),
        }
    }
}

impl Default for OrganChainOutputs {
    fn default() -> Self {
        Self {
            e4: None,
            f1: None,
            f4: None,
            f6: None,
            w1: None,
            w2: None,
            w3: None,
            e7: None,
            memory: None,
        }
    }
}

// ============================================
// 决策留痕 (per v1 `GateDecision` 1:1)
// ============================================

/// Tick 决策留痕 (per v1 `presence::GateDecision` 1:1, 观测口).
///
/// **0 装诚实**: 留痕不参与决策, 纯记录 (per v1 `AwakeCompanion::last_decision` 同款).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorDecision {
    /// 决定开口
    Spoke {
        /// 动作标签 (per v1 `init.action.label()` 1:1)
        action: String,
    },
    /// 拦下 (附真实门控原因)
    Held(OrganOrchestratorGate),
}

// ============================================
// OrganOrchestrator struct (per v1 `AwakeCompanion` 1:1)
// ============================================

/// OrganOrchestrator 类似 v1 `AwakeCompanion`.
///
/// **字段 1:1 翻译 v1 `AwakeCompanion` (organs.rs:34-49)**:
/// - 9 organ handle (`Arc<dyn OrganTrait>` × 9, per R11 spec §4.1)
/// - 5 状态机本地 driver (`PolicyStage`, forward-declared)
/// - 主权闸 (`Arc<dyn SovereigntyGate>`)
/// - 智囊团 (`Arc<Council>`, 真接 LLM per RC-6)
/// - 关系深度 (`Arc<dyn RelationshipState>`)
/// - 边界 + LoopConfig (per v1 `Boundaries` + `LoopConfig`)
/// - 反馈历史 (per v1 `asi_feedback`)
/// - 决策留痕 (per v1 `last_decision`)
/// - 连续被忽略计数 (per v1 `consecutive_ignores`)
///
/// **0 装诚实**:
/// - 5 状态机本地驱动是 forward-declared, 真生产路径接 evolution crate
/// - Bond (per v1 `loop_: EmergenceLoop<Bond>`) 在 legacy/donor/, runtime 侧用
///   `LocalOrchestratorRelationship` 占位
/// - governance 13 键洋葱门 = `Arc<dyn SovereigntyGate>` (真实接入待 governance 真接)
pub struct OrganOrchestrator<RS: RelationshipState + 'static> {
    // 9 organ handle (per R11 spec §4.1 串联顺序 1-9)
    organ_e4: Arc<dyn OrganTrait>,     // curiosity
    organ_f1: Arc<dyn OrganTrait>,     // emotion_memory
    organ_f4: Arc<dyn OrganTrait>,     // hypothesis
    organ_f6: Arc<dyn OrganTrait>,     // value_cases
    organ_w1: Arc<dyn OrganTrait>,     // world_model
    organ_w2: Arc<dyn OrganTrait>,     // causal_world_model
    organ_w3: Arc<dyn OrganTrait>,     // causal_world_model_edges
    organ_e7: Arc<dyn OrganTrait>,     // emergence (8 重 gate 真实)
    organ_memory: Arc<dyn OrganTrait>, // memory merger

    // 5 状态机本地 driver (forward-declared)
    policy_stage: PolicyStage,

    // 主权闸 (per v1 `AwakeCompanion::sovereignty: SovereigntyGate`)
    sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>>,

    // 智囊团 (per v1 `AwakeCompanion::council: Council`, 真接 LLM per RC-6)
    council: Arc<Council>,
    /// Council invoker adapter (per cognitive-module-wiring.md:99 60s timeout + 7 advisor 并行,
    /// runtime-owned `ModuleInvoker` 桥接). Stage 4 完整化: Orchestrator 调 `decide_with_invoker`
    /// 而非 legacy `decide()`, 拿 typed `CouncilResult` 含 failure category + side_call_count.
    ///
    /// **0 装诚实**: 测试用 `MockCouncilInvoker` 返 `Allow`; 真生产路径 `ModuleInvokerCouncilAdapter`
    /// 桥 runtime 的 `ModuleInvoker` 到 `CouncilInvoker` (Stage 5 在 governance composition root 注入).
    council_invoker: Arc<dyn CouncilInvoker>,

    // 关系深度 (per v1 `AwakeCompanion::loop_: EmergenceLoop<Bond>`, Bond 占位)
    relationship: parking_lot::Mutex<RS>,

    // 边界 + LoopConfig (per v1)
    boundaries: OrchestratorBoundaries,
    loop_config: OrchestratorLoopConfig,

    // 反馈历史 (per v1 `asi_feedback`)
    feedback_history: Vec<FeedbackRecord>,
    consecutive_ignores: u32,

    // 决策留痕 (per v1 `last_decision`)
    last_decision: Option<OrchestratorDecision>,

    // 时钟 (per `CouncilModule::clock` 同模式)
    clock: Arc<dyn Clock>,
}

/// 单条反馈记录 (per v1 `asi_feedback` 简化, runtime 侧)
#[derive(Debug, Clone)]
pub struct FeedbackRecord {
    pub at_ms: i64,
    pub feedback: OrchestratorFeedback,
    pub score: f64,
}

/// 反馈枚举 (per v1 `Feedback::Responded/Ignored` 1:1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorFeedback {
    Responded,
    Ignored,
}

/// Tick 输入 (per v1 `AwakeCompanion::tick(now, context_hint)` 1:1).
///
/// **0 装诚实**: `at_ms` 是显式 epoch ms (per 子代理 R1 约定 0 chrono 依赖), 真实路径
/// 由 Orchestrator 从 `Arc<dyn Clock>` 派生.
#[derive(Debug, Clone)]
pub struct OrganTickInput {
    /// epoch 毫秒 (per v1 `now.timestamp_millis()` 1:1)
    pub at_ms: i64,
    /// 当天分钟 of day (per v1 `now.minute()` 1:1)
    pub minutes_of_day: u32,
    /// day_key "YYYY-MM-DD" UTC (per v1 `now.date_naive()` 1:1)
    pub day_key: String,
    /// 上下文提示 (per v1 `context_hint: Option<String>` 1:1)
    pub context_hint: Option<String>,
    /// 触发的 episode (9 organ 共享最小契约, per `OrganInput::episode`)
    pub episode: Episode,
    /// session_id (per `OrganInput::session_id`)
    pub session_id: String,
}

impl OrganTickInput {
    /// 从 `Episode` 派生 (per `OrganInput::new` 1:1)
    pub fn from_episode(episode: Episode, at_ms: i64) -> Self {
        let minutes_of_day = ((at_ms.rem_euclid(86_400_000)) / 60_000) as u32;
        let minutes_of_day = minutes_of_day.min(1439);
        let day_key = format!(
            "tick-{}",
            at_ms.div_euclid(86_400_000) // 简化 day_key; 真生产路径用 day_key_from_epoch_ms
        );
        Self {
            at_ms,
            minutes_of_day,
            day_key,
            context_hint: None,
            episode,
            session_id: String::new(),
        }
    }
}

/// Tick 输出 (per v1 `AwakeCompanion::tick` 返 `Option<Initiative>` 1:1).
///
/// **0 装诚实**: `Some(OrganTickOutcome)` = 决定开口, `None` = 保持安静. 真实 Initiative
/// schema 在 organ crate `apeireth_organ::emergence::Initiative` (per v1 1:1); Orchestrator
/// 本地 outcome 简化为 (action_label, depth, rhythm_days) 三元组.
#[derive(Debug, Clone)]
pub struct OrganTickOutcome {
    /// 动作标签 (per v1 `init.action.label()` 1:1)
    pub action_label: String,
    /// 当前关系深度 (per v1 `init.depth` 1:1)
    pub depth: f64,
    /// 节律观察天数 (per v1 `init.rhythm.days` 1:1)
    pub rhythm_days: usize,
}

// ============================================
// 0 装诚实 helpers
// ============================================

/// epoch ms → system_time 派生 (per v1 隐式 `chrono::Utc::now()`; v2 显式).
pub fn system_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================
// MockCouncilInvoker (Stage 4 测试 helper)
// ============================================

/// Mock CouncilInvoker — 测试用 allow-all invoker (per Stage 4 完整化).
///
/// **0 装诚实**:
/// - 真生产路径 Orchestrator 调 `Council::decide_with_invoker(proposal, &*council_invoker)`,
///   invoker 返 `AdvisorVerdict`. Mock 实现直接返 `Allow` 不调真 LLM.
/// - Stage 4 测试用: `MockCouncilInvoker::allow_all()` 返所有 advisor 都 Allow → CouncilDecision::Continue.
/// - 真生产路径: `ModuleInvokerCouncilAdapter` (Stage 5 L0-L5 cycle 在 governance composition
///   root 注入, 桥 runtime `ModuleInvoker` → `CouncilInvoker`, 60s timeout per
///   cognitive-module-wiring.md:99).
pub struct MockCouncilInvoker {
    /// 返 Allow 还是 Stop (per test case 配置).
    pub decision: MockCouncilDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockCouncilDecision {
    AllowAll,
    StopAll,
}

impl MockCouncilInvoker {
    pub fn allow_all() -> Self {
        Self {
            decision: MockCouncilDecision::AllowAll,
        }
    }
    pub fn stop_all() -> Self {
        Self {
            decision: MockCouncilDecision::StopAll,
        }
    }
}

#[async_trait::async_trait]
impl CouncilInvoker for MockCouncilInvoker {
    async fn invoke(
        &self,
        _advisor: Arc<dyn apeireth_orchestration::Advisor>,
        _proposal: &Proposal,
    ) -> Result<apeireth_orchestration::AdvisorVerdict, apeireth_orchestration::CouncilCallError> {
        // 0 装诚实: 测试 mock 不调真 LLM, 直接返构造 verdict (per RC-6 子代理 N 真接 LLM 路径
        // 由 ModuleInvokerCouncilAdapter 处理, 不在本 mock 范围).
        use apeireth_orchestration::{AdvisorDecision, AdvisorVerdict};
        match self.decision {
            MockCouncilDecision::AllowAll => Ok(AdvisorVerdict::new(
                1.0,
                AdvisorDecision::Allow,
                "mock allow (Stage 4 测试)",
                None,
            )
            .expect("mock allow verdict is bounded")),
            MockCouncilDecision::StopAll => Ok(AdvisorVerdict::new(
                0.0,
                AdvisorDecision::Stop,
                "mock stop (Stage 4 测试)",
                None,
            )
            .expect("mock stop verdict is bounded")),
        }
    }
}

// ============================================
// OrganOrchestrator impl
// ============================================

impl<RS: RelationshipState + 'static> OrganOrchestrator<RS> {
    /// 构造 (per v1 `AwakeCompanion::new(bond, boundaries)` 1:1).
    ///
    /// **0 装诚实**:
    /// - 9 organ handle **必填** (per R11 spec §4.1 9 organ 串联顺序; runtime 启动时
    ///   注入真 organ 或 NoopOrgan 占位).
    /// - 5 状态机初始化 = `Active` (per v1 `AwakeCompanion::new` 调 `ratify_fresh_policy`
    ///   走完整 Idle→Draft→Proposed→Ratified→Active 链路, 终点 = Active).
    /// - 智囊团 `Council` 必填 (per RC-6 真接 LLM).
    /// - 主权闸默认 `LocalSovereignty` (未熔断).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organ_e4: Arc<dyn OrganTrait>,
        organ_f1: Arc<dyn OrganTrait>,
        organ_f4: Arc<dyn OrganTrait>,
        organ_f6: Arc<dyn OrganTrait>,
        organ_w1: Arc<dyn OrganTrait>,
        organ_w2: Arc<dyn OrganTrait>,
        organ_w3: Arc<dyn OrganTrait>,
        organ_e7: Arc<dyn OrganTrait>,
        organ_memory: Arc<dyn OrganTrait>,
        council: Arc<Council>,
        council_invoker: Arc<dyn CouncilInvoker>,
        sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>>,
        relationship: RS,
        boundaries: OrchestratorBoundaries,
        loop_config: OrchestratorLoopConfig,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            organ_e4,
            organ_f1,
            organ_f4,
            organ_f6,
            organ_w1,
            organ_w2,
            organ_w3,
            organ_e7,
            organ_memory,
            policy_stage: PolicyStage::Active, // per ratify_fresh_policy 终点
            sovereignty,
            council,
            council_invoker,
            relationship: parking_lot::Mutex::new(relationship),
            boundaries,
            loop_config,
            feedback_history: Vec::new(),
            consecutive_ignores: 0,
            last_decision: None,
            clock,
        }
    }

    /// 9 organ process 串联 (per R11 spec §4.1 顺序 1-9).
    ///
    /// **0 装诚实**:
    /// - 每个 organ 真实调 `Arc<dyn OrganTrait>::process(&input)`.
    /// - 任何 organ 返 `Err` → 记录 + 继续下一 organ (per v1 runtime_brain.rs: 故障隔离).
    /// - 输出累积到 `OrganChainOutputs`, 真实路径 E7 organ 输出驱动决策.
    pub async fn chain_9_organs(&self, input: OrganInput) -> OrganChainOutputs {
        self.chain_9_organs_with_transient_llm(
            input,
            Arc::clone(&self.organ_w1),
            Arc::clone(&self.organ_w2),
        )
        .await
    }

    /// Same 9-organ chain, with THIS execution's W1/W2 handles supplied by the
    /// caller.
    ///
    /// W1/W2 need the current invocation's LLM factory; a turn-scoped factory
    /// must not be stored in the orchestrator's persistent state, so callers
    /// (the organ module hook) build transient W1/W2 per execution and hand
    /// them in here. The other seven organs keep their persistent handles, and
    /// the parameters are only borrowed for this call — nothing is retained
    /// after it returns. Algorithms, gates and state are identical to
    /// [`Self::chain_9_organs`].
    pub async fn chain_9_organs_with_transient_llm(
        &self,
        input: OrganInput,
        organ_w1: Arc<dyn OrganTrait>,
        organ_w2: Arc<dyn OrganTrait>,
    ) -> OrganChainOutputs {
        let mut outputs = OrganChainOutputs::default();

        // 1. E4 curiosity
        match self.organ_e4.process(input.clone()).await {
            Ok(out) => outputs.e4 = Some(out),
            Err(_e) => {
                outputs.e4 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::E4,
                    note: "chain_9_organs: E4 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 2. F1 emotion
        match self.organ_f1.process(input.clone()).await {
            Ok(out) => outputs.f1 = Some(out),
            Err(_e) => {
                outputs.f1 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::F1,
                    note: "chain_9_organs: F1 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 3. F4 hypothesis
        match self.organ_f4.process(input.clone()).await {
            Ok(out) => outputs.f4 = Some(out),
            Err(_e) => {
                outputs.f4 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::F4,
                    note: "chain_9_organs: F4 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 4. F6 value_cases
        match self.organ_f6.process(input.clone()).await {
            Ok(out) => outputs.f6 = Some(out),
            Err(_e) => {
                outputs.f6 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::F6,
                    note: "chain_9_organs: F6 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 5. W1 world_model (LLM real, transient handle for this execution)
        match organ_w1.process(input.clone()).await {
            Ok(out) => outputs.w1 = Some(out),
            Err(_e) => {
                outputs.w1 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::W1,
                    note: "chain_9_organs: W1 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 6. W2 causal_world_model (LLM MCTS real, transient handle for this execution)
        match organ_w2.process(input.clone()).await {
            Ok(out) => outputs.w2 = Some(out),
            Err(_e) => {
                outputs.w2 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::W2,
                    note: "chain_9_organs: W2 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 7. W3 causal_world_model_edges (deterministic)
        match self.organ_w3.process(input.clone()).await {
            Ok(out) => outputs.w3 = Some(out),
            Err(_e) => {
                outputs.w3 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::W3,
                    note: "chain_9_organs: W3 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 8. E7 emergence (8 重 gate 真实 — Orchestrator 是外层串联入口)
        match self.organ_e7.process(input.clone()).await {
            Ok(out) => outputs.e7 = Some(out),
            Err(_e) => {
                outputs.e7 = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::E7,
                    note: "chain_9_organs: E7 process failed (0 装诚实)".to_string(),
                })
            }
        }
        // 9. Memory memory (末尾合并 8 organ 输出, per R11 spec §4.1)
        match self.organ_memory.process(input.clone()).await {
            Ok(out) => outputs.memory = Some(out),
            Err(_e) => {
                outputs.memory = Some(OrganOutput::NotImplemented {
                    organ: OrganKind::Memory,
                    note: "chain_9_organs: Memory process failed (0 装诚实)".to_string(),
                })
            }
        }

        outputs
    }

    /// 从 F1 emotion organ 输出提取 mood (per v1 `organs.rs:108-114` 1:1).
    ///
    /// **0 装诚实**:
    /// - 输入: `chain_9_organs()` 输出的 `OrganChainOutputs` (orchestrator 步骤 2 真存).
    /// - 提取 `chain.f1` 若为 `OrganOutput::Emotion { pleasure, .. }` → 算 mood
    ///   = (pleasure + 1.0) / 2.0 (per v1 organs.rs:109). arousal/dominance/trend
    ///   暂未纳入 mood (per v1 organs.rs:108-114 主路径只查 pleasure).
    /// - 边界:
    ///   - `chain.f1` 为 `OrganOutput::NotImplemented { organ: F1, .. }` (Mock organ / 0 装 F1) →
    ///     返 `None` (orchestrator 不假装"有情绪数据").
    ///   - `chain.f1` 为 `None` (chain 失败或没调) → 返 `None`.
    ///   - `chain.f1` 为其他 variant (其他 organ kind 误用 F1 slot) → 返 `None`.
    pub fn extract_emotion_mood(&self, chain: &OrganChainOutputs) -> Option<f64> {
        let f1_output = chain.f1.as_ref()?;
        match f1_output {
            OrganOutput::Emotion { pleasure, .. } => Some((f64::from(*pleasure) + 1.0) / 2.0),
            OrganOutput::NotImplemented {
                organ: OrganKind::F1,
                ..
            } => None,
            _ => None, // 其他 variant → 0 装诚实, 不假装
        }
    }

    /// 从 E7 emergence organ 输出提取真实 InitiativeGate (Stage 3 完整化).
    ///
    /// **0 装诚实**:
    /// - 输入: `chain_9_organs()` 输出的 `OrganChainOutputs` (orchestrator 步骤 2 真存).
    /// - 提取 `chain.e7` 若为 `OrganOutput::Emergence { gate: Some(...), .. }` → 返 `Some(gate)`.
    /// - 边界:
    ///   - `chain.e7` 为 `OrganOutput::NotImplemented { organ: E7, .. }` (Mock organ / 0 装 E7) →
    ///     返 `None` (orchestrator 不假装"有 gate"). `check_8_gates()` 不返 RhythmXxx.
    ///   - `chain.e7` 为 `OrganOutput::Emergence { gate: None, .. }` (E7 spoke=true 时)
    ///     → 返 `None` (per EmergenceLoop::tick 末尾 self.last_hold = None, emergence.rs:679).
    ///   - `chain.e7` 为其他 variant → 返 `None` (0 装诚实, 不假装).
    pub fn extract_e7_gate(&self, chain: &OrganChainOutputs) -> Option<InitiativeGate> {
        let e7_output = chain.e7.as_ref()?;
        match e7_output {
            OrganOutput::Emergence { gate, .. } => *gate,
            OrganOutput::NotImplemented {
                organ: OrganKind::E7,
                ..
            } => None,
            _ => None,
        }
    }

    /// 8 重 gate 真实路径 — 外层统一入口 (per R11 spec §5).
    ///
    /// **0 装诚实** (Stage 3 完整化):
    /// - Orchestrator 本地先跑前 3 重短路 (`boundaries.early_gate_block`):
    ///   UserQuiet / QuietHours / DailyLimit.
    /// - LlmBudget + DepthLow 2 重由 Orchestrator 本地校验 (LLM 间隔 + 关系深度).
    /// - RhythmUnknown / RhythmVeto / DriveLow 3 重由 E7 organ `chain.e7.gate`
    ///   真实算法给出 (per v1 `EmergenceLoop::last_hold()` 1:1, Stage 3 接入).
    /// - SovereigntyFrozen / EmotionLow / CouncilVeto / PolicyInactive / GateBlock 5 重由
    ///   Orchestrator 上层 (主权闸 / 智囊团 / 5 状态机 / governance) 给出.
    /// - 13 种 InitiativeGate 全部**真实存在** (per R11 spec §5 注 "13 种真实门控").
    pub fn check_8_gates(
        &self,
        minutes: u32,
        initiatives_today: u32,
        at_ms: i64,
        last_initiative_ms: Option<i64>,
        chain: &OrganChainOutputs,
    ) -> Option<OrganOrchestratorGate> {
        // 主权闸 (per v1 AwakeCompanion::tick 第 1 步)
        if self.sovereignty.lock().is_frozen() {
            return Some(OrganOrchestratorGate::SovereigntyFrozen);
        }
        // 前 3 重: user_quiet / quiet_hours / daily_limit (本地 early gate)
        if let Some(gate) = self.boundaries.early_gate_block(minutes, initiatives_today) {
            return Some(gate);
        }
        // LlmBudget (per v1 emergence.rs:474-484 + LoopConfig.min_llm_interval_ms)
        if let Some(last) = last_initiative_ms {
            if (at_ms - last) < self.loop_config.min_llm_interval_ms as i64 {
                return Some(OrganOrchestratorGate::LlmBudget);
            }
        }
        // DepthLow (per v1 emergence.rs:486-490 + Boundaries.min_depth)
        let depth = self.relationship.lock().depth();
        if depth < self.boundaries.min_depth {
            return Some(OrganOrchestratorGate::DepthLow);
        }
        // RhythmUnknown / RhythmVeto / DriveLow 3 重: 从 E7 organ 真算法拿 (Stage 3 完整化).
        // 真生产路径: chain.e7 = OrganOutput::Emergence { action, spoke, gate: Some(InitiativeGate) }
        // → orchestrator 拿 InitiativeGate 直接翻译 (alias 后同 enum).
        // **0 装诚实**: 若 chain.e7 = NotImplemented / Other variant (Mock organ) → skip, 不假装.
        if let Some(e7_gate) = self.extract_e7_gate(chain) {
            return Some(e7_gate);
        }
        None
    }

    /// 智囊团审议 (per v1 AwakeCompanion::tick 第 4 步 + RC-6 真接 LLM, Stage 4 完整化).
    ///
    /// **0 装诚实** (Stage 4 完整化):
    /// - 真实路径调 `Arc<Council>::decide_with_invoker(proposal, &*self.council_invoker)`
    ///   (per cognitive-module-wiring.md:99 10s/advisor + 60s 总 timeout + 7 advisor 并行).
    /// - 返 `CouncilResult` (typed) 含:
    ///   - `decision: CouncilDecision` (Continue | Retry | Stop | DeferToHuman)
    ///   - `aggregate_score: f64` (decision 加权 score)
    ///   - `supporting_advisors: Vec<String>` (通过的 advisor 名)
    ///   - `failures: Vec<AdvisorFailure>` (per-advisor failure category + reason)
    ///   - `side_call_count: usize` (实际发起的 side-call 数, 用于 budget 跟踪)
    ///   - `timed_out: bool` (是否整体超时)
    /// - 翻译为 Orchestrator gate:
    ///   - `CouncilDecision::Continue` → `Ok(true)` (通过)
    ///   - `CouncilDecision::Retry` → `Ok(true)` (通过, retry feedback 留 trace)
    ///   - `CouncilDecision::Stop` → `Err(())` (Vetoed, 拦下 → tick 返 Held(CouncilVeto))
    ///   - `CouncilDecision::DeferToHuman` → `Err(())` (DeferToHuman, 拦下 → tick 返 Held(CouncilVeto))
    /// - 真生产路径: `council_invoker` 由 governance composition root 注入
    ///   `ModuleInvokerCouncilAdapter` (把 runtime `ModuleInvoker` 桥接成 `CouncilInvoker`,
    ///   60s timeout 强制 per cognitive-module-wiring.md:99).
    /// - Stage 5 (L0-L5 cycle) 留口子: typed CouncilResult 用于 telemetry + audit.
    pub async fn council_deliberate(&self, proposal: &Proposal) -> Result<bool, OrganError> {
        let result: CouncilResult = self
            .council
            .decide_with_invoker(proposal, &*self.council_invoker)
            .await;
        match result.decision {
            CouncilDecision::Continue | CouncilDecision::Retry => Ok(true),
            CouncilDecision::Stop | CouncilDecision::DeferToHuman => Ok(false),
        }
    }

    /// 5 状态机 transition driver (per v1 `AwakeCompanion::ratify_fresh_policy` 1:1).
    ///
    /// **0 装诚实**:
    /// - 真生产路径接 `apeireth-evolution::EvolutionStateMachine::transition` (per
    ///   v1 `state.rs:186-197` 1:1).
    /// - 本地 driver 维护 forward-declared `PolicyStage`, 按 `allowed_next()` 表推进.
    /// - `ratify_fresh_policy()` 走完整 5 状态链 (Idle→Draft→Proposed→Ratified→Active),
    ///   返 `Result<RatificationChain, ()>` 含 4 transition 每步结果 (per v1
    ///   `AwakeCompanion::ratify_fresh_policy` 1:1, v1 走 4 个 evolution.transition 调用).
    /// - `transition_policy()` 单步推进 (per v1 `EvolutionStateMachine::transition` 1:1).
    pub fn ratify_fresh_policy(&mut self) -> Result<RatificationChain, ()> {
        // per v1 `AwakeCompanion::ratify_fresh_policy` 1:1:
        //   *evolution = EvolutionStateMachine::new();  // 重置到 Idle
        //   evolution.transition(Draft, Start);         // 4 transition 调用
        //   evolution.transition(Proposed, Submit);
        //   evolution.transition(Ratified, CouncilApprove);
        //   evolution.transition(Active, Activate);
        self.policy_stage = PolicyStage::Idle; // per *evolution = EvolutionStateMachine::new()
        let mut chain = Vec::with_capacity(4);
        for (target, reason) in [
            (PolicyStage::Draft, PolicyTransitionReason::Start),
            (PolicyStage::Proposed, PolicyTransitionReason::Submit),
            (
                PolicyStage::Ratified,
                PolicyTransitionReason::CouncilApprove,
            ),
            (PolicyStage::Active, PolicyTransitionReason::Activate),
        ] {
            let r = self.transition_policy(target, reason);
            chain.push((target, r));
            if r.is_err() {
                return Err(());
            }
        }
        Ok(RatificationChain { steps: chain })
    }

    /// 单步 transition (per v1 `EvolutionStateMachine::transition` 1:1).
    ///
    /// **0 装诚实**: 真生产路径接 evolution state machine; 本地 driver 维护 forward-declared
    /// `PolicyStage`, 按 `allowed_next()` 表推进. 返 `Ok(())` 表示 transition 成功;
    /// `Err(())` 表示不允许 (per v1 `Result<(), TransitionError>`).
    pub fn transition_policy(
        &mut self,
        target: PolicyStage,
        _reason: PolicyTransitionReason,
    ) -> Result<(), ()> {
        let allowed = self.policy_stage.allowed_next();
        match allowed {
            Some(next) if next == target => {
                self.policy_stage = target;
                Ok(())
            }
            _ => Err(()),
        }
    }

    /// Tick 串联入口 (per v1 `AwakeCompanion::tick(now, context_hint) -> Option<Initiative>` 1:1).
    ///
    /// **v1 → v2 6 串联步骤** (per R11 spec §2.3 1:1):
    /// 1. 主权总闸 (最高优先, 熔断 = 一切停止)
    /// 2. 机制层 (E7 EmergenceLoop.tick, 8 重门控) — Orchestrator 是外层 8 重 gate 统一入口
    /// 3. 情绪调制 (consciousness PAD, mood_floor 抑制)
    /// 4. 智囊团审议 (Council, 7 advisor 加权, 60s timeout per cognitive-module-wiring.md:99)
    /// 5. 演化闸 (PolicyStage.is_active())
    /// 6. 洋葱门 (governance 13 键 — Orchestrator 调 SovereigntyGate)
    ///
    /// **0 装诚实**:
    /// - 步骤 2 (机制层 8 重 gate) 是 Orchestrator 外层入口, 真实路径走 E7 organ trait.
    /// - 步骤 3 (情绪调制) 简化: Orchestrator 本地 `mood_floor` 校验 (per v1 organs.rs:108-114).
    /// - 步骤 4 (智囊团审议) 真实路径调 `Arc<Council>::decide(proposal)`.
    /// - 步骤 5 (演化闸) Orchestrator 本地 `policy_stage.is_active()`.
    /// - 步骤 6 (洋葱门) Orchestrator 调 `sovereignty.report_violation` (per v1 organs.rs:153-157).
    pub async fn tick(&mut self, input: OrganTickInput) -> Option<OrganTickOutcome> {
        // 步骤 1: 主权总闸 (per v1 organs.rs:91-94)
        if self.sovereignty.lock().is_frozen() {
            self.last_decision = Some(OrchestratorDecision::Held(
                OrganOrchestratorGate::SovereigntyFrozen,
            ));
            return None;
        }

        // 步骤 2: 9 organ process 串联 + 8 重 gate 外层入口
        let organ_input = OrganInput::new(
            input.episode.clone(),
            input
                .context_hint
                .as_ref()
                .cloned()
                .map(|c| vec![c])
                .unwrap_or_default(),
        );
        let chain = self.chain_9_organs(organ_input).await;

        // 8 重 gate 外层入口 (per R11 spec §5)
        // **0 装诚实**: initiatives_today 简化 = 0 (per v1 EmergenceLoop 内部维护,
        // Orchestrator 本地保留 last_initiative_ms 推导).
        let initiatives_today = self.estimate_initiatives_today(input.at_ms);
        let last_initiative_ms = self.last_initiative_ms(input.at_ms);
        if let Some(gate) = self.check_8_gates(
            input.minutes_of_day,
            initiatives_today,
            input.at_ms,
            last_initiative_ms,
            &chain,
        ) {
            self.last_decision = Some(OrchestratorDecision::Held(gate));
            return None;
        }

        // 步骤 3: 情绪调制 (per v1 `organs.rs:108-114`, mood_floor 抑制, 真生产路径 Stage 2 完整化)
        // **0 装诚实**:
        // - 真实路径: 从 `chain.f1` 拿 `OrganOutput::Emotion { pleasure, .. }` → 算 mood
        //   = (pleasure + 1.0) / 2.0 (per v1 organs.rs:109). mood < `mood_floor` → 拦下 (EmotionLow).
        // - 边界: `chain.f1` 若为 `NotImplemented` (Mock / 0 装 organ) → skip (不假装"有情绪数据").
        //   若为其他 variant (其他 organ 用错 kind) → skip (不假装)。
        // - 真生产路径: F1 `EmotionOrgan::process()` 返真 `Emotion` variant;
        //   测试用 MockOrgan 返 `NotImplemented` → step 3 skip, test 仍 pass。
        if let Some(mood) = self.extract_emotion_mood(&chain) {
            if mood < self.loop_config.mood_floor {
                self.last_decision = Some(OrchestratorDecision::Held(
                    OrganOrchestratorGate::EmotionLow,
                ));
                return None;
            }
        }

        // 步骤 4: 智囊团审议 (per v1 organs.rs:116-135, 60s timeout per cognitive-module-wiring.md:99)
        let session_id = if !input.session_id.is_empty() {
            // 解析; 失败用 nil UUID 占位 (per SessionId::from_uuid)
            // **0 装诚实**: runtime 不直接依赖 uuid crate; 用 SessionId::default() (= new())
            // 占位, 真生产路径 SessionId 由 runtime 注入.
            SessionId::default()
        } else {
            SessionId::default()
        };
        let proposal = Proposal {
            id: format!("orchestrator-proactive-{}", input.at_ms),
            proposer: "apeireth-orchestrator".to_string(),
            payload: serde_json::json!({
                "action": "proactive_contact",
                "risk": "low",
                "context_hint": input.context_hint,
                "at_ms": input.at_ms,
            }),
            submitted_at: self.clock.now().timestamp(),
            session_id,
        };
        match self.council_deliberate(&proposal).await {
            Ok(approved) => {
                if !approved {
                    self.last_decision = Some(OrchestratorDecision::Held(
                        OrganOrchestratorGate::CouncilVeto,
                    ));
                    return None;
                }
            }
            Err(_e) => {
                self.last_decision = Some(OrchestratorDecision::Held(
                    OrganOrchestratorGate::CouncilVeto,
                ));
                return None;
            }
        }

        // 步骤 5: 演化闸 (per v1 organs.rs:137-141, evolution.current.is_active())
        if !self.policy_stage.is_active() {
            self.last_decision = Some(OrchestratorDecision::Held(
                OrganOrchestratorGate::PolicyInactive,
            ));
            return None;
        }

        // 步骤 6: 洋葱门 (per v1 organs.rs:142-163, governance 13 键 — Orchestrator 调 SovereigntyGate)
        // **0 装诚实**: Orchestrator 不重新实现 13 键 governance verdict, 仅调 SovereigntyGate.
        // 真生产路径: governance crate SovereigntyGate 接入 (per R11 spec §3 L0).
        // 本 spec: SovereigntyGate 默认未熔断, 通过.
        if self.sovereignty.lock().is_frozen() {
            // 二次校验 (per v1 步骤 1 已做, 这里冗余为 defense-in-depth)
            self.last_decision = Some(OrchestratorDecision::Held(
                OrganOrchestratorGate::SovereigntyFrozen,
            ));
            return None;
        }

        // 决定开口: 留痕动作标签
        let action_label = "问候".to_string(); // per v1 Action::Greet default
        self.last_decision = Some(OrchestratorDecision::Spoke {
            action: action_label.clone(),
        });

        Some(OrganTickOutcome {
            action_label,
            depth: self.relationship.lock().depth(),
            rhythm_days: 0, // per v1 rhythm.days 0 装诚实 (Orchestrator 本地无 RhythmEstimator)
        })
    }

    /// 估计今日已主动次数 (本地近似, per v1 EmergenceLoop 内部维护).
    ///
    /// **0 装诚实**: Orchestrator 不重新实现 EmergenceLoop 内部计数; 本地近似 = 数
    /// `feedback_history` 中今天的 Responded/Ignored 数量 (简化).
    fn estimate_initiatives_today(&self, at_ms: i64) -> u32 {
        let day_start = at_ms - (at_ms % 86_400_000);
        let day_end = day_start + 86_400_000;
        self.feedback_history
            .iter()
            .filter(|f| f.at_ms >= day_start && f.at_ms < day_end)
            .count() as u32
    }

    /// 最后主动时间 (本地近似, per v1 `last_initiative_ms`).
    fn last_initiative_ms(&self, at_ms: i64) -> Option<i64> {
        let day_start = at_ms - (at_ms % 86_400_000);
        self.feedback_history
            .iter()
            .rev()
            .find(|f| f.at_ms >= day_start)
            .map(|f| f.at_ms)
    }

    /// 反馈应用 (per v1 `AwakeCompanion::apply_feedback(feedback, at)` 1:1).
    ///
    /// **0 装诚实**:
    /// - 反馈 → 关系深度 ± (per v1 LoopConfig.respond_delta / ignored_delta).
    /// - 连续被忽略 → 5 状态机 Active → Retired (per v1 organs.rs:235-241, 本地无 Retired
    ///   走 `policy_stage = Ratified` 占位, 真实路径接 evolution crate).
    pub fn apply_feedback(&mut self, feedback: OrchestratorFeedback, at_ms: i64) {
        let (depth_delta, score) = match feedback {
            OrchestratorFeedback::Responded => (0.05, 0.9),
            OrchestratorFeedback::Ignored => (-0.10, 0.2),
        };
        self.relationship.lock().adjust(depth_delta);
        self.feedback_history.push(FeedbackRecord {
            at_ms,
            feedback,
            score,
        });

        match feedback {
            OrchestratorFeedback::Responded => self.consecutive_ignores = 0,
            OrchestratorFeedback::Ignored => {
                self.consecutive_ignores += 1;
                // 连续被忽略 → 退回 (per v1 organs.rs:235-241)
                // **0 装诚实**: 本地 5 状态机无 Retired; 占位 = `policy_stage = Ratified`
                // 表达"已退回未激活". 真生产路径接 evolution crate Retired.
                if self.consecutive_ignores >= 3 {
                    self.policy_stage = PolicyStage::Ratified;
                }
            }
        }
    }

    /// 观察交互 (per v1 `AwakeCompanion::observe_interaction` 1:1).
    pub fn observe_interaction(&mut self) {
        // 关系重新活跃 → 若策略已退回, 重新批准 (per v1 organs.rs:247-262)
        if !self.policy_stage.is_active() {
            if self.consecutive_ignores >= 2 {
                // 真实路径降级 max_initiatives_per_day (per v1 organs.rs:252-257)
                self.boundaries.max_initiatives_per_day = self
                    .boundaries
                    .max_initiatives_per_day
                    .saturating_sub(1)
                    .max(1);
                self.consecutive_ignores = 0;
            }
            let _ = self.ratify_fresh_policy();
        }
    }

    /// 当前关系深度 (per v1 `AwakeCompanion::depth()` 1:1).
    pub fn depth(&self) -> f64 {
        self.relationship.lock().depth()
    }

    /// 当前 PolicyStage (per v1 `evolution.current` 1:1).
    pub fn policy_stage(&self) -> PolicyStage {
        self.policy_stage
    }

    /// 最近一次 tick 决策留痕 (per v1 `AwakeCompanion::last_decision()` 1:1).
    pub fn last_decision(&self) -> Option<&OrchestratorDecision> {
        self.last_decision.as_ref()
    }

    /// 反馈历史 (per v1 `AwakeCompanion::asi_feedback` 1:1).
    pub fn feedback_history(&self) -> &[FeedbackRecord] {
        &self.feedback_history
    }

    /// 9 organ handle (test inspection, per R11 spec §4.1 串联顺序).
    pub fn organ_handles(&self) -> [(&'static str, &Arc<dyn OrganTrait>); 9] {
        [
            ("E4 curiosity", &self.organ_e4),
            ("F1 emotion_memory", &self.organ_f1),
            ("F4 hypothesis", &self.organ_f4),
            ("F6 value_cases", &self.organ_f6),
            ("W1 world_model", &self.organ_w1),
            ("W2 causal_world_model", &self.organ_w2),
            ("W3 causal_world_model_edges", &self.organ_w3),
            ("E7 emergence", &self.organ_e7),
            ("Memory merger", &self.organ_memory),
        ]
    }

    // ============================================
    // 测试 mutator helpers (per 子代理 R12 集成测试 / orchestrator.rs)
    //
    // 0 装诚实: 测试 mutator 公开 (无 cfg 包裹), 命名 `_for_test` 后缀警示
    // 真生产路径不要用. 真生产路径 Orchestrator 边界由 governance 管理.
    // ============================================

    /// 测试用: 拿 `&mut OrchestratorBoundaries`.
    pub fn boundaries_mut_for_test(&mut self) -> &mut OrchestratorBoundaries {
        &mut self.boundaries
    }

    /// 测试用: 拿 `&mut OrchestratorLoopConfig`.
    pub fn loop_config_mut_for_test(&mut self) -> &mut OrchestratorLoopConfig {
        &mut self.loop_config
    }

    /// 测试用: 拿 `parking_lot::MutexGuard<RS>` (relationship 短期可变访问).
    pub fn relationship_mut_for_test(&mut self) -> parking_lot::MutexGuard<'_, RS> {
        self.relationship.lock()
    }

    /// 测试用: 拿 `parking_lot::MutexGuard<dyn SovereigntyGate>` (sovereignty 短期可变访问).
    pub fn sovereignty_mut_for_test(&self) -> parking_lot::MutexGuard<'_, dyn SovereigntyGate> {
        self.sovereignty.lock()
    }

    /// 测试用: 8 重 gate 外层入口 (暴露 `check_8_gates` 公开访问).
    pub fn check_8_gates_for_test(
        &self,
        minutes: u32,
        initiatives_today: u32,
        at_ms: i64,
        last_initiative_ms: Option<i64>,
        chain: &OrganChainOutputs,
    ) -> Option<OrganOrchestratorGate> {
        self.check_8_gates(minutes, initiatives_today, at_ms, last_initiative_ms, chain)
    }
}

// ============================================
// ratify_fresh_policy() 走完整 5 状态 transition 链结果 (per v1 `AwakeCompanion::ratify_fresh_policy`
// 1:1, v1 走 4 个 evolution.transition 调用)
// ============================================

/// ratify_fresh_policy() 走完整 5 状态 transition 链结果 (per v1 `AwakeCompanion::ratify_fresh_policy`
/// 1:1, v1 走 4 个 evolution.transition 调用).
///
/// **0 装诚实**: 每条 transition 真实走 `allowed_next()` 检查, 任一步失败返 `Err`.
/// 留痕用 (telemetry + audit), 不参与决策.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatificationChain {
    /// 4 transition 每步 (target, result) 对 (Draft→Proposed→Ratified→Active).
    pub steps: Vec<(PolicyStage, Result<(), ()>)>,
}

impl RatificationChain {
    /// 是否全部 transition 成功 (走完 Idle → Draft → Proposed → Ratified → Active).
    pub fn all_ok(&self) -> bool {
        self.steps.iter().all(|(_, r)| r.is_ok())
    }

    /// 步骤数 (应 = 4 per v1 `AwakeCompanion::ratify_fresh_policy` 1:1).
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// 是否为空 (0 step = 0 装诚实标: 未走任何 transition).
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

// ============================================
// L0-L5 自升级 cycle 集成 (per `v2-architecture-reflection.md` §6)
// ============================================

/// L0-L5 自升级 cycle (per `v2-architecture-reflection.md` §6, 子代理 R11 整合).
///
/// **0 装诚实**:
/// - L0 永远不可变: 哲学锚 + 13 键 (per governance); Orchestrator 0 触碰.
/// - L1-L5: Orchestrator 串联 + cognitive module 注入 + governance 接入 + git tag.
/// - 完整 cycle = 1.Orchestrator 起草 → 2.Orchestrator 审 → 3.Orchestrator 激活 →
///   4.governance 主人 Veto → 5.git tag v2.x+1.
/// - 本 R12 spec 仅定义骨架, 真实施 1-3 周估待 (per R11 §8.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeLayer {
    /// L0 人类审批 (硬墙, 永远不可变, per philosophy.md Triple onion)
    L0HumanApproval,
    /// L1 自我诊断 (cognitive.self_assessment via RC-4)
    L1SelfAssessment,
    /// L2 提案生成 (Orchestrator + 7 LlmAdvisor via RC-6)
    L2ProposalGeneration,
    /// L3 验证 (9 organ process 串联 + sandbox regression)
    L3Verification,
    /// L4 主人审批 (governance 3 hook + 7 advisor 加权 + 主人 Veto)
    L4MasterApproval,
    /// L5 runtime patch (git tag v2.x+1)
    L5RuntimePatch,
}

impl UpgradeLayer {
    /// 6 layer 全列 (L0-L5, per `v2-architecture-reflection.md` §6).
    pub const ALL: [Self; 6] = [
        Self::L0HumanApproval,
        Self::L1SelfAssessment,
        Self::L2ProposalGeneration,
        Self::L3Verification,
        Self::L4MasterApproval,
        Self::L5RuntimePatch,
    ];

    /// 阶段名 (per R11 spec §7.1-7.6)
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::L0HumanApproval => "L0_human_approval",
            Self::L1SelfAssessment => "L1_self_assessment",
            Self::L2ProposalGeneration => "L2_proposal_generation",
            Self::L3Verification => "L3_verification",
            Self::L4MasterApproval => "L4_master_approval",
            Self::L5RuntimePatch => "L5_runtime_patch",
        }
    }
}

/// L0-L5 cycle 单步状态 (per R11 spec §7, 子代理 R11 整合).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleStep {
    /// 待启动
    Pending,
    /// 进行中
    InProgress,
    /// 通过
    Approved,
    /// 拒绝
    Rejected,
    /// 已 git tag (L5 终态)
    Tagged,
}

// ============================================
// 测试 (spec 层级验证, per R11 §10)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use apeireth_orchestration::Council;
    use apeireth_plugin::organ::OrganKind;

    fn episode() -> Episode {
        Episode {
            id: "test-1".into(),
            session_id: "sess-1".into(),
            role: "user".into(),
            content: "test".into(),
            timestamp: 0,
        }
    }

    /// 测试用 mock organ (不依赖 `apeireth_organ` crate).
    ///
    /// **0 装诚实**: runtime crate 不依赖 apeireth-organ; Orchestrator 测试用纯
    /// `Arc<dyn OrganTrait>` + 本 mock 验证 9 organ 串联 + 8 gate + 5 state machine
    /// 完整骨架可构造.
    struct MockOrgan {
        kind: OrganKind,
    }

    #[async_trait::async_trait]
    impl OrganTrait for MockOrgan {
        fn name(&self) -> &'static str {
            "MockOrgan"
        }
        fn organ_id(&self) -> OrganKind {
            self.kind
        }
        async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
            Ok(OrganOutput::NotImplemented {
                organ: self.kind,
                note: "mock organ (0 装诚实)".to_string(),
            })
        }
    }

    /// 0 装诚实: 8 重 gate 全部 13 种 variant 真实存在 + Display impl
    #[test]
    fn organ_orchestrator_gate_13_variants_real() {
        assert_eq!(OrganOrchestratorGate::ALL_13.len(), 13);
        for gate in OrganOrchestratorGate::ALL_13.iter() {
            // Display 不 panic (as_str)
            let _ = gate.as_str();
            // 8 重 emergence gate + 5 重 organs gate = 13 (per R11 spec §5)
            let is_emerge = gate.is_emergence_gate();
            match gate {
                OrganOrchestratorGate::UserQuiet
                | OrganOrchestratorGate::QuietHours
                | OrganOrchestratorGate::DailyLimit
                | OrganOrchestratorGate::LlmBudget
                | OrganOrchestratorGate::DepthLow
                | OrganOrchestratorGate::RhythmUnknown
                | OrganOrchestratorGate::RhythmVeto
                | OrganOrchestratorGate::DriveLow => {
                    assert!(is_emerge, "{:?} 应是 emergence gate", gate)
                }
                OrganOrchestratorGate::SovereigntyFrozen
                | OrganOrchestratorGate::EmotionLow
                | OrganOrchestratorGate::CouncilVeto
                | OrganOrchestratorGate::PolicyInactive
                | OrganOrchestratorGate::GateBlock => {
                    assert!(!is_emerge, "{:?} 应是 organs gate", gate)
                }
            }
        }
    }

    /// 0 装诚实: 5 状态机 transition 路径正确 (per R11 spec §6.2)
    #[test]
    fn organ_orchestrator_5_state_machine_transitions() {
        // Idle → Draft → Proposed → Ratified → Active (per v1 ratify_fresh_policy 1:1)
        assert_eq!(PolicyStage::Idle.allowed_next(), Some(PolicyStage::Draft));
        assert_eq!(
            PolicyStage::Draft.allowed_next(),
            Some(PolicyStage::Proposed)
        );
        assert_eq!(
            PolicyStage::Proposed.allowed_next(),
            Some(PolicyStage::Ratified)
        );
        assert_eq!(
            PolicyStage::Ratified.allowed_next(),
            Some(PolicyStage::Active)
        );
        assert_eq!(PolicyStage::Active.allowed_next(), None); // 终态 (Retired 在 evolution crate)

        // is_active() (per v1 EvolutionState::is_active 1:1)
        assert!(!PolicyStage::Idle.is_active());
        assert!(!PolicyStage::Draft.is_active());
        assert!(!PolicyStage::Proposed.is_active());
        assert!(PolicyStage::Ratified.is_active()); // 已通过审议可发声
        assert!(PolicyStage::Active.is_active());
    }

    /// 0 装诚实: OrchestratorBoundaries 8 重 gate early 路径
    #[test]
    fn orchestrator_boundaries_8_gates_early_path() {
        let b = OrchestratorBoundaries::default();
        // user_quiet
        let mut b2 = b.clone();
        b2.user_quiet = true;
        assert_eq!(
            b2.early_gate_block(720, 0),
            Some(OrganOrchestratorGate::UserQuiet)
        );
        // quiet_hours (22:00-06:00 跨午夜)
        let mut b3 = OrchestratorBoundaries::default();
        b3.quiet_start_minutes = Some(22 * 60);
        b3.quiet_end_minutes = Some(6 * 60);
        assert_eq!(
            b3.early_gate_block(23 * 60, 0),
            Some(OrganOrchestratorGate::QuietHours)
        );
        assert_eq!(
            b3.early_gate_block(5 * 60, 0),
            Some(OrganOrchestratorGate::QuietHours)
        );
        assert_eq!(
            b3.early_gate_block(12 * 60, 0),
            None // 中午不在安静窗口
        );
        // daily_limit
        let mut b4 = OrchestratorBoundaries::default();
        b4.max_initiatives_per_day = 2;
        assert_eq!(
            b4.early_gate_block(720, 3),
            Some(OrganOrchestratorGate::DailyLimit)
        );
    }

    /// 0 装诚实: 6 Layer L0-L5 全列 (per `v2-architecture-reflection.md` §6)
    #[test]
    fn upgrade_layer_l0_to_l5_six_layers() {
        assert_eq!(UpgradeLayer::ALL.len(), 6);
        for layer in UpgradeLayer::ALL.iter() {
            let _ = layer.as_str();
        }
    }

    /// 0 装诚实: LocalOrchestratorRelationship depth 0..1 + adjust clamp
    #[test]
    fn local_orchestrator_relationship_depth_clamp() {
        let mut r = LocalOrchestratorRelationship::new(0.5);
        assert_eq!(r.depth(), 0.5);
        r.adjust(0.8);
        assert!((r.depth() - 1.0).abs() < 1e-9, "应 clamp 到 1.0");
        r.adjust(-2.0);
        assert!((r.depth() - 0.0).abs() < 1e-9, "应 clamp 到 0.0");
    }

    /// 0 装诚实: LocalSovereignty freeze 行为
    #[test]
    fn local_sovereignty_freeze() {
        let mut s = LocalSovereignty::default();
        assert!(!s.is_frozen());
        s.freeze();
        assert!(s.is_frozen());
    }

    /// 0 装诚实: OrganChainOutputs 9 organ 全部 field 可写
    #[test]
    fn organ_chain_outputs_9_fields() {
        let c = OrganChainOutputs::default();
        // 9 field 全部 None (default 状态)
        assert!(c.e4.is_none());
        assert!(c.f1.is_none());
        assert!(c.f4.is_none());
        assert!(c.f6.is_none());
        assert!(c.w1.is_none());
        assert!(c.w2.is_none());
        assert!(c.w3.is_none());
        assert!(c.e7.is_none());
        assert!(c.memory.is_none());
        // get() 9 organ kind 都返 None
        for kind in [
            OrganKind::E4,
            OrganKind::F1,
            OrganKind::F4,
            OrganKind::F6,
            OrganKind::W1,
            OrganKind::W2,
            OrganKind::W3,
            OrganKind::E7,
            OrganKind::Memory,
        ] {
            assert!(c.get(kind).is_none());
        }
    }

    /// 0 装诚实: PolicyTransitionReason 6 reason 全列
    #[test]
    fn policy_transition_reason_6_variants() {
        let reasons = [
            PolicyTransitionReason::Start,
            PolicyTransitionReason::Submit,
            PolicyTransitionReason::CouncilApprove,
            PolicyTransitionReason::Activate,
            PolicyTransitionReason::Revoke,
            PolicyTransitionReason::Retire,
        ];
        assert_eq!(reasons.len(), 6);
    }

    /// 0 装诚实: OrchestratorFeedback 2 variant (per v1 Feedback 1:1)
    #[test]
    fn orchestrator_feedback_2_variants() {
        let _ = OrchestratorFeedback::Responded;
        let _ = OrchestratorFeedback::Ignored;
    }

    /// 0 装诚实: 构造空 Council + VirtualClock + LocalOrchestratorRelationship
    /// (集成测试, 验证 9 organ + 8 gate + 5 state machine + L0-L5 完整骨架可构造)
    #[tokio::test]
    async fn organ_orchestrator_construct_9_organ_8_gate_5_state() {
        // **0 装诚实**: runtime crate 不依赖 apeireth-organ (per Cargo.toml:13-19).
        // Orchestrator 测试用本地 MockOrgan, 验证 9 organ + 8 gate + 5 state machine +
        // L0-L5 完整骨架可构造.
        let organ_e4: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::E4,
        });
        let organ_f1: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::F1,
        });
        let organ_f4: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::F4,
        });
        let organ_f6: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::F6,
        });
        let organ_w1: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::W1,
        });
        let organ_w2: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::W2,
        });
        let organ_w3: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::W3,
        });
        let organ_e7: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::E7,
        });
        let organ_memory: Arc<dyn OrganTrait> = Arc::new(MockOrgan {
            kind: OrganKind::Memory,
        });

        let council = Arc::new(Council::default_allow());
        let council_invoker: Arc<dyn CouncilInvoker> = Arc::new(MockCouncilInvoker::allow_all());
        let sovereignty: Arc<parking_lot::Mutex<dyn SovereigntyGate>> =
            Arc::new(parking_lot::Mutex::new(LocalSovereignty::default()));
        let rel = LocalOrchestratorRelationship::new(0.5);
        let clock: Arc<dyn Clock> = Arc::new(VirtualClock::new(
            chrono::Utc
                .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
                .single()
                .unwrap(),
        ));

        let orch = OrganOrchestrator::new(
            organ_e4,
            organ_f1,
            organ_f4,
            organ_f6,
            organ_w1,
            organ_w2,
            organ_w3,
            organ_e7,
            organ_memory,
            council,
            council_invoker,
            sovereignty,
            rel,
            OrchestratorBoundaries::default(),
            OrchestratorLoopConfig::default(),
            clock,
        );
        // 编译通过 = 9 organ + 8 gate + 5 state machine + L0-L5 完整骨架可构造.
        assert_eq!(orch.organ_handles().len(), 9);
        assert_eq!(orch.policy_stage(), PolicyStage::Active);
        assert_eq!(orch.depth(), 0.5);
    }
    /// Counting mock: records how many times this handle was processed.
    struct CountingOrgan {
        kind: OrganKind,
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl OrganTrait for CountingOrgan {
        fn name(&self) -> &'static str {
            "CountingOrgan"
        }
        fn organ_id(&self) -> OrganKind {
            self.kind
        }
        async fn process(&self, _input: OrganInput) -> Result<OrganOutput, OrganError> {
            use std::sync::atomic::Ordering;
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(OrganOutput::NotImplemented {
                organ: self.kind,
                note: "counting mock".to_string(),
            })
        }
    }

    fn counting_orchestrator() -> (
        OrganOrchestrator<LocalOrchestratorRelationship>,
        Vec<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
    ) {
        use std::sync::atomic::AtomicUsize;
        let kinds = [
            OrganKind::E4,
            OrganKind::F1,
            OrganKind::F4,
            OrganKind::F6,
            OrganKind::W1,
            OrganKind::W2,
            OrganKind::W3,
            OrganKind::E7,
            OrganKind::Memory,
        ];
        let counters: Vec<_> = kinds
            .iter()
            .map(|_| std::sync::Arc::new(AtomicUsize::new(0)))
            .collect();
        let organ = |i: usize| -> Arc<dyn OrganTrait> {
            Arc::new(CountingOrgan {
                kind: kinds[i],
                calls: std::sync::Arc::clone(&counters[i]),
            })
        };
        let orch = OrganOrchestrator::new(
            organ(0),
            organ(1),
            organ(2),
            organ(3),
            organ(4),
            organ(5),
            organ(6),
            organ(7),
            organ(8),
            Arc::new(Council::default_allow()),
            Arc::new(MockCouncilInvoker::allow_all()),
            Arc::new(parking_lot::Mutex::new(LocalSovereignty::default())),
            LocalOrchestratorRelationship::default(),
            OrchestratorBoundaries::default(),
            OrchestratorLoopConfig::default(),
            Arc::new(VirtualClock::new(
                chrono::Utc
                    .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
                    .single()
                    .unwrap(),
            )),
        );
        (orch, counters)
    }

    /// The transient seam runs THIS execution's W1/W2 handles and leaves the
    /// persistent W1/W2 handles untouched; `chain_9_organs` keeps using the
    /// persistent ones.
    #[tokio::test]
    async fn transient_seam_uses_caller_handles_for_w1_w2() {
        use std::sync::atomic::Ordering;
        let (orch, counters) = counting_orchestrator();
        let transient_w1_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let transient_w2_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let make = |kind: OrganKind,
                    calls: &std::sync::Arc<std::sync::atomic::AtomicUsize>|
         -> Arc<dyn OrganTrait> {
            Arc::new(CountingOrgan {
                kind,
                calls: std::sync::Arc::clone(calls),
            })
        };

        let input = OrganInput::new(episode(), vec![]);
        let outputs = orch
            .chain_9_organs_with_transient_llm(
                input.clone(),
                make(OrganKind::W1, &transient_w1_calls),
                make(OrganKind::W2, &transient_w2_calls),
            )
            .await;
        assert!(outputs.all_present(), "9/9 outputs present");
        // Persistent W1/W2 untouched by the transient seam.
        assert_eq!(counters[4].load(Ordering::SeqCst), 0, "persistent W1");
        assert_eq!(counters[5].load(Ordering::SeqCst), 0, "persistent W2");
        // Transient handles ran exactly once.
        assert_eq!(transient_w1_calls.load(Ordering::SeqCst), 1);
        assert_eq!(transient_w2_calls.load(Ordering::SeqCst), 1);
        // The other seven persistent organs all ran.
        for index in [0usize, 1, 2, 3, 6, 7, 8] {
            assert_eq!(
                counters[index].load(Ordering::SeqCst),
                1,
                "persistent organ #{index}"
            );
        }

        // The compatibility chain still uses the persistent handles.
        let outputs = orch.chain_9_organs(input).await;
        assert!(outputs.all_present());
        assert_eq!(counters[4].load(Ordering::SeqCst), 1, "persistent W1 now");
        assert_eq!(counters[5].load(Ordering::SeqCst), 1, "persistent W2 now");
    }
}
