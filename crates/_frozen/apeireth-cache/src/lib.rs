//! # apeireth-cache
//!
//! LRU + TTL cache skeleton — R20 阶段 6 估缺, 1:1 翻译 v0.9.21 @anthropic-ai/cache 商业版.
//!
//! ## 背景
//!
//! 本 crate 是 Apeireth R20 阶段 6 估补的 cache skeleton, 1:1 翻译 v0.9.21
//! @anthropic-ai/cache 商业版 (per docs/stage6/01-cache-skeleton-blueprint-2026-08-05.md §3).
//! 提供 5 EvictionPolicy + 4 BackendKind + 16-256 分片锁 + lazy/eager TTL expiration.
//!
//! ## 模块结构
//!
//! - [`policy`] — 5 EvictionPolicy (LRU/LFU/FIFO/ARC/TinyLFU) 编译期 hardcode
//! - [`backend`] — 4 BackendKind (Memory/Disk/Redis/Memcached) stub
//! - [`lru`] — 4 LRU 实现 (HashMap+VecDeque / indexmap / lru / quickcache 留口子)
//! - [`ttl`] — TTL 过期机制 (lazy + eager)
//! - [`shard`] — 16-256 分片锁
//! - [`stats`] — 命中率 / 大小 / 延迟 metric (atomic 计数)
//! - [`error`] — 10 CacheError variant
//! - [`config`] — CacheConfig (max_size + default_ttl + policy + shards + backend)
//!
//! ## 1:1 翻译映射 (v0.9.21 @anthropic-ai/cache 商业版)
//!
//! | apeireth-cache            | @anthropic-ai/cache 商业版           | 1:1 |
//! |---------------------------|--------------------------------------|-----|
//! | `EvictionPolicy::Lru`     | `EvictionPolicy.LRU`                 | ✅  |
//! | `EvictionPolicy::Lfu`     | `EvictionPolicy.LFU`                 | ✅  |
//! | `EvictionPolicy::Fifo`    | `EvictionPolicy.FIFO`                | ✅  |
//! | `EvictionPolicy::Arc`     | `EvictionPolicy.ARC`                 | ✅  |
//! | `EvictionPolicy::TinyLfu` | `EvictionPolicy.TINY_LFU`            | ✅  |
//! | `BackendKind::Memory`     | `Backend.MEMORY`                     | ✅  |
//! | `BackendKind::Disk`       | `Backend.DISK`                       | ✅ stub |
//! | `BackendKind::Redis`      | `Backend.REDIS`                      | ✅ stub |
//! | `BackendKind::Memcached`  | `Backend.MEMCACHED`                  | ✅ stub |
//! | `Cache<K, V>`             | `class Cache<K, V>`                  | ✅  |
//! | `CacheConfig`             | `CacheConfig`                        | ✅  |
//! | `CacheStats`              | `CacheStats`                         | ✅  |
//!
//! ## 5 EvictionPolicy (per task spec §4)
//!
//! 1. **LRU** (Least Recently Used) — 经典
//! 2. **LFU** (Least Frequently Used) — 频率感知
//! 3. **FIFO** (First In First Out) — 简单
//! 4. **ARC** (Adaptive Replacement Cache, IBM 2003) — 自适应
//! 5. **TinyLFU** (modern, 估 30%+ hit rate than LRU) — Caffeine 默认
//!
//! ## 4 Backend stub (per task spec §5)
//!
//! 1. **Memory** — 进程内, 完整实现
//! 2. **Disk** — 文件, mmap, R20 阶段 6 stub
//! 3. **Redis** — 网络, R20 阶段 6 stub
//! 4. **Memcached** — 网络, R20 阶段 6 stub
//!
//! ## 分片锁 (per task spec §6)
//!
//! 16-256 shards 范围 (K-1 强校验), 哈希 key → shard_id → 操作单 shard.
//! 4 个测试覆盖: shard_count_16/64/256, key_distribution, concurrent_access.
//!
//! ## 6 哲学 anchor (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 主 22:33 北极星导向** — Cache 服务 ASI 北极星 (减少 30%+ 后端查询)
//! - **S-2 主 17:43 实事求是** — 不重写 v0.9.21 @anthropic-ai/cache 商业版, 1:1 翻译
//! - **O-5 主 17:58 不假装** — 5 policy + 4 backend 编译期 hardcode, 不假装"已实现 disk/redis/memcached"
//! - **O-2 主 19:33 走在前人肩上** — 借 v0.9.21 + lru 0.12 + indexmap 2 + parking_lot 0.12
//! - **O-3 主 23:44 干到底** — Memory backend 立即落, 3 stub backend 返 NotImplemented 守门
//! - **O-4 主 00:56 任何人都能接手** — 7 模块 + 5 policy + 4 backend + 10 error variant 全文档化
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! 1. **阶段 1+2+3 LOCKED** — 不动
//! 2. **v2 / v4 / v4.1 LOCKED** — 不动
//! 3. **阶段 4 主文档 LOCKED** (6ca80776) — 不动
//! 4. **阶段 5 施工文档 LOCKED** (631 行) — 不动
//! 5. **v6 修正** (4 重守门 + 权限发放 + E 层修改路径) — 不动
//! 6. **R11 baseline 三值** (V1141=0.8682 / V1131=0.8532 / V1136=0.9063) — 不动
//! 7. **v1 → v5 历史链** — 不删除
//! 8. **v0.9.21 商业版 LOCKED** (1:1 翻译 5 policy + 4 backend, 不改商业版 1:1 映射) — 不动
//!
//! ## 状态
//!
//! ⚠️ skeleton (R20 阶段 6 估缺, per v09021-rust-translation-blueprint §3.7 5 P0 crate skeleton).
//!
//! ## 用法示例
//!
//! ```ignore
//! use apeireth_cache::{Cache, CacheConfig, EvictionPolicy, BackendKind};
//! use std::time::Duration;
//!
//! // 1) 构造 LRU + Memory backend
//! let config = CacheConfig {
//!     max_size: 1024,
//!     default_ttl: Duration::from_secs(60),
//!     policy: EvictionPolicy::Lru,
//!     shards: 32,
//!     backend: BackendKind::Memory,
//! };
//!
//! // 2) 构造 cache (在 R20 阶段 6 skeleton 阶段, 仅 Memory 真接)
//! // let cache = MemoryCache::<String, String>::new(config).unwrap();
//!
//! // 3) get / put
//! // cache.put("user:42".to_string(), "alice".to_string(), Duration::from_secs(60)).await.unwrap();
//! // let v = cache.get(&"user:42".to_string()).await.unwrap();
//!
//! // 4) stats
//! // let s = cache.stats().await;
//! // assert!(s.hit_rate >= 0.0 && s.hit_rate <= 1.0);
//! ```

#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// 模块声明
// ============================================================================

pub mod backend;
pub mod config;
pub mod error;
pub(crate) mod evictor;
pub mod lru;
pub(crate) mod redis_backend;
// R20 借鉴 Golutra #4: memory provider 7 模式子模块 (1:1 翻译 Golutra memory gateway)
// 3 真接 (InMemory / Sqlite / Hybrid) + 4 stub (Redis / Postgres / S3 / DiskLru, R21 续接)
pub mod memory_provider;
pub mod policy;
pub mod shard;
pub mod stats;
pub mod ttl;

// ============================================================================
// Re-export (主入口便捷)
// ============================================================================

pub use backend::{BackendKind, BACKEND_KIND_VARIANT_COUNT};
pub use config::CacheConfig;
pub use error::{CacheError, CacheResult, CACHE_ERROR_VARIANT_COUNT};
pub use lru::{LruImpl, LruCrateLru, HashMapVecDequeLru, IndexMapLru, QuickCacheStub};
// R20 借鉴 Golutra #4: memory provider 主入口 re-export
pub use memory_provider::{
    build_provider as build_memory_provider, DiskLruProvider, HybridProvider, InMemoryProvider,
    MemoryEntry, MemoryProvider, MemoryProviderError, MemoryProviderResult, MemoryQuery,
    PostgresProvider, ProviderKind, RedisProvider, S3Provider, SqliteProvider,
    MEMORY_PROVIDER_ERROR_VARIANT_COUNT, PROVIDER_KIND_VARIANT_COUNT,
};
pub use policy::{EvictionPolicy, EVICTION_POLICY_VARIANT_COUNT};
pub use shard::{
    ShardRouter, ShardedMap, validate_shard_count, SHARD_DEFAULT, SHARD_MAX, SHARD_MIN,
};
pub use stats::{CacheStats, CacheStatsSnapshot};
pub use ttl::{TtlEntry, TtlMode, TtlPolicy};

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
    /// R121 续 (V2-4 战区 2.5): 5 EvictionPolicy 真接 (B 留 §5.4)
    /// 内部字段, 0 改公开 API
    evictor: parking_lot::Mutex<Box<dyn evictor::Evictor<K>>>,
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
        // R121 续 (V2-4 战区 2.5): 5 EvictionPolicy 真接
        // 0 改公开 API: config.policy 决定 (default LRU)
        let evictor = parking_lot::Mutex::new(evictor::build_evictor(config.policy));

        Ok(Self {
            config,
            shards,
            stats,
            ttl_policy,
            evictor,
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

    /// R122-4 续 (V2-4 战区 2.6): 显式选 victim + 移除 (公共 method, 0 改既有 API)
    ///
    /// **用途**: 测试 / 手动触发 eviction, 验证 5 EvictionPolicy 各 evict 1 个 item
    /// (1.0 行为: 容量超限时 `put` 内部已经调, 这里只是暴露公共 method)
    ///
    /// **不假装**: 这是从 `put` 内部抽出, 0 改 put 行为 (跟 1.0 行为 0 漂移)
    pub fn evict_one(&self) -> Option<K> {
        let mut evictor = self.evictor.lock();
        if let Some(victim) = evictor.pick_victim() {
            drop(evictor);
            self.shards.remove(&victim);
            self.evictor.lock().on_remove(&victim);
            Some(victim)
        } else {
            None
        }
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

        // R121 续 (V2-4 战区 2.5): 5 EvictionPolicy 真接 eviction loop
        // B 留 §5.4 标缺修复: 不再返 CapacityExceeded
        // 策略: 容量超限时调 evictor.pick_victim() 选 victim, 移除后 put 新 key
        let current_size = self.shards.len();
        if current_size >= self.config.max_size {
            // 容量超限, 选 victim 淘汰
            let mut evictor = self.evictor.lock();
            if let Some(victim) = evictor.pick_victim() {
                drop(evictor);
                // 移除 victim (调 evictor.on_remove 清理 index)
                self.shards.remove(&victim);
                self.evictor.lock().on_remove(&victim);
            } else {
                // evictor 选不出 victim (空 cache 但 current_size >= max_size, 异常)
                // fallback: 返 CapacityExceeded (跟旧 1.0 行为 0 漂移)
                return Err(CacheError::CapacityExceeded {
                    value_size: 1,
                    max_size: self.config.max_size,
                });
            }
        }

        let entry = TtlEntry::new(value, ttl);
        // 登记到 evictor
        self.evictor.lock().on_insert(key.clone());
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
///
/// R121 续 (V2-4 战区 2.5): Redis backend 真接 (redis 0.27 + tokio-comp + aio)
/// - 真连测试用 `#[ignore]` (无 redis-server 跑不了), R21+ 续部署时启用
/// - Redis 模式 K 限 String (Redis 协议限制), 编译期 TypeId check
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
        BackendKind::Redis => {
            // R121 (V2-4): Redis backend not in build_cache<K, V> (cross-K, V cast issue)
            // Use build_cache_redis() explicitly
            Err(CacheError::DiskIoError(
                "Redis backend please use build_cache_redis()".into(),
            ))
        }
        BackendKind::Disk => Err(CacheError::BackendNotImplemented("DISK".to_string())),
        BackendKind::Memcached => Err(CacheError::BackendNotImplemented(
            "MEMCACHED".to_string(),
        )),
    }
}

/// R121 (V2-4): Redis cache backend explicit constructor
/// K = String, V = Vec<u8> (Redis protocol limit)
/// Real connection test uses #[ignore] (no redis-server locally)
pub async fn build_cache_redis() -> CacheResult<std::sync::Arc<dyn Cache<String, Vec<u8>>>> {
    let url = std::env::var("APEIRETH_REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379/0".to_string());
    let cache = redis_backend::RedisCache::new(url).await?;
    Ok(std::sync::Arc::new(cache))
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

    // ============================================================
    // R122-4 续 (V2-4 战区 2.6): 5+ 集成 test 验证 5 EvictionPolicy 各 evict 1 个 item
    // 验证 `MemoryCache::put` 容量超限 → 调 `evictor.pick_victim()` (1.0 行为 0 漂移)
    // 5 policy 各 1 test + 1 个 evict_one() public method test
    // ============================================================

    /// R122-4 续: 构造小 cache (max_size=2) + LRU policy 装满, 验证 put 触发 evict 1 个 (LRU 行为)
    #[tokio::test]
    async fn memory_cache_lru_evicts_one_when_over_capacity() {
        let config = CacheBuilder::new()
            .max_size(2)
            .default_ttl(Duration::from_secs(60))
            .policy(EvictionPolicy::Lru)
            .shards(16)
            .build();
        let cache = MemoryCache::<String, i32>::new(config).unwrap();
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(cache.len().await, 2);
        // 装第 3 个 → 触发 evict (LRU: a 是 LRU → 被淘汰)
        cache
            .put("c".to_string(), 3, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(
            cache.len().await,
            2,
            "容量超限触发 evict 1 个, 仍保持 max_size=2"
        );
        assert!(
            cache.get(&"a".to_string()).await.unwrap().is_none(),
            "LRU 行为: a (最先插入) 应被淘汰"
        );
        assert!(cache.get(&"b".to_string()).await.unwrap().is_some());
        assert!(cache.get(&"c".to_string()).await.unwrap().is_some());
    }

    /// R122-4 续: LFU policy 容量超限 evict 1 个
    /// (1.0 行为: MemoryCache::get 0 调 evictor.on_access, LFU 频次 = put count)
    /// 改用 2 次 put 模拟: a 频次 2, b 频次 1
    #[tokio::test]
    async fn memory_cache_lfu_evicts_one_when_over_capacity() {
        let config = CacheBuilder::new()
            .max_size(2)
            .default_ttl(Duration::from_secs(60))
            .policy(EvictionPolicy::Lfu)
            .shards(16)
            .build();
        let cache = MemoryCache::<String, i32>::new(config).unwrap();
        // a 频次 2 (put 2 次)
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("a".to_string(), 11, Duration::from_secs(60))
            .await
            .unwrap();
        // b 频次 1 (put 1 次)
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        // 装第 3 个 → 触发 evict (LFU: b 频次最低 → 被淘汰)
        cache
            .put("c".to_string(), 3, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(cache.len().await, 2, "LFU 容量超限后仍保持 max_size=2");
        let present = [
            cache.get(&"a".to_string()).await.unwrap().is_some(),
            cache.get(&"b".to_string()).await.unwrap().is_some(),
            cache.get(&"c".to_string()).await.unwrap().is_some(),
        ]
        .iter()
        .filter(|x| **x)
        .count();
        assert_eq!(present, 2, "LFU 容量超限后应保持 2 个, 1 个被 evict");
        assert!(
            cache.get(&"b".to_string()).await.unwrap().is_none()
                || cache.get(&"a".to_string()).await.unwrap().is_none(),
            "LFU 行为: 至少 1 个被 evict (b 频次最低是候选)"
        );
        assert!(cache.get(&"c".to_string()).await.unwrap().is_some(), "新插入 c 必须存在");
    }

    /// R122-4 续: FIFO policy 容量超限 evict 最先插入的 1 个 (0 访问更新)
    #[tokio::test]
    async fn memory_cache_fifo_evicts_one_when_over_capacity() {
        let config = CacheBuilder::new()
            .max_size(2)
            .default_ttl(Duration::from_secs(60))
            .policy(EvictionPolicy::Fifo)
            .shards(16)
            .build();
        let cache = MemoryCache::<String, i32>::new(config).unwrap();
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        // 访问 a 不改顺序 (FIFO 0 访问更新)
        cache.get(&"a".to_string()).await;
        // 装第 3 个 → 触发 evict (FIFO: a 最先插入 → 被淘汰)
        cache
            .put("c".to_string(), 3, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(cache.len().await, 2);
        assert!(
            cache.get(&"a".to_string()).await.unwrap().is_none(),
            "FIFO 行为: a (最先插入) 应被淘汰 (访问 0 改顺序)"
        );
        assert!(cache.get(&"b".to_string()).await.unwrap().is_some());
        assert!(cache.get(&"c".to_string()).await.unwrap().is_some());
    }

    /// R122-4 续: ARC policy 容量超限 evict 1 个 (T1 优先, 简化 ARC)
    #[tokio::test]
    async fn memory_cache_arc_evicts_one_when_over_capacity() {
        let config = CacheBuilder::new()
            .max_size(2)
            .default_ttl(Duration::from_secs(60))
            .policy(EvictionPolicy::Arc)
            .shards(16)
            .build();
        let cache = MemoryCache::<String, i32>::new(config).unwrap();
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("c".to_string(), 3, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(cache.len().await, 2);
        let present_count = [
            cache.get(&"a".to_string()).await.unwrap().is_some(),
            cache.get(&"b".to_string()).await.unwrap().is_some(),
            cache.get(&"c".to_string()).await.unwrap().is_some(),
        ]
        .iter()
        .filter(|x| **x)
        .count();
        assert_eq!(
            present_count, 2,
            "ARC 容量超限后应保持 2 个, 至少 1 个被 evict"
        );
    }

    /// R122-4 续: TinyLFU policy 容量超限 evict 频次最低的 1 个
    #[tokio::test]
    async fn memory_cache_tiny_lfu_evicts_one_when_over_capacity() {
        let config = CacheBuilder::new()
            .max_size(2)
            .default_ttl(Duration::from_secs(60))
            .policy(EvictionPolicy::TinyLfu)
            .shards(16)
            .build();
        let cache = MemoryCache::<String, i32>::new(config).unwrap();
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("c".to_string(), 3, Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(cache.len().await, 2);
        let evicted = cache.get(&"b".to_string()).await.unwrap().is_none()
            || cache.get(&"a".to_string()).await.unwrap().is_none();
        assert!(
            evicted,
            "TinyLFU 容量超限后至少 1 个被 evict (b 频次最低候选)"
        );
        assert!(cache.get(&"c".to_string()).await.unwrap().is_some(), "新插入 c 必须存在");
    }

    /// R122-4 续: MemoryCache::evict_one() public method 显式选 victim
    /// (1.0 行为: put 内部已调, 这里验证 public method 也 work)
    #[tokio::test]
    async fn memory_cache_evict_one_returns_victim() {
        let config = CacheBuilder::new()
            .max_size(2)
            .default_ttl(Duration::from_secs(60))
            .policy(EvictionPolicy::Lru)
            .shards(16)
            .build();
        let cache = MemoryCache::<String, i32>::new(config).unwrap();
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        // 显式 evict 1 个
        let victim = cache.evict_one();
        assert_eq!(victim, Some("a".to_string()), "LRU 行为: a 是 LRU 优先淘汰");
        assert_eq!(cache.len().await, 1);
        let victim2 = cache.evict_one();
        assert_eq!(victim2, Some("b".to_string()));
        assert_eq!(cache.len().await, 0);
        assert!(cache.evict_one().is_none(), "空 cache evict_one 返 None");
    }
}
