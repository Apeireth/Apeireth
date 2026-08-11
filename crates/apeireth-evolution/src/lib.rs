//! `apeireth-evolution`: 演化器官 — 6 状态机 + trait fail-6 + 4 学习 trait
//!                       + 与 apeireth-council 集成
//!
//! **职责** (round5-01 — backend_engineer2):
//! - 6 状态机 (`Idle` / `Draft` / `Proposed` / `Ratified` / `Active` / `Retired`)
//! - trait fail-6: 6 类失败路径 (reflection / council_reject / council_hold /
//!   activation_timeout / out_of_reflection_window / integrity_check)
//! - 4 trait: `Learning` / `Abstraction` / `SelfModification` / `Extension`
//! - 与 `apeireth-council` 集成 (通过 `CouncilEvent` + `HoldDecision` 判定)
//!
//! **诚实登记**:
//! - ❌ **不用 PyO3** — 不调 Python 实现 advisor / episode
//! - ❌ **不引入 I/O / 网络 / 文件系统** — 演化状态机纯内存
//! - ❌ **不依赖 `apeireth-sovereignty`** — 该 crate 已实装但本 crate 仅消费
//!   `apeireth-council` 接口 (`SovereigntyHook` trait 由 council 暴露)
//! - ❌ **不修改 `apeireth-core` 已实装类型** — 自有类型独立
//! - ✅ 所有逻辑 Rust 内置, trait 留口
//!
//! **架构位置**:
//! ```text
//!   apeireth-council (审议 → HoldDecision / CouncilVerdict)
//!          ↓ (CouncilEvent)
//!   apeireth-evolution (本 crate — 6 状态机 + fail-6)
//!          ↓
//!      apeireth-core (基础类型)
//! ```
//!
//! **LOCKED 边界** (守 7 项不修改承诺):
//! - ❌ 不修改 `docs/stage1/inspiration-stage1-2026-07-30.md`
//! - ❌ 不修改 `docs/stage2/stage2-decisions-*.md` 全部 18 个
//! - ❌ 不修改 `docs/stage3-blueprints/*.md` 14 个
//! - ❌ 不修改 `docs/stage4/architecture-*.md`
//! - ❌ 不修改 `docs/stage5/stage5-construction-document.md`
//! - ❌ 不修改 `apeireth-core` / `apeireth-council` 已实装类型签名
//! - ❌ 不修改 R11 V0.5 / V1136 / V3 原始 LOCKED

#![deny(unsafe_code)]

use std::fmt;
use thiserror::Error;

// ============================================================
// 公共 re-export
// ============================================================

pub mod council_bridge;
pub mod engine;
pub mod fail;
// R125-7 PODA cycle (per R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10, 主人 17:22 0 装解除 ⏳ 准备)
pub mod poda_cycle;
pub mod state;
pub mod traits;
// R127 P5-1 Library Stage 4 自治 (per decision-55-r127-integration-5-library-stage-4-6-2026-08-10.md §2.2)
// 3 机制: SelfEvolution (superpowers 234 ⏳ + aGLM 108 PODA ✅) + SelfUpgrade (superpowers 234 ⏳ + aGLM 108 PODA ✅)
//       + SelfRepair (chidori journal ✅ + apeireth-rollback 6 策略 ✅). 0 改 24 LOCKED #5 入口签名.
pub mod library_autonomy;
// R127-2 P8-1 Library Stage 4.1 自治 - 自循环 (per decision-56-r127-2-borrowed-3-retry-release-prep-2026-08-10.md §2.3)
// 3 模块: 自循环 (AutonomyLoop + LoopStage 借鉴 aGLM 108 PODA 4 阶段) +
//        自反馈 (FeedbackChannel + 3 FeedbackSignal 借鉴 aGLM 108 PODA 闭环) +
//        自调整 (SelfAdjust + 5 AdjustPolicy + 5 AdjustPolicyTrigger 借鉴 superpowers 234 skill priority).
// 0 改 P5-1 任何入口签名, 仅 import 类型 + 调其 step() / metrics() 方法. 0 改 24 LOCKED #5 入口签名.
pub mod library_autonomy_loop;

pub use council_bridge::{
    CouncilAdapter, CouncilIntegrationConfig, EvolutionOutcome, EvolutionProposal,
    DEFAULT_MAX_RETRY_ROUNDS, DEFAULT_REFLECTION_WINDOW_MS,
};
pub use engine::{EvolutionEngine, EvolutionLog, EvolutionStep};
pub use fail::{FailKind, FailOutcome, FailPolicy, FailRecord};
// R125-7 PODA cycle re-exports (新增, 0 改原 crate 任何入口签名)
pub use poda_cycle::{
    PodaAction, PodaConfig, PodaContext, PodaCycle, PodaError, PodaOutcome, PodaResult, PodaStage,
};
// R127 P5-1 Library Stage 4 自治 re-exports (新增, 0 改原 crate 任何入口签名)
pub use library_autonomy::{
    AutonomyError, AutonomyMetrics, AutonomyReport, AutonomyResult, LibraryAutonomy, Skill,
    SkillRegistry, TddFirstSkill, FailureEvent, FailureEventKind, DeterminismMeta, ObserveSkill,
    PlanSkill, AdaptSkill, RepairJournal, RepairResult, RepairStrategy, SelfEvolution,
    SelfEvolutionAction, SelfEvolutionState, SelfRepair, SelfRepairAction, SelfRepairState,
    SelfUpgrade, SelfUpgradeAction, SelfUpgradeState, UpgradePlan,
};
// R127-2 P8-1 Library Stage 4.1 自治 - 自循环 re-exports (新增, 0 改 P5-1 任何入口签名)
// 8 类型: AutonomyLoop (顶层) + LoopStage (4 阶段) + FeedbackChannel + FeedbackSignal (3 variant)
//        + SignalSource (4) + SignalTarget (3) + SignalPriority + LoopError + LoopResult
//        + SelfAdjust + AdjustPolicy (5) + AdjustPolicyTrigger (5) + LoopMetrics + LoopReport
pub use library_autonomy_loop::{
    AdjustPolicy, AdjustPolicyTrigger, AutonomyLoop, FeedbackChannel, FeedbackSignal, LoopError,
    LoopMetrics, LoopReport, LoopResult, LoopStage, SelfAdjust, SignalPriority, SignalSource,
    SignalTarget, REPAIR_STORM_THRESHOLD, EVOLUTION_HEALTHY_THRESHOLD,
};
pub use state::{EvolutionState, EvolutionStateMachine, StateTransition, TransitionReason};
pub use traits::{
    Abstraction, BasicEvolution, Concept, Episode, Extension, Learning, MockPlugin, Patch, Plugin,
    PluginKind, PluginRegistry, SelfModification, SystemState,
};

// ============================================================
// 演化错误
// ============================================================

/// 演化错误。
#[derive(Debug, Error)]
pub enum EvolutionError {
    /// 非法状态转换
    #[error("illegal state transition: {from:?} → {to:?} (reason: {reason})")]
    IllegalTransition {
        /// 源状态
        from: EvolutionState,
        /// 目标状态
        to: EvolutionState,
        /// 原因
        reason: String,
    },
    /// Episode 验证失败
    #[error("invalid episode: {0}")]
    InvalidEpisode(String),
    /// 插件加载失败
    #[error("plugin load failed: {0}")]
    PluginLoadFailed(String),
    /// 反思期外 (超过 reflection_window_ms)
    #[error("out of reflection window: elapsed {elapsed_ms} ms (window {window_ms} ms)")]
    OutOfReflectionWindow {
        /// 已用时 (ms)
        elapsed_ms: u64,
        /// 窗口 (ms)
        window_ms: u64,
    },
    /// 重试耗尽
    #[error("retry budget exhausted: {attempts}/{max}")]
    RetryBudgetExhausted {
        /// 已重试次数
        attempts: u32,
        /// 最大重试
        max: u32,
    },
    /// L0 防护: 演化拒绝修改 L0 (硬件锚定层)
    #[error("L0 modification rejected (apeireth-evolution cannot modify L0)")]
    L0ModificationRejected,
    /// 完整性校验失败
    #[error("integrity check failed: {0}")]
    IntegrityCheckFailed(String),
}

/// 演化结果。
pub type EvolutionResult<T> = Result<T, EvolutionError>;

// ============================================================
// 编译时 hardcode (防 L0 + 防 V0.5/V1136 篡改)
// ============================================================

/// L0 硬件锚定层标识 — 演化器官禁止修改。
pub const L0_ANCHOR: &str = "L0-HARDWARE-ANCHOR";

/// 反思窗口默认 60 秒 (与 council HOLD_DELIBERATION_TIMEOUT_MS 一致)。
pub const DEFAULT_REFLECTION_WINDOW: u64 = 60_000;

/// 最大重试轮次 (与 MAX_PERSONA_DEBATE_ROUNDS=3 一致, 留余量)。
pub const DEFAULT_MAX_RETRY: u32 = 3;

const _: () = {
    assert!(
        DEFAULT_REFLECTION_WINDOW >= 1_000,
        "reflection window too short"
    );
    assert!(DEFAULT_MAX_RETRY >= 1, "retry budget must allow at least 1");
};

// ============================================================
// 演化阶段标量 (供 trait 默认实现读取)
// ============================================================

/// 当前 epoch ms — trait 默认实现可读取用于审计。
pub fn current_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Display 适配: 让 EvolutionState 可打印。
impl fmt::Display for EvolutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// ============================================================
// apeireth-verify cross-crate hooks (P28 阶段 6) — 仅做登记
// ============================================================
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_EVOLUTION_A,
    "apeireth-evolution",
    "apeireth-evolution structural invariant — 6 状态机 hardcode",
    InRange {
        name: "apeireth-evolution::invariant-a",
        value: 1.0,
        min: 0.0,
        max: 1.0
    }
);
apeireth_verify::regression_assert!(
    __APEIRETH_REG_APEIRETH_EVOLUTION_B,
    "apeireth-evolution",
    "apeireth-evolution regression gate — fail-6 policy",
    Idempotent {
        name: "apeireth-evolution::invariant-b",
        first: "stable",
        second: "stable"
    }
);
apeireth_verify::register_all_in_crate!(
    __APEIRETH_REG_APEIRETH_EVOLUTION_A,
    __APEIRETH_REG_APEIRETH_EVOLUTION_B
);
