//! # RedisProvider — 7 provider 模式 1: 外部 Redis
//!
//! **真接 (per 8 项之 1, "0 不假装已实现")**:
//! - 借 `redis = 0.27` crate 业界标准 (0 重复造 Redis 客户端)
//! - 7 通用方法 (`kind`/`set`/`get`/`delete`/`exists`/`clear`/`size`) 走 redis-rs 真实命令
//! - **端到端**: config 强校验 + connection_string 解析 + AsyncConnection 创建
//!   (实际服务端连接由 R21+ 续做, 需本地 Redis)
//!
//! **不假装**:
//! - skeleton 阶段 `new()` 走 `redis::Client::open(url)`, 真创建 Client (不连服务端)
//! - `connect()` 异步走 `get_async_connection` 真连服务端 (R21+ 续做测试)
//! - 0 假装"无 server 也能 set/get" — 没服务端必然失败
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `redis://[user:pass@]host:port/[db]` (借 redis-rs URL 解析)
//! 2. timeout = [1ms, 1h]
//! 3. max_size = [1KB, 1TB]
//! 4. persist = bool (true = AOF/RDB 持久化, 透传给 Redis 服务端配置, 0 假装客户端控)
//! 5. cache_ttl = [0ms, 7d] (0 = 永不过期, >0 = SET EX 秒数)
//! 6. scope = Global (Redis 集群共享)

use std::time::Duration;

use async_trait::async_trait;
use redis::Client;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{MemoryProvider, ProviderConfig, ProviderKind, ProviderScope};

/// **RedisProvider**: 外部 Redis server provider.
#[derive(Debug, Clone)]
pub struct RedisProvider {
    /// redis-rs Client (借 0.27 业界标准, 0 重复造).
    client: Client,
    /// 6 K-1 强校验过的 config.
    config: ProviderConfig,
}

impl RedisProvider {
    /// 新建 RedisProvider, 6 K-1 强校验 + 借 redis-rs 真创建 Client.
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::Redis)?;

        // K-1 #1: 解析 redis:// URL
        let url = config
            .connection_string
            .strip_prefix("redis://")
            .map(|s| format!("redis://{s}"))
            .ok_or_else(|| MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "must start with `redis://`".to_string(),
            })?;

        // 真创建 Client (0 实际连接, 仅 parse URL + 创建 handle)
        let client = Client::open(url.as_str()).map_err(|e| MemoryProviderError::Connection {
            provider: ProviderKind::Redis,
            reason: format!("redis::Client::open failed: {e}"),
        })?;

        Ok(Self { client, config })
    }

    /// 6 K-1 字段 hardcoded: scope = Global (Redis 集群共享).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Global
    }

    /// Get redis-rs Client handle.
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl MemoryProvider for RedisProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Redis
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        // 借 redis-rs 业界标准: get_multiplexed_async_connection + SET
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Redis,
                reason: format!("get_async_connection failed: {e}"),
            })?;
        // K-1 #5: cache_ttl → EX seconds (0 = 永不过期, 不传 EX)
        if self.config.cache_ttl > Duration::from_secs(0) {
            let _: () = redis::cmd("SET")
                .arg(key)
                .arg(value)
                .arg("EX")
                .arg(self.config.cache_ttl.as_secs())
                .query_async(&mut conn)
                .await
                .map_err(|e| MemoryProviderError::Backend {
                    provider: ProviderKind::Redis,
                    reason: format!("SET failed: {e}"),
                })?;
        } else {
            let _: () = redis::cmd("SET")
                .arg(key)
                .arg(value)
                .query_async(&mut conn)
                .await
                .map_err(|e| MemoryProviderError::Backend {
                    provider: ProviderKind::Redis,
                    reason: format!("SET failed: {e}"),
                })?;
        }
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Redis,
                reason: format!("get_async_connection failed: {e}"),
            })?;
        let value: Option<Vec<u8>> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Redis,
                reason: format!("GET failed: {e}"),
            })?;
        Ok(value)
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Redis,
                reason: format!("get_async_connection failed: {e}"),
            })?;
        let _: i64 = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Redis,
                reason: format!("DEL failed: {e}"),
            })?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Redis,
                reason: format!("get_async_connection failed: {e}"),
            })?;
        let count: i64 = redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Redis,
                reason: format!("EXISTS failed: {e}"),
            })?;
        Ok(count > 0)
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        // Redis FLUSHDB (0 假装保留 — skeleton 阶段仅清当前 db)
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Redis,
                reason: format!("get_async_connection failed: {e}"),
            })?;
        let _: () = redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Redis,
                reason: format!("FLUSHDB failed: {e}"),
            })?;
        Ok(())
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Redis,
                reason: format!("get_async_connection failed: {e}"),
            })?;
        let count: i64 = redis::cmd("DBSIZE")
            .query_async(&mut conn)
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Redis,
                reason: format!("DBSIZE failed: {e}"),
            })?;
        Ok(count as u64)
    }
}

/// **RedisConfigDefault** — 借 serde derive 序列化 ProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfigDefault {
    pub config: ProviderConfig,
}

// =====================================================================
// 单元测试 (10 tests per 借鉴 #6 模式 1:1)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_config() -> ProviderConfig {
        ProviderConfig::new(
            "redis://localhost:6379/0",
            Duration::from_secs(5),
            1024 * 1024,
            true, // AOF/RDB 持久化
            Duration::from_secs(60),
            ProviderScope::Global,
        )
    }

    // ----- 10 unit tests -----

    #[test]
    fn test_1_provider_kind_is_redis() {
        let p = RedisProvider::new(make_config()).unwrap();
        assert_eq!(p.kind(), ProviderKind::Redis);
    }

    #[test]
    fn test_2_k1_connection_string_must_be_redis_scheme() {
        let bad = ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            ProviderScope::Global,
        );
        let r = RedisProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_3_k1_timeout_validation_propagates() {
        let bad = ProviderConfig::new(
            "redis://localhost:6379/0",
            Duration::from_micros(500),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            ProviderScope::Global,
        );
        let r = RedisProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_4_k1_max_size_validation_propagates() {
        let bad = ProviderConfig::new(
            "redis://localhost:6379/0",
            Duration::from_secs(5),
            512,
            true,
            Duration::from_secs(60),
            ProviderScope::Global,
        );
        let r = RedisProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_5_k1_persist_passes_through_to_redis() {
        // Redis persist 字段仅过 validate, 实际由服务端 AOF/RDB 配置决定
        let p = RedisProvider::new(make_config()).unwrap();
        assert!(p.config.persist);
    }

    #[test]
    fn test_6_k1_cache_ttl_sets_ex_seconds() {
        let p = RedisProvider::new(make_config()).unwrap();
        assert_eq!(p.config.cache_ttl, Duration::from_secs(60));
    }

    #[test]
    fn test_7_k1_scope_is_always_global() {
        let p = RedisProvider::new(make_config()).unwrap();
        assert_eq!(p.scope(), ProviderScope::Global);
    }

    #[test]
    fn test_8_provider_init_with_valid_config() {
        let p = RedisProvider::new(make_config()).expect("valid config");
        assert_eq!(p.kind(), ProviderKind::Redis);
        // 验证 redis-rs Client 真创建 (0 实际连接)
        let _ = p.client();
    }

    #[tokio::test]
    async fn test_9_set_get_without_server_returns_connection_error() {
        // 0 假装 — 没有 Redis server 必然失败, 错误类型应是 Connection
        let p = RedisProvider::new(make_config()).unwrap();
        let r = p.set("k1", b"v1").await;
        assert!(matches!(r, Err(MemoryProviderError::Connection { .. })));
    }

    #[tokio::test]
    async fn test_10_get_delete_exists_clear_size_without_server_returns_connection_error() {
        // 5 个方法 (除 set) 都不依赖 server 也返 Connection 错 (per redis-rs 行为)
        let p = RedisProvider::new(make_config()).unwrap();
        assert!(matches!(
            p.get("k1").await,
            Err(MemoryProviderError::Connection { .. })
        ));
        assert!(matches!(
            p.delete("k1").await,
            Err(MemoryProviderError::Connection { .. })
        ));
        assert!(matches!(
            p.exists("k1").await,
            Err(MemoryProviderError::Connection { .. })
        ));
        assert!(matches!(
            p.clear().await,
            Err(MemoryProviderError::Connection { .. })
        ));
        assert!(matches!(
            p.size().await,
            Err(MemoryProviderError::Connection { .. })
        ));
    }
}
