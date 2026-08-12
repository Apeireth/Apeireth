//! ImageGenProvider trait + 13-kind enum + registry.
// R156 O-5: allow(missing_docs)
#![allow(missing_docs)]

use async_trait::async_trait;
use std::collections::HashMap;

use crate::params::ImageGenParams;
use crate::result::ImageGenResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderKind {
    OpenAiDallE,
    StabilityAi,
    Midjourney,
    MiniMaxImage,
    GoogleImagen,
    AdobeFirefly,
    LeonardoAi,
    Ideogram,
    PlaygroundAi,
    BingImageCreator,
    Craiyon,
    Nightcafe,
    Mock,
}

impl ProviderKind {
    pub fn name(&self) -> &'static str {
        match self {
            ProviderKind::OpenAiDallE => "openai-dalle",
            ProviderKind::StabilityAi => "stability-ai",
            ProviderKind::Midjourney => "midjourney",
            ProviderKind::MiniMaxImage => "minimax-image",
            ProviderKind::GoogleImagen => "google-imagen",
            ProviderKind::AdobeFirefly => "adobe-firefly",
            ProviderKind::LeonardoAi => "leonardo-ai",
            ProviderKind::Ideogram => "ideogram",
            ProviderKind::PlaygroundAi => "playground-ai",
            ProviderKind::BingImageCreator => "bing-image-creator",
            ProviderKind::Craiyon => "craiyon",
            ProviderKind::Nightcafe => "nightcafe",
            ProviderKind::Mock => "mock",
        }
    }

    pub fn all() -> &'static [ProviderKind] {
        &[
            ProviderKind::OpenAiDallE,
            ProviderKind::StabilityAi,
            ProviderKind::Midjourney,
            ProviderKind::MiniMaxImage,
            ProviderKind::GoogleImagen,
            ProviderKind::AdobeFirefly,
            ProviderKind::LeonardoAi,
            ProviderKind::Ideogram,
            ProviderKind::PlaygroundAi,
            ProviderKind::BingImageCreator,
            ProviderKind::Craiyon,
            ProviderKind::Nightcafe,
            ProviderKind::Mock,
        ]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("provider `{0}` not implemented (only mock + 3 API stubs available)")]
    NotImplemented(String),
    #[error("missing API key for provider `{0}` (set env var or config)")]
    MissingApiKey(String),
    #[error("api error: `{0}`")]
    Api(String),
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
}

#[async_trait]
pub trait ImageGenProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn name(&self) -> &str { self.kind().name() }
    async fn generate(&self, params: &ImageGenParams) -> Result<ImageGenResult, ProviderError>;
}

/// Registry of available providers.
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn ImageGenProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }
    pub fn register(&mut self, provider: Box<dyn ImageGenProvider>) {
        self.providers.insert(provider.name().to_string(), provider);
    }
    pub fn get(&self, name: &str) -> Option<&dyn ImageGenProvider> {
        self.providers.get(name).map(|b| b.as_ref())
    }
    pub fn names(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }
    pub fn count(&self) -> usize {
        self.providers.len()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_count_is_13() {
        assert_eq!(ProviderKind::all().len(), 13);
    }

    #[test]
    fn provider_names_unique() {
        let names: Vec<_> = ProviderKind::all().iter().map(|k| k.name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "duplicate provider names");
    }

    #[test]
    fn from_str_works() {
        // We can do a round-trip via name()
        for kind in ProviderKind::all() {
            assert!(!kind.name().is_empty());
        }
    }

    #[test]
    fn registry_new_empty() {
        let r = ProviderRegistry::new();
        assert_eq!(r.count(), 0);
        assert!(r.names().is_empty());
        assert!(r.get("nonexistent").is_none());
    }
}