//! TTL-bounded response cache for controlled fetch.
//!
//! Ported semantics from legacy `apeireth-tool-fetch::cache` (R265): a TTL
//! map with hit/miss/eviction statistics. Expired entries are evicted on
//! access; `invalidate`/`clear` give callers explicit control. The donor used
//! a `parking_lot` RwLock; this port uses `std::sync::RwLock` so the crate
//! gains no new dependency.
//!
//! M14 (2026-09-24 审计): 缓存有界化 —— 无上限的 map 会随唯一 URL 数单调
//! 增长 (模型循环拉取大量不同 URL → 长跑进程 OOM)。现有 `max_entries`
//! (LRU 驱逐) + 总字节预算 (序列化长度近似) 双维度上限。
//!
//! Caching is a process-local scheduling optimization. It never changes what
//! was approved: keys are the exact frozen normalized request URLs, and only
//! successful textual responses are stored.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// 默认最大条目数 (M14): 超限按 LRU 驱逐。
pub const DEFAULT_MAX_ENTRIES: usize = 256;

/// 默认总字节预算 (M14): 所有条目序列化长度之和的上限, 超限按 LRU 驱逐。
pub const DEFAULT_MAX_TOTAL_BYTES: usize = 16 * 1024 * 1024;

/// Snapshot of cache counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseCacheStats {
    pub size: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

#[derive(Debug)]
struct CacheEntry {
    value: serde_json::Value,
    expiry: Instant,
    /// LRU  ticks: 越大越近期使用 (get 命中与 put 都刷新)。
    last_used: u64,
    /// 近似序列化字节量 (总预算的计量单位)。
    bytes: usize,
}

#[derive(Debug)]
struct Inner {
    entries: HashMap<String, CacheEntry>,
    hits: u64,
    misses: u64,
    evictions: u64,
    /// 单调递增的 LRU 时钟。
    tick: u64,
    /// 所有条目 bytes 之和 (O(1) 预算核算)。
    total_bytes: usize,
}

/// A TTL cache from request URL to stored response value.
#[derive(Debug)]
pub struct ResponseCache {
    inner: RwLock<Inner>,
    ttl: Duration,
    max_entries: usize,
    max_total_bytes: usize,
}

impl ResponseCache {
    /// Build a cache with the given time-to-live.
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: RwLock::new(Inner {
                entries: HashMap::new(),
                hits: 0,
                misses: 0,
                evictions: 0,
                tick: 0,
                total_bytes: 0,
            }),
            ttl,
            max_entries: DEFAULT_MAX_ENTRIES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
        }
    }

    /// M14: 覆盖最大条目数上限 (测试与受限部署用)。
    #[must_use]
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries.max(1);
        self
    }

    /// M14: 覆盖总字节预算 (测试与受限部署用)。
    #[must_use]
    pub fn with_max_total_bytes(mut self, max_total_bytes: usize) -> Self {
        self.max_total_bytes = max_total_bytes;
        self
    }

    /// The configured TTL.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// The configured entry-count cap.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// The configured total-byte budget.
    pub fn max_total_bytes(&self) -> usize {
        self.max_total_bytes
    }

    /// Get the cached value for `key` when present and unexpired.
    ///
    /// An expired entry is evicted and counted as an eviction plus a miss.
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = Instant::now();
        let unexpired = g
            .entries
            .get(key)
            .filter(|entry| now < entry.expiry)
            .map(|entry| entry.value.clone());
        if let Some(value) = unexpired {
            g.tick += 1;
            let tick = g.tick;
            if let Some(entry) = g.entries.get_mut(key) {
                entry.last_used = tick;
            }
            g.hits += 1;
            return Some(value);
        }
        // 惰性过期驱逐 (保留既有语义: 过期即 eviction + miss)。
        if let Some(evicted) = g.entries.remove(key) {
            g.total_bytes = g.total_bytes.saturating_sub(evicted.bytes);
            g.evictions += 1;
        }
        g.misses += 1;
        None
    }

    /// Store `value` under `key` with the configured TTL.
    ///
    /// M14: 写入后执行 LRU 驱逐 —— 条目数超 `max_entries` 或总字节超
    /// `max_total_bytes` 时淘汰最久未用者, 直到回到预算内。
    pub fn put(&self, key: impl Into<String>, value: serde_json::Value) {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let key = key.into();
        let bytes = serde_json::to_vec(&value).map(|b| b.len()).unwrap_or(0);
        g.tick += 1;
        let tick = g.tick;
        let expiry = Instant::now() + self.ttl;
        if let Some(previous) = g.entries.remove(&key) {
            g.total_bytes = g.total_bytes.saturating_sub(previous.bytes);
        }
        g.entries.insert(
            key,
            CacheEntry {
                value,
                expiry,
                last_used: tick,
                bytes,
            },
        );
        g.total_bytes = g.total_bytes.saturating_add(bytes);
        g.evict_to_budget(self.max_entries, self.max_total_bytes);
    }

    /// Remove one entry. True when it existed.
    pub fn invalidate(&self, key: &str) -> bool {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match g.entries.remove(key) {
            Some(entry) => {
                g.total_bytes = g.total_bytes.saturating_sub(entry.bytes);
                true
            }
            None => false,
        }
    }

    /// Remove every entry.
    pub fn clear(&self) {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        g.entries.clear();
        g.total_bytes = 0;
    }

    /// Current counters.
    pub fn stats(&self) -> ResponseCacheStats {
        let g = self
            .inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        ResponseCacheStats {
            size: g.entries.len(),
            hits: g.hits,
            misses: g.misses,
            evictions: g.evictions,
        }
    }
}

impl Inner {
    /// LRU 驱逐直到条目数与总字节都回到预算内。
    ///
    /// 条目数超限无条件驱逐到 `max_entries`; 字节预算至少保留 1 条 ——
    /// 单条响应超预算时无限自驱逐会让缓存永远存不进东西 (抖动), 保留最新
    /// 写入换来"最坏一条"的有界性 (fetch 响应体本身另有上限)。
    fn evict_to_budget(&mut self, max_entries: usize, max_total_bytes: usize) {
        while self.entries.len() > max_entries {
            if !self.evict_lru() {
                return;
            }
        }
        while self.entries.len() > 1 && self.total_bytes > max_total_bytes {
            if !self.evict_lru() {
                return;
            }
        }
    }

    /// 淘汰最久未用的一条; 返回是否真的驱逐了 (空 map 返 false)。
    fn evict_lru(&mut self) -> bool {
        let Some(lru_key) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| key.clone())
        else {
            return false;
        };
        if let Some(evicted) = self.entries.remove(&lru_key) {
            self.total_bytes = self.total_bytes.saturating_sub(evicted.bytes);
        }
        self.evictions += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn put_get_round_trip() {
        let c = ResponseCache::new(Duration::from_secs(60));
        c.put("https://x.com", json!({"status": 200}));
        assert_eq!(c.get("https://x.com"), Some(json!({"status": 200})));
    }

    #[test]
    fn miss_returns_none_and_counts() {
        let c = ResponseCache::new(Duration::from_secs(60));
        assert_eq!(c.get("nope"), None);
        assert_eq!(c.stats().misses, 1);
    }

    #[test]
    fn invalidate_removes_entry() {
        let c = ResponseCache::new(Duration::from_secs(60));
        c.put("k", json!(1));
        assert!(c.invalidate("k"));
        assert_eq!(c.get("k"), None);
        assert!(!c.invalidate("k"));
    }

    #[test]
    fn stats_track_hits_misses_and_size() {
        let c = ResponseCache::new(Duration::from_secs(60));
        c.put("k", json!(1));
        let _ = c.get("k");
        let _ = c.get("absent");
        let s = c.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.misses, 1);
        assert_eq!(s.size, 1);
        assert_eq!(s.evictions, 0);
    }

    #[test]
    fn expired_entry_is_evicted_on_access() {
        let c = ResponseCache::new(Duration::from_millis(30));
        c.put("k", json!("v"));
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(c.get("k"), None);
        let s = c.stats();
        assert_eq!(s.evictions, 1);
        assert_eq!(s.misses, 1);
        assert_eq!(s.size, 0);
    }

    #[test]
    fn clear_removes_all() {
        let c = ResponseCache::new(Duration::from_secs(60));
        c.put("a", json!(1));
        c.put("b", json!(2));
        c.clear();
        assert_eq!(c.stats().size, 0);
    }

    #[test]
    fn ttl_is_preserved() {
        let c = ResponseCache::new(Duration::from_millis(1234));
        assert_eq!(c.ttl(), Duration::from_millis(1234));
    }

    // ---- M14 (2026-09-24 审计): 有界化 ----

    #[test]
    fn lru_evicts_oldest_when_over_max_entries() {
        let c = ResponseCache::new(Duration::from_secs(60)).with_max_entries(2);
        c.put("a", json!(1));
        c.put("b", json!(2));
        c.put("c", json!(3));
        assert_eq!(c.get("a"), None, "oldest entry must be evicted");
        assert_eq!(c.get("c"), Some(json!(3)));
        let s = c.stats();
        assert_eq!(s.size, 2);
        assert_eq!(s.evictions, 1);
    }

    #[test]
    fn get_refreshes_lru_order() {
        let c = ResponseCache::new(Duration::from_secs(60)).with_max_entries(2);
        c.put("a", json!(1));
        c.put("b", json!(2));
        assert_eq!(c.get("a"), Some(json!(1)), "touch a so b becomes LRU");
        c.put("c", json!(3));
        assert_eq!(c.get("b"), None, "untouched b must be evicted");
        assert_eq!(c.get("a"), Some(json!(1)));
        assert_eq!(c.get("c"), Some(json!(3)));
    }

    #[test]
    fn total_byte_budget_evicts_lru() {
        // 每个条目约几十字节; 预算设置为只放得下两个的值。
        let value = json!({ "payload": "x".repeat(200) });
        let one = serde_json::to_vec(&value).unwrap().len();
        let c = ResponseCache::new(Duration::from_secs(60))
            .with_max_entries(64)
            .with_max_total_bytes(one * 2);
        c.put("a", value.clone());
        c.put("b", value.clone());
        assert_eq!(c.stats().size, 2);
        c.put("c", value.clone());
        assert_eq!(c.get("a"), None, "budget overflow must evict the LRU entry");
        assert_eq!(c.get("c"), Some(value));
        assert!(c.stats().evictions >= 1);
    }

    #[test]
    fn single_oversize_entry_is_kept_but_cache_stays_bounded() {
        // 单条就超预算: 不无限自驱逐 (至少保留最新写入), 但缓存不再增长。
        let big = json!({ "payload": "y".repeat(4096) });
        let one = serde_json::to_vec(&big).unwrap().len();
        let c = ResponseCache::new(Duration::from_secs(60))
            .with_max_entries(64)
            .with_max_total_bytes(one / 2);
        c.put("only", big.clone());
        assert_eq!(c.get("only"), Some(big));
        assert_eq!(c.stats().size, 1);
    }

    #[test]
    fn invalidate_reclaims_byte_budget() {
        let value = json!({ "payload": "z".repeat(200) });
        let c = ResponseCache::new(Duration::from_secs(60))
            .with_max_entries(64)
            .with_max_total_bytes(serde_json::to_vec(&value).unwrap().len());
        c.put("a", value.clone());
        c.put("b", value.clone());
        assert_eq!(c.get("a"), None, "second insert must evict the first");
        c.invalidate("b");
        c.put("c", value.clone());
        assert_eq!(c.get("c"), Some(value), "budget must be reclaimed");
    }
}
