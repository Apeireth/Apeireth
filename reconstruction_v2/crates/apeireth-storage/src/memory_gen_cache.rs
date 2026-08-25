//! Memory GenCache - 通用缓存 (抄 v1 apeireth-memory/gen_cache.rs)
use std::collections::HashMap;
use std::time::{Duration, Instant};
pub struct CacheEntry<V> { pub value: V, pub inserted_at: Instant }
pub struct GenCache<V> { pub entries: HashMap<String, CacheEntry<V>>, pub ttl: Duration, pub capacity: usize }
impl<V: Clone> GenCache<V> {
    pub fn new(ttl: Duration, capacity: usize) -> Self { Self { entries: HashMap::new(), ttl, capacity } }
    pub fn put(&mut self, k: impl Into<String>, v: V) {
        let key = k.into();
        self.entries.insert(key, CacheEntry { value: v, inserted_at: Instant::now() });
        if self.entries.len() > self.capacity {
            if let Some((oldest_k, _)) = self.entries.iter().min_by_key(|(_, e)| e.inserted_at).map(|(k, _)| (k.clone(), ())).clone() { self.entries.remove(&oldest_k); }
        }
    }
    pub fn get(&self, k: &str) -> Option<V> {
        self.entries.get(k).and_then(|e| if e.inserted_at.elapsed() < self.ttl { Some(e.value.clone()) } else { None })
    }
    pub fn count(&self) -> usize { self.entries.len() }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_basic() { let mut c: GenCache<String> = GenCache::new(Duration::from_secs(60), 10); c.put("a", "v".to_string()); assert_eq!(c.get("a"), Some("v".into())); } #[test] fn test_miss() { let c: GenCache<String> = GenCache::new(Duration::from_secs(60), 10); assert!(c.get("x").is_none()); } }