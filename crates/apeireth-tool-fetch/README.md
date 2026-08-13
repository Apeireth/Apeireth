# apeireth-tool-fetch

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
