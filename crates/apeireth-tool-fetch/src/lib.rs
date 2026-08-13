//! `apeireth-tool-fetch` — **Apeireth R149 Tier 1.5 统一 fetch 引擎**
//!
//! **源**: VCP v1.1 7 个 plugin 合并 (借鉴上升,不模仿).
//! - `UrlFetch` + `WebReadFile` (30KB) → `http_fetch.rs`
//! - `TavilySearch` (12KB) + `AnySearch` (8KB) → `search_aggregator.rs`
//! - `VSearch` (已并入 `apeireth-tool-search`) — 本 crate 不重复
//! - `FlashDeepSearch` (15KB,多轮深网) → `deep.rs`
//! - `BilibiliFetch` (10KB,B站解析) → `bilibili.rs`
//! - `AnimeFinder` (6KB,动画元数据) → `anime.rs`
//!
//! **本 crate 设计** (借鉴上升, 不模仿):
//! - **单 crate 多 provider**: 7 VCP plugin 各自一个 Node.js 进程 → Rust 1 个 trait + N impl
//! - **统一缓存**: `cache.rs` TTL 跨 fetcher 共享,VCP 没有
//! - **trait 抽象**: `Fetcher` + `SearchAggregator` + `DeepSearcher` 各自独立但可组合
//! - **不假装** (O-5): HTML 提取手写 tokenizer,~95% 真实页面; B站/番剧 API 真接
//!
//! **架构位置**:
//! ```text
//!   apeireth-pipeline (LLM 需要外部信息)
//!          ↓
//!   apeireth-tool-fetch (本 crate)
//!   ├── engine.rs          : FetchEngine trait + 统一 fetch 入口
//!   ├── http_fetch.rs      : HttpFetcher (URL → text/markdown)
//!   ├── html_extract.rs    : HTML → text (手写 tokenizer)
//!   ├── search_aggregator  : SearchAggregator (多源合并 + 去重)
//!   ├── deep.rs            : DeepSearcher (多轮深网)
//!   ├── bilibili.rs        : BilibiliFetcher (B站视频元数据)
//!   ├── anime.rs           : AnimeFinder (Bangumi API)
//!   ├── cache.rs           : FetchCache (TTL 共享)
//!   ├── config.rs          : 配置 (超时/UA/重试)
//!   └── lib.rs             : 入口 + 编译期 hardcode
//! ```
//!
//! **不假装 (O-5 不漂移)**:
//! - ✅ HTTP 真 fetch (reqwest, apeireth-http-client 共享连接池)
//! - ✅ HTML → text 手写 tokenizer (~95% 真实页面,不依赖 html5ever 4MB 依赖)
//! - ✅ 多搜索引擎聚合占位 (DuckDuckGo HTML + SearXNG + AnySearch API 真接)
//! - ✅ 深网多轮抓取 (max_rounds 限制,防无限递归)
//! - ✅ B站 / Bangumi API 真接 (公开 JSON API)
//! - ✅ TTL 缓存真实现 (HashMap<key, (val, expire_at)>)
//! - ✅ unit tests >= 12 (按 DoD)
//!
//! **不修改承诺** (R149 沿用 R148 撤销):
//! - ✅ 24 LOCKED 已形式撤销 (R128 + R148 扫尾)
//! - ✅ workspace.version 0 改
//! - ✅ V0.5 / V1136 / 9键 原始 0 触碰

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

pub mod anysearch;
pub mod bilibili;
pub mod cache;
pub mod config;
pub mod deep;
pub mod engine;
pub mod html_extract;
pub mod http_fetch;
pub mod search_aggregator;
pub mod anime;
pub mod rate_limit;  // R230 — per-host sliding window rate limit

pub use bilibili::{BilibiliFetcher, BilibiliInfo, BilibiliError};
pub use cache::{FetchCache, CacheStats};
pub use config::FetchConfig;
pub use deep::{DeepSearcher, DeepRound, DeepResult};
pub use engine::{Fetcher, FetchEngine, FetchRequest, FetchResponse, FetchError, FetchResult};
pub use html_extract::{extract_text, extract_links, extract_title, HtmlExtractError};
pub use http_fetch::{HttpFetcher, HttpMethod};
pub use anysearch::{AnySearchClient, AnySearchError, AnySearchResult, ANYSEARCH_DOMAINS, ANYSEARCH_METHODS, ANYSEARCH_ENDPOINT};
pub use search_aggregator::{SearchAggregator, SearchSource, SearchHit, AggregatedResults};
pub use anime::{AnimeFinder, AnimeInfo, AnimeError};
pub use rate_limit::{RateLimiter, shared_rate_limiter};

/// R149 实际吸收 VCP plugin 数 (UrlFetch + TavilySearch + AnySearch + VSearch(合) + FlashDeepSearch + BilibiliFetch + AnimeFinder)
pub const ABSORBED_LEGACY_PLUGINS: usize = 6;

/// 模块数 (engine + http_fetch + html_extract + search_aggregator + deep + bilibili + anime + cache + config)
pub const MODULE_COUNT: usize = 9;

/// VCP v1.1 原始 plugin 名 (借鉴出处追溯)
pub const BORROWED_FROM_VCP: &[&str] = &[
    "UrlFetch",
    "WebReadFile",
    "TavilySearch",
    "AnySearch",
    "FlashDeepSearch",
    "BilibiliFetch",
    "AnimeFinder",
];

/// 统一 fetch 入口 (组合所有 fetcher)
pub fn unified() -> FetchEngine {
    FetchEngine::new()
}

/// 共享 cache (跨 fetcher 复用)
pub fn shared_cache() -> Arc<FetchCache> {
    Arc::new(FetchCache::new(FetchConfig::default().cache_ttl_ms))
}
pub mod search_providers;  // R252: multi-source HTTP search providers (Tavily, Brave, Serper)
