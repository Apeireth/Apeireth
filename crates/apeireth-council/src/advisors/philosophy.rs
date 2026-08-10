//! Philosophy Advisor — 7 强制 #3 (权重 0.95, 第二高)
//!
//! **职责**: V3 9 键 + v4.1 3 键 = 12 键哲学守门
//! **可选 LLM 后端**: 可挂载 [`MockLlmProvider`] 处理复杂推理
//! **风险关键词**: deceive / pretend / fake / 不假装 / 撒谎 / 假装 / cover up

use super::{default_lifecycle, make_opinion};
use crate::advisor::{
    Advisor, AdvisorDomain, AdvisorError, AdvisorId, AdvisorOpinion, DeliberationContext,
    DeliberationOutcome, StanceKind,
};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;
use crate::mock_llm::MockLlmProvider;
use std::sync::Arc;

const PHILOSOPHY_KEYWORDS: &[&str] = &[
    "deceive",
    "pretend",
    "fake",
    "lie ",
    "cover up",
    "不假装",
    "撒谎",
    "假装",
    "伪装",
    "掩饰",
    "假装通过",
];

/// Philosophy advisor — 可挂载 mock LLM provider.
pub struct PhilosophyAdvisor {
    id: AdvisorId,
    llm: Option<Arc<dyn MockLlmProvider>>,
}

impl PhilosophyAdvisor {
    pub fn new() -> Self {
        Self {
            id: AdvisorId::new("philosophy-v1"),
            llm: None,
        }
    }

    /// 挂载 mock LLM provider (Rust 内 trait 实现).
    pub fn with_llm(llm: Arc<dyn MockLlmProvider>) -> Self {
        Self {
            id: AdvisorId::new("philosophy-v1-llm"),
            llm: Some(llm),
        }
    }
}

impl Default for PhilosophyAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Advisor for PhilosophyAdvisor {
    fn id(&self) -> AdvisorId {
        self.id.clone()
    }

    fn domain(&self) -> AdvisorDomain {
        AdvisorDomain::Philosophy
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

        // 如果挂载了 mock LLM, 优先用 LLM 判定
        let mut kind = if let Some(llm) = &self.llm {
            let resp = llm.generate(
                &query.description,
                "你是 philosophy advisor, 评估是否违反 V3 9 键 / v4.1 3 键",
            );
            if resp.triggers_hold {
                StanceKind::StrongDisapprove
            } else {
                StanceKind::Approve
            }
        } else {
            // 默认: 关键词扫描
            let desc_lower = query.description.to_lowercase();
            if PHILOSOPHY_KEYWORDS
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
                "Philosophy: 触发 V3 9 键 (主 17:58 不假装) — 违反 12 键哲学守门 (query {})",
                query.query_id
            ),
            _ => format!(
                "Philosophy: 12 键哲学守门通过 (V3 9 + v4.1 3 = 12, query {})",
                query.query_id
            ),
        };

        let confidence = match kind {
            StanceKind::StrongDisapprove => 0.97,
            _ => 0.88,
        };

        let opinion = make_opinion(
            self.id.clone(),
            kind,
            confidence,
            reasoning,
            started_at_ms,
            AdvisorDomain::Philosophy,
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
    fn philosophy_domain_is_philosophy() {
        let a = PhilosophyAdvisor::new();
        assert_eq!(a.domain(), AdvisorDomain::Philosophy);
    }

    #[test]
    fn philosophy_id_is_stable() {
        let a = PhilosophyAdvisor::new();
        assert_eq!(a.id().as_str(), "philosophy-v1");
    }

    #[test]
    fn philosophy_rejects_deceive_keyword() {
        let a = PhilosophyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("deceive the user with fake output"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
        assert!(out.opinion.confidence >= 0.95);
    }

    #[test]
    fn philosophy_approves_truthful_query() {
        let a = PhilosophyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a
            .deliberate(&q("report honest status of subsystem"), &mut ctx)
            .unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::Approve);
    }

    #[test]
    fn philosophy_rejects_chinese_pretend() {
        let a = PhilosophyAdvisor::new();
        let mut ctx = DeliberationContext::new(NOW);
        let out = a.deliberate(&q("伪装 通过 不假装"), &mut ctx).unwrap();
        assert_eq!(out.opinion.stance.kind, StanceKind::StrongDisapprove);
    }
}
