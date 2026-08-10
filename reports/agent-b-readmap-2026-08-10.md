# Agent B Readmap — 战区 2 (LLM Gateway) 升级前置

**时间**: 2026-08-10 03:00-04:30 (B1 阶段)
**作者**: 团队成员 B (派活方 Mavis, 主人 02:55 离场, 7h 窗口)
**目标**: 摸清 `apeireth-api` 当前形态, 确认 B2-B4 hook 点位, 列清单

---

## 1. 项目形态 (1.1 release, R35 1.1 合并后)

| 项 | 值 | 来源 |
| --- | --- | --- |
| workspace version | **1.1.0** | `Cargo.toml:246` (R35 1.1 release) |
| 协议端点数 | 4 (OpenAI Chat / OpenAI Responses / Anthropic Messages / Gemini) | `apeireth-api/src/server.rs:117-122` |
| 4 协议编解码 | 1 文件, 47KB, ~1500 行 | `apeireth-api/src/protocol_handlers.rs` |
| V2 6 类 JSON 端点 | 6 类 (tools/memory/organs/asi/sovereignty/agent), 1 文件 84KB | `apeireth-api/src/v2_endpoints.rs` |
| 流式 SSE 直通 | R25 修, OpenAI Chat 走 reqwest bytes_stream | `apeireth-api/src/server.rs:216-257` |
| telemetry umbrella | R35 1.1, 4 module (cache/metric/trace/observability) | `apeireth-telemetry/src/lib.rs:1-58` |
| cache 1:1 商业版 | `apeireth-cache` 5 policy + 4 backend (Memory 真接) | `apeireth-cache/src/lib.rs:215-372` |

---

## 2. 当前 retry / cache 逻辑在哪 (B3 / B2 hook 入口)

### 2.1 retry —— 散在 apeireth-http-client, 不在 protocol_handlers

- **Retry 入口**: `apeireth-http-client` 的 `HttpClient` 内部 (战役 1-2 Keep-Alive LIFO 5 字段 + retry), **不是** `protocol_handlers`
- **当前退避**: 只有 1s / 3s / 10s 三档 (主人 §6 失败模式说明)
- **不重试**: 4xx (除 408/429), 5xx 重试, network error 重试
- **B3 hook 点**: 在 `apeireth-api/src/protocol_handlers.rs::dispatch()` (line 798) 外层包 retry wrapper
  - 输入: `&Pipeline`, `ProtocolKind`, `NormalizedRequest`, `BackoffPolicy`
  - 输出: 多次调 `dispatch` 直到成功 / 退避耗尽
  - 关键: **4xx 立刻返, 5xx / network 走退避**

### 2.2 cache —— **完全没接**, 唯一相关是 v2_endpoints.rs 里的 `lru::LruCache` (AgentManager alias 解析用)

- **当前状态**: 0 任何 response cache
- **唯一缓存**: `v2_endpoints.rs:63` `use lru::LruCache;` 给 V2Agent alias 解析
- **B2 hook 点**: `protocol_handlers.rs::dispatch()` (line 798) 包 cache wrapper
  - 入口: `cache_lookup(&req, &kind)` 返 `Option<NormalizedResponse>`
  - 命中 → 写 `cache.hit` metric → 直接返
  - 未命中 → 走原 `dispatch` → 成功后 `cache_store(&req, &resp, &kind)` + `cache.put` metric
  - 关键: **stream (req.stream) 跳过 cache**

### 2.3 metrics / tracing —— telemetry crate 已 1.1 合并, 但 `apeireth-api` Cargo.toml **没引**

- `apeireth-api/Cargo.toml:14-58` 现有 deps: `apeireth-observability`, `apeireth-core`, `apeireth-protocol`, `apeireth-http-client`, `apeireth-pipeline`, `apeireth-tool-registry`, `apeireth-tools`, `apeireth-keyring`, `lru 0.16`, `tracing 0.1`
- **缺**: `apeireth-telemetry` (1.1 umbrella), `apeireth-cache` (cache 1:1 翻译)
- **B2 需改**: `apeireth-api/Cargo.toml` 加 `apeireth-cache = { path = "../apeireth-cache" }` + `apeireth-telemetry = { path = "../apeireth-telemetry" }`
- **关键 API**:
  - `apeireth_telemetry::metric::counter::Counter::new(name, help, labels).unwrap().inc()`
  - `apeireth_telemetry::trace::span::Span` / `SpanKind::Server` / `SpanStatus::Ok`
  - `apeireth_cache::{CacheConfig, EvictionPolicy, BackendKind, MemoryCache, build_cache}`

---

## 3. LlmProvider trait 形状 (R17 战役 0 保留, 用于 council_advise)

`apeireth-api/src/llm/` 目录 (lib.rs:94 `pub mod llm;`), R17 战役 0 保留:
- `trait LlmProvider: Send + Sync`
- `async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>`
- `name() -> &str` 等辅助

**重要**: `LlmProvider` **不在战区 2 主路径上**。主路径 (4 协议端点) 走 `apeireth-pipeline::Pipeline` 5 步管线, **不**走 LlmProvider。只有 `/council/advise` 走 LlmProvider (server.rs:328-405)。

**B2-B3 决策**: 我只对 4 协议主路径加 cache + retry。`/council/advise` 不动 (不在战区 2 范围)。

---

## 4. 主路径 flow (战区 2 核心 4 协议)

```
客户端 → server.rs 4 endpoint
  ↓ req → protocol_handlers::*_to_normalized(&req)
  ↓ NormalizedRequest
  ↓ dispatch(&pipeline, kind, normalized)         ← B2 cache hook 在此入口
    ↓ match kind
      ↓ OpenAI Chat / OpenAI Responses
        ↓ pipeline.run(kind, input) 5 步管线
      ↓ Gemini / Anthropic@minimaxi
        ↓ run_gemini / run_anthropic_minimaxi
          ↓ run_pipeline_prelude (3 步)
          ↓ encode_for_kind (1 步)
          ↓ send_and_decode (1 步 HTTP)
  ↓ NormalizedResponse
  ↓ protocol_handlers::*_from_normalized(&resp)
  ↓ 协议原生 JSON
```

**B2-B3-B4 三个 hook 点的精确位置**:

| Hook | 文件:行 | 类型 | 备注 |
| --- | --- | --- | --- |
| cache.get | `protocol_handlers.rs::dispatch` 入口前 | wrap async fn | req.stream == true 跳过 |
| cache.put | `protocol_handlers.rs::dispatch` 出口后 | wrap async fn | 成功才 put |
| retry | `protocol_handlers.rs::dispatch` 外层 | wrap async fn | 4xx 立刻返, 5xx / network 走退避 |
| X-Apeireth-Protocol header | `server.rs:184-298` 4 个 handler | parse header | 4 个 handler 都加 |
| X-Apeireth-Force-Cache header | 同上 | parse header | bypass dispatch, 直接用 cache |
| 关键路径 span | `server.rs:184-298` + `v2_endpoints` | tracing Span | `/v1/*` + `/council/*` + `/verdict` |

---

## 5. 关键约束 (R119 硬约束核验)

| 约束 | 当前状态 | B2-B4 怎么做 |
| --- | --- | --- |
| 0 改 workspace.version 1.1.0 | 0 触碰 | 不动 `Cargo.toml:246` |
| 0 触碰 24 LOCKED (cognition/core/sovereignty/formal) | 0 触碰 | B2-B4 只改 `apeireth-api/src/{cache,retry}.rs` (新) + `protocol_handlers.rs` + `server.rs` + `lib.rs` + `Cargo.toml` |
| 0 改 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱 / 9 器官 | 0 触碰 | 不动 |
| 0 改 R11 baseline 3 值 (V1141 / V1131 / V1136) | 0 触碰 | 不动 |
| 0 主动 commit | 0 commit | 主人 02:55 离场, 10:00 验收 |

---

## 6. B2-B4 实施优先级

| 阶段 | 时间 | 改什么 | 风险 |
| --- | --- | --- | --- |
| **B2** Response cache | 1.5-3.5h | 新建 `cache.rs` (hook NormalizedRequest → NormalizedResponse) + dispatch wrap | **低** (新文件, wrap 现有 fn) |
| **B3** Retry 多层退避 | 3.5-5h | 新建 `retry.rs` (BackoffPolicy enum) + dispatch wrap | **中** (改 dispatch 行为) |
| **B4** 协议路由 + tracing | 5-6.5h | 改 server.rs 4 handler + key path span | **中** (改公开 API) |

---

## 7. 决策点 (Mavis 拍板, 主 02:55 离场)

| 决策 | 选 | 理由 |
| --- | --- | --- |
| Cache 策略 | **LRU** | 跟现有 `apeireth-cache` 1:1 翻译, 默认 `EvictionPolicy::Lru`, 主人 S-1 经验路径 |
| 退避默认 | **Patient (1s/3s/10s/30s/2m/10m)** | 主人 6 锚 S-1 服务 ASI 北极星 — 可靠 > 快, 流式 + 关键路径更稳 |
| Cache key hash | **BLAKE3** | workspace 已有 `apeireth-telemetry`, 轻量, 快, 不引 sha2 (已经引了 sha2 给 auth, 不重复) |
| 缓存范围 | **仅非流式, 全部 4 协议** | 流式边界 case 留给 B5, 任务要求 |
| Cache TTL | **60s 默认** | 跟 `apeireth-cache::DEFAULT_TTL_SECS = 60` 1:1 |
| Cache max_size | **1024 items** | 跟 `apeireth-cache::DEFAULT_MAX_SIZE = 1024` 1:1 |
| Cache shards | **32** | 跟 `apeireth-cache::DEFAULT_SHARDS = 32` 1:1 |
| Header 协议切换 | `X-Apeireth-Protocol: openai\|anthropic\|gemini` | 任务要求, 默认 OpenAI (4 协议主入口) |
| Header 强制缓存 | `X-Apeireth-Force-Cache: true` | 任务要求, debug 用 |
| Tracing span 关键路径 | `/v1/chat/completions`, `/v1/responses`, `/v1/messages`, `/v1beta/.../generateContent`, `/council/advise`, `/verdict` | 任务要求 |

---

## 8. 文件清单 (B2-B4 计划改/新建)

**新建**:
- `crates/apeireth-api/src/cache.rs` — Response replay cache (B2)
- `crates/apeireth-api/src/retry.rs` — 多层退避 (B3)
- `reports/agent-b-b2-2026-08-10.md` — 阶段报告
- `reports/agent-b-b3-2026-08-10.md`
- `reports/agent-b-b4-2026-08-10.md`
- `reports/agent-b-final-2026-08-10.md`
- `reports/decision-log-2026-08-10.md` (主人 10 项偏好 #10 决策日志)

**改**:
- `crates/apeireth-api/Cargo.toml` — 加 `apeireth-cache` + `apeireth-telemetry` (新 deps)
- `crates/apeireth-api/src/lib.rs` — 加 `pub mod cache; pub mod retry;`
- `crates/apeireth-api/src/protocol_handlers.rs` — `dispatch` 加 cache + retry wrap
- `crates/apeireth-api/src/server.rs` — 4 handler 加 header parse + tracing span

**不碰**:
- 任何 24 LOCKED crate (cognition/core/sovereignty/formal/council/asi/...)
- `apeireth-telemetry/`, `apeireth-cache/`, `apeireth-protocol/`, `apeireth-pipeline/`, `apeireth-http-client/` 任何文件
- `apeireth-api/src/{v2_endpoints,auth,endpoints,audit_sqlite,ws_v1,v1_tools,llm,v2_routes,observability}.rs`
- workspace `Cargo.toml`

---

## 9. 验收硬指标 (Mavis 拍板核验)

- `cargo check -p apeireth-api --lib --tests --examples` exit 0
- `cargo check -p apeireth-protocol --lib --tests` exit 0
- `cargo check -p apeireth-cache --lib --tests` exit 0
- `cargo test -p apeireth-api` 0 failed (新增 ≥ 60 tests 累计)
- `cargo metadata` 能解析
- 0 改 `Cargo.toml:246` (workspace.version)
- 0 触碰 24 LOCKED

---

## 10. 风险 / 阻塞预案

| 风险 | 应对 |
| --- | --- |
| `apeireth-telemetry` 1.1 umbrella 编译失败 | 0 触碰 telemetry 源码, 只 import 公开 API; 失败立刻写 blocked-2026-08-10.md |
| `apeireth-cache::MemoryCache` 容量超限 (task spec 简化: 不做真 eviction loop) | 选 max_size 1024 + TTL 60s, 容量超限返 CapacityExceeded → 不入 cache (fail-soft) |
| 流式 (req.stream) 误入 cache | 在 `dispatch` 入口加 `if req.stream { skip cache }` 显式守门 |
| 4xx 重试雪崩 | 在 `BackoffPolicy::should_retry(status)` 显式: 4xx (除 408/425/429) 不重试 |
| 4 协议主路径行为变更 (1.0 已验收) | cache 命中返旧 response, 跟原行为 0 偏差; retry 失败 fallback 走原路径, 0 行为变更 |

---

**B1 完成时间**: 2026-08-10 04:30 (估)
**Mavis 状态**: 主人 02:55 离场, 我按主人 10 项偏好 #10 自主决策, 每决策写 decision-log-2026-08-10.md
**下一阶段**: B2 Response cache 实施, 起手 `cache.rs` 新建 + 改 Cargo.toml
