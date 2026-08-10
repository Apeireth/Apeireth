# R122-1-retry readmap — Response Replay Cache (VCP 借鉴, retry 版)

**时间**: 2026-08-10 14:17 (启动)
**项目**: `.openclaw\workspace\promethean\Apeireth-rust`
**借鉴 ID**: `R122-1-retry-VCP-ResponseReplayCache-2026-08-10`
**目标**: 新建 `crates/apeireth-api/src/replay_cache.rs` + 集成到 `dispatch_inner`
**VCP 借鉴源**: `research/source/vcptoolbox/modules/chatCompletionHandler.js:56-124` `class ResponseReplayCache`

---

## 0. 跟原 R122-1 readmap 的关键差异 (retry 原因)

| # | 原 R122-1 计划 (Connection error 失败) | retry 新计划 (本文件) |
|---|----------------------------------------|------------------------|
| 1 | Hook `send_and_decode_with_status` (HTTP 出口) | **Hook `dispatch_inner`** (任务 spec 明确要求) |
| 2 | 跟 B2 cache (R120 ResponseCache) 同 API surface (cache_key 风格) | **独立 module**, 不同 hash 策略, 跟 B2 共存 2 层 cache |
| 3 | 10 unit tests (含并发) | **7+ unit tests** (per 任务 spec 明列) |
| 4 | 0 改 server.rs + 0 改 dispatch 签名 | **0 改 server.rs + 0 改 dispatch 签名** (跟 R121-retry 严守) |

**retry 根因**: Connection error (CI 编译环境网络断开, 跟代码无关). retry 0 改任务 spec, 0 改 8 墙, 只重做实施.

---

## 1. VCP 借鉴源字段级分析

### 1.1 VCP `class ResponseReplayCache` (chatCompletionHandler.js:56-124)

```javascript
class ResponseReplayCache {
  constructor({ enabled = false, maxEntries = 100, debugMode = false } = {}) {
    this.enabled = enabled;
    this.maxEntries = ...;
    this.cache = new Map();  // <-- Rust 翻译成 Arc<RwLock<HashMap>>
  }

  get(key) {
    if (!this.enabled || !this.cache.has(key)) return null;
    const value = this.cache.get(key);
    this.cache.delete(key);     // <-- JS Map 保序, delete+set 实现 LRU
    this.cache.set(key, value); // <-- Rust HashMap 无序, LRU 由 evict_lru 独立管
    return value;
  }

  set(key, entry) {
    if (!this.enabled || !entry || !entry.chunks) return;
    if (this.cache.has(key)) this.cache.delete(key);
    this.cache.set(key, { ...entry, cachedAt: Date.now() });
    while (this.cache.size > this.maxEntries) {  // <-- Rust record() 内联
      const oldestKey = this.cache.keys().next().value;
      this.cache.delete(oldestKey);
    }
  }
}
```

### 1.2 VCP 字段 → Rust port 映射

| VCP 字段 | Rust 字段 | 借鉴/简化决策 |
|----------|-----------|---------------|
| `class ResponseReplayCache` | `pub struct ResponseReplayCache` | 1:1 |
| `this.cache = new Map()` | `Arc<RwLock<HashMap<String, ReplayEntry>>>` | 1:1 升级 (Rust 加锁) |
| `this.maxEntries = 100` | `max_entries: usize` (Default = 1000) | 1:1 字段 + 升级 default |
| `this.enabled = false` | (无, 走 global singleton) | 简化 (1.0 行为 0 漂移) |
| `get(key)` 返 value | `lookup(hash) -> Option<ReplayEntry>` | 1:1 |
| `set(key, entry)` 自动 evict | `record(hash, payload) -> Result<()>` 内 evict 1 oldest | 1:1 |
| VCP 0 TTL (JS Map 无 TTL) | `evict_expired(now) -> usize` + 1h Default TTL | 升级 (Rust 加) |
| `cachedAt: Date.now()` | `created_at: SystemTime` | 1:1 |
| VCP LRU `delete + set` | `evict_lru(max) -> usize` 独立函数 | 1:1 升级 (显式 API) |
| (VCP 无 hash 函数, key 是 `${ip}::${msgId}`) | `hash_request(method, url, body) -> String` (SHA-256 hex) | 升级 (任务 spec 明确要求) |
| `installResponseCacheRecorder` (line 126-179) | (0 移植) | 简化 (VCP 在 Express middleware 装 chunk recorder, Rust 在 dispatch 层 hook NormalizedResponse 即可) |

---

## 2. 目标 crate 状态

### 2.1 `apeireth-api` 现状

- **版本**: `1.1.0` (workspace 继承) — **0 改** ✓
- **lib.rs 现有 12 mod**:
  ```rust
  pub mod llm;
  pub mod protocol_handlers;
  pub mod server;
  pub mod cache;       // R120 B2: ResponseCache (apeireth-cache MemoryCache, NormalizedRequest hash)
  pub mod retry;       // R120 B3
  pub mod routing;     // R120 B4
  pub mod v2_endpoints;
  pub mod audit_sqlite;
  pub mod v2_routes;
  pub mod observability;
  pub mod endpoints;
  pub mod v1_tools;
  pub mod auth;
  pub mod ws_v1;
  ```
- **Cargo.toml 已有 dep** (per line 35, 38, 50, 51, 61):
  - `apeireth-cache` (R120 B2)
  - `apeireth-telemetry` (R120 B2/B3/B4)
  - `sha2 = "0.10"` (R20 阶段 2, 已有 ✓)
  - `once_cell = "1.20"` (R20 阶段 2, 已有 ✓)
  - `parking_lot = "0.12"` (R20 阶段 2, 已有 ✓)
  - `serde_json` (workspace 继承 ✓)
  - `chrono` (workspace 继承 ✓)
- **0 新 dep 需求** ✓ — 复用 sha2 / once_cell / parking_lot / serde_json / chrono

### 2.2 `dispatch_inner` 现状 (per `protocol_handlers.rs:982-991`)

```rust
fn dispatch_inner(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NormalizedResponse, String>> + Send + '_>> {
    Box::pin(async move {
        let (_status, result) = dispatch_inner_with_status(pipeline, kind, input).await;
        result
    })
}
```

**集成点 (in-function)**:
- 函数体内, async block 之前: 计算 `cache_key` (hash_request), `cache_hit` (lookup)
- async block 内, 开头: 命中返 cached
- async block 内, 末尾: 成功后 `cache.record`

**0 改 fn 签名** ✓ — 仅函数体内加 ~25 行, 签名 `fn(pipeline, kind, input) -> Pin<Box<...>>` 1:1 保留

### 2.3 B2 `ResponseCache` 共存关系 (per `cache.rs:147-208`)

- **B2 cache** (R120): `apeireth-cache::MemoryCache`, key = `cache_key(req, kind)` SHA-256 of NormalizedRequest 字段, 在 `dispatch_cached_with_status` (line 844-873) hook
- **B5 replay cache** (R122-1-retry): `Arc<RwLock<HashMap>>`, key = `hash_request(method, url, body)` SHA-256, 在 `dispatch_inner` hook
- **2 层 cache 共存**: 同请求 B2 miss → 调 dispatch_inner → B5 miss → 调 dispatch_inner_with_status. 2 cache key 不同 (B2 字段级, B5 raw HTTP level), 互不替代

---

## 3. 目标文件清单 (新建 + 最小改 LOCKED)

| 文件 | 类型 | 行数估算 | 内容 |
|------|------|---------|------|
| `crates/apeireth-api/src/replay_cache.rs` | **新建** | ~280 | ResponseReplayCache + ReplayEntry + ResponsePayload + ReplayStats + ReplayError + hash_request + global() + 7 tests |
| `crates/apeireth-api/src/lib.rs` | 改 +1 行 | +1 | 加 `pub mod replay_cache;` (mod 声明段) |
| `crates/apeireth-api/src/protocol_handlers.rs` | 改 ~30 行 | +30 | `dispatch_inner` 内加 cache fast-path + record |

**0 改**:
- workspace.version (1.1.0)
- R11 baseline 3 值 (integration_r_measure.rs)
- 24 LOCKED crate mtime
- 9 器官 logic
- 6 哲学锚 / 12 键 / 5 重守门 / V0.5 24 维 / 双洋葱
- 11 agent 公共 API 签名
- `server.rs` (per 任务 hard-constraint)
- `dispatch` / `dispatch_cached` / `dispatch_cached_with_status` / `dispatch_with_retry` 任何 fn 签名
- `cache.rs` (R120 B2 0 触碰, 共存)

---

## 4. `replay_cache.rs` 设计 (~280 行)

### 4.1 类型 (per 任务 spec)

```rust
pub const DEFAULT_MAX_ENTRIES: usize = 1000;
pub const DEFAULT_TTL: Duration = Duration::from_secs(3600);
pub const DEFAULT_HTTP_METHOD: &str = "POST";

pub struct ResponseReplayCache {
    entries: Arc<RwLock<HashMap<String, ReplayEntry>>>,
    max_entries: usize,
    ttl: Duration,
    stats: Arc<RwLock<ReplayStats>>,
}

pub struct ReplayEntry {
    pub request_hash: String,
    pub response: ResponsePayload,
    pub created_at: SystemTime,
    pub hit_count: u32,
}

pub struct ResponsePayload {
    pub body: serde_json::Value,    // 完整 NormalizedResponse JSON
    pub content: String,            // debug / log
    pub model: String,              // debug / log
    pub created_at_secs: i64,
    pub status: u16,                // 0 for non-HTTP / cache-only
}

pub struct ReplayStats { pub hits: u64, pub misses: u64, pub evictions: u64 }

pub enum ReplayError { LockPoisoned, Deserialize(String), Other(String) }
```

### 4.2 方法 (per 任务 spec 7 项)

```rust
impl ResponseReplayCache {
    pub fn new(max_entries: usize, ttl: Duration) -> Self;
    pub fn record(&self, hash: String, response: ResponsePayload) -> Result<(), ReplayError>;
    pub fn lookup(&self, hash: &str) -> Option<ReplayEntry>;
    pub fn evict_expired(&self, now: SystemTime) -> usize;
    pub fn evict_lru(&self, max: usize) -> usize;
    pub fn stats(&self) -> ReplayStats;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn max_entries(&self) -> usize;
    pub fn ttl(&self) -> Duration;
}

impl Default for ResponseReplayCache {
    fn default() -> Self { Self::new(1000, Duration::from_secs(3600)) }
}

pub fn hash_request(method: &str, url: &str, body: &[u8]) -> String;  // SHA-256 hex

static GLOBAL: OnceLock<Arc<ResponseReplayCache>> = OnceLock::new();
pub fn global() -> Arc<ResponseReplayCache> {
    GLOBAL.get_or_init(|| Arc::new(ResponseReplayCache::default())).clone()
}
```

### 4.3 7 unit tests (per 任务 spec 明列)

1. `record_then_lookup` — record 1 entry, lookup 返 Some, hit_count=1
2. `miss_returns_none` — lookup 不存在 hash 返 None, stats.misses=1
3. `evict_expired` — TTL=100ms, record 2 entries, sleep 200ms, evict_expired 返 2, len=0
4. `evict_lru_when_over_capacity` — max=2, record 3, len=2, oldest 1 被 evict
5. `stats_tracks_hits_and_misses` — 3 misses + 1 record + 2 hits, stats 完全对
6. `hash_request_is_deterministic` — 同输入 2 次 hash 相等, 长度=64 (SHA-256 hex)
7. `hash_request_different_for_different_input` — method/url/body 任一不同, hash 不同

**+3 bonus tests** (保险 ≥ 7):
8. `default_config_is_1000_entries_1h_ttl` — Default 字段核验
9. `lookup_expired_entry_treated_as_miss` — lazy eviction 行为
10. `global_singleton_returns_same_arc` — global() 同 Arc

---

## 5. `protocol_handlers.rs` 集成 (0 改签名)

```rust
fn dispatch_inner(
    pipeline: &Pipeline,
    kind: ProtocolKind,
    input: NormalizedRequest,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<NormalizedResponse, String>> + Send + '_>> {
    use crate::replay_cache::{global as replay_cache, hash_request, ResponsePayload, ReplayEntry, DEFAULT_HTTP_METHOD};
    let cache = replay_cache();
    let is_stream = input.stream;

    // Stream bypass: 0 cache (跟 R120 B2 1:1)
    if is_stream {
        return Box::pin(async move {
            let (_status, result) = dispatch_inner_with_status(pipeline, kind, input).await;
            result
        });
    }

    // Compute cache key (method + url + body)
    let url = match kind {
        ProtocolKind::Gemini => endpoint_url(&pipeline.config().base_url, kind, &input.model).ok(),
        _ => endpoint_url(&pipeline.config().base_url, kind, "").ok(),
    };
    let body_bytes = serde_json::to_vec(&input).ok();
    let cache_key = match (url, body_bytes) {
        (Some(u), Some(b)) => Some(hash_request(DEFAULT_HTTP_METHOD, &u, &b)),
        _ => None,
    };

    // Lookup (fail-soft: lock poisoned → None)
    let cache_hit: Option<ReplayEntry> = cache_key
        .as_ref()
        .and_then(|k| cache.lookup(k));

    Box::pin(async move {
        // Fast path: hit → deserialize + return
        if let Some(entry) = cache_hit {
            if let Ok(resp) = entry.response.to_response() {
                return Ok(resp);
            }
            // 反序列化失败 → 走原路径 (cache invalid)
        }

        // Slow path: pipeline 5 步
        let (_status, result) = dispatch_inner_with_status(pipeline, kind, input).await;

        // Record on success (fail-soft: lock error 静默)
        if let (Ok(resp), Some(k)) = (&result, &cache_key) {
            let payload = ResponsePayload::from_response(resp, 200);
            let _ = cache.record(k.clone(), payload);
        }

        result
    })
}
```

**关键不变量**:
- 0 改 `fn dispatch_inner(...)` 签名 (参数, 返回类型) ✓
- 流式 (stream=true) 完全 bypass cache (跟 B2 1:1) ✓
- 反序列化失败 fall-through (cache invalid → recompute) ✓
- record 失败静默 (fail-soft, 跟 B2 put 1:1) ✓
- 1.0 行为 (无 cache 命中) 1:1 保留 ✓

---

## 6. 8 墙硬核验 (实施前)

| # | 墙 | 状态 | 证据 |
|---|----|------|------|
| 1 | 0 改 workspace.version (1.1.0) | ✓ | 0 动 workspace Cargo.toml |
| 2 | 0 改 R11 baseline 3 值 | ✓ | 0 动 integration_r_measure.rs |
| 3 | 0 触碰 24 LOCKED | ✓ | 0 动其他 crate, 只 apeireth-api 内部 |
| 4 | 0 触碰 9 器官 logic | ✓ | 9 器官在 apeireth-memory/cognition 等, 0 触碰 |
| 5 | 0 改 11 agent 公共 API 签名 | ✓ | 0 改 apeireth-agent 任何 pub fn, 只加新 mod |
| 6 | 0 主动 commit | ✓ | 改动 working tree 等主 review |
| 7 | 0 装 (O-5) | ✓ | sha2/once_cell/parking_lot/serde_json/chrono 已在 dep |
| 8 | 0 范围扩散 | ✓ | 0 改 server.rs, 0 改 dispatch 任何 fn 签名 |

---

## 7. 实施节奏 (58 min, 严守)

| 段 | 时间 | 内容 | 用时 |
|----|------|------|------|
| Readmap | 14:17-14:25 (现在 ✓) | 本文件 | 8 min |
| 实施 replay_cache.rs | 14:25-14:50 | 新建 + 7+ tests + 编译期核验 | 25 min |
| 集成 protocol_handlers.rs | 14:50-15:00 | dispatch_inner hook + lib.rs +1 行 | 10 min |
| Verify | 15:00-15:10 | cargo build + cargo test (2 命令) | 10 min |
| Final + Decision Log | 15:10-15:15 | 2 报告 | 5 min |

**截止**: 15:15 (1h 启动时间)

---

## 8. 验收硬指标 (DoD)

- [x] Readmap 报告 (本文件)
- [ ] `replay_cache.rs` ~280 行, VCP `class ResponseReplayCache` 字段 1:1 翻译
- [ ] 7+ unit tests in `mod replay_cache_tests` (10+ 实际)
- [ ] 集成到 `dispatch_inner` (0 改 fn 签名, 内部加 ~25 行)
- [ ] `lib.rs` 加 `pub mod replay_cache;` (1 行)
- [ ] `cargo build -p apeireth-api` 0 error
- [ ] `cargo test -p apeireth-api --lib replay_cache_tests` 7+ passed, 0 failed
- [ ] `cargo test -p apeireth-api --lib` 全过 (原 30+ tests + 10 新 tests)
- [ ] 0 触碰 24 LOCKED, 0 改 workspace.version, 0 改 R11 baseline
- [ ] Final + Decision Log 2 报告

---

**R122-1-retry readmap 完成, 14:25 等实施. Mavis 待 review.**
