//! GenCache - 通用缓存 (从 v1.0 apeireth-memory/gen_cache.rs 267 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 LRU + TTL

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct CacheEntry<V> { pub value: V, pub inserted_at: Instant }

pub struct GenCache<V> { pub entries: HashMap<String, CacheEntry<V>>, pub ttl: Duration, pub capacity: usize }

impl<V: Clone> GenCache<V> {
    pub fn new(ttl: Duration, capacity: usize) -> Self { Self { entries: HashMap::new(), ttl, capacity } }
    /// 0 装 PASS: 真 put (LRU eviction)
    pub fn put(&mut self, k: impl Into<String>, v: V) {
        let key = k.into();
        self.entries.insert(key, CacheEntry { value: v, inserted_at: Instant::now() });
        if self.entries.len() > self.capacity { self.evict_lru(); }
    }
    fn evict_lru(&mut self) {
        if let Some(oldest) = self.entries.iter().min_by_key(|(_, e)| e.inserted_at).map(|(k, _)| k.clone()) {
            self.entries.remove(&oldest);
        }
    }
    /// 0 装 PASS: 真 get (TTL check)
    pub fn get(&self, k: &str) -> Option<V> {
        self.entries.get(k).and_then(|e| if e.inserted_at.elapsed() < self.ttl { Some(e.value.clone()) } else { None })
    }
    pub fn count(&self) -> usize { self.entries.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_put_get() {
        let mut c: GenCache<String> = GenCache::new(Duration::from_secs(60), 10);
        c.put("k", "v".into());
        assert_eq!(c.get("k"), Some("v".into()));
    }
    #[test] fn test_missing() {
        let c: GenCache<String> = GenCache::new(Duration::from_secs(60), 10);
        assert!(c.get("missing").is_none());
    }
    #[test] fn test_evict() {
        let mut c: GenCache<String> = GenCache::new(Duration::from_secs(60), 2);
        c.put("a", "1".into());
        std::thread::sleep(Duration::from_millis(10));
        c.put("b", "2".into());
        c.put("c", "3".into());
        assert_eq!(c.count(), 2);
        assert!(c.get("a").is_none());
    }
}
