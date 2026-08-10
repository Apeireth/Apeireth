//! R122-1-retry: Response Replay Cache (VCP `chatCompletionHandler.js:73-156` 借鉴)
//!
//! **借鉴源**: `research/source/vcptoolbox/modules/chatCompletionHandler.js:56-124`
//!   `class ResponseReplayCache`
//! **借鉴 ID**: `R122-1-retry-VCP-ResponseReplayCache-2026-08-10`
//!
//! **目的**: 重复 `(method, url, body)` 三元组命中 fast path, 跳过上游 LLM 调用
//!
//! **VCP 字段级对应** (per 07 §1 O-2 走在前人经验上):
//! - VCP `class ResponseReplayCache` → Rust `pub struct ResponseReplayCache`
//! - VCP `this.cache = new Map()` → Rust `Arc<RwLock<HashMap<String, ReplayEntry>>>`
//! - VCP `this.maxEntries = 100` → Rust `max_entries: usize` (Default = 1000, 升级 10x)
//! - VCP `get(key) { delete + set }` (JS Map 保序即 LRU) → Rust `lookup(hash)`
//!   (HashMap 无序, LRU 显式 `evict_lru`)
//! - VCP `set(key, { ...entry, cachedAt: Date.now() })` → Rust `record(hash, payload)` (内 evict 1 oldest)
//! - VCP `while (size > maxEntries) { delete oldestKey }` → Rust `record()` 内 + `evict_lru()` 独立函数
//! - VCP 0 TTL (JS Map 不支持) → Rust `evict_expired(now)` + 1h Default TTL (升级)
//! - VCP `buildKey(clientIp, messageId)` → Rust `hash_request(method, url, body)` SHA-256 hex
//!
//! **VCP 0 装 (本实现不装的字段)**:
//! - ❌ `enabled: false` 全局开关 — 简化: 走 process-wide global singleton, 1.0 行为 0 漂移
//! - ❌ `debugMode: false` debug 日志 — 简化: 用 tracing / 0 装 console log
//! - ❌ `clientIp` per-IP key — 简化: 纯 (method, url, body) 三元组 hash
//! - ❌ `messageId` 客户端传 — 简化: 全 body hash, 客户端无感
//! - ❌ `streamMode` SSE chunk 录制 — 简化: 走 dispatch 层 fast-path, 流式 bypass
//! - ❌ `installResponseCacheRecorder` (line 126-179 Express middleware) — 0 移植 (VCP 装
//!   在 HTTP res.write/end 上录 chunk, Rust 在 dispatch 层 hook NormalizedResponse 即可)
//!
//! **架构位置**:
//! ```text
//!   客户端 4 协议请求
//!     ↓
//!   server.rs 4 endpoint
//!     ↓ protocol_handlers::dispatch_cached (B2: ResponseCache outer layer, R120)
//!       ├── cache.get → 命中 → 返
//!       └── miss → dispatch_inner (B5: ResponseReplayCache inner layer, NEW R122-1-retry)
//!                   ├── hash_request("POST", url, body) → SHA-256
//!                   ├── cache.lookup → 命中 → 返 NormalizedResponse (反序列化)
//!                   └── miss → dispatch_inner_with_status → 5 步管线 → cache.record
//! ```
//!
//! **设计原则**:
//! - **0 改 dispatch 签名** — 通过 process-wide global singleton 接入, 1.0 行为 0 漂移
//! - **fail-soft** — cache 内部错误 (LockPoisoned / 反序列化失败) 走原 dispatch 路径, 0 影响主路径
//! - **0 新 dep** — 复用 apeireth-api 已有 sha2 / std::sync / serde_json / chrono
//! - **独立于 B2 ResponseCache** — 不同 hash 策略 (B2: NormalizedRequest 字段级 / B5: raw HTTP level)
//! - **编译期 hardcode** — DEFAULT_MAX_ENTRIES / DEFAULT_TTL 编译期 const, 0 漂移
//!
//! **不假装** (主哲学锚 #1):
//! - ✅ 真用 SHA-256 hash (sha2 crate, 0 装 1:1)
//! - ✅ 真 RwLock 保护 HashMap (concurrency safe, 跟 VCP JS Map 语义不同但等价)
//! - ✅ 真 evict_lru / evict_expired (VCP 0 装这俩, 我加 for production 清理)
//! - ✅ 真 LRU 近似 = 按 created_at ASC 排序 (HashMap 无序, 0 假装 Java Map 严格 LRU)
//!
//! **决策日志**: `reports/agent-r122-1-retry-decision-log-2026-08-10.md`

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ============================================================
// 编译期常量 (平台不变性, 主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// Default max entries (per 任务 spec: 1000, 比 VCP `maxEntries = 100` 升级 10x)
pub const DEFAULT_MAX_ENTRIES: usize = 1000;

/// Default TTL: 1h (per 任务 spec: 1h, VCP 0 TTL 我加 for production)
pub const DEFAULT_TTL: Duration = Duration::from_secs(3600);

/// HTTP method 守门常量 (本 cache 仅 POST 复用, GET/HEAD 不入 cache)
pub const DEFAULT_HTTP_METHOD: &str = "POST";

/// SHA-256 hex 长度 (64 chars, debug_assert 守门)
const SHA256_HEX_LEN: usize = 64;

/// Cache key namespace prefix (debug / log 识别, 不参与 hash 本身)
pub const CACHE_KEY_NAMESPACE: &str = "apeireth-api:replay:";

// ============================================================
// 类型定义
// ============================================================

/// Cache entry (VCP `entry` 对象的 1:1 升级版, 加 metadata)
///
/// **VCP 字段级对应**:
/// - `request_hash` (无 VCP 对应, 我加 for 反查)
/// - `response` ↔ VCP `entry` (chunks + statusCode + headers)
/// - `created_at` ↔ VCP `cachedAt: Date.now()`
/// - `hit_count` (无 VCP 对应, 我加 for metrics)
#[derive(Clone, Debug)]
pub struct ReplayEntry {
    /// 请求 hash (lookup key, 0 装: 我加 for 反查)
    pub request_hash: String,
    /// 缓存的响应 (VCP `entry` 1:1 升级)
    pub response: ResponsePayload,
    /// 缓存时间 (VCP `cachedAt: Date.now()` 1:1 升级)
    pub created_at: SystemTime,
    /// 命中次数 (0 装: 我加 for metrics, lookup 命中后 +1)
    pub hit_count: u32,
}

/// Cached response payload (VCP `entry` 内部 `chunks[]` + `statusCode` 的 Rust 升级)
///
/// **VCP 字段级对应**:
/// - `body` ↔ VCP `chunks[]` (字节流, 我用序列化 Value for 反序列化方便)
/// - `content` (无 VCP 对应, 我加 for debug / log)
/// - `model` (无 VCP 对应, 我加 for debug / log)
/// - `created_at_secs` (无 VCP 对应, 我加 for serialization-friendly timestamp)
/// - `status` ↔ VCP `statusCode` (HTTP status, 0 for non-HTTP / cache-only)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponsePayload {
    /// 完整 NormalizedResponse JSON (供反序列化用, 1:1 serde roundtrip)
    pub body: serde_json::Value,
    /// 响应文本 (debug / log 用, 不参与 cache key)
    pub content: String,
    /// 模型名 (debug / log 用, 不参与 cache key)
    pub model: String,
    /// 缓存时的 Unix timestamp (debug / log 用)
    pub created_at_secs: i64,
    /// HTTP status (VCP `statusCode`, 默认 200 for cache hit)
    pub status: u16,
}

impl ResponsePayload {
    /// 从 NormalizedResponse 构造 (集成 helper, protocol_handlers 集成用)
    ///
    /// **不漂移**: `serde_json::to_value` 失败 → body 退化为 Null (fail-soft)
    pub fn from_response(
        resp: &apeireth_protocol::NormalizedResponse,
        status: u16,
    ) -> Self {
        Self {
            body: serde_json::to_value(resp).unwrap_or(serde_json::Value::Null),
            content: resp.content.clone(),
            model: resp.model.clone(),
            created_at_secs: chrono::Utc::now().timestamp(),
            status,
        }
    }

    /// 反序列化为 NormalizedResponse (集成 helper, 失败返 Err 让 caller 走原路径)
    pub fn to_response(&self) -> Result<apeireth_protocol::NormalizedResponse, serde_json::Error> {
        serde_json::from_value(self.body.clone())
    }
}

/// Cache stats (VCP 0 装, 我加 for metrics / observability)
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplayStats {
    /// 命中次数 (lookup 返 Some 时 +1)
    pub hits: u64,
    /// 未命中次数 (lookup 返 None 时 +1, 含过期 lazy eviction)
    pub misses: u64,
    /// 淘汰次数 (evict_expired / evict_lru / record 自动淘汰累加)
    pub evictions: u64,
}

/// ReplayError (`pub fn record(...) -> Result<()>` 的 Err 类型)
#[derive(Debug)]
pub enum ReplayError {
    /// RwLock poisoned (panic in another thread holding lock)
    LockPoisoned,
    /// 反序列化失败 (cache 内容损坏 / schema drift)
    Deserialize(String),
    /// 其他 (e.g. 容量为 0 等 invariant 违反, 0 主动构造)
    Other(String),
}

impl std::fmt::Display for ReplayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockPoisoned => write!(f, "replay cache lock poisoned"),
            Self::Deserialize(s) => write!(f, "replay cache deserialize: {s}"),
            Self::Other(s) => write!(f, "replay cache: {s}"),
        }
    }
}

impl std::error::Error for ReplayError {}

// ============================================================
// ResponseReplayCache
// ============================================================

/// VCP `class ResponseReplayCache` Rust 翻译版
///
/// **不漂移 VCP 1:1 行为**:
/// - 构造 (Default): 1000 entries / 1h TTL
/// - `record`: 自动 evict 1 oldest 当容量满
/// - `lookup`: 命中返 Some, miss 返 None, 过期 lazy evict
/// - `evict_expired` / `evict_lru`: 显式批量清理
pub struct ResponseReplayCache {
    entries: Arc<RwLock<HashMap<String, ReplayEntry>>>,
    max_entries: usize,
    ttl: Duration,
    stats: Arc<RwLock<ReplayStats>>,
}

impl ResponseReplayCache {
    /// 构造 (custom config)
    ///
    /// **不变量**:
    /// - `max_entries > 0` (0 假装: 0 容量 = 永远 evict = 死循环, 0 port)
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        debug_assert!(max_entries > 0, "max_entries must be > 0");
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
            ttl,
            stats: Arc::new(RwLock::new(ReplayStats::default())),
        }
    }

    /// 记录 (cache miss 后调)
    ///
    /// **行为** (VCP `set(key, entry)` 1:1):
    /// - `entries.len() >= max_entries` → 自动 evict 1 oldest (按 `created_at` ASC)
    /// - 插入新 entry (覆盖同 hash, VCP `cache.has(key) → delete` 1:1)
    /// - 成功 → `Ok(())`, 失败 (LockPoisoned) → `Err(ReplayError::LockPoisoned)`
    pub fn record(
        &self,
        request_hash: String,
        response: ResponsePayload,
    ) -> Result<(), ReplayError> {
        let mut entries = self
            .entries
            .write()
            .map_err(|_| ReplayError::LockPoisoned)?;
        let mut evicted_count: u64 = 0;

        // 容量守门: 满则 evict 1 oldest (VCP `while (size > maxEntries)` 1:1)
        if entries.len() >= self.max_entries {
            if let Some(oldest_key) = entries
                .iter()
                .min_by_key(|(_, e)| e.created_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
                evicted_count += 1;
            }
        }

        // 插入新 entry (覆盖同 hash, 跟 VCP `if (cache.has(key)) cache.delete(key)` 1:1)
        let entry = ReplayEntry {
            request_hash: request_hash.clone(),
            response,
            created_at: SystemTime::now(),
            hit_count: 0,
        };
        entries.insert(request_hash, entry);
        drop(entries); // 显式释放 entries 锁

        // 累加 evictions (独立 lock, 避免 entries + stats 死锁)
        if evicted_count > 0 {
            if let Ok(mut stats) = self.stats.write() {
                stats.evictions += evicted_count;
            }
        }

        Ok(())
    }

    /// 查询 (cache hit / miss 判断 + 过期 lazy evict)
    ///
    /// **行为** (VCP `get(key)` 升级版):
    /// - hash 不存在 → `None`, `stats.misses += 1`
    /// - hash 存在但 expired (created_at + TTL < now) → `None` (lazy eviction),
    ///   `stats.misses += 1`, `stats.evictions += 1`
    /// - hash 存在且未过期 → `Some(entry)`, `stats.hits += 1`, `entry.hit_count += 1`
    ///
    /// **fail-soft**: RwLock poisoned 当 miss (返 None + 累加 misses, 0 panic)
    pub fn lookup(&self, request_hash: &str) -> Option<ReplayEntry> {
        // Step 1: 读 lock 拿 entry + 判 expired
        let (entry_opt, expired) = {
            let Ok(entries) = self.entries.read() else {
                // fail-soft: lock poisoned 当 miss
                self.bump_misses();
                return None;
            };
            match entries.get(request_hash) {
                Some(e) => {
                    let now = SystemTime::now();
                    let expired = now
                        .duration_since(e.created_at)
                        .map(|d| d > self.ttl)
                        .unwrap_or(false);
                    (Some(e.clone()), expired)
                }
                None => (None, false),
            }
        };

        match (entry_opt, expired) {
            (Some(mut entry), false) => {
                // Hit: 累加 entry.hit_count (本地 clone + 写回 map) + stats.hits
                entry.hit_count = entry.hit_count.saturating_add(1);
                if let Ok(mut entries) = self.entries.write() {
                    if let Some(stored) = entries.get_mut(request_hash) {
                        stored.hit_count = entry.hit_count;
                    }
                }
                if let Ok(mut stats) = self.stats.write() {
                    stats.hits += 1;
                }
                Some(entry)
            }
            (Some(_), true) => {
                // Expired: lazy evict + miss
                if let Ok(mut entries) = self.entries.write() {
                    entries.remove(request_hash);
                }
                if let Ok(mut stats) = self.stats.write() {
                    stats.misses += 1;
                    stats.evictions += 1;
                }
                None
            }
            (None, _) => {
                // Miss (not found), expired flag 永远 false 在这分支
                self.bump_misses();
                None
            }
        }
    }

    /// 内部 helper: bump misses stats (fail-soft)
    fn bump_misses(&self) {
        if let Ok(mut stats) = self.stats.write() {
            stats.misses += 1;
        }
    }

    /// 批量 evict 过期 entries (per 任务 spec: `evict_expired(now) -> usize`)
    ///
    /// **行为**: 遍历 entries, 移除 `now - created_at > ttl` 的, 返 count
    /// **fail-soft**: lock poisoned → 返 0
    pub fn evict_expired(&self, now: SystemTime) -> usize {
        let Ok(mut entries) = self.entries.write() else { return 0 };
        let expired_keys: Vec<String> = entries
            .iter()
            .filter_map(|(k, e)| {
                if now.duration_since(e.created_at).map(|d| d > self.ttl).unwrap_or(false) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();
        let count = expired_keys.len();
        for k in &expired_keys {
            entries.remove(k);
        }
        drop(entries);

        if count > 0 {
            if let Ok(mut stats) = self.stats.write() {
                stats.evictions += count as u64;
            }
        }
        count
    }

    /// 批量 evict, 直到 `len() <= max` (per 任务 spec: `evict_lru(max) -> usize`)
    ///
    /// **LRU 近似**: 按 `created_at` ASC 排序删最旧 (HashMap 无序, 0 假装 Java Map 严格 LRU)
    /// **行为**: `to_evict = len - max`, 取最早 `to_evict` 个, 返 count
    /// **fail-soft**: lock poisoned → 返 0
    pub fn evict_lru(&self, max: usize) -> usize {
        let Ok(mut entries) = self.entries.write() else { return 0 };
        let to_evict = entries.len().saturating_sub(max);
        if to_evict == 0 {
            return 0;
        }
        // 按 created_at ASC 排序 (旧 → 新)
        let mut sorted: Vec<(String, SystemTime)> = entries
            .iter()
            .map(|(k, v)| (k.clone(), v.created_at))
            .collect();
        sorted.sort_by_key(|(_, t)| *t);
        let mut count = 0;
        for (k, _) in sorted.iter().take(to_evict) {
            entries.remove(k);
            count += 1;
        }
        drop(entries);

        if count > 0 {
            if let Ok(mut stats) = self.stats.write() {
                stats.evictions += count as u64;
            }
        }
        count
    }

    /// 当前 stats 快照 (per 任务 spec: `stats() -> ReplayStats`)
    pub fn stats(&self) -> ReplayStats {
        self.stats.read().map(|s| s.clone()).unwrap_or_default()
    }

    /// 当前 entry 数
    pub fn len(&self) -> usize {
        self.entries.read().map(|e| e.len()).unwrap_or(0)
    }

    /// `len() == 0`?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 构造期 `max_entries` 字段 (test / debug 用)
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// 构造期 `ttl` 字段 (test / debug 用)
    pub fn ttl(&self) -> Duration {
        self.ttl
    }
}

impl Default for ResponseReplayCache {
    /// per 任务 spec: max 1000 / 1h TTL
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ENTRIES, DEFAULT_TTL)
    }
}

// ============================================================
// hash_request (free function, VCP `buildKey` 升级版)
// ============================================================

/// 构造 `(method, url, body)` 三元组的 SHA-256 hex 哈希
///
/// **VCP 对比**:
/// - VCP `buildKey(clientIp, messageId)` = `${clientIp}::${messageId}` 简单拼接
/// - 本实现 = `SHA-256(method || \n || url || \n || body)` 安全 hash
///
/// **0 装 / 简化**:
/// - 0 装 clientIp (VCP 用 IP 隔离多客户端, 简化掉 — Rust 由 axum middleware 隔离)
/// - 0 装 messageId (客户端给, 简化成全 body hash)
///
/// **field separator**: `\n` 避免 `method="POST" url="https://api"` 跟 `method="P"
/// url="OST" url="https://api"` 撞 hash (per 编译期守门)
pub fn hash_request(method: &str, url: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.as_bytes());
    hasher.update(b"\n");
    hasher.update(url.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    let hash = hasher.finalize();
    let hex = format!("{:x}", hash);
    debug_assert_eq!(hex.len(), SHA256_HEX_LEN);
    hex
}

// ============================================================
// Process-wide singleton (R122-1-retry 集成入口)
// ============================================================

/// 进程级 ResponseReplayCache singleton (OnceLock lazy init)
///
/// **不漂移 1.0 行为**: `global()` 始终返 Default 配置 (1000/1h) 的同一 `Arc`,
/// 多次调用 `Arc::ptr_eq` 成立 (test #10 守门)
static GLOBAL: OnceLock<Arc<ResponseReplayCache>> = OnceLock::new();

/// 进程级 ResponseReplayCache singleton (Default config: 1000 entries / 1h TTL)
///
/// **集成用法** (per `protocol_handlers::dispatch_inner`):
/// ```ignore
/// use crate::replay_cache::{global as replay_cache, hash_request, ResponsePayload, ReplayEntry};
/// let cache = replay_cache();
/// let key = hash_request("POST", &url, &body_bytes);
/// if let Some(entry) = cache.lookup(&key) { /* hit */ }
/// cache.record(key, payload)?;
/// ```
pub fn global() -> Arc<ResponseReplayCache> {
    GLOBAL
        .get_or_init(|| Arc::new(ResponseReplayCache::default()))
        .clone()
}

// ============================================================
// 单元测试 (≥ 7 per 任务 spec)
// ============================================================

#[cfg(test)]
mod replay_cache_tests {
    use super::*;
    use apeireth_protocol::{NormalizedResponse, NormalizedFinishReason, normalized::NormalizedUsage};
    use std::sync::Arc;

    // ---------- 测试辅助 ----------

    fn make_payload(content: &str) -> ResponsePayload {
        ResponsePayload {
            body: serde_json::json!({"content": content, "id": format!("resp-{content}")}),
            content: content.to_string(),
            model: "gpt-4o".to_string(),
            created_at_secs: 0,
            status: 200,
        }
    }

    fn make_response(content: &str) -> NormalizedResponse {
        NormalizedResponse {
            id: format!("resp-{content}"),
            model: "gpt-4o".to_string(),
            content: content.to_string(),
            finish_reason: Some(NormalizedFinishReason::Stop),
            usage: NormalizedUsage::new(10, 5),
            tool_calls: Vec::new(),
            raw_metadata: Default::default(),
        }
    }

    // ---------- Test 1: record + lookup (任务 spec 明列) ----------

    #[test]
    fn record_then_lookup() {
        let cache = ResponseReplayCache::new(10, Duration::from_secs(60));
        let payload = make_payload("hello");
        cache
            .record("hash1".to_string(), payload.clone())
            .expect("record should succeed");

        let entry = cache.lookup("hash1").expect("should hit");
        assert_eq!(entry.request_hash, "hash1");
        assert_eq!(entry.response.content, "hello");
        assert_eq!(entry.hit_count, 1, "first lookup should bump hit_count to 1");
        assert_eq!(cache.len(), 1);
    }

    // ---------- Test 2: miss returns None (任务 spec 明列) ----------

    #[test]
    fn miss_returns_none() {
        let cache = ResponseReplayCache::default();
        let result = cache.lookup("nonexistent_hash");
        assert!(result.is_none(), "lookup nonexistent should return None");

        let stats = cache.stats();
        assert_eq!(stats.misses, 1, "miss should bump stats.misses");
        assert_eq!(stats.hits, 0, "no hit should bump stats.hits");
        assert_eq!(stats.evictions, 0);
    }

    // ---------- Test 3: evict_expired (任务 spec 明列) ----------

    #[test]
    fn evict_expired() {
        let cache = ResponseReplayCache::new(10, Duration::from_millis(100));
        cache
            .record("k1".to_string(), make_payload("a"))
            .expect("record k1");
        cache
            .record("k2".to_string(), make_payload("b"))
            .expect("record k2");
        assert_eq!(cache.len(), 2);

        // 等过期
        std::thread::sleep(Duration::from_millis(200));

        let now = SystemTime::now();
        let evicted = cache.evict_expired(now);
        assert_eq!(evicted, 2, "both entries should be evicted as expired");
        assert_eq!(cache.len(), 0, "len should be 0 after evict_expired");

        let stats = cache.stats();
        assert_eq!(stats.evictions, 2, "stats.evictions should reflect 2 evictions");
    }

    // ---------- Test 4: evict_lru when over capacity (任务 spec 明列) ----------

    #[test]
    fn evict_lru_when_over_capacity() {
        let cache = ResponseReplayCache::new(2, Duration::from_secs(60));
        cache
            .record("k1".to_string(), make_payload("a"))
            .expect("record k1");
        std::thread::sleep(Duration::from_millis(10));
        cache
            .record("k2".to_string(), make_payload("b"))
            .expect("record k2");
        std::thread::sleep(Duration::from_millis(10));
        cache
            .record("k3".to_string(), make_payload("c"))
            .expect("record k3 (should trigger auto-evict of k1)");

        // 容量 2, 第 3 个 record 触发 auto-evict 1 oldest (k1)
        assert_eq!(cache.len(), 2);
        assert!(cache.lookup("k1").is_none(), "k1 should be auto-evicted");
        assert!(cache.lookup("k2").is_some(), "k2 should still be present");
        assert!(cache.lookup("k3").is_some(), "k3 should be present (just added)");

        // 显式 evict_lru(1) 再 evict 1 oldest
        let initial_stats = cache.stats();
        let evicted = cache.evict_lru(1);
        assert_eq!(evicted, 1, "evict_lru(1) should evict exactly 1");
        assert_eq!(cache.len(), 1, "len should be 1 after evict_lru(1)");
        let final_stats = cache.stats();
        assert_eq!(
            final_stats.evictions,
            initial_stats.evictions + 1,
            "stats.evictions should bump by 1"
        );
    }

    // ---------- Test 5: stats tracks hits and misses (任务 spec 明列) ----------

    #[test]
    fn stats_tracks_hits_and_misses() {
        let cache = ResponseReplayCache::default();

        // 3 misses (不存在的 key)
        for i in 0..3 {
            let _ = cache.lookup(&format!("miss{i}"));
        }

        // 1 record + 2 hits
        cache
            .record("k".to_string(), make_payload("ok"))
            .expect("record k");
        let _ = cache.lookup("k");
        let _ = cache.lookup("k");

        let stats = cache.stats();
        assert_eq!(stats.misses, 3, "3 misses should bump stats.misses to 3");
        assert_eq!(stats.hits, 2, "2 hits should bump stats.hits to 2");
        assert_eq!(stats.evictions, 0, "no eviction should bump stats.evictions");
    }

    // ---------- Test 6: hash_request is deterministic (任务 spec 明列) ----------

    #[test]
    fn hash_request_is_deterministic() {
        let h1 = hash_request("POST", "https://api.example.com/v1/chat", b"hello");
        let h2 = hash_request("POST", "https://api.example.com/v1/chat", b"hello");
        assert_eq!(h1, h2, "same (method, url, body) should produce same hash");
        assert_eq!(h1.len(), SHA256_HEX_LEN, "SHA-256 hex is 64 chars");
        // 编译期守门 (跟 const 同步)
        assert_eq!(SHA256_HEX_LEN, 64);
    }

    // ---------- Test 7: hash_request different for different input (任务 spec 明列) ----------

    #[test]
    fn hash_request_different_for_different_input() {
        let h1 = hash_request("POST", "https://api.example.com/v1/chat", b"hello");

        // body 不同
        let h2 = hash_request("POST", "https://api.example.com/v1/chat", b"world");
        assert_ne!(h1, h2, "different body should produce different hash");

        // method 不同
        let h3 = hash_request("GET", "https://api.example.com/v1/chat", b"hello");
        assert_ne!(h1, h3, "different method should produce different hash");

        // url 不同
        let h4 = hash_request("POST", "https://api.example.com/v2/chat", b"hello");
        assert_ne!(h1, h4, "different url should produce different hash");
    }

    // ---------- Test 8 (bonus): Default impl config (1000/1h) ----------

    #[test]
    fn default_config_is_1000_entries_1h_ttl() {
        let cache = ResponseReplayCache::default();
        assert_eq!(cache.max_entries(), DEFAULT_MAX_ENTRIES);
        assert_eq!(cache.max_entries(), 1000, "Default max_entries must be 1000 per spec");
        assert_eq!(cache.ttl(), DEFAULT_TTL);
        assert_eq!(cache.ttl(), Duration::from_secs(3600), "Default TTL must be 1h per spec");
    }

    // ---------- Test 9 (bonus): lookup expired entry treated as miss (lazy eviction) ----------

    #[test]
    fn lookup_expired_entry_treated_as_miss() {
        let cache = ResponseReplayCache::new(10, Duration::from_millis(50));
        cache
            .record("k".to_string(), make_payload("x"))
            .expect("record k");

        // 立即查: 命中
        let hit = cache.lookup("k");
        assert!(hit.is_some(), "first lookup should hit");
        let stats_after_hit = cache.stats();
        assert_eq!(stats_after_hit.hits, 1);
        assert_eq!(stats_after_hit.misses, 0);
        assert_eq!(stats_after_hit.evictions, 0);

        // 等过期
        std::thread::sleep(Duration::from_millis(100));

        // 再查: 过期, lazy evict, 当 miss
        let result = cache.lookup("k");
        assert!(result.is_none(), "expired lookup should return None");
        let stats_after_expired = cache.stats();
        assert_eq!(stats_after_expired.hits, 1, "hit count stays at 1");
        assert_eq!(stats_after_expired.misses, 1, "expired lookup counts as miss");
        assert_eq!(stats_after_expired.evictions, 1, "expired entry counted as eviction");
        assert_eq!(cache.len(), 0, "expired entry should be removed from entries");
    }

    // ---------- Test 10 (bonus): global() returns same Arc ----------

    #[test]
    fn global_singleton_returns_same_arc() {
        let g1 = global();
        let g2 = global();
        assert!(Arc::ptr_eq(&g1, &g2), "global() should return the same Arc instance");
    }

    // ---------- Test 11 (bonus): ResponsePayload from_response / to_response roundtrip ----------

    #[test]
    fn response_payload_roundtrip_preserves_fields() {
        let resp = make_response("hello world");
        let payload = ResponsePayload::from_response(&resp, 200);
        let restored = payload.to_response().expect("to_response should succeed");

        assert_eq!(restored.id, resp.id);
        assert_eq!(restored.content, resp.content);
        assert_eq!(restored.model, resp.model);
        assert_eq!(restored.usage.prompt_tokens, resp.usage.prompt_tokens);
        assert_eq!(restored.usage.completion_tokens, resp.usage.completion_tokens);
        assert_eq!(restored.usage.total_tokens, resp.usage.total_tokens);
    }

    // ---------- Test 12 (bonus): cache_key prefix constant ----------

    #[test]
    fn cache_key_namespace_constant_set() {
        assert!(!CACHE_KEY_NAMESPACE.is_empty());
        assert!(CACHE_KEY_NAMESPACE.starts_with("apeireth-api:"));
        assert!(CACHE_KEY_NAMESPACE.ends_with(':'));
    }

    // ---------- Test 13 (bonus): record overwrites same key ----------

    #[test]
    fn record_overwrites_existing_key() {
        let cache = ResponseReplayCache::new(10, Duration::from_secs(60));
        cache
            .record("k".to_string(), make_payload("v1"))
            .expect("record v1");
        cache
            .record("k".to_string(), make_payload("v2"))
            .expect("record v2 (overwrite)");

        let entry = cache.lookup("k").expect("should hit");
        assert_eq!(entry.response.content, "v2", "should return v2 (latest)");
        assert_eq!(entry.hit_count, 1);
        assert_eq!(cache.len(), 1, "overwrite should not increase len");
    }

    // ---------- Test 14 (bonus): Default max_entries / TTL 编译期 const 守门 ----------

    #[test]
    fn compile_time_constants_match_spec() {
        // 编译期守门: 跟任务 spec 1:1, 改了编译 fail
        assert_eq!(DEFAULT_MAX_ENTRIES, 1000);
        assert_eq!(DEFAULT_TTL, Duration::from_secs(3600));
        assert_eq!(DEFAULT_HTTP_METHOD, "POST");
        assert_eq!(SHA256_HEX_LEN, 64);
    }
}
