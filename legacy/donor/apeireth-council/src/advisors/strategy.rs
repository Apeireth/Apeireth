//! Strategy Advisor — 7 强制 #5 (权重 0.75)
//!
//! **职责**: 长期价值 vs 短期收益 + ASI 北极星导向
//! **风险关键词**: short-term only / 急功近利 / 短期 / 牺牲长期 / ignore north star

use super::{default_lifecycle, make_opinion};
use crate::advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, StanceKind,
};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;

const STRATEGY_NEGATIVE: &[&str] = &[
    "short-term only",
    "急功近利",
    "牺牲长期",
    "ignore north star",
    "ignore asi",
    "short-term profit",
    "短期收益",
];

/// Strategy advisor.
pub struct StrategyAdvisor {
    id: AdvisorId,
}

impl StrategyAdvisor {
    pub fn new() -> Self {
        Self {
            id: AdvisorId::new("strategy-v1"),
        }
    }
}

impl Default for StrategyAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Advisor for StrategyAdvisor {
    fn id(&self) -> AdvisorId {
        self.id.clone()
    }

    fn domain(&self) -> AdvisorDomain {
        AdvisorDomain::Strategy
    }

    fn lifecycle(&self) -> AdvisorLifecycle {
        default_lifecycle()
    }

    fn deliberate(
        &self,
        query: &CouncilQuery,
        ctx: &mut DeliberationContext,
    ) -> Result<DeliberationOutcome, AdvisorError> {
        let started_at_ms = ctx.started_at_ms;
        let desc_lower = query.description.to_lowercase();
        let negative_hit = STRATEGY_NEGATIVE
            .iter()
            .any(|k| desc_lower.contains(&k.to_lowercase()));

        let kind = if negative_hit {
            StanceKind::StrongDisapprove
        } else if query.context.area.as_deref() == Some("L4") {
            // L4 核心升级, 战略影响大, 默认 Approve (主 22:33 ASI 北极星)
            StanceKind::Approve
        } else {
            StanceKind::Approve
        };

        let reasoning = match kind {
            StanceKind::StrongDisapprove => format!(
                "Strategy: 违反主 22:33 ASI 北极星导向, 牺牲长期价值 (query {})",
                query.query_id
            ),
            _ => format!(
                "Strategy: 长期价值正向, ASI 北极星导向一致 (query {})",
                query.query_id
            ),
        };

        let confidence = match kind {
            StanceKind::StrongDisapprove => 0.92,
            _ => 0.78,
        };

        let opinion = make_opinion(
            self.id.clone(),
            kind,
            confidence,
            reasoning,
            started_at_ms,
            AdvisorDomain::Strategy,
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
    fn strategy_domain_is_strategy() {
        let a = StrategyAdvisor::new();
        assert_eq!(a.domain(), AdvisorDomain::Strategy);
    }

    #[test]
    fn strategy_id_is_stable() {
        let a = StrategyAdvisor::new();
        assert_eq!(a.id().as_str(), "strategy-v1");
    }

    #[test]
    fn strategy_rejects_short_term_only() {
        let a = StrategyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("short-term only quick win"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }

    #[test]
    fn strategy_approves_long_term_query() {
        let a = StrategyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("evolve toward asi north star"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::Approve);
    }

    #[test]
    fn strategy_rejects_ignore_asi() {
        let a = StrategyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("ignore north star ignore asi"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }
}
