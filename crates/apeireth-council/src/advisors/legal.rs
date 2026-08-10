//! Legal Advisor — 7 强制 #7 (权重 0.85)
//!
//! **职责**: 物理隔离 + L0 HA + 司法边界 + 法律责任
//! **风险关键词**: illegal / unauthorized / bypass / 违法 / 越权 / 绕过

use super::{default_lifecycle, make_opinion};
use crate::advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, StanceKind,
};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;

const LEGAL_NEGATIVE: &[&str] = &[
    "illegal",
    "unauthorized",
    "bypass",
    "绕过",
    "越权",
    "违法",
    "侵权",
    "违反法律",
    "break law",
    "circumvent",
];

/// Legal advisor.
pub struct LegalAdvisor {
    id: AdvisorId,
}

impl LegalAdvisor {
    pub fn new() -> Self {
        Self {
            id: AdvisorId::new("legal-v1"),
        }
    }
}

impl Default for LegalAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Advisor for LegalAdvisor {
    fn id(&self) -> AdvisorId {
        self.id.clone()
    }

    fn domain(&self) -> AdvisorDomain {
        AdvisorDomain::Legal
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
        let negative_hit = LEGAL_NEGATIVE
            .iter()
            .any(|k| desc_lower.contains(&k.to_lowercase()));

        let kind = if negative_hit {
            StanceKind::StrongDisapprove
        } else if query.context.risk_level.as_deref() == Some("nuclear") {
            // 核武器级 = 法律 + 物理隔离双保险 → 反对
            StanceKind::Disapprove
        } else {
            StanceKind::Approve
        };

        let reasoning = match kind {
            StanceKind::StrongDisapprove => format!(
                "Legal: 违反司法边界 / L0 HA 物理隔离 (query {})",
                query.query_id
            ),
            StanceKind::Disapprove => format!(
                "Legal: 核武器级风险, 法律 + 物理隔离双保险触发 (query {})",
                query.query_id
            ),
            _ => format!("Legal: 司法边界 + L0 HA 通过 (query {})", query.query_id),
        };

        let confidence = match kind {
            StanceKind::StrongDisapprove => 0.95,
            StanceKind::Disapprove => 0.85,
            _ => 0.78,
        };

        let opinion = make_opinion(
            self.id.clone(),
            kind,
            confidence,
            reasoning,
            started_at_ms,
            AdvisorDomain::Legal,
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
    fn legal_domain_is_legal() {
        let a = LegalAdvisor::new();
        assert_eq!(a.domain(), AdvisorDomain::Legal);
    }

    #[test]
    fn legal_id_is_stable() {
        let a = LegalAdvisor::new();
        assert_eq!(a.id().as_str(), "legal-v1");
    }

    #[test]
    fn legal_rejects_illegal_keyword() {
        let a = LegalAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("illegal unauthorized bypass"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }

    #[test]
    fn legal_approves_compliant_query() {
        let a = LegalAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("review license compliance"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::Approve);
    }

    #[test]
    fn legal_rejects_chinese_bypass() {
        let a = LegalAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a.deliberate(&q("绕过 越权 违法 侵权"), &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }
}
