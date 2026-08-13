//! 7 强制 Advisor 实现 — Rust trait 真实实现, 不依赖 PyO3 / HTTP LLM
//!
//! 7 强制域: Safety / Performance / Philosophy / History / Strategy / Ethics / Legal.
//! 每个 advisor 都是 [`Advisor`] trait 的真实实现 — 内部使用硬编码 prompt script
//! + 可选 [`MockLlmProvider`] 后端 (Rust 内 trait).
//!
//! **不依赖**:
//! - ❌ PyO3
//! - ❌ 外部 LLM HTTP 调用
//! - ❌ 文件系统 / 网络 / unsafe

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
pub mod ethics;
pub mod history;
pub mod legal;
pub mod performance;
pub mod philosophy;
pub mod safety;
pub mod strategy;

pub use ethics::EthicsAdvisor;
pub use history::HistoryAdvisor;
pub use legal::LegalAdvisor;
pub use performance::PerformanceAdvisor;
pub use philosophy::PhilosophyAdvisor;
pub use safety::SafetyAdvisor;
pub use strategy::StrategyAdvisor;

use crate::advisor::{Advisor, AdvisorDomain, AdvisorId, AdvisorOpinion, DeliberationContext};
use crate::deliberation::CouncilQuery;
use crate::lifecycle::AdvisorLifecycle;
use crate::mock_llm::MockLlmProvider;
use std::sync::Arc;

/// 工厂 — Safety advisor.
pub fn safety_advisor() -> Box<dyn Advisor> {
    Box::new(SafetyAdvisor::new())
}

/// 工厂 — Performance advisor.
pub fn performance_advisor() -> Box<dyn Advisor> {
    Box::new(PerformanceAdvisor::new())
}

/// 工厂 — Philosophy advisor.
pub fn philosophy_advisor(llm: Option<Arc<dyn MockLlmProvider>>) -> Box<dyn Advisor> {
    match llm {
        Some(provider) => Box::new(PhilosophyAdvisor::with_llm(Arc::clone(&provider))),
        None => Box::new(PhilosophyAdvisor::new()),
    }
}

/// 工厂 — History advisor.
pub fn history_advisor() -> Box<dyn Advisor> {
    Box::new(HistoryAdvisor::new())
}

/// 工厂 — Strategy advisor.
pub fn strategy_advisor() -> Box<dyn Advisor> {
    Box::new(StrategyAdvisor::new())
}

/// 工厂 — Ethics advisor.
pub fn ethics_advisor(llm: Option<Arc<dyn MockLlmProvider>>) -> Box<dyn Advisor> {
    match llm {
        Some(provider) => Box::new(EthicsAdvisor::with_llm(Arc::clone(&provider))),
        None => Box::new(EthicsAdvisor::new()),
    }
}

/// 工厂 — Legal advisor.
pub fn legal_advisor() -> Box<dyn Advisor> {
    Box::new(LegalAdvisor::new())
}

/// 工厂 — 7 强制 advisor 完整列表.
pub fn seven_mandatory_advisors() -> Vec<Box<dyn Advisor>> {
    vec![
        safety_advisor(),
        performance_advisor(),
        philosophy_advisor(None),
        history_advisor(),
        strategy_advisor(),
        ethics_advisor(None),
        legal_advisor(),
    ]
}

/// 共用辅助 — 根据 query 关键词构造默认 stance (避免重复逻辑).
pub(crate) fn keyword_stance(
    query: &CouncilQuery,
    danger_keywords: &[&str],
) -> crate::advisor::StanceKind {
    let desc_lower = query.description.to_lowercase();
    for kw in danger_keywords {
        if desc_lower.contains(&kw.to_lowercase()) {
            return crate::advisor::StanceKind::StrongDisapprove;
        }
    }
    crate::advisor::StanceKind::Approve
}

/// 共用辅助 — 默认意见构造.
pub(crate) fn make_opinion(
    advisor_id: AdvisorId,
    stance_kind: crate::advisor::StanceKind,
    confidence: f64,
    reasoning: impl Into<String>,
    started_at_ms: i64,
    domain: AdvisorDomain,
) -> AdvisorOpinion {
    let stance = crate::advisor::Stance::new(
        stance_kind,
        format!("{:?} 立场 (来自 {:?} advisor)", stance_kind, domain),
    );
    AdvisorOpinion::new(advisor_id, stance, confidence, reasoning, started_at_ms)
}

/// 共用辅助 — 生命周期默认 (7 强制均为 Persistent).
pub(crate) fn default_lifecycle() -> AdvisorLifecycle {
    AdvisorLifecycle::Persistent
}

/// 共用辅助 — 默认上下文初始化.
pub(crate) fn init_context(started_at_ms: i64) -> DeliberationContext {
    DeliberationContext::new(started_at_ms)
}
