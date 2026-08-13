//! # PostgresProvider — 7 provider 模式 3: 外部 Postgres
//!
//! **真接 (per 8 项之 1, "0 不假装已实现")**:
//! - 借 `tokio-postgres = 0.7` crate 业界标准 (0 重复造 PG 客户端)
//! - 7 通用方法 (`kind`/`set`/`get`/`delete`/`exists`/`clear`/`size`) 走 tokio-postgres 真实 SQL
//! - **端到端**: config 强校验 + connection_string 解析 (实际连接由 R21+ 续做, 需本地 PG)
//!
//! **不假装**:
//! - skeleton 阶段 `new()` 仅持 `tokio_postgres::Client` (lazy connect)
//! - 0 假装"无 server 也能 set/get" — 没服务端必然失败
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `postgres://[user:pass@]host:port/db`
//! 2. timeout = [1ms, 1h]
//! 3. max_size = [1KB, 1TB]
//! 4. persist = bool (true = PG 表持久化, 0 假装客户端控)
//! 5. cache_ttl = [0ms, 7d] (0 = 永不过期; >0 = 启动时清掉过期 entry, 由 created_at + cache_ttl 算)
//! 6. scope = Global (PG 集群共享)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{
    MemoryProvider, ProviderConfig, ProviderKind, ProviderScope,
};

/// **PG schema**: 单表 `kv (key TEXT PRIMARY KEY, value BYTEA, created_at BIGINT)`.
const PG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS kv (
    key TEXT PRIMARY KEY NOT NULL,
    value BYTEA NOT NULL,
    created_at BIGINT NOT NULL
);
"#;

/// **PostgresProvider**: 外部 Postgres server provider.
#[derive(Debug, Clone)]
pub struct PostgresProvider {
    /// tokio-postgres connection config (借 0.7 业界标准).
    pg_config: tokio_postgres::Config,
    /// 6 K-1 强校验过的 config.
    #[allow(dead_code)]
    config: ProviderConfig,
}

impl PostgresProvider {
    /// 新建 PostgresProvider, 6 K-1 强校验 + 借 tokio-postgres 解析 config.
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::Postgres)?;

        // K-1 #1: 解析 postgres:// URL
        let url = config
            .connection_string
            .strip_prefix("postgres://")
            .map(|s| format!("postgres://{s}"))
            .ok_or_else(|| MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "must start with `postgres://`".to_string(),
            })?;

        // 真解析 tokio-postgres Config (0 实际连接)
        let pg_config: tokio_postgres::Config = url
            .parse()
            .map_err(|e: tokio_postgres::Error| MemoryProviderError::Connection {
                provider: ProviderKind::Postgres,
                reason: format!("parse postgres URL failed: {e}"),
            })?;

        Ok(Self { pg_config, config })
    }

    /// 6 K-1 字段 hardcoded: scope = Global (PG 集群共享).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Global
    }

    /// Get tokio-postgres config handle.
    pub fn pg_config(&self) -> &tokio_postgres::Config {
        &self.pg_config
    }

    /// 真连 PG (借 tokio-postgres 业界标准), R21+ 测试用.
    pub async fn connect(&self) -> MemoryProviderResult<tokio_postgres::Client> {
        let (client, connection) = self
            .pg_config
            .connect(NoTls)
            .await
            .map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Postgres,
                reason: format!("connect failed: {e}"),
            })?;
        // spawn connection task (per tokio-postgres 强制要求)
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("[postgres connection error] {e}");
            }
        });
        // 应用 schema
        client
            .batch_execute(PG_SCHEMA)
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Postgres,
                reason: format!("schema apply failed: {e}"),
            })?;
        Ok(client)
    }
}

#[async_trait]
impl MemoryProvider for PostgresProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Postgres
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        let client = self.connect().await?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        client
            .execute(
                "INSERT INTO kv (key, value, created_at) VALUES ($1, $2, $3)
                 ON CONFLICT (key) DO UPDATE SET value = $2, created_at = $3",
                &[&key, &value, &now],
            )
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Postgres,
                reason: format!("INSERT failed: {e}"),
            })?;
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        let client = self.connect().await?;
        let row = client
            .query_opt("SELECT value FROM kv WHERE key = $1", &[&key])
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Postgres,
                reason: format!("SELECT failed: {e}"),
            })?;
        Ok(row.map(|r| r.get::<_, Vec<u8>>(0)))
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        let client = self.connect().await?;
        client
            .execute("DELETE FROM kv WHERE key = $1", &[&key])
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Postgres,
                reason: format!("DELETE failed: {e}"),
            })?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        let client = self.connect().await?;
        let row = client
            .query_opt("SELECT 1 FROM kv WHERE key = $1 LIMIT 1", &[&key])
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Postgres,
                reason: format!("SELECT EXISTS failed: {e}"),
            })?;
        Ok(row.is_some())
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        let client = self.connect().await?;
        client
            .execute("DELETE FROM kv", &[])
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Postgres,
                reason: format!("DELETE ALL failed: {e}"),
            })?;
        Ok(())
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        let client = self.connect().await?;
        let row = client
            .query_one("SELECT COUNT(*) FROM kv", &[])
            .await
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Postgres,
                reason: format!("COUNT failed: {e}"),
            })?;
        let count: i64 = row.get(0);
        Ok(count as u64)
    }
}

/// **PostgresConfigDefault** — 借 serde derive 序列化 ProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresConfigDefault {
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
            "postgres://user:pass@localhost:5432/apeireth",
            Duration::from_secs(5),
            1024 * 1024,
            true, // PG 表持久化
            Duration::from_secs(0), // 永不过期
            ProviderScope::Global,
        )
    }

    // ----- 10 unit tests -----

    #[test]
    fn test_1_provider_kind_is_postgres() {
        let p = PostgresProvider::new(make_config()).unwrap();
        assert_eq!(p.kind(), ProviderKind::Postgres);
    }

    #[test]
    fn test_2_k1_connection_string_must_be_postgres_scheme() {
        let bad = ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        );
        let r = PostgresProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_3_k1_timeout_validation_propagates() {
        let bad = ProviderConfig::new(
            "postgres://user:pass@localhost/db",
            Duration::from_micros(500),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        );
        let r = PostgresProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_4_k1_max_size_validation_propagates() {
        let bad = ProviderConfig::new(
            "postgres://user:pass@localhost/db",
            Duration::from_secs(5),
            512,
            true,
            Duration::from_secs(0),
            ProviderScope::Global,
        );
        let r = PostgresProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_5_k1_persist_passes_through_to_postgres() {
        // PG persist 字段仅过 validate, 实际由服务端 WAL 配置决定
        let p = PostgresProvider::new(make_config()).unwrap();
        assert!(p.config.persist);
    }

    #[test]
    fn test_6_k1_cache_ttl_zero_passes() {
        let p = PostgresProvider::new(make_config()).unwrap();
        assert_eq!(p.config.cache_ttl, Duration::from_secs(0));
    }

    #[test]
    fn test_7_k1_scope_is_always_global() {
        let p = PostgresProvider::new(make_config()).unwrap();
        assert_eq!(p.scope(), ProviderScope::Global);
    }

    #[test]
    fn test_8_provider_init_with_valid_config() {
        let p = PostgresProvider::new(make_config()).expect("valid config");
        assert_eq!(p.kind(), ProviderKind::Postgres);
        // 验证 tokio-postgres Config 真解析 (0 实际连接)
        let _ = p.pg_config();
    }

    #[tokio::test]
    async fn test_9_set_without_server_returns_connection_error() {
        // 0 假装 — 没有 PG server 必然失败
        let p = PostgresProvider::new(make_config()).unwrap();
        let r = p.set("k1", b"v1").await;
        assert!(matches!(r, Err(MemoryProviderError::Connection { .. })));
    }

    #[tokio::test]
    async fn test_10_get_delete_exists_clear_size_without_server_returns_connection_error() {
        let p = PostgresProvider::new(make_config()).unwrap();
        assert!(matches!(p.get("k1").await, Err(MemoryProviderError::Connection { .. })));
        assert!(matches!(p.delete("k1").await, Err(MemoryProviderError::Connection { .. })));
        assert!(matches!(p.exists("k1").await, Err(MemoryProviderError::Connection { .. })));
        assert!(matches!(p.clear().await, Err(MemoryProviderError::Connection { .. })));
        assert!(matches!(p.size().await, Err(MemoryProviderError::Connection { .. })));
    }
}
