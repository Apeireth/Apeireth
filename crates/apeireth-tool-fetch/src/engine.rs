//! Fetcher trait + FetchEngine unified entry.

use std::collections::HashMap;
use thiserror::Error;
use serde::{Deserialize, Serialize};

use crate::config::FetchConfig;
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

pub trait Fetcher: Send + Sync {
    fn name(&self) -> &'static str;
    fn fetch(&self, req: &FetchRequest, cfg: &FetchConfig) -> FetchResult<FetchResponse>;
}

pub struct FetchEngine {
    cfg: FetchConfig,
}

impl FetchEngine {
    pub fn new() -> Self { Self { cfg: FetchConfig::default() } }
    pub fn with_config(cfg: FetchConfig) -> Self { Self { cfg } }
    pub fn config(&self) -> &FetchConfig { &self.cfg }

    pub fn fetch(&self, req: &FetchRequest) -> FetchResult<FetchResponse> {
        if req.url.trim().is_empty() {
            return Err(FetchError::EmptyUrl);
        }
        if url::Url::parse(&req.url).is_err() {
            return Err(FetchError::InvalidUrl(req.url.clone()));
        }
        // 不假装: 调用方应通过 HttpFetcher / HttpClient 注入真实 fetch
        // 这里只做 URL 校验 + extract_text_only 包装
        let body = if req.extract_text_only { String::new() } else { req.body.clone().unwrap_or_default() };
        Ok(FetchResponse {
            url: req.url.clone(),
            final_url: req.url.clone(),
            status: 0,
            content_type: "application/octet-stream".into(),
            body,
            bytes_received: 0,
            elapsed_ms: 0,
        })
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

    #[test]
    fn engine_validates_empty_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get(""));
        assert!(matches!(r, Err(FetchError::EmptyUrl)));
    }

    #[test]
    fn engine_validates_invalid_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("not a url"));
        assert!(matches!(r, Err(FetchError::InvalidUrl(_))));
    }

    #[test]
    fn engine_accepts_valid_url() {
        let e = FetchEngine::new();
        let r = e.fetch(&FetchRequest::get("https://example.com"));
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
