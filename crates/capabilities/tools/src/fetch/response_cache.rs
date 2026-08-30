//! TTL-bounded response cache for controlled fetch.
//!
//! Ported semantics from legacy `apeireth-tool-fetch::cache` (R265): a TTL
//! map with hit/miss/eviction statistics. Expired entries are evicted on
//! access; `invalidate`/`clear` give callers explicit control. The donor used
//! a `parking_lot` RwLock; this port uses `std::sync::RwLock` so the crate
//! gains no new dependency.
//!
//! Caching is a process-local scheduling optimization. It never changes what
//! was approved: keys are the exact frozen normalized request URLs, and only
//! successful textual responses are stored.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Snapshot of cache counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseCacheStats {
    pub size: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

#[derive(Debug)]
struct Inner {
    entries: HashMap<String, (serde_json::Value, Instant)>,
    hits: u64,
    misses: u64,
    evictions: u64,
}

/// A TTL cache from request URL to stored response value.
#[derive(Debug)]
pub struct ResponseCache {
    inner: RwLock<Inner>,
    ttl: Duration,
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
            }),
            ttl,
        }
    }

    /// The configured TTL.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Get the cached value for `key` when present and unexpired.
    ///
    /// An expired entry is evicted and counted as an eviction plus a miss.
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((value, expiry)) = g.entries.get(key).cloned() {
            if Instant::now() < expiry {
                g.hits += 1;
                return Some(value);
            }
            g.entries.remove(key);
            g.evictions += 1;
        }
        g.misses += 1;
        None
    }

    /// Store `value` under `key` with the configured TTL.
    pub fn put(&self, key: impl Into<String>, value: serde_json::Value) {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        g.entries
            .insert(key.into(), (value, Instant::now() + self.ttl));
    }

    /// Remove one entry. True when it existed.
    pub fn invalidate(&self, key: &str) -> bool {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        g.entries.remove(key).is_some()
    }

    /// Remove every entry.
    pub fn clear(&self) {
        let mut g = self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        g.entries.clear();
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
}
