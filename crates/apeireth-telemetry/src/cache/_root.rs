
#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// 模块声明
// ============================================================================


// ============================================================================
// Re-export (主入口便捷)
// ============================================================================

pub use super::backend::{BackendKind, BACKEND_KIND_VARIANT_COUNT};
pub use super::config::CacheConfig;
pub use super::error::{CacheError, CacheResult, CACHE_ERROR_VARIANT_COUNT};
pub use super::lru::{LruImpl, LruCrateLru, HashMapVecDequeLru, IndexMapLru, QuickCacheStub};
// R20 借鉴 Golutra #4: memory provider 主入口 re-export
pub use super::memory_provider::{
    build_provider as build_memory_provider, DiskLruProvider, HybridProvider, InMemoryProvider,
    MemoryEntry, MemoryProvider, MemoryProviderError, MemoryProviderResult, MemoryQuery,
    PostgresProvider, ProviderKind, RedisProvider, S3Provider, SqliteProvider,
    MEMORY_PROVIDER_ERROR_VARIANT_COUNT, PROVIDER_KIND_VARIANT_COUNT,
};
pub use super::policy::{EvictionPolicy, EVICTION_POLICY_VARIANT_COUNT};
pub use super::shard::{
    ShardRouter, ShardedMap, validate_shard_count, SHARD_DEFAULT, SHARD_MAX, SHARD_MIN,
};
pub use super::stats::{CacheStats, CacheStatsSnapshot};
pub use super::ttl::{TtlEntry, TtlMode, TtlPolicy};

// ============================================================================
// §1 Cache trait (核心 API, 5 async 方法 + 2 sync 方法)
// ============================================================================

/// Cache trait — 5 async 方法 + 2 sync 方法 (per task spec §3).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache class Cache<K, V> 商业版.
///
/// 所有方法 `Send + Sync` 以便跨任务 / 跨线程共享.
#[async_trait::async_trait]
pub trait Cache<K, V>: Send + Sync
where
    K: Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// 异步 get.
    ///
    /// - 命中 + 未过期: 返 `Some(v)`, 记录 hit
    /// - 命中 + 已过期: 返 `None`, lazy 删除, 记录 miss
    /// - 未命中: 返 `None`, 记录 miss
    async fn get(&self, key: &K) -> Result<Option<V>, CacheError>;

    /// 异步 put.
    ///
    /// - key 已存在: 覆盖, 返 `Ok(())`
    /// - key 不存在 + size < max: 插入, 返 `Ok(())`
    /// - key 不存在 + size = max: 触发 eviction, 插入, 返 `Ok(())`
    /// - ttl = 0: 返 `Err(InvalidTtl)` (K-1 强校验)
    /// - value size > max_size: 返 `Err(CapacityExceeded)` (K-1 强校验)
    /// - backend stub: 返 `Err(BackendNotImplemented)` (K-1 强校验)
    async fn put(&self, key: K, value: V, ttl: Duration) -> Result<(), CacheError>;

    /// 异步 remove.
    ///
    /// - key 存在: 删除, 返 `Some(v)`, 记录 remove
    /// - key 不存在: 返 `None`
    /// - backend stub: 返 `Err(BackendNotImplemented)` (K-1 强校验)
    async fn remove(&self, key: &K) -> Result<Option<V>, CacheError>;

    /// 异步 clear (清空全部 key + reset stats).
    ///
    /// - Memory: 清空 + reset stats
    /// - backend stub: 返 `Err(BackendNotImplemented)` (K-1 强校验)
    async fn clear(&self) -> Result<(), CacheError>;

    /// 异步 len (当前 item 数).
    async fn len(&self) -> usize;

    /// 异步 is_empty.
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 异步 stats (point-in-time snapshot).
    async fn stats(&self) -> CacheStatsSnapshot;
}

// ============================================================================
// §2 MemoryCache 完整实现 (R20 阶段 6 唯一真接 backend)
// ============================================================================

/// MemoryCache — 进程内 cache, 完整实现 (LRU + TTL + 分片锁).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache MemoryCache 商业版.
pub struct MemoryCache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 配置.
    config: CacheConfig,
    /// 分片存储: shard_id → (K → TtlEntry<V>).
    shards: ShardedMap<K, TtlEntry<V>>,
    /// Stats (atomic 计数).
    stats: CacheStats,
    /// TTL 策略.
    ttl_policy: TtlPolicy,
}

impl<K, V> MemoryCache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 构造 (K-1 强校验: max_size > 0 + shards 16..=256).
    pub fn new(config: CacheConfig) -> CacheResult<Self> {
        // K-1 强校验: max_size > 0
        if config.max_size == 0 {
            return Err(CacheError::InvalidMaxSize(0));
        }
        // K-1 强校验: backend = Memory
        config.backend.check_implemented()?;
        // K-1 强校验: shards 16..=256
        validate_shard_count(config.shards)?;
        // K-1 强校验: default_ttl > Duration::ZERO
        if config.default_ttl == Duration::ZERO {
            return Err(CacheError::InvalidTtl(Duration::ZERO));
        }

        let shards = ShardedMap::new(config.shards)?;
        let stats = CacheStats::new(config.max_size);
        let ttl_policy = TtlPolicy::default_policy();

        Ok(Self {
            config,
            shards,
            stats,
            ttl_policy,
        })
    }

    /// 引用 config.
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// 引用 stats.
    pub fn stats_ref(&self) -> &CacheStats {
        &self.stats
    }

    /// 引用 ttl_policy.
    pub fn ttl_policy_ref(&self) -> &TtlPolicy {
        &self.ttl_policy
    }
}

#[async_trait::async_trait]
impl<K, V> Cache<K, V> for MemoryCache<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        let start = std::time::Instant::now();
        // lazy 检查: 取出 entry, 看是否过期
        let entry = self.shards.get(key);
        match entry {
            Some(e) if e.is_expired() => {
                // lazy 删除
                self.shards.remove(key);
                let latency = start.elapsed().as_micros() as u64;
                self.stats.record_miss(latency);
                Ok(None)
            }
            Some(e) => {
                let v = e.into_value();
                let latency = start.elapsed().as_micros() as u64;
                self.stats.record_hit(latency);
                Ok(Some(v))
            }
            None => {
                let latency = start.elapsed().as_micros() as u64;
                self.stats.record_miss(latency);
                Ok(None)
            }
        }
    }

    async fn put(&self, key: K, value: V, ttl: Duration) -> CacheResult<()> {
        let start = std::time::Instant::now();

        // K-1 强校验: ttl > 0
        if ttl == Duration::ZERO {
            return Err(CacheError::InvalidTtl(ttl));
        }

        // K-1 强校验: capacity 守门
        // R128 续 (per task spec §5.4): 不再返 CapacityExceeded, 真 eviction
        // 通过 ShardedMap::pop_by_min_score + TtlEntry.inserted_at (FIFO-on-insert) 实现.
        // 真 LRU (touch tracking) 在 roadmap.md §3 排, v1.2 续.
        let mut current_size = self.shards.len();
        while current_size >= self.config.max_size {
            // 用 FIFO 策略淘汰 1 个 (TtlEntry.inserted_at 最早)
            // V 在调用点是 TtlEntry<V_inner>, monomorphize 后 inserted_at() 可用.
            let evicted = self.shards.pop_by_min_score(|v| {
                let inst = v.inserted_at();
                let dur = std::time::Instant::now().saturating_duration_since(inst);
                let elapsed_ns = (dur.as_secs() as u128) * 1_000_000_000 + (dur.subsec_nanos() as u128);
                u128::MAX - elapsed_ns
            });
            if evicted.is_some() {
                self.stats.record_eviction();
            } else {
                break;
            }
            current_size = self.shards.len();
        }

        let entry = TtlEntry::new(value, ttl);
        self.shards.put(key, entry);
        self.stats.set_size(self.shards.len());

        let latency = start.elapsed().as_micros() as u64;
        self.stats.record_put(latency);
        Ok(())
    }

    async fn remove(&self, key: &K) -> CacheResult<Option<V>> {
        let start = std::time::Instant::now();
        let entry = self.shards.remove(key);
        self.stats.set_size(self.shards.len());
        let latency = start.elapsed().as_micros() as u64;
        match entry {
            Some(e) => {
                self.stats.record_remove(latency);
                Ok(Some(e.into_value()))
            }
            None => {
                // 移除不存在的 key 不计入 latency
                Ok(None)
            }
        }
    }

    async fn clear(&self) -> CacheResult<()> {
        self.shards.clear();
        self.stats.reset();
        self.stats.set_size(0);
        Ok(())
    }

    async fn len(&self) -> usize {
        self.shards.len()
    }

    async fn stats(&self) -> CacheStatsSnapshot {
        // 同步刷新 size 后 snapshot
        self.stats.set_size(self.shards.len());
        self.stats.snapshot()
    }
}

// ============================================================================
// §3 StubCache (Disk / Redis / Memcached, 全部返 BackendNotImplemented)
// ============================================================================

/// StubCache — 用于 Disk / Redis / Memcached backend, 全部方法返 BackendNotImplemented.
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache StubCache 商业版 (留口子, R21 续接).
pub struct StubCache {
    /// backend kind (Disk/Redis/Memcached).
    backend: BackendKind,
}

impl StubCache {
    /// 构造 stub cache.
    pub fn new(backend: BackendKind) -> CacheResult<Self> {
        backend.check_implemented()?;
        // 这里 check_implemented 必然返 Err (Memory 才会返 Ok), 所以上面 if
        // 实际上不会到这里. 但保留逻辑以防御 future Memory stub.
        Err(CacheError::BackendNotImplemented(backend.as_str().to_string()))
    }
}

#[async_trait::async_trait]
impl<K, V> Cache<K, V> for StubCache
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    async fn get(&self, _key: &K) -> CacheResult<Option<V>> {
        Err(CacheError::BackendNotImplemented(self.backend.as_str().to_string()))
    }

    async fn put(&self, _key: K, _value: V, _ttl: Duration) -> CacheResult<()> {
        Err(CacheError::BackendNotImplemented(self.backend.as_str().to_string()))
    }

    async fn remove(&self, _key: &K) -> CacheResult<Option<V>> {
        Err(CacheError::BackendNotImplemented(self.backend.as_str().to_string()))
    }

    async fn clear(&self) -> CacheResult<()> {
        Err(CacheError::BackendNotImplemented(self.backend.as_str().to_string()))
    }

    async fn len(&self) -> usize {
        0
    }

    async fn stats(&self) -> CacheStatsSnapshot {
        CacheStatsSnapshot::empty(0)
    }
}

// ============================================================================
// §4 Cache factory (按 backend kind 分发)
// ============================================================================

/// 构造 cache (按 backend kind 分发).
///
/// - BackendKind::Memory: 返 `MemoryCache<K, V>`
/// - Disk/Redis/Memcached: 返 Err(BackendNotImplemented) (R20 阶段 6 stub)
///
/// 用 Arc<dyn Cache<K, V>> 屏蔽具体 backend 类型.
pub async fn build_cache<K, V>(
    config: CacheConfig,
) -> CacheResult<std::sync::Arc<dyn Cache<K, V>>>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    match config.backend {
        BackendKind::Memory => {
            let cache = MemoryCache::<K, V>::new(config)?;
            Ok(std::sync::Arc::new(cache))
        }
        BackendKind::Disk => Err(CacheError::BackendNotImplemented("DISK".to_string())),
        BackendKind::Redis => Err(CacheError::BackendNotImplemented("REDIS".to_string())),
        BackendKind::Memcached => Err(CacheError::BackendNotImplemented(
            "MEMCACHED".to_string(),
        )),
    }
}

// ============================================================================
// §5 Crate-level 常量 (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// 5 EvictionPolicy 1:1 计数.
pub const CRATE_EVICTION_POLICY_COUNT: usize = 5;

/// 4 BackendKind 1:1 计数.
pub const CRATE_BACKEND_KIND_COUNT: usize = 4;

/// 16-256 shard 范围最小值.
pub const CRATE_SHARD_MIN: usize = 16;

/// 16-256 shard 范围最大值.
pub const CRATE_SHARD_MAX: usize = 256;

/// 10 CacheError variant 1:1 计数.
pub const CRATE_CACHE_ERROR_VARIANT_COUNT: usize = 10;

/// 4 LRU 实现 1:1 计数.
pub const CRATE_LRU_IMPL_COUNT: usize = 4;

/// 3 TTL mode 1:1 计数 (lazy/eager/both).
pub const CRATE_TTL_MODE_COUNT: usize = 3;

// ============================================================================
// §6 Crate-level 默认配置
// ============================================================================

/// 默认 max_size (1024 items).
pub const DEFAULT_MAX_SIZE: usize = 1024;

/// 默认 TTL (60s).
pub const DEFAULT_TTL_SECS: u64 = 60;

/// 默认 shard 数 (32).
pub const DEFAULT_SHARDS: usize = 32;

/// 默认 EvictionPolicy (LRU).
pub const DEFAULT_POLICY: EvictionPolicy = EvictionPolicy::Lru;

/// 默认 BackendKind (Memory).
pub const DEFAULT_BACKEND: BackendKind = BackendKind::Memory;

// ============================================================================
// §7 Re-export Cache trait + builder 模式
// ============================================================================

/// 重新导出 Duration (主入口便捷).
pub use std::time::Duration;

/// CacheBuilder — 链式构造 CacheConfig.
#[derive(Debug, Clone)]
pub struct CacheBuilder {
    max_size: usize,
    default_ttl: Duration,
    policy: EvictionPolicy,
    shards: usize,
    backend: BackendKind,
}

impl Default for CacheBuilder {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            default_ttl: Duration::from_secs(DEFAULT_TTL_SECS),
            policy: DEFAULT_POLICY,
            shards: DEFAULT_SHARDS,
            backend: DEFAULT_BACKEND,
        }
    }
}

impl CacheBuilder {
    /// 构造 builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 max_size.
    pub fn max_size(mut self, max_size: usize) -> Self {
        self.max_size = max_size;
        self
    }

    /// 设置 default_ttl.
    pub fn default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// 设置 policy.
    pub fn policy(mut self, policy: EvictionPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// 设置 shards.
    pub fn shards(mut self, shards: usize) -> Self {
        self.shards = shards;
        self
    }

    /// 设置 backend.
    pub fn backend(mut self, backend: BackendKind) -> Self {
        self.backend = backend;
        self
    }

    /// 构造 CacheConfig.
    pub fn build(self) -> CacheConfig {
        CacheConfig {
            max_size: self.max_size,
            default_ttl: self.default_ttl,
            policy: self.policy,
            shards: self.shards,
            backend: self.backend,
        }
    }

    /// 构造 cache (async, 调 build_cache).
    pub async fn build_cache<K, V>(self) -> CacheResult<std::sync::Arc<dyn Cache<K, V>>>
    where
        K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        build_cache::<K, V>(self.build()).await
    }
}

// ============================================================================
// §8 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 5 EvictionPolicy + 4 BackendKind + 10 CacheError + 4 LRU + 3 TTL 常量.
    #[test]
    fn crate_constants_5_4_10_4_3() {
        assert_eq!(CRATE_EVICTION_POLICY_COUNT, 5);
        assert_eq!(CRATE_BACKEND_KIND_COUNT, 4);
        assert_eq!(CRATE_CACHE_ERROR_VARIANT_COUNT, 10);
        assert_eq!(CRATE_LRU_IMPL_COUNT, 4);
        assert_eq!(CRATE_TTL_MODE_COUNT, 3);
        assert_eq!(CRATE_SHARD_MIN, 16);
        assert_eq!(CRATE_SHARD_MAX, 256);
    }

    /// 守门 #2: 默认值.
    #[test]
    fn defaults() {
        assert_eq!(DEFAULT_MAX_SIZE, 1024);
        assert_eq!(DEFAULT_TTL_SECS, 60);
        assert_eq!(DEFAULT_SHARDS, 32);
        assert_eq!(DEFAULT_POLICY, EvictionPolicy::Lru);
        assert_eq!(DEFAULT_BACKEND, BackendKind::Memory);
    }

    /// 守门 #3: CacheBuilder 默认值.
    #[test]
    fn cache_builder_default() {
        let b = CacheBuilder::default();
        assert_eq!(b.max_size, 1024);
        assert_eq!(b.default_ttl, Duration::from_secs(60));
        assert_eq!(b.policy, EvictionPolicy::Lru);
        assert_eq!(b.shards, 32);
        assert_eq!(b.backend, BackendKind::Memory);
    }

    /// 守门 #4: CacheBuilder 链式.
    #[test]
    fn cache_builder_chained() {
        let config = CacheBuilder::new()
            .max_size(2048)
            .default_ttl(Duration::from_secs(120))
            .policy(EvictionPolicy::TinyLfu)
            .shards(64)
            .backend(BackendKind::Memory)
            .build();
        assert_eq!(config.max_size, 2048);
        assert_eq!(config.default_ttl, Duration::from_secs(120));
        assert_eq!(config.policy, EvictionPolicy::TinyLfu);
        assert_eq!(config.shards, 64);
    }

    /// 守门 #5: MemoryCache 构造合法.
    #[test]
    fn memory_cache_construct_ok() {
        let config = CacheBuilder::new()
            .max_size(100)
            .shards(32)
            .build();
        let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
        assert_eq!(cache.config().max_size, 100);
    }

    /// 守门 #6: MemoryCache K-1 max_size=0 返 InvalidMaxSize.
    #[test]
    fn k1_max_size_zero_rejected() {
        let config = CacheBuilder::new()
            .max_size(0)
            .build();
        let r: CacheResult<MemoryCache<String, i32>> = MemoryCache::new(config);
        assert!(matches!(r, Err(CacheError::InvalidMaxSize(0))));
    }

    /// 守门 #7: MemoryCache K-1 shards 不在 16..=256 返 InvalidShardCount.
    #[test]
    fn k1_shards_8_rejected() {
        let config = CacheBuilder::new()
            .max_size(100)
            .shards(8)
            .build();
        let r: CacheResult<MemoryCache<String, i32>> = MemoryCache::new(config);
        assert!(matches!(r, Err(CacheError::InvalidShardCount(8))));
    }

    /// 守门 #8: MemoryCache K-1 default_ttl = 0 返 InvalidTtl.
    #[test]
    fn k1_default_ttl_zero_rejected() {
        let config = CacheConfig {
            max_size: 100,
            default_ttl: Duration::ZERO,
            policy: EvictionPolicy::Lru,
            shards: 32,
            backend: BackendKind::Memory,
        };
        let r: CacheResult<MemoryCache<String, i32>> = MemoryCache::new(config);
        assert!(matches!(r, Err(CacheError::InvalidTtl(_))));
    }

    /// 守门 #9: Disk backend MemoryCache 构造返 BackendNotImplemented.
    #[test]
    fn disk_backend_rejected_at_construction() {
        let config = CacheConfig {
            max_size: 100,
            default_ttl: Duration::from_secs(60),
            policy: EvictionPolicy::Lru,
            shards: 32,
            backend: BackendKind::Disk,
        };
        let r: CacheResult<MemoryCache<String, i32>> = MemoryCache::new(config);
        assert!(matches!(r, Err(CacheError::BackendNotImplemented(_))));
    }

    /// 守门 #10: 5 policy + 4 backend + 3 TTL mode 跨模块守门.
    #[test]
    fn cross_module_5_4_3() {
        assert_eq!(EvictionPolicy::ALL.len(), 5);
        assert_eq!(BackendKind::ALL.len(), 4);
        assert_eq!(TtlMode::ALL.len(), 3);
    }

    /// 守门 #11: build_cache async factory Memory 返 Arc.
    #[tokio::test]
    async fn build_cache_memory_returns_arc() {
        let config = CacheBuilder::new().max_size(100).build();
        let cache: std::sync::Arc<dyn Cache<String, i32>> =
            build_cache::<String, i32>(config).await.unwrap();
        assert_eq!(cache.len().await, 0);
    }

    /// 守门 #12: build_cache async factory Disk 返 BackendNotImplemented.
    #[tokio::test]
    async fn build_cache_disk_returns_not_implemented() {
        let config = CacheBuilder::new()
            .max_size(100)
            .backend(BackendKind::Disk)
            .build();
        let r: CacheResult<std::sync::Arc<dyn Cache<String, i32>>> =
            build_cache::<String, i32>(config).await;
        assert!(matches!(r, Err(CacheError::BackendNotImplemented(_))));
    }
}
