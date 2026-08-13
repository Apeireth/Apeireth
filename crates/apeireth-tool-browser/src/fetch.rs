//! HTTP fetch browser implementation (default mode).
//!
//! Uses `apeireth-http-client` (5-field keep-alive) for the actual HTTP GET,
//! then extracts accessibility tree from the response body. No Chrome needed.
//!
//! This is the recommended mode for coding agents per v2 plan §9.2:
//! - No tool schema overhead (token-efficient vs MCP)
//! - Works for static HTML (90%+ of pages)
//! - Falls back gracefully when JS is required (just returns raw HTML in
//!   `raw_html` field; agent can decide what to do)

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use async_trait::async_trait;
use std::sync::Mutex;

use crate::accessibility::extract_tree;
use crate::browser::{Browser, BrowserError, BrowserMode, PageSnapshot};
use apeireth_http_client::{HttpClient, KeepAliveConfig};

#[derive(Debug, Clone)]
pub struct FetchConfig {
    pub user_agent: String,
    pub timeout_ms: u64,
    pub follow_redirects: bool,
    pub max_response_bytes: usize,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            user_agent: "apeireth/1.2.0 (R139 browser)".to_string(),
            timeout_ms: 30_000,
            follow_redirects: true,
            max_response_bytes: 10 * 1024 * 1024, // 10 MB
        }
    }
}

pub struct FetchBrowser {
    config: FetchConfig,
    client: HttpClient,
    last_url: Mutex<Option<String>>,
}

impl FetchBrowser {
    pub fn new() -> Result<Self, BrowserError> {
        let cfg = KeepAliveConfig::default();
        let client = HttpClient::new(cfg).map_err(|e| BrowserError::Http(e.to_string()))?;
        Ok(Self {
            config: FetchConfig::default(),
            client,
            last_url: Mutex::new(None),
        })
    }

    pub fn with_config(config: FetchConfig) -> Result<Self, BrowserError> {
        let cfg = KeepAliveConfig::default();
        let client = HttpClient::new(cfg).map_err(|e| BrowserError::Http(e.to_string()))?;
        Ok(Self {
            config,
            client,
            last_url: Mutex::new(None),
        })
    }

    pub fn config(&self) -> &FetchConfig {
        &self.config
    }
}

impl Default for FetchBrowser {
    fn default() -> Self {
        Self::new().expect("HttpClient::new() should not fail")
    }
}

#[async_trait]
impl Browser for FetchBrowser {
    async fn navigate(&self, url: &str) -> Result<PageSnapshot, BrowserError> {
        if url.is_empty() {
            return Err(BrowserError::EmptyUrl);
        }
        // Validate URL
        let parsed = url::Url::parse(url).map_err(|e| BrowserError::Url(e.to_string()))?;

        // Build request with apeireth-http-client
        let response = self
            .client
            .get(parsed.as_str())
            .await
            .map_err(|e| BrowserError::Http(e.to_string()))?;

        let status = response.status().as_u16();
        let body = response.text().await.map_err(|e| BrowserError::Http(e.to_string()))?;

        // Truncate if needed
        let html = if body.len() > self.config.max_response_bytes {
            body[..self.config.max_response_bytes].to_string()
        } else {
            body
        };

        // Extract title
        let title = extract_title(&html);

        // Extract accessibility tree
        let accessibility = extract_tree(&html);

        // Update last_url
        {
            let mut last = self.last_url.lock().expect("poisoned");
            *last = Some(parsed.to_string());
        }

        Ok(PageSnapshot {
            url: parsed.to_string(),
            title,
            accessibility,
            raw_html: Some(html),
            status,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn snapshot(&self) -> Result<PageSnapshot, BrowserError> {
        let url = {
            let last = self.last_url.lock().expect("poisoned");
            last.clone()
        };
        match url {
            Some(u) => self.navigate(&u).await,
            None => Err(BrowserError::Navigation(
                "no last URL — call navigate() first".to_string(),
            )),
        }
    }

    async fn extract_text(&self) -> Result<String, BrowserError> {
        let snap = self.snapshot().await?;
        Ok(snap.accessibility.to_snapshot())
    }

    fn mode(&self) -> BrowserMode {
        BrowserMode::Fetch
    }
}

/// Extract `<title>` content from HTML (simple regex-free approach).
fn extract_title(html: &str) -> String {
    if let Some(open) = html.find("<title>") {
        let after = &html[open + 7..];
        if let Some(close) = after.find("</title>") {
            return after[..close].trim().to_string();
        }
    }
    // Try case-insensitive fallback
    let lower = html.to_lowercase();
    if let Some(open) = lower.find("<title>") {
        let after = &html[open + 7..];
        if let Some(close) = after.to_lowercase().find("</title>") {
            return after[..close].trim().to_string();
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser::Browser;

    #[tokio::test]
    async fn fetch_rejects_empty_url() {
        let b = FetchBrowser::new().unwrap();
        let r = b.navigate("").await;
        assert!(matches!(r, Err(BrowserError::EmptyUrl)));
    }

    #[tokio::test]
    async fn fetch_rejects_invalid_url() {
        let b = FetchBrowser::new().unwrap();
        let r = b.navigate("not a url").await;
        assert!(matches!(r, Err(BrowserError::Url(_))));
    }

    #[test]
    fn extract_title_basic() {
        let html = "<html><head><title>My Page</title></head></html>";
        assert_eq!(extract_title(html), "My Page");
    }

    #[test]
    fn extract_title_with_whitespace() {
        let html = "<title>  Spaced Out  </title>";
        assert_eq!(extract_title(html), "Spaced Out");
    }

    #[test]
    fn extract_title_missing() {
        let html = "<html><body>No title</body></html>";
        assert_eq!(extract_title(html), "");
    }

    #[test]
    fn default_config_reasonable() {
        let c = FetchConfig::default();
        assert!(c.user_agent.contains("apeireth"));
        assert_eq!(c.timeout_ms, 30_000);
        assert!(c.follow_redirects);
    }

    #[test]
    fn browser_mode_is_fetch() {
        let b = FetchBrowser::new().unwrap();
        assert_eq!(b.mode(), BrowserMode::Fetch);
    }
}