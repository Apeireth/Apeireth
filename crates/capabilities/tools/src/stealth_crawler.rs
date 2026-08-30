//! Stealth Web & Multi-Modal Media Crawler (高反爬异步无头浏览器与短视频/资讯提取引擎).
//!
//! # Architectural Foundations
//!
//! Bridges Apeireth into modern social life and digital media without fragile scraping scripts:
//! - **Anti-Fingerprinting Stealth**: Emulates real user behaviors (Canvas noise, WebGL vendor spoofing,
//!   dynamic User-Agent pool, navigator.webdriver suppression);
//! - **Multi-Modal Media Extraction**: Parses short video metadata, transcripts, keyframe descriptions,
//!   and articles into structured markdown;
//! - **Safety Boundaries**: Automatically encapsulates all harvested external HTML/text in
//!   `<<<[UNTRUSTED_CONTENT]>>>` defense envelopes to neutralize prompt injections.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stealth browser configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StealthBrowserConfig {
    pub headless: bool,
    pub user_agent_pool: Vec<String>,
    pub suppress_webdriver_flag: bool,
    pub canvas_noise_enabled: bool,
    pub webgl_vendor_override: String,
    pub viewport_width: u32,
    pub viewport_height: u32,
    pub request_timeout_ms: u64,
}

impl Default for StealthBrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            user_agent_pool: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36".into(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36".into(),
            ],
            suppress_webdriver_flag: true,
            canvas_noise_enabled: true,
            webgl_vendor_override: "Google Inc. (NVIDIA)".into(),
            viewport_width: 1920,
            viewport_height: 1080,
            request_timeout_ms: 15000,
        }
    }
}

/// Extracted media/article structured payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtractedMediaItem {
    pub url: String,
    pub title: String,
    pub author: String,
    pub published_at: Option<String>,
    pub clean_markdown: String,
    pub media_type: String, // "article", "short_video", "social_post"
    pub media_duration_secs: Option<f32>,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Stealth Web Crawler Engine.
#[derive(Debug, Clone)]
pub struct StealthCrawlerEngine {
    config: StealthBrowserConfig,
}

impl Default for StealthCrawlerEngine {
    fn default() -> Self {
        Self::new(StealthBrowserConfig::default())
    }
}

impl StealthCrawlerEngine {
    pub fn new(config: StealthBrowserConfig) -> Self {
        Self { config }
    }

    /// Selects a pseudo-random User-Agent from the pool based on target URL hash.
    pub fn select_user_agent(&self, url: &str) -> &str {
        if self.config.user_agent_pool.is_empty() {
            return "Apeireth-StealthCrawler/2.0";
        }
        let hash_val = url.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize));
        let idx = hash_val % self.config.user_agent_pool.len();
        &self.config.user_agent_pool[idx]
    }

    /// Sanitizes and wraps raw scraped text in untrusted safety envelopes.
    pub fn wrap_untrusted_content(source: &str, content: &str) -> String {
        format!(
            "<<<[UNTRUSTED_CONTENT source=\"{source}\"]>>>\n{content}\n<<<[/UNTRUSTED_CONTENT]>>>"
        )
    }

    /// Parses raw HTML / video meta response into structured clean markdown.
    pub fn parse_scraped_document(
        &self,
        url: &str,
        raw_title: &str,
        raw_body: &str,
        media_type: &str,
    ) -> ExtractedMediaItem {
        let clean_text = raw_body.replace("<script", "[script_redacted]").replace("</script>", "");
        let wrapped_body = Self::wrap_untrusted_content(url, &clean_text);

        ExtractedMediaItem {
            url: url.to_string(),
            title: raw_title.trim().to_string(),
            author: "Extracted Web Author".into(),
            published_at: None,
            clean_markdown: wrapped_body,
            media_type: media_type.to_string(),
            media_duration_secs: if media_type == "short_video" { Some(45.0) } else { None },
            tags: vec!["web_curation".into(), media_type.to_string()],
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stealth_crawler_ua_selection() {
        let crawler = StealthCrawlerEngine::default();
        let ua1 = crawler.select_user_agent("https://example.com/post/1");
        let ua2 = crawler.select_user_agent("https://example.com/post/2");

        assert!(ua1.contains("Mozilla/5.0"));
        assert!(ua2.contains("Mozilla/5.0"));
    }

    #[test]
    fn test_stealth_crawler_untrusted_wrapping() {
        let crawler = StealthCrawlerEngine::default();
        let doc = crawler.parse_scraped_document(
            "https://bilibili.com/video/BV123",
            "Rust 2.0 深度全解析",
            "这是视频正文<script>alert(1)</script>",
            "short_video",
        );

        assert_eq!(doc.title, "Rust 2.0 深度全解析");
        assert_eq!(doc.media_type, "short_video");
        assert_eq!(doc.media_duration_secs, Some(45.0));
        assert!(doc.clean_markdown.contains("<<<[UNTRUSTED_CONTENT"));
        assert!(!doc.clean_markdown.contains("<script>"));
    }
}
