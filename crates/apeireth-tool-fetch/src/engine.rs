//! Fetcher trait + FetchEngine unified entry.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::collections::HashMap;
use thiserror::Error;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::config::FetchConfig;
use crate::http_fetch::HttpFetcher;
use crate::rate_limit::RateLimiter;
use crate::html_extract::extract_text;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("empty URL")]
    EmptyUrl,
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("response too large: {0} bytes")]
    TooLarge(usize),
    #[error("parse error: {0}")]
    Parse(String),
    /// R231 — per-host rate limit exceeded
    #[error("rate limited for host {0}")]
    RateLimited(String),
}

pub type FetchResult<T> = Result<T, FetchError>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FetchRequest {
    pub url: String,
    pub method: Option<String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub extract_text_only: bool,
}

impl FetchRequest {
    pub fn get(url: impl Into<String>) -> Self {
        Self { url: url.into(), method: None, headers: HashMap::new(), body: None, extract_text_only: true }
    }
    pub fn with_header(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.headers.insert(k.into(), v.into());
        self
    }
    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self.extract_text_only = false;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub url: String,
    pub final_url: String,
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub bytes_received: usize,
    pub elapsed_ms: u64,
}

impl FetchResponse {
    pub fn is_html(&self) -> bool {
        self.content_type.to_lowercase().contains("html")
    }
}

#[async_trait]
pub trait Fetcher: Send + Sync {
    fn name(&self) -> &'static str;
    async fn fetch(&self, req: &FetchRequest, cfg: &FetchConfig) -> FetchResult<FetchResponse>;
}

pub struct FetchEngine {
    cfg: FetchConfig,
    /// R231 — per-host rate limiter (None = disabled)
    rate_limiter: Option<std::sync::Arc<parking_lot::Mutex<crate::rate_limit::RateLimiter>>>,
}

impl FetchEngine {
    pub fn new() -> Self { Self { cfg: FetchConfig::default(), rate_limiter: None } }
    pub fn with_config(cfg: FetchConfig) -> Self { Self { cfg, rate_limiter: None } }
    pub fn config(&self) -> &FetchConfig { &self.cfg }

    /// **R231 — 启用 per-host rate limit** (默认 60 req/60s)
    pub fn with_rate_limit(mut self) -> Self {
        self.rate_limiter = Some(std::sync::Arc::new(parking_lot::Mutex::new(RateLimiter::new())));
        self
    }

    /// **R231 — 自定义 rate limit** (max_requests / window)
    pub fn with_rate_limit_config(mut self, max_requests: usize, window_ms: u64) -> Self {
        let rl = RateLimiter::with_limit(max_requests, std::time::Duration::from_millis(window_ms));
        self.rate_limiter = Some(std::sync::Arc::new(parking_lot::Mutex::new(rl)));
        self
    }

    /// **R231 — 拿 rate limiter 引用** (None = 未启用)
    pub fn rate_limiter(&self) -> Option<&std::sync::Arc<parking_lot::Mutex<RateLimiter>>> {
        self.rate_limiter.as_ref()
    }

    pub async fn fetch(&self, req: &FetchRequest) -> FetchResult<FetchResponse> {
        // R174 K-1 强校验: URL 校验先做, 返语义化错误, 再委派 HttpFetcher 真接
        if req.url.trim().is_empty() {
            return Err(FetchError::EmptyUrl);
        }
        let parsed = url::Url::parse(&req.url)
            .map_err(|_| FetchError::InvalidUrl(req.url.clone()))?;
        // R231: per-host rate limit check (if enabled)
        if let Some(rl) = &self.rate_limiter {
            let host = parsed.host_str().unwrap_or("").to_string();
            if !host.is_empty() {
                let allowed = rl.lock().check(&host);
                if !allowed {
                    return Err(FetchError::RateLimited(host));
                }
                // 记录本次请求 (在 HTTP 调用前, 因为可能失败; 反正已占用配额)
                rl.lock().record(&host);
            }
        }
        // 真接 apeireth-http-client
        let fetcher = HttpFetcher::new();
        fetcher.fetch(req, &self.cfg).await
    }

    /// 用已 fetch 的 raw response 提取 text (供 HttpClient 调用)
    pub fn extract_from(&self, resp: FetchResponse, text_only: bool) -> FetchResponse {
        if text_only && resp.is_html() {
            let body = extract_text(&resp.body).unwrap_or_else(|_| resp.body.clone());
            FetchResponse { body, ..resp }
        } else {
            resp
        }
    }
}

impl Default for FetchEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn engine_validates_empty_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("")).await;
        assert!(matches!(r, Err(FetchError::EmptyUrl)));
    }

    #[tokio::test]
    async fn engine_validates_invalid_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("not a url")).await;
        assert!(matches!(r, Err(FetchError::InvalidUrl(_))));
    }

    #[tokio::test]
    async fn engine_accepts_valid_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("https://example.com")).await;
        assert!(r.is_ok());
    }

    #[test]
    fn request_get_helper() {
        let r = FetchRequest::get("https://x.com");
        assert!(r.extract_text_only);
        assert!(r.headers.is_empty());
    }

    #[test]
    fn request_with_header() {
        let r = FetchRequest::get("https://x.com").with_header("X-Key", "v");
        assert_eq!(r.headers.get("X-Key").unwrap(), "v");
    }

    #[test]
    fn request_with_body_disables_extract() {
        let r = FetchRequest::get("https://x.com").with_body("{\"a\":1}");
        assert!(!r.extract_text_only);
        assert_eq!(r.body.as_deref(), Some("{\"a\":1}"));
    }

    #[test]
    fn response_is_html() {
        let r = FetchResponse {
            url: "x".into(), final_url: "x".into(), status: 200,
            content_type: "text/html".into(), body: "".into(),
            bytes_received: 0, elapsed_ms: 0,
        };
        assert!(r.is_html());
    }
}


// ============================================================
// R231 — FetchEngine rate limit 集成 (5 cases)
// ============================================================

#[test]
fn r231_01_engine_no_rate_limit_by_default() {
    let engine = FetchEngine::new();
    assert!(engine.rate_limiter().is_none());
}

#[test]
fn r231_02_engine_with_rate_limit_enabled() {
    let engine = FetchEngine::new().with_rate_limit();
    assert!(engine.rate_limiter().is_some());
}

#[test]
fn r231_03_engine_with_rate_limit_custom() {
    let engine = FetchEngine::new().with_rate_limit_config(5, 1000);
    assert!(engine.rate_limiter().is_some());
}

#[test]
fn r231_04_engine_record_via_rl_accessor() {
    let engine = FetchEngine::new().with_rate_limit_config(2, 60_000);
    let rl = engine.rate_limiter().expect("rl enabled");
    rl.lock().record("example.com");
    rl.lock().record("example.com");
    assert!(!rl.lock().check("example.com"));
}

#[test]
fn r231_05_rate_limited_error_includes_host() {
    let err = FetchError::RateLimited("example.com".to_string());
    let s = format!("{err}");
    assert!(s.contains("example.com"), "错误信息应含 host: {s}");
}
