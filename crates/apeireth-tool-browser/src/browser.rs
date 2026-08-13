//! Browser trait + page snapshot abstractions.
//!
//! Defines the unified interface that all browser modes (HTTP fetch, CDP)
//! implement. The trait stays minimal so unit tests don't require Chrome.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use thiserror::Error;
use crate::accessibility::AccessibilityTree;

/// Browser execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserMode {
    /// HTTP fetch + HTML parse (default, no Chrome required)
    Fetch,
    /// Real Chrome via CDP (requires `cdp` feature)
    Cdp,
    /// Auto: try Cdp if available, fall back to Fetch
    Auto,
}

/// Errors from browser operations.
#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("http: `{0}`")]
    Http(String),
    #[error("url parse: `{0}`")]
    Url(String),
    #[error("browser mode `{0:?}` is not enabled (rebuild with `--features cdp`)")]
    ModeNotEnabled(BrowserMode),
    #[error("cdp not enabled in build (rebuild with `--features cdp`)")]
    CdpNotEnabled,
    #[error("navigation failed: `{0}`")]
    Navigation(String),
    #[error("timeout after `{0}ms`")]
    Timeout(u64),
    #[error("empty url")]
    EmptyUrl,
}

/// A page snapshot: URL + title + accessibility tree + raw HTML.
#[derive(Debug, Clone)]
pub struct PageSnapshot {
    /// Final URL (after redirects)
    pub url: String,
    /// Page title
    pub title: String,
    /// Accessibility tree (LLM-friendly)
    pub accessibility: AccessibilityTree,
    /// Raw HTML (optional; large pages may skip)
    pub raw_html: Option<String>,
    /// HTTP status code
    pub status: u16,
    /// Timestamp (RFC3339)
    pub timestamp: String,
}

/// Browser abstraction. Implementations:
/// - `FetchBrowser` (HTTP fetch + HTML parse, default)
/// - `CdpBrowser` (Chromium via CDP, requires `cdp` feature)
#[async_trait::async_trait]
pub trait Browser: Send + Sync {
    /// Navigate to URL, returning a page snapshot.
    async fn navigate(&self, url: &str) -> Result<PageSnapshot, BrowserError>;

    /// Get current accessibility tree without navigating.
    /// Fetch implementation re-fetches the last URL.
    async fn snapshot(&self) -> Result<PageSnapshot, BrowserError>;

    /// Extract text content from page (for LLM context windows).
    async fn extract_text(&self) -> Result<String, BrowserError>;

    /// Browser mode this implementation uses.
    fn mode(&self) -> BrowserMode;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_equality() {
        assert_eq!(BrowserMode::Fetch, BrowserMode::Fetch);
        assert_ne!(BrowserMode::Fetch, BrowserMode::Cdp);
    }

    #[test]
    fn page_snapshot_construction() {
        let snap = PageSnapshot {
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            accessibility: AccessibilityTree::empty(),
            raw_html: None,
            status: 200,
            timestamp: "2026-08-12T00:00:00Z".to_string(),
        };
        assert_eq!(snap.status, 200);
    }

    #[test]
    fn error_display() {
        let e = BrowserError::Timeout(5000);
        assert!(e.to_string().contains("5000"));
        let e2 = BrowserError::EmptyUrl;
        assert!(e2.to_string().contains("empty"));
    }
}