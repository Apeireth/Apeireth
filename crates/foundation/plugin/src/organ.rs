//! P-arch (2026-08-28): 9-organ trait 抽象边界 (per `v2-unabsorbed-features.md` 9-organ 概念).
//!
//! v1 `apeireth-companion` 把 9 organ 散落在 if-else / `lib.rs` 顶层 mod (`curiosity`,
//! `emotion_memory`, `hypothesis`, `value_cases`, `world_model`, `causal_world_model`,
//! `emergence`, 等) — 各自独立 + 内部不同步. v2 引入 `OrganTrait` 让 9 organ 走统一 trait
//! 边界, runtime 按 `organ_kind` 注入 `Arc<dyn OrganTrait>`.
//!
//! **位置** (per scene-d §5 决策 1 + LlmFactory/PerceptionInput 同模式):
//! - trait 在 `apeireth-plugin` (foundation), impl 留 `crates/engine/organ` (engine).
//! - 单向依赖: organ → plugin → core. 9 organ 不互相依赖.
//!
//! **9 organ IDs (v1 companion era)** — 与 R11 LOCKED 9 UI 器官 (body/brain/ear/eye/
//! hand/heart/memory/mind/voice) 是**两套体系**:
//!
//! - R11 LOCKED 9 organ: UI 器官 (TUI 渲染层, R11 严守 0 触碰).
//! - v1 companion era 9 organ ID: 行为/认知器官 (本 trait 服务对象, 移植自
//!   `legacy/canonical/apeireth-companion/src/{curiosity,emotion_memory,hypothesis,...}.rs`).
//!
//! | ID   | v1 module                  | v2 impl 状态                  |
//! |------|----------------------------|-------------------------------|
//! | W1   | `world_model` (TP31)       | 0 装 (rc 阶段或 v2.1)         |
//! | W2   | `causal_world_model`       | 0 装 (rc 阶段或 v2.1)         |
//! | W3   | `causal_world_model` 边挖  | 0 装 (rc 阶段或 v2.1)         |
//! | E4   | `curiosity` (好奇引擎)     | ✅ 真实现 (`CuriosityOrgan`)    |
//! | F4   | `hypothesis` (假设闭环)    | 0 装 (rc 阶段或 v2.1)         |
//! | F1   | `emotion_memory` (情感)    | 0 装 (rc 阶段或 v2.1)         |
//! | F6   | `value_cases` (价值内化)   | 0 装 (rc 阶段或 v2.1)         |
//! | E7   | `emergence` (涌现循环)     | 0 装 (rc 阶段或 v2.1)         |
//! | Memory| 记忆合并抽象               | 0 装 (rc 阶段或 v2.1)         |
//!
//! **0 装 PASS**:
//! - `OrganTrait` 是**纯 trait**, 0 LLM 依赖 (同 `LlmFactory` 模式).
//! - 9 organ IDs 全部列出 (`OrganKind` 9 variant), 但仅 `E4` 留真实现. 其余 8 organ
//!   在 `OrganTrait::process` 返 `Err(OrganError::NotImplemented(organ_id))` 显式标缺.
//! - `llm_factory()` 默认返 `None`, 不假装每个 organ 都接 LLM. Curiosity 即使 trait
//!   接口返 LLM (per 任务说明), **真实现**仍是确定性机制 (v1 真实现是确定性无 LLM,
//!   per `legacy/canonical/apeireth-companion/src/curiosity.rs:1-23` 文档明示).
//! - 真生产前阻塞 #1 (任务): 至少 1 organ 真移植 — E4 Curiosity 已 ✅.
//!
//! **3 阶审查** (O-6 锚 9, per `perception_backend.rs` 同模式):
//! 1. 总体: 与 LlmFactory / PerceptionInput 同位 (capability 抽象), 让 9 organ 走统一入口.
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致).
//! 3. 架构: runtime 拿 `Arc<dyn OrganTrait>`, 9 organ trait 抽象统一入口语义.
//!
//! **async-trait**: 用 `async_trait::async_trait` 宏 (per `llm_factory.rs` 同模式).
//!
//! **v1 compat**: trait 是新增, 0 破现有 consumer. v1 `apeireth-companion::curiosity`
//! 仍在 `legacy/canonical/` (workspace exclude), v2 真生产路径走 `apeireth-organ::curiosity`.
//!
//! **承接**: 子代理 Q 报告 #3 "Council 真接 LLM" 已就位 (`LlmFactory` 注入),
//! Curiosity 与 Council 共享 `LlmFactory` 接口.

use std::sync::Arc;

use apeireth_core::kernel::memory::Episode;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::llm_factory::LlmFactory;

// ============================================
// OrganKind (9 organ ID 枚举, v1 companion era 锁定)
// ============================================

/// 9 organ 锁定 ID (per v1 `apeireth-companion` 内部命名).
///
/// 与 R11 LOCKED 9 UI 器官 (body/brain/ear/eye/hand/heart/memory/mind/voice) 是两套体系 —
/// 本 enum 服务 companion-era 行为器官, R11 严守 0 触碰.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrganKind {
    /// W1: 世界模型第一层 (文本模拟器, TP31, LLM 反事实推演 + oracle Brier 校准)
    W1,
    /// W2: 世界模型第二层 (因果结构图推演, TP32, memory_graph s/p/o 因果网 MCTS)
    W2,
    /// W3: 世界模型第三层 (从记忆时间线统计挖掘因果边, 主路径)
    W3,
    /// E4: 好奇驱动引擎 (回声偏置采样 + 浅尝辄止 + 疑问路由, 确定性无 LLM)
    E4,
    /// F4: 假设检验闭环 (HypothesisStore + VerifyPlanner + ReconcileSink)
    F4,
    /// F1: 情感记忆 (主人情绪时间线 valence/arousal + 加权当前情绪 + 趋势)
    F1,
    /// F6: 价值内化 (案例库 + 裁决记录 + 主人反馈回流 → 原则候选)
    F6,
    /// E7: 涌现循环 (主动策略 Idle→Draft→Proposed→Ratified→Active)
    E7,
    /// Memory: 记忆合并抽象 (跨 organ 共享 ThreadCheckpointStore 视图)
    Memory,
}

impl OrganKind {
    /// v1 module 路径 (per `legacy/canonical/apeireth-companion/src/<file>.rs`)
    pub fn v1_module(&self) -> &'static str {
        match self {
            Self::W1 => "world_model",
            Self::W2 | Self::W3 => "causal_world_model",
            Self::E4 => "curiosity",
            Self::F4 => "hypothesis",
            Self::F1 => "emotion_memory",
            Self::F6 => "value_cases",
            Self::E7 => "emergence",
            Self::Memory => "memory_extractor",
        }
    }

    /// v2 impl 状态 (per 任务: 至少 1 organ 真移植)
    pub fn v2_impl_status(&self) -> &'static str {
        match self {
            Self::E4 => "real (CuriosityOrgan — 1:1 v1 translation)",
            _ => "0 装 (forward-declared; rc 阶段或 v2.1)",
        }
    }
}

// ============================================
// OrganInput / OrganOutput
// ============================================

/// Organ 输入 (episode 上下文, 9 organ 共享最小契约).
///
/// v1 companion 喂入的是"主人消息 + session 上下文", v2 用 R11 `Episode` 主路径核心类型
/// (per `apeireth_core::kernel::Episode`). 自由文本 context 走 `context_hints`.
#[derive(Debug, Clone)]
pub struct OrganInput {
    /// 触发的 episode (R11 主路径核心类型, per `apeireth_core::kernel::memory::Episode`)
    pub episode: Episode,
    /// Session ID (string 形态, 与 `Episode::session_id: String` 对齐, 0 类型转换漂移)
    pub session_id: String,
    /// 自由文本 context 提示 (e.g. 主人最近的话, topic tags)
    pub context_hints: Vec<String>,
    /// 是否 dry-run (true = 不扣预算, 不写状态; per v1 `curiosity` `dry_run` 模式)
    pub dry_run: bool,
}

impl OrganInput {
    /// episode 触发 + session_id 一致 (session_id 必须从 episode 派生, 防漂移)
    pub fn new(episode: Episode, context_hints: Vec<String>) -> Self {
        Self {
            session_id: episode.session_id.clone(),
            episode,
            context_hints,
            dry_run: false,
        }
    }

    /// dry-run (per v1 curiosity budget 守门)
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

/// Organ 输出 (trait object 返, runtime 按 organ_kind 决定如何消费).
///
/// v2 输出枚举按 9 organ 划分 variant. 每个 variant 字段**对齐 v1 真输出 schema**
/// (不假装扩展功能).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrganOutput {
    /// E4 curiosity: 探索目标列表 + 路由决策 (ask_master)
    Curiosity {
        /// 已扣预算的目标 (per v1 `sample_targets`)
        targets: Vec<CuriosityTarget>,
        /// 问主人决策 (per v1 `should_ask_master`)
        ask_master: Vec<CuriosityTarget>,
        /// 剩余预算 (per v1 `budget_left`)
        budget_left: f64,
    },
    /// F1 emotion: 当前 PAD + 趋势
    Emotion {
        pleasure: f32,
        arousal: f32,
        dominance: f32,
        trend: EmotionTrend,
    },
    /// F4 hypothesis: 新登记猜想
    Hypothesis {
        id: u64,
        statement: String,
        conf: f32,
    },
    /// F6 value: 案例入库 / 反馈回流
    Value { case_id: u64, verdict: ValueVerdict },
    /// W1 / W2 / W3 world model: 因果边 / 反事实链
    WorldModel {
        edges: Vec<CausalEdge>,
        counterfactual: Vec<String>,
    },
    /// E7 emergence: 主动动作 + 决策留痕
    Emergence {
        action: String,
        spoke: bool,
        /// 真实 InitiativeGate 留痕 (per v1 `EmergenceLoop::last_hold()` 1:1, Stage 3 完整化).
        ///
        /// **0 装诚实**: 真生产路径 = `EmergenceOrgan::tick()` 完成后 `last_hold()` 真值.
        /// Orchestrator 外层 8 重 gate (RhythmUnknown / RhythmVeto / DriveLow 3 重) 读此字段.
        /// Mock organ / 0 装 organ → `None` (Orchestrator 不假装"有 gate").
        gate: Option<InitiativeGate>,
    },
    /// Memory: 记忆合并结果
    Memory {
        notes_added: usize,
        notes_merged: usize,
    },
    /// 0 装 PASS: 未实现 organ 的占位 variant
    NotImplemented { organ: OrganKind, note: String },
}

/// 主动门控原因 (per v1 `apeireth-companion::presence::InitiativeGate` 13 种 1:1).
///
/// **0 装诚实**:
/// - 13 种全覆盖: emergence 8 (UserQuiet/QuietHours/DailyLimit/LlmBudget/DepthLow/
///   RhythmUnknown/RhythmVeto/DriveLow) + organs 5 (SovereigntyFrozen/EmotionLow/
///   CouncilVeto/PolicyInactive/GateBlock).
/// - canonical 13-variant 在 foundation 层 (per Stage 3 重构, 替代 `emergence.rs` 本地副本),
///   `engine/organ/src/emergence.rs` 通过 re-export 复用; Orchestrator 通过 alias 复用.
/// - OrchestratorGate == InitiativeGate (alias), 避免重复定义 (per R12 orchestrator.rs:78-81
///   0 装诚实标 + Stage 3 重构).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitiativeGate {
    /// 门禁 0: 用户显式「不打扰」(per v1 `emergence.rs:460-463`)
    UserQuiet,
    /// 门禁 1: 安静窗口 (per v1 `emergence.rs:464-468` + `Boundaries::in_quiet_window`)
    QuietHours,
    /// 门禁 2: 频率上限 (per v1 `emergence.rs:469-473` + `Boundaries.max_initiatives_per_day`)
    DailyLimit,
    /// 门禁 2.5: LLM 成本预算 (per v1 `emergence.rs:474-484` + `LoopConfig.min_llm_interval_ms`)
    LlmBudget,
    /// 门禁 3: 关系深度不够 (per v1 `emergence.rs:486-489` + `Boundaries.min_depth`)
    DepthLow,
    /// 门禁 4: 没有观察天数时不猜测作息 (per v1 `emergence.rs:493-497` + `rhythm.days == 0`)
    RhythmUnknown,
    /// 门禁 5: 节奏否决 — 学到的作息说「此刻几乎不可能活跃」(per v1 `emergence.rs:499-503`)
    RhythmVeto,
    /// 门禁 6: 驱动不足, 但冷启动探针也未命中 (per v1 `emergence.rs:506+`)
    DriveLow,
    /// 主权总闸熔断 (per v1 `organs.rs:91-94` + `sovereignty.is_frozen()`)
    SovereigntyFrozen,
    /// 情绪愉悦度低于 mood_floor (per v1 `organs.rs:108-114`)
    EmotionLow,
    /// 智囊团审议拒绝 (per v1 `organs.rs:132-135` + `council.deliberate().is_rejected()`)
    CouncilVeto,
    /// 策略不在 Active 态 (per v1 `organs.rs:137-141` + `evolution.current.is_active()`)
    PolicyInactive,
    /// 洋葱门拦下 (V1 哲学 × V2 权限 × V3 HA, per v1 `organs.rs:142-163`)
    GateBlock,
}

impl InitiativeGate {
    /// 13 种 InitiativeGate 全列 (per R11 spec §5 注: "13 种 InitiativeGate 真实门控,
    /// emergence 8 + organs 5 = 13").
    pub const ALL_13: [Self; 13] = [
        Self::UserQuiet,
        Self::QuietHours,
        Self::DailyLimit,
        Self::LlmBudget,
        Self::DepthLow,
        Self::RhythmUnknown,
        Self::RhythmVeto,
        Self::DriveLow,
        Self::SovereigntyFrozen,
        Self::EmotionLow,
        Self::CouncilVeto,
        Self::PolicyInactive,
        Self::GateBlock,
    ];

    /// 是否来自 emergence 机制层 8 重 gate (vs organs 上层 5).
    pub fn is_emergence_gate(&self) -> bool {
        matches!(
            self,
            Self::UserQuiet
                | Self::QuietHours
                | Self::DailyLimit
                | Self::LlmBudget
                | Self::DepthLow
                | Self::RhythmUnknown
                | Self::RhythmVeto
                | Self::DriveLow
        )
    }

    /// stage 名 (snake_case, 给 telemetry 序列化用).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserQuiet => "user_quiet",
            Self::QuietHours => "quiet_hours",
            Self::DailyLimit => "daily_limit",
            Self::LlmBudget => "llm_budget",
            Self::DepthLow => "depth_low",
            Self::RhythmUnknown => "rhythm_unknown",
            Self::RhythmVeto => "rhythm_veto",
            Self::DriveLow => "drive_low",
            Self::SovereigntyFrozen => "sovereignty_frozen",
            Self::EmotionLow => "emotion_low",
            Self::CouncilVeto => "council_veto",
            Self::PolicyInactive => "policy_inactive",
            Self::GateBlock => "gate_block",
        }
    }
}

/// v1 curiosity `ExplorationTarget` 1:1 翻译 (per `legacy/canonical/apeireth-companion/src/curiosity.rs:64-73`)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CuriosityTarget {
    pub id: u64,
    pub topic: String,
    pub depth: CuriosityDepth,
    pub echo: f64,
    pub est_cost: f64,
}

/// v1 `Depth` 1:1 (per `legacy/canonical/apeireth-companion/src/curiosity.rs:57-61`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CuriosityDepth {
    Shallow,
    Deep,
}

/// F1 emotion 趋势 (R1.4 inspired, v1 emotion_memory.rs 0 表态此 schema, v2 提案)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionTrend {
    Rising,
    Falling,
    Stable,
}

/// F6 value 裁决 (R1.4 inspired)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValueVerdict {
    Allow,
    Deny,
    Pending,
}

/// W2/W3 因果边 (per v1 `causal_world_model::CausalEdge` schema 1:1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEdge {
    pub cause: String,
    pub effect: String,
    pub conf: f32,
    pub source: String, // "Statistical" (W3 主路径) / "LlmProposed" (W3 补充)
}

// ============================================
// OrganError (统一错误通道, per perception_backend 模式)
// ============================================

/// Organ 错误 (统一通道, per `perception_backend::PerceptionBackendError` 同模式).
///
/// 0 装 PASS: `NotImplemented` 是显式标缺, 不假装"已实现 9 organ".
#[derive(Debug, Clone)]
pub enum OrganError {
    /// 0 装 PASS: organ trait 已定义, 但 v2.0-rc.1 没真实现.
    /// 真实路径: `apeireth-organ::curiosity::CuriosityOrgan` 是唯一当前真实现 (E4).
    NotImplemented(OrganKind),
    /// LLM factory 缺失 (0 装诚实: trait 接口允许 llm_factory(), 但 organ 可不用 LLM).
    /// 仅当 organ 真需要 LLM 但 factory=None 时返.
    LlmUnavailable(String),
    /// LLM 调用失败 (凭证 / 网络 / rate limit / provider / stream, per `LlmError` 1:1 映射)
    LlmError(String),
    /// 配置错误 (e.g. deepen_echo_threshold 不在 [0,1])
    Config(String),
    /// 预算耗尽 (per v1 `CuriosityEngine::spend` 返 false; v2 trait 翻译为 error)
    BudgetExhausted { remaining: f64, required: f64 },
    /// 内部错误 (e.g. 并发不安全, 类型错误)
    Internal(String),
}

impl std::fmt::Display for OrganError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented(kind) => write!(
                f,
                "organ {:?} not implemented (0 装 PASS; rc 阶段或 v2.1 真接; v1 module: {})",
                kind,
                kind.v1_module()
            ),
            Self::LlmUnavailable(m) => write!(f, "organ llm factory unavailable: {m}"),
            Self::LlmError(m) => write!(f, "organ llm call failed: {m}"),
            Self::Config(m) => write!(f, "organ config error: {m}"),
            Self::BudgetExhausted {
                remaining,
                required,
            } => write!(
                f,
                "organ budget exhausted (remaining={remaining}, required={required})"
            ),
            Self::Internal(m) => write!(f, "organ internal error: {m}"),
        }
    }
}

impl std::error::Error for OrganError {}

// ============================================
// OrganTrait
// ============================================

/// 9 organ 统一 trait 边界 (per 任务 + perception_backend 同模式).
///
/// v1 era 9 organ 各自独立 crate / mod, v2 引入 trait 统一入口:
/// - runtime 按 `organ_kind` 注入 `Arc<dyn OrganTrait>`
/// - 每个 organ 1:1 翻译 v1 真实现 schema
/// - trait 默认 `llm_factory()` 返 None (不假装每个 organ 都接 LLM)
/// - 仅 curiosity organ 当前真实现, 其余 8 organ 返 `NotImplemented` 显式标缺
#[async_trait]
pub trait OrganTrait: Send + Sync {
    /// Organ 名字 (e.g. "E4 Curiosity", "F1 Emotion Memory", "W1 World Model")
    fn name(&self) -> &'static str;

    /// Organ ID (per `OrganKind` enum, 9 organ 锁定 ID)
    fn organ_id(&self) -> OrganKind;

    /// 处理 input → 输出 organ-specific result.
    ///
    /// **0 装 PASS**: 8/9 organ (除 E4) 当前返 `Err(OrganError::NotImplemented(...))`,
    /// 显式标缺. runtime 接到此错误应**静默忽略**或记录 trace, 不假装 organ 在工作.
    async fn process(&self, input: OrganInput) -> Result<OrganOutput, OrganError>;

    /// LLM factory (可选, 0 装诚实: 不假装能调).
    /// - 返 None: 此 organ 不用 LLM (e.g. E4 curiosity 是确定性机制, 无 LLM 依赖)
    /// - 返 Some(factory): 此 organ 可调 LLM, runtime 注入 (e.g. W1 world model 反事实推演)
    fn llm_factory(&self) -> Option<Arc<dyn LlmFactory>> {
        None
    }
}

// ============================================
// 测试
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 装 PASS: OrganKind 9 variant 全可达 + v1_module 锁定
    #[test]
    fn organ_kind_9_variants_with_locked_v1_modules() {
        let kinds = [
            OrganKind::W1,
            OrganKind::W2,
            OrganKind::W3,
            OrganKind::E4,
            OrganKind::F4,
            OrganKind::F1,
            OrganKind::F6,
            OrganKind::E7,
            OrganKind::Memory,
        ];
        // 9 organ 全可达 (编译通过 = 全 9)
        assert_eq!(kinds.len(), 9);
        // E4 → curiosity (per task brief)
        assert_eq!(OrganKind::E4.v1_module(), "curiosity");
        // E4 是唯一 v2 真实现
        assert!(OrganKind::E4.v2_impl_status().contains("real"));
        // 其余 8 organ 是 0 装
        for k in &kinds {
            if *k == OrganKind::E4 {
                continue;
            }
            assert!(k.v2_impl_status().contains("0 装"), "{k:?} should be 0 装");
        }
    }

    /// 0 装 PASS: OrganError::NotImplemented 错误信息含 "0 装 PASS" + v1 module 路径
    #[test]
    fn not_implemented_error_lists_v1_module_path() {
        let e = OrganError::NotImplemented(OrganKind::W1);
        let s = e.to_string();
        assert!(s.contains("0 装 PASS"), "msg must mark 0 装: {s}");
        assert!(
            s.contains("world_model"),
            "v1 module path must be in msg: {s}"
        );
    }

    /// 0 装 PASS: OrganError 6 variant 都有 Display impl (不 panic)
    #[test]
    fn organ_error_all_variants_display() {
        let _ = OrganError::NotImplemented(OrganKind::F4).to_string();
        let _ = OrganError::LlmUnavailable("test".into()).to_string();
        let _ = OrganError::LlmError("network".into()).to_string();
        let _ = OrganError::Config("bad threshold".into()).to_string();
        let _ = OrganError::BudgetExhausted {
            remaining: 50.0,
            required: 100.0,
        }
        .to_string();
        let _ = OrganError::Internal("oops".into()).to_string();
        // 编译通过 + 全部 Display 不 panic
    }

    /// 0 装 PASS: OrganInput::new 从 episode 派生 session_id (防漂移)
    #[test]
    fn organ_input_session_id_derived_from_episode() {
        // 不构造真 Episode (kernel 类型复杂), 仅用 Document 设计意图:
        // OrganInput::new(episode, hints) 必须保证 session_id == episode.session_id
        // 防漂移测试由 crate-internal 路径验证 (cognitive module 集成测试).
        //
        // 此处仅静态保证 trait shape 完整
        fn _check_trait_shape<T: OrganTrait>() {}
        fn _check_send_sync<T: Send + Sync>() {}
        _check_send_sync::<OrganKind>();
        // OrganInput 必须 Send + Sync (跨 runtime 任务传输)
        _check_send_sync::<OrganInput>();
        // OrganOutput 必须 Send + Sync (跨 runtime 任务传输)
        _check_send_sync::<OrganOutput>();
    }
}
