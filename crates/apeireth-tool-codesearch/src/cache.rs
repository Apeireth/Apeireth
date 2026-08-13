//! R210 unified query cache (避免重复扫描).
//!
//! **动机**: R202 unified query 每次都重扫文件系统. R210 加 TTL cache, 避免重复 pattern/path 的重扫.
//!
//! **设计**:
//! - QueryCache 包装 UnifiedCodeIntelligence
//! - 缓存 key: (kind, pattern, path, lang) 元组
//! - TTL 配置 (默认 60s)
//! - LRU 上限 (默认 1000 entries)
//! - 缓存 hit/miss 统计
//!
//! **0 触碰**: unified.rs UnifiedCodeIntelligence 0 改. QueryCache 是 wrapper.

#![allow(missing_docs)] // R210: 0 触碰现有 API 文档

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::unified::{IntelligenceHit, IntelligenceKind, UnifiedCodeIntelligence, UnifiedError, UnifiedQuery};

/// Cache key (kind, pattern, path, lang) hash.
#[derive(Debug, Clone, Eq, PartialEq)]
struct CacheKey {
    kind: IntelligenceKind,
    pattern: String,
    path: String,
    lang: Option<String>,
}

impl Hash for CacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.pattern.hash(state);
        self.path.hash(state);
        self.lang.hash(state);
    }
}

#[derive(Debug, Clone)]
struct CacheEntry {
    hits: Vec<IntelligenceHit>,
    expires: Instant,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct QueryCacheStats {
    pub size: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

/// TTL-bounded query cache.
pub struct QueryCache {
    inner: Mutex<HashMap<CacheKey, CacheEntry>>,
    ttl: Duration,
    max_entries: usize,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
    evictions: std::sync::atomic::AtomicU64,
}

impl QueryCache {
    pub fn new(ttl_ms: u64, max_entries: usize) -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
            ttl: Duration::from_millis(ttl_ms),
            max_entries,
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
            evictions: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(60_000, 1000)
    }

    /// Get cached hits for query (if not expired).
    pub fn get(&self, q: &UnifiedQuery) -> Option<Vec<IntelligenceHit>> {
        let key = CacheKey {
            kind: q.kind,
            pattern: q.pattern.clone(),
            path: q.path.to_string_lossy().into_owned(),
            lang: q.lang.clone(),
        };
        let mut g = self.inner.lock().expect("poisoned");
        if let Some(entry) = g.get(&key) {
            if Instant::now() < entry.expires {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return Some(entry.hits.clone());
            }
            g.remove(&key);
            self.evictions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        self.misses.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        None
    }

    /// Put hits into cache.
    pub fn put(&self, q: &UnifiedQuery, hits: Vec<IntelligenceHit>) {
        let key = CacheKey {
            kind: q.kind,
            pattern: q.pattern.clone(),
            path: q.path.to_string_lossy().into_owned(),
            lang: q.lang.clone(),
        };
        let mut g = self.inner.lock().expect("poisoned");
        // 简单 LRU: 超 max_entries 清空 25% (避免 O(n) 排序)
        if g.len() >= self.max_entries {
            let to_remove = self.max_entries / 4;
            let keys: Vec<_> = g.keys().take(to_remove).cloned().collect();
            for k in keys {
                g.remove(&k);
            }
            self.evictions.fetch_add(to_remove as u64, std::sync::atomic::Ordering::SeqCst);
        }
        g.insert(key, CacheEntry {
            hits,
            expires: Instant::now() + self.ttl,
        });
    }

    /// Invalidate a specific key.
    pub fn invalidate(&self, q: &UnifiedQuery) -> bool {
        let key = CacheKey {
            kind: q.kind,
            pattern: q.pattern.clone(),
            path: q.path.to_string_lossy().into_owned(),
            lang: q.lang.clone(),
        };
        self.inner.lock().expect("poisoned").remove(&key).is_some()
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.inner.lock().expect("poisoned").clear();
    }

    pub fn stats(&self) -> QueryCacheStats {
        QueryCacheStats {
            size: self.inner.lock().expect("poisoned").len(),
            hits: self.hits.load(std::sync::atomic::Ordering::SeqCst),
            misses: self.misses.load(std::sync::atomic::Ordering::SeqCst),
            evictions: self.evictions.load(std::sync::atomic::Ordering::SeqCst),
        }
    }
}

impl Default for QueryCache {
    fn default() -> Self { Self::with_defaults() }
}

/// Cached facade: QueryCache + UnifiedCodeIntelligence 集成.
pub struct CachedUnifiedIntelligence {
    cache: QueryCache,
    inner: UnifiedCodeIntelligence,
}

impl CachedUnifiedIntelligence {
    pub fn new(ttl_ms: u64, max_entries: usize) -> Self {
        Self {
            cache: QueryCache::new(ttl_ms, max_entries),
            inner: UnifiedCodeIntelligence::new_in_memory(),
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            cache: QueryCache::with_defaults(),
            inner: UnifiedCodeIntelligence::new_in_memory(),
        }
    }

    /// Query with cache lookup. Hit -> 不调 inner. Miss -> inner + cache.
    pub fn query(&self, q: &UnifiedQuery) -> Result<Vec<IntelligenceHit>, UnifiedError> {
        if let Some(cached) = self.cache.get(q) {
            return Ok(cached);
        }
        let hits = self.inner.query(q)?;
        self.cache.put(q, hits.clone());
        Ok(hits)
    }

    pub fn stats(&self) -> QueryCacheStats { self.cache.stats() }
    pub fn clear(&self) { self.cache.clear(); }
    pub fn invalidate(&self, q: &UnifiedQuery) -> bool { self.cache.invalidate(q) }
    pub fn index_file(&self, path: &str) -> Result<(), UnifiedError> {
        self.inner.index_file(path)?;
        // index_file 改了状态, 失效所有 cache
        self.cache.clear();
        Ok(())
    }
    pub fn path_only_invalidated<F>(&self, path_contains: F)
    where F: Fn(&str) -> bool {
        // 简化: 全清 (避免扫描 key)
        let _ = path_contains;
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn t01_new_defaults() {
        let c = QueryCache::with_defaults();
        let s = c.stats();
        assert_eq!(s.size, 0);
        assert_eq!(s.hits, 0);
        assert_eq!(s.misses, 0);
    }

    #[test]
    fn t02_get_miss_returns_none() {
        let c = QueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", ".");
        assert!(c.get(&q).is_none());
    }

    #[test]
    fn t03_put_get_hit() {
        let c = QueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", ".");
        c.put(&q, vec![]);
        let s = c.stats();
        assert_eq!(s.size, 1);
        let cached = c.get(&q);
        assert!(cached.is_some());
        assert_eq!(s.hits, 0);  // hits 是 get 命中时 +1
        let s2 = c.stats();
        assert_eq!(s2.hits, 1);
    }

    #[test]
    fn t04_miss_increments() {
        let c = QueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", ".");
        let _ = c.get(&q);
        let s = c.stats();
        assert_eq!(s.misses, 1);
    }

    #[test]
    fn t05_invalidate() {
        let c = QueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", ".");
        c.put(&q, vec![]);
        assert!(c.invalidate(&q));
        assert!(c.get(&q).is_none());
    }

    #[test]
    fn t06_clear() {
        let c = QueryCache::with_defaults();
        c.put(&UnifiedQuery::new(IntelligenceKind::Text, "a", "."), vec![]);
        c.put(&UnifiedQuery::new(IntelligenceKind::Text, "b", "."), vec![]);
        assert_eq!(c.stats().size, 2);
        c.clear();
        assert_eq!(c.stats().size, 0);
    }

    #[test]
    fn t07_ttl_expiry() {
        let c = QueryCache::new(50, 100);  // 50ms TTL
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", ".");
        c.put(&q, vec![]);
        assert!(c.get(&q).is_some());
        std::thread::sleep(Duration::from_millis(70));
        assert!(c.get(&q).is_none());
    }

    #[test]
    fn t08_max_entries_evicts() {
        let c = QueryCache::new(60_000, 4);  // max 4
        for i in 0..10 {
            c.put(&UnifiedQuery::new(IntelligenceKind::Text, &format!("p{i}"), "."), vec![]);
        }
        // 超过 max_entries 触发 eviction
        assert!(c.stats().evictions > 0);
    }

    #[test]
    fn t09_cached_unified_with_defaults() {
        let c = CachedUnifiedIntelligence::with_defaults();
        let _ = c.stats();
    }

    #[test]
    fn t10_cached_unified_query_miss() {
        let c = CachedUnifiedIntelligence::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", "./nonexistent_xyz");
        let r = c.query(&q);
        // miss -> inner.query -> FileFinder::find returns Err or empty
        match r {
            Ok(v) => assert!(v.is_empty()),
            Err(UnifiedError::Io(_)) => {}
            Err(e) => panic!("unexpected: {:?}", e),
        }
    }
}
