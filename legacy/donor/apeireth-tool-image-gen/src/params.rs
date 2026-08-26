//! Generation parameters (prompt / size / quality / style / count).
// R156 O-5: allow(missing_docs)
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageSize {
    /// 256x256
    Small,
    /// 512x512
    Medium,
    /// 1024x1024
    Large,
    /// 1024x1792 (portrait)
    Portrait,
    /// 1792x1024 (landscape)
    Landscape,
    /// Custom (width, height)
    Custom(u32, u32),
}

impl ImageSize {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            ImageSize::Small => (256, 256),
            ImageSize::Medium => (512, 512),
            ImageSize::Large => (1024, 1024),
            ImageSize::Portrait => (1024, 1792),
            ImageSize::Landscape => (1792, 1024),
            ImageSize::Custom(w, h) => (*w, *h),
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            ImageSize::Small => "256x256",
            ImageSize::Medium => "512x512",
            ImageSize::Large => "1024x1024",
            ImageSize::Portrait => "1024x1792",
            ImageSize::Landscape => "1792x1024",
            ImageSize::Custom(_, _) => "custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageQuality {
    Draft,
    Standard,
    HD,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageStyle {
    Natural,
    Vivid,
    Anime,
    Photographic,
    Cinematic,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)] // N17/TP2: serde_json::from_value 用
pub struct ImageGenParams {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub size: ImageSize,
    pub quality: ImageQuality,
    pub style: ImageStyle,
    pub count: u8,
    pub seed: Option<u64>,
}

impl ImageGenParams {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            negative_prompt: None,
            size: ImageSize::Large,
            quality: ImageQuality::Standard,
            style: ImageStyle::Natural,
            count: 1,
            seed: None,
        }
    }
    pub fn with_size(mut self, size: ImageSize) -> Self {
        self.size = size;
        self
    }
    pub fn with_quality(mut self, q: ImageQuality) -> Self {
        self.quality = q;
        self
    }
    pub fn with_style(mut self, s: ImageStyle) -> Self {
        self.style = s;
        self
    }
    pub fn with_count(mut self, n: u8) -> Self {
        self.count = n;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_params() {
        let p = ImageGenParams::new("a cat");
        assert_eq!(p.prompt, "a cat");
        assert_eq!(p.count, 1);
        assert_eq!(p.size, ImageSize::Large);
    }

    #[test]
    fn builder_chain() {
        let p = ImageGenParams::new("test")
            .with_size(ImageSize::Medium)
            .with_quality(ImageQuality::HD)
            .with_count(3);
        assert_eq!(p.size, ImageSize::Medium);
        assert_eq!(p.quality, ImageQuality::HD);
        assert_eq!(p.count, 3);
    }

    #[test]
    fn size_dimensions() {
        assert_eq!(ImageSize::Small.dimensions(), (256, 256));
        assert_eq!(ImageSize::Portrait.dimensions(), (1024, 1792));
        assert_eq!(ImageSize::Custom(800, 600).dimensions(), (800, 600));
    }

    #[test]
    fn size_str_no_panic() {
        let _ = ImageSize::Small.as_str();
        let _ = ImageSize::Portrait.as_str();
        let _ = ImageSize::Custom(100, 100).as_str();
    }
}
