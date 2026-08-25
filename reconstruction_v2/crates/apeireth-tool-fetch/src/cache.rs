//! TTL cache shared across fetchers.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct FetchCache {
    inner: RwLock<Inner>,
    ttl: Duration,
}

struct Inner {
    entries: HashMap<String, (String, Instant)>,
    hits: u64,
    misses: u64,
    evictions: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    pub size: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl FetchCache {
    pub fn new(ttl_ms: u64) -> Self {
        Self {
            inner: RwLock::new(Inner {
                entries: HashMap::new(),
                hits: 0,
                misses: 0,
                evictions: 0,
            }),
            ttl: Duration::from_millis(ttl_ms),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut g = self.inner.write();
        if let Some((v, exp)) = g.entries.get(key).cloned() {
            if Instant::now() < exp {
                g.hits += 1;
                return Some(v);
            }
            g.entries.remove(key);
            g.evictions += 1;
        }
        g.misses += 1;
        None
    }

    pub fn put(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut g = self.inner.write();
        g.entries
            .insert(key.into(), (value.into(), Instant::now() + self.ttl));
    }

    pub fn invalidate(&self, key: &str) -> bool {
        self.inner.write().entries.remove(key).is_some()
    }

    pub fn clear(&self) {
        self.inner.write().entries.clear();
    }

    pub fn stats(&self) -> CacheStats {
        let g = self.inner.read();
        CacheStats {
            size: g.entries.len(),
            hits: g.hits,
            misses: g.misses,
            evictions: g.evictions,
        }
    }
}

impl Default for FetchCache {
    fn default() -> Self {
        Self::new(60_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_get_basic() {
        let c = FetchCache::new(60_000);
        c.put("k", "v");
        assert_eq!(c.get("k"), Some("v".into()));
    }

    #[test]
    fn miss_returns_none() {
        let c = FetchCache::new(60_000);
        assert_eq!(c.get("nope"), None);
    }

    #[test]
    fn invalidate_removes() {
        let c = FetchCache::new(60_000);
        c.put("k", "v");
        assert!(c.invalidate("k"));
        assert_eq!(c.get("k"), None);
    }

    #[test]
    fn stats_track() {
        let c = FetchCache::new(60_000);
        c.put("k", "v");
        c.get("k");
        c.get("nope");
        let s = c.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.misses, 1);
        assert_eq!(s.size, 1);
    }

    #[test]
    fn clear_removes_all() {
        let c = FetchCache::new(60_000);
        c.put("a", "1");
        c.put("b", "2");
        c.clear();
        assert_eq!(c.stats().size, 0);
    }
}
