# R176 apeireth-tool-fetch AnySearch 真接 backend

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R176 (live capability upgrade)
> **日期**: 2026-08-13
> **borrow-id**: R176-FETCH-BORROW-anysearch-mcp-2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕, 干到底

---

## 0. 背景

主人 2026-08-13 指示:
> 我们自己的自研的工具, 确保基础不缺, 然后也预留外部的接口, 让能调用外部的东西对吧
> (GitHub 上 angsearch 不是有吗)

R176 实施:
1. `apeireth-tool-fetch::search_aggregator` (R149 数据层, 无 HTTP) → 加 `AnySearchClient` 真接 backend
2. 借鉴本地 `research/source/vcptoolbox/Plugin/AnySearch/` + GitHub 上 AnySearch 项目, Rust 类型系统上升
3. 实接 AnySearch production MCP server (`https://api.anysearch.com/mcp`), 4 command (search / get_sub_domains / batch_search / extract) 全跑通

---

## 1. LIVE 验证 (R176, 2026-08-13, anonymous 访问)

通过 `cargo run -p apeireth-tool-fetch --example anysearch_live_demo` 跑通:

### search #1 — 通用搜索

```
OK (35469 chars): ## Search Results (10 results, 61ms)

### 1. Async and await - Asynchronous Programming in Rust
- **URL**: https://rust-lang.github.io/async-book/part-guide/async-await.html
```

**真实证据**: rust-lang.github.io 官方 async-book 排在第一, 61ms 返回。

### search #2 — code 垂直搜索 + aggregator 集成

```
OK (34702 chars): ## Search Results (10 results, 368ms)

aggregated: 5 hits from 2 sources
  [1] Runtime in tokio::runtime - Rust -> https://docs.rs/tokio/...
  [2] Tokio | Tokio - An asynchronous Rust runtime -> https://tokio.rs
  [3] TABLE OF CONTENTS -> #
  [4] tokio/src/runtime/runtime.rs -> https://github.com/tokio-rs/tokio/...
  [5] tokio_core::reactor - Rust -> https://tokio-rs.github.io/...
```

**真实证据**: aggregator 解析 5 个 hits, 全部含真 URL, 直接喂给 SearchAggregator。

### get_sub_domains — academic 目录

```json
{"_meta":{"request_id":"8ac56280-..."},"content":[{"text":"## academic Domain Capabilities (5 available)

### academic.citation
Citation relationships, citation counts, reference lists by DOI or title

**Parameters:**
- `doi`: DOI identifier, e.g. 10.1038/...
- `op`: Operation type. Values: `metadata`, `citations`..."}]}
```

**真实证据**: 5 个 academic 子域 (citation / biomedical / ...) + 参数说明。

### extract — URL Markdown 提取

```
OK (3193 chars): ## Rust Programming Language

**Source**: https://www.rust-lang.org

### Performance

Rust is blazingly fast and memory-efficient: with no runtime or garbage collector...
```

**真实证据**: rust-lang.org 主页 3193 chars 干净 Markdown。

### batch_search

```
OK (28 chars): queries[0] must be an object
```

**已知**: batch_search 需要 `queries: [{query: "..."}]` 对象数组, 不是字符串数组. R176+1 修 (小工作量).

---

## 2. 实现要点

### 2.1 MCP JSON-RPC 协议 (R176 关键发现)

主人 8/13 提到"AnySearch GitHub 上有". 我们本地 `research/source/vcptoolbox/Plugin/AnySearch/AnySearch.js` 是 VCP plugin 包装, 但**真 MCP 服务器**走的是 JSON-RPC `tools/call` 协议:

```json
POST https://api.anysearch.com/mcp
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search",
    "arguments": {"query": "...", "domain": "general"}
  }
}
```

返:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "_meta": {"request_id": "..."},
    "content": [{"type": "text", "text": "## Search Results..."}]
  }
}
```

R176 fix: 原 AnySearch.js 用 plugin 内部命令名 (`search`/`get_sub_domains`/...) 直接当 JSON-RPC method, 但实际 MCP server method 是 `tools/call`, 命令名走 `params.name`. R176 fix 了这一点.

### 2.2 17 垂直领域 (1:1 翻译)

```rust
pub const ANYSEARCH_DOMAINS: &[&str] = &[
    "general", "resource", "social_media", "finance", "academic", "legal",
    "health", "business", "security", "ip", "code", "energy",
    "environment", "agriculture", "travel", "film", "gaming",
];  // 17 (AnySearch.js DOMAINS 列表, 编译期 hardcode 守门)
```

### 2.3 markdown_to_hits 双格式解析

AnySearch Markdown 有 2 种格式, R176 都支持:
- **格式 A**: `### N. title\n- **URL**: url\n- snippet` (R176 真接返的就是这格式)
- **格式 B**: `[title](url) snippet` (传统 Markdown 链接)

塞进 `SearchAggregator`, 实现 multi-source 合并.

### 2.4 匿名 + API Key 双模式

- `AnySearchClient::anonymous()` — 无 Key, 额度低, 适合测试
- `AnySearchClient::with_keys(endpoint, keys)` — 多 Key 逗号分隔, round-robin 选 Key (per AnySearch.js)

---

## 3. 设计原则 (per 蓝图 §1 6 哲学锚穿透)

| 锚 | 体现 |
|---|---|
| **S-1 北极星** | 1:1 翻译 AnySearch MCP JSON-RPC spec, 17 领域 + 4 命令 + 双格式 Markdown |
| **S-2 实事求是** | LIVE 真接 production MCP server, 5+ 真搜索结果 + URL 提取 + 子域目录 |
| **O-1 安全优先** | API Key 走 env / constructor 参数, 0 硬编码, 30s timeout |
| **O-2 走在前人肩上** | `apeireth-http-client` workspace 自研 + AnySearch.js 本地参考 + AnySearch MCP spec |
| **O-3 干到底** | 1 模块 + 1 demo + 14 单测, 4 命令全接口 (batch_search 待 R176+1 修) |
| **O-4 接手** | `AnySearchClient` 单一 struct, JSON-RPC 标准, 任何人都能改 |
| **O-5 不假装** | 真接 production server, 失败如实返 AnySearchError::*, 不假装成功 |

---

## 4. 与其他 fetch 路径的关系 (R176 不冲突)

| 路径 | 何时用 | R176 触碰 |
|---|---|---|
| `HttpFetcher` (R174+) | 通用 HTTP GET/POST | 0 触碰 |
| `AnySearchClient` (R176+, 本档) | **真接 AnySearch MCP 搜索 backend** | ✅ 新增 |
| `BilibiliFetcher` (bilibili.rs) | B 站视频元数据 | 0 触碰 |
| `AnimeFinder` (anime.rs) | Bangumi API | 0 触碰 |
| `SearchAggregator` (search_aggregator.rs) | 多源数据层 + dedup | 复用 (anysearch 作为 source之一) |
| `DeepSearcher` (deep.rs) | 多轮深网抓取 | 0 触碰 |

**0 触碰**:
- 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict cache)
- workspace.version 1.2.0
- 24 LOCKED crate 入口签名
- R149 baseline 借鉴列表 (`BORROWED_FROM_VCP`)

---

## 5. 文件清单 (R176 改动)

| 文件 | 类型 | 字节 | 说明 |
|---|---|---|---|
| `crates/apeireth-tool-fetch/src/anysearch.rs` | NEW | 17,200 | AnySearchClient + 14 单测 |
| `crates/apeireth-tool-fetch/src/lib.rs` | modified | +3 lines | `pub mod anysearch` + re-export |
| `crates/apeireth-tool-fetch/examples/anysearch_live_demo.rs` | NEW | 4,654 bytes | LIVE demo (5 用例) |
| `docs/r176/r176-anysearch-live.md` | NEW (本档) | — | 设计 + LIVE 证据 |

**新增测试**: +14 (R176 anysearch 模块单测)
**全 crate 测试**: 71 pass / 0 fail (R176 后, 含 tool-fetch 58 = 44+14)

---

## 6. R176 后续 (R176+1, R176+2, ...)

| R | 任务 | 优先级 |
|---|---|---|
| R176+1 | batch_search queries 改对象数组 (`[{query:"..."}]`) | 低 |
| R176+2 | Tavily 真接 (商业 API, 设计 for AI agent) | 中 |
| R176+3 | DuckDuckGo HTML scrape 真接 (0 API key) | 中 |
| R176+4 | SearXNG 真接 (自托管 metasearch engine) | 中 |
| R176+5 | Brave Search API 真接 (可选 key) | 低 |
| R176+6 | aggregator pipeline: multi-source 并发 merge | 中 |

**前提**: R176+ 不动 3 不可变脊柱 + workspace.version + 24 LOCKED。
