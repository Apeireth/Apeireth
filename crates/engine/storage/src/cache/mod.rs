//! In-process LRU + TTL + shard + eviction cache.
//!
//! Recovered from `legacy/frozen/apeireth-cache`. This is a generic process
//! cache, **not** a second memory store: it does not persist episodes, notes,
//! or history streams.
//!
//! Engine extras that are not ported:
//! - `indexmap` / `lru` crate / `quickcache` backends (new deps)
//! - Redis / Disk / Memcached backends (honest stubs in the canonical)
//! - `memory_provider` (would be a second MemoryStore)
//! - eager background TTL scanner (needs a runtime task; lazy TTL is the
//!   load-bearing algorithm)

mod evictor;
mod lru;
mod shard;
mod stats;
mod ttl;

pub use evictor::EvictionPolicy;
pub use lru::HashMapVecDequeLru;
pub use shard::{
    validate_shard_count, ShardRouter, ShardedMap, SHARD_DEFAULT, SHARD_MAX, SHARD_MIN,
};
pub use stats::{CacheStats, CacheStatsSnapshot};
pub use ttl::{TtlEntry, TtlMode, TtlPolicy};

use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use evictor::{build_evictor, Evictor};

/// Cache errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CacheError {
    /// `max_size` must be > 0.
    #[error("invalid max_size: {0}, must be > 0")]
    InvalidMaxSize(usize),
    /// TTL must be > 0.
    #[error("invalid ttl: {0:?}, must be > Duration::ZERO")]
    InvalidTtl(Duration),
    /// Shard count must be in 16..=256.
    #[error("invalid shard count: {0}, must be in 16..=256")]
    InvalidShardCount(usize),
    /// Evictor could not pick a victim at capacity.
    #[error("capacity exceeded: cache is full (max_size {max_size})")]
    CapacityExceeded {
        /// Configured max item count.
        max_size: usize,
    },
}

/// Result alias.
pub type CacheResult<T> = Result<T, CacheError>;

/// In-process cache configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheConfig {
    /// Maximum live items.
    pub max_size: usize,
    /// Default TTL used by [`MemoryCache::put_default`].
    pub default_ttl: Duration,
    /// Eviction policy.
    pub policy: EvictionPolicy,
    /// Shard count (16..=256).
    pub shards: usize,
}

impl CacheConfig {
    /// Default: 1024 items, 60s TTL, LRU, 32 shards.
    pub fn default_config() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            default_ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            policy: EvictionPolicy::Lru,
            shards: DEFAULT_SHARDS,
        }
    }

    /// Validate K-1 constraints.
    pub fn validate(&self) -> CacheResult<()> {
        if self.max_size == 0 {
            return Err(CacheError::InvalidMaxSize(0));
        }
        if self.default_ttl == Duration::ZERO {
            return Err(CacheError::InvalidTtl(Duration::ZERO));
        }
        validate_shard_count(self.shards).map_err(CacheError::InvalidShardCount)?;
        Ok(())
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Default max item count.
pub const DEFAULT_MAX_SIZE: usize = 1024;
/// Default TTL seconds.
pub const DEFAULT_TTL_SECS: u64 = 60;
/// Default shard count.
pub const DEFAULT_SHARDS: usize = 32;

/// Fluent builder.
#[derive(Debug, Clone)]
pub struct CacheBuilder {
    max_size: usize,
    default_ttl: Duration,
    policy: EvictionPolicy,
    shards: usize,
}

impl Default for CacheBuilder {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            default_ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            policy: EvictionPolicy::Lru,
            shards: DEFAULT_SHARDS,
        }
    }
}

impl CacheBuilder {
    /// Start from defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set max item count.
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// Set default TTL.
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set eviction policy.
    pub fn policy(mut self, policy: EvictionPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Set shard count.
    pub fn shards(mut self, shards: usize) -> Self {
        self.shards = shards;
        self
    }

    /// Build a config (does not construct the cache).
    pub fn build(self) -> CacheConfig {
        CacheConfig {
            max_size: self.max_size,
            default_ttl: self.default_ttl,
            policy: self.policy,
            shards: self.shards,
        }
    }
}

/// In-process sharded cache with lazy TTL and pluggable eviction.
pub struct MemoryCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    config: CacheConfig,
    shards: ShardedMap<K, TtlEntry<V>>,
    stats: CacheStats,
    evictor: Mutex<Box<dyn Evictor<K>>>,
}

impl<K, V> MemoryCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Construct after validating config.
    pub fn new(config: CacheConfig) -> CacheResult<Self> {
        config.validate()?;
        let shards = ShardedMap::new(config.shards).map_err(CacheError::InvalidShardCount)?;
        let stats = CacheStats::new(config.max_size);
        let evictor = Mutex::new(build_evictor(config.policy));
        Ok(Self {
            config,
            shards,
            stats,
            evictor,
        })
    }

    /// Immutable config.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Live stats handle.
    pub fn stats_ref(&self) -> &CacheStats {
        &self.stats
    }

    /// Get using wall clock.
    pub fn get(&self, key: &K) -> Option<V> {
        self.get_at(key, Instant::now())
    }

    /// Get at an explicit clock (tests).
    pub fn get_at(&self, key: &K, now: Instant) -> Option<V> {
        let start = Instant::now();
        match self.shards.get(key) {
            Some(e) if e.is_expired_at(now) => {
                self.shards.remove(key);
                if let Ok(mut evictor) = self.evictor.lock() {
                    evictor.on_remove(key);
                }
                self.stats.set_size(self.shards.len());
                self.stats.record_miss(start.elapsed().as_micros() as u64);
                None
            }
            Some(e) => {
                if let Ok(mut evictor) = self.evictor.lock() {
                    evictor.on_access(key);
                }
                self.stats.record_hit(start.elapsed().as_micros() as u64);
                Some(e.into_value())
            }
            None => {
                self.stats.record_miss(start.elapsed().as_micros() as u64);
                None
            }
        }
    }

    /// Put using the configured default TTL.
    pub fn put_default(&self, key: K, value: V) -> CacheResult<()> {
        self.put(key, value, self.config.default_ttl)
    }

    /// Put using wall clock.
    pub fn put(&self, key: K, value: V, ttl: Duration) -> CacheResult<()> {
        self.put_at(key, value, ttl, Instant::now())
    }

    /// Put at an explicit clock.
    pub fn put_at(&self, key: K, value: V, ttl: Duration, now: Instant) -> CacheResult<()> {
        let start = Instant::now();
        if ttl == Duration::ZERO {
            return Err(CacheError::InvalidTtl(ttl));
        }

        let replacing = self.shards.contains_key(&key);
        if !replacing && self.shards.len() >= self.config.max_size {
            let victim = {
                let mut evictor =
                    self.evictor
                        .lock()
                        .map_err(|_| CacheError::CapacityExceeded {
                            max_size: self.config.max_size,
                        })?;
                evictor.pick_victim()
            };
            if let Some(victim) = victim {
                self.shards.remove(&victim);
                if let Ok(mut evictor) = self.evictor.lock() {
                    evictor.on_remove(&victim);
                }
                self.stats.record_eviction();
            } else {
                return Err(CacheError::CapacityExceeded {
                    max_size: self.config.max_size,
                });
            }
        }

        let entry = TtlEntry::with_inserted_at(value, ttl, now);
        if replacing {
            if let Ok(mut evictor) = self.evictor.lock() {
                evictor.on_access(&key);
            }
        } else if let Ok(mut evictor) = self.evictor.lock() {
            evictor.on_insert(key.clone());
        }
        self.shards.put(key, entry);
        self.stats.set_size(self.shards.len());
        self.stats.record_put(start.elapsed().as_micros() as u64);
        Ok(())
    }

    /// Remove a key.
    pub fn remove(&self, key: &K) -> Option<V> {
        let start = Instant::now();
        let entry = self.shards.remove(key);
        if entry.is_some() {
            if let Ok(mut evictor) = self.evictor.lock() {
                evictor.on_remove(key);
            }
        }
        self.stats.set_size(self.shards.len());
        match entry {
            Some(e) => {
                self.stats.record_remove(start.elapsed().as_micros() as u64);
                Some(e.into_value())
            }
            None => None,
        }
    }

    /// Drop every entry and reset stats.
    pub fn clear(&self) {
        self.shards.clear();
        if let Ok(mut evictor) = self.evictor.lock() {
            *evictor = build_evictor(self.config.policy);
        }
        self.stats.reset();
        self.stats.set_size(0);
    }

    /// Live item count (includes not-yet-lazy-expired entries).
    pub fn len(&self) -> usize {
        self.shards.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.shards.is_empty()
    }

    /// Point-in-time stats snapshot.
    pub fn stats(&self) -> CacheStatsSnapshot {
        self.stats.set_size(self.shards.len());
        self.stats.snapshot()
    }

    /// Manually evict one victim. Used by tests.
    pub fn evict_one(&self) -> Option<K> {
        let victim = {
            let mut evictor = self.evictor.lock().ok()?;
            evictor.pick_victim()
        };
        if let Some(victim) = victim {
            self.shards.remove(&victim);
            if let Ok(mut evictor) = self.evictor.lock() {
                evictor.on_remove(&victim);
            }
            self.stats.record_eviction();
            self.stats.set_size(self.shards.len());
            Some(victim)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_builder_default() {
        let c = CacheBuilder::default().build();
        assert_eq!(c.max_size, 1024);
        assert_eq!(c.default_ttl, Duration::from_secs(60));
        assert_eq!(c.policy, EvictionPolicy::Lru);
        assert_eq!(c.shards, 32);
    }

    #[test]
    fn cache_builder_chained() {
        let config = CacheBuilder::new()
            .max_size(2048)
            .default_ttl(Duration::from_secs(120))
            .policy(EvictionPolicy::TinyLfu)
            .shards(64)
            .build();
        assert_eq!(config.max_size, 2048);
        assert_eq!(config.policy, EvictionPolicy::TinyLfu);
        assert_eq!(config.shards, 64);
    }

    #[test]
    fn memory_cache_construct_ok() {
        let config = CacheBuilder::new().max_size(100).shards(32).build();
        let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
        assert_eq!(cache.config().max_size, 100);
    }

    #[test]
    fn k1_max_size_zero_rejected() {
        let config = CacheBuilder::new().max_size(0).build();
        let r: CacheResult<MemoryCache<String, i32>> = MemoryCache::new(config);
        assert!(matches!(r, Err(CacheError::InvalidMaxSize(0))));
    }

    #[test]
    fn k1_shards_8_rejected() {
        let config = CacheBuilder::new().max_size(100).shards(8).build();
        let r: CacheResult<MemoryCache<String, i32>> = MemoryCache::new(config);
        assert!(matches!(r, Err(CacheError::InvalidShardCount(8))));
    }

    #[test]
    fn k1_default_ttl_zero_rejected() {
        let config = CacheConfig {
            max_size: 100,
            default_ttl: Duration::ZERO,
            policy: EvictionPolicy::Lru,
            shards: 32,
        };
        let r: CacheResult<MemoryCache<String, i32>> = MemoryCache::new(config);
        assert!(matches!(r, Err(CacheError::InvalidTtl(_))));
    }

    fn small_lru() -> MemoryCache<String, i32> {
        let config = CacheBuilder::new()
            .max_size(2)
            .default_ttl(Duration::from_secs(60))
            .policy(EvictionPolicy::Lru)
            .shards(16)
            .build();
        MemoryCache::new(config).unwrap()
    }

    #[test]
    fn memory_cache_lru_evicts_one_when_over_capacity() {
        let cache = small_lru();
        cache.put("a".into(), 1, Duration::from_secs(60)).unwrap();
        cache.put("b".into(), 2, Duration::from_secs(60)).unwrap();
        assert_eq!(cache.len(), 2);
        cache.put("c".into(), 3, Duration::from_secs(60)).unwrap();
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&"a".into()).is_none());
        assert_eq!(cache.get(&"b".into()), Some(2));
        assert_eq!(cache.get(&"c".into()), Some(3));
    }

    #[test]
    fn memory_cache_lfu_evicts_least_frequent() {
        let config = CacheBuilder::new()
            .max_size(2)
            .policy(EvictionPolicy::Lfu)
            .shards(16)
            .build();
        let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
        cache.put("a".into(), 1, Duration::from_secs(60)).unwrap();
        cache.put("b".into(), 2, Duration::from_secs(60)).unwrap();
        cache.get(&"a".into());
        cache.get(&"a".into());
        cache.put("c".into(), 3, Duration::from_secs(60)).unwrap();
        assert!(cache.get(&"b".into()).is_none());
        assert_eq!(cache.get(&"a".into()), Some(1));
        assert_eq!(cache.get(&"c".into()), Some(3));
    }

    #[test]
    fn memory_cache_fifo_ignores_access() {
        let config = CacheBuilder::new()
            .max_size(2)
            .policy(EvictionPolicy::Fifo)
            .shards(16)
            .build();
        let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
        cache.put("a".into(), 1, Duration::from_secs(60)).unwrap();
        cache.put("b".into(), 2, Duration::from_secs(60)).unwrap();
        cache.get(&"a".into());
        cache.put("c".into(), 3, Duration::from_secs(60)).unwrap();
        assert!(cache.get(&"a".into()).is_none());
        assert_eq!(cache.get(&"b".into()), Some(2));
    }

    #[test]
    fn ttl_lazy_expiration() {
        let now = Instant::now();
        let cache = small_lru();
        cache
            .put_at("a".into(), 1, Duration::from_millis(10), now)
            .unwrap();
        assert_eq!(cache.get_at(&"a".into(), now), Some(1));
        let later = now + Duration::from_millis(20);
        assert!(cache.get_at(&"a".into(), later).is_none());
    }

    #[test]
    fn stats_hit_miss_and_zero_rate() {
        let cache = small_lru();
        assert_eq!(cache.stats().hit_rate, 0.0);
        cache.put("a".into(), 1, Duration::from_secs(60)).unwrap();
        assert_eq!(cache.get(&"a".into()), Some(1));
        assert!(cache.get(&"missing".into()).is_none());
        let snap = cache.stats();
        assert_eq!(snap.hit, 1);
        assert_eq!(snap.miss, 1);
        assert!((snap.hit_rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn clear_resets_stats() {
        let cache = small_lru();
        cache.put("a".into(), 1, Duration::from_secs(60)).unwrap();
        cache.get(&"a".into());
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.stats().hit, 0);
        assert_eq!(cache.stats().put_count, 0);
    }

    #[test]
    fn put_zero_ttl_rejected() {
        let cache = small_lru();
        let r = cache.put("a".into(), 1, Duration::ZERO);
        assert!(matches!(r, Err(CacheError::InvalidTtl(_))));
    }

    #[test]
    fn remove_existing_and_missing() {
        let cache = small_lru();
        cache.put("a".into(), 1, Duration::from_secs(60)).unwrap();
        assert_eq!(cache.remove(&"a".into()), Some(1));
        assert_eq!(cache.remove(&"a".into()), None);
        assert_eq!(cache.len(), 0);
    }
}
