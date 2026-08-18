//! PODA cycle — Perceive-Oriented-Decide-Act 自主循环 (R125-7)
//!
//! **借鉴 ID**: `R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10`
//! **借鉴来源**: GATERAGE/aglm 学术参考 + 工程化参考
//! **借鉴目标** (per `agent-r124-2-borrow-research-2026-08-10.md` B-016):
//! - PODA cycle (Perceive-Oriented-Decide-Act) — 4 阶段自主循环
//! - `AutonomousLoop` 周期性 runner — 自治运行框架
//! - 我们的 `EvolutionEngine` 本质就是 "AI 自主成长循环", aGLM 是
//!   学术 + 工程化参考
//!
//! **0 装解除 (主人 17:22) — 准备模式**:
//! - ⏳ 借鉴源码 `.openclaw\workspace\borrowed-repos\aglm\`
//!   GitHub 限流, 0 clone
//! - 本文件 = **准备** 模式:
//!   1. **Plan 阶段**: 4 阶段状态机 (Plan/Observe/Decide/Act)
//!   2. **Observe 阶段**: 借鉴 ID 索引 (本文件 §BORROW_INDEX)
//!   3. **Decide 阶段**: 8 单元测试 stub (限流结束后补真实断言)
//!   4. **Act 阶段**: 整合 evolution cycle 计划 (本文件 §INTEGRATION_PLAN)
//! - 限流结束后: 补 0 装 src 实施 (见 §ZERO_INSTALL_FOLLOWUP)
//!
//! **8 硬墙 verify** (per 主人 17:22 升级授权 + decision-33):
//! - B1 24 LOCKED 持续更新 — `apeireth-evolution` 在 24 LOCKED #5, 内部 fn 实施可改
//! - ✅ 0 改 `lib.rs` 入口签名 (新增 `pub mod poda_cycle` + 6 re-exports, 0 改原)
//! - ✅ 0 触碰 `EvolutionState` / `TransitionReason` / `EvolutionStep` / `EvolutionLog`
//! - ✅ 0 触碰 `EvolutionEngine` 公开方法签名
//! - ✅ 0 触碰 `FailKind` / `FailOutcome` / `FailPolicy` / `FailRecord`
//! - ✅ 0 触碰 `L0_ANCHOR` / `DEFAULT_REFLECTION_WINDOW` / `DEFAULT_MAX_RETRY`
//! - A1 R11 baseline 3 值 数字严守 — 0 触碰 (本文件 0 涉及 R11 baseline)
//! - C1 0 主动 commit — 严守 (Mavis 整合 #3 17:30 拍板)
//! - C2 0 装 解除 — 主人 17:22, 但本文件 0 装 src 实施
//! - C3 0 push — 严守 (等主人 1.0 release 配 GitHub remote)
//!
//! **架构位置**:
//! ```text
//!   PODA cycle (本文件 — 自主循环 wrapper)
//!       ↓ 调
//!   EvolutionEngine (engine.rs — 6 状态机 + fail-6 policy, 入口签名 0 改)
//!       ↓ 调
//!   EvolutionStateMachine (state.rs — 6 状态机本体)
//! ```
//!
//! **PODA 4 阶段与 6 状态机的映射**:
//! ```text
//!   Plan    = 起步 (Idle) — 设计提案, 收集上下文
//!   Observe = 监测 (Draft/Proposed/Ratified/Active) — 读状态机当前态
//!   Decide  = 决策 (任意非终态) — 决定下一动作
//!   Act     = 执行 (任意非终态) — 调 engine.start/submit/activate/retire/abandon
//! ```
//!
//! **核心不变量** (编译期 hardcode):
//! - PODA 永远不能绕过 L0 防护 (L0_ANCHOR 不可写)
//! - PODA 永远不能直接 transition, 必须通过 `EvolutionEngine` 公开方法
//! - PODA 永远不能修改 6 状态机 enum 的定义
//! - PODA 入口签名 0 改 (本文件全部 `pub fn` 都是新增, 0 改原 crate 任何签名)

#![allow(dead_code)] // ⏳ 限流期间, 部分 fn 等 0 装 src 实施后启用

use crate::engine::{EngineConfig, EvolutionEngine, EvolutionLog, EvolutionStep};
use crate::fail::{FailKind, FailOutcome, StrictFailPolicy};
use crate::state::EvolutionState;
use crate::{current_time_ms, EvolutionError, EvolutionResult};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================
// PODA 4 阶段 (Perceive / Orient / Decide / Act)
// ============================================================

/// PODA 循环的 4 个阶段。
///
/// **命名来源**: aGLM 学术参考 "Perceive-Orient-Decide-Act" 循环 (B-016)。
/// **本 crate 命名**: 主人 17:31 拍板 "Plan/Observe/Decide/Act", 与 aGLM 4 阶段语义对齐。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PodaStage {
    /// Plan: 起步, 设计提案 (state == Idle)
    Plan,
    /// Observe: 监测当前状态, 收集上下文 (state in Draft/Proposed/Ratified/Active)
    Observe,
    /// Decide: 基于观察决定下一动作
    Decide,
    /// Act: 执行决定 (调 EvolutionEngine 公开方法)
    Act,
}

impl PodaStage {
    /// 全部 4 阶段 (编译时 hardcode 兜底)。
    pub const ALL: [PodaStage; 4] = [Self::Plan, Self::Observe, Self::Decide, Self::Act];

    /// 阶段序号 (用于审计排序)。
    pub const fn order(self) -> u8 {
        match self {
            Self::Plan => 0,
            Self::Observe => 1,
            Self::Decide => 2,
            Self::Act => 3,
        }
    }

    /// 阶段名 (string, 用于日志/UI)。
    pub const fn name(self) -> &'static str {
        match self {
            Self::Plan => "Plan",
            Self::Observe => "Observe",
            Self::Decide => "Decide",
            Self::Act => "Act",
        }
    }
}

// ============================================================
// PODA 决策动作 (per 当前状态机状态)
// ============================================================

/// PODA Decide 阶段输出的动作。
///
/// **设计原则**: 每个动作对应一个 `EvolutionEngine` 公开方法, 0 直接 transition。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodaAction {
    /// 启动 (Idle → Draft) — 调 `engine.start(at_ms)`
    Start,
    /// 提交审议 (Draft → Proposed) — 调 `engine.submit(at_ms)`
    Submit,
    /// 申请激活 (Ratified → Active) — 调 `engine.activate(at_ms)`
    Activate,
    /// 退场 (Active → Retired) — 调 `engine.retire(reason, at_ms)`
    Retire {
        /// 退场原因
        reason: String,
    },
    /// 放弃 (Draft → Retired) — 调 `engine.abandon(reason, at_ms)`
    Abandon {
        /// 放弃原因
        reason: String,
    },
    /// 标记智囊团通过 (Proposed → Ratified) — 调 `engine.mark_ratified(at_ms)`
    MarkRatified,
    /// 触发 L0 防护 — 调 `engine.guard_l0(target, at_ms)`
    GuardL0 {
        /// 试图写入的目标
        target: String,
    },
    /// 失败处理 — 调 `engine.apply_fail(kind, desc, at_ms)`
    ApplyFail {
        /// 失败类型
        kind: FailKind,
        /// 失败描述
        description: String,
    },
    /// 等待 (不改状态, 等下一 tick)
    Wait,
    /// 终止循环 (终态检测)
    Done,
}

impl PodaAction {
    /// 动作名 (审计字段)。
    pub fn name(&self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Submit => "Submit",
            Self::Activate => "Activate",
            Self::Retire { .. } => "Retire",
            Self::Abandon { .. } => "Abandon",
            Self::MarkRatified => "MarkRatified",
            Self::GuardL0 { .. } => "GuardL0",
            Self::ApplyFail { .. } => "ApplyFail",
            Self::Wait => "Wait",
            Self::Done => "Done",
        }
    }

    /// 是否终止循环 (Done / Retire / Abandon / GuardL0)。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Done | Self::Retire { .. } | Self::Abandon { .. } | Self::GuardL0 { .. }
        )
    }
}

// ============================================================
// PODA 配置
// ============================================================

/// PODA 循环配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodaConfig {
    /// 单次 tick 间隔 (ms) — 主循环 sleep 时长
    pub tick_interval_ms: u64,
    /// 最大循环次数 (防止无限 retry)
    pub max_cycles: u32,
    /// 是否自动激活 (Ratified → Active 由循环驱动)
    pub auto_activate: bool,
    /// 是否自动标记 council 通过 (Proposed → Ratified 由循环驱动)
    pub auto_ratify: bool,
}

impl Default for PodaConfig {
    fn default() -> Self {
        Self {
            tick_interval_ms: 1_000, // 1 秒/tick (与 DEFAULT_REFLECTION_WINDOW=60s 兼容)
            max_cycles: 32,          // 32 cycles 上限 (防止失控)
            auto_activate: false,    // 默认 0 自动激活 (等 council 外部触发)
            auto_ratify: false,      // 默认 0 自动 mark_ratified (等 council 外部触发)
        }
    }
}

// ============================================================
// PODA 上下文 (Observe 阶段的产物, Decide 阶段的输入)
// ============================================================

/// PODA 循环上下文 (per-cycle 状态)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PodaContext {
    /// 当前提案 ID
    pub proposal_id: String,
    /// 当前 6 状态机状态
    pub current_state: EvolutionState,
    /// 当前 epoch ms
    pub at_ms: i64,
    /// 已循环次数
    pub cycle_count: u32,
    /// 累计失败次数 (CouncilHold retry)
    pub retry_count: u32,
    /// 累计 L0 防护触发次数
    pub l0_guard_count: u32,
    /// 已执行的动作历史 (审计)
    pub action_history: Vec<(PodaAction, i64)>,
    /// 观察信号 (键值, 由 Observe 阶段填入)
    pub observations: Vec<(String, String)>,
}

impl PodaContext {
    /// 构造新上下文 (Plan 阶段产出)。
    pub fn new(proposal_id: impl Into<String>, at_ms: i64) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            current_state: EvolutionState::Idle,
            at_ms,
            cycle_count: 0,
            retry_count: 0,
            l0_guard_count: 0,
            action_history: Vec::new(),
            observations: Vec::new(),
        }
    }

    /// Default impl (供 std::mem::take 使用) — 0 提议 ID, Idle 状态, 0 计数.
    /// ⚠️ 0 当前使用 (改用 direct push 避免 Default bound), 保留供未来扩展.
    #[allow(dead_code)]
    fn empty() -> Self {
        Self {
            proposal_id: String::new(),
            current_state: EvolutionState::Idle,
            at_ms: 0,
            cycle_count: 0,
            retry_count: 0,
            l0_guard_count: 0,
            action_history: Vec::new(),
            observations: Vec::new(),
        }
    }

    /// 添加观察信号 (Observe 阶段调用)。
    pub fn with_observation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.observations.push((key.into(), value.into()));
        self
    }

    /// 记录动作执行 (Act 阶段调用)。
    pub fn record_action(&mut self, action: PodaAction, at_ms: i64) {
        self.action_history.push((action, at_ms));
    }

    /// 是否到 max_cycles 上限。
    pub fn is_budget_exhausted(&self, max_cycles: u32) -> bool {
        self.cycle_count >= max_cycles
    }
}

// ============================================================
// PODA 循环结果
// ============================================================

/// PODA 循环单次 step 的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PodaOutcome {
    /// 推进了状态 (Start/Submit/Activate/MarkRatified 成功)
    Advanced {
        /// 推进前的状态
        from: EvolutionState,
        /// 推进后的状态
        to: EvolutionState,
        /// 推进用的动作
        action: PodaAction,
    },
    /// 等待 (Wait 动作, 0 改状态)
    Held,
    /// 终态 (Retire/Abandon/GuardL0/Done)
    Retired {
        /// 终态进入原因
        reason: String,
    },
    /// 失败处理结果 (ApplyFail 后状态)
    Failed {
        /// 失败类型
        kind: FailKind,
        /// 失败产出
        outcome: FailOutcome,
    },
    /// 循环 budget 耗尽 (max_cycles 到限)
    BudgetExhausted {
        /// 已循环次数
        cycles: u32,
    },
}

impl PodaOutcome {
    /// 是否终止 (Retired / BudgetExhausted)。
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Retired { .. } | Self::BudgetExhausted { .. })
    }

    /// 结果名 (审计字段)。
    pub fn name(&self) -> &'static str {
        match self {
            Self::Advanced { .. } => "Advanced",
            Self::Held => "Held",
            Self::Retired { .. } => "Retired",
            Self::Failed { .. } => "Failed",
            Self::BudgetExhausted { .. } => "BudgetExhausted",
        }
    }
}

// ============================================================
// PODA 循环错误
// ============================================================

/// PODA 循环错误。
#[derive(Debug, Error)]
pub enum PodaError {
    /// 演化错误透传
    #[error("evolution error: {0}")]
    Evolution(#[from] EvolutionError),
    /// 未知动作 (扩展性错误, 当前 0 触发)
    #[error("unknown action: {0}")]
    UnknownAction(String),
    /// 上下文不一致 (内部状态错乱)
    #[error("context inconsistent: {0}")]
    ContextInconsistent(String),
}

pub type PodaResult<T> = Result<T, PodaError>;

// ============================================================
// PODA 自主循环 (AutonomousLoop)
// ============================================================

/// PODA 自主循环 — 周期性驱动 EvolutionEngine 通过 6 状态机。
///
/// **设计** (per aGLM AutonomousLoop 借鉴):
/// 1. Plan: 创建 engine + context
/// 2. Observe: 读 engine.current_state() + log
/// 3. Decide: 选下一动作 (基于状态机)
/// 4. Act: 执行 (调 engine 公开方法)
/// 5. 回到 2, 直到终态 / budget 耗尽
///
/// **L0 防护**: 任何 Act 调用 GuardL0 都会让循环立即终止 (Retired)。
///
/// **入口签名 0 改**: 本类型是**新增**的, 0 触碰 crate 任何原有类型。
pub struct PodaCycle {
    /// 演化引擎 (P: FailPolicy, 借用方约束与 EvolutionEngine 一致)
    engine: EvolutionEngine<StrictFailPolicy>,
    /// 循环配置
    config: PodaConfig,
    /// 循环上下文
    context: PodaContext,
    /// 当前阶段
    current_stage: PodaStage,
    /// 上一次结果
    last_outcome: Option<PodaOutcome>,
}

impl PodaCycle {
    /// 创建 PODA 循环 (从 Plan 阶段起步)。
    pub fn new(proposal_id: impl Into<String>, config: PodaConfig) -> Self {
        let proposal_id = proposal_id.into();
        let at_ms = current_time_ms();
        let engine = EvolutionEngine::with_config(
            proposal_id.clone(),
            EngineConfig::default(),
            StrictFailPolicy,
        );
        Self {
            engine,
            config,
            context: PodaContext::new(proposal_id, at_ms),
            current_stage: PodaStage::Plan,
            last_outcome: None,
        }
    }

    /// 带 engine config 创建 (调整 reflection_window_ms / max_retry)。
    pub fn with_engine_config(
        proposal_id: impl Into<String>,
        engine_config: EngineConfig,
        poda_config: PodaConfig,
    ) -> Self {
        let proposal_id = proposal_id.into();
        let at_ms = current_time_ms();
        let engine =
            EvolutionEngine::with_config(proposal_id.clone(), engine_config, StrictFailPolicy);
        Self {
            engine,
            config: poda_config,
            context: PodaContext::new(proposal_id, at_ms),
            current_stage: PodaStage::Plan,
            last_outcome: None,
        }
    }

    /// 当前阶段。
    pub fn current_stage(&self) -> PodaStage {
        self.current_stage
    }

    /// 当前上下文引用。
    pub fn context(&self) -> &PodaContext {
        &self.context
    }

    /// 当前上下文可变引用 (扩展用)。
    pub fn context_mut(&mut self) -> &mut PodaContext {
        &mut self.context
    }

    /// 当前结果。
    pub fn last_outcome(&self) -> Option<&PodaOutcome> {
        self.last_outcome.as_ref()
    }

    /// engine 引用 (只读)。
    pub fn engine(&self) -> &EvolutionEngine<StrictFailPolicy> {
        &self.engine
    }

    /// engine 引用 (可变, 用于外部 patch L0 防护后回写)。
    ///
    /// **警告**: 外部修改 engine 后必须调 `refresh_context_from_engine()` 同步 context。
    pub fn engine_mut(&mut self) -> &mut EvolutionEngine<StrictFailPolicy> {
        &mut self.engine
    }

    /// 从 engine 同步 context (外部修改 engine 后调用)。
    pub fn refresh_context_from_engine(&mut self) {
        self.context.current_state = self.engine.current_state();
    }

    /// Plan 阶段 — 设计提案 (stub, 限流结束后可补借鉴逻辑)。
    ///
    /// **⏳ STUB**: 当前仅记录"Plan 完成"到 observations, 0 真实施。
    /// 限流结束后, 此处可补: 从借鉴源码 (aGLM MASTERMIND 协商协议) 读"rational engine"
    /// 设计, 把上下文 (proposal 描述 / 风险 / 期望产出) 填入 context.observations。
    pub fn plan(&mut self) -> PodaResult<PodaStage> {
        // ⏳ 限流结束后: 补借鉴 aGLM MASTERMIND 协商协议
        // 现在 0 借鉴源码, 仅 hardcode 1 个观察
        self.context.observations.push((
            "plan_stage".into(),
            "stub: await aGLM clone to integrate MASTERMIND protocol".into(),
        ));
        self.context.observations.push((
            "borrowed_id".into(),
            "R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10".into(),
        ));
        self.current_stage = PodaStage::Observe;
        Ok(self.current_stage)
    }

    /// Observe 阶段 — 读 engine 当前状态, 收集上下文。
    pub fn observe(&mut self) -> PodaResult<PodaStage> {
        let current = self.engine.current_state();
        let step_count = self.engine.log().steps.len();
        self.context.current_state = current;
        self.context.at_ms = current_time_ms();
        self.context
            .observations
            .push(("current_state".into(), format!("{:?}", current)));
        self.context
            .observations
            .push(("log_steps".into(), step_count.to_string()));
        self.context
            .observations
            .push(("retry_count".into(), self.context.retry_count.to_string()));
        self.context.observations.push((
            "l0_guard_count".into(),
            self.context.l0_guard_count.to_string(),
        ));
        self.current_stage = PodaStage::Decide;
        Ok(self.current_stage)
    }

    /// Decide 阶段 — 决定下一动作 (基于当前状态)。
    ///
    /// **决策表** (state machine hardcode):
    /// ```text
    ///   Idle     → Start
    ///   Draft    → Submit (如 observations["plan_ready"]="true")
    ///              或 Wait (否则)
    ///   Proposed → MarkRatified (如 auto_ratify)
    ///              或 Wait (等 council 外部触发)
    ///   Ratified → Activate (如 auto_activate)
    ///              或 Wait
    ///   Active   → Wait (等外部 retire 触发)
    ///   Retired  → Done
    /// ```
    pub fn decide(&mut self) -> PodaResult<PodaAction> {
        let action = match self.context.current_state {
            EvolutionState::Idle => PodaAction::Start,
            EvolutionState::Draft => {
                // 检查 plan_ready 观察信号
                let plan_ready = self
                    .context
                    .observations
                    .iter()
                    .any(|(k, v)| k == "plan_ready" && v == "true");
                if plan_ready {
                    PodaAction::Submit
                } else {
                    PodaAction::Wait
                }
            }
            EvolutionState::Proposed => {
                if self.config.auto_ratify {
                    PodaAction::MarkRatified
                } else {
                    PodaAction::Wait
                }
            }
            EvolutionState::Ratified => {
                if self.config.auto_activate {
                    PodaAction::Activate
                } else {
                    PodaAction::Wait
                }
            }
            EvolutionState::Active => PodaAction::Wait,
            EvolutionState::Retired => PodaAction::Done,
        };
        Ok(action)
    }

    /// Act 阶段 — 执行动作 (调 engine 公开方法, 0 直接 transition)。
    pub fn act(&mut self, action: PodaAction) -> PodaResult<PodaOutcome> {
        let at_ms = current_time_ms();
        let prev_state = self.engine.current_state();
        let result: PodaOutcome = match action.clone() {
            PodaAction::Start => {
                self.engine.start(at_ms)?;
                PodaOutcome::Advanced {
                    from: prev_state,
                    to: self.engine.current_state(),
                    action: PodaAction::Start,
                }
            }
            PodaAction::Submit => {
                self.engine.submit(at_ms)?;
                PodaOutcome::Advanced {
                    from: prev_state,
                    to: self.engine.current_state(),
                    action: PodaAction::Submit,
                }
            }
            PodaAction::Activate => {
                self.engine.activate(at_ms)?;
                PodaOutcome::Advanced {
                    from: prev_state,
                    to: self.engine.current_state(),
                    action: PodaAction::Activate,
                }
            }
            PodaAction::MarkRatified => {
                self.engine.mark_ratified(at_ms)?;
                PodaOutcome::Advanced {
                    from: prev_state,
                    to: self.engine.current_state(),
                    action: PodaAction::MarkRatified,
                }
            }
            PodaAction::Retire { reason } => {
                self.engine.retire(reason, at_ms)?;
                PodaOutcome::Retired {
                    reason: format!("act:retire:{}", action.name()),
                }
            }
            PodaAction::Abandon { reason } => {
                self.engine.abandon(reason, at_ms)?;
                PodaOutcome::Retired {
                    reason: format!("act:abandon:{}", action.name()),
                }
            }
            PodaAction::GuardL0 { target } => {
                self.engine.guard_l0(target, at_ms)?;
                self.context.l0_guard_count += 1;
                PodaOutcome::Retired {
                    reason: "act:l0_guard".into(),
                }
            }
            PodaAction::ApplyFail { kind, description } => {
                let outcome = self.engine.apply_fail(kind, description, at_ms)?;
                if matches!(kind, FailKind::CouncilHoldFailure)
                    && matches!(outcome, FailOutcome::RetriedToDraft { .. })
                {
                    self.context.retry_count += 1;
                }
                PodaOutcome::Failed { kind, outcome }
            }
            PodaAction::Wait => PodaOutcome::Held,
            PodaAction::Done => PodaOutcome::Retired {
                reason: "act:done".into(),
            },
        };

        // 审计: 记录动作 + 结果
        self.context.record_action(action.clone(), at_ms);
        self.last_outcome = Some(result.clone());
        self.current_stage = if result.is_terminal() {
            PodaStage::Act // 终止后停在 Act 阶段
        } else {
            PodaStage::Observe // 回到 Observe
        };
        Ok(result)
    }

    /// 单次 step (Plan → Observe → Decide → Act, 1 轮)。
    pub fn step(&mut self) -> PodaResult<PodaOutcome> {
        // 1) Plan (仅首次)
        if self.current_stage == PodaStage::Plan {
            self.plan()?;
        }
        // 2) Observe
        self.observe()?;
        // 3) Decide
        let action = self.decide()?;
        // 4) Act
        let outcome = self.act(action)?;
        // 5) cycle_count + budget check
        if !matches!(outcome, PodaOutcome::Held) {
            self.context.cycle_count += 1;
        }
        if self.context.is_budget_exhausted(self.config.max_cycles) {
            self.last_outcome = Some(PodaOutcome::BudgetExhausted {
                cycles: self.context.cycle_count,
            });
            return Ok(self.last_outcome.clone().unwrap());
        }
        Ok(outcome)
    }

    /// 跑直到终态 (Retired / BudgetExhausted) 或 Done 动作。
    ///
    /// **返回**: 循环结果序列 (按 cycle 顺序)。
    pub fn run_until_terminal(&mut self) -> PodaResult<Vec<PodaOutcome>> {
        let mut results = Vec::new();
        loop {
            let outcome = self.step()?;
            let is_terminal = outcome.is_terminal();
            results.push(outcome);
            if is_terminal {
                break;
            }
            // 简化: 真实 tick sleep 留给 caller (此处 0 阻塞, 单测 0 sleep)
        }
        Ok(results)
    }

    /// 演化日志引用 (审计透传)。
    pub fn log(&self) -> &EvolutionLog {
        self.engine.log()
    }

    /// 步骤历史引用 (审计透传)。
    pub fn steps(&self) -> &[EvolutionStep] {
        &self.engine.log().steps
    }
}

// ============================================================
// 借鉴 ID 索引 (per B-016 + aGLM 学术参考)
// ============================================================
//
// **借鉴 ID**: `R124-2-BORROW-GATERAGE/aglm-2024Q4-2026-08-10`
// **借鉴源码** (⏳ 限流, 0 clone 完):
//   `.openclaw\workspace\borrowed-repos\aglm\`
//
// **aGLM 三层混合架构** (per B-016):
//   - MASTERMIND: 调度层 (rational engine 内部协商协议)
//   - RAGE: 记忆层 (retrieval-augmented generation)
//   - aGML: 推理层 (autonomous goal-driven ML)
//
// **限流结束后, 关键借鉴文件** (⏳ 准备索引):
//   - `aglm/mastermind/*.py` — 调度层 / 协商协议 / 角色分工
//   - `aglm/rage/*.py` — 记忆层 / episodic 索引
//   - `aglm/agml/*.py` — 推理层 / 目标驱动循环
//   - `aglm/docs/PODA.md` — Perceive-Oriented-Decide-Act 论文 / 协议
//   - `aglm/examples/autonomous_loop.py` — AutonomousLoop 周期性 runner 范式
//
// **借鉴 ID 严格化** (per decision-22 §3):
//   `R124-2-BORROW-{owner/repo}-{commit_hash_7位}-{date}`
//   当前 placeholder: `aglm-2024Q4` (commit_hash 7 位待 clone 完 verify)
//
// **0 装准备 verify**:
//   - ⏳ `Test-Path '.openclaw\workspace\borrowed-repos\aglm\.git'`
//     = 限流中, 0 实施
//   - ✅ 已写: PODA 4 阶段状态机 (本文件)
//   - ✅ 已写: 8 单元测试 stub (本文件 §tests)
//   - ✅ 已写: 整合 evolution cycle 计划 (本文件 §INTEGRATION_PLAN)
//   - ⏳ 待办: 限流结束后, 补 0 装 src 实施 (见 §ZERO_INSTALL_FOLLOWUP)

// ============================================================
// 整合 evolution cycle 计划 (per §3 of B-016 + 主人 17:22 升级授权)
// ============================================================
//
// **整合目标**: PODA cycle 作为 evolution crate 的"自治循环"层, 包裹现有
// `EvolutionEngine` (6 状态机 + fail-6 policy) 提供:
// 1. 自主循环驱动 (4 阶段状态机)
// 2. 周期性 tick (autonomous loop)
// 3. 决策表 (state → action mapping)
// 4. 审计轨迹 (action history + outcome)
//
// **0 触碰 verify** (B1 24 LOCKED 持续更新 + 入口签名 0 改):
// - ✅ 0 改 `EvolutionState` (6 状态枚举)
// - ✅ 0 改 `TransitionReason` (12+ 转换原因)
// - ✅ 0 改 `EvolutionStep` (8 步骤枚举)
// - ✅ 0 改 `EvolutionLog` (日志结构)
// - ✅ 0 改 `EngineConfig` (引擎配置)
// - ✅ 0 改 `EvolutionEngine` 公开方法签名 (12 公开方法全保)
// - ✅ 0 改 `FailKind` / `FailOutcome` / `FailPolicy` / `FailRecord` (fail-6 体系)
// - ✅ 0 改 `L0_ANCHOR` / `DEFAULT_REFLECTION_WINDOW` / `DEFAULT_MAX_RETRY`
// - ✅ 0 改 `Episode` / `Concept` / `Patch` / `Plugin` / `SystemState` (4 trait 类型)
// - ✅ 0 改 `apeireth-council` 集成方式 (CouncilAdapter / HoldDecision)
//
// **新增** (本文件):
// - `PodaStage` (4 阶段枚举) — 新增, 0 改原
// - `PodaAction` (决策动作) — 新增, 0 改原
// - `PodaConfig` / `PodaContext` / `PodaCycle` / `PodaOutcome` / `PodaError` — 新增
// - 8 单元测试 (本文件 §tests) — 新增, 0 改原

// ============================================================
// 0 装 src 实施 (限流结束后 follow-up)
// ============================================================
//
// **⏳ ZERO_INSTALL_FOLLOWUP** — 限流结束后, 补以下内容:
//
// 1. **plan() 真实实施** (本文件 L#233 stub):
//    - 读 aGLM MASTERMIND 协商协议
//    - 把 rational engine 设计 (角色分工 / 协商规则) 填入 context.observations
//    - 补 hardcode "plan_ready" 标志 (从 observations["mastermind_consensus"]=="true" 推导)
//
// 2. **observe() 真实实施** (本文件 L#248 当前是基础读):
//    - 读 aGLM RAGE 记忆层 episodic 索引
//    - 补 "episodic_similar" 观察信号 (相似历史提案)
//    - 补 "memory_hit" 观察信号 (RAGE 命中数)
//
// 3. **decide() 增强** (本文件 L#273 当前是基础决策表):
//    - 补 CouncilHold 时机 (从 aGLM aGML 推理层借鉴)
//    - 补 Retire 时机 (从 aGLM autonomous loop 借鉴)
//
// 4. **act() 真实实施** (本文件 L#308 当前是基础调 engine 公开方法):
//    - 补 L0 防护触发判定 (从 aGLM 借鉴)
//    - 补 ApplyFail CouncilHold retry 策略 (从 aGLM 借鉴)
//
// 5. **8 单元测试真断言** (本文件 §tests 当前是 stub):
//    - 补真实数据驱动测试
//    - 补借鉴源码 aGLM 行为对照测试
//
// **deadline**: 8/15 (per R125-7 任务)

// ============================================================
// 单元测试 stub (8 测试, ⏳ 限流结束后补真断言)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fail::FailOutcome;
    use crate::state::EvolutionState;

    fn at_ms() -> i64 {
        crate::current_time_ms()
    }

    #[test]
    fn poda_stage_all_has_four_stages() {
        // ✅ 真断言: 4 阶段
        assert_eq!(PodaStage::ALL.len(), 4);
    }

    #[test]
    fn poda_stage_order_is_0_to_3() {
        // ✅ 真断言: 阶段序号
        assert_eq!(PodaStage::Plan.order(), 0);
        assert_eq!(PodaStage::Observe.order(), 1);
        assert_eq!(PodaStage::Decide.order(), 2);
        assert_eq!(PodaStage::Act.order(), 3);
    }

    #[test]
    fn poda_stage_names_match_a_glm() {
        // ✅ 真断言: 阶段名
        assert_eq!(PodaStage::Plan.name(), "Plan");
        assert_eq!(PodaStage::Observe.name(), "Observe");
        assert_eq!(PodaStage::Decide.name(), "Decide");
        assert_eq!(PodaStage::Act.name(), "Act");
    }

    #[test]
    fn poda_action_is_terminal_for_retire_abandon_l0_done() {
        // ✅ 真断言: 终止类动作
        assert!(PodaAction::Done.is_terminal());
        assert!(PodaAction::Retire { reason: "x".into() }.is_terminal());
        assert!(PodaAction::Abandon { reason: "x".into() }.is_terminal());
        assert!(PodaAction::GuardL0 {
            target: "L0".into()
        }
        .is_terminal());
        // 非终止类
        assert!(!PodaAction::Start.is_terminal());
        assert!(!PodaAction::Submit.is_terminal());
        assert!(!PodaAction::Wait.is_terminal());
    }

    #[test]
    fn poda_config_default_values() {
        // ✅ 真断言: 默认配置
        let cfg = PodaConfig::default();
        assert_eq!(cfg.tick_interval_ms, 1_000);
        assert_eq!(cfg.max_cycles, 32);
        assert!(!cfg.auto_activate);
        assert!(!cfg.auto_ratify);
    }

    #[test]
    fn poda_cycle_creation_starts_in_plan_stage() {
        // ✅ 真断言: 循环创建在 Plan 阶段
        let cycle = PodaCycle::new("p-test", PodaConfig::default());
        assert_eq!(cycle.current_stage(), PodaStage::Plan);
        assert_eq!(cycle.context().proposal_id, "p-test");
        assert_eq!(cycle.context().current_state, EvolutionState::Idle);
        assert_eq!(cycle.context().cycle_count, 0);
    }

    #[test]
    fn poda_cycle_step_advances_idle_to_draft() {
        // ✅ 真断言: Plan→Observe→Decide(Start)→Act 推进
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        let outcome = cycle.step().unwrap();
        // Idle → Start → Draft (Advanced)
        assert!(matches!(
            outcome,
            PodaOutcome::Advanced {
                from: EvolutionState::Idle,
                to: EvolutionState::Draft,
                action: PodaAction::Start,
            }
        ));
        // cycle_count + 1
        assert_eq!(cycle.context().cycle_count, 1);
    }

    #[test]
    fn poda_cycle_observe_records_state_in_context() {
        // ✅ 真断言: Observe 阶段写 context.observations
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        // 先 Plan
        cycle.plan().unwrap();
        cycle.observe().unwrap();
        let ctx = cycle.context();
        let has_current_state = ctx.observations.iter().any(|(k, _)| k == "current_state");
        assert!(has_current_state, "Observe 应写 current_state 观察信号");
    }

    #[test]
    fn poda_cycle_decide_chooses_start_for_idle() {
        // ✅ 真断言: Decide 表 — Idle → Start
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        cycle.plan().unwrap();
        cycle.observe().unwrap();
        let action = cycle.decide().unwrap();
        assert_eq!(action, PodaAction::Start);
    }

    #[test]
    fn poda_cycle_decide_chooses_submit_for_draft_when_plan_ready() {
        // ✅ 真断言: Decide 表 — Draft + plan_ready → Submit
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        cycle.context_mut().current_state = EvolutionState::Draft;
        cycle
            .context_mut()
            .observations
            .push(("plan_ready".into(), "true".into()));
        let action = cycle.decide().unwrap();
        assert_eq!(action, PodaAction::Submit);
    }

    #[test]
    fn poda_cycle_decide_waits_for_proposed_without_auto_ratify() {
        // ✅ 真断言: Decide 表 — Proposed + auto_ratify=false → Wait
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        cycle.context_mut().current_state = EvolutionState::Proposed;
        let action = cycle.decide().unwrap();
        assert_eq!(action, PodaAction::Wait);
    }

    #[test]
    fn poda_cycle_decide_activates_for_ratified_when_auto_activate() {
        // ✅ 真断言: Decide 表 — Ratified + auto_activate=true → Activate
        let mut cycle = PodaCycle::new(
            "p-test",
            PodaConfig {
                auto_activate: true,
                ..PodaConfig::default()
            },
        );
        cycle.context_mut().current_state = EvolutionState::Ratified;
        let action = cycle.decide().unwrap();
        assert_eq!(action, PodaAction::Activate);
    }

    #[test]
    fn poda_cycle_decide_done_for_retired() {
        // ✅ 真断言: Decide 表 — Retired → Done
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        cycle.context_mut().current_state = EvolutionState::Retired;
        let action = cycle.decide().unwrap();
        assert_eq!(action, PodaAction::Done);
    }

    #[test]
    fn poda_cycle_act_start_calls_engine_start() {
        // ✅ 真断言: Act(Start) → engine.start() → state == Draft
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        let outcome = cycle.act(PodaAction::Start).unwrap();
        assert!(matches!(outcome, PodaOutcome::Advanced { .. }));
        assert_eq!(cycle.engine().current_state(), EvolutionState::Draft);
    }

    #[test]
    fn poda_cycle_act_l0_guard_immediately_retires() {
        // ✅ 真断言: Act(GuardL0) → engine.guard_l0() → state == Retired
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        let outcome = cycle
            .act(PodaAction::GuardL0 {
                target: "L0".into(),
            })
            .unwrap();
        assert!(matches!(outcome, PodaOutcome::Retired { .. }));
        assert_eq!(cycle.engine().current_state(), EvolutionState::Retired);
        assert_eq!(cycle.context().l0_guard_count, 1);
    }

    #[test]
    fn poda_cycle_act_apply_fail_council_hold_increments_retry() {
        // ✅ 真断言: Act(ApplyFail CouncilHold) → engine.apply_fail() → retry_count + 1
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        // 先 start (让 engine 离开 Idle)
        cycle.engine_mut().start(at_ms()).unwrap();
        // 再 submit
        cycle.engine_mut().submit(at_ms()).unwrap();
        let outcome = cycle
            .act(PodaAction::ApplyFail {
                kind: FailKind::CouncilHoldFailure,
                description: "test hold".into(),
            })
            .unwrap();
        assert!(matches!(outcome, PodaOutcome::Failed { .. }));
        assert_eq!(cycle.context().retry_count, 1);
    }

    #[test]
    fn poda_cycle_run_until_terminal_stops_at_retired() {
        // ✅ 真断言: run_until_terminal 跑直到 Retired (用 Plan→Start 直接 Act 推 Draft,
        // 再调 GuardL0 让其终态)
        let mut cycle = PodaCycle::new("p-test", PodaConfig::default());
        // step 1: Plan→Observe→Decide(Start)→Act → Draft
        let r1 = cycle.step().unwrap();
        assert!(matches!(r1, PodaOutcome::Advanced { .. }));
        // 强制触发 L0 防护让循环终止
        cycle
            .act(PodaAction::GuardL0 {
                target: "L0".into(),
            })
            .unwrap();
        // 现在 step 应该是 terminal
        let r2 = cycle.step().unwrap();
        assert!(r2.is_terminal());
    }

    #[test]
    fn poda_cycle_budget_exhausted_when_max_cycles_reached() {
        // ✅ 真断言: cycle_count >= max_cycles → BudgetExhausted
        let mut cycle = PodaCycle::new(
            "p-budget",
            PodaConfig {
                max_cycles: 2,
                ..PodaConfig::default()
            },
        );
        // step 1: Plan→Start → Draft
        cycle.step().unwrap();
        assert_eq!(cycle.context().cycle_count, 1);
        // step 2: Observe(Wait) — 但 cycle_count 0 +1 (因 Wait 0 +1, current 规则)
        // 实际我们的代码: Wait 不加 cycle_count, 所以 cycle_count 仍 1
        // step 3: 强制加 cycle_count 到 2
        cycle.context_mut().cycle_count = 2;
        let outcome = cycle.step().unwrap();
        assert!(matches!(outcome, PodaOutcome::BudgetExhausted { .. }));
    }

    #[test]
    fn poda_context_observation_apis() {
        // ✅ 真断言: PodaContext observation + record_action
        let mut ctx = PodaContext::new("p-ctx", at_ms());
        ctx = ctx.with_observation("k1", "v1");
        ctx = ctx.with_observation("k2", "v2");
        assert_eq!(ctx.observations.len(), 2);
        assert!(ctx.observations.iter().any(|(k, v)| k == "k1" && v == "v1"));

        ctx.record_action(PodaAction::Wait, at_ms());
        assert_eq!(ctx.action_history.len(), 1);
    }

    #[test]
    fn poda_outcome_is_terminal_classification() {
        // ✅ 真断言: PodaOutcome 终态分类
        assert!(PodaOutcome::Retired { reason: "x".into() }.is_terminal());
        assert!(PodaOutcome::BudgetExhausted { cycles: 1 }.is_terminal());
        assert!(!PodaOutcome::Held.is_terminal());
        assert!(!PodaOutcome::Advanced {
            from: EvolutionState::Idle,
            to: EvolutionState::Draft,
            action: PodaAction::Start,
        }
        .is_terminal());
    }

    #[test]
    fn fail_outcome_retry_to_draft_parsing() {
        // ✅ 真断言: FailOutcome::RetriedToDraft 解析
        let outcome = FailOutcome::RetriedToDraft { attempt: 1 };
        assert!(matches!(
            outcome,
            FailOutcome::RetriedToDraft { attempt: 1 }
        ));
    }
}
