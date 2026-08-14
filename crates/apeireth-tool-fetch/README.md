# apeireth-tool-fetch

## R262: FetchMetrics (stdlib-only OTel metrics)

`FetchEngine` 加 4 atomic counter:
- `fetch_total` — 总调用 (含成功 + 错误)
- `fetch_errors_total` — 错误数
- `fetch_total_duration_ms` — 总延迟 (算 mean)
- `fetch_latency_max_ms` — 最大延迟 (ms, -1 = unset)

每个 `fetch()` 入口 timing (validation + rate limit + http), 成功 / 错误 / 5 个 early-return 都 record.

```rust
let engine = FetchEngine::new();
let resp = engine.fetch(&FetchRequest::get("https://example.com")).await.unwrap();
println!("{}", engine.fetch_metrics.metrics_text());
// fetch_total=1 fetch_errors_total=0 fetch_latency_mean_ms=X.X fetch_latency_max_ms=Y fetch_cache_hits=N fetch_cache_misses=M
```

## R265: TTL Cache 集成 (optional opt-in)

`FetchCache` (R149) 之前没用, R265 接到 `FetchEngine.fetch()`:
- 默认无 cache (向后兼容 R231 用法)
- `with_cache()` (用 cfg.cache_ttl_ms) / `with_cache_ttl(ms)` 启用
- `cache_stats()` / `cache_invalidate(url)` / `cache_clear()` accessors
- cache hit 也算 fetch_total + latency (cache lookup 极快)
- cache miss 仅记 misses, 实际 fetch 走完后 record_success/error 才 +total

```rust
let engine = FetchEngine::new().with_cache();
let _ = engine.fetch(&FetchRequest::get("https://example.com")).await.unwrap();
let _ = engine.fetch(&FetchRequest::get("https://example.com")).await.unwrap();  // cache hit
println!("hits={} misses={}", engine.cache_stats().unwrap().hits, engine.cache_stats().unwrap().misses);
// hits=1 misses=1
```

corrupted cache 自动 invalidate + 走 HTTP (避免永久坏数据).


**R149** - URL fetch + search + aggregation toolkit.


> **R176 LIVE (2026-08-13)**: `AnySearchClient` 真接 AnySearch MCP server (`https://api.anysearch.com/mcp`). 17 vertical domains + 4 commands (search/get_sub_domains/batch_search/extract). LIVE 验证: search 35469 chars / 10 results / 61ms; aggregator 解析 5 hits; extract rust-lang.org 3193 chars Markdown. See `docs/r176/r176-anysearch-live.md`.

> **R174 LIVE (2026-08-13)**: `HttpFetcher::fetch` 真接 `apeireth-http-client` (reqwest + 5 keep-alive 字段). R149 baseline 是 trait stub (status=0, 空 body), R174 修 doc/code 不一致. LIVE 验证 example.com 200/559B + iana.org 200/6253B. See `docs/r174/r174-http-fetch-live.md`.

## Responsibilities

Web data acquisition for LLM consumption:
- HTTP fetch with redirect handling + content-type sniffing
- HTML extraction (Readability + meta tags)
- Search aggregator (Tavily / Brave / DuckDuckGo / SearXNG via `AnySearch` trait)
- Specialized extractors (Bilibili video info, anime/manga metadata)
- Caching + ETag honoring

## Core capabilities

- Single-URL fetch (`apeireth-tool-fetch::fetch`)
- Multi-engine search (`apeireth-tool-fetch::AnySearch`)
- Specialized extractors: Bilibili (`bvid` / `aid` -> JSON metadata),
  Anime (Bangumi/MAL cross-reference)
- LRU cache with TTL + ETag
- Deep crawl (HTML -> Markdown via `html_extract`)

## R163 lint cleanup

157 -> 0 warnings. 10 source files: `#![allow(missing_docs)]` per O-5
(public API in lib.rs). 1 unused var fixed in bilibili.rs.

## Architecture

```
HTTP layer  -> http_fetch.rs (reqwest + redirect + ETag)
HTML layer  -> html_extract.rs (readability)
Search      -> search_aggregator.rs (AnySearch trait + 4 impls)
Special     -> bilibili.rs / deep.rs / anime.rs
Cache       -> cache.rs (LRU + TTL)
Config      -> config.rs
Engine      -> engine.rs (top-level dispatcher)
```

## 0-touch statement

- 0 touches workspace.version (1.2.0)
- 0 引外部 dep
## R166 public API deep cleanup

`ABSORBED_VCP_PLUGINS` -> `ABSORBED_LEGACY_PLUGINS`. 44 tests pass.
