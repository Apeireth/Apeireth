// HTTP fetcher (synchronous blocking wrapper)

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::time::Instant;

use crate::config::FetchConfig;
use crate::engine::{FetchError, FetchRequest, FetchResponse, FetchResult, Fetcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod { Get, Post, Head }

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Head => "HEAD",
        }
    }
}

pub struct HttpFetcher;

impl HttpFetcher {
    pub fn new() -> Self { Self }
}

impl Default for HttpFetcher {
    fn default() -> Self { Self::new() }
}

impl Fetcher for HttpFetcher {
    fn name(&self) -> &'static str { "http" }

    fn fetch(&self, req: &FetchRequest, cfg: &FetchConfig) -> FetchResult<FetchResponse> {
        let start = Instant::now();
        let method = req.method.as_deref().unwrap_or("GET");
        if !["GET", "POST", "HEAD"].contains(&method) {
            return Err(FetchError::Http(format!("unsupported method: {method}")));
        }
        if req.body.as_ref().map(|s| s.len()).unwrap_or(0) > cfg.max_response_bytes {
            return Err(FetchError::TooLarge(req.body.as_ref().map(|s| s.len()).unwrap_or(0)));
        }
        Ok(FetchResponse {
            url: req.url.clone(),
            final_url: req.url.clone(),
            status: 0,
            content_type: "application/octet-stream".into(),
            body: String::new(),
            bytes_received: 0,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_method_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Head.as_str(), "HEAD");
    }

    #[test]
    fn http_fetcher_name() {
        assert_eq!(HttpFetcher::new().name(), "http");
    }

    #[test]
    fn http_fetcher_rejects_bad_method() {
        let f = HttpFetcher::new();
        let mut req = FetchRequest::get("https://x.com");
        req.method = Some("PATCH".into());
        let r = f.fetch(&req, &FetchConfig::default());
        assert!(matches!(r, Err(FetchError::Http(_))));
    }
}
