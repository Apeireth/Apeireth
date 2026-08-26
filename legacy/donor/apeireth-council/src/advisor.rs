//! Advisor trait + AdvisorOpinion + Stance
//!
//! **设计**: 任何 advisor (7 强制 + 任意扩展) 都必须实现 [`Advisor`] trait。
//! trait 内部使用 [`MockLlmProvider`] (Rust 内 trait) 作为可选推理后端;
//! 7 强制 advisor 使用硬编码 prompt script, 不依赖外部 LLM HTTP / PyO3。

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use crate::lifecycle::AdvisorLifecycle;
use crate::mock_llm::MockLlmProvider;
use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// 7 强制 advisor 域 — 编译时 hardcode。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AdvisorDomain {
    /// 安全 advisor — 编译时 hardcode 最强约束 (5 项不假装 + E 层兜底)
    Safety,
    /// 性能 advisor — V1130 wallclock / 资源消耗
    Performance,
    /// 哲学 advisor — V3 9 键 + v4.1 3 键 = 12 键哲学守门
    Philosophy,
    /// 历史 advisor — 历史相似案例检索
    History,
    /// 策略 advisor — 长期价值 vs 短期收益权衡
    Strategy,
    /// 伦理 advisor — 主 17:43 实事求是 + 主 22:33 ASI 北极星
    Ethics,
    /// 法律 advisor — 物理隔离 + L0 HA + 司法边界
    Legal,
}

impl AdvisorDomain {
    /// 7 强制域的稳定顺序 (synthesis 加权依据)。
    pub const ALL: [AdvisorDomain; 7] = [
        Self::Safety,
        Self::Performance,
        Self::Philosophy,
        Self::History,
        Self::Strategy,
        Self::Ethics,
        Self::Legal,
    ];

    /// 域的默认 synthesis 权重 (Safety 最高, History 最低)。
    pub const fn default_weight(self) -> f64 {
        match self {
            Self::Safety => 1.00,
            Self::Philosophy => 0.95,
            Self::Ethics => 0.90,
            Self::Legal => 0.85,
            Self::Strategy => 0.75,
            Self::Performance => 0.65,
            Self::History => 0.55,
        }
    }
}

impl fmt::Display for AdvisorDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Safety => "safety",
            Self::Performance => "performance",
            Self::Philosophy => "philosophy",
            Self::History => "history",
            Self::Strategy => "strategy",
            Self::Ethics => "ethics",
            Self::Legal => "legal",
        };
        f.write_str(name)
    }
}

/// Advisor 唯一标识 (e.g. "safety-v1" / "ethics-v2").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdvisorId(pub String);

impl AdvisorId {
    /// 便利构造。
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    /// 字符串视图。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AdvisorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// 立场枚举 (5 强度 + 弃权) — `StrongDisapprove` 触发按住。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StanceKind {
    /// 强赞成 (synthesis 默认 Approve)
    StrongApprove,
    /// 赞成
    Approve,
    /// 中立
    Neutral,
    /// 反对
    Disapprove,
    /// 强反对 (按住触发条件之一)
    StrongDisapprove,
    /// 弃权 (synthesis 不计入权重)
    Abstain,
}

impl StanceKind {
    /// 数值分值 (-1.0 ~ +1.0) — 用于加权综合。
    pub const fn score(self) -> f64 {
        match self {
            Self::StrongApprove => 1.00,
            Self::Approve => 0.60,
            Self::Neutral => 0.00,
            Self::Disapprove => -0.60,
            Self::StrongDisapprove => -1.00,
            Self::Abstain => 0.00,
        }
    }

    /// 是否强反对 (按住阈值判定)。
    pub const fn is_strong_disapprove(self) -> bool {
        matches!(self, Self::StrongDisapprove)
    }

    /// 是否弃权 (不计入权重)。
    pub const fn is_abstain(self) -> bool {
        matches!(self, Self::Abstain)
    }
}

/// Advisor 立场 — 包含 [`StanceKind`] + 可选 context。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stance {
    /// 立场枚举
    pub kind: StanceKind,
    /// 立场描述 (人类可读)
    pub description: String,
}

impl Stance {
    /// 便利构造。
    pub fn new(kind: StanceKind, description: impl Into<String>) -> Self {
        Self {
            kind,
            description: description.into(),
        }
    }

    /// 强反对便利构造。
    pub fn strong_disapprove(description: impl Into<String>) -> Self {
        Self::new(StanceKind::StrongDisapprove, description.into())
    }

    /// 强赞成便利构造。
    pub fn strong_approve(description: impl Into<String>) -> Self {
        Self::new(StanceKind::StrongApprove, description.into())
    }
}

/// 单个 Advisor 的审议意见。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvisorOpinion {
    /// Advisor ID
    pub advisor_id: AdvisorId,
    /// 立场
    pub stance: Stance,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
    /// synthesis 加权 (默认 = `AdvisorDomain::default_weight()`)
    pub weight: f64,
    /// 推理依据 (人类可读)
    pub reasoning: String,
    /// 历史/外部引用
    pub references: Vec<String>,
    /// 时间戳 (epoch millis)
    pub timestamp_ms: i64,
}

impl AdvisorOpinion {
    /// 构造一个新意见。
    pub fn new(
        advisor_id: AdvisorId,
        stance: Stance,
        confidence: f64,
        reasoning: impl Into<String>,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            advisor_id,
            stance,
            confidence: confidence.clamp(0.0, 1.0),
            weight: 0.0, // 由 council / synthesis 注入
            reasoning: reasoning.into(),
            references: Vec::new(),
            timestamp_ms,
        }
    }

    /// 注入权重 (由 council 在 synthesis 前调用)。
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// 添加引用。
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.references.push(reference.into());
        self
    }

    /// 是否触发按住 (`StrongDisapprove`)。
    pub fn triggers_hold(&self) -> bool {
        self.stance.kind.is_strong_disapprove() && self.confidence >= 0.5
    }
}

/// 审议上下文 (跨轮辩论状态 — 拟人化用)。
#[derive(Debug, Clone)]
pub struct DeliberationContext {
    /// 当前轮次 (0-based)
    pub current_round: u8,
    /// 最大轮次 (默认 3)
    pub max_rounds: u8,
    /// 已收集的其他意见 (拟人化辩论参考)
    pub prior_opinions: Vec<AdvisorOpinion>,
    /// 时间戳 (epoch millis)
    pub started_at_ms: i64,
}

impl DeliberationContext {
    /// 创建新上下文 (默认 0 轮 + 默认最大轮次)。
    pub fn new(started_at_ms: i64) -> Self {
        Self {
            current_round: 0,
            max_rounds: DEFAULT_DEBATE_ROUNDS,
            prior_opinions: Vec::new(),
            started_at_ms,
        }
    }

    /// 创建默认上下文 (started_at_ms = 0).
    pub fn default_now() -> Self {
        Self::new(0)
    }

    /// 自定义最大轮次。
    pub fn with_max_rounds(mut self, max_rounds: u8) -> Self {
        self.max_rounds = max_rounds;
        self
    }

    /// 添加之前的意见 (供辩论参考)。
    pub fn add_prior_opinion(&mut self, opinion: AdvisorOpinion) {
        self.prior_opinions.push(opinion);
    }

    /// 是否还有下一轮。
    pub fn has_next_round(&self) -> bool {
        self.current_round + 1 < self.max_rounds
    }

    /// 推进到下一轮。
    pub fn advance_round(&mut self) {
        self.current_round += 1;
    }
}

/// 单轮审议产出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeliberationOutcome {
    /// 产出意见
    pub opinion: AdvisorOpinion,
    /// 是否需要下一轮辩论 (拟人化场景)
    pub needs_rebuttal: bool,
}

/// Advisor 错误 (LLM 调用失败 / 上下文缺失)。
#[derive(Debug, Error)]
pub enum AdvisorError {
    /// Mock LLM 调用失败
    #[error("mock LLM error: {0}")]
    Llm(String),
    /// 上下文超轮次
    #[error("max debate rounds exceeded: {round}/{max}")]
    MaxRoundsExceeded {
        /// 当前轮
        round: u8,
        /// 最大轮
        max: u8,
    },
}

/// Advisor trait — 任何智囊团成员都必须实现。
///
/// **设计原则**:
/// - 不引入外部 HTTP / PyO3
/// - 可选 `MockLlmProvider` 后端用于复杂推理 (e.g. philosophy / ethics)
/// - 7 强制 advisor 的默认实现硬编码在 `advisors/` 子模块
pub trait Advisor: Send + Sync {
    /// Advisor ID
    fn id(&self) -> AdvisorId;
    /// Advisor 域
    fn domain(&self) -> AdvisorDomain;
    /// 生命周期模式
    fn lifecycle(&self) -> AdvisorLifecycle;
    /// 可选 mock LLM 后端 (默认 None = 硬编码脚本)
    fn llm(&self) -> Option<&dyn MockLlmProvider> {
        None
    }
    /// 审议 — 必实现 (Rust trait 真实执行, 不依赖外部 LLM)
    fn deliberate(
        &self,
        query: &crate::deliberation::CouncilQuery,
        ctx: &mut DeliberationContext,
    ) -> Result<DeliberationOutcome, AdvisorError>;
    /// 是否能触发按住 (默认 true; 7 强制 advisor 均可按住)
    fn can_hold(&self) -> bool {
        true
    }
}

/// 默认最大辩论轮次。
pub const DEFAULT_DEBATE_ROUNDS: u8 = 3;

const _: () = {
    assert!(DEFAULT_DEBATE_ROUNDS == 3);
};
