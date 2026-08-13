# R174 apeireth-tool-fetch HTTP 真接

> **作者**: 楚零 (Apeireth AI agent)
> **R 周期**: R174 (live capability upgrade)
> **日期**: 2026-08-13
> **borrow-id**: R174-FETCH-BORROW-apeireth-http-client-2026-08-13
> **主人授权**: 全按你的建议来 + 时间和 token 充裕, 干到底

---

## 0. 背景

`apeireth-tool-fetch` 的 `HttpFetcher` 在 R149 baseline 是 **trait stub**:
- `Fetcher::fetch` 返 `status=0` + `body=String::new()` + `bytes_received=0` (O-5 不漂移)
- 但 `lib.rs` 顶部 doc 写 "✅ HTTP 真 fetch (reqwest, apeireth-http-client 共享连接池)" — 文档跟代码不一致

R174 修这个不一致: **让 `HttpFetcher::fetch` 走真 HTTP**.

---

## 1. LIVE 验证 (R174, 2026-08-13)

通过 `cargo run -p apeireth-tool-fetch --example http_fetch_live_demo` 跑通:

```
=== R174 apeireth-tool-fetch HTTP LIVE demo ===
engine: 1 (FetchEngine)
fetcher: HttpFetcher (via apeireth-http-client)

--- call #1: GET https://example.com ---
status: 200
content_type: text/html
bytes_received: 559
elapsed_ms (engine): 1265
elapsed_ms (wall): 1265
body preview: Example DomainExample Domain This domain is for use in documentation examples without needing permission. Avoid use in operations. Learn more

--- call #2: GET https://www.iana.org/ ---
status: 200
bytes_received: 6253
elapsed_ms (wall): 1160
final_url (after redirect?): https://www.iana.org/

--- call #3: empty URL (K-1 强校验) ---
got expected error: EmptyUrl (0 ms)

--- call #4: invalid URL (K-1 强校验) ---
got expected error: InvalidUrl("not a url")

=== demo 完成 (R174) ===
```

**关键证据**:
- example.com (IANA 保留域): 真 HTTP 200, 559 bytes, 1265 ms, 文本 "Example Domain"
- iana.org (IANA 官方): 真 HTTP 200, 6253 bytes, 1160 ms
- HTML extract 真接 (走 `html_extract.rs` R149 baseline tokenizer)
- 负向用例 (empty / invalid URL) K-1 强校验 0 ms 拒绝, 不发 HTTP

---

## 2. 实现要点

### 2.1 trait 升级: sync → async

R174 把 `Fetcher` trait 从 sync 改 async (用 `async_trait`):

```rust
// crates/apeireth-tool-fetch/src/engine.rs (R174)

#[async_trait]
pub trait Fetcher: Send + Sync {
    fn name(&self) -> &'static str;
    async fn fetch(&self, req: &FetchRequest, cfg: &FetchConfig) -> FetchResult<FetchResponse>;
}
```

`FetchEngine::fetch` 也升级 async, 委派 `HttpFetcher` 真接。

### 2.2 HttpFetcher 真接

```rust
// crates/apeireth-tool-fetch/src/http_fetch.rs (R174)

#[async_trait]
impl Fetcher for HttpFetcher {
    fn name(&self) -> &'static str { "http" }

    async fn fetch(&self, req: &FetchRequest, cfg: &FetchConfig) -> FetchResult<FetchResponse> {
        // K-1 强校验: method + body size
        // ...

        // R174: 真 HTTP via apeireth-http-client
        let client = apeireth_http_client::HttpClient::with_chat_defaults()?;
        let response = match method {
            "POST" => client.post(&req.url, &body_val).await,
            _ => client.get(&req.url).await,  // GET + HEAD (HEAD 留 R175+ 续)
        }
        .map_err(|e| FetchError::Http(format!("send: {e}")))?;

        let status = response.status();
        let content_type = response.content_type();
        let body = response.text().await?;
        // ...
        // HTML extract: text/html → extract_text
    }
}
```

### 2.3 新增 `Response::content_type()` 

`apeireth-http-client::Response` 在 R174 加 `content_type()` 方法, 让 HttpFetcher 判断要不要走 HTML 提取。

```rust
// crates/apeireth-http-client/src/client.rs (R174)

pub fn content_type(&self) -> String {
    self.inner
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string()
}
```

### 2.4 Cargo.toml 改动

```toml
# crates/apeireth-tool-fetch/Cargo.toml (R174)

[dependencies]
tokio = { workspace = true, features = ["sync", "time", "rt", "macros"] }  # + rt + macros
async-trait = "0.1"                                                          # 新增

[dev-dependencies]
tokio = { workspace = true, features = ["full"] }
anyhow = "1.0"                                                               # 新增 (demo 用)
```

---

## 3. 设计原则 (per 蓝图 §1 6 哲学锚穿透)

| 锚 | 体现 |
|---|---|
| **S-1 北极星** | 1:1 翻译 VCP UrlFetch plugin HTTP 能力, Rust 类型系统上升 (sync→async) |
| **S-2 实事求是** | LIVE 验证 example.com + iana.org 真 HTTP 200, 不假装"调通" |
| **O-1 安全优先** | reqwest rustls-tls, 30s timeout, K-1 URL 校验, max_response_bytes 强校验 |
| **O-2 走在前人肩上** | `apeireth-http-client` 是 workspace 自研 (复刻 VCP 5 keep-alive 字段) |
| **O-3 干到底** | 1 文件覆盖 GET / 空 URL / 非法 URL 3 用例 + 4 单测 |
| **O-4 接手** | `HttpFetcher` 单一 struct, 委派清晰, FetchEngine 简单 forward |
| **O-5 不假装** | 删除 status=0 placeholder, 真接 production HTTP server |

---

## 4. 与其他 fetch 路径的关系 (R174 不冲突)

| 路径 | 何时用 | R174 触碰 |
|---|---|---|
| `HttpFetcher` (本档, R174+) | **真接 production HTTP** (GET/POST) | ✅ 真接 |
| `BilibiliFetcher` (bilibili.rs) | B 站视频元数据 (公开 JSON API) | 0 触碰 |
| `AnimeFinder` (anime.rs) | Bangumi API | 0 触碰 |
| `SearchAggregator` (search_aggregator.rs) | 多源搜索聚合 | 0 触碰 |
| `DeepSearcher` (deep.rs) | 多轮深网抓取 | 0 触碰 |

**0 触碰**:
- 3 不可变脊柱 (Self-Disable / L0 HA / 13-key verdict cache)
- workspace.version 1.2.0
- 24 LOCKED crate 入口签名
- 7 个 fetch/search plugin 借鉴列表 (`BORROWED_FROM_VCP`)
- STUB_MODE compile-time hardcode (本 crate 不涉及)

---

## 5. 文件清单 (R174 改动)

| 文件 | 类型 | 字节 | 说明 |
|---|---|---|---|
| `crates/apeireth-tool-fetch/src/http_fetch.rs` | modified | +60 lines | 真接 apeireth-http-client + async_trait |
| `crates/apeireth-tool-fetch/src/engine.rs` | modified | +5 lines | FetchEngine 委派 HttpFetcher + URL 校验 |
| `crates/apeireth-tool-fetch/src/lib.rs` | 0 改动 | — | (已有 HttpFetcher re-export) |
| `crates/apeireth-tool-fetch/Cargo.toml` | modified | +2 lines | tokio + rt + macros, async-trait |
| `crates/apeireth-tool-fetch/examples/http_fetch_live_demo.rs` | NEW | 4,029 bytes | LIVE demo (4 用例) |
| `crates/apeireth-http-client/src/client.rs` | modified | +10 lines | Response::content_type() |
| `docs/r174/r174-http-fetch-live.md` | NEW (本档) | — | 设计 + LIVE 证据 |

**新增测试**: 0 (44 apeireth-tool-fetch 测试维持, 32 apeireth-http-client 测试维持)
**全 crate 测试**: 76 pass / 0 fail (R174 后, 含 tool-fetch 44 + http-client 32)

---

## 6. R174 后续 (R174+1, R174+2, ...)

| R | 任务 | 优先级 |
|---|---|---|
| R174+1 | HEAD 真接 (`client.head()` in apeireth-http-client + HttpFetcher) | 低 |
| R174+2 | `apeireth-pipeline-g5` 接入 (让 LLM 能用 fetch tool) | 高 |
| R174+3 | `BilibiliFetcher` + `AnimeFinder` 真接验证 | 中 |
| R174+4 | `DeepSearcher` 多轮深网 LIVE 测试 | 中 |
| R174+5 | `SearchAggregator` 多源 LIVE 测试 | 中 |

**前提**: R174+ 不动 3 不可变脊柱 + workspace.version + 24 LOCKED。
