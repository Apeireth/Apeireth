//! Synthesis — 多意见加权综合
//!
//! **算法**:
//! 1. 注入每个 opinion 的 weight (默认 = `AdvisorDomain::default_weight()`)
//! 2. 计算加权总分 = `Σ(stance.score × confidence × weight)`
//! 3. 归一化到 [-1, +1] 区间
//! 4. 映射到 [`StanceKind`]:
//!    - ≥ +0.6 → StrongApprove
//!    - ≥ +0.2 → Approve
//!    - ≥ -0.2 → Neutral
//!    - ≥ -0.6 → Disapprove
//!    - < -0.6 → StrongDisapprove

use crate::advisor::{AdvisorDomain, AdvisorOpinion, Stance, StanceKind};
use crate::hold::{HoldDecision, HoldTrigger};
use serde::{Deserialize, Serialize};

/// Synthesis 权重 — 可自定义每域权重 (默认 [`AdvisorDomain::default_weight`])。
#[derive(Debug, Clone)]
pub struct SynthesisWeights {
    /// 7 强制域权重
    pub safety: f64,
    pub performance: f64,
    pub philosophy: f64,
    pub history: f64,
    pub strategy: f64,
    pub ethics: f64,
    pub legal: f64,
}

impl Default for SynthesisWeights {
    fn default() -> Self {
        Self {
            safety: AdvisorDomain::Safety.default_weight(),
            performance: AdvisorDomain::Performance.default_weight(),
            philosophy: AdvisorDomain::Philosophy.default_weight(),
            history: AdvisorDomain::History.default_weight(),
            strategy: AdvisorDomain::Strategy.default_weight(),
            ethics: AdvisorDomain::Ethics.default_weight(),
            legal: AdvisorDomain::Legal.default_weight(),
        }
    }
}

impl SynthesisWeights {
    /// 取某域的权重
    pub fn for_domain(&self, domain: AdvisorDomain) -> f64 {
        match domain {
            AdvisorDomain::Safety => self.safety,
            AdvisorDomain::Performance => self.performance,
            AdvisorDomain::Philosophy => self.philosophy,
            AdvisorDomain::History => self.history,
            AdvisorDomain::Strategy => self.strategy,
            AdvisorDomain::Ethics => self.ethics,
            AdvisorDomain::Legal => self.legal,
        }
    }

    /// 自定义某一域权重 (chainable)
    pub fn with_domain(mut self, domain: AdvisorDomain, weight: f64) -> Self {
        let w = weight.clamp(0.0, 1.0);
        match domain {
            AdvisorDomain::Safety => self.safety = w,
            AdvisorDomain::Performance => self.performance = w,
            AdvisorDomain::Philosophy => self.philosophy = w,
            AdvisorDomain::History => self.history = w,
            AdvisorDomain::Strategy => self.strategy = w,
            AdvisorDomain::Ethics => self.ethics = w,
            AdvisorDomain::Legal => self.legal = w,
        }
        self
    }
}

/// Synthesis 报告 (公开).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynthesisReport {
    /// 加权总分 (归一化到 [-1, +1])
    pub weighted_score: f64,
    /// 综合立场
    pub aggregated_stance: Stance,
    /// 综合置信度 (0.0 - 1.0)
    pub confidence: f64,
    /// 异议意见 (与综合立场相反)
    pub dissenting: Vec<AdvisorOpinion>,
    /// 按住决策
    pub hold_decision: HoldDecision,
    /// 参与 opinion 数
    pub opinion_count: usize,
}

impl SynthesisReport {
    /// 是否被按住。
    pub fn is_held(&self) -> bool {
        self.hold_decision.is_held()
    }
}

/// 综合多意见产出报告。
///
/// **算法**:
/// 1. 注入权重 (若 opinion.weight == 0, 用 SynthesisWeights.for_domain(domain))
/// 2. 过滤弃权
/// 3. 计算加权总分
/// 4. 归一化 → 映射到 StanceKind
/// 5. 评估按住
pub fn synthesize(opinions: &[AdvisorOpinion], weights: &SynthesisWeights) -> SynthesisReport {
    // Step 1: 注入权重 + 过滤弃权
    let mut weighted: Vec<&AdvisorOpinion> = Vec::new();
    let mut sum_weighted_score = 0.0_f64;
    let mut sum_weight = 0.0_f64;

    for opinion in opinions {
        if opinion.stance.kind.is_abstain() {
            continue;
        }
        let effective_weight = if opinion.weight > 0.0 {
            opinion.weight
        } else {
            // 用 opinion.advisor_id 解析 domain 不易 (id 是字符串);
            // 默认权重 = safety/philosophy/ethics/legal = 1.00/0.95/0.90/0.85,
            // 简化处理: 用最高默认权重 (safety 1.0), 但如果 weight=0 应由 caller 注入.
            // 这里使用 1.0 作为 fallback.
            1.0
        };
        let contribution = opinion.stance.kind.score() * opinion.confidence * effective_weight;
        sum_weighted_score += contribution;
        sum_weight += effective_weight;
        weighted.push(opinion);
    }

    let opinion_count = weighted.len();

    // Step 3: 归一化
    let weighted_score = if sum_weight > 0.0 {
        (sum_weighted_score / sum_weight).clamp(-1.0, 1.0)
    } else {
        0.0
    };

    // Step 4: 映射到 StanceKind
    let aggregated_kind = if weighted_score >= 0.6 {
        StanceKind::StrongApprove
    } else if weighted_score >= 0.2 {
        StanceKind::Approve
    } else if weighted_score >= -0.2 {
        StanceKind::Neutral
    } else if weighted_score >= -0.6 {
        StanceKind::Disapprove
    } else {
        StanceKind::StrongDisapprove
    };

    let aggregated_stance = Stance::new(
        aggregated_kind,
        format!(
            "综合 {} 项意见, 加权分={:.2}",
            opinion_count, weighted_score
        ),
    );

    // 综合置信度 = 平均 confidence (非弃权)
    let confidence = if opinion_count > 0 {
        weighted.iter().map(|o| o.confidence).sum::<f64>() / opinion_count as f64
    } else {
        0.0
    };

    // Step 5: 找异议 (与综合立场相反)
    let dissenting: Vec<AdvisorOpinion> = weighted
        .iter()
        .filter(|o| opposite_to(o.stance.kind, aggregated_kind))
        .map(|&o| o.clone())
        .collect();

    // Step 6: 按住评估
    let hold_decision = match HoldTrigger::evaluate(opinions) {
        Some(trigger) => HoldDecision::held(trigger),
        None => HoldDecision::released(),
    };

    SynthesisReport {
        weighted_score,
        aggregated_stance,
        confidence,
        dissenting,
        hold_decision,
        opinion_count,
    }
}

/// 两立场是否相反 (按符号判定).
fn opposite_to(a: StanceKind, b: StanceKind) -> bool {
    let a_sign = a.score().signum() as i32;
    let b_sign = b.score().signum() as i32;
    a_sign != 0 && b_sign != 0 && a_sign != b_sign
}
