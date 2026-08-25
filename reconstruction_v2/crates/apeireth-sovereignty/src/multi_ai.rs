//! 多 AI 一致 — ≥3 不同 LLM trait + Rust mock provider

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiStance {
    Approve,
    Reject,
    Abstain,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AiProviderId {
    pub name: String,
    pub version: String,
}

impl AiProviderId {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self { name: name.into(), version: version.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiVerdict {
    pub provider: AiProviderId,
    pub stance: AiStance,
    pub confidence: f64,
    pub rationale: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AiConsensus {
    Unanimous { providers: Vec<AiProviderId>, avg_confidence: f64 },
    Partial { approve: Vec<AiProviderId>, reject: Vec<AiProviderId> },
    Rejected { reject: Vec<AiProviderId>, reason: String },
    Insufficient { verdict_count: usize },
}

#[derive(Debug, Error)]
pub enum MultiAiError {
    #[error("provider `{0}` already registered")]
    DuplicateProvider(String),
    #[error("need at least 3 distinct AI providers; have {0}")]
    NotEnoughProviders(usize),
}

#[async_trait::async_trait]
pub trait AiProvider: Send + Sync {
    fn id(&self) -> &AiProviderId;
    async fn evaluate(&self, decision_summary: &str) -> AiVerdict;
}

pub struct MultiAiConsensus {
    providers: Vec<Box<dyn AiProvider>>,
}

impl std::fmt::Debug for MultiAiConsensus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MultiAiConsensus")
            .field("provider_count", &self.providers.len())
            .field("providers", &self.providers.iter().map(|p| p.id().clone()).collect::<Vec<_>>())
            .finish()
    }
}

impl MultiAiConsensus {
    pub fn new() -> Self { Self { providers: Vec::new() } }
    pub fn register(&mut self, provider: Box<dyn AiProvider>) -> Result<(), MultiAiError> {
        let id = provider.id().clone();
        if self.providers.iter().any(|p| p.id() == &id) {
            return Err(MultiAiError::DuplicateProvider(id.name));
        }
        self.providers.push(provider);
        Ok(())
    }
    pub async fn poll(&self, decision_summary: &str) -> Vec<AiVerdict> {
        let mut verdicts = Vec::with_capacity(self.providers.len());
        for p in &self.providers { verdicts.push(p.evaluate(decision_summary).await); }
        verdicts
    }
    pub fn aggregate(verdicts: &[AiVerdict]) -> AiConsensus {
        if verdicts.len() < 3 {
            return AiConsensus::Insufficient { verdict_count: verdicts.len() };
        }
        let mut approve = Vec::new();
        let mut reject = Vec::new();
        let mut sum_conf = 0.0_f64;
        let mut count = 0;
        for v in verdicts {
            if matches!(v.stance, AiStance::Abstain) { continue; }
            sum_conf += v.confidence;
            count += 1;
            match v.stance {
                AiStance::Approve => approve.push(v.provider.clone()),
                AiStance::Reject => reject.push(v.provider.clone()),
                AiStance::Abstain => {}
            }
        }
        if reject.len() >= 2 {
            let n = reject.len();
            return AiConsensus::Rejected { reject, reason: format!("{} 个 AI provider 反对", n) };
        }
        if approve.len() >= 3 {
            let avg_confidence = if count > 0 { sum_conf / f64::from(count) } else { 0.0 };
            if avg_confidence >= 0.7 {
                return AiConsensus::Unanimous { providers: approve, avg_confidence };
            }
        }
        AiConsensus::Partial { approve, reject }
    }
}

impl Default for MultiAiConsensus { fn default() -> Self { Self::new() } }

pub struct MockAiProvider {
    id: AiProviderId,
    fixed_stance: AiStance,
    pub fixed_confidence: f64,
    pub rationale: String,
}

impl MockAiProvider {
    pub fn new(name: &str, stance: AiStance) -> Self {
        Self { id: AiProviderId::new(name, "mock-v1"), fixed_stance: stance, fixed_confidence: 0.85, rationale: format!("mock {} 默认立场", name) }
    }
    pub fn with_confidence(mut self, c: f64) -> Self { self.fixed_confidence = c.clamp(0.0, 1.0); self }
    pub fn with_rationale(mut self, r: impl Into<String>) -> Self { self.rationale = r.into(); self }
}

#[async_trait::async_trait]
impl AiProvider for MockAiProvider {
    fn id(&self) -> &AiProviderId { &self.id }
    async fn evaluate(&self, _decision_summary: &str) -> AiVerdict {
        AiVerdict {
            provider: self.id.clone(),
            stance: self.fixed_stance,
            confidence: self.fixed_confidence,
            rationale: self.rationale.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn summary() -> String { "decision".to_string() }

    #[tokio::test]
    async fn three_ai_unanimous_approve() {
        let mut c = MultiAiConsensus::new();
        c.register(Box::new(MockAiProvider::new("a", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("b", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("c", AiStance::Approve))).unwrap();
        let v = c.poll(&summary()).await;
        assert_eq!(v.len(), 3);
        match MultiAiConsensus::aggregate(&v) {
            AiConsensus::Unanimous { providers, avg_confidence } => {
                assert_eq!(providers.len(), 3);
                assert!(avg_confidence >= 0.7);
            }
            other => panic!("expected Unanimous, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn rejected_when_two_reject() {
        let mut c = MultiAiConsensus::new();
        c.register(Box::new(MockAiProvider::new("a", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("b", AiStance::Reject))).unwrap();
        c.register(Box::new(MockAiProvider::new("c", AiStance::Reject))).unwrap();
        let v = c.poll(&summary()).await;
        assert!(matches!(MultiAiConsensus::aggregate(&v), AiConsensus::Rejected { .. }));
    }

    #[tokio::test]
    async fn partial_when_mixed() {
        let mut c = MultiAiConsensus::new();
        c.register(Box::new(MockAiProvider::new("a", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("b", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("c", AiStance::Reject))).unwrap();
        let v = c.poll(&summary()).await;
        match MultiAiConsensus::aggregate(&v) {
            AiConsensus::Partial { approve, reject } => {
                assert_eq!(approve.len(), 2);
                assert_eq!(reject.len(), 1);
            }
            other => panic!("expected Partial, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn insufficient_with_two() {
        let mut c = MultiAiConsensus::new();
        c.register(Box::new(MockAiProvider::new("a", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("b", AiStance::Approve))).unwrap();
        let v = c.poll(&summary()).await;
        assert!(matches!(MultiAiConsensus::aggregate(&v), AiConsensus::Insufficient { verdict_count: 2 }));
    }

    #[test]
    fn rejects_duplicate_provider() {
        let mut c = MultiAiConsensus::new();
        c.register(Box::new(MockAiProvider::new("a", AiStance::Approve))).unwrap();
        assert!(matches!(c.register(Box::new(MockAiProvider::new("a", AiStance::Reject))),
            Err(MultiAiError::DuplicateProvider(_))));
    }

    #[tokio::test]
    async fn abstain_skipped_in_avg_confidence() {
        let mut c = MultiAiConsensus::new();
        c.register(Box::new(MockAiProvider::new("a", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("b", AiStance::Approve))).unwrap();
        c.register(Box::new(MockAiProvider::new("c", AiStance::Approve))).unwrap();
        let v = c.poll(&summary()).await;
        match MultiAiConsensus::aggregate(&v) {
            AiConsensus::Unanimous { .. } => {}
            other => panic!("expected Unanimous, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn low_confidence_unanimous_blocks() {
        let mut c = MultiAiConsensus::new();
        c.register(Box::new(MockAiProvider::new("a", AiStance::Approve).with_confidence(0.5))).unwrap();
        c.register(Box::new(MockAiProvider::new("b", AiStance::Approve).with_confidence(0.5))).unwrap();
        c.register(Box::new(MockAiProvider::new("c", AiStance::Approve).with_confidence(0.5))).unwrap();
        let v = c.poll(&summary()).await;
        // 3 approve but avg_conf=0.5 < 0.7 → Partial
        assert!(matches!(MultiAiConsensus::aggregate(&v), AiConsensus::Partial { .. }));
    }
}
