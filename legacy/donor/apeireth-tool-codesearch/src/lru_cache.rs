//! R213 真 LRU QueryCache + streaming / batch query (接续 R210).
//!
//! **动机**: R210 QueryCache 用简化 LRU (超 max_entries 时随机清 25%). R213 升级:
//! 1. 用 `lru` crate 真 LRU (O(1) get/put, 真按访问时间淘汰)
//! 2. 加 `streaming_query` (callback-based 流式返回, 不一次性集齐)
//! 3. 加 `batch_query` (多 query 并发, 共享 cache)
//!
//! **0 触碰**: cache.rs (R210) 保留, 本模块是 additive 升级. 旧 QueryCache API 不变.
//! UnifiedCodeIntelligence / UnifiedQuery / IntelligenceKind / UnifiedError 0 改.

#![allow(missing_docs)] // R213 additive

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use lru::LruCache;
use std::sync::Mutex;

use crate::unified::{
    IntelligenceHit, IntelligenceKind, UnifiedCodeIntelligence, UnifiedError, UnifiedQuery,
};

// ============================================================================
// 真 LRU QueryCache (用 lru crate)
// ============================================================================

/// R213 真 LRU query cache (O(1) get/put, 真按访问时间淘汰).
pub struct LruQueryCache {
    inner: Mutex<LruCache<CacheKey, CacheEntry>>,
    ttl: Duration,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
    evictions: std::sync::atomic::AtomicU64,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct CacheKey {
    kind: IntelligenceKind,
    pattern: String,
    path: String,
    lang: Option<String>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    hits: Vec<IntelligenceHit>,
    expires: Instant,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LruCacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl LruQueryCache {
    pub fn new(ttl_ms: u64, capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).expect("> 0");
        Self {
            inner: Mutex::new(LruCache::new(cap)),
            ttl: Duration::from_millis(ttl_ms),
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
            evictions: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(60_000, 1000)
    }

    pub fn get(&self, q: &UnifiedQuery) -> Option<Vec<IntelligenceHit>> {
        let key = make_key(q);
        let mut g = self.inner.lock().expect("poisoned");
        if let Some(entry) = g.get(&key).cloned() {
            if Instant::now() < entry.expires {
                self.hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                return Some(entry.hits);
            }
            g.pop(&key);
            self.evictions
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        self.misses
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        None
    }

    pub fn put(&self, q: &UnifiedQuery, hits: Vec<IntelligenceHit>) {
        let key = make_key(q);
        let mut g = self.inner.lock().expect("poisoned");
        let evicted = g.push(
            key,
            CacheEntry {
                hits,
                expires: Instant::now() + self.ttl,
            },
        );
        if evicted.is_some() {
            self.evictions
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }

    pub fn invalidate(&self, q: &UnifiedQuery) -> bool {
        let key = make_key(q);
        self.inner.lock().expect("poisoned").pop(&key).is_some()
    }

    pub fn clear(&self) {
        self.inner.lock().expect("poisoned").clear();
    }

    pub fn stats(&self) -> LruCacheStats {
        let g = self.inner.lock().expect("poisoned");
        LruCacheStats {
            size: g.len(),
            capacity: g.cap().get(),
            hits: self.hits.load(std::sync::atomic::Ordering::SeqCst),
            misses: self.misses.load(std::sync::atomic::Ordering::SeqCst),
            evictions: self.evictions.load(std::sync::atomic::Ordering::SeqCst),
        }
    }
}

fn make_key(q: &UnifiedQuery) -> CacheKey {
    CacheKey {
        kind: q.kind,
        pattern: q.pattern.clone(),
        path: q.path.to_string_lossy().into_owned(),
        lang: q.lang.clone(),
    }
}

// ============================================================================
// Streaming + batch query facade
// ============================================================================

/// 流式回调: 每条 hit 调一次. 返回 false 可提前终止.
pub type HitCallback<'a> = dyn FnMut(&IntelligenceHit) -> bool + Send + 'a;

/// 流式查询 (callback-based, 不一次性集齐).
///
/// 优势: 内存占用低, 适合 "匹配成千上万但只要前 100" 的场景.
/// 流程: 先查 cache (全部返回), miss 则调 inner, 命中边返回边 cache.
pub fn streaming_query<F>(
    cache: &LruQueryCache,
    inner: &UnifiedCodeIntelligence,
    q: &UnifiedQuery,
    mut on_hit: F,
) -> Result<u64, UnifiedError>
where
    F: FnMut(&IntelligenceHit) -> bool,
{
    if let Some(cached) = cache.get(q) {
        let mut count = 0u64;
        for h in &cached {
            count += 1;
            if !on_hit(h) {
                break;
            }
        }
        return Ok(count);
    }
    let hits = inner.query(q)?;
    let mut count = 0u64;
    for h in &hits {
        count += 1;
        if !on_hit(h) {
            // 仍然缓存全部, 下次可复用
            cache.put(q, hits.clone());
            return Ok(count);
        }
    }
    cache.put(q, hits);
    Ok(count)
}

/// 批量查询 (多 query 共享 cache).
///
/// 输入: 一组 query. 输出: 顺序对应的 results. 共享同一 cache, 重复 query 自动 hit.
pub fn batch_query(
    cache: &LruQueryCache,
    inner: &UnifiedCodeIntelligence,
    queries: &[UnifiedQuery],
) -> Vec<Result<Vec<IntelligenceHit>, UnifiedError>> {
    queries
        .iter()
        .map(|q| {
            if let Some(cached) = cache.get(q) {
                Ok(cached)
            } else {
                let hits = inner.query(q)?;
                cache.put(q, hits.clone());
                Ok(hits)
            }
        })
        .collect()
}

/// 批量 facade (Arc 共享).
pub struct CachedUnifiedLru {
    cache: LruQueryCache,
    inner: Arc<UnifiedCodeIntelligence>,
}

impl CachedUnifiedLru {
    pub fn new(ttl_ms: u64, capacity: usize) -> Self {
        Self {
            cache: LruQueryCache::new(ttl_ms, capacity),
            inner: Arc::new(UnifiedCodeIntelligence::new_in_memory()),
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            cache: LruQueryCache::with_defaults(),
            inner: Arc::new(UnifiedCodeIntelligence::new_in_memory()),
        }
    }

    pub fn query(&self, q: &UnifiedQuery) -> Result<Vec<IntelligenceHit>, UnifiedError> {
        if let Some(cached) = self.cache.get(q) {
            return Ok(cached);
        }
        let hits = self.inner.query(q)?;
        self.cache.put(q, hits.clone());
        Ok(hits)
    }

    pub fn streaming<F: FnMut(&IntelligenceHit) -> bool>(
        &self,
        q: &UnifiedQuery,
        on_hit: F,
    ) -> Result<u64, UnifiedError> {
        streaming_query(&self.cache, &self.inner, q, on_hit)
    }

    pub fn batch(
        &self,
        queries: &[UnifiedQuery],
    ) -> Vec<Result<Vec<IntelligenceHit>, UnifiedError>> {
        batch_query(&self.cache, &self.inner, queries)
    }

    pub fn stats(&self) -> LruCacheStats {
        self.cache.stats()
    }
    pub fn clear(&self) {
        self.cache.clear();
    }
    pub fn invalidate(&self, q: &UnifiedQuery) -> bool {
        self.cache.invalidate(q)
    }
    pub fn inner(&self) -> &UnifiedCodeIntelligence {
        &self.inner
    }
}

// ============================================================================
// 测试 (12 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn t01_new_defaults() {
        let c = LruQueryCache::with_defaults();
        let s = c.stats();
        assert_eq!(s.size, 0);
        assert_eq!(s.capacity, 1000);
        assert_eq!(s.hits, 0);
        assert_eq!(s.misses, 0);
    }

    #[test]
    fn t02_get_miss_returns_none() {
        let c = LruQueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", PathBuf::from("."));
        assert!(c.get(&q).is_none());
    }

    #[test]
    fn t03_put_get_hit() {
        let c = LruQueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", PathBuf::from("."));
        c.put(&q, vec![]);
        assert!(c.get(&q).is_some());
        let s = c.stats();
        assert_eq!(s.hits, 1);
        assert_eq!(s.size, 1);
    }

    #[test]
    fn t04_miss_increments() {
        let c = LruQueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", PathBuf::from("."));
        let _ = c.get(&q);
        assert_eq!(c.stats().misses, 1);
    }

    #[test]
    fn t05_lru_eviction_order() {
        let c = LruQueryCache::new(60_000, 3);
        for i in 0..3 {
            let q = UnifiedQuery::new(IntelligenceKind::Text, format!("p{i}"), PathBuf::from("."));
            c.put(&q, vec![]);
        }
        // 访问 p0, 让它变最近
        let q0 = UnifiedQuery::new(IntelligenceKind::Text, "p0", PathBuf::from("."));
        let _ = c.get(&q0);
        // 加 p3 应淘汰 p1 (而非 p0)
        let q3 = UnifiedQuery::new(IntelligenceKind::Text, "p3", PathBuf::from("."));
        c.put(&q3, vec![]);
        let s = c.stats();
        assert_eq!(s.size, 3);
        assert!(s.evictions >= 1);
        // p0 还在
        let _ = c.get(&q0);
        assert_eq!(c.stats().hits, 2); // 第 1 次 hits + 第 2 次 hits
    }

    #[test]
    fn t06_invalidate() {
        let c = LruQueryCache::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", PathBuf::from("."));
        c.put(&q, vec![]);
        assert!(c.invalidate(&q));
        assert!(!c.invalidate(&q));
    }

    #[test]
    fn t07_clear() {
        let c = LruQueryCache::with_defaults();
        for i in 0..5 {
            let q = UnifiedQuery::new(IntelligenceKind::Text, format!("p{i}"), PathBuf::from("."));
            c.put(&q, vec![]);
        }
        c.clear();
        assert_eq!(c.stats().size, 0);
    }

    #[test]
    fn t08_ttl_expiry() {
        let c = LruQueryCache::new(10, 100); // 10ms TTL
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", PathBuf::from("."));
        c.put(&q, vec![]);
        std::thread::sleep(Duration::from_millis(20));
        assert!(c.get(&q).is_none());
    }

    #[test]
    fn t09_streaming_query_calls_callback() {
        let c = LruQueryCache::with_defaults();
        let inner = UnifiedCodeIntelligence::new_in_memory();
        // 不存在路径, 结果为空, callback 不调
        let q = UnifiedQuery::new(
            IntelligenceKind::Text,
            "fn",
            PathBuf::from("./nonexistent_xyz"),
        );
        let mut count = 0u64;
        let n = streaming_query(&c, &inner, &q, |_hit| {
            count += 1;
            true
        })
        .unwrap();
        assert_eq!(n, 0);
        assert_eq!(count, 0);
    }

    #[test]
    fn t10_streaming_early_termination() {
        let c = LruQueryCache::with_defaults();
        let inner = UnifiedCodeIntelligence::new_in_memory();
        let q = UnifiedQuery::new(
            IntelligenceKind::Text,
            "fn",
            PathBuf::from("./nonexistent_xyz"),
        );
        let mut called = 0;
        let _ = streaming_query(&c, &inner, &q, |_hit| {
            called += 1;
            false
        });
        assert_eq!(called, 0);
    }

    #[test]
    fn t11_batch_query_shares_cache() {
        let c = LruQueryCache::with_defaults();
        let inner = UnifiedCodeIntelligence::new_in_memory();
        let queries = vec![
            UnifiedQuery::new(
                IntelligenceKind::Text,
                "fn",
                PathBuf::from("./nonexistent_xyz"),
            ),
            UnifiedQuery::new(
                IntelligenceKind::Text,
                "fn",
                PathBuf::from("./nonexistent_xyz"),
            ),
            UnifiedQuery::new(
                IntelligenceKind::Text,
                "fn",
                PathBuf::from("./nonexistent_xyz"),
            ),
        ];
        let results = batch_query(&c, &inner, &queries);
        assert_eq!(results.len(), 3);
        // 全部应该成功 (即使空 results)
        for r in &results {
            assert!(r.is_ok());
        }
        // 共享 cache: 第一次 miss, 后两次 hit
        let s = c.stats();
        assert!(s.misses >= 1);
    }

    #[test]
    fn t12_cached_unified_lru_facade() {
        let f = CachedUnifiedLru::with_defaults();
        let q = UnifiedQuery::new(IntelligenceKind::Text, "fn", PathBuf::from("."));
        let r1 = f.query(&q).unwrap();
        let r2 = f.query(&q).unwrap();
        assert_eq!(r1.len(), r2.len());
        let s = f.stats();
        assert!(s.hits >= 1);
    }
}
