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

// ============================================================================
// R262: FetchMetrics -- stdlib-only OTel metrics for FetchEngine.fetch()
// (counter + errors counter + duration sum + max latency). Self-contained,
// uses AtomicU64/AtomicI64 (0 new external dep).
// ============================================================================
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering as AtomicOrdering};

#[derive(Debug, Default)]
pub struct FetchMetrics {
    pub fetch_total: AtomicU64,
    pub fetch_errors_total: AtomicU64,
    pub fetch_total_duration_ms: AtomicU64,
    pub fetch_latency_max_ms: AtomicI64,  // -1 = unset
}

impl FetchMetrics {
    pub fn new() -> Self {
        Self {
            fetch_total: AtomicU64::new(0),
            fetch_errors_total: AtomicU64::new(0),
            fetch_total_duration_ms: AtomicU64::new(0),
            fetch_latency_max_ms: AtomicI64::new(-1),
        }
    }
    pub fn record_success(&self, latency_ms: f64) {
        self.fetch_total.fetch_add(1, AtomicOrdering::Relaxed);
        self.fetch_total_duration_ms.fetch_add(latency_ms as u64, AtomicOrdering::Relaxed);
        let prev = self.fetch_latency_max_ms.load(AtomicOrdering::Relaxed);
        let cur = latency_ms as i64;
        if cur > prev {
            self.fetch_latency_max_ms.store(cur, AtomicOrdering::Relaxed);
        }
    }
    pub fn record_error(&self, latency_ms: f64) {
        self.fetch_total.fetch_add(1, AtomicOrdering::Relaxed);
        self.fetch_errors_total.fetch_add(1, AtomicOrdering::Relaxed);
        self.fetch_total_duration_ms.fetch_add(latency_ms as u64, AtomicOrdering::Relaxed);
        let prev = self.fetch_latency_max_ms.load(AtomicOrdering::Relaxed);
        let cur = latency_ms as i64;
        if cur > prev {
            self.fetch_latency_max_ms.store(cur, AtomicOrdering::Relaxed);
        }
    }
    pub fn metrics_text(&self) -> String {
        let total = self.fetch_total.load(AtomicOrdering::Relaxed);
        let errors = self.fetch_errors_total.load(AtomicOrdering::Relaxed);
        let sum = self.fetch_total_duration_ms.load(AtomicOrdering::Relaxed);
        let max = self.fetch_latency_max_ms.load(AtomicOrdering::Relaxed);
        let mean = if total > 0 { sum as f64 / total as f64 } else { 0.0 };
        format!("fetch_total={} fetch_errors_total={} fetch_latency_mean_ms={:.3} fetch_latency_max_ms={}\n",
                total, errors, mean, max)
    }
}

#[cfg(test)]
mod metrics_tests {
    use super::*;
    #[test]
    fn r262_01_initial_zero() {
        let m = FetchMetrics::new();
        assert_eq!(m.fetch_total.load(AtomicOrdering::Relaxed), 0);
        assert_eq!(m.fetch_errors_total.load(AtomicOrdering::Relaxed), 0);
        assert_eq!(m.fetch_total_duration_ms.load(AtomicOrdering::Relaxed), 0);
        assert_eq!(m.fetch_latency_max_ms.load(AtomicOrdering::Relaxed), -1);
    }
    #[test]
    fn r262_02_record_success_increments_total_and_updates_max() {
        let m = FetchMetrics::new();
        m.record_success(50.0);
        assert_eq!(m.fetch_total.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(m.fetch_errors_total.load(AtomicOrdering::Relaxed), 0);
        assert_eq!(m.fetch_latency_max_ms.load(AtomicOrdering::Relaxed), 50);
        m.record_success(100.0);
        assert_eq!(m.fetch_total.load(AtomicOrdering::Relaxed), 2);
        assert_eq!(m.fetch_latency_max_ms.load(AtomicOrdering::Relaxed), 100);
        m.record_success(10.0);
        assert_eq!(m.fetch_latency_max_ms.load(AtomicOrdering::Relaxed), 100);
    }
    #[test]
    fn r262_03_record_error_increments_both() {
        let m = FetchMetrics::new();
        m.record_error(20.0);
        assert_eq!(m.fetch_total.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(m.fetch_errors_total.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(m.fetch_total_duration_ms.load(AtomicOrdering::Relaxed), 20);
        assert_eq!(m.fetch_latency_max_ms.load(AtomicOrdering::Relaxed), 20);
    }
    #[test]
    fn r262_04_metrics_text_format() {
        let m = FetchMetrics::new();
        m.record_success(50.0);
        m.record_success(150.0);
        let s = m.metrics_text();
        assert!(s.contains("fetch_total=2"));
        assert!(s.contains("fetch_errors_total=0"));
        assert!(s.contains("fetch_latency_mean_ms=100.000"));
        assert!(s.contains("fetch_latency_max_ms=150"));
    }
    #[tokio::test]
    async fn r262_05_fetch_engine_records_metrics_on_empty_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("")).await;
        assert!(r.is_err());
        assert_eq!(e.fetch_metrics.fetch_total.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(e.fetch_metrics.fetch_errors_total.load(AtomicOrdering::Relaxed), 1);
    }
    #[tokio::test]
    async fn r262_06_fetch_engine_records_metrics_on_success() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("https://example.com")).await;
        assert!(r.is_ok(), "expected ok, got {:?}", r.err());
        assert_eq!(e.fetch_metrics.fetch_total.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(e.fetch_metrics.fetch_errors_total.load(AtomicOrdering::Relaxed), 0);
    }
    #[tokio::test]
    async fn r262_07_fetch_engine_records_metrics_on_invalid_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("not a url")).await;
        assert!(r.is_err());
        assert_eq!(e.fetch_metrics.fetch_total.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(e.fetch_metrics.fetch_errors_total.load(AtomicOrdering::Relaxed), 1);
    }
    #[test]
    fn r262_08_metrics_text_after_real_calls() {
        let m = FetchMetrics::new();
        m.record_success(75.0);
        m.record_error(125.0);
        let s = m.metrics_text();
        assert!(s.contains("fetch_total=2"));
        assert!(s.contains("fetch_errors_total=1"));
        assert!(s.contains("fetch_latency_max_ms=125"));
    }
}

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
    /// R262: OTel metrics for fetch() invocations.
    pub fetch_metrics: FetchMetrics,
}

impl FetchEngine {
    pub fn new() -> Self { Self { cfg: FetchConfig::default(), rate_limiter: None, fetch_metrics: FetchMetrics::new() } }
    pub fn with_config(cfg: FetchConfig) -> Self { Self { cfg, rate_limiter: None, fetch_metrics: FetchMetrics::new() } }
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
        // R262: time the entire fetch (validation + rate limit + http).
        let started = std::time::Instant::now();
        // R174 K-1 强校验: URL 校验先做, 返语义化错误, 再委派 HttpFetcher 真接
        if req.url.trim().is_empty() {
            self.fetch_metrics.record_error(started.elapsed().as_secs_f64() * 1000.0);
            return Err(FetchError::EmptyUrl);
        }
        let parsed = match url::Url::parse(&req.url) {
            Ok(u) => u,
            Err(_) => {
                self.fetch_metrics.record_error(started.elapsed().as_secs_f64() * 1000.0);
                return Err(FetchError::InvalidUrl(req.url.clone()));
            }
        };
        // R231: per-host rate limit check (if enabled)
        if let Some(rl) = &self.rate_limiter {
            let host = parsed.host_str().unwrap_or("").to_string();
            if !host.is_empty() {
                let allowed = rl.lock().check(&host);
                if !allowed {
                    self.fetch_metrics.record_error(started.elapsed().as_secs_f64() * 1000.0);
                    return Err(FetchError::RateLimited(host));
                }
                // 记录本次请求 (在 HTTP 调用前, 因为可能失败; 反正已占用配额)
                rl.lock().record(&host);
            }
        }
        // 真接 apeireth-http-client
        let fetcher = HttpFetcher::new();
        let result = fetcher.fetch(req, &self.cfg).await;
        let latency_ms = started.elapsed().as_secs_f64() * 1000.0;
        match &result {
            Ok(_) => self.fetch_metrics.record_success(latency_ms),
            Err(_) => self.fetch_metrics.record_error(latency_ms),
        }
        result
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
