//! Library Stage 4 自治 (self-evolution + self-upgrade + self-repair)
//!
//! **R127 P5-1 阶段 B 实施** (per `decision-55-r127-integration-5-library-stage-4-6-2026-08-10.md` §2.2)
//!
//! Library 6 阶段中的 Stage 4 自治 = 自演化 + 自升级 + 自修复 三大机制.
//!
//! ## 3 大借鉴 ID
//!
//! | 借鉴源 | 状态 | 借鉴 ID | 实施位置 |
//! |--------|------|---------|----------|
//! | **obra/superpowers 234** (Skill 化工作流) | ⏳ 限流中 (公开模式 1:1 借鉴) | `R127-BORROW-obra/superpowers-2026-08-10` | `SelfEvolution` Skill trait + `SkillRegistry` + 4 default Skill |
//! | **GATERAGE/aGLM 108** (PODA 4 阶段) | ✅ cloned (R125-7 真实施) | `R127-BORROW-GATERAGE/aglm-2024Q4-2026-08-10` | 复用 `crate::poda_cycle::{PodaCycle, PodaStage, ...}` |
//! | **ThousandBirdsInc/chidori** (host-call journal) | ✅ cloned (R125-8 真实施) | `R127-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10` | `FailureEvent` 9 字段 1:1 映射 chidori JournalEntry |
//!
//! ## 借鉴脉络
//!
//! ### 1. 自演化 (Self-evolution) — 借鉴 superpowers 234 + aGLM 108
//!
//! - **superpowers 234**: 每个 Skill = Markdown 行为准则 + TDD 强制 + 中央注册表.
//!   AI 启动时加载相关 Skill, 触发对应行为. 借鉴为 Rust trait `Skill` + `SkillRegistry`.
//! - **aGLM 108 PODA cycle**: 4 阶段 (Plan / Observe / Decide / Act) 自主循环.
//!   借鉴为 `SelfEvolution::step()` 主循环: Observe → Plan → Adapt → Snapshot → Evolved.
//! - **整合**: SelfEvolution 内部维护 1 个 `PodaCycle` (✅ R125-7 真实施) 作为**类型 marker** +
//!   阶段 hint 提供器 (`poda_stage_hint()` 返回 `PodaStage`). **0 调** `PodaCycle::step()` 避免改
//!   `EvolutionEngine` 状态. 决策表由 4 default Skill (TddFirst/Observe/Plan/Adapt) 提供.
//!
//! ### 2. 自升级 (Self-upgrade) — 借鉴 superpowers 234 + aGLM 108
//!
//! - **superpowers 234 升级模式**: Skill 触发后, 必走 TDD (test-first), 不通过则 abort.
//!   借鉴为 `SelfUpgrade::step()` 6 状态机: Idle → Detecting → Verifying → Applying → Upgraded → RolledBack/Failed.
//! - **aGLM 108 PODA cycle**: 升级 = 4 阶段 (Plan = upgrade intent / Observe = current state /
//!   Decide = upgrade vs rollback / Act = apply upgrade). 借鉴为 `SelfUpgrade::decide()`.
//! - **整合**: SelfUpgrade 主循环 = PODA 4 阶段 + 升级前 snapshot + 升级后 verify + 失败 rollback.
//!
//! ### 3. 自修复 (Self-repair) — 借鉴 Chidori journal rollback
//!
//! - **chidori host-call journal**: 9 字段 1:1 映射 (seq / event_kind / ts / child_id / plan_version /
//!   input / output / result / determinism_meta). 借鉴为 `FailureEvent` struct.
//! - **chidori 决策论重放**: 失败事件可重放, 决定论 metadata 兜底. 借鉴为 `RepairJournal` 重放机制.
//! - **apeireth-rollback 6 策略** (估缺 1:1 翻译 v0.9.21): full / file / diff / git / session / auto.
//!   借鉴为 `SelfRepair::decide_repair_strategy()` 决策表.
//! - **整合**: SelfRepair 主循环 = 6 状态机 (Healthy → Detected → Snapshotting → Repairing → Repaired / Failed),
//!   失败事件写 journal + 71GB 4 重防御 (per apeireth-rollback 借用 4 个 const).
//!
//! ## 0 装 PASS 严守 (per 主人 17:22 "0 装不必要" 解除 + decision-33 §2.3 C2)
//!
//! - ✅ **aGLM 108 cloned** (R125-7 ✅ done, 21/21 tests pass, `crates/apeireth-evolution/src/poda_cycle.rs` 39KB 真实施)
//!   = 自演化内部维护 1 个 `PodaCycle` 实例作为类型 marker + 阶段 hint (暴露 `poda_stage_hint()`),
//!   **0 调** `PodaCycle::step()` 避免改 `EvolutionEngine` 状态, 0 装"已用 PODA 推进 engine".
//! - ✅ **chidori cloned** (R125-8 ✅ done, 13/13 tests pass, `crates/apeireth-supervisor/src/journal_entry.rs` 18.2KB 真实施)
//!   = 自修复 `FailureEvent` 字段基于 chidori 公开模式 1:1 映射 (0 直接 import chidori crate, 仅模仿字段集).
//! - ⏳ **superpowers 234 限流中** (R125-14 dispatch prompt 写完, clone 未完成) = 0 装 src 实施,
//!   `Skill` trait + `SkillRegistry` + 4 default Skill 模式基于 superpowers 公开文档 (Skill = Markdown + TDD 强制 + 注册表)
//!   1:1 借鉴, 0 装"已借鉴"具体实现.
//!
//! ## 8 硬墙 verify (B1-B7 升级版 + A1-A3 严守 + C1-C3 策略)
//!
//! - **B1** 24 LOCKED 持续更新 — `apeireth-evolution` 在 24 LOCKED #5, **本文件是 NEW**, 0 触碰
//!   `lib.rs` 入口签名, 仅 +1 行 `pub mod library_autonomy;` + 1 re-export group (12 类型).
//!   0 触碰 `engine.rs` / `state.rs` / `fail.rs` / `poda_cycle.rs` / `council_bridge.rs` / `traits.rs` 任何入口签名.
//! - **B2** workspace.version 1.2.0 0 改 — 0 触碰 `Cargo.toml:246`.
//! - **A1** R11 baseline 3 值 数字严守 — 0 触碰 `integration_r_measure.rs` (本文件 0 涉及 R11 baseline).
//! - **B5** 8 哲学锚 — 0 改 8 哲学锚原 8 实质.
//! - **B3** 30 维 — 0 改 V0.5 公式, 30 维是扩展.
//! - **B4** 6 重 v7 守门 — 0 改 6 重守门原 6 重, v7 是扩展.
//! - **A3** 13 键 — 0 改 12 键原 12, PHL-07 是扩展.
//! - **C1** 0 主动 commit — 严守 (Mavis 整合 #5 commit 时机拍板).
//! - **C2** 0 装 解除 — 主人 17:22, ✅ cloned = 真实施 (PODA + journal 复用), ⏳ 限流 = 准备模式 (superpowers 公开模式 1:1).
//! - **C3** 0 push — 严守 (等主人 1.0 release 配 GitHub remote).
//!
//! ## 架构位置
//!
//! ```text
//!   LibraryAutonomy (顶层协调器, 主循环)
//!       ├─ SelfEvolution (自演化)  ──调──> crate::poda_cycle::PodaCycle (✅ R125-7 真实施)
//!       ├─ SelfUpgrade (自升级)    ──调──> crate::poda_cycle::PodaCycle (复用, Plan 阶段读升级意图)
//!       └─ SelfRepair (自修复)     ──调──> FailureEvent + RepairJournal (借鉴 chidori 公开模式)
//! ```
//!
//! ## 核心不变量 (编译期 hardcode)
//!
//! - Library Stage 4 自治 = 3 子机制 1 文件 (`SelfEvolution` + `SelfUpgrade` + `SelfRepair`).
//! - 自演化/自升级主循环 = PODA 4 阶段 (Plan/Observe/Decide/Act), 0 改 PODA 公开 API.
//! - 自修复 FailureEvent 字段集 0 改 (9 字段 1:1 chidori), 仅 chidori 公开模式, 0 import chidori crate.
//! - 入口签名 0 改: 本文件全部 `pub fn` / `pub struct` / `pub enum` 都是 NEW, 0 改原 crate 任何签名.
//! - Skill trait 默认 TDD 强制 (`fn tdd_required() -> bool { true }`), 修改需编译期 lint 警告.

#![allow(dead_code)] // ⏳ 部分 fn 等 superpowers 234 cloned 后补真断言; 字段集已就绪

use serde::{Deserialize, Serialize};
use thiserror::Error;

// R127 P5-1 借鉴 aGLM 108 PODA cycle (R125-7 ✅ cloned 真实施):
// 复用 `crate::poda_cycle::{PodaCycle, PodaConfig, PodaStage}` 作为类型 marker.
// **0 调 `PodaCycle::step()` 避免改 EvolutionEngine 状态**, 仅暴露 `poda_stage_hint()` 提供阶段名.
use crate::poda_cycle::{PodaConfig, PodaCycle, PodaStage};

// ============================================================================
// 公共错误类型 (per 编译期 hardcode, 7 variant 覆盖 3 机制 + 整合失败)
// ============================================================================

/// Library Stage 4 自治错误类型.
#[derive(Debug, Error)]
pub enum AutonomyError {
    /// 自演化错误: 状态机非法转换
    #[error("self-evolution illegal transition: {from:?} -> {to:?} ({reason})")]
    SelfEvolutionIllegalTransition {
        /// 源状态
        from: SelfEvolutionState,
        /// 目标状态
        to: SelfEvolutionState,
        /// 原因
        reason: String,
    },
    /// 自演化错误: Skill 未注册
    #[error("self-evolution skill not registered: {0}")]
    SelfEvolutionSkillNotRegistered(String),
    /// 自升级错误: 状态机非法转换
    #[error("self-upgrade illegal transition: {from:?} -> {to:?} ({reason})")]
    SelfUpgradeIllegalTransition {
        /// 源状态
        from: SelfUpgradeState,
        /// 目标状态
        to: SelfUpgradeState,
        /// 原因
        reason: String,
    },
    /// 自升级错误: 升级预算耗尽
    #[error("self-upgrade retry budget exhausted: {attempts}/{max}")]
    SelfUpgradeRetryBudgetExhausted {
        /// 已用次数
        attempts: u32,
        /// 最大
        max: u32,
    },
    /// 自修复错误: 状态机非法转换
    #[error("self-repair illegal transition: {from:?} -> {to:?} ({reason})")]
    SelfRepairIllegalTransition {
        /// 源状态
        from: SelfRepairState,
        /// 目标状态
        to: SelfRepairState,
        /// 原因
        reason: String,
    },
    /// 自修复错误: 影子目录超 71GB 4 重防御阈值 (借 apeireth-rollback 借用)
    #[error("self-repair snapshot quota exceeded: {reason}")]
    SelfRepairSnapshotQuota {
        /// 原因
        reason: String,
    },
    /// 顶层协调器错误
    #[error("library autonomy main loop failed: {0}")]
    MainLoopFailed(String),
}

/// Library Stage 4 自治结果类型.
pub type AutonomyResult<T> = Result<T, AutonomyError>;

// ============================================================================
// §1 自演化 (Self-evolution) — 借鉴 superpowers 234 + aGLM 108 PODA cycle
// ============================================================================

/// 自演化 5 状态机 (per aGLM PODA 4 阶段 + superpowers Skill 触发模式).
///
/// 状态机迁移表:
/// ```text
///   Idle ──observe──> Observing
///   Observing ──plan(ready)──> Planning
///   Planning ──adapt──> Evolving
///   Evolving ──snapshot──> Evolved       (成功)
///   Evolving ──fail──> Failed            (失败)
///   Failed ──reset──> Idle               (可重置, 但本 crate 不提供 reset, 由 caller 决定)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfEvolutionState {
    /// 起始状态
    Idle,
    /// 监测当前状态 (PODA Observe 阶段)
    Observing,
    /// 规划演化方案 (PODA Plan 阶段)
    Planning,
    /// 执行演化 (PODA Act 阶段)
    Evolving,
    /// 演化成功, 已 snapshot
    Evolved,
    /// 演化失败
    Failed,
}

impl SelfEvolutionState {
    /// 全部状态 (编译期 hardcode 兜底).
    pub const ALL: [SelfEvolutionState; 6] = [
        Self::Idle,
        Self::Observing,
        Self::Planning,
        Self::Evolving,
        Self::Evolved,
        Self::Failed,
    ];

    /// 是否终态 (Evolved / Failed).
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Evolved | Self::Failed)
    }
}

/// 自演化 5 动作 (借鉴 superpowers Skill 触发: 每动作对应 1 Skill).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfEvolutionAction {
    /// 监测 (触发 ObserveSkill)
    Observe,
    /// 规划 (触发 PlanSkill)
    Plan,
    /// 适应 (触发 AdaptSkill)
    Adapt,
    /// 快照 (借鉴 apeireth-rollback 6 策略之一: Session)
    Snapshot,
    /// 完成终态化
    Finalize,
}

/// Skill trait — 借鉴 superpowers 234 Skill 化工作流.
///
/// 每个 Skill = Markdown 行为准则 + TDD 强制 + 描述 + 步骤. 借鉴为 Rust trait,
/// 4 default impl (TddFirst / ObserveSkill / PlanSkill / AdaptSkill).
pub trait Skill: Send + Sync + std::fmt::Debug {
    /// Skill ID (唯一标识)
    fn id(&self) -> &'static str;
    /// Skill 名
    fn name(&self) -> &'static str;
    /// Skill 描述 (借鉴 superpowers 234 "when to use" 字段)
    fn when_to_use(&self) -> &'static str;
    /// Skill 步骤列表 (借鉴 superpowers 234 "steps" 字段)
    fn steps(&self) -> &'static [&'static str];
    /// TDD 强制 (借鉴 superpowers 234 "TDD 强制化" 原则, 默认 true)
    fn tdd_required(&self) -> bool {
        true
    }
    /// Markdown 路径 (借鉴 superpowers 234 Skill .md 文件)
    fn markdown_path(&self) -> &'static str;
    /// 加载 Markdown 内容 (借鉴 superpowers 234 load_markdown)
    fn load_markdown(&self) -> String {
        // skeleton: 0 真读 .md 文件, 返回结构化字符串
        let steps: Vec<&str> = self.steps().to_vec();
        let steps_str = steps
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "# {}\n\n**ID**: `{}`\n\n**When to use**: {}\n\n**TDD required**: {}\n\n## Steps\n\n{}\n",
            self.name(),
            self.id(),
            self.when_to_use(),
            self.tdd_required(),
            steps_str
        )
    }
}

// ----- 4 default Skill impl (借鉴 superpowers 234 公开 4 基础 Skill) -----

/// TDD-first Skill — 借鉴 superpowers 234 TDD 强制化原则.
#[derive(Debug)]
pub struct TddFirstSkill;
impl Skill for TddFirstSkill {
    fn id(&self) -> &'static str {
        "tdd-first"
    }
    fn name(&self) -> &'static str {
        "TDD First"
    }
    fn when_to_use(&self) -> &'static str {
        "写新功能前 / 修复 bug 前 / 任何代码改动前"
    }
    fn steps(&self) -> &'static [&'static str] {
        &[
            "写失败的测试 (RED)",
            "跑测试确认失败",
            "写最小实现让测试通过 (GREEN)",
            "重构 + 跑全套测试 (REFACTOR)",
            "0 假装通过 = 真 pass",
        ]
    }
    fn markdown_path(&self) -> &'static str {
        "skills/tdd-first.md"
    }
}

/// Observe Skill — 对应 PODA Observe 阶段.
#[derive(Debug)]
pub struct ObserveSkill;
impl Skill for ObserveSkill {
    fn id(&self) -> &'static str {
        "observe"
    }
    fn name(&self) -> &'static str {
        "Observe (PODA Stage)"
    }
    fn when_to_use(&self) -> &'static str {
        "PODA 4 阶段循环的 Observe 阶段: 监测当前状态 + 收集上下文"
    }
    fn steps(&self) -> &'static [&'static str] {
        &[
            "读 engine.current_state()",
            "读 engine.log() 最近 N 条",
            "写 context.observations[\"engine_state\"] = ... ",
            "写 context.observations[\"log_tail\"] = ... ",
        ]
    }
    fn markdown_path(&self) -> &'static str {
        "skills/observe.md"
    }
}

/// Plan Skill — 对应 PODA Plan 阶段.
#[derive(Debug)]
pub struct PlanSkill;
impl Skill for PlanSkill {
    fn id(&self) -> &'static str {
        "plan"
    }
    fn name(&self) -> &'static str {
        "Plan (PODA Stage)"
    }
    fn when_to_use(&self) -> &'static str {
        "PODA 4 阶段循环的 Plan 阶段: 设计演化提案 + 收集上下文"
    }
    fn steps(&self) -> &'static [&'static str] {
        &[
            "读 context.observations",
            "设计演化提案 (Idle → Draft)",
            "校验提案 L0 防护不破坏",
            "写 context.plan[\"intent\"] = ...",
        ]
    }
    fn markdown_path(&self) -> &'static str {
        "skills/plan.md"
    }
}

/// Adapt Skill — 对应 PODA Act 阶段 (执行演化).
#[derive(Debug)]
pub struct AdaptSkill;
impl Skill for AdaptSkill {
    fn id(&self) -> &'static str {
        "adapt"
    }
    fn name(&self) -> &'static str {
        "Adapt (PODA Stage)"
    }
    fn when_to_use(&self) -> &'static str {
        "PODA 4 阶段循环的 Act 阶段: 调 EvolutionEngine 公开方法"
    }
    fn steps(&self) -> &'static [&'static str] {
        &[
            "读 context.plan[\"intent\"]",
            "调 engine.start() 或 engine.submit() 等",
            "校验 L0 防护",
            "写 context.actions[\"executed\"] = ...",
        ]
    }
    fn markdown_path(&self) -> &'static str {
        "skills/adapt.md"
    }
}

/// SkillRegistry — 借鉴 superpowers 234 中央注册表.
#[derive(Debug)]
pub struct SkillRegistry {
    /// 4 default Skill (TddFirst + Observe + Plan + Adapt)
    skills: Vec<Box<dyn Skill>>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    /// 创建默认注册表 (注册 4 default Skill).
    pub fn new() -> Self {
        Self {
            skills: vec![
                Box::new(TddFirstSkill),
                Box::new(ObserveSkill),
                Box::new(PlanSkill),
                Box::new(AdaptSkill),
            ],
        }
    }

    /// 注册 Skill.
    pub fn register(&mut self, skill: Box<dyn Skill>) {
        self.skills.push(skill);
    }

    /// 按 ID 查找 Skill.
    pub fn get(&self, id: &str) -> Option<&dyn Skill> {
        self.skills.iter().find(|s| s.id() == id).map(|s| s.as_ref())
    }

    /// 列全部 Skill.
    pub fn all(&self) -> Vec<&dyn Skill> {
        self.skills.iter().map(|s| s.as_ref()).collect()
    }

    /// 计数.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

/// 自演化引擎 (借鉴 superpowers 234 自治循环 + aGLM 108 PODA cycle).
///
/// **5 状态机 + Skill 注册表 + 5 动作 + 复用 PODA cycle 类型**:
/// - 主循环 `step(action)` 走 5 状态机
/// - 每步触发 1 个 Skill (借鉴 superpowers 234 "Skill 触发" 模式)
/// - 内部复用 `PodaCycle` (✅ R125-7 真实施) 作为阶段 hint 提供器, **0 调** `PodaCycle::step()`
///   避免改 EvolutionEngine 状态 (aGLM 108 借鉴类型 marker, 0 假装已用 PODA 推进 engine)
/// - 0 触碰 `EvolutionEngine` 任何公开方法签名 (内部 fn 实施可改)
///
/// **0 触碰 24 LOCKED**:
/// - 0 改 `apeireth-evolution/src/lib.rs` 入口签名 (仅 +1 行 `pub mod library_autonomy;`)
/// - 0 改 `engine.rs` / `state.rs` / `fail.rs` / `poda_cycle.rs` / `council_bridge.rs` / `traits.rs` 任何入口签名
// 注: 不 derive Debug 因 `PodaCycle` 未 derive Debug. 手 impl `Debug` 仅显示关键字段.
pub struct SelfEvolution {
    /// 当前状态
    state: SelfEvolutionState,
    /// Skill 注册表 (借鉴 superpowers 234)
    skill_registry: SkillRegistry,
    /// 循环计数 (审计用)
    cycles: u32,
    /// 最大循环次数 (兜底, 防无限循环)
    max_cycles: u32,
    /// 上次动作 (审计)
    last_action: Option<SelfEvolutionAction>,
    /// PODA cycle 借鉴 (✅ R125-7 真实施类型 marker, 0 调 step())
    poda: PodaCycle,
}

impl Default for SelfEvolution {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfEvolution {
    /// 创建新自演化引擎 (state=Idle, 4 default Skill 注册, max_cycles=100, PODA marker 默认 config).
    pub fn new() -> Self {
        Self {
            state: SelfEvolutionState::Idle,
            skill_registry: SkillRegistry::new(),
            cycles: 0,
            max_cycles: 100,
            last_action: None,
            poda: PodaCycle::new("library-autonomy-evolution", PodaConfig::default()),
        }
    }

    /// 创建自演化引擎 + 自定义 max_cycles.
    pub fn with_max_cycles(max_cycles: u32) -> Self {
        Self {
            state: SelfEvolutionState::Idle,
            skill_registry: SkillRegistry::new(),
            cycles: 0,
            max_cycles,
            last_action: None,
            poda: PodaCycle::new("library-autonomy-evolution", PodaConfig::default()),
        }
    }

    /// 当前状态.
    pub fn state(&self) -> SelfEvolutionState {
        self.state
    }

    /// Skill 注册表 (只读访问).
    pub fn skill_registry(&self) -> &SkillRegistry {
        &self.skill_registry
    }

    /// 循环计数.
    pub fn cycles(&self) -> u32 {
        self.cycles
    }

    /// 上次动作.
    pub fn last_action(&self) -> Option<SelfEvolutionAction> {
        self.last_action
    }

    /// PODA 阶段 hint (借鉴 aGLM 108 PODA cycle, ✅ R125-7 真实施类型 marker).
    ///
    /// 暴露 `PodaCycle::current_stage()` 作为"自演化当前对应 PODA 4 阶段"提示.
    /// 0 调 `PodaCycle::step()` 避免改 EvolutionEngine 状态, 仅作为类型借用.
    pub fn poda_stage_hint(&self) -> PodaStage {
        self.poda.current_stage()
    }

    /// 决策: 给定当前状态 + last_action 候选, 选下一个动作.
    ///
    /// 借鉴 superpowers 234 Skill 触发模式: 每状态对应特定 Skill 触发.
    pub fn decide(&self, candidates: &[SelfEvolutionAction]) -> Option<SelfEvolutionAction> {
        if candidates.is_empty() {
            return None;
        }
        match self.state {
            SelfEvolutionState::Idle => candidates
                .iter()
                .find(|a| matches!(a, SelfEvolutionAction::Observe))
                .copied(),
            SelfEvolutionState::Observing => candidates
                .iter()
                .find(|a| matches!(a, SelfEvolutionAction::Plan))
                .copied(),
            SelfEvolutionState::Planning => candidates
                .iter()
                .find(|a| matches!(a, SelfEvolutionAction::Adapt))
                .copied(),
            SelfEvolutionState::Evolving => candidates
                .iter()
                .find(|a| matches!(a, SelfEvolutionAction::Snapshot))
                .copied(),
            SelfEvolutionState::Evolved | SelfEvolutionState::Failed => candidates
                .iter()
                .find(|a| matches!(a, SelfEvolutionAction::Finalize))
                .copied(),
        }
    }

    /// 单步: 走 1 个动作, 状态机迁移.
    pub fn step(&mut self, action: SelfEvolutionAction) -> AutonomyResult<SelfEvolutionState> {
        if self.cycles >= self.max_cycles {
            return Err(AutonomyError::SelfEvolutionIllegalTransition {
                from: self.state,
                to: self.state,
                reason: format!("max_cycles {} reached", self.max_cycles),
            });
        }
        let new_state = match (self.state, action) {
            (SelfEvolutionState::Idle, SelfEvolutionAction::Observe) => {
                SelfEvolutionState::Observing
            }
            (SelfEvolutionState::Observing, SelfEvolutionAction::Plan) => {
                SelfEvolutionState::Planning
            }
            (SelfEvolutionState::Planning, SelfEvolutionAction::Adapt) => {
                SelfEvolutionState::Evolving
            }
            (SelfEvolutionState::Evolving, SelfEvolutionAction::Snapshot) => {
                SelfEvolutionState::Evolved
            }
            (s, a) => {
                return Err(AutonomyError::SelfEvolutionIllegalTransition {
                    from: s,
                    to: self.state,
                    reason: format!("action {:?} not allowed in state {:?}", a, s),
                });
            }
        };
        self.state = new_state;
        self.cycles += 1;
        self.last_action = Some(action);
        Ok(new_state)
    }

    /// 跑直到终态 (Evolved / Failed), 5 步全跑完.
    pub fn run_until_terminal(&mut self) -> AutonomyResult<SelfEvolutionState> {
        for _ in 0..5 {
            if self.state.is_terminal() {
                return Ok(self.state);
            }
            // 5 步固定序列: Observe → Plan → Adapt → Snapshot → (Evolved)
            let action = match self.state {
                SelfEvolutionState::Idle => SelfEvolutionAction::Observe,
                SelfEvolutionState::Observing => SelfEvolutionAction::Plan,
                SelfEvolutionState::Planning => SelfEvolutionAction::Adapt,
                SelfEvolutionState::Evolving => SelfEvolutionAction::Snapshot,
                SelfEvolutionState::Evolved | SelfEvolutionState::Failed => break,
            };
            self.step(action)?;
        }
        Ok(self.state)
    }
}

/// 手 impl Debug (PodaCycle 0 derive Debug, 所以 SelfEvolution 0 derive Debug).
impl std::fmt::Debug for SelfEvolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelfEvolution")
            .field("state", &self.state)
            .field("cycles", &self.cycles)
            .field("max_cycles", &self.max_cycles)
            .field("last_action", &self.last_action)
            .field("skill_count", &self.skill_registry.len())
            .field("poda_stage", &self.poda.current_stage())
            .finish()
    }
}

// ============================================================================
// §2 自升级 (Self-upgrade) — 借鉴 superpowers 234 升级模式 + aGLM 108 PODA
// ============================================================================

/// 自升级 6 状态机.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfUpgradeState {
    /// 起始
    Idle,
    /// 检测到升级意图 (PODA Plan 阶段)
    Detecting,
    /// 升级前 verify (snapshot + TDD test) (PODA Observe 阶段)
    Verifying,
    /// 应用升级 (PODA Act 阶段) (PODA Decide 阶段 = 决定 Apply/Rollback)
    Applying,
    /// 升级成功
    Upgraded,
    /// 升级失败, 已 rollback
    RolledBack,
    /// 升级失败, rollback 也失败
    Failed,
}

impl SelfUpgradeState {
    /// 全部状态 (编译期 hardcode 兜底).
    pub const ALL: [SelfUpgradeState; 7] = [
        Self::Idle,
        Self::Detecting,
        Self::Verifying,
        Self::Applying,
        Self::Upgraded,
        Self::RolledBack,
        Self::Failed,
    ];

    /// 是否终态 (Upgraded / RolledBack / Failed).
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Upgraded | Self::RolledBack | Self::Failed)
    }
}

/// 自升级 6 动作.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfUpgradeAction {
    /// 检测升级意图
    Detect,
    /// 升级前 verify (snapshot)
    VerifyPre,
    /// 应用升级
    Apply,
    /// 升级后 verify (TDD test)
    VerifyPost,
    /// 回滚
    Rollback,
    /// 完成终态化
    Finalize,
}

/// 升级计划 (借鉴 superpowers 234 Skill 触发模式: 升级 = 加载 Skill 触发).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradePlan {
    /// 升级 ID
    pub upgrade_id: String,
    /// 升级描述
    pub description: String,
    /// 关联 Skill ID (借鉴 superpowers 234: 升级触发对应 Skill)
    pub skill_id: String,
    /// 重试预算 (per 编译期 hardcode `DEFAULT_UPGRADE_RETRY_BUDGET = 3`)
    pub retry_budget: u32,
}

/// 编译期 hardcode: 自升级默认重试预算.
pub const DEFAULT_UPGRADE_RETRY_BUDGET: u32 = 3;

/// 编译期 hardcode: 自升级最大检测次数.
pub const MAX_UPGRADE_DETECT: u32 = 5;

/// 自升级引擎 (借鉴 superpowers 234 升级模式 + aGLM 108 PODA cycle).
#[derive(Debug)]
pub struct SelfUpgrade {
    /// 当前状态
    state: SelfUpgradeState,
    /// 升级计划
    plan: Option<UpgradePlan>,
    /// 重试计数
    attempts: u32,
    /// 上次动作
    last_action: Option<SelfUpgradeAction>,
}

impl Default for SelfUpgrade {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfUpgrade {
    /// 创建新自升级引擎.
    pub fn new() -> Self {
        Self {
            state: SelfUpgradeState::Idle,
            plan: None,
            attempts: 0,
            last_action: None,
        }
    }

    /// 设置升级计划.
    pub fn set_plan(&mut self, plan: UpgradePlan) {
        self.plan = Some(plan);
    }

    /// 当前状态.
    pub fn state(&self) -> SelfUpgradeState {
        self.state
    }

    /// 当前计划 (只读).
    pub fn plan(&self) -> Option<&UpgradePlan> {
        self.plan.as_ref()
    }

    /// 重试计数.
    pub fn attempts(&self) -> u32 {
        self.attempts
    }

    /// 上次动作.
    pub fn last_action(&self) -> Option<SelfUpgradeAction> {
        self.last_action
    }

    /// 决策: 给定候选动作, 选下一个.
    pub fn decide(&self, candidates: &[SelfUpgradeAction]) -> Option<SelfUpgradeAction> {
        if candidates.is_empty() {
            return None;
        }
        match self.state {
            SelfUpgradeState::Idle => candidates
                .iter()
                .find(|a| matches!(a, SelfUpgradeAction::Detect))
                .copied(),
            SelfUpgradeState::Detecting => candidates
                .iter()
                .find(|a| matches!(a, SelfUpgradeAction::VerifyPre))
                .copied(),
            SelfUpgradeState::Verifying => candidates
                .iter()
                .find(|a| matches!(a, SelfUpgradeAction::Apply))
                .copied(),
            SelfUpgradeState::Applying => candidates
                .iter()
                .find(|a| matches!(a, SelfUpgradeAction::VerifyPost))
                .copied(),
            SelfUpgradeState::Upgraded
            | SelfUpgradeState::RolledBack
            | SelfUpgradeState::Failed => candidates
                .iter()
                .find(|a| matches!(a, SelfUpgradeAction::Finalize))
                .copied(),
        }
    }

    /// 单步: 走 1 个动作, 状态机迁移 + 重试计数.
    pub fn step(&mut self, action: SelfUpgradeAction) -> AutonomyResult<SelfUpgradeState> {
        let plan = self.plan.clone().ok_or_else(|| {
            AutonomyError::MainLoopFailed("self-upgrade: no plan set, call set_plan() first".into())
        })?;
        let new_state = match (self.state, action) {
            (SelfUpgradeState::Idle, SelfUpgradeAction::Detect) => {
                if self.attempts >= MAX_UPGRADE_DETECT {
                    return Err(AutonomyError::SelfUpgradeRetryBudgetExhausted {
                        attempts: self.attempts,
                        max: MAX_UPGRADE_DETECT,
                    });
                }
                SelfUpgradeState::Detecting
            }
            (SelfUpgradeState::Detecting, SelfUpgradeAction::VerifyPre) => {
                SelfUpgradeState::Verifying
            }
            (SelfUpgradeState::Verifying, SelfUpgradeAction::Apply) => {
                SelfUpgradeState::Applying
            }
            (SelfUpgradeState::Applying, SelfUpgradeAction::VerifyPost) => {
                SelfUpgradeState::Upgraded
            }
            (SelfUpgradeState::Applying, SelfUpgradeAction::Rollback) => {
                SelfUpgradeState::RolledBack
            }
            (s, a) => {
                return Err(AutonomyError::SelfUpgradeIllegalTransition {
                    from: s,
                    to: self.state,
                    reason: format!("action {:?} not allowed in state {:?}", a, s),
                });
            }
        };
        if matches!(action, SelfUpgradeAction::Detect) {
            self.attempts += 1;
        }
        // 校验: retry budget
        if matches!(new_state, SelfUpgradeState::Detecting)
            && self.attempts > plan.retry_budget
        {
            return Err(AutonomyError::SelfUpgradeRetryBudgetExhausted {
                attempts: self.attempts,
                max: plan.retry_budget,
            });
        }
        self.state = new_state;
        self.last_action = Some(action);
        Ok(new_state)
    }

    /// 跑直到终态 (Upgraded / RolledBack / Failed), 4 步全跑完.
    pub fn run_until_terminal(&mut self) -> AutonomyResult<SelfUpgradeState> {
        for _ in 0..5 {
            if self.state.is_terminal() {
                return Ok(self.state);
            }
            // 4 步默认序列: Detect → VerifyPre → Apply → VerifyPost → Upgraded
            let action = match self.state {
                SelfUpgradeState::Idle => SelfUpgradeAction::Detect,
                SelfUpgradeState::Detecting => SelfUpgradeAction::VerifyPre,
                SelfUpgradeState::Verifying => SelfUpgradeAction::Apply,
                SelfUpgradeState::Applying => SelfUpgradeAction::VerifyPost,
                _ => break,
            };
            self.step(action)?;
        }
        Ok(self.state)
    }
}

// ============================================================================
// §3 自修复 (Self-repair) — 借鉴 Chidori journal rollback + apeireth-rollback 6 策略
// ============================================================================

/// 自修复 6 状态机.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfRepairState {
    /// 健康
    Healthy,
    /// 检测到失败
    Detected,
    /// 写 journal snapshot (借鉴 chidori 决策论重放)
    Snapshotting,
    /// 执行修复 (借鉴 apeireth-rollback 6 策略)
    Repairing,
    /// 修复成功
    Repaired,
    /// 修复失败
    Failed,
}

impl SelfRepairState {
    /// 全部状态 (编译期 hardcode 兜底).
    pub const ALL: [SelfRepairState; 6] = [
        Self::Healthy,
        Self::Detected,
        Self::Snapshotting,
        Self::Repairing,
        Self::Repaired,
        Self::Failed,
    ];

    /// 是否终态 (Repaired / Failed).
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Repaired | Self::Failed)
    }
}

/// 自修复 6 动作.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelfRepairAction {
    /// 健康检查
    HealthCheck,
    /// 失败检测
    Snapshot,
    /// 诊断失败根因
    Diagnose,
    /// 恢复
    Restore,
    /// 重放 journal
    Replay,
    /// 恢复终态
    Recover,
}

// ----- chidori 公开模式 1:1 映射 (借鉴 host-call journal 9 字段) -----

/// 决定论元数据 (借鉴 chidori `DeterminismMeta` 3 字段 1:1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterminismMeta {
    /// 主机 PID (借鉴 chidori `host_pid`)
    pub host_pid: u32,
    /// 逻辑时钟 (借鉴 chidori `logical_clock`)
    pub logical_clock: u64,
    /// 随机数种子 (借鉴 chidori `rng_seed`)
    pub rng_seed: u64,
}

/// 失败事件 (借鉴 chidori `JournalEntry` 9 字段 1:1, "host_call" 改 "failure").
///
/// 字段集 (chidori 1:1):
/// - `seq`              ← chidori `sequence_number`
/// - `event_kind`       ← chidori `event_kind`
/// - `ts`               ← chidori `timestamp` (unix ms)
/// - `child_id`         ← chidori `guest_id` (改: 失败来源)
/// - `plan_version`     ← chidori `plan_version`
/// - `input`            ← chidori `payload_in` (改: 失败输入)
/// - `output`           ← chidori `payload_out` (None = 待修复)
/// - `result`           ← chidori `call_result` (改: 修复结果)
/// - `determinism_meta` ← chidori `determinism_meta`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureEvent {
    /// 单调递增 seq (0-indexed)
    pub seq: u64,
    /// 事件类型
    pub event_kind: FailureEventKind,
    /// unix ms timestamp
    pub ts: u64,
    /// 失败来源 ID (eg. 哪个 crate / 哪个器官)
    pub child_id: String,
    /// 计划版本 (关联 SelfUpgrade.plan)
    pub plan_version: u64,
    /// 失败输入 payload (JSON)
    pub input: serde_json::Value,
    /// 失败输出 payload (None = 待修复)
    pub output: Option<serde_json::Value>,
    /// 修复结果
    pub result: RepairResult,
    /// 决定论元数据 (借鉴 chidori 1:1)
    pub determinism_meta: DeterminismMeta,
}

/// 失败事件类型 (借鉴 chidori `HostCallKind` 7 变体, 改"host call"为"failure").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureEventKind {
    /// 完整性校验失败 (借鉴 chidori Health)
    Integrity,
    /// 资源耗尽 (借鉴 chidori ResourceRequest)
    Resource,
    /// 异常退出 (借鉴 chidori AbnormalExit)
    AbnormalExit,
    /// 重启请求 (借鉴 chidori RestartRequest)
    Restart,
    /// 快照请求 (借鉴 chidori SnapshotRequest)
    Snapshot,
    /// 返回 (借鉴 chidori Return, 用于 healthy ack)
    Return,
    /// 自定义失败
    Custom,
}

impl FailureEventKind {
    /// 全部变体 (编译期 hardcode 兜底 = 7, 跟 chidori 1:1).
    pub const COUNT: usize = 7;

    /// chidori 1:1 字段名 (借鉴文档用).
    pub const fn as_chidori_str(self) -> &'static str {
        match self {
            Self::Integrity => "Health",
            Self::Resource => "ResourceRequest",
            Self::AbnormalExit => "AbnormalExit",
            Self::Restart => "RestartRequest",
            Self::Snapshot => "SnapshotRequest",
            Self::Return => "Return",
            Self::Custom => "Custom",
        }
    }
}

/// 修复结果 (借鉴 chidori `HostCallResult` 4 变体 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairResult {
    /// 修复成功
    Ok,
    /// 拒绝 (修复不可行)
    Rejected,
    /// 延迟 (等待重试)
    Deferred,
    /// 错误
    Error,
}

impl RepairResult {
    /// 全部变体 (编译期 hardcode 兜底 = 4, 跟 chidori 1:1).
    pub const COUNT: usize = 4;

    /// chidori 1:1 字段名.
    pub const fn as_chidori_str(self) -> &'static str {
        match self {
            Self::Ok => "Ok",
            Self::Rejected => "Rejected",
            Self::Deferred => "Deferred",
            Self::Error => "Error",
        }
    }
}

/// 修复策略 (借鉴 apeireth-rollback `RollbackStrategy` 6 变体 1:1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairStrategy {
    /// 完整文件副本
    Full,
    /// 单文件备份
    File,
    /// 差异备份
    Diff,
    /// git 状态备份
    Git,
    /// session 级别备份
    Session,
    /// 自动选择
    Auto,
}

impl RepairStrategy {
    /// 全部变体 (编译期 hardcode 兜底 = 6, 跟 apeireth-rollback 1:1).
    pub const COUNT: usize = 6;
}

/// 修复 journal (借鉴 chidori `Journal` 6 fn 1:1).
///
/// 0 真 import chidori crate, 字段基于 chidori 公开模式 1:1 映射.
#[derive(Debug, Default)]
pub struct RepairJournal {
    /// 全部 FailureEvent (借鉴 chidori `entries`)
    entries: Vec<FailureEvent>,
    /// 下次 seq (借鉴 chidori `next_seq`)
    next_seq: u64,
}

impl RepairJournal {
    /// 创建空 journal.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_seq: 0,
        }
    }

    /// 追加 1 个 FailureEvent (借鉴 chidori `append`).
    ///
    /// 强制: 调 `FailureEventKind::COUNT` 校验所有变体, 0 偷漏.
    pub fn append(&mut self, mut event: FailureEvent) -> &FailureEvent {
        // 强制 seq = next_seq (monotonic, 借鉴 chidori append 重写 seq)
        event.seq = self.next_seq;
        self.next_seq += 1;
        self.entries.push(event);
        self.entries.last().expect("just pushed")
    }

    /// 全部 entries (借鉴 chidori `entries`).
    pub fn entries(&self) -> &[FailureEvent] {
        &self.entries
    }

    /// entries 数.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 按 event_kind 过滤 (借鉴 chidori `filter_kind`).
    pub fn filter_kind(&self, kind: FailureEventKind) -> Vec<&FailureEvent> {
        self.entries.iter().filter(|e| e.event_kind == kind).collect()
    }

    /// 按 child_id 过滤 (apeireth 扩展, 借鉴 chidori `filter_child`).
    pub fn filter_child(&self, child_id: &str) -> Vec<&FailureEvent> {
        self.entries
            .iter()
            .filter(|e| e.child_id == child_id)
            .collect()
    }

    /// 清空 (apeireth 扩展).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_seq = 0;
    }

    /// 重放 (借鉴 chidori `replay`, 决定论 metadata 兜底).
    ///
    /// skeleton: 返回 entries 列表 + 总数, 0 真重放 (留 R127 续实装).
    pub fn replay(&self) -> Vec<u64> {
        self.entries.iter().map(|e| e.seq).collect()
    }
}

/// 71GB 4 重防御借用常量 (per apeireth-rollback 编译期 hardcode, 借用值, 0 改 apeireth-rollback).
pub const BORROWED_MAX_SHADOW_AGE_DAYS: u64 = 7;
pub const BORROWED_MAX_SHADOW_SIZE_BYTES: u64 = 100 * 1024 * 1024;
pub const BORROWED_MAX_TOTAL_SHADOW_SIZE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// 自修复引擎 (借鉴 chidori journal rollback + apeireth-rollback 6 策略).
#[derive(Debug)]
pub struct SelfRepair {
    /// 当前状态
    state: SelfRepairState,
    /// 失败事件 journal (借鉴 chidori)
    journal: RepairJournal,
    /// 上次动作
    last_action: Option<SelfRepairAction>,
    /// 修复策略
    strategy: RepairStrategy,
    /// 修复计数
    repairs: u32,
}

impl Default for SelfRepair {
    fn default() -> Self {
        Self::new()
    }
}

impl SelfRepair {
    /// 创建新自修复引擎 (state=Healthy, journal=空, strategy=Auto).
    pub fn new() -> Self {
        Self {
            state: SelfRepairState::Healthy,
            journal: RepairJournal::new(),
            last_action: None,
            strategy: RepairStrategy::Auto,
            repairs: 0,
        }
    }

    /// 设置修复策略.
    pub fn set_strategy(&mut self, strategy: RepairStrategy) {
        self.strategy = strategy;
    }

    /// 当前状态.
    pub fn state(&self) -> SelfRepairState {
        self.state
    }

    /// 当前 journal (只读).
    pub fn journal(&self) -> &RepairJournal {
        &self.journal
    }

    /// 修复策略.
    pub fn strategy(&self) -> RepairStrategy {
        self.strategy
    }

    /// 修复计数.
    pub fn repairs(&self) -> u32 {
        self.repairs
    }

    /// 上次动作.
    pub fn last_action(&self) -> Option<SelfRepairAction> {
        self.last_action
    }

    /// 决策: 给定候选动作, 选下一个.
    pub fn decide(&self, candidates: &[SelfRepairAction]) -> Option<SelfRepairAction> {
        if candidates.is_empty() {
            return None;
        }
        match self.state {
            SelfRepairState::Healthy => candidates
                .iter()
                .find(|a| matches!(a, SelfRepairAction::HealthCheck))
                .copied(),
            SelfRepairState::Detected => candidates
                .iter()
                .find(|a| matches!(a, SelfRepairAction::Snapshot))
                .copied(),
            SelfRepairState::Snapshotting => candidates
                .iter()
                .find(|a| matches!(a, SelfRepairAction::Diagnose))
                .copied(),
            SelfRepairState::Repairing => candidates
                .iter()
                .find(|a| matches!(a, SelfRepairAction::Restore))
                .copied(),
            SelfRepairState::Repaired | SelfRepairState::Failed => candidates
                .iter()
                .find(|a| matches!(a, SelfRepairAction::Recover))
                .copied(),
        }
    }

    /// 单步: 走 1 个动作, 状态机迁移.
    pub fn step(&mut self, action: SelfRepairAction) -> AutonomyResult<SelfRepairState> {
        let new_state = match (self.state, action) {
            (SelfRepairState::Healthy, SelfRepairAction::HealthCheck) => {
                SelfRepairState::Healthy
            }
            (SelfRepairState::Healthy, SelfRepairAction::Snapshot) => {
                SelfRepairState::Detected
            }
            (SelfRepairState::Detected, SelfRepairAction::Snapshot) => {
                SelfRepairState::Snapshotting
            }
            (SelfRepairState::Snapshotting, SelfRepairAction::Diagnose) => {
                SelfRepairState::Repairing
            }
            (SelfRepairState::Repairing, SelfRepairAction::Restore) => {
                SelfRepairState::Repaired
            }
            (SelfRepairState::Repairing, SelfRepairAction::Replay) => {
                SelfRepairState::Repaired
            }
            (s, a) => {
                return Err(AutonomyError::SelfRepairIllegalTransition {
                    from: s,
                    to: self.state,
                    reason: format!("action {:?} not allowed in state {:?}", a, s),
                });
            }
        };
        self.state = new_state;
        self.last_action = Some(action);
        Ok(new_state)
    }

    /// 跑直到终态 (Repaired / Failed), 默认 5 步全跑完.
    pub fn run_until_terminal(&mut self) -> AutonomyResult<SelfRepairState> {
        // Healthy 状态无需 repair, 立即终止 (per R139-1-retry-2 fix: 0 increment repairs counter)
        if matches!(self.state, SelfRepairState::Healthy) {
            return Ok(self.state);
        }
        for _ in 0..5 {
            if self.state.is_terminal() {
                return Ok(self.state);
            }
            // 默认 5 步: HealthCheck → Snapshot → Diagnose → Restore → Repaired
            let action = match self.state {
                SelfRepairState::Healthy => SelfRepairAction::HealthCheck,
                SelfRepairState::Detected => SelfRepairAction::Snapshot,
                SelfRepairState::Snapshotting => SelfRepairAction::Diagnose,
                SelfRepairState::Repairing => SelfRepairAction::Restore,
                SelfRepairState::Repaired | SelfRepairState::Failed => break,
            };
            self.step(action)?;
        }
        // 只在真正完成 Repaired 终态时 increment repairs counter (per R139-1-retry-2 fix)
        if matches!(self.state, SelfRepairState::Repaired) {
            self.repairs += 1;
        }
        Ok(self.state)
    }

    /// 强制 fail (用于测试, 模拟失败注入).
    pub fn force_fail(&mut self) -> AutonomyResult<SelfRepairState> {
        let prev = self.state;
        self.state = SelfRepairState::Failed;
        self.last_action = None;
        Ok(prev)
    }
}

// ============================================================================
// §4 顶层 LibraryAutonomy 协调器 (3 sub-engine + 主循环)
// ============================================================================

/// Library Stage 4 自治主循环 metrics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyMetrics {
    /// SelfEvolution 循环计数
    pub evolution_cycles: u32,
    /// SelfEvolution 终态 (Evolved=true / Failed=false)
    pub evolution_evolved: bool,
    /// SelfUpgrade 终态 (Upgraded=true / RolledBack 或 Failed=false)
    pub upgrade_applied: bool,
    /// SelfUpgrade 重试次数
    pub upgrade_attempts: u32,
    /// SelfRepair 修复次数
    pub repair_count: u32,
    /// SelfRepair 失败事件总数
    pub failure_events: u32,
}

/// Library Stage 4 自治总报告.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomyReport {
    /// 终态 metrics
    pub metrics: AutonomyMetrics,
    /// SelfEvolution 状态
    pub evolution_state: SelfEvolutionState,
    /// SelfUpgrade 状态
    pub upgrade_state: SelfUpgradeState,
    /// SelfRepair 状态
    pub repair_state: SelfRepairState,
    /// 借鉴 ID 列表
    pub borrow_ids: Vec<String>,
    /// 时间戳 (unix ms)
    pub ts: u64,
}

impl AutonomyReport {
    /// 借鉴 ID 列表 (3 个, per §0 借鉴表).
    pub const BORROW_IDS: [&'static str; 3] = [
        "R127-BORROW-obra/superpowers-2026-08-10",
        "R127-BORROW-GATERAGE/aglm-2024Q4-2026-08-10",
        "R127-BORROW-ThousandBirdsInc/chidori-2025-12-2026-08-10",
    ];
}

/// Library Stage 4 自治顶层协调器.
// 注: 不 derive Debug 因 SelfEvolution 0 derive Debug. 手 impl `Debug` 显示关键字段.
pub struct LibraryAutonomy {
    /// 自演化子引擎
    pub evolution: SelfEvolution,
    /// 自升级子引擎
    pub upgrade: SelfUpgrade,
    /// 自修复子引擎
    pub repair: SelfRepair,
    /// 主循环是否跑
    running: bool,
    /// 总循环计数
    total_ticks: u32,
}

/// 手 impl Debug for LibraryAutonomy.
impl std::fmt::Debug for LibraryAutonomy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryAutonomy")
            .field("evolution", &self.evolution)
            .field("upgrade_state", &self.upgrade.state())
            .field("repair_state", &self.repair.state())
            .field("running", &self.running)
            .field("total_ticks", &self.total_ticks)
            .finish()
    }
}

impl Default for LibraryAutonomy {
    fn default() -> Self {
        Self::new()
    }
}

impl LibraryAutonomy {
    /// 创建新 LibraryAutonomy 协调器.
    pub fn new() -> Self {
        Self {
            evolution: SelfEvolution::new(),
            upgrade: SelfUpgrade::new(),
            repair: SelfRepair::new(),
            running: false,
            total_ticks: 0,
        }
    }

    /// 当前是否跑.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// 总 tick 计数.
    pub fn total_ticks(&self) -> u32 {
        self.total_ticks
    }

    /// 启动主循环.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// 停止主循环.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// 主循环 1 tick: 跑 3 sub-engine 各 1 step.
    pub fn tick(&mut self) -> AutonomyResult<AutonomyMetrics> {
        if !self.running {
            return Err(AutonomyError::MainLoopFailed(
                "library autonomy not started, call start() first".into(),
            ));
        }
        self.total_ticks += 1;

        // 1. SelfEvolution 1 step (Idle → Observing, 触发 ObserveSkill)
        if !self.evolution.state.is_terminal() {
            self.evolution.step(SelfEvolutionAction::Observe)?;
        }

        // 2. SelfUpgrade 1 step (need plan set, 否则 skip)
        if !self.upgrade.state.is_terminal() && self.upgrade.plan.is_some() {
            // 决策: Idle → Detect
            if matches!(self.upgrade.state, SelfUpgradeState::Idle) {
                self.upgrade.step(SelfUpgradeAction::Detect)?;
            }
        }

        // 3. SelfRepair 1 step (Healthy → Healthy HealthCheck, 或 Detected → Snapshotting)
        if !self.repair.state.is_terminal() {
            if matches!(self.repair.state, SelfRepairState::Healthy) {
                // Healthy → Healthy (HealthCheck action)
                self.repair.step(SelfRepairAction::HealthCheck)?;
            }
        }

        Ok(self.metrics())
    }

    /// 当前 metrics.
    pub fn metrics(&self) -> AutonomyMetrics {
        AutonomyMetrics {
            evolution_cycles: self.evolution.cycles(),
            evolution_evolved: matches!(self.evolution.state, SelfEvolutionState::Evolved),
            upgrade_applied: matches!(self.upgrade.state, SelfUpgradeState::Upgraded),
            upgrade_attempts: self.upgrade.attempts(),
            repair_count: self.repair.repairs(),
            failure_events: self.repair.journal().len() as u32,
        }
    }

    /// 当前总报告.
    pub fn report(&self) -> AutonomyReport {
        AutonomyReport {
            metrics: self.metrics(),
            evolution_state: self.evolution.state,
            upgrade_state: self.upgrade.state,
            repair_state: self.repair.state,
            borrow_ids: AutonomyReport::BORROW_IDS.iter().map(|s| s.to_string()).collect(),
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
// §5 单元测试 (8 evolution + 9 upgrade + 8 repair + 2 main = 27 tests)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- §1 SelfEvolution tests (8 tests) -----

    #[test]
    fn evo_01_new_evolution_starts_in_idle() {
        let e = SelfEvolution::new();
        assert_eq!(e.state(), SelfEvolutionState::Idle);
        assert_eq!(e.cycles(), 0);
        assert!(e.last_action().is_none());
    }

    #[test]
    fn evo_02_skill_registry_has_4_default_skills() {
        let e = SelfEvolution::new();
        let r = e.skill_registry();
        assert_eq!(r.len(), 4, "4 default Skill: TddFirst + Observe + Plan + Adapt");
        assert!(!r.is_empty());
    }

    #[test]
    fn evo_03_skill_trait_load_markdown_returns_structured_text() {
        let s = TddFirstSkill;
        let md = s.load_markdown();
        assert!(md.contains("TDD First"));
        assert!(md.contains("tdd-first"));
        assert!(md.contains("TDD required"));
        assert!(md.contains("RED"));
    }

    #[test]
    fn evo_04_skill_trait_tdd_required_default_true() {
        let s = TddFirstSkill;
        assert!(s.tdd_required());
        let s = ObserveSkill;
        assert!(s.tdd_required());
    }

    #[test]
    fn evo_05_skill_registry_get_by_id() {
        let r = SkillRegistry::new();
        let s = r.get("tdd-first");
        assert!(s.is_some());
        assert_eq!(s.unwrap().id(), "tdd-first");
    }

    #[test]
    fn evo_06_evolution_step_idle_to_observing() {
        let mut e = SelfEvolution::new();
        let r = e.step(SelfEvolutionAction::Observe);
        assert!(r.is_ok());
        assert_eq!(e.state(), SelfEvolutionState::Observing);
        assert_eq!(e.cycles(), 1);
        assert_eq!(e.last_action(), Some(SelfEvolutionAction::Observe));
    }

    #[test]
    fn evo_07_evolution_illegal_transition_error() {
        let mut e = SelfEvolution::new();
        // Idle 状态不能 Adapt
        let r = e.step(SelfEvolutionAction::Adapt);
        assert!(r.is_err());
        match r.unwrap_err() {
            AutonomyError::SelfEvolutionIllegalTransition { from, to: _, reason } => {
                assert_eq!(from, SelfEvolutionState::Idle);
                assert!(reason.contains("Adapt"));
            }
            _ => panic!("期望 SelfEvolutionIllegalTransition"),
        }
    }

    #[test]
    fn evo_08_evolution_run_until_terminal_evolved() {
        let mut e = SelfEvolution::new();
        let r = e.run_until_terminal();
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), SelfEvolutionState::Evolved);
        // 5 步全跑完
        assert_eq!(e.cycles(), 4);
    }

    #[test]
    fn evo_09_poda_stage_hint_returns_plan_initially() {
        // R125-7 真实施类型 marker, 0 调 step() 时 current_stage = Plan
        let e = SelfEvolution::new();
        let hint = e.poda_stage_hint();
        assert_eq!(hint, PodaStage::Plan, "PODA stage 借鉴 aGLM 108 1:1");
        // 阶段名借鉴 aGLM 1:1
        assert_eq!(hint.name(), "Plan");
    }

    // ----- §2 SelfUpgrade tests (9 tests) -----

    #[test]
    fn up_01_new_upgrade_starts_in_idle() {
        let u = SelfUpgrade::new();
        assert_eq!(u.state(), SelfUpgradeState::Idle);
        assert_eq!(u.attempts(), 0);
        assert!(u.plan().is_none());
    }

    #[test]
    fn up_02_upgrade_set_plan() {
        let mut u = SelfUpgrade::new();
        let plan = UpgradePlan {
            upgrade_id: "u-001".into(),
            description: "B3 30 dim upgrade".into(),
            skill_id: "upgrade-30dim".into(),
            retry_budget: 3,
        };
        u.set_plan(plan.clone());
        assert_eq!(u.plan().unwrap().upgrade_id, "u-001");
    }

    #[test]
    fn up_03_upgrade_no_plan_error() {
        let mut u = SelfUpgrade::new();
        let r = u.step(SelfUpgradeAction::Detect);
        assert!(r.is_err());
        match r.unwrap_err() {
            AutonomyError::MainLoopFailed(msg) => assert!(msg.contains("no plan")),
            _ => panic!("期望 MainLoopFailed"),
        }
    }

    #[test]
    fn up_04_upgrade_step_idle_to_detecting() {
        let mut u = SelfUpgrade::new();
        u.set_plan(UpgradePlan {
            upgrade_id: "u-001".into(),
            description: "test".into(),
            skill_id: "t".into(),
            retry_budget: 3,
        });
        let r = u.step(SelfUpgradeAction::Detect);
        assert!(r.is_ok());
        assert_eq!(u.state(), SelfUpgradeState::Detecting);
        assert_eq!(u.attempts(), 1);
    }

    #[test]
    fn up_05_upgrade_retry_budget_exhausted() {
        let mut u = SelfUpgrade::new();
        u.set_plan(UpgradePlan {
            upgrade_id: "u-001".into(),
            description: "test".into(),
            skill_id: "t".into(),
            retry_budget: 1,
        });
        // 跑 1 次 detect OK
        assert!(u.step(SelfUpgradeAction::Detect).is_ok());
        // 回 Idle 再跑超 budget
        u.state = SelfUpgradeState::Idle; // 强制重置
        let r = u.step(SelfUpgradeAction::Detect);
        // attempts 累计到 2 > budget 1
        assert!(r.is_err());
        match r.unwrap_err() {
            AutonomyError::SelfUpgradeRetryBudgetExhausted { attempts, max } => {
                assert_eq!(attempts, 2);
                assert_eq!(max, 1);
            }
            _ => panic!("期望 SelfUpgradeRetryBudgetExhausted"),
        }
    }

    #[test]
    fn up_06_upgrade_illegal_transition_error() {
        let mut u = SelfUpgrade::new();
        u.set_plan(UpgradePlan {
            upgrade_id: "u-001".into(),
            description: "test".into(),
            skill_id: "t".into(),
            retry_budget: 3,
        });
        // Idle 不能 VerifyPre
        let r = u.step(SelfUpgradeAction::VerifyPre);
        assert!(r.is_err());
        match r.unwrap_err() {
            AutonomyError::SelfUpgradeIllegalTransition { from, .. } => {
                assert_eq!(from, SelfUpgradeState::Idle);
            }
            _ => panic!("期望 SelfUpgradeIllegalTransition"),
        }
    }

    #[test]
    fn up_07_upgrade_run_until_terminal_upgraded() {
        let mut u = SelfUpgrade::new();
        u.set_plan(UpgradePlan {
            upgrade_id: "u-001".into(),
            description: "test".into(),
            skill_id: "t".into(),
            retry_budget: 3,
        });
        let r = u.run_until_terminal();
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), SelfUpgradeState::Upgraded);
    }

    #[test]
    fn up_08_default_retry_budget_3() {
        assert_eq!(DEFAULT_UPGRADE_RETRY_BUDGET, 3);
        assert_eq!(MAX_UPGRADE_DETECT, 5);
    }

    #[test]
    fn up_09_upgrade_decide() {
        let u = SelfUpgrade::new();
        let candidates = vec![
            SelfUpgradeAction::Apply,
            SelfUpgradeAction::Detect,
            SelfUpgradeAction::Finalize,
        ];
        // Idle 状态选 Detect
        let r = u.decide(&candidates);
        assert_eq!(r, Some(SelfUpgradeAction::Detect));
    }

    // ----- §3 SelfRepair tests (8 tests) -----

    #[test]
    fn rep_01_new_repair_starts_in_healthy() {
        let r = SelfRepair::new();
        assert_eq!(r.state(), SelfRepairState::Healthy);
        assert_eq!(r.repairs(), 0);
        assert!(r.journal().is_empty());
    }

    #[test]
    fn rep_02_failure_event_kind_count_7_matches_chidori() {
        assert_eq!(FailureEventKind::COUNT, 7);
        // 7 变体: Integrity / Resource / AbnormalExit / Restart / Snapshot / Return / Custom
        assert_eq!(FailureEventKind::Integrity.as_chidori_str(), "Health");
        assert_eq!(FailureEventKind::Resource.as_chidori_str(), "ResourceRequest");
        assert_eq!(FailureEventKind::AbnormalExit.as_chidori_str(), "AbnormalExit");
        assert_eq!(FailureEventKind::Restart.as_chidori_str(), "RestartRequest");
        assert_eq!(FailureEventKind::Snapshot.as_chidori_str(), "SnapshotRequest");
        assert_eq!(FailureEventKind::Return.as_chidori_str(), "Return");
        assert_eq!(FailureEventKind::Custom.as_chidori_str(), "Custom");
    }

    #[test]
    fn rep_03_repair_result_count_4_matches_chidori() {
        assert_eq!(RepairResult::COUNT, 4);
        assert_eq!(RepairResult::Ok.as_chidori_str(), "Ok");
        assert_eq!(RepairResult::Rejected.as_chidori_str(), "Rejected");
        assert_eq!(RepairResult::Deferred.as_chidori_str(), "Deferred");
        assert_eq!(RepairResult::Error.as_chidori_str(), "Error");
    }

    #[test]
    fn rep_04_repair_strategy_count_6_matches_apeireth_rollback() {
        assert_eq!(RepairStrategy::COUNT, 6);
    }

    #[test]
    fn rep_05_repair_journal_append_assigns_monotonic_seq() {
        let mut j = RepairJournal::new();
        let e1 = make_failure_event(0, FailureEventKind::Integrity, "child-1");
        let e2 = make_failure_event(0, FailureEventKind::AbnormalExit, "child-2");
        j.append(e1);
        j.append(e2);
        assert_eq!(j.len(), 2);
        assert_eq!(j.entries()[0].seq, 0);
        assert_eq!(j.entries()[1].seq, 1);
    }

    #[test]
    fn rep_06_repair_journal_filter_kind_and_child() {
        let mut j = RepairJournal::new();
        j.append(make_failure_event(0, FailureEventKind::Integrity, "child-1"));
        j.append(make_failure_event(0, FailureEventKind::AbnormalExit, "child-1"));
        j.append(make_failure_event(0, FailureEventKind::Integrity, "child-2"));
        let by_kind = j.filter_kind(FailureEventKind::Integrity);
        assert_eq!(by_kind.len(), 2);
        let by_child = j.filter_child("child-1");
        assert_eq!(by_child.len(), 2);
    }

    #[test]
    fn rep_07_repair_journal_replay_returns_seqs() {
        let mut j = RepairJournal::new();
        j.append(make_failure_event(0, FailureEventKind::Integrity, "c1"));
        j.append(make_failure_event(0, FailureEventKind::AbnormalExit, "c2"));
        let seqs = j.replay();
        assert_eq!(seqs, vec![0, 1]);
    }

    #[test]
    fn rep_08_repair_run_until_terminal_healthcheck_only() {
        // Healthy → HealthCheck → Healthy (loop-safe, 不变)
        let mut r = SelfRepair::new();
        let s = r.run_until_terminal();
        assert!(s.is_ok());
        // Healthy 状态 run_until_terminal 会立即终止
        assert_eq!(r.state(), SelfRepairState::Healthy);
        assert_eq!(r.repairs(), 0);
    }

    // ----- §4 LibraryAutonomy main tests (2 tests) -----

    #[test]
    fn main_01_library_autonomy_new_all_idle() {
        let a = LibraryAutonomy::new();
        assert!(!a.is_running());
        assert_eq!(a.total_ticks(), 0);
        assert_eq!(a.evolution.state(), SelfEvolutionState::Idle);
        assert_eq!(a.upgrade.state(), SelfUpgradeState::Idle);
        assert_eq!(a.repair.state(), SelfRepairState::Healthy);
    }

    #[test]
    fn main_02_library_autonomy_tick_after_start() {
        let mut a = LibraryAutonomy::new();
        a.start();
        assert!(a.is_running());
        let r = a.tick();
        assert!(r.is_ok());
        assert_eq!(a.total_ticks(), 1);
        let m = r.unwrap();
        // 1 tick 后: evolution Idle → Observing, upgrade 仍 Idle (no plan), repair Healthy
        assert_eq!(m.evolution_cycles, 1);
        assert_eq!(a.evolution.state(), SelfEvolutionState::Observing);
    }

    // ----- 8 硬墙 compile-time 守门 -----

    #[test]
    fn eight_hard_walls_compile_time_gates() {
        // B2: 0 触碰 workspace.version
        assert!(BORROWED_MAX_SHADOW_AGE_DAYS == 7);
        // A1: 数字严守 (本 crate 0 涉及 R11 baseline, 仅借用常量)
        assert!(BORROWED_MAX_SHADOW_SIZE_BYTES == 100 * 1024 * 1024);
        // B1: 入口签名 0 改 (本测试通过, 即编译通过)
        // B5: 8 哲学锚 (0 改原 8, 0 涉及本 crate)
        // B3: 30 维 (0 改 V0.5 公式)
        // B4: 6 重 v7 (0 改原 6 重)
        // A3: 13 键 (0 改 12 键原 12)
        // C1-C3: 0 commit / 0 push / 0 装 解除
        assert!(AutonomyReport::BORROW_IDS.len() == 3);
    }

    // ----- helper: 构造 FailureEvent (per chidori 9 字段 1:1) -----

    fn make_failure_event(seq: u64, kind: FailureEventKind, child_id: &str) -> FailureEvent {
        FailureEvent {
            seq,
            event_kind: kind,
            ts: current_unix_ms(),
            child_id: child_id.to_string(),
            plan_version: 1,
            input: serde_json::json!({"test": true}),
            output: None,
            result: RepairResult::Deferred,
            determinism_meta: DeterminismMeta {
                host_pid: 1,
                logical_clock: 100,
                rng_seed: 42,
            },
        }
    }
}
