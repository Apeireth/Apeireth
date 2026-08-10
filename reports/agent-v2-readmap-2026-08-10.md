# Agent V2.0-续 Readmap — B 留 5 项 + Pre-existing 2 错误 (2026-08-10)

**时间**: 2026-08-10 04:10 (主人 02:55 离场 → 10:00 验收, 4h 窗口)
**作者**: 团队成员 V2.0-续 (Mavis 派, 工程化战区, 主人授权自主决策)
**状态**: V2-1 阶段完成, 5 项 + 2 错具体形状已列, 衔接 10 agent 0 冲突

---

## §0. TL;DR

10 个 agent 已 succeeded (A / A-2 / A-3 / B / B-2 / C / D-1 / D-2 / D-3 + Mavis 修 1 compile error).
B final report §5 留 5 项 + 1 项被 B-2 覆盖 (latency P99) + 1 项 (dispatch_with_retry 接入 server.rs) 留 R121.
我负责的工程化战区任务 (5 项 + 2 错) 全部具体形状已列, 衔接 10 agent 0 冲突.

| # | 任务 | 来源 | 状态 |
|---|---|---|---|
| 1 | 流式 SSE cache 边界 (4 协议流式分支) | B final §5.1 | V2-2 优先 |
| 2 | W3C traceparent 传播 | B final §5.6 | V2-3 |
| 3 | Redis cache backend stub | B final §5.2 | V2-4 |
| 4 | cache 容量超限 eviction (5 policy) | B final §5.4 | V2-4 |
| 5 | retry jitter (AWS SDK retry pattern) | B final §5.5 | V2-4 |
| 6 | workspace_e2e 1 failed 修 | C final §3 | V2-5 |
| 7 | tui bench 8 errors 修 | C final §3 + overnight §6 | V2-6 |

---

## §1. B 留 5 项具体形状 (B final §5 + read)

### 1.1 流式 SSE cache 边界 (中)

**B final §5.1 标缺**:
> 流式 (SSE) 不缓存 (边界 case 留给 B5). 我做了 bypass (req.stream == true → skip cache), 但流式 4 协议 (OpenAI Responses / Anthropic / Gemini) 还**没**走 dispatch 流式分支. 1.0 行为是流式走 `stream_chat_completions_forward` 0 走 cache. R121 续

**实际读 server.rs**:
- `chat_completions` (line 196-279) ✅ 实现流式分支 (req.stream → `stream_chat_completions_forward` line 290-331)
- `responses` (line 332-382) ❌ 流式不识别, 走 `Json<...>` 反序列化
- `messages` (line 384-434) ❌ 同上
- `generate_content` (line 437-489) ❌ 同上

**B 留的具体形状**: 4 协议流式分支要 1:1 镜像 `stream_chat_completions_forward`, 但用各自 endpoint URL.
- OpenAI Chat: `POST {base}/v1/chat/completions`
- OpenAI Responses: `POST {base}/v1/responses`
- Anthropic Messages: `POST {base}/v1/messages` (minimaxi Anthropic quirk: 加 `/anthropic` 前缀, per protocol_handlers.rs:173)
- Gemini: `POST {base}/v1beta/models/{model}:generateContent`

**0 漂移约束**:
- 流式 → 0 走 cache (req.stream skip)
- 流式 → 0 走 dispatch_cached
- 流式 → 0 改 NormalizedRequest 字段
- 流式 → 0 触碰 B 写的 `dispatch_cached` / `dispatch_with_retry` 公共 API 签名

**实现计划 (V2-2)**:
1. 抽 `stream_forward(pipeline, kind, raw_body) -> Response` (跟 `stream_chat_completions_forward` 同形, kind 决定 endpoint URL)
2. 4 handler 在 `req.stream == true` 时改调 `stream_forward`
3. 加 8 unit test (4 协议 × stream on/off, 0 真发请求, 验 endpoint 选对)
4. 0 触碰 cache / retry / routing 公共 API

### 1.2 W3C traceparent 传播 (中)

**B final §5.6 标缺**:
> 当前 span trace_id 每次 new (1 request 1 trace), 没解析 HTTP `traceparent` header. 跨服务 trace 关联需要 W3C 解析. R121 续 (跟 `apeireth-telemetry::trace::propagation::W3CTraceContextPropagator` 1:1 集成)

**实际读 apeireth-telemetry/src/trace/propagation.rs**:
- ✅ `W3CTraceContextPropagator` 已完整实现 (line 99-156)
- ✅ `parse_traceparent` 函数 (line 161-206) 解析 `00-<32hex>-<16hex>-<2hex>` 格式
- ✅ `parse_kv_list` (line 209-221) 解析 tracestate/baggage
- ✅ `is_valid_trace_id` / `is_valid_span_id` K-1 强校验
- ⚠️ 但接口是 `HashMap<String, String>`, 不是 `http::HeaderMap`, 需要薄 wrapper

**实际读 routing.rs**:
- `KeyPathSpan::start(name)` (line 127-136) 总是 `generate_trace_id()` (新 ID)
- 没解析入站 `traceparent` header

**0 漂移约束**:
- 0 改 telemetry propagation API 签名
- 0 改 B 写 routing 公共 API
- 0 触碰 24 LOCKED (apeireth-telemetry 在观测层, 不在 24 LOCKED 名单)

**实现计划 (V2-3)**:
1. routing.rs 加 `parse_traceparent_from_headers(headers: &HeaderMap) -> Option<TraceContext>` (用 HashMap 桥 + W3C propagator extract)
2. `KeyPathSpan::start(name, parent_ctx: Option<TraceContext>)` — 接受 parent, 跟 现有 `start(name)` 兼容 (加 Option 参数是向后兼容的扩展)
3. server.rs 4 handler 在 start span 时调 parse_traceparent_from_headers
4. 加 6 unit test (1 完整 round-trip + 5 边界: 无 header / 错格式 / 大写 / 太短 / 0 采样)
5. 0 改 telemetry propagation, 0 改 cache / retry / dispatch

### 1.3 Redis / Memcached cache backend stub (中)

**B final §5.2 标缺**:
> `apeireth-cache` 当前仅 Memory backend 真接, Disk / Redis / Memcached 返 `BackendNotImplemented`. 跨 daemon 部署需要 Redis. R21+ 续 (任务 spec 明确 R21 续)

**实际读 apeireth-cache**:
- ✅ `BackendKind` 4 variant 编译期 hardcode (Memory/Disk/Redis/Memcached, line 39-51)
- ✅ `StubCache` (line 381-396) 已存在, 4 backend 走它都返 `BackendNotImplemented`
- ✅ `MemoryCache` 完整实现 (line 215-310)
- ❌ Disk/Redis/Memcached 3 backend 0 真接

**Redis 已检查**: workspace Cargo.lock 已有 `redis v0.27.6` 依赖 (在 apeireth-telemetry 旁). 但 apeireth-cache/Cargo.toml 还没引 redis.

**0 漂移约束**:
- 0 改 BackendKind enum (4 variant 编译期 hardcode 严守)
- 0 改 MemoryCache 现有 32 分片锁
- 0 改 apeireth-cache 公共 API
- 0 触碰 24 LOCKED (apeireth-cache 在观测层, 不在 24 LOCKED 名单)

**实现计划 (V2-4 部分)**:
1. 加 `apeireth-cache/Cargo.toml` `[dependencies] redis = "0.27"` (复用 workspace 已锁版本)
2. 新建 `crates/apeireth-cache/src/redis_backend.rs` (RedisCache struct + Cache trait impl)
3. 公开契约: 跟 `MemoryCache<K, V>` 同形 (用 redis::Client + connection pool + GET/SET/DEL/EXPIRE)
4. `build_cache` 函子 (lib.rs:437-453) 在 `BackendKind::Redis` 分支加 RedisCache 构造
5. 加 5 unit test (基础 get/put/delete + 连接失败返 CacheError + TTL 走 EXPIRE)
6. ⚠️ Redis 真连需要 redis-server 跑, 0 fixture 时返 BackendNotImplemented (跟现有 StubCache 模式一致), 加 #[ignore] 真连测试

**主决策** (per 任务 "Redis 真接 vs stub 自己定"): **真接 (但单测用 #[ignore] + fixture)**. 理由:
- redis = "0.27" 已 workspace 锁
- RedisCache 公开 API 1:1 镜像 MemoryCache, 接口层稳
- 真连测试用 #[ignore] 隔, 0 影响 CI (跟 R20 stub 模式 1:1)

### 1.4 cache 容量超限 eviction (低)

**B final §5.4 标缺**:
> `apeireth-cache` skeleton 阶段简化返 `CapacityExceeded` (fail-soft), R21 续真接 5 policy eviction loop

**实际读 apeireth-cache**:
- ✅ `EvictionPolicy` 5 variant 编译期 hardcode (Lru/Lfu/Fifo/Arc/TinyLfu, policy.rs:38-53)
- ❌ `MemoryCache` 当前 put 时超 max_size 返 `CacheError::CapacityExceeded` (fail-soft, 0 evict)
- ❌ 5 policy 只有 Lru 真接 (MemoryCache 内部用 LRU), Lfu/Fifo/Arc/TinyLfu 是 enum 但 0 实现

**0 漂移约束**:
- 0 改 EvictionPolicy enum (5 variant 编译期 hardcode 严守)
- 0 改 MemoryCache 公共 API (`new` / `get` / `put` / `len` / `clear` / `remove` / `stats`)
- 0 改 apeireth-cache/lib.rs 公共 API
- 0 触碰 24 LOCKED

**实现计划 (V2-4 部分)**:
1. 抽 `Evictor` trait (内部使用, 0 公开 API 改): `fn on_insert(&mut self, key: K) -> Option<K> /* 返被踢 key */`
2. 5 实现: `LruEvictor` (HashMap+VecDeque), `LfuEvictor` (HashMap+频次桶), `FifoEvictor` (HashMap+VecDeque), `ArcEvictor` (简化自适应, 2 LRU list 平衡), `TinyLfuEvictor` (sliding window + 频次近似)
3. `MemoryCache` 内部用 Evictor 替换现有 LRU 逻辑 (内部细节, 0 改公开)
4. `MemoryCache::with_policy(policy: EvictionPolicy)` 新增构造器 (1:1 跟现有 `new()` 并存)
5. 加 10 unit test (5 policy 各 2 个: kick 最少用 + 5 policy 都走 happy path)

### 1.5 retry jitter (低)

**B final §5.5 标缺**:
> 1.0 退避无 jitter, 多 daemon 同时退避可能"thundering herd". 1.0 行为 OK, R21+ 续 exponential jitter (AWS SDK retry pattern)

**实际读 retry.rs**:
- ✅ `BackoffPolicy` 4 variant (Aggressive/Default/Patient/Custom)
- ✅ `to_durations()` 返固定 Vec<Duration>
- ❌ 0 jitter 机制

**AWS SDK retry pattern 借鉴** (web search R1 / R2):
- Full Jitter: `sleep = random(0, base)` (最大 jitter, 防止 thundering herd)
- Equal Jitter: `sleep = base/2 + random(0, base/2)` (中等 jitter)
- Decorrelated Jitter: `sleep = min(cap, random(base, prev*3))` (AWS SDK 标准)

**0 漂移约束**:
- 0 改 BackoffPolicy 4 variant 公共 API
- 0 改 1.0 退避行为 (无 jitter 模式仍要能跑)
- 0 触碰 24 LOCKED (retry 在 apeireth-api 战区, 不在 24 LOCKED 名单)

**实现计划 (V2-4 部分)**:
1. retry.rs 加 `JitterMode` enum: `None` (默认, 0 漂移 1.0) / `Full` (AWS SDK full jitter) / `Equal` (equal jitter) / `Decorrelated` (decorrelated jitter)
2. `BackoffPolicy::with_jitter(mode: JitterMode)` 构造器 (向后兼容, 0 改 default)
3. `next_sleep_with_jitter(prev: Option<Duration>) -> Duration` 方法
4. `RetryStats` 加 1 Counter: `apeireth_api_retry_jittered_total` (走 jitter 重试次数)
5. 加 8 unit test (4 jitter mode 各 2 个: 返回 Duration 在范围内 + 多次调用分布)

### 1.6 (B 留但任务 spec 没列) dispatch_with_retry 接入 server.rs (低)

**B final §5.7 标缺**:
> 任务 spec 明确 B3 加 retry hook. 我**没**在 server.rs 4 handler 调 `dispatch_with_retry` (只调 `dispatch_cached`), 因为 B3 spec 写"在 protocol_handlers.rs 加 BackoffPolicy", 没要求 server.rs 集成. retry 在 `protocol_handlers.rs` 已就绪, 留 server.rs 接入.

**任务 spec 列的 5 项没这个**, 主人拍板"5 项中至少 3 项实现 (流式 SSE / W3C traceparent / cache eviction 必做, Redis / retry jitter 优先)". 我建议:
- **必做 3 项**: 流式 SSE / W3C traceparent / cache eviction
- **优先 2 项**: Redis / retry jitter
- **不接 server.rs 集成**: 跟 B 留的 1.0 行为 0 漂移原则一致, dispatch_with_retry 留 R121 续

---

## §2. Pre-existing 2 错误根因 (C final §3 + overnight §6)

### 2.1 workspace_e2e::tests::test_workspace_8_promises_audit_passes_runs

**C final §3 提的 (actually §3 标 "pre-existing 1 failed")**:
> `workspace_e2e::tests::test_workspace_8_promises_audit_passes_runs` (baseline 已有, 不是我引入)

**实际读 workspace_e2e.rs (line 169-186)**:
```rust
pub fn test_workspace_8_promises_audit_passes(workspace_root: &Path) -> E2EResult<()> {
    for f in EIGHT_PROMISES_SOURCE_FILES {  // 8 个文件
        let p = workspace_root.join(f);
        if !p.exists() {
            missing.push(f.to_string());
        }
    }
    ...
}
```

**8 个期望文件** (line 57-66):
1. `APEIRETH-COMPLETE-OMNIBUS-2026-07-30.md` ❌ 不存在
2. `APEIRETH-CONVENTIONS.md` ❌ 不存在
3. `APEIRETH-VERSIONING.md` ❌ 不存在
4. `GLOSSARY.md` ❌ 不存在
5. `docs/stage4/8-locked-unified-2026-08-05.md` ✅ 存在
6. `FINISH-CONSTRUCTION.md` ❌ 不存在
7. `START-CONSTRUCTION.md` ❌ 不存在
8. `START-HERE-FOR-CONSTRUCTION-LEADER.md` ❌ 不存在

**根因**: 测试逻辑假设的"8 项不修改承诺源头文件"是 R11 baseline 阶段产物, 当前 R11 文档已演化为 `docs/conventions/10-locked.md` + `docs/glossary/08-5-no-fake.md` + `docs/stage4/8-locked-unified-2026-08-05.md` 等. 测试 0 更新.

**主决策** (per 任务 "修测试逻辑 vs 删测试"): **修测试逻辑**, 0 删测试.

**修法 (V2-5)**:
把 `EIGHT_PROMISES_SOURCE_FILES` 改成实际存在的 8 个"8 项不修改承诺"承载文件:
1. `docs/conventions/10-locked.md` (1:1 替代 APEIRETH-CONVENTIONS.md)
2. `docs/conventions/09-anchor.md` (6 哲学锚)
3. `docs/conventions/11-baseline.md` (baseline 3 值)
4. `docs/glossary/08-5-no-fake.md` (5 不假装)
5. `docs/glossary/01-north-star.md` (S-1 北极星)
6. `docs/glossary/02-double-onion.md` (双洋葱)
7. `docs/glossary/15-9-phase-lifecycle.md` (9 器官)
8. `docs/stage4/8-locked-unified-2026-08-05.md` (8 项统一)

**核验**: 这 8 个文件全部实际存在 (verifiable by Test-Path), 测试 0 漂移"8 项不修改承诺" 概念, 1:1 改承载.

**0 触碰 24 LOCKED**: `apeireth-integration-e2e` 不在 24 LOCKED 名单 (24 LOCKED 是 cognition/core/sovereignty/formal/asi/memory/onion/bus/verify/extension/evolution/perception/motivation/supervisor/pybridge/config/naming-v05/cron/life-force/value/consciousness/relation/action), 我可以改.

### 2.2 apeireth-tui/benches/render_5_nav.rs (8 bench errors)

**C final §3 提的 + overnight §6**:
> `apeireth-tui/benches/render_5_nav.rs` (8 bench errors, binary crate 用 `crate::` 路径错, 跟我无关)

**实际读 render_5_nav.rs + app.rs + pages**:
- `[[bench]] name = "render_5_nav" harness = false` (Cargo.toml:80-82)
- bench 用 `#[path = "../src/app.rs"]` `#[path = "../src/theme.rs"]` `#[path = "../src/http_llm.rs"]` `#[path = "../src/backend.rs"]` `#[path = "../src/pages/mod.rs"]` 5 个 mod
- bench 用 `app::{App, NavPage}` + `pages::bridge::render(f, area, &app, &style)` 等

**潜在问题**:
1. `bench` 借 `#[path]` 引 `app.rs`, 但 `app.rs` 引用 `crate::theme::Theme` (line 15), bench 不引 `theme.rs` 作为 crate 路径, 但有 (line 22-23)
2. `pages/dialogue.rs:35: pub fn render(f: &mut Frame, area: Rect, app: &mut App, style: &ThemeStyle)` — bench 用 `black_box(&app)` (immutable) 但需要 &mut App ⚠️
3. `pages/mod.rs` 引 `crate::organ::*` 之类, bench 0 引 organ → 缺 mod

**根因分析 (需要跑 `cargo check -p apeireth-tui --benches` 验 8 errors)**:
- binary crate 模式 + `#[path]` 引 src/, 但 src 内的 `use crate::xxx` 跟 bench 模块路径不一致
- 0 改 src 严守 (24 LOCKED 严守, tui 不在 24 LOCKED 名单但属工程化战区)
- 主决策 (per 任务 "加 lib.rs vs 改 bench 路径"): **加 lib.rs** 跟其他 binary crate 1:1 (task hint 推荐)

**修法 (V2-6)**:
1. 加 `crates/apeireth-tui/src/lib.rs` (re-export 5 nav pages + App + NavPage + Theme + backend)
2. Cargo.toml: 加 `[lib] name = "apeireth-tui" path = "src/lib.rs"` 段
3. bench 改用 `use apeireth_tui::{App, NavPage, Theme, ThemeStyle, pages}` (跟其他 binary crate 1:1)
4. 移除 bench 内的 `#[path = ...]` 5 个 mod
5. 0 改 `src/main.rs` (binary 仍走 main.rs, lib 是公开 API 给 bench 用)

**0 触碰 24 LOCKED**: `apeireth-tui` 不在 24 LOCKED 名单, 我可以加 lib.rs.

---

## §3. 衔接 10 agent 已交付 (0 冲突核验)

| Agent | 公共 API 签名 | 我会触碰吗? | 0 冲突 |
|---|---|---|---|
| A (vector + memory) | `SqliteVecBackend`, `SemanticIndex`, `UserProfile`, `EmbedFn` | 0 (我战区 api/cache/tui) | ✅ |
| A-3 (PersistentSemanticIndex) | `PersistentSemanticIndex`, `open_persistent_semantic_index`, `semantic_search_persistent` | 0 | ✅ |
| A-2 (.github) | 3 ISSUE_TEMPLATE + 1 PR template | 0 | ✅ |
| B (cache + retry + routing) | `ResponseCache`, `BackoffPolicy`, `RetryStats`, `KeyPathSpan`, `parse_protocol_kind`, `extract_*` | **V2-2/3/4 触碰**(扩展, 0 改签名) | ✅ 验证 |
| B-2 (bench) | `swe_bench` / `agent_bench` / `self_disable_bench` / `latency_bench` | 0 | ✅ |
| C (9 product crate tests) | 9 tests files | 0 | ✅ |
| D-1 (CI workflow) | rustfmt.yml + rust.yml + rust-ci.yml 注释 | 0 | ✅ |
| D-2 (tool-registry classifier) | `Classifier` trait, 9 Category, 3 impl | 0 | ✅ |
| D-3 (council 4 模式) | `CollaborationMode`, 4 模式 + `RoleConstitution` + `TraceReport` | 0 | ✅ |
| Mavis (修 1 compile error) | apeireth-cli AppState.response_cache 字段加 None | 0 | ✅ |

**结论**: 我只触碰 B 留的 cache / retry / routing / server (向后兼容扩展, 0 改公共签名) + workspace_e2e 改 1 行 (EIGHT_PROMISES_SOURCE_FILES 8 个 file 名) + tui 加 lib.rs (新文件, 0 改 src).

---

## §4. 风险 + 决策

| # | 决策 | 选择 | 理由 |
|---|---|---|---|
| 1 | 流式 SSE 范围 | 4 协议全加 (B 留) | 主人 #3 不假装"做部分" |
| 2 | W3C 传播 | 用现有 `W3CTraceContextPropagator` (薄 wrapper) | 0 重写 telemetry, 主人 #6 不重复造轮子 |
| 3 | Redis backend | 真接 (单测 #[ignore] + fixture) | redis = "0.27" 已锁, 公开 API 1:1 镜像 |
| 4 | 5 policy eviction | 5 真接 (LRU 真改 + LFU/FIFO/ARC/TinyLFU 4 新增) | EvictionPolicy enum 严守 5 variant |
| 5 | retry jitter | 4 mode (None / Full / Equal / Decorrelated), None 默认 | 0 漂移 1.0 行为 |
| 6 | workspace_e2e 修 | 改 EIGHT_PROMISES_SOURCE_FILES 8 个 file 名 | 主人 #3 不假装, 0 删测试 |
| 7 | tui bench 修 | 加 src/lib.rs (跟其他 binary crate 1:1) | task hint 推荐 |
| 8 | dispatch_with_retry server 集成 | **不做** (B 留 R121 续) | 5 项是任务 spec, 这项不在列 |

---

## §5. 0 假装 (per 主人偏好 #3 + #7)

- ⚠️ latency P99 数字 (B-2 实测 wiremock 0 网络) — 跟 B-2 1:1 0 重测
- ⚠️ 5 policy eviction 中 ARC 是简化版 (2 LRU list + 简化平衡算法) — 标"非完整 IBM ARC spec"
- ⚠️ TinyLFU sliding window 是简化版 (3 counter approximate, 0 真 Caffeine 1:1)
- ⚠️ RedisCache 真连测试用 #[ignore] (无 redis-server 跑不了), 0 假装"已真连"
- ✅ 0 写"production ready"
- ✅ 0 改 workspace.version (1.1.0)
- ✅ 0 触碰 24 LOCKED

---

## §6. 阶段总览 (4h 窗口)

| 阶段 | 时间 | 任务 | 状态 |
|---|---|---|---|
| V2-1 | 0-0.5h | readmap (本文件) | ✅ 04:10 |
| V2-2 | 0.5-1.5h | 流式 SSE 4 协议 | 待 |
| V2-3 | 1.5-2.5h | W3C traceparent 传播 | 待 |
| V2-4 | 2.5-3h | Redis stub + 5 policy eviction + retry jitter (3 小项) | 待 |
| V2-5 | 3-3.5h | workspace_e2e 1 failed 修 | 待 |
| V2-6 | 3.5-4h | tui bench 8 errors 修 + final report | 待 |
| 09:30 | 强制收尾 | 主人 10:00 验收 | — |

---

**V2-1 完. 等主人 10:00 验收. 衔接 10 agent 0 冲突, 5 项 + 2 错具体形状已列.**
