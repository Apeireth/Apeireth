//! Generation result types.
// R156 O-5: allow(missing_docs)
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    /// Image bytes (PNG/JPEG encoded)
    pub data: Vec<u8>,
    /// MIME type
    pub mime: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// Optional URL (when provider returns hosted URL)
    pub url: Option<String>,
    /// Optional seed used (for reproducibility)
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenResult {
    pub provider: String,
    pub model: String,
    pub images: Vec<GeneratedImage>,
    pub timestamp: String,
    pub elapsed_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_image_construction() {
        let img = GeneratedImage {
            data: vec![0x89, 0x50, 0x4E, 0x47],
            mime: "image/png".to_string(),
            width: 1,
            height: 1,
            url: None,
            seed: Some(42),
        };
        assert_eq!(img.mime, "image/png");
        assert_eq!(img.width, 1);
        assert_eq!(img.seed, Some(42));
    }

    #[test]
    fn result_construction() {
        let r = ImageGenResult {
            provider: "mock".to_string(),
            model: "mock-v1".to_string(),
            images: vec![],
            timestamp: "2026-08-12T00:00:00Z".to_string(),
            elapsed_ms: 100,
        };
        assert_eq!(r.provider, "mock");
        assert!(r.images.is_empty());
    }
}