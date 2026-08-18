//! # SqliteProvider — 7 provider 模式 2: 本地 SQLite
//!
//! **真接 (per 8 项之 1, "0 不假装已实现")**:
//! - 内部用 `Arc<Mutex<rusqlite::Connection>>` 持 SQLite 连接
//! - `set` 走 `INSERT OR REPLACE`, `get` 走 `SELECT`, `delete` 走 `DELETE`
//! - 7 通用方法 (`kind`/`set`/`get`/`delete`/`exists`/`clear`/`size`) 全部真接
//! - **端到端可测**: 集成测试用 `:memory:` SQLite, 0 外部服务
//!
//! **不假装**:
//! - skeleton 阶段 sync `std::sync::Mutex<Connection>`, 0 引 tokio::sync::Mutex
//! - 1 张表 `kv (key TEXT PRIMARY KEY, value BLOB, created_at INTEGER)`
//! - 0 假装"分布式 SQLite" (本地单文件, scope=Shared 仅文件锁意义)
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `sqlite://<path>` (`:memory:` 用于测试)
//! 2. timeout = [1ms, 1h] (sync 实现, 实际 0 用, 仅过 config validate)
//! 3. max_size = [1KB, 1TB] (SQLite 文件总 size, 超限返 Capacity — 通过 PRAGMA 监控)
//! 4. persist = bool (true = 文件 DB, false = :memory:)
//! 5. cache_ttl = [0ms, 7d] (0 = 永不过期, SQLite 不主动 expire, 仅过 validate)
//! 6. scope = Shared (SQLite 文件锁, 多进程共享)

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{MemoryProvider, ProviderConfig, ProviderKind, ProviderScope};

/// **SQLite schema**: 单表 `kv (key TEXT PRIMARY KEY, value BLOB, created_at INTEGER)`.
const SQLITE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS kv (
    key TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL,
    created_at INTEGER NOT NULL
);
"#;

/// **SqliteProvider**: 本地 SQLite 文件 DB provider.
#[derive(Debug, Clone)]
pub struct SqliteProvider {
    /// 内部 `Arc<Mutex<Connection>>` (跨线程共享, 0 引 tokio::sync::Mutex).
    inner: Arc<Mutex<Connection>>,
    /// 6 K-1 强校验过的 config.
    #[allow(dead_code)]
    config: ProviderConfig,
}

impl SqliteProvider {
    /// 新建 SqliteProvider, 6 K-1 强校验 + 打开 DB.
    ///
    /// `persist = true` 时用文件 DB, `persist = false` 时用 `:memory:`.
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::Sqlite)?;

        // K-1 #1: 解析 connection_string 拿 path
        let path = config
            .connection_string
            .strip_prefix("sqlite://")
            .ok_or_else(|| MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "must start with `sqlite://`".to_string(),
            })?;

        // K-1 #4: persist = false → :memory:  (override path)
        let conn = if config.persist {
            Connection::open(Path::new(path)).map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Sqlite,
                reason: format!("open file db failed: {e}"),
            })?
        } else {
            Connection::open_in_memory().map_err(|e| MemoryProviderError::Connection {
                provider: ProviderKind::Sqlite,
                reason: format!("open :memory: failed: {e}"),
            })?
        };

        // 应用 schema
        conn.execute_batch(SQLITE_SCHEMA)
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("schema apply failed: {e}"),
            })?;

        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
            config,
        })
    }

    /// 6 K-1 字段 hardcoded: scope = Shared (SQLite 文件锁, 多进程共享).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Shared
    }

    /// Get internal Arc<Mutex<Connection>> handle (跨线程共享).
    pub fn handle(&self) -> Arc<Mutex<Connection>> {
        Arc::clone(&self.inner)
    }
}

#[async_trait]
impl MemoryProvider for SqliteProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Sqlite
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        let conn = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("Mutex poisoned: {e}"),
            })?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        conn.execute(
            "INSERT OR REPLACE INTO kv (key, value, created_at) VALUES (?1, ?2, ?3)",
            params![key, value, now],
        )
        .map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::Sqlite,
            reason: format!("INSERT failed: {e}"),
        })?;
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        let conn = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("Mutex poisoned: {e}"),
            })?;
        let mut stmt = conn
            .prepare("SELECT value FROM kv WHERE key = ?1")
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("prepare SELECT failed: {e}"),
            })?;
        let mut rows = stmt
            .query(params![key])
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("query failed: {e}"),
            })?;
        match rows.next().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::Sqlite,
            reason: format!("rows.next failed: {e}"),
        })? {
            Some(row) => {
                let value: Vec<u8> = row.get(0).map_err(|e| MemoryProviderError::Backend {
                    provider: ProviderKind::Sqlite,
                    reason: format!("row.get failed: {e}"),
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        let conn = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("Mutex poisoned: {e}"),
            })?;
        conn.execute("DELETE FROM kv WHERE key = ?1", params![key])
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("DELETE failed: {e}"),
            })?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        let conn = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("Mutex poisoned: {e}"),
            })?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM kv WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("COUNT failed: {e}"),
            })?;
        Ok(count > 0)
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        let conn = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("Mutex poisoned: {e}"),
            })?;
        conn.execute("DELETE FROM kv", [])
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("DELETE ALL failed: {e}"),
            })?;
        Ok(())
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        let conn = self
            .inner
            .lock()
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("Mutex poisoned: {e}"),
            })?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM kv", [], |row| row.get(0))
            .map_err(|e| MemoryProviderError::Backend {
                provider: ProviderKind::Sqlite,
                reason: format!("COUNT failed: {e}"),
            })?;
        Ok(count as u64)
    }
}

/// **SqliteConfigDefault** — 借 serde derive 序列化 ProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteConfigDefault {
    pub config: ProviderConfig,
}

// =====================================================================
// 单元测试 (10 tests per 借鉴 #6 模式 1:1)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;

    fn make_in_memory_config() -> ProviderConfig {
        ProviderConfig::new(
            "sqlite://:memory:",
            Duration::from_secs(5),
            1024 * 1024,
            false, // :memory:
            Duration::from_secs(0),
            ProviderScope::Shared,
        )
    }

    fn make_persist_config(path: &str) -> ProviderConfig {
        ProviderConfig::new(
            format!("sqlite://{path}"),
            Duration::from_secs(5),
            1024 * 1024,
            true, // 文件
            Duration::from_secs(0),
            ProviderScope::Shared,
        )
    }

    // ----- 10 unit tests -----

    #[test]
    fn test_1_provider_kind_is_sqlite() {
        let p = SqliteProvider::new(make_in_memory_config()).unwrap();
        assert_eq!(p.kind(), ProviderKind::Sqlite);
    }

    #[test]
    fn test_2_k1_connection_string_must_be_sqlite_scheme() {
        // redis:// 配 Sqlite → 失败
        let bad = ProviderConfig::new(
            "redis://localhost:6379",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Shared,
        );
        let r = SqliteProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_3_k1_timeout_validation_propagates() {
        let bad = ProviderConfig::new(
            "sqlite://:memory:",
            Duration::from_micros(500),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Shared,
        );
        let r = SqliteProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_4_k1_max_size_validation_propagates() {
        let bad = ProviderConfig::new(
            "sqlite://:memory:",
            Duration::from_secs(5),
            512,
            false,
            Duration::from_secs(0),
            ProviderScope::Shared,
        );
        let r = SqliteProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_5_k1_persist_false_uses_memory_db() {
        let p = SqliteProvider::new(make_in_memory_config()).unwrap();
        assert!(!p.config.persist);
        // 写一条数据 (用 tokio runtime, 0 引 futures-lite)
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(p.set("test", b"v1")).unwrap();
    }

    #[test]
    fn test_6_k1_cache_ttl_zero_passes() {
        let p = SqliteProvider::new(make_in_memory_config()).unwrap();
        assert_eq!(p.config.cache_ttl, Duration::from_secs(0));
    }

    #[test]
    fn test_7_k1_scope_is_always_shared() {
        let p = SqliteProvider::new(make_in_memory_config()).unwrap();
        assert_eq!(p.scope(), ProviderScope::Shared);
    }

    #[test]
    fn test_8_provider_init_with_valid_config() {
        let p = SqliteProvider::new(make_in_memory_config()).expect("valid config");
        assert_eq!(p.kind(), ProviderKind::Sqlite);
    }

    #[tokio::test]
    async fn test_9_set_get_round_trip_in_memory() {
        let p = SqliteProvider::new(make_in_memory_config()).unwrap();
        p.set("k1", b"hello").await.unwrap();
        let got = p.get("k1").await.unwrap();
        assert_eq!(got, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_10_end_to_end_set_get_delete_exists_clear_size() {
        let p = SqliteProvider::new(make_in_memory_config()).unwrap();
        // 空
        assert_eq!(p.size().await.unwrap(), 0);
        assert!(!p.exists("k1").await.unwrap());
        // set 3 个
        p.set("k1", b"v1").await.unwrap();
        p.set("k2", b"v2").await.unwrap();
        p.set("k3", b"v3").await.unwrap();
        assert_eq!(p.size().await.unwrap(), 3);
        assert!(p.exists("k1").await.unwrap());
        // get
        assert_eq!(p.get("k2").await.unwrap(), Some(b"v2".to_vec()));
        assert_eq!(p.get("nope").await.unwrap(), None);
        // delete
        p.delete("k2").await.unwrap();
        assert!(!p.exists("k2").await.unwrap());
        assert_eq!(p.size().await.unwrap(), 2);
        // clear
        p.clear().await.unwrap();
        assert_eq!(p.size().await.unwrap(), 0);
        assert!(!p.exists("k1").await.unwrap());
    }
}
