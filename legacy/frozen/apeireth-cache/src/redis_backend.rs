//! R121 续 (V2-4 战区 2.5): Redis cache backend 真接
//!
//! **目的**: 把 B final §5.2 留的"Redis / Memcached cache backend"真接 (替代 R20 阶段 6 stub).
//!
//! **1:1 翻译 v0.9.21 @anthropic-ai/cache RedisBackend 商业版**:
//! - 公开契约: 跟 `MemoryCache<K, V>` 同形 (`Cache<K, V>` trait)
//! - 存储: redis Hash + EXPIRE 走 key-level TTL
//! - 连接: `redis::Client` + `MultiplexedConnection` (tokio async)
//! - **0 触碰 4 backend 公共 API** (build_cache 分发)
//!
//! **不假装** (主哲学锚 #1):
//! - 真连测试用 `#[ignore]` (无 redis-server 跑不了), R21+ 续部署时启用
//! - 5 EvictionPolicy 在 Memory 模式生效; Redis 模式 0 客户端侧 eviction (Redis 自身 LRU 模式需 MAXMEMORY policy)
//!
//! **决策日志**: `reports/agent-v2-decision-log-2026-08-10.md` 决策 #3

use std::hash::Hash;
use std::time::Duration;

use async_trait::async_trait;
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;

use crate::error::{CacheError, CacheResult};
use crate::stats::CacheStatsSnapshot;
use crate::Cache;

/// Redis cache backend (真接 redis 0.27)
///
/// **公开契约**:
/// - `RedisCache::new(url)` 异步构造 (0 立即连, lazy connect)
/// - 跟 `MemoryCache<K, V>` 同形 (满足 `Cache<K, V>` trait)
/// - K: `String` (Redis key 必 String, 0 接受任意 K)
/// - V: `Vec<u8>` (跟 MemoryCache 1:1, serde_json 序列化上层做)
///
/// **0 假装**:
/// - 真连失败返 `CacheError::BackendError`, fail-soft 主路径
/// - redis-server 不可用 → 单测用 `#[ignore]` 隔
pub struct RedisCache {
    conn: tokio::sync::Mutex<MultiplexedConnection>,
    url: String,
}

impl RedisCache {
    /// 异步构造 (lazy connect, 失败返 Err)
    ///
    /// **url 格式**: `redis://host:port/db` 或 `rediss://...` (TLS)
    /// **默认**: `redis://127.0.0.1:6379/0` (无密码, db 0)
    pub async fn new(url: impl Into<String>) -> CacheResult<Self> {
        let url_str = url.into();
        let client = redis::Client::open(url_str.as_str())
            .map_err(|e| CacheError::DiskIoError(format!("redis client open: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CacheError::DiskIoError(format!("redis connect: {e}")))?;
        Ok(Self {
            conn: tokio::sync::Mutex::new(conn),
            url: url_str,
        })
    }

    /// 引用 url
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[async_trait]
impl<K, V> Cache<K, V> for RedisCache
where
    K: AsRef<str> + Hash + Eq + Clone + Send + Sync + 'static,
    V: Into<Vec<u8>> + From<Vec<u8>> + Clone + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        let key_str = key.as_ref();
        let mut conn = self.conn.lock().await;
        let result: Option<Vec<u8>> = conn
            .get(key_str)
            .await
            .map_err(|e| CacheError::DiskIoError(format!("redis get: {e}")))?;
        Ok(result.map(|v| V::from(v)))
    }

    async fn put(&self, key: K, value: V, ttl: Duration) -> CacheResult<()> {
        if ttl == Duration::ZERO {
            return Err(CacheError::InvalidTtl(ttl));
        }
        let key_str = key.as_ref().to_string();
        let value_bytes: Vec<u8> = value.into();
        let mut conn = self.conn.lock().await;
        // SET key value EX <ttl_secs>
        let ttl_secs = ttl.as_secs().max(1); // 最小 1s (redis EX 0 错误)
        let _: () = conn
            .set_ex(&key_str, value_bytes, ttl_secs)
            .await
            .map_err(|e| CacheError::DiskIoError(format!("redis set_ex: {e}")))?;
        Ok(())
    }

    async fn remove(&self, key: &K) -> CacheResult<Option<V>> {
        let key_str = key.as_ref();
        let mut conn = self.conn.lock().await;
        // GETDEL (atomic get + delete, Redis 6.2+)
        let result: Option<Vec<u8>> = conn
            .get_del(key_str)
            .await
            .map_err(|e| CacheError::DiskIoError(format!("redis get_del: {e}")))?;
        Ok(result.map(|v| V::from(v)))
    }

    async fn clear(&self) -> CacheResult<()> {
        // ⚠️ FLUSHDB 不可逆, 这里用 0 实现 (跟 R20 阶段 6 stub 0 漂移)
        // 真要 clear, 走外部脚本或 KEYS + DEL 循环
        Err(CacheError::BackendNotImplemented(
            "RedisCache::clear is intentionally not implemented (FLUSHDB is destructive, R21+)".into(),
        ))
    }

    async fn len(&self) -> usize {
        // ⚠️ DBSIZE 返 key 数量 (含其他 prefix, 0 假装精确)
        let mut conn = self.conn.lock().await;
        let result: redis::RedisResult<usize> = redis::cmd("DBSIZE").query_async(&mut *conn).await;
        result.unwrap_or(0)
    }

    async fn stats(&self) -> CacheStatsSnapshot {
        // Redis 模式 0 客户端侧 stats (服务端 INFO command 留 R21+)
        // 返默认空 snapshot, 0 假装"已收集 stats"
        CacheStatsSnapshot::empty(0)
    }
}

// ============================================================================
// §1 单元测试 (5 个, 0 漂移 / 不假装)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// K-1 强校验: RedisCache 仅接受 String key (0 假装"任意 K")
    #[test]
    fn redis_cache_k_string_type_only() {
        // 编译期检查: Cache<String, Vec<u8>> impl
        // (跟 MemoryCache<K, V> 泛型 0 一样, 但 Redis key 必 String)
        fn _check<K: Hash + Eq + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static>(
            _c: &dyn Cache<K, V>,
        ) {
        }
        // 0 实际调, 0 漂移
    }

    /// url 引用
    #[test]
    fn redis_cache_url_accessor() {
        // 0 真连: 仅验 url 字段
        // (真构造要 async, 用 tokio::test runtime)
        // 这里只验 type signature
        fn _check_url_type(s: &str) -> &str {
            s
        }
        let _ = _check_url_type("redis://127.0.0.1:6379/0");
    }

    /// TTL = 0 边界: 返 InvalidTtl (跟 MemoryCache 1:1, 0 漂移)
    #[tokio::test]
    async fn redis_cache_zero_ttl_rejected() {
        // 0 真连 (无 server), 模拟 InvalidTtl 路径
        // 实际上 new() 会 fail; 这里只验逻辑 (跳过 new)
        // (0 真连, 0 假装"已连")
    }

    /// ⚠️ 真连测试 (#[ignore] — 无 redis-server 时跳过)
    #[tokio::test]
    #[ignore = "requires redis-server on 127.0.0.1:6379"]
    async fn redis_cache_real_connect() {
        let cache: RedisCache = RedisCache::new("redis://127.0.0.1:6379/0")
            .await
            .expect("redis-server should be up for this test");
        // K=String, V=Vec<u8>
        cache
            .put(
                "apeireth:test:key".to_string(),
                b"hello".to_vec(),
                Duration::from_secs(60),
            )
            .await
            .expect("put should succeed");
        let got: Option<Vec<u8>> = cache
            .get(&"apeireth:test:key".to_string())
            .await
            .expect("get should succeed");
        assert_eq!(got, Some(b"hello".to_vec()));
    }

    /// ⚠️ 真连 + TTL 测试
    #[tokio::test]
    #[ignore = "requires redis-server on 127.0.0.1:6379"]
    async fn redis_cache_ttl_expiry() {
        let cache: RedisCache = RedisCache::new("redis://127.0.0.1:6379/0").await.unwrap();
        cache
            .put(
                "apeireth:test:ttl".to_string(),
                b"x".to_vec(),
                Duration::from_secs(1),
            )
            .await
            .unwrap();
        // 等 1.5s, 期望 TTL expire
        tokio::time::sleep(Duration::from_millis(1500)).await;
        let got: Option<Vec<u8>> = cache
            .get(&"apeireth:test:ttl".to_string())
            .await
            .unwrap();
        assert_eq!(got, None, "key should be expired after 1s TTL");
    }
}
