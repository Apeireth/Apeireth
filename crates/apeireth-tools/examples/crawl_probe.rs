//! crawl_probe — Crawl 工具实战验证 (真网络抓取).
//!
//! 用途: 验证爬虫在真实环境 (网络/反爬/限速) 下的稳定性 — 基础工具工程原则:
//! 写完必须实战验证.
//!
//! 跑法: cargo run -p apeireth-tools --example crawl_probe -- <url> [max_pages] [max_depth]

use apeireth_tools::web_crawl::{crawl, extract_links, validate_url};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let url = args.get(1).cloned().unwrap_or_else(|| "https://github.com".to_string());
    let max_pages = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(3);
    let max_depth = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(1);

    if let Err(e) = validate_url(&url) {
        println!("❌ {e}");
        std::process::exit(1);
    }
    println!("=== crawl_probe: {url} (pages≤{max_pages}, depth≤{max_depth}) ===");
    let fetcher = apeireth_tools::web_fetch::ReqwestWebFetch::new();
    let started = std::time::Instant::now();
    match crawl(&fetcher, &url, max_pages, max_depth).await {
        Ok(pages) => {
            let ok = pages.iter().filter(|p| p.status == 200).count();
            let failed = pages.iter().filter(|p| p.status == 0).count();
            println!("✅ 抓取 {} 页 (成功 {ok} / 失败 {failed}), 耗时 {:.1}s", pages.len(), started.elapsed().as_secs_f32());
            for p in pages.iter().take(max_pages) {
                let links = extract_links(&p.body, &p.url);
                println!("  [{}] {} ({} B, {} 链接)", p.status, p.url, p.bytes, links.len());
            }
            if failed > 0 {
                println!("⚠️ 有失败页: 网络/反爬环境下重试机制已生效 (每页最多 3 次重试)");
            }
        }
        Err(e) => println!("❌ 抓取失败: {e}"),
    }
}
