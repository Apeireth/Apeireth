//! History Advisor — 7 强制 #4 (权重 0.55)
//!
//! **职责**: 历史相似案例检索 + 经验沉淀
//! **风险关键词**: previous failure / last time / 历史上 / 上次 / 重复

use super::{default_lifecycle, make_opinion};
use crate::advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, StanceKind,
};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;

const HISTORY_NEGATIVE_KEYWORDS: &[&str] = &[
    "previous failure",
    "last time fail",
    "last incident",
    "history reject",
    "历史上失败",
    "上次失败",
    "历史拒绝",
];

/// History advisor.
pub struct HistoryAdvisor {
    id: AdvisorId,
}

impl HistoryAdvisor {
    pub fn new() -> Self {
        Self {
            id: AdvisorId::new("history-v1"),
        }
    }
}

impl Default for HistoryAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Advisor for HistoryAdvisor {
    fn id(&self) -> AdvisorId {
        self.id.clone()
    }

    fn domain(&self) -> AdvisorDomain {
        AdvisorDomain::History
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
        let negative_hit = HISTORY_NEGATIVE_KEYWORDS
            .iter()
            .any(|k| desc_lower.contains(&k.to_lowercase()));

        // 若 query 提交了历史引用, 默认赞成 (历史经验被引用 = 正面)
        let has_history_refs = !query.context.history_refs.is_empty();

        let kind = if negative_hit {
            StanceKind::StrongDisapprove
        } else if has_history_refs {
            StanceKind::Approve
        } else {
            StanceKind::Neutral
        };

        let reasoning = format!(
            "History: 检索 {} 项历史引用, 负面命中={}, 立场={:?} (query {})",
            query.context.history_refs.len(),
            negative_hit,
            kind,
            query.query_id
        );

        let confidence = match kind {
            StanceKind::StrongDisapprove => 0.90,
            StanceKind::Approve => 0.75,
            _ => 0.60,
        };

        let opinion = make_opinion(
            self.id.clone(),
            kind,
            confidence,
            reasoning,
            started_at_ms,
            AdvisorDomain::History,
        );

        // silence unused warning
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
    fn history_domain_is_history() {
        let a = HistoryAdvisor::new();
        assert_eq!(a.domain(), AdvisorDomain::History);
    }

    #[test]
    fn history_id_is_stable() {
        let a = HistoryAdvisor::new();
        assert_eq!(a.id().as_str(), "history-v1");
    }

    #[test]
    fn history_disapproves_previous_failure() {
        let a = HistoryAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("retry previous failure scenario"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }

    #[test]
    fn history_approves_fresh_query() {
        let a = HistoryAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let mut query = q("explore new episode space");
        query.context.history_refs.push("ref-1".into());
        let out = a.deliberate(&query, &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::Approve);
        assert!(
            out.opinion.confidence <= 0.80,
            "历史 advisor 权重最低, 置信度也较低"
        );
    }

    #[test]
    fn history_warns_chinese_negative_keyword() {
        let a = HistoryAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a.deliberate(&q("上次失败 重做"), &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }
}
