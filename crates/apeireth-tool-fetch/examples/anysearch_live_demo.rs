//! R176: `apeireth-tool-fetch::anysearch` LIVE demo.
//!
//! **区别于 R149 baseline**: R149 时 `search_aggregator` 只做数据层, 无真 HTTP backend.
//! R176 新增 `AnySearchClient`, 真接 AnySearch production JSON-RPC API.
//!
//! ## 运行
//!
//! ```bash
//! # 匿名访问 (无 API Key, 额度低)
//! cargo run -p apeireth-tool-fetch --example anysearch_live_demo
//!
//! # 带 API Key (推荐, 去 https://anysearch.com/console/api-keys 申请)
//! $env:ANYSEARCH_API_KEY = "as_sk_xxx"
//! cargo run -p apeireth-tool-fetch --example anysearch_live_demo
//! ```
//!
//! ## 6 哲学锚穿透 (per 蓝图 §1)
//!
//! - **S-1 北极星**: 1:1 翻译 AnySearch JSON-RPC spec, 18 垂直领域 + 4 命令
//! - **S-2 实事求是**: R176 LIVE 真接 AnySearch production endpoint, 不假装
//! - **O-1 安全优先**: api_key 走 env, 0 硬编码, Bearer auth
//! - **O-2 走在前人肩上**: 借鉴 VCP AnySearch.js + `apeireth-http-client` (workspace 自研)
//! - **O-3 干到底**: 1 文件覆盖 search / get_sub_domains / batch_search / extract 4 命令
//! - **O-5 不假装**: 失败如实返 AnySearchError::*, 不假装成功

use apeireth_tool_fetch::{
    AnySearchClient, SearchAggregator, ANYSEARCH_DOMAINS, ANYSEARCH_ENDPOINT,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    println!("=== R176 apeireth-tool-fetch AnySearch LIVE demo ===");
    println!("endpoint: {ANYSEARCH_ENDPOINT}");
    println!("domains: {} vertical\n", ANYSEARCH_DOMAINS.len());

    // 1) 创建 client — 优先 env, fallback 匿名
    let keys: Vec<String> = std::env::var("ANYSEARCH_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.split(',').map(|k| k.trim().to_string()).collect())
        .unwrap_or_default();
    let client = AnySearchClient::with_keys(ANYSEARCH_ENDPOINT, keys)?;
    let key_mode = if client.key_count() == 0 { "anonymous" } else { "authenticated" };
    println!("client: {} ({} keys)\n", key_mode, client.key_count());

    // 2) search 命令: 通用搜索
    println!("--- search #1: general 'Rust async programming' ---");
    match client.search("Rust async programming", Some("general"), None, "").await {
        Ok(md) => {
            let preview: String = md.chars().take(300).collect();
            println!("OK ({} chars): {}\n", md.len(), preview);
        }
        Err(e) => println!("ERR: {:?}\n", e),
    }

    // 3) search 命令: code 垂直搜索
    println!("--- search #2: code 'rust tokio runtime' ---");
    match client.search("rust tokio runtime", Some("code"), None, "").await {
        Ok(md) => {
            let preview: String = md.chars().take(300).collect();
            println!("OK ({} chars): {}\n", md.len(), preview);
            // 解析为 SearchHits, 塞进 SearchAggregator
            let agg = SearchAggregator::new();
            let hits = client.markdown_to_hits("rust tokio runtime", &md, 5);
            agg.add_hits(apeireth_tool_fetch::SearchSource::AnySearch, hits);
            let r = agg.aggregate("rust tokio runtime", 5);
            println!("aggregated: {} hits from {} sources", r.hits.len(), r.total_sources);
            for (i, h) in r.hits.iter().enumerate() {
                println!("  [{}] {} -> {}", i + 1, h.title, h.url);
            }
            println!();
        }
        Err(e) => println!("ERR: {:?}\n", e),
    }

    // 4) get_sub_domains 命令: 看 academic 的子域
    println!("--- get_sub_domains: academic ---");
    match client.get_sub_domains("academic").await {
        Ok(v) => {
            let preview = serde_json::to_string(&v).unwrap_or_default();
            println!("OK: {}\n", preview.chars().take(400).collect::<String>());
        }
        Err(e) => println!("ERR: {:?}\n", e),
    }

    // 5) batch_search 命令: 多查询并行
    println!("--- batch_search: ['Rust', 'Tokio', 'async'] ---");
    match client.batch_search(&["Rust", "Tokio", "async"], Some("general")).await {
        Ok(md) => {
            let preview: String = md.chars().take(300).collect();
            println!("OK ({} chars): {}\n", md.len(), preview);
        }
        Err(e) => println!("ERR: {:?}\n", e),
    }

    // 6) extract 命令: 抓 URL 转 Markdown
    println!("--- extract: https://www.rust-lang.org ---");
    match client.extract("https://www.rust-lang.org").await {
        Ok(md) => {
            let preview: String = md.chars().take(300).collect();
            println!("OK ({} chars): {}\n", md.len(), preview);
        }
        Err(e) => println!("ERR: {:?}\n", e),
    }

    println!("=== demo 完成 (R176) ===");
    Ok(())
}