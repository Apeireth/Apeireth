// HTTP fetcher (synchronous blocking wrapper)

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::time::Instant;

use async_trait::async_trait;

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

#[async_trait]
impl Fetcher for HttpFetcher {
    fn name(&self) -> &'static str { "http" }

    async fn fetch(&self, req: &FetchRequest, cfg: &FetchConfig) -> FetchResult<FetchResponse> {
        // R174: real HTTP via apeireth-http-client (reqwest + 5 keep-alive fields)
        let start = Instant::now();
        let method = req.method.as_deref().unwrap_or("GET");
        if !["GET", "POST", "HEAD"].contains(&method) {
            return Err(FetchError::Http(format!("unsupported method: {method}")));
        }
        if req.body.as_ref().map(|s| s.len()).unwrap_or(0) > cfg.max_response_bytes {
            return Err(FetchError::TooLarge(req.body.as_ref().map(|s| s.len()).unwrap_or(0)));
        }

        let client = apeireth_http_client::HttpClient::with_chat_defaults()
            .map_err(|e| FetchError::Http(format!("http client init: {e}")))?;

        let response = match method {
            "POST" => {
                let body_val = serde_json::Value::String(req.body.clone().unwrap_or_default());
                client.post(&req.url, &body_val).await
            }
            _ => client.get(&req.url).await,  // GET + HEAD via inner reqwest (HEAD 暂走 GET, R175+ 续)
        }
        .map_err(|e| FetchError::Http(format!("send: {e}")))?;

        let status = response.status();
        let final_url = response.url().to_string();
        let content_type = response.content_type();
        let _elapsed_ms = response.elapsed_ms();
        let body = response.text().await
            .map_err(|e| FetchError::Http(format!("read body: {e}")))?;
        let bytes_received = body.len();

        // text/html 提取走 html_extract (R149 baseline)
        let body = if req.extract_text_only && content_type.to_lowercase().contains("html") {
            crate::html_extract::extract_text(&body).unwrap_or(body)
        } else {
            body
        };

        Ok(FetchResponse {
            url: req.url.clone(),
            final_url,
            status: status.as_u16(),
            content_type,
            body,
            bytes_received,
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

    #[tokio::test]
    async fn http_fetcher_rejects_bad_method() {
        let f = HttpFetcher::new();
        let mut req = FetchRequest::get("https://x.com");
        req.method = Some("PATCH".into());
        let r = f.fetch(&req, &FetchConfig::default()).await;
        assert!(matches!(r, Err(FetchError::Http(_))));
    }
}
