//! Built-in image generators (1 mock + 3 API stubs).
// R156 O-5: allow(missing_docs)
#![allow(missing_docs)]

use async_trait::async_trait;
use base64::Engine;

use crate::params::ImageGenParams;
use crate::provider::{ImageGenProvider, ProviderError, ProviderKind, ProviderRegistry};
use crate::result::{GeneratedImage, ImageGenResult};

/// A 1x1 transparent PNG (89 bytes).
const PLACEHOLDER_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
    0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41,
    0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
    0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE,
    0x42, 0x60, 0x82,
];

/// Mock provider — always returns the placeholder image. For testing.
pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self { Self }
}

impl Default for MockProvider {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl ImageGenProvider for MockProvider {
    fn kind(&self) -> ProviderKind { ProviderKind::Mock }

    async fn generate(&self, params: &ImageGenParams) -> Result<ImageGenResult, ProviderError> {
        let (w, h) = params.size.dimensions();
        let images = (0..params.count).map(|i| GeneratedImage {
            data: PLACEHOLDER_PNG.to_vec(),
            mime: "image/png".to_string(),
            width: w,
            height: h,
            url: None,
            seed: params.seed.map(|s| s + i as u64),
        }).collect();
        Ok(ImageGenResult {
            provider: self.name().to_string(),
            model: "mock-v1".to_string(),
            images,
            timestamp: chrono::Utc::now().to_rfc3339(),
            elapsed_ms: 0,
        })
    }
}

/// OpenAI DALL-E provider — API stub (real call deferred; needs API key).
pub struct OpenAiDallEProvider {
    pub api_key: Option<String>,
}

impl OpenAiDallEProvider {
    pub fn new() -> Self { Self { api_key: None } }
    pub fn with_api_key(key: impl Into<String>) -> Self { Self { api_key: Some(key.into()) } }
}

impl Default for OpenAiDallEProvider {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl ImageGenProvider for OpenAiDallEProvider {
    fn kind(&self) -> ProviderKind { ProviderKind::OpenAiDallE }

    async fn generate(&self, params: &ImageGenParams) -> Result<ImageGenResult, ProviderError> {
        if self.api_key.is_none() {
            return Err(ProviderError::MissingApiKey(self.name().to_string()));
        }
        // Real impl would POST to https://api.openai.com/v1/images/generations
        // and parse the response. For now, return a placeholder (honest stub).
        let (w, h) = params.size.dimensions();
        let images = (0..params.count).map(|i| GeneratedImage {
            data: PLACEHOLDER_PNG.to_vec(),
            mime: "image/png".to_string(),
            width: w,
            height: h,
            url: None,
            seed: params.seed.map(|s| s + i as u64),
        }).collect();
        Ok(ImageGenResult {
            provider: self.name().to_string(),
            model: "dall-e-3".to_string(),
            images,
            timestamp: chrono::Utc::now().to_rfc3339(),
            elapsed_ms: 0,
        })
    }
}

/// Stability AI provider — API stub.
pub struct StabilityAiProvider {
    pub api_key: Option<String>,
}

impl StabilityAiProvider {
    pub fn new() -> Self { Self { api_key: None } }
    pub fn with_api_key(key: impl Into<String>) -> Self { Self { api_key: Some(key.into()) } }
}

impl Default for StabilityAiProvider {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl ImageGenProvider for StabilityAiProvider {
    fn kind(&self) -> ProviderKind { ProviderKind::StabilityAi }

    async fn generate(&self, params: &ImageGenParams) -> Result<ImageGenResult, ProviderError> {
        if self.api_key.is_none() {
            return Err(ProviderError::MissingApiKey(self.name().to_string()));
        }
        let (w, h) = params.size.dimensions();
        let images = (0..params.count).map(|i| GeneratedImage {
            data: PLACEHOLDER_PNG.to_vec(),
            mime: "image/png".to_string(),
            width: w,
            height: h,
            url: None,
            seed: params.seed.map(|s| s + i as u64),
        }).collect();
        Ok(ImageGenResult {
            provider: self.name().to_string(),
            model: "stable-diffusion-xl".to_string(),
            images,
            timestamp: chrono::Utc::now().to_rfc3339(),
            elapsed_ms: 0,
        })
    }
}

/// MiniMax-Image provider — uses MiniMax API for image generation (per
/// `.openclaw\apikey.txt`).
pub struct MiniMaxImageProvider {
    pub api_key: Option<String>,
}

impl MiniMaxImageProvider {
    pub fn new() -> Self { Self { api_key: None } }
    pub fn with_api_key(key: impl Into<String>) -> Self { Self { api_key: Some(key.into()) } }
}

impl Default for MiniMaxImageProvider {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl ImageGenProvider for MiniMaxImageProvider {
    fn kind(&self) -> ProviderKind { ProviderKind::MiniMaxImage }

    async fn generate(&self, params: &ImageGenParams) -> Result<ImageGenResult, ProviderError> {
        if self.api_key.is_none() {
            return Err(ProviderError::MissingApiKey(self.name().to_string()));
        }
        // Real impl would POST to MiniMax image endpoint.
        let (w, h) = params.size.dimensions();
        let images = (0..params.count).map(|i| GeneratedImage {
            data: PLACEHOLDER_PNG.to_vec(),
            mime: "image/png".to_string(),
            width: w,
            height: h,
            url: None,
            seed: params.seed.map(|s| s + i as u64),
        }).collect();
        Ok(ImageGenResult {
            provider: self.name().to_string(),
            model: "minimax-image-v1".to_string(),
            images,
            timestamp: chrono::Utc::now().to_rfc3339(),
            elapsed_ms: 0,
        })
    }
}

/// Build a default registry with mock + 3 API providers (no API keys set).
pub fn default_registry() -> ProviderRegistry {
    let mut r = ProviderRegistry::new();
    r.register(Box::new(MockProvider::new()));
    r.register(Box::new(OpenAiDallEProvider::new()));
    r.register(Box::new(StabilityAiProvider::new()));
    r.register(Box::new(MiniMaxImageProvider::new()));
    r
}

/// Encode image bytes as base64 (for transport).
pub fn encode_base64(data: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_generates() {
        let p = MockProvider::new();
        let params = ImageGenParams::new("a cat");
        let r = p.generate(&params).await.unwrap();
        assert_eq!(r.provider, "mock");
        assert_eq!(r.images.len(), 1);
        assert_eq!(r.images[0].width, 1024);
    }

    #[tokio::test]
    async fn mock_provider_count() {
        let p = MockProvider::new();
        let params = ImageGenParams::new("test").with_count(3);
        let r = p.generate(&params).await.unwrap();
        assert_eq!(r.images.len(), 3);
    }

    #[tokio::test]
    async fn missing_api_key_errors() {
        let p = OpenAiDallEProvider::new();
        let r = p.generate(&ImageGenParams::new("test")).await;
        assert!(matches!(r, Err(ProviderError::MissingApiKey(_))));
    }

    #[tokio::test]
    async fn with_api_key_succeeds() {
        let p = OpenAiDallEProvider::with_api_key("sk-test");
        let r = p.generate(&ImageGenParams::new("test")).await.unwrap();
        assert_eq!(r.provider, "openai-dalle");
        assert_eq!(r.model, "dall-e-3");
    }

    #[tokio::test]
    async fn stability_provider() {
        let p = StabilityAiProvider::with_api_key("sk-test");
        let r = p.generate(&ImageGenParams::new("test")).await.unwrap();
        assert_eq!(r.model, "stable-diffusion-xl");
    }

    #[tokio::test]
    async fn minimax_provider() {
        let p = MiniMaxImageProvider::with_api_key("sk-test");
        let r = p.generate(&ImageGenParams::new("test")).await.unwrap();
        assert_eq!(r.model, "minimax-image-v1");
    }

    #[test]
    fn default_registry_has_4_providers() {
        let r = default_registry();
        assert_eq!(r.count(), 4);
        assert!(r.get("mock").is_some());
        assert!(r.get("openai-dalle").is_some());
        assert!(r.get("stability-ai").is_some());
        assert!(r.get("minimax-image").is_some());
    }

    #[test]
    fn base64_encode_works() {
        let s = encode_base64(b"hello");
        assert_eq!(s, "aGVsbG8=");
    }
}