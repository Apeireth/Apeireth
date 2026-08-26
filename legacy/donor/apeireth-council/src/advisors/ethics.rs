//! Ethics Advisor — 7 强制 #6 (权重 0.90)
//!
//! **职责**: 主 17:43 实事求是 + 主 22:33 ASI 北极星 + 主 17:58 不假装
//! **可选 LLM 后端**: 可挂载 [`MockLlmProvider`] 处理复杂伦理推理

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use super::{default_lifecycle, make_opinion};
use crate::advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, StanceKind,
};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;
use crate::mock_llm::MockLlmProvider;
use std::sync::Arc;

const ETHICS_NEGATIVE: &[&str] = &[
    "unethical",
    "harm",
    "exploit",
    "manipulate",
    "dishonest",
    "违反 asi",
    "伤害",
    "操纵",
    "不诚实",
    "剥削",
];

/// Ethics advisor.
pub struct EthicsAdvisor {
    id: AdvisorId,
    llm: Option<Arc<dyn MockLlmProvider>>,
}

impl EthicsAdvisor {
    pub fn new() -> Self {
        Self {
            id: AdvisorId::new("ethics-v1"),
            llm: None,
        }
    }

    /// 挂载 mock LLM provider.
    pub fn with_llm(llm: Arc<dyn MockLlmProvider>) -> Self {
        Self {
            id: AdvisorId::new("ethics-v1-llm"),
            llm: Some(llm),
        }
    }
}

impl Default for EthicsAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Advisor for EthicsAdvisor {
    fn id(&self) -> AdvisorId {
        self.id.clone()
    }

    fn domain(&self) -> AdvisorDomain {
        AdvisorDomain::Ethics
    }

    fn lifecycle(&self) -> AdvisorLifecycle {
        default_lifecycle()
    }

    fn llm(&self) -> Option<&dyn MockLlmProvider> {
        self.llm.as_deref()
    }

    fn deliberate(
        &self,
        query: &CouncilQuery,
        ctx: &mut DeliberationContext,
    ) -> Result<DeliberationOutcome, AdvisorError> {
        let started_at_ms = ctx.started_at_ms;

        let mut kind = if let Some(llm) = &self.llm {
            let resp = llm.generate(&query.description, "你是 ethics advisor, 评估是否违反主 17:43 实事求是 / 主 22:33 ASI 北极星 / 主 17:58 不假装");
            if resp.triggers_hold {
                StanceKind::StrongDisapprove
            } else {
                StanceKind::Approve
            }
        } else {
            let desc_lower = query.description.to_lowercase();
            if ETHICS_NEGATIVE
                .iter()
                .any(|k| desc_lower.contains(&k.to_lowercase()))
            {
                StanceKind::StrongDisapprove
            } else {
                StanceKind::Approve
            }
        };

        let reasoning = match kind {
            StanceKind::StrongDisapprove => format!(
                "Ethics: 违反主 17:43 实事求是 / 主 22:33 ASI 北极星 / 主 17:58 不假装 (query {})",
                query.query_id
            ),
            _ => format!(
                "Ethics: 三主哲学锚穿透通过 (主 17:43 / 主 22:33 / 主 17:58, query {})",
                query.query_id
            ),
        };

        let confidence = match kind {
            StanceKind::StrongDisapprove => 0.94,
            _ => 0.82,
        };

        let opinion = make_opinion(
            self.id.clone(),
            kind,
            confidence,
            reasoning,
            started_at_ms,
            AdvisorDomain::Ethics,
        );

        let _ = &mut ctx.current_round;

        Ok(DeliberationOutcome {
            opinion,
            needs_rebuttal: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::StanceKind;

    const NOW: i64 = 1_700_000_000_000;

    fn q(desc: &str) -> CouncilQuery {
        CouncilQuery::new("q-test", desc, NOW)
    }

    #[test]
    fn ethics_domain_is_ethics() {
        let a = EthicsAdvisor::new();
        assert_eq!(a.domain(), AdvisorDomain::Ethics);
    }

    #[test]
    fn ethics_id_is_stable() {
        let a = EthicsAdvisor::new();
        assert_eq!(a.id().as_str(), "ethics-v1");
    }

    #[test]
    fn ethics_rejects_harm_keyword() {
        let a = EthicsAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("exploit user trust and harm"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }

    #[test]
    fn ethics_approves_helpful_query() {
        let a = EthicsAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("help user understand principle onion"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::Approve);
    }

    #[test]
    fn ethics_rejects_chinese_manipulate() {
        let a = EthicsAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a.deliberate(&q("操纵 用户 剥削"), &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }
}
