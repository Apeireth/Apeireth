//! `apeireth-tools::web_search` — 联网搜索 trait + HTTP 真实现
//!
//! **战役 2-5**: web_search trait 复刻 VCP `WebSearchTool` 字段级 (WebReadFile / 自定义 search URL).
//!
//! **VCP 字段级引用**:
//! - VCP `plugin-manifest.json:59 WebReadFile` (HTTP GET + parse response) → `HttpWebSearch::search` 行为
//! - VCP `chatCompletionHandler.js:22-28` 5 字段 keep-alive → 战役 1-2 `HttpClient`, 我们直接复用
//!
//! **设计**:
//! - `WebSearch` trait: `search(query, max_results) -> Result<ToolResult, String>`
//! - `HttpWebSearch` impl: 持 `Arc<HttpClient>` + 可配置 search URL (e.g. minimaxi / Google CSE / DuckDuckGo)
//! - `WebSearchTool`: 适配 `Tool` trait, 让 web_search 能通过 `apeireth-tool-registry` 统一注册
//!
//! **不假装**:
//! - ✅ 真用战役 1-2 `HttpClient` 5 字段 keep-alive (no mock socket)
//! - ✅ 端到端真测: 起本地 TCP echo HTTP server + 真发 GET + 验 query 字段透传
//! - ✅ query 是必传, 空字符串必失败 (字段级校验)

use std::sync::Arc;

use apeireth_http_client::HttpClient;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::result::ToolResult;

/// **WebSearch trait — 联网搜索**
///
/// **字段级参考** VCP `WebReadFile` (plugin-manifest.json:59) 行为:
/// - query 必传
/// - max_results 限制返回条数
/// - 返 `Result<ToolResult, String>` (outer 是 IO 错误, inner 是 typed 结果)
#[async_trait]
pub trait WebSearch: Send + Sync {
    /// 搜索接口
    ///
    /// **参数**:
    /// - `query`: 搜索关键词 (必, 空字符串应报错)
    /// - `max_results`: 最大返回条数
    ///
    /// **返**:
    /// - `Ok(ToolResult::Ok(...))` — 搜索成功
    /// - `Ok(ToolResult::Err { code, message })` — 搜索失败但 HTTP 通了
    /// - `Err(String)` — IO/序列化错误
    async fn search(&self, query: &str, max_results: u32) -> Result<ToolResult, String>;

    /// 工具名 (借战役 2-1 Tool trait `name()` 字段级)
    fn name(&self) -> &str;
}

// =============================================================================
// HttpWebSearch — 战役 1-2 HttpClient 真实现
// =============================================================================

/// **HTTP 实现的 WebSearch**
///
/// **配置**:
/// - `client`: 战役 1-2 `HttpClient` (5 字段 keep-alive, LIFO 池)
/// - `search_url`: 搜索 API URL (e.g. `https://www.googleapis.com/customsearch/v1?q={query}`),
///   占位符 `{query}` 会被 URL-encoded 替换; 占位符 `{max}` 会被 max_results 替换
/// - `name`: 工具名 (借 Tool trait `name()` 字段)
///
/// **请求行为** (VCP `WebReadFile` 字段级):
/// 1. URL template 替换 `{query}` → URL-encoded query
/// 2. URL template 替换 `{max}` → max_results
/// 3. GET 请求
/// 4. status 200 → parse JSON 响应 → 返 `Ok(ToolResult::Ok(...))`
/// 5. status 非 200 → 返 `Ok(ToolResult::Err { code: status, message: ... })`
/// 6. IO 错 → 返 `Err(String)`
pub struct HttpWebSearch {
    /// 战役 1-2 HTTP 客户端 (5 字段 keep-alive)
    client: Arc<HttpClient>,
    /// 搜索 URL template (e.g. `https://api.example.com/search?q={query}&n={max}`)
    search_url: String,
    /// 工具名
    name: String,
}

impl HttpWebSearch {
    /// 构造 HTTP WebSearch
    ///
    /// **参数**:
    /// - `client`: 战役 1-2 `HttpClient` (Arc 共享)
    /// - `search_url`: URL template, 支持 `{query}` + `{max}` 占位符
    /// - `name`: 工具名 (默认 "WebSearch")
    pub fn new(
        client: Arc<HttpClient>,
        search_url: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            client,
            search_url: search_url.into(),
            name: name.into(),
        }
    }

    /// 构造 minimaxi 域默认配置 (URL 留空, 实战配)
    pub fn with_minimaxi_default(client: Arc<HttpClient>) -> Self {
        // 实战: 主人可配 minimaxi 搜索 API URL, 这里留占位
        // 我们的 search_url 模板会透传 query + max_results 参数
        Self::new(
            client,
            "https://api.minimaxi.com/v1/search?q={query}&max={max}",
            "WebSearch",
        )
    }

    /// 当前 search URL (供 debug)
    pub fn search_url(&self) -> &str {
        &self.search_url
    }
}

#[async_trait]
impl WebSearch for HttpWebSearch {
    async fn search(&self, query: &str, max_results: u32) -> Result<ToolResult, String> {
        // 字段级校验: 空 query 必失败
        if query.is_empty() {
            return Ok(ToolResult::err(
                400,
                "query is required (VCP WebReadFile 字段级校验)",
            ));
        }
        if max_results == 0 {
            return Ok(ToolResult::err(400, "max_results must be > 0"));
        }

        // 1. URL template 替换 (URL-encoded)
        let encoded_query = url_encode(query);
        let url = self
            .search_url
            .replace("{query}", &encoded_query)
            .replace("{max}", &max_results.to_string());

        // 2. GET 请求 (战役 1-2 HttpClient 5 字段 keep-alive 走起)
        let resp = self.client.get(&url).await.map_err(|e| e.to_string())?;

        let status = i32::from(resp.status().as_u16());
        let url_final = resp.url().to_string();
        let elapsed_ms = resp.elapsed_ms();

        if !(200..300).contains(&status) {
            // 非 2xx: typed 错误
            return Ok(ToolResult::err(
                status,
                format!("HTTP {status} from {url_final} (elapsed={elapsed_ms}ms)"),
            ));
        }

        // 3. parse JSON 响应
        let body: Value = resp.json().await.map_err(|e| format!("JSON parse: {e}"))?;

        // 4. 返 Ok (实战可裁剪 results 数组到 max_results)
        Ok(ToolResult::ok(json!({
            "query": query,
            "max_results": max_results,
            "url": url_final,
            "elapsed_ms": elapsed_ms,
            "status": status,
            "results": body,
        })))
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 简单的 URL 编码 (URL-encode 主要字符, CJK 透传)
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for ch in s.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                out.push(ch);
            }
            ' ' => out.push_str("%20"),
            // CJK 等多字节 Unicode 字符 → 透传 (服务端 UTF-8 接收)
            ch if (ch as u32) > 0x7F => out.push(ch),
            // 其他 ASCII 标点 → %xx 编码
            ch => {
                let b = ch as u32 as u8;
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

// =============================================================================
// WebSearchTool — 适配 Tool trait (借战役 2-1 Tool 4 方法)
// =============================================================================

/// **WebSearch → Tool 适配器**
///
/// 让 `WebSearch` impl 通过战役 2-1 `ToolRegistry::register` 统一注册.
///
/// **args 协议** (JSON):
/// - `query` (String, 必)
/// - `max_results` (u32, 选, 默认 10)
pub struct WebSearchTool {
    inner: Arc<dyn WebSearch>,
}

impl WebSearchTool {
    /// 构造适配器
    pub fn new(inner: Arc<dyn WebSearch>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl apeireth_tool_registry::Tool for WebSearchTool {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn kind(&self) -> apeireth_tool_registry::ToolKind {
        // WebSearch 是异步外部依赖 → Async (战役 2-1 6 类 enum)
        apeireth_tool_registry::ToolKind::Async
    }
    fn axes(&self) -> apeireth_tool_registry::ToolAxes {
        apeireth_tool_registry::ToolAxes {
            trigger: apeireth_tool_registry::TriggerAxis::OnDemand,
            awaiting: apeireth_tool_registry::AwaitingAxis::Deferred,
            resident: apeireth_tool_registry::ResidentAxis::Ephemeral,
            transport: apeireth_tool_registry::TransportAxis::Network,
            output: apeireth_tool_registry::OutputAxis::Value,
        }
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'query' string".to_string())?;
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as u32;

        let result = self.inner.search(query, max_results).await?;
        // ToolResult → JSON Value
        match result {
            ToolResult::Ok(v) => Ok(v),
            ToolResult::Err { code, message } => {
                // 错误也用 JSON Value 表达 (caller 可拿到 code + message)
                Ok(json!({"error_code": code, "error_message": message}))
            }
        }
    }
}

// =============================================================================
// 单元测试 (DoD ≥ 2 个 + 端到端)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_query_returns_400() {
        // 字段级校验: 空 query 必返 400 typed 错误
        let client = Arc::new(HttpClient::with_vcp_defaults().expect("client"));
        let s = HttpWebSearch::new(client, "https://example.com/api?q={query}", "TestSearch");
        let r = s.search("", 10).await.expect("outer result");
        assert!(r.is_err(), "空 query 必失败");
        assert_eq!(r.err_code(), Some(400));
        assert!(r.err_message().unwrap().contains("required"));
    }

    #[tokio::test]
    async fn zero_max_results_returns_400() {
        let client = Arc::new(HttpClient::with_vcp_defaults().expect("client"));
        let s = HttpWebSearch::new(client, "https://example.com/api?q={query}", "TestSearch");
        let r = s.search("rust", 0).await.expect("outer result");
        assert!(r.is_err());
        assert_eq!(r.err_code(), Some(400));
    }

    #[test]
    fn url_encode_ascii() {
        assert_eq!(url_encode("hello"), "hello");
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(url_encode("a-b_c.d~e"), "a-b_c.d~e"); // unreserved 不编码
    }

    #[test]
    fn url_encode_cjk_passthrough() {
        // CJK 字符不强行编码, 透传 (服务端 UTF-8 接收)
        assert_eq!(url_encode("你好"), "你好");
        assert_eq!(url_encode("中文 abc"), "中文%20abc");
    }

    #[test]
    fn with_minimaxi_default_url() {
        let client = Arc::new(HttpClient::with_vcp_defaults().expect("client"));
        let s = HttpWebSearch::with_minimaxi_default(client);
        assert_eq!(s.name(), "WebSearch");
        assert!(s.search_url().contains("{query}"));
        assert!(s.search_url().contains("{max}"));
    }

    /// 端到端真测: 起本地 HTTP server, 真发 GET, 验证 query + max 透传
    #[tokio::test]
    async fn end_to_end_local_http_server() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        // 1. 起本地 HTTP echo server (极简, 解析 GET path 拿 query)
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let local_addr = listener.local_addr().expect("addr");
        let server_task = tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.expect("accept");
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    if socket.read(&mut buf).await.is_err() {
                        return;
                    }
                    let req = String::from_utf8_lossy(&buf);
                    // 提取 GET /path 部分
                    let path = req
                        .lines()
                        .next()
                        .and_then(|l| l.split_whitespace().nth(1))
                        .unwrap_or("/");
                    // JSON 响应, 包含 path
                    let body = format!(
                        r#"{{"echo_path":"{path}","service":"local-test-server","results":["a","b","c"]}}"#
                    );
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        });

        // 2. 配 HttpWebSearch 指向本地 server
        let client = Arc::new(HttpClient::with_vcp_defaults().expect("client"));
        let url = format!("http://{local_addr}/search?q={{query}}&n={{max}}");
        let search = HttpWebSearch::new(client, url, "LocalTestSearch");

        // 3. 真发请求
        let r = search.search("hello world", 5).await.expect("outer result");
        assert!(r.is_ok(), "本地 server 应返 200, 实际: {r:?}");
        let v = r.value().expect("value");
        assert_eq!(v["query"], "hello world");
        assert_eq!(v["max_results"], 5);
        // 服务端 echo 回 path, 应含 URL-encoded query
        let echo = v["results"]["echo_path"].as_str().expect("echo_path");
        assert!(
            echo.contains("hello%20world"),
            "query 应 URL-encoded, 实际: {echo}"
        );
        assert!(echo.contains("n=5"), "max 应透传, 实际: {echo}");

        // 4. 清理
        server_task.abort();
        let _ = server_task.await;
    }
}
