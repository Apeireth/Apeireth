//! R230: Crawl tool - 轻量爬虫 (WebFetch 扩展: 抓取 + 链接提取 + 深度遍历).
//!
//! **设计**: 不启动浏览器, 基于 WebFetch 的 reqwest 抓取, 递归提取链接.
//! - max_pages / max_depth 上限 (防滥用: ≤10 页, ≤2 层)
//! - 只 http(s), 去重, 每页 max_bytes 截断
//! - 链接提取: 简单 href 扫描 (免 regex 依赖, 诚实: 非完整 HTML 解析)
//!
//! **0 假装**: 真 HTTP 抓取 (reqwest), 真链接提取, 真上限控制.

use apeireth_tool_registry::{Tool, ToolAxes, ToolKind};
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::web_fetch::{FetchResult, ReqwestWebFetch, WebFetch};

const MAX_PAGES: usize = 10;
const MAX_DEPTH: usize = 2;
const PAGE_BYTES: usize = 50 * 1024;

/// 轻量爬虫 (抓取 + 链接提取 + 深度遍历).
pub struct CrawlTool {
    fetcher: ReqwestWebFetch,
}

impl Default for CrawlTool {
    fn default() -> Self {
        Self { fetcher: ReqwestWebFetch::new() }
    }
}

/// 从 HTML 提取 href 链接 (简单扫描, 免 regex 依赖).
fn extract_links(body: &str, base: &str) -> Vec<String> {
    let mut links = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i + 6 < bytes.len() {
        // 找 href=" 或 href='
        let window = &body[i..];
        let start = window.find("href=").map(|p| i + p + 5);
        let Some(start) = start else { break };
        let rest = &body[start..];
        let quote = rest.chars().next().unwrap_or('"');
        if quote != '"' && quote != '\'' {
            i = start + 1;
            continue;
        }
        let end = rest[quote.len_utf8()..]
            .find(quote)
            .map(|p| start + quote.len_utf8() + p);
        let Some(end) = end else { break };
        let raw = &body[start + quote.len_utf8()..end];
        let url = raw.trim();
        if url.starts_with("http://") || url.starts_with("https://") {
            links.push(url.to_string());
        } else if url.starts_with('/') {
            // 站内相对链接 → 拼接 base origin (scheme://host)
            if let Some(rest) = base.splitn(3, '/').nth(2) {
                let host = rest.split('/').next().unwrap_or(rest);
                let scheme = if base.starts_with("https://") { "https" } else { "http" };
                links.push(format!("{scheme}://{host}{url}"));
            }
        }
        i = end + 1;
    }
    links
}

/// 链接 URL 校验 (纯函数, 免 async 测试依赖).
pub fn validate_url(url: &str) -> Result<(), String> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        Err("只支持 http(s) 链接".into())
    }
}

/// 递归抓取: BFS 深度遍历 (去重 + 上限).
pub async fn crawl(
    fetcher: &dyn WebFetch,
    start_url: &str,
    max_pages: usize,
    max_depth: usize,
) -> Result<Vec<FetchResult>, String> {
    let mut pages: Vec<FetchResult> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<(String, usize)> = std::collections::VecDeque::new();
    queue.push_back((start_url.to_string(), 0));
    while let Some((url, depth)) = queue.pop_front() {
        if pages.len() >= max_pages {
            break;
        }
        if !seen.insert(url.clone()) {
            continue;
        }
        let page = match fetcher.fetch(&url, PAGE_BYTES).await {
            Ok(p) => p,
            Err(e) => {
                pages.push(FetchResult {
                    url: url.clone(),
                    status: 0,
                    content_type: String::new(),
                    bytes: 0,
                    body: format!("(抓取失败: {e})"),
                });
                continue;
            }
        };
        let is_html = page.content_type.contains("text/html");
        if is_html && depth < max_depth {
            for link in extract_links(&page.body, &url) {
                if !seen.contains(&link) {
                    queue.push_back((link, depth + 1));
                }
            }
        }
        pages.push(page);
        if pages.len() >= max_pages {
            break;
        }
    }
    Ok(pages)
}

#[async_trait]
impl Tool for CrawlTool {
    fn name(&self) -> &str {
        "Crawl"
    }
    fn kind(&self) -> ToolKind {
        ToolKind::Sync
    }
    fn axes(&self) -> ToolAxes {
        ToolAxes::default()
    }
    async fn call(&self, args: Value) -> Result<Value, String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "url 不能为空".to_string())?;
        validate_url(url)?;
        let max_pages = args.get("max_pages").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(5).min(MAX_PAGES);
        let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).map(|v| v as usize).unwrap_or(1).min(MAX_DEPTH);
        let pages = crawl(&self.fetcher, url, max_pages, max_depth).await?;
        let page_json: Vec<Value> = pages
            .iter()
            .map(|p| json!({
                "url": p.url,
                "status": p.status,
                "bytes": p.bytes,
                "content": p.body.chars().take(800).collect::<String>(),
            }))
            .collect();
        let links: Vec<String> = pages
            .iter()
            .flat_map(|p| extract_links(&p.body, &p.url))
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(20)
            .collect();
        Ok(json!({
            "pages": page_json,
            "total": pages.len(),
            "links_found": links,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_links_finds_absolute_and_relative() {
        let html = "<a href=\"https://a.com/x\">A</a><a href=\"/y\">B</a><a href=\"#frag\">C</a>";
        let links = extract_links(html, "https://a.com/page");
        assert!(links.contains(&"https://a.com/x".to_string()));
        assert!(links.contains(&"https://a.com/y".to_string()), "相对链接应拼接 origin");
        assert!(!links.iter().any(|l| l.contains("#frag")), "锚点不应算链接");
    }

    #[test]
    fn reject_non_http_url() {
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("ftp://x").is_err());
        assert!(validate_url("https://ok.com").is_ok());
    }

    #[tokio::test]
    async fn crawl_respects_limits_and_dedup() {
        // 用假 fetcher: 返回固定 HTML, 验证 BFS/去重/上限 (不发真网络)
        struct FakeFetch;
        #[async_trait]
        impl WebFetch for FakeFetch {
            async fn fetch(&self, url: &str, max_bytes: usize) -> Result<FetchResult, String> {
                Ok(FetchResult {
                    url: url.to_string(),
                    status: 200,
                    content_type: "text/html".into(),
                    bytes: max_bytes,
                    body: "<a href=\"https://x.com/1\">1</a><a href=\"https://x.com/2\">2</a><a href=\"https://x.com/1\">dup</a>".into(),
                })
            }
        }
        let pages = crawl(&FakeFetch, "https://x.com/start", 3, 1).await.unwrap();
        assert_eq!(pages.len(), 3, "上限 3 页 (start + 2 链接, dup 去重)");
        let urls: Vec<&str> = pages.iter().map(|p| p.url.as_str()).collect();
        assert_eq!(urls.iter().filter(|u| **u == "https://x.com/1").count(), 1, "去重");
    }
}
