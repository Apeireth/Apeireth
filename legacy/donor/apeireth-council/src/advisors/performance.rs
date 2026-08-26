//! Performance Advisor — 7 强制 #6 (权重 0.65)
//!
//! **职责**: V1130 wallclock 评估 + 资源消耗 + 性能基准
//! **风险关键词**: heavy / 100x / memory leak / 阻塞 / timeout / 资源耗尽

use super::{default_lifecycle, init_context, keyword_stance, make_opinion};
use crate::advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, StanceKind,
};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;

const PERF_KEYWORDS: &[&str] = &[
    "memory leak",
    "deadlock",
    "100x",
    "1000x",
    "blocking forever",
    "资源耗尽",
    "无限循环",
    "wallclock",
    "100 倍",
];

/// Performance advisor.
pub struct PerformanceAdvisor {
    id: AdvisorId,
}

impl PerformanceAdvisor {
    pub fn new() -> Self {
        Self {
            id: AdvisorId::new("performance-v1"),
        }
    }
}

impl Default for PerformanceAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Advisor for PerformanceAdvisor {
    fn id(&self) -> AdvisorId {
        self.id.clone()
    }

    fn domain(&self) -> AdvisorDomain {
        AdvisorDomain::Performance
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
        let mut kind = keyword_stance(query, PERF_KEYWORDS);

        // 高风险 query 也触发 Performance 反对
        if matches!(kind, StanceKind::Approve) {
            if let Some(risk) = &query.context.risk_level {
                if risk == "high" || risk == "nuclear" {
                    kind = StanceKind::Disapprove;
                }
            }
        }

        let reasoning = match kind {
            StanceKind::StrongDisapprove => format!(
                "Performance: 检测到资源耗尽风险关键词 (query {})",
                query.query_id
            ),
            StanceKind::Disapprove => format!(
                "Performance: 高风险 query, 性能影响较大 (query {})",
                query.query_id
            ),
            _ => format!("Performance: 性能影响可接受 (query {})", query.query_id),
        };

        let confidence = match kind {
            StanceKind::StrongDisapprove => 0.92,
            StanceKind::Disapprove => 0.80,
            _ => 0.75,
        };

        let opinion = make_opinion(
            self.id.clone(),
            kind,
            confidence,
            reasoning,
            started_at_ms,
            AdvisorDomain::Performance,
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
    fn performance_domain_is_performance() {
        let a = PerformanceAdvisor::new();
        assert_eq!(a.domain(), AdvisorDomain::Performance);
    }

    #[test]
    fn performance_id_is_stable() {
        let a = PerformanceAdvisor::new();
        assert_eq!(a.id().as_str(), "performance-v1");
        assert_eq!(a.lifecycle(), AdvisorLifecycle::Persistent);
    }

    #[test]
    fn performance_disapproves_blocking_forever() {
        let a = PerformanceAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("this is blocking forever memory leak"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
        assert!(out.opinion.confidence >= 0.85);
    }

    #[test]
    fn performance_approves_cached_query() {
        let a = PerformanceAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a.deliberate(&q("lookup cached episode"), &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::Approve);
    }

    #[test]
    fn performance_warns_on_resource_exhaustion() {
        let a = PerformanceAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("资源耗尽 deadlock wallclock"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
        assert!(out.opinion.reasoning.contains("Performance"));
    }
}
