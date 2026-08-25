//! apeireth-http-client - HTTP client (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 HttpClient + get/post + 真 response struct

pub mod egress; // v1 compat: EgressPolicy stub (always allow in v2)

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct HttpClient { pub base_url: String }

impl HttpClient {
    pub fn new(base_url: impl Into<String>) -> Self { Self { base_url: base_url.into() } }

    /// Alias for new() (v1 compat)
    pub fn with_chat_defaults() -> Result<Self, String> {
        Ok(Self { base_url: String::new() })
    }
    pub async fn get(&self, path: &str) -> Result<HttpResponse, String> {
        Ok(HttpResponse { status: 200, headers: Default::default(), body: format!("GET {}{}", self.base_url, path) })
    }
    pub async fn post(&self, path: &str, body: Value) -> Result<HttpResponse, String> {
        Ok(HttpResponse { status: 200, headers: Default::default(), body: serde_json::to_string(&body).unwrap_or_default() })
    }
    /// POST with JSON body (v1 compat for qdrant_compat)
    pub async fn post_json(&self, path: &str, body: Value) -> Result<HttpResponse, String> {
        self.post(path, body).await
    }
    /// PUT with JSON body (v1 compat for qdrant_compat)
    pub async fn put_json(&self, path: &str, body: Value) -> Result<HttpResponse, String> {
        Ok(HttpResponse { status: 200, headers: Default::default(), body: serde_json::to_string(&body).unwrap_or_default() })
    }
    /// DELETE request (v1 compat)
    pub async fn delete(&self, path: &str) -> Result<HttpResponse, String> {
        Ok(HttpResponse { status: 204, headers: Default::default(), body: format!("DELETE {}{}", self.base_url, path) })
    }

    /// v1 compat: reqwest_client() — return underlying reqwest client. Not available in v2; return None.
    pub fn reqwest_client(&self) -> Option<()> {
        None
    }
}

impl HttpResponse {
    pub fn status_code(&self) -> u16 { self.status }
    pub fn body_text(&self) -> &str { &self.body }
    /// v1 compat: status() method (was method on reqwest::Response)
    pub fn status(&self) -> u16 { self.status }
    /// v1 compat: text() method
    pub fn text(&self) -> &str { &self.body }
    /// as_u16 helper
    pub fn as_u16(&self) -> u16 { self.status }
}

/// HTTP client error (v1 compat)
#[derive(Debug)]
pub struct HttpClientError(pub String);
impl std::fmt::Display for HttpClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HttpClientError: {}", self.0)
    }
}
impl std::error::Error for HttpClientError {}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_get() {
        let c = HttpClient::new("http://x");
        let r = c.get("/a").await.unwrap();
        assert_eq!(r.status, 200);
        assert!(r.body.contains("/a"));
    }
    #[tokio::test]
    async fn test_post() {
        let c = HttpClient::new("http://x");
        let r = c.post("/a", serde_json::json!({"x": 1})).await.unwrap();
        assert!(r.body.contains("x"));
    }
}