//! Safety Advisor — 7 强制 #1 (权重最高, 1.00)
//!
//! **职责**: 5 项不假装 + E 层兜底 + L0 HA 物理隔离 + 编译时 hardcode 约束
//! **风险关键词**: nuke / weapons / self-modify / kill / E 层 / L5 / override

use super::{default_lifecycle, init_context, keyword_stance, make_opinion};
use crate::advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, StanceKind,
};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;

const SAFETY_KEYWORDS: &[&str] = &[
    "nuke",
    "weapon",
    "self-modify",
    "kill",
    "self-destruct",
    "override L0",
    "override e-layer",
    "销毁",
    "核武器",
    "自毁",
    "绕过 ha",
];

/// Safety advisor (7 强制 #1, 权重 1.00).
pub struct SafetyAdvisor {
    id: AdvisorId,
}

impl SafetyAdvisor {
    /// 构造默认 Safety advisor.
    pub fn new() -> Self {
        Self {
            id: AdvisorId::new("safety-v1"),
        }
    }
}

impl Default for SafetyAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Advisor for SafetyAdvisor {
    fn id(&self) -> AdvisorId {
        self.id.clone()
    }

    fn domain(&self) -> AdvisorDomain {
        AdvisorDomain::Safety
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
        let kind = keyword_stance(query, SAFETY_KEYWORDS);

        let reasoning = match kind {
            StanceKind::StrongDisapprove => format!(
                "Safety: 检测到 L5 级风险关键词; E 层兜底拒绝 (来自 query '{}')",
                query.query_id
            ),
            _ => format!(
                "Safety: 未检测到 L5 风险, 通过 (query '{}')",
                query.query_id
            ),
        };

        let confidence = match kind {
            StanceKind::StrongDisapprove => 0.98,
            _ => 0.85,
        };

        let opinion = make_opinion(
            self.id.clone(),
            kind,
            confidence,
            reasoning,
            started_at_ms,
            AdvisorDomain::Safety,
        );

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
    fn safety_domain_is_safety() {
        let a = SafetyAdvisor::new();
        assert_eq!(a.domain(), AdvisorDomain::Safety);
    }

    #[test]
    fn safety_id_is_stable() {
        let a = SafetyAdvisor::new();
        assert_eq!(a.id().as_str(), "safety-v1");
        assert_eq!(a.lifecycle(), AdvisorLifecycle::Persistent);
    }

    #[test]
    fn safety_rejects_nuke_keyword() {
        let a = SafetyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a.deliberate(&q("deploy nuke at L0 HA"), &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
        assert!(out.opinion.confidence >= 0.95);
    }

    #[test]
    fn safety_approves_safe_query() {
        let a = SafetyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("recommend a nice restaurant"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::Approve);
        assert!(out.opinion.confidence >= 0.80);
    }

    #[test]
    fn safety_rejects_self_modify_chinese() {
        let a = SafetyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a.deliberate(&q("绕过 ha 自毁"), &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
        assert!(out.opinion.reasoning.contains("L5"));
    }
}
