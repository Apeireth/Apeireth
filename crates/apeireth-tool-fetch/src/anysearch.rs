//! # R176 `apeireth-apeireth-tool-fetch::anysearch` — AnySearch 真接 backend
//!
//! **R176 新增** — 实接 AnySearch JSON-RPC API, 拿真垂直搜索 / 批量 / 提取结果.
//!
//! ## 借鉴来源 (per O-5 不假装)
//!
//! - **AnySearch 官方**: https://anysearch.com (JSON-RPC API at `https://api.anysearch.com/mcp`)
//! - **本地参考**: `research/source/vcptoolbox/Plugin/AnySearch/AnySearch.js` (主人 8/13 指示)
//!
//! ## MCP JSON-RPC 协议 (R176 fix)
//!
//! AnySearch 是 MCP server, 走 `tools/call` 协议:
//! ```json
//! {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search","arguments":{"query":"..."}}}
//! ```
//!
//! 暴露 4 tool: search / get_sub_domains / batch_search / extract (per AnySearch.js COMMANDS)
//!
//! ## 17 垂直领域 (1:1 翻译 AnySearch.js DOMAINS)
//!
//! general / resource / social_media / finance / academic / legal / health /
//! business / security / ip / code / energy / environment / agriculture /
//! travel / film / gaming
//!
//! ## 0 触碰 3 不可变脊柱
//!
//! 本模块仅是 `apeireth-apeireth-tool-fetch` 内的 HTTP backend, 不进入 `apeireth-sovereignty` 调用链.

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use tracing::debug;

use crate::search_aggregator::{SearchHit, SearchSource};

// ============================================================================
// §1 Compile-time hardcodes (1:1 翻译 AnySearch.js)
// ============================================================================

pub const ANYSEARCH_ENDPOINT: &str = "https://api.anysearch.com/mcp";
pub const ANYSEARCH_TIMEOUT_MS: u64 = 30_000;
pub const JSONRPC_VERSION: &str = "2.0";
pub const ANYSEARCH_MCP_METHOD: &str = "tools/call";

/// 17 vertical domains (1:1 翻译 AnySearch.js DOMAINS).
pub const ANYSEARCH_DOMAINS: &[&str] = &[
    "general", "resource", "social_media", "finance", "academic", "legal",
    "health", "business", "security", "ip", "code", "energy",
    "environment", "agriculture", "travel", "film", "gaming",
];

/// 4 tool names (1:1 翻译 AnySearch.js COMMANDS).
pub const ANYSEARCH_METHODS: &[&str] = &[
    "search", "get_sub_domains", "batch_search", "extract",
];

const _: () = assert!(ANYSEARCH_DOMAINS.len() == 17);
const _: () = assert!(ANYSEARCH_METHODS.len() == 4);

// ============================================================================
// §2 Error type
// ============================================================================

#[derive(Debug, Error)]
pub enum AnySearchError {
    #[error("anysearch http transport error: {0}")]
    Transport(String),
    #[error("anysearch jsonrpc protocol error: code={code}, msg={msg}")]
    Protocol { code: i64, msg: String },
    #[error("anysearch api error: {0}")]
    Api(String),
    #[error("anysearch json parse error: {0}")]
    Parse(String),
    #[error("anysearch empty result")]
    Empty,
}

pub type AnySearchResult<T> = Result<T, AnySearchError>;

// ============================================================================
// §3 JSON-RPC types
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    pub params: Value,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Value) -> Self {
        Self { jsonrpc: JSONRPC_VERSION, id, method: method.into(), params }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

// ============================================================================
// §4 AnySearchClient
// ============================================================================

#[derive(Clone)]
pub struct AnySearchClient {
    endpoint: String,
    api_keys: Vec<String>,
    http: apeireth_http_client::HttpClient,
    next_id: Arc<AtomicU64>,
}

impl AnySearchClient {
    pub fn anonymous() -> AnySearchResult<Self> {
        Self::new(ANYSEARCH_ENDPOINT, vec![])
    }

    pub fn with_keys(endpoint: impl Into<String>, api_keys: Vec<String>) -> AnySearchResult<Self> {
        Self::new(endpoint, api_keys)
    }

    pub fn new(endpoint: impl Into<String>, api_keys: Vec<String>) -> AnySearchResult<Self> {
        let http = apeireth_http_client::HttpClient::with_chat_defaults()
            .map_err(|e| AnySearchError::Transport(format!("init http client: {e}")))?;
        Ok(Self {
            endpoint: endpoint.into(),
            api_keys,
            http,
            next_id: Arc::new(AtomicU64::new(1)),
        })
    }

    pub fn key_count(&self) -> usize { self.api_keys.len() }
    pub fn endpoint(&self) -> &str { &self.endpoint }

    pub async fn search(
        &self,
        query: &str,
        domain: Option<&str>,
        sub_domain: Option<&str>,
        sub_params: &str,
    ) -> AnySearchResult<String> {
        let mut args = json!({ "query": query });
        if let Some(d) = domain { args["domain"] = json!(d); }
        if let Some(sd) = sub_domain { args["sub_domain"] = json!(sd); }
        if !sub_params.is_empty() { args["sub_params"] = json!(sub_params); }
        let r = self.call("search", args).await?;
        extract_content(&r).ok_or(AnySearchError::Empty)
    }

    pub async fn get_sub_domains(&self, domain: &str) -> AnySearchResult<Value> {
        self.call("get_sub_domains", json!({ "domain": domain })).await
    }

    pub async fn batch_search(
        &self,
        queries: &[&str],
        domain: Option<&str>,
    ) -> AnySearchResult<String> {
        let mut args = json!({ "queries": queries });
        if let Some(d) = domain { args["domain"] = json!(d); }
        let r = self.call("batch_search", args).await?;
        extract_content(&r).ok_or(AnySearchError::Empty)
    }

    pub async fn extract(&self, url: &str) -> AnySearchResult<String> {
        let r = self.call("extract", json!({ "url": url })).await?;
        extract_content(&r).ok_or(AnySearchError::Empty)
    }

    /// 转换 Markdown content 为 SearchHit (塞进 SearchAggregator).
    ///
    /// 支持 2 种 AnySearch Markdown 格式:
    /// - 格式 A: `### N. title\n- **URL**: url`
    /// - 格式 B: `[title](url) snippet`
    pub fn markdown_to_hits(&self, query: &str, markdown: &str, limit: usize) -> Vec<SearchHit> {
        let mut hits = Vec::new();
        let mut current_title: Option<String> = None;
        for line in markdown.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }

            // 格式 A: "### 1. Title" 抓 title
            if let Some(rest) = line.strip_prefix("###") {
                let cleaned: String = rest.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.').trim().to_string();
                if !cleaned.is_empty() {
                    current_title = Some(cleaned);
                    continue;
                }
            }

            // 格式 A: "- **URL**: https://..." 抓 url + 上一个 title
            if let Some(url_part) = line.strip_prefix("- **URL**:") {
                let url = url_part.trim().to_string();
                let title = current_title.take().unwrap_or_else(|| query.to_string());
                hits.push(SearchHit {
                    title, url, snippet: String::new(),
                    source: SearchSource::AnySearch,
                    score: 1.0 - (hits.len() as f64 * 0.01),
                });
                if hits.len() >= limit { break; }
                continue;
            }

            // 格式 B: "[title](url) snippet"
            if let Some(start) = line.find('[') {
                if let Some(mid) = line.find("](") {
                    if let Some(end) = line[mid..].find(')') {
                        let title = line[start + 1..mid].to_string();
                        let url = line[mid + 2..mid + end].to_string();
                        let snippet_start = mid + end + 1;
                        let snippet = if snippet_start < line.len() {
                            line[snippet_start..].trim().to_string()
                        } else { String::new() };
                        hits.push(SearchHit {
                            title, url, snippet,
                            source: SearchSource::AnySearch,
                            score: 1.0 - (hits.len() as f64 * 0.01),
                        });
                        if hits.len() >= limit { break; }
                    }
                }
            }
        }
        if hits.is_empty() && !markdown.is_empty() {
            hits.push(SearchHit {
                title: query.to_string(),
                url: String::new(),
                snippet: markdown.chars().take(500).collect(),
                source: SearchSource::AnySearch,
                score: 0.5,
            });
        }
        hits
    }

    async fn call(&self, tool_name: &str, arguments: Value) -> AnySearchResult<Value> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        // MCP tools/call protocol: {name, arguments}
        let req = JsonRpcRequest::new(id, ANYSEARCH_MCP_METHOD, json!({
            "name": tool_name,
            "arguments": arguments,
        }));
        let body = serde_json::to_value(&req)?;

        let api_key = if self.api_keys.is_empty() {
            None
        } else {
            Some(self.api_keys[id as usize % self.api_keys.len()].clone())
        };

        debug!(target: "apeireth_tool_fetch::anysearch",
               "[R176] tools/call({}) POST {} id={}", tool_name, self.endpoint, id);

        let client = self.http.reqwest_client();
        let mut builder = client.post(&self.endpoint).json(&body);
        if let Some(key) = &api_key {
            builder = builder.bearer_auth(key);
        }
        builder = builder.header("X-AnySearch-Client", "apeireth-rust/0.1.0");
        builder = builder.timeout(Duration::from_millis(ANYSEARCH_TIMEOUT_MS));

        let response = builder.send().await
            .map_err(|e| AnySearchError::Transport(format!("send: {e}")))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AnySearchError::Transport(format!("HTTP {status}")));
        }

        let parsed: JsonRpcResponse = response.json().await
            .map_err(|e| AnySearchError::Parse(e.to_string()))?;

        if let Some(err) = parsed.error {
            return Err(AnySearchError::Protocol { code: err.code, msg: err.message });
        }
        parsed.result.ok_or(AnySearchError::Empty)
    }
}

impl From<serde_json::Error> for AnySearchError {
    fn from(e: serde_json::Error) -> Self { AnySearchError::Parse(e.to_string()) }
}

fn extract_content(result: &Value) -> Option<String> {
    let content = result.get("content")?.as_array()?;
    let mut out = String::new();
    for item in content {
        if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
            if !out.is_empty() { out.push_str("\n\n"); }
            out.push_str(text);
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

// ============================================================================
// §5 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_count_17() {
        assert_eq!(ANYSEARCH_DOMAINS.len(), 17);
        assert!(ANYSEARCH_DOMAINS.contains(&"general"));
        assert!(ANYSEARCH_DOMAINS.contains(&"code"));
    }

    #[test]
    fn methods_count_4() {
        assert_eq!(ANYSEARCH_METHODS.len(), 4);
        assert!(ANYSEARCH_METHODS.contains(&"search"));
    }

    #[test]
    fn endpoint_is_https() {
        assert!(ANYSEARCH_ENDPOINT.starts_with("https://"));
    }

    #[test]
    fn anonymous_client_constructs() {
        let c = AnySearchClient::anonymous().unwrap();
        assert_eq!(c.key_count(), 0);
    }

    #[test]
    fn with_keys_client_constructs() {
        let c = AnySearchClient::with_keys(ANYSEARCH_ENDPOINT, vec!["k1".into(), "k2".into()]).unwrap();
        assert_eq!(c.key_count(), 2);
    }

    #[test]
    fn jsonrpc_envelope() {
        let req = JsonRpcRequest::new(42, "search", json!({"query":"x"}));
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 42);
        assert_eq!(req.method, "search");
    }

    #[test]
    fn jsonrpc_response_success() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"hi"}]}}"#;
        let r: JsonRpcResponse = serde_json::from_str(raw).unwrap();
        assert!(r.error.is_none());
        assert_eq!(extract_content(&r.result.unwrap()).unwrap(), "hi");
    }

    #[test]
    fn jsonrpc_response_protocol_error() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"not found"}}"#;
        let r: JsonRpcResponse = serde_json::from_str(raw).unwrap();
        let err = r.error.unwrap();
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn extract_content_empty_array() {
        let v = json!({"content": []});
        assert!(extract_content(&v).is_none());
    }

    #[test]
    fn extract_content_concatenates() {
        let v = json!({"content":[{"type":"text","text":"a"},{"type":"text","text":"b"}]});
        let out = extract_content(&v).unwrap();
        assert!(out.contains("a") && out.contains("b"));
    }

    #[test]
    fn markdown_hits_format_a() {
        // 格式 A: ### N. Title\n- **URL**: url
        let c = AnySearchClient::anonymous().unwrap();
        let md = "### 1. Foo Title\n- **URL**: https://foo.com\n- snippet here\n\n### 2. Bar Title\n- **URL**: https://bar.com";
        let hits = c.markdown_to_hits("test", md, 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Foo Title");
        assert_eq!(hits[0].url, "https://foo.com");
        assert_eq!(hits[1].title, "Bar Title");
        assert_eq!(hits[1].url, "https://bar.com");
    }

    #[test]
    fn markdown_hits_format_b() {
        let c = AnySearchClient::anonymous().unwrap();
        let md = "[Rust](https://rust-lang.org) A language";
        let hits = c.markdown_to_hits("test", md, 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://rust-lang.org");
    }

    #[test]
    fn markdown_hits_fallback() {
        let c = AnySearchClient::anonymous().unwrap();
        let md = "no links here";
        let hits = c.markdown_to_hits("test", md, 10);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].url.is_empty());
    }

    #[test]
    fn markdown_hits_respects_limit() {
        let c = AnySearchClient::anonymous().unwrap();
        let md = (0..10).map(|i| format!("### {}. T{}\n- **URL**: https://x.com/{i}", i, i)).collect::<Vec<_>>().join("\n");
        let hits = c.markdown_to_hits("test", &md, 3);
        assert_eq!(hits.len(), 3);
    }
}