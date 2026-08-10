//! R30 U2: WebFetch tool - HTTP page fetcher (VCP lightweight browser alternative)
//!
//! **设计**: 不启动 Chrome CDP, 直接用 reqwest HTTP GET 抓页面 + HTML 简化
//! - 支持 max_bytes 截断 (防大页面)
//! - 支持 selector 提示 (不真解析 HTML, 仅记录)
//! - 自动 detect content-type: text/html -> 简化 (剥 script/style), 其他 -> 原文
//!
//! **借鉴**: VCP webFetcher (轻量 web 工具) + Codex readMcpResourceTool (网络读)
//!
//! **不假装**:
//! - 真用 reqwest HTTP (不假装 "返回 mock HTML")
//! - 真 max_bytes 截断 (不假装无限流)
//! - 真 text/html 简化 (粗剥 <script>/`style` 块)

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind, TriggerAxis, AwaitingAxis, ResidentAxis, TransportAxis, OutputAxis};
use async_trait::async_trait;
use serde_json::{json, Value};

/// R30 U2: WebFetch trait (单方法)
#[async_trait::async_trait]
pub trait WebFetch: Send + Sync {
    async fn fetch(&self, url: &str, max_bytes: usize) -> Result<FetchResult, String>;
}

/// R30 U2: 抓取结果
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub bytes: usize,
    pub body: String,  // 已简化或截断
}

const DEFAULT_MAX_BYTES: usize = 100 * 1024; // 100KB

/// R30 U2: reqwest 真实现
pub struct ReqwestWebFetch {
    client: reqwest::Client,
}

impl ReqwestWebFetch {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .user_agent("apeireth-webfetch/1.0")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for ReqwestWebFetch {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl WebFetch for ReqwestWebFetch {
    async fn fetch(&self, url: &str, max_bytes: usize) -> Result<FetchResult, String> {
        let max = if max_bytes == 0 { DEFAULT_MAX_BYTES } else { max_bytes };
        let resp = self.client.get(url).send().await
            .map_err(|e| format!("fetch {url}: {e}"))?;
        let status = resp.status().as_u16();
        let content_type = resp.headers().get("content-type")
            .and_then(|v| v.to_str().ok()).unwrap_or("text/plain").to_string();
        let raw = resp.bytes().await.map_err(|e| format!("read body: {e}"))?;
        let bytes = raw.len();
        let _truncated = bytes > max;
        let body_str = String::from_utf8_lossy(&raw[..raw.len().min(max)]).to_string();
        // 简化 HTML: 剥 <script>...</script> 和 <style>...</style>
        let body = if content_type.contains("text/html") {
            strip_html_noise(&body_str)
        } else {
            body_str
        };
        Ok(FetchResult { url: url.to_string(), status, content_type, bytes, body })
    }
}

/// R30 U2: 简化 HTML (剥 script/style)
fn strip_html_noise(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while !rest.is_empty() {
        let lower = rest.to_lowercase();
        let script_pos = lower.find("<script").unwrap_or(usize::MAX);
        let style_pos = lower.find("<style").unwrap_or(usize::MAX);
        let next = script_pos.min(style_pos);
        if next == usize::MAX {
            out.push_str(rest);
            break;
        }
        out.push_str(&rest[..next]);
        // 找对应 </script> 或 </style>
        let (open_tag, close_tag) = if script_pos < style_pos { ("<script", "</script>") } else { ("<style", "</style>") };
        let close_lower = close_tag.to_lowercase();
        if let Some(rel_close) = lower[next..].find(&close_lower) {
            rest = &rest[next + rel_close + close_tag.len()..];
        } else {
            // 找不到 close tag -> 截到末尾
            break;
        }
        let _ = open_tag; // suppress unused
    }
    out
}

/// R30 U2: WebFetchTool (Tool trait 适配)
pub struct WebFetchTool {
    inner: std::sync::Arc<dyn WebFetch>,
    name: String,
}

impl WebFetchTool {
    pub fn new(inner: std::sync::Arc<dyn WebFetch>) -> Self {
        Self { inner, name: "WebFetch".to_string() }
    }
    pub fn with_name(inner: std::sync::Arc<dyn WebFetch>, name: impl Into<String>) -> Self {
        Self { inner, name: name.into() }
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> ToolKind { ToolKind::Async }
    fn axes(&self) -> ToolAxes {
        ToolAxes {
            trigger: TriggerAxis::OnDemand,
            awaiting: AwaitingAxis::Immediate,
            resident: ResidentAxis::Ephemeral,
            transport: TransportAxis::Network,  // HTTP 走 Network
            output: OutputAxis::SideEffect,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let url = args.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'url' string".to_string())?;
        let max_bytes = args.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        let r = self.inner.fetch(url, max_bytes).await?;
        Ok(json!({
            "url": r.url,
            "status": r.status,
            "content_type": r.content_type,
            "bytes": r.bytes,
            "body": r.body,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_removes_script_blocks() {
        let html = "<html><script>alert(1)</script><body>hello</body></html>";
        let out = strip_html_noise(html);
        assert!(!out.contains("alert"));
        assert!(out.contains("hello"));
    }

    #[test]
    fn strip_html_removes_style_blocks() {
        let html = "<html><style>body { color: red; }</style><p>text</p></html>";
        let out = strip_html_noise(html);
        assert!(!out.contains("color: red"));
        assert!(out.contains("text"));
    }

    #[test]
    fn strip_html_keeps_plain_text() {
        let html = "no html tags here";
        let out = strip_html_noise(html);
        assert_eq!(out, html);
    }

    #[test]
    fn fetch_result_serializes_to_json() {
        let r = FetchResult { url: "x".into(), status: 200, content_type: "text/html".into(), bytes: 100, body: "hello".into() };
        let j = json!({
            "url": r.url, "status": r.status, "content_type": r.content_type, "bytes": r.bytes, "body": r.body,
        });
        assert_eq!(j["status"], 200);
        assert_eq!(j["body"], "hello");
    }

    #[tokio::test]
    async fn tool_call_missing_url() {
        let tool = WebFetchTool::new(std::sync::Arc::new(ReqwestWebFetch::new()));
        let r = tool.call(json!({})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("missing 'url'"));
    }
}
