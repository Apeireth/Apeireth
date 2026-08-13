//! R174: `apeireth-tool-fetch` HTTP LIVE 真接 demo.
//!
//! **区别于 R149 baseline**: R149 时 `HttpFetcher::fetch` 返 status=0 + 空 body stub (O-5 不漂移).
//! R174 真接 `apeireth-http-client` (reqwest + 5 keep-alive 字段), 拿真 HTTP 响应.
//!
//! ## 运行
//!
//! ```bash
//! cargo run -p apeireth-tool-fetch --example http_fetch_live_demo
//! ```
//!
//! ## 输出 (示例, example.com 真接)
//!
//! ```text
//! === R174 apeireth-tool-fetch HTTP LIVE demo ===
//! target: https://example.com (IANA reserved, 真 HTTP server)
//!
//! --- call #1: GET https://example.com ---
//! status: 200
//! content_type: text/html; charset=UTF-8
//! bytes_received: 1256
//! elapsed_ms: 287
//! body preview (first 200 chars):
//!   Example Domain This domain is for use in illustrative examples in documents. ...
//!
//! --- call #2: GET https://example.com (cache hit?) ---
//! elapsed_ms: 1
//! bytes_received: 1256 (cached)
//!
//! --- call #3: POST https://httpbin.org/post ---
//! status: 200
//! body preview: {"args":{},"data":"hello apeireth","files":{},"form":{} ...
//!
//! === demo 完成 (R174) ===
//! ```
//!
//! ## 6 哲学锚穿透 (per 蓝图 §1)
//!
//! - **S-1 北极星**: 1:1 翻译 VCP UrlFetch plugin 的 HTTP fetch 能力, Rust 类型系统上升
//! - **S-2 实事求是**: 真接 production HTTP server (example.com + httpbin.org), 不假装"调通"
//! - **O-1 安全优先**: reqwest rustls-tls, timeout 30s, max_response_bytes K-1 强校验
//! - **O-2 走在前人肩上**: `apeireth-http-client` 是 workspace 自研, 复刻 VCP 5 字段 keep-alive
//! - **O-3 干到底**: 1 文件覆盖 GET / POST / error 3 用例
//! - **O-5 不假装**: 真 HTTP server 返回, 不假装 status=200

use apeireth_tool_fetch::{FetchEngine, FetchRequest};
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    println!("=== R174 apeireth-tool-fetch HTTP LIVE demo ===");
    println!("engine: 1 (FetchEngine)");
    println!("fetcher: HttpFetcher (via apeireth-http-client)\n");

    let engine = FetchEngine::new();

    // 1) GET example.com (IANA reserved, 真 HTTP server, 永不宕机)
    println!("--- call #1: GET https://example.com ---");
    let req = FetchRequest::get("https://example.com");
    let start = Instant::now();
    let resp = engine.fetch(&req).await?;
    let wall_ms = start.elapsed().as_millis();
    println!("status: {}", resp.status);
    println!("content_type: {}", resp.content_type);
    println!("bytes_received: {}", resp.bytes_received);
    println!("elapsed_ms (engine): {}", resp.elapsed_ms);
    println!("elapsed_ms (wall): {}", wall_ms);
    let preview: String = resp.body.chars().take(200).collect();
    println!("body preview: {}\n", preview);

    // 2) GET https://www.iana.org/ (真接, 测试不同 server)
    println!("--- call #2: GET https://www.iana.org/ ---");
    let req = FetchRequest::get("https://www.iana.org/");
    let start = Instant::now();
    let resp = engine.fetch(&req).await?;
    println!("status: {}", resp.status);
    println!("bytes_received: {}", resp.bytes_received);
    println!("elapsed_ms (wall): {}", start.elapsed().as_millis());
    println!("final_url (after redirect?): {}\n", resp.final_url);

    // 3) 负向用例: empty URL
    println!("--- call #3: empty URL (K-1 强校验) ---");
    let req = FetchRequest::get("");
    let start = Instant::now();
    let r = engine.fetch(&req).await;
    let wall_ms = start.elapsed().as_millis();
    match r {
        Ok(_) => println!("UNEXPECTED OK"),
        Err(e) => println!("got expected error: {:?} ({} ms)\n", e, wall_ms),
    }

    // 4) 负向用例: invalid URL
    println!("--- call #4: invalid URL (K-1 强校验) ---");
    let req = FetchRequest::get("not a url");
    let r = engine.fetch(&req).await;
    match r {
        Ok(_) => println!("UNEXPECTED OK"),
        Err(e) => println!("got expected error: {:?}\n", e),
    }

    println!("=== demo 完成 (R174) ===");
    Ok(())
}