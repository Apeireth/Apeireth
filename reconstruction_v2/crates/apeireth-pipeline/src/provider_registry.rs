//! Provider registry (LiteLLM-style).

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderCapability {
    Chat,
    Embedding,
    Vision,
    Audio,
    Tool,
}

pub const ALL_PROVIDER_CAPABILITIES: [ProviderCapability; 5] = [
    ProviderCapability::Chat,
    ProviderCapability::Embedding,
    ProviderCapability::Vision,
    ProviderCapability::Audio,
    ProviderCapability::Tool,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionStrategy {
    LowestCost,
    LowestLatency,
    RoundRobin,
    Weighted,
    Failover,
}

pub const ALL_SELECTION_STRATEGIES: [SelectionStrategy; 5] = [
    SelectionStrategy::LowestCost,
    SelectionStrategy::LowestLatency,
    SelectionStrategy::RoundRobin,
    SelectionStrategy::Weighted,
    SelectionStrategy::Failover,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    pub name: String,
    pub base_url: String,
    pub capabilities: Vec<ProviderCapability>,
    pub cost_per_1k: f64,
    pub weight: f64,
}

#[derive(Debug, Default)]
pub struct CostTracker {
    total_cost: f64,
    by_provider: std::collections::HashMap<String, f64>,
}

impl CostTracker {
    pub fn new() -> Self { Self::default() }
    pub fn record(&mut self, provider: &str, cost: f64) {
        self.total_cost += cost;
        *self.by_provider.entry(provider.to_string()).or_insert(0.0) += cost;
    }
    pub fn total(&self) -> f64 { self.total_cost }
    pub fn by_provider(&self) -> &std::collections::HashMap<String, f64> { &self.by_provider }
}

#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub provider: String,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("provider `{0}` not found")]
    NotFound(String),
    #[error("no provider has capability `{0:?}`")]
    NoProvider(ProviderCapability),
}

#[derive(Debug, Error)]
pub enum FallbackError {
    #[error("all providers failed")]
    AllFailed,
    #[error("provider failed: {0}")]
    ProviderFailed(String),
}

#[derive(Debug, Clone)]
pub struct FallbackChain {
    pub providers: Vec<String>,
}

impl FallbackChain {
    pub fn new(providers: Vec<String>) -> Self { Self { providers } }
}

#[derive(Debug, Default)]
pub struct ProviderRegistry {
    providers: std::collections::HashMap<String, ProviderSpec>,
}

impl ProviderRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, spec: ProviderSpec) {
        self.providers.insert(spec.name.clone(), spec);
    }

    pub fn get(&self, name: &str) -> Option<&ProviderSpec> {
        self.providers.get(name)
    }

    pub fn all(&self) -> Vec<&ProviderSpec> { self.providers.values().collect() }

    pub fn select_for_capability(&self, cap: ProviderCapability, strategy: SelectionStrategy) -> Result<&ProviderSpec, RegistryError> {
        let candidates: Vec<&ProviderSpec> = self.providers
            .values()
            .filter(|p| p.capabilities.contains(&cap))
            .collect();
        if candidates.is_empty() { return Err(RegistryError::NoProvider(cap)); }
        Ok(match strategy {
            SelectionStrategy::LowestCost => candidates.iter().min_by(|a, b| a.cost_per_1k.partial_cmp(&b.cost_per_1k).unwrap()).unwrap(),
            SelectionStrategy::LowestLatency => candidates.iter().min_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap()).unwrap(),
            _ => candidates[0],
        })
    }

    pub fn len(&self) -> usize { self.providers.len() }
    pub fn is_empty(&self) -> bool { self.providers.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_registry() -> ProviderRegistry {
        let mut r = ProviderRegistry::new();
        r.register(ProviderSpec {
            name: "openai".into(),
            base_url: "https://api.openai.com".into(),
            capabilities: vec![ProviderCapability::Chat],
            cost_per_1k: 0.03,
            weight: 1.0,
        });
        r.register(ProviderSpec {
            name: "local".into(),
            base_url: "http://localhost".into(),
            capabilities: vec![ProviderCapability::Chat, ProviderCapability::Embedding],
            cost_per_1k: 0.0,
            weight: 0.5,
        });
        r
    }

    #[test]
    fn register_and_get() {
        let r = mk_registry();
        assert!(r.get("openai").is_some());
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn select_lowest_cost() {
        let r = mk_registry();
        let s = r.select_for_capability(ProviderCapability::Chat, SelectionStrategy::LowestCost).unwrap();
        assert_eq!(s.name, "local");
    }

    #[test]
    fn select_capability_fails() {
        let r = mk_registry();
        let r = r.select_for_capability(ProviderCapability::Audio, SelectionStrategy::RoundRobin);
        assert!(matches!(r, Err(RegistryError::NoProvider(_))));
    }

    #[test]
    fn cost_tracker() {
        let mut c = CostTracker::new();
        c.record("openai", 1.5);
        c.record("local", 0.5);
        assert_eq!(c.total(), 2.0);
        assert_eq!(c.by_provider()["openai"], 1.5);
    }

    #[test]
    fn fallback_chain() {
        let chain = FallbackChain::new(vec!["a".into(), "b".into()]);
        assert_eq!(chain.providers.len(), 2);
    }

    #[test]
    fn all_constants() {
        assert_eq!(ALL_PROVIDER_CAPABILITIES.len(), 5);
        assert_eq!(ALL_SELECTION_STRATEGIES.len(), 5);
    }
}
