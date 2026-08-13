# apeireth-tool-fetch

**R149** - URL fetch + search + aggregation toolkit.

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
