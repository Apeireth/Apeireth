//! Fixture: in-process apeireth-cache 5 policy + 4 backend + 16-256 shard + TTL lazy/eager (per
//! RIVAL §蓝图 §3.7 fixture 模式).
//!
//! 25+ 测试覆盖 (per task spec §7):
//! 1. `test_policy_5_enums` — 5 EvictionPolicy 1:1 列表
//! 2. `test_backend_4_enums` — 4 BackendKind 1:1 列表
//! 3. `test_lru_basic_get_put` — MemoryCache LRU 基础
//! 4. `test_lru_eviction` — 超 max_size 时返 CapacityExceeded (skeleton 简化)
//! 5. `test_lru_update_existing` — 覆盖已有 key
//! 6. `test_ttl_lazy_expiration` — lazy TTL 过期
//! 7. `test_ttl_eager_expiration` — eager TTL 策略 (实现存在)
//! 8. `test_shard_16_concurrent` — 16 shard + 并发
//! 9. `test_shard_64_concurrent` — 64 shard + 并发
//! 10. `test_shard_256_concurrent` — 256 shard + 并发
//! 11. `test_stats_hit_miss` — hit + miss 计数
//! 12. `test_stats_latency` — latency 字段
//! 13. `test_k1_max_size_zero` — K-1 强校验
//! 14. `test_k1_ttl_zero` — K-1 强校验
//! 15. `test_disk_backend_not_implemented` — Disk stub
//! 16. `test_redis_backend_not_implemented` — Redis stub
//! 17. `test_memcached_backend_not_implemented` — Memcached stub
//! 18. `test_clear_resets_stats` — clear 重置 stats
//! 19. `test_concurrent_get_put_100_tasks` — 100 任务并发
//! 20. `test_lru_impl_4_enums` — 4 LruImpl
//! 21. `test_ttl_mode_3_enums` — 3 TtlMode
//! 22. `test_shard_distribution_even` — 1000 key 分布大致均匀
//! 23. `test_stats_hit_rate_zero_when_no_calls` — K-1 hit_rate = 0.0
//! 24. `test_lru_crate_basic` — lru crate 实现
//! 25. `test_indexmap_lru_basic` — indexmap 实现
//! 26. `test_lru_impl_quickcache_stub` — quickcache 留口子
//! 27. `test_cache_builder_default` — CacheBuilder 默认
//! 28. `test_cache_builder_chained` — 链式
//!
//! 5 P0 crate 用同 fixture 模式 (per §蓝图 §3.7).

use apeireth_cache::{
    build_cache, BackendKind, Cache, CacheBuilder, CacheConfig, CacheError, EvictionPolicy,
    LruCrateLru, LruImpl, HashMapVecDequeLru, IndexMapLru, MemoryCache, ShardRouter, ShardedMap,
    TtlEntry, TtlMode, TtlPolicy, BACKEND_KIND_VARIANT_COUNT, CACHE_ERROR_VARIANT_COUNT,
    DEFAULT_BACKEND, DEFAULT_MAX_SIZE, DEFAULT_POLICY, DEFAULT_SHARDS, DEFAULT_TTL_SECS,
    EVICTION_POLICY_VARIANT_COUNT, SHARD_MAX, SHARD_MIN,
};
use std::sync::Arc;
use std::time::Duration;

// ============================================================================
// K-1 #1: 5 EvictionPolicy 1:1 列表
// ============================================================================

#[test]
fn test_policy_5_enums() {
    assert_eq!(EVICTION_POLICY_VARIANT_COUNT, 5);
    assert_eq!(EvictionPolicy::ALL.len(), 5);
    assert!(EvictionPolicy::ALL.contains(&EvictionPolicy::Lru));
    assert!(EvictionPolicy::ALL.contains(&EvictionPolicy::Lfu));
    assert!(EvictionPolicy::ALL.contains(&EvictionPolicy::Fifo));
    assert!(EvictionPolicy::ALL.contains(&EvictionPolicy::Arc));
    assert!(EvictionPolicy::ALL.contains(&EvictionPolicy::TinyLfu));
}

#[test]
fn test_policy_5_specific_strs() {
    assert_eq!(EvictionPolicy::Lru.as_str(), "LRU");
    assert_eq!(EvictionPolicy::Lfu.as_str(), "LFU");
    assert_eq!(EvictionPolicy::Fifo.as_str(), "FIFO");
    assert_eq!(EvictionPolicy::Arc.as_str(), "ARC");
    assert_eq!(EvictionPolicy::TinyLfu.as_str(), "TINY_LFU");
}

// ============================================================================
// K-1 #2: 4 BackendKind 1:1 列表
// ============================================================================

#[test]
fn test_backend_4_enums() {
    assert_eq!(BACKEND_KIND_VARIANT_COUNT, 4);
    assert_eq!(BackendKind::ALL.len(), 4);
    assert!(BackendKind::ALL.contains(&BackendKind::Memory));
    assert!(BackendKind::ALL.contains(&BackendKind::Disk));
    assert!(BackendKind::ALL.contains(&BackendKind::Redis));
    assert!(BackendKind::ALL.contains(&BackendKind::Memcached));
}

#[test]
fn test_backend_4_specific_strs() {
    assert_eq!(BackendKind::Memory.as_str(), "MEMORY");
    assert_eq!(BackendKind::Disk.as_str(), "DISK");
    assert_eq!(BackendKind::Redis.as_str(), "REDIS");
    assert_eq!(BackendKind::Memcached.as_str(), "MEMCACHED");
}

#[test]
fn test_backend_4_implemented_status() {
    assert!(BackendKind::Memory.is_implemented());
    assert!(!BackendKind::Disk.is_implemented());
    assert!(!BackendKind::Redis.is_implemented());
    assert!(!BackendKind::Memcached.is_implemented());
}

// ============================================================================
// K-1 #3: 4 LruImpl 1:1 列表
// ============================================================================

#[test]
fn test_lru_impl_4_enums() {
    assert_eq!(LruImpl::ALL.len(), 4);
    assert!(LruImpl::ALL.contains(&LruImpl::HashMapVecDeque));
    assert!(LruImpl::ALL.contains(&LruImpl::IndexMapBacked));
    assert!(LruImpl::ALL.contains(&LruImpl::LruCrate));
    assert!(LruImpl::ALL.contains(&LruImpl::QuickCache));
}

// ============================================================================
// K-1 #4: 3 TtlMode 1:1 列表
// ============================================================================

#[test]
fn test_ttl_mode_3_enums() {
    assert_eq!(TtlMode::ALL.len(), 3);
    assert!(TtlMode::ALL.contains(&TtlMode::Lazy));
    assert!(TtlMode::ALL.contains(&TtlMode::Eager));
    assert!(TtlMode::ALL.contains(&TtlMode::Both));
}

#[test]
fn test_ttl_mode_3_enabled_status() {
    // Lazy: 仅 lazy
    assert!(TtlMode::Lazy.is_lazy_enabled());
    assert!(!TtlMode::Lazy.is_eager_enabled());
    // Eager: 仅 eager
    assert!(!TtlMode::Eager.is_lazy_enabled());
    assert!(TtlMode::Eager.is_eager_enabled());
    // Both: 双开
    assert!(TtlMode::Both.is_lazy_enabled());
    assert!(TtlMode::Both.is_eager_enabled());
}

// ============================================================================
// K-1 #5: 10 CacheError 1:1 计数
// ============================================================================

#[test]
fn test_cache_error_10_variants() {
    assert_eq!(CACHE_ERROR_VARIANT_COUNT, 10);
}

// ============================================================================
// LRU 基础测试
// ============================================================================

#[test]
fn test_lru_basic_get_put() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        let va = cache.get(&"a".to_string()).await.unwrap();
        let vb = cache.get(&"b".to_string()).await.unwrap();
        assert_eq!(va, Some(1));
        assert_eq!(vb, Some(2));
    });
}

#[test]
fn test_lru_update_existing() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        // 覆盖
        cache
            .put("a".to_string(), 100, Duration::from_secs(60))
            .await
            .unwrap();
        let va = cache.get(&"a".to_string()).await.unwrap();
        assert_eq!(va, Some(100));
        assert_eq!(cache.len().await, 1);
    });
}

#[test]
fn test_lru_eviction() {
    // R121 续 (V2-4 战区 2.5): 5 EvictionPolicy 真接, 0 返 CapacityExceeded
    // B 留 §5.4 修复: 超 max_size 时调 evictor.pick_victim() 选 victim, 0 假装"硬解析"
    let config = CacheBuilder::new()
        .max_size(2)
        .shards(32)
        .policy(EvictionPolicy::Lru)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .put("b".to_string(), 2, Duration::from_secs(60))
            .await
            .unwrap();
        // 第 3 个 put 触发 LRU eviction, 替掉最早 a, 0 返 CapacityExceeded
        let r = cache
            .put("c".to_string(), 3, Duration::from_secs(60))
            .await;
        assert!(r.is_ok(), "eviction should succeed (0 返 CapacityExceeded): {r:?}");
        // 容量稳定在 2
        assert_eq!(cache.len().await, 2);
        // a 被淘汰, c 加入
        assert!(cache.get(&"a".to_string()).await.unwrap().is_none());
        assert_eq!(cache.get(&"b".to_string()).await.unwrap(), Some(2));
        assert_eq!(cache.get(&"c".to_string()).await.unwrap(), Some(3));
    });
}

// ============================================================================
// TTL 过期测试
// ============================================================================

#[test]
fn test_ttl_lazy_expiration() {
    // lazy: get 时检查 + 删除
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 构造已过期的 entry (用负 ttl 兜底是不允许的, 改用直接 get miss)
        // 真实 lazy expiration 测: 用 past inserted_at
        let past = std::time::Instant::now() - Duration::from_secs(120);
        let entry = TtlEntry::with_inserted_at(42_i32, Duration::from_secs(60), past);
        assert!(entry.is_expired());

        // 实际 cache: put 短期 ttl + sleep
        cache
            .put("a".to_string(), 1, Duration::from_millis(50))
            .await
            .unwrap();
        let v1 = cache.get(&"a".to_string()).await.unwrap();
        assert_eq!(v1, Some(1));

        tokio::time::sleep(Duration::from_millis(100)).await;

        // TTL 过期, lazy 检查返 None
        let v2 = cache.get(&"a".to_string()).await.unwrap();
        assert_eq!(v2, None);
    });
}

#[test]
fn test_ttl_eager_expiration() {
    // eager 策略: TtlPolicy::eager_only / both 都能用
    let p1 = TtlPolicy::eager_only(Duration::from_millis(500));
    assert_eq!(p1.mode, TtlMode::Eager);
    assert_eq!(p1.scan_interval, Duration::from_millis(500));

    let p2 = TtlPolicy::both(Duration::from_secs(1));
    assert_eq!(p2.mode, TtlMode::Both);

    // Default 是 both + 1s
    let p3 = TtlPolicy::default_policy();
    assert_eq!(p3.mode, TtlMode::Both);
    assert_eq!(p3.scan_interval, Duration::from_secs(1));
}

// ============================================================================
// 分片锁测试
// ============================================================================

#[test]
fn test_shard_16_concurrent() {
    let m: ShardedMap<String, i32> = ShardedMap::new(16).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let m = Arc::new(m);
        let mut handles = vec![];
        for i in 0..50 {
            let m = Arc::clone(&m);
            handles.push(tokio::spawn(async move {
                m.put(format!("k{i}"), i);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        // 50 items 全部 put 成功
        assert_eq!(m.len(), 50);
        // shard_count = 16
        assert_eq!(m.shard_count(), 16);
    });
}

#[test]
fn test_shard_64_concurrent() {
    let m: ShardedMap<String, i32> = ShardedMap::new(64).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let m = Arc::new(m);
        let mut handles = vec![];
        for i in 0..100 {
            let m = Arc::clone(&m);
            handles.push(tokio::spawn(async move {
                m.put(format!("k{i}"), i);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(m.len(), 100);
        assert_eq!(m.shard_count(), 64);
    });
}

#[test]
fn test_shard_256_concurrent() {
    let m: ShardedMap<String, i32> = ShardedMap::new(256).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let m = Arc::new(m);
        let mut handles = vec![];
        for i in 0..200 {
            let m = Arc::clone(&m);
            handles.push(tokio::spawn(async move {
                m.put(format!("k{i}"), i);
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(m.len(), 200);
        assert_eq!(m.shard_count(), 256);
    });
}

#[test]
fn test_shard_distribution_even() {
    let m: ShardedMap<String, i32> = ShardedMap::new_unchecked(16);
    for i in 0..1000 {
        m.put(format!("k{i}"), i);
    }
    // 1000 items / 16 shards ≈ 62.5, 允许每个 shard < 200
    for shard_id in 0..16 {
        let s = m.shard_len(shard_id);
        assert!(s < 200, "shard {shard_id} has {s} > 200 (uneven)");
    }
}

#[test]
fn test_shard_count_constants() {
    assert_eq!(SHARD_MIN, 16);
    assert_eq!(SHARD_MAX, 256);
    assert_eq!(DEFAULT_SHARDS, 32);
}

// ============================================================================
// Stats 测试
// ============================================================================

#[test]
fn test_stats_hit_miss() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 3 put + 2 hit + 1 miss
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

        cache.get(&"a".to_string()).await.unwrap();
        cache.get(&"b".to_string()).await.unwrap();
        cache.get(&"zzz".to_string()).await.unwrap(); // miss

        let stats = cache.stats().await;
        assert_eq!(stats.hit, 2);
        assert_eq!(stats.miss, 1);
        assert!((stats.hit_rate - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(stats.put_count, 3);
    });
}

#[test]
fn test_stats_latency() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache.get(&"a".to_string()).await.unwrap();

        let stats = cache.stats().await;
        // latency 字段是 0 或 > 0 (取决于实际耗时, 不精确断言)
        assert!(stats.avg_get_latency_us >= 0.0);
        assert!(stats.avg_put_latency_us >= 0.0);
    });
}

#[test]
fn test_stats_hit_rate_zero_when_no_calls() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let stats = cache.stats().await;
        assert_eq!(stats.hit_rate, 0.0); // K-1 强校验
        assert_eq!(stats.hit, 0);
        assert_eq!(stats.miss, 0);
    });
}

// ============================================================================
// K-1 强校验测试
// ============================================================================

#[test]
fn test_k1_max_size_zero() {
    let config = CacheBuilder::new()
        .max_size(0)
        .build();
    let r: Result<MemoryCache<String, i32>, _> = MemoryCache::new(config);
    assert!(matches!(r, Err(CacheError::InvalidMaxSize(0))));
}

#[test]
fn test_k1_ttl_zero() {
    let config = CacheBuilder::new()
        .max_size(100)
        .default_ttl(Duration::ZERO)
        .build();
    let r: Result<MemoryCache<String, i32>, _> = MemoryCache::new(config);
    assert!(matches!(r, Err(CacheError::InvalidTtl(_))));
}

#[test]
fn test_k1_shards_below_16() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(8)
        .build();
    let r: Result<MemoryCache<String, i32>, _> = MemoryCache::new(config);
    assert!(matches!(r, Err(CacheError::InvalidShardCount(8))));
}

#[test]
fn test_k1_shards_above_256() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(257)
        .build();
    let r: Result<MemoryCache<String, i32>, _> = MemoryCache::new(config);
    assert!(matches!(r, Err(CacheError::InvalidShardCount(257))));
}

#[test]
fn test_k1_put_ttl_zero() {
    // put 调用时 ttl = 0 应返 InvalidTtl
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let r = cache.put("a".to_string(), 1, Duration::ZERO).await;
        assert!(matches!(r, Err(CacheError::InvalidTtl(_))));
    });
}

// ============================================================================
// Backend stub 测试
// ============================================================================

#[test]
fn test_disk_backend_not_implemented() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .backend(BackendKind::Disk)
        .build();
    let r: Result<MemoryCache<String, i32>, _> = MemoryCache::new(config);
    assert!(matches!(r, Err(CacheError::BackendNotImplemented(_))));
}

#[test]
fn test_redis_backend_not_implemented() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .backend(BackendKind::Redis)
        .build();
    let r: Result<MemoryCache<String, i32>, _> = MemoryCache::new(config);
    assert!(matches!(r, Err(CacheError::BackendNotImplemented(_))));
}

#[test]
fn test_memcached_backend_not_implemented() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .backend(BackendKind::Memcached)
        .build();
    let r: Result<MemoryCache<String, i32>, _> = MemoryCache::new(config);
    assert!(matches!(r, Err(CacheError::BackendNotImplemented(_))));
}

#[test]
fn test_build_cache_disk_stub() {
    let config = CacheBuilder::new()
        .max_size(100)
        .backend(BackendKind::Disk)
        .build();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let r: Result<Arc<dyn Cache<String, i32>>, _> = build_cache(config).await;
        assert!(matches!(r, Err(CacheError::BackendNotImplemented(_))));
    });
}

// ============================================================================
// Clear + Stats 测试
// ============================================================================

#[test]
fn test_clear_resets_stats() {
    let config = CacheBuilder::new()
        .max_size(100)
        .shards(32)
        .build();
    let cache: MemoryCache<String, i32> = MemoryCache::new(config).unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        cache
            .put("a".to_string(), 1, Duration::from_secs(60))
            .await
            .unwrap();
        cache.get(&"a".to_string()).await.unwrap();

        let before = cache.stats().await;
        assert!(before.hit > 0 || before.miss > 0);

        cache.clear().await.unwrap();

        let after = cache.stats().await;
        assert_eq!(after.hit, 0);
        assert_eq!(after.miss, 0);
        assert_eq!(after.size, 0);
        assert_eq!(after.put_count, 0);
    });
}

// ============================================================================
// 并发测试
// ============================================================================

#[test]
fn test_concurrent_get_put_100_tasks() {
    let config = CacheBuilder::new()
        .max_size(500)
        .shards(64)
        .build();
    let cache: Arc<MemoryCache<String, i32>> = Arc::new(MemoryCache::new(config).unwrap());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut handles = vec![];
        for i in 0..100 {
            let c = Arc::clone(&cache);
            handles.push(tokio::spawn(async move {
                if i % 2 == 0 {
                    c.put(format!("k{i}"), i, Duration::from_secs(60)).await.unwrap();
                } else {
                    let _ = c.get(&format!("k{i}")).await;
                }
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        // 50 put 全部成功 (size 50, < max_size 500)
        assert_eq!(cache.len().await, 50);

        // stats: 50 put + 50 get
        let stats = cache.stats().await;
        assert_eq!(stats.put_count, 50);
        // hit + miss = 50
        assert_eq!(stats.hit + stats.miss, 50);
    });
}

// ============================================================================
// LRU 4 实现测试
// ============================================================================

#[test]
fn test_hashmap_vecdeque_lru_basic() {
    let mut lru = HashMapVecDequeLru::new(2);
    lru.put("a".to_string(), 1);
    lru.put("b".to_string(), 2);
    assert_eq!(lru.get(&"a".to_string()), Some(1));
    assert_eq!(lru.get(&"b".to_string()), Some(2));
    assert_eq!(lru.len(), 2);
    assert_eq!(lru.capacity(), 2);
}

#[test]
fn test_indexmap_lru_basic() {
    let mut lru = IndexMapLru::new(2);
    lru.put("a".to_string(), 1);
    lru.put("b".to_string(), 2);
    assert_eq!(lru.get(&"a".to_string()), Some(1));
    assert_eq!(lru.get(&"b".to_string()), Some(2));
    assert_eq!(lru.len(), 2);
}

#[test]
fn test_lru_crate_basic() {
    let mut lru = LruCrateLru::new(2);
    lru.put("a".to_string(), 1);
    lru.put("b".to_string(), 2);
    assert_eq!(lru.get_cloned(&"a".to_string()), Some(1));
    assert_eq!(lru.get_cloned(&"b".to_string()), Some(2));
    assert_eq!(lru.len(), 2);
}

#[test]
fn test_lru_impl_quickcache_stub() {
    assert!(!LruImpl::QuickCache.is_implemented());
    assert!(LruImpl::HashMapVecDeque.is_implemented());
    assert!(LruImpl::IndexMapBacked.is_implemented());
    assert!(LruImpl::LruCrate.is_implemented());
}

// ============================================================================
// CacheBuilder + CacheConfig 测试
// ============================================================================

#[test]
fn test_cache_builder_default() {
    let b = CacheBuilder::default();
    let c = b.build();
    assert_eq!(c.max_size, DEFAULT_MAX_SIZE);
    assert_eq!(c.default_ttl, Duration::from_secs(DEFAULT_TTL_SECS));
    assert_eq!(c.policy, DEFAULT_POLICY);
    assert_eq!(c.shards, DEFAULT_SHARDS);
    assert_eq!(c.backend, DEFAULT_BACKEND);
}

#[test]
fn test_cache_builder_chained() {
    let c = CacheBuilder::new()
        .max_size(2048)
        .default_ttl(Duration::from_secs(120))
        .policy(EvictionPolicy::TinyLfu)
        .shards(128)
        .backend(BackendKind::Memory)
        .build();
    assert_eq!(c.max_size, 2048);
    assert_eq!(c.default_ttl, Duration::from_secs(120));
    assert_eq!(c.policy, EvictionPolicy::TinyLfu);
    assert_eq!(c.shards, 128);
}

#[test]
fn test_cache_config_5_fields() {
    let c = CacheConfig {
        max_size: 100,
        default_ttl: Duration::from_secs(60),
        policy: EvictionPolicy::Lru,
        shards: 32,
        backend: BackendKind::Memory,
    };
    assert!(c.validate().is_ok());
    assert_eq!(c.max_size, 100);
}

#[test]
fn test_cache_config_serde_roundtrip() {
    let c = CacheConfig {
        max_size: 512,
        default_ttl: Duration::from_secs(30),
        policy: EvictionPolicy::Arc,
        shards: 16,
        backend: BackendKind::Memory,
    };
    let s = serde_json::to_string(&c).unwrap();
    let parsed: CacheConfig = serde_json::from_str(&s).unwrap();
    assert_eq!(c, parsed);
}

// ============================================================================
// ShardRouter 测试
// ============================================================================

#[test]
fn test_shard_router_default() {
    let r = ShardRouter::default();
    assert_eq!(r.shards(), 32);
    let key = "test";
    let id = r.route(&key);
    assert!(id < 32);
}

#[test]
fn test_shard_router_16_64_256() {
    for &n in &[16usize, 64, 256] {
        let r = ShardRouter::new(n).unwrap();
        for i in 0..100 {
            let key = format!("k{i}");
            let id = r.route(&key);
            assert!(id < n, "shard {id} >= {n}");
        }
    }
}
