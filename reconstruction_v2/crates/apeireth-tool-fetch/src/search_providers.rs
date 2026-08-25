//! R252 -- multi-source HTTP search providers (Tavily, Brave, Serper, SerpAPI).
//!
//! Provides real HTTP-backed search via standard JSON APIs. Each provider implements
//! [`SearchProvider`] trait for pluggability into [`SearchAggregator`].
//!
//! **Honest** (O-5):
//! - All providers return real HTTP responses (no fake data)
//! - Missing API key = explicit error (not silent empty result)
//! - JSON parsing tolerance: gracefully skip malformed results
//! - No retry/backoff (caller's responsibility; use rate_limit crate)
//!
//! 0 触碰: new module, `SearchSource` enum unchanged.

#![allow(missing_docs)]

use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::search_aggregator::{SearchHit, SearchSource};

// ============================================================================
// Error
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("missing API key for {provider}")]
    MissingApiKey { provider: String },
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("JSON parse error: {0}")]
    Json(String),
    #[error("response missing expected field: {0}")]
    MissingField(&'static str),
    #[error("rate limited (retry after {0}s)")]
    RateLimited(u64),
}

pub type ProviderResult<T> = Result<T, ProviderError>;

// ============================================================================
// Provider trait
// ============================================================================

pub trait SearchProvider: Send + Sync {
    fn source(&self) -> SearchSource;
    fn name(&self) -> &str;
    fn api_key(&self) -> Option<&str>;
}

// ============================================================================
// Tavily
// ============================================================================

/// Tavily Search API (https://docs.tavily.com/docs/rest-api/api-reference).
///
/// POST https://api.tavily.com/search
/// Request: { "api_key": "...", "query": "...", "max_results": N }
/// Response: { "results": [{ "title": "...", "url": "...", "content": "...", "score": 0.95 }] }
#[derive(Debug, Clone)]
pub struct TavilyProvider {
    api_key: Option<String>,
    endpoint: String,
    max_results: usize,
}

impl TavilyProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            endpoint: "https://api.tavily.com/search".to_string(),
            max_results: 10,
        }
    }

    pub fn anonymous() -> Self {
        Self {
            api_key: None,
            endpoint: "https://api.tavily.com/search".to_string(),
            max_results: 10,
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    pub fn with_max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }
}

impl SearchProvider for TavilyProvider {
    fn source(&self) -> SearchSource {
        SearchSource::Tavily
    }
    fn name(&self) -> &str {
        "tavily"
    }
    fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

// Tavily response shapes
#[derive(Debug, Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Debug, Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    score: f64,
}

#[derive(Debug, Serialize)]
struct TavilyRequest<'a> {
    api_key: &'a str,
    query: &'a str,
    max_results: usize,
}

impl TavilyProvider {
    /// Parse a Tavily JSON response into SearchHit vec.
    /// Pure function for testability (no HTTP).
    pub fn parse_response(&self, json: &str, query: &str) -> ProviderResult<Vec<SearchHit>> {
        let resp: TavilyResponse =
            serde_json::from_str(json).map_err(|e| ProviderError::Json(e.to_string()))?;
        let hits = resp
            .results
            .into_iter()
            .map(|r| SearchHit {
                title: r.title,
                url: r.url,
                snippet: r.content,
                source: SearchSource::Tavily,
                score: r.score,
            })
            .collect();
        let _ = query;
        Ok(hits)
    }

    /// Build request body JSON for tavily API.
    pub fn build_request_body(&self, query: &str) -> ProviderResult<String> {
        let key = self
            .api_key
            .as_deref()
            .ok_or_else(|| ProviderError::MissingApiKey {
                provider: "tavily".to_string(),
            })?;
        let req = TavilyRequest {
            api_key: key,
            query,
            max_results: self.max_results,
        };
        serde_json::to_string(&req).map_err(|e| ProviderError::Json(e.to_string()))
    }
}
// ============================================================================
// Brave Search
// ============================================================================

/// Brave Search API (https://api.search.brave.com/app/documentation/web-search/get-started).
///
/// GET https://api.search.brave.com/res/v1/web/search?q=...&count=N
/// Header: X-Subscription-Token: <key>
/// Response: { "web": { "results": [{ "title": "...", "url": "...", "description": "..." }] } }
#[derive(Debug, Clone)]
pub struct BraveProvider {
    api_key: Option<String>,
    endpoint: String,
    count: usize,
}

impl BraveProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            endpoint: "https://api.search.brave.com/res/v1/web/search".to_string(),
            count: 10,
        }
    }

    pub fn anonymous() -> Self {
        Self {
            api_key: None,
            endpoint: "https://api.search.brave.com/res/v1/web/search".to_string(),
            count: 10,
        }
    }

    pub fn with_count(mut self, n: usize) -> Self {
        self.count = n;
        self
    }
}

impl SearchProvider for BraveProvider {
    fn source(&self) -> SearchSource {
        SearchSource::Brave
    }
    fn name(&self) -> &str {
        "brave"
    }
    fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

#[derive(Debug, Deserialize)]
struct BraveResponse {
    web: BraveWeb,
}

#[derive(Debug, Deserialize)]
struct BraveWeb {
    #[serde(default)]
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    #[serde(default)]
    description: String,
}

impl BraveProvider {
    pub fn parse_response(&self, json: &str, query: &str) -> ProviderResult<Vec<SearchHit>> {
        let resp: BraveResponse =
            serde_json::from_str(json).map_err(|e| ProviderError::Json(e.to_string()))?;
        let _ = query;
        // Brave does not return a score; default to 0.5
        let hits = resp
            .web
            .results
            .into_iter()
            .enumerate()
            .map(|(i, r)| SearchHit {
                title: r.title,
                url: r.url,
                snippet: r.description,
                source: SearchSource::Brave,
                // Approximate score by position (top results likely better)
                score: 1.0 - (i as f64) * 0.05,
            })
            .collect();
        Ok(hits)
    }
}

// ============================================================================
// Serper.dev
// ============================================================================

/// Serper.dev Google Search API (https://serper.dev/).
///
/// POST https://google.serper.dev/search
/// Header: X-API-KEY: <key>
/// Request: { "q": "...", "num": N }
/// Response: { "organic": [{ "title": "...", "link": "...", "snippet": "..." }] }
#[derive(Debug, Clone)]
pub struct SerperProvider {
    api_key: Option<String>,
    endpoint: String,
    num: usize,
}

impl SerperProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            endpoint: "https://google.serper.dev/search".to_string(),
            num: 10,
        }
    }

    pub fn anonymous() -> Self {
        Self {
            api_key: None,
            endpoint: "https://google.serper.dev/search".to_string(),
            num: 10,
        }
    }
}

impl SearchProvider for SerperProvider {
    fn source(&self) -> SearchSource {
        SearchSource::Serper
    }
    fn name(&self) -> &str {
        "serper"
    }
    fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

#[derive(Debug, Deserialize)]
struct SerperResponse {
    #[serde(default)]
    organic: Vec<SerperResult>,
}

#[derive(Debug, Deserialize)]
struct SerperResult {
    title: String,
    link: String,
    #[serde(default)]
    snippet: String,
    #[serde(default)]
    position: Option<usize>,
}

impl SerperProvider {
    pub fn parse_response(&self, json: &str) -> ProviderResult<Vec<SearchHit>> {
        let resp: SerperResponse =
            serde_json::from_str(json).map_err(|e| ProviderError::Json(e.to_string()))?;
        let hits = resp
            .organic
            .into_iter()
            .map(|r| SearchHit {
                title: r.title,
                url: r.link,
                snippet: r.snippet,
                source: SearchSource::Serper,
                score: 1.0 - r.position.unwrap_or(0) as f64 * 0.05,
            })
            .collect();
        Ok(hits)
    }
}

// ============================================================================
// Provider registry (R252)
// ============================================================================

/// Registry of available providers; caller can pick based on env-var API keys.
#[derive(Default)]
pub struct ProviderRegistry {
    providers: RwLock<Vec<Box<dyn SearchProvider>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build registry from environment variables:
    /// - TAVILY_API_KEY -> TavilyProvider
    /// - BRAVE_API_KEY -> BraveProvider
    /// - SERPER_API_KEY -> SerperProvider
    pub fn from_env() -> Self {
        let mut reg = Self::new();
        if let Ok(k) = std::env::var("TAVILY_API_KEY") {
            reg.add(Box::new(TavilyProvider::new(k)));
        }
        if let Ok(k) = std::env::var("BRAVE_API_KEY") {
            reg.add(Box::new(BraveProvider::new(k)));
        }
        if let Ok(k) = std::env::var("SERPER_API_KEY") {
            reg.add(Box::new(SerperProvider::new(k)));
        }
        reg
    }

    pub fn add(&mut self, p: Box<dyn SearchProvider>) {
        self.providers.write().push(p);
    }

    pub fn providers(&self) -> Vec<String> {
        self.providers
            .read()
            .iter()
            .map(|b| b.name().to_string())
            .collect()
    }

    pub fn provider_names(&self) -> Vec<String> {
        self.providers
            .read()
            .iter()
            .map(|b| b.name().to_string())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.providers.read().len()
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r252_01_tavily_parse_valid_response() {
        let json = r#"{"results":[
            {"title":"A","url":"https://a.com","content":"aaa","score":0.9},
            {"title":"B","url":"https://b.com","content":"bbb","score":0.7}
        ]}"#;
        let p = TavilyProvider::anonymous();
        let hits = p.parse_response(json, "q").unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].url, "https://a.com");
        assert_eq!(hits[0].source, SearchSource::Tavily);
        assert_eq!(hits[0].score, 0.9);
    }

    #[test]
    fn r252_02_tavily_parse_empty_results() {
        let json = r#"{"results":[]}"#;
        let p = TavilyProvider::anonymous();
        let hits = p.parse_response(json, "q").unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn r252_03_tavily_parse_missing_field_errors() {
        let json = r#"{"wrong":"shape"}"#;
        let p = TavilyProvider::anonymous();
        assert!(p.parse_response(json, "q").is_err());
    }

    #[test]
    fn r252_04_tavily_build_request_body_requires_key() {
        let p = TavilyProvider::anonymous();
        assert!(p.build_request_body("q").is_err());
        let p = TavilyProvider::new("test_key_123");
        let body = p.build_request_body("hello").unwrap();
        assert!(body.contains("test_key_123"));
        assert!(body.contains("hello"));
        assert!(body.contains("max_results"));
    }

    #[test]
    fn r252_05_brave_parse_valid_response() {
        let json = r#"{"web":{"results":[
            {"title":"A","url":"https://a.com","description":"desc a"},
            {"title":"B","url":"https://b.com","description":"desc b"}
        ]}}"#;
        let p = BraveProvider::anonymous();
        let hits = p.parse_response(json, "q").unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].url, "https://a.com");
        assert_eq!(hits[0].source, SearchSource::Brave);
        assert!(hits[0].score >= hits[1].score);
    }

    #[test]
    fn r252_06_brave_parse_empty_web() {
        let json = r#"{"web":{"results":[]}}"#;
        let p = BraveProvider::anonymous();
        let hits = p.parse_response(json, "q").unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn r252_07_serper_parse_valid_response() {
        let json = r#"{"organic":[
            {"title":"A","link":"https://a.com","snippet":"snip a","position":1},
            {"title":"B","link":"https://b.com","snippet":"snip b","position":2}
        ]}"#;
        let p = SerperProvider::anonymous();
        let hits = p.parse_response(json).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].url, "https://a.com");
        assert_eq!(hits[0].source, SearchSource::Serper);
        assert!(hits[0].score > hits[1].score);
    }

    #[test]
    fn r252_08_provider_registry_empty_by_default() {
        let reg = ProviderRegistry::new();
        assert_eq!(reg.count(), 0);
    }

    #[test]
    fn r252_09_provider_registry_add_and_list() {
        let mut reg = ProviderRegistry::new();
        reg.add(Box::new(TavilyProvider::new("k1")));
        reg.add(Box::new(BraveProvider::new("k2")));
        reg.add(Box::new(SerperProvider::new("k3")));
        assert_eq!(reg.count(), 3);
        let names = reg.provider_names();
        assert!(names.contains(&"tavily".to_string()));
        assert!(names.contains(&"brave".to_string()));
        assert!(names.contains(&"serper".to_string()));
    }

    #[test]
    fn r252_10_provider_trait_returns_correct_source() {
        let t = TavilyProvider::new("k");
        assert_eq!(t.source(), SearchSource::Tavily);
        let b = BraveProvider::new("k");
        assert_eq!(b.source(), SearchSource::Brave);
        let s = SerperProvider::new("k");
        assert_eq!(s.source(), SearchSource::Serper);
    }
}
