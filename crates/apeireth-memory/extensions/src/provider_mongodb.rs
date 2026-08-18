//! # MongoDbProvider — 9 provider 模式 8: 外部 MongoDB (skeleton)
//!
//! **不假装 (per 8 项之 1)**: 当前 0 接 mongodb client.
//! 所有 set/get/delete/exists/clear/size 调用立即返回
//! MemoryProviderError::Connection { reason: "skeleton: mongodb client not wired" }.
//! **0 fake success**, R23+ 续补: 引入 mongodb crate (tokio runtime), 配置
//! connection string + database + collection, 把 7 通用方法映射到
//! Collection::insert_one / find_one / delete_one / count_documents / drop.
//!
//! **不假装**:
//! - skeleton 阶段 new() 仅持 config (lazy connect), 0 假装"无 server 也能 set/get"
//! - 没服务端必然失败 (跟 Postgres 模式 1:1, 0 抄 Golutra 业务代码)
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `mongodb://[user:pass@]host:port/db` (mongodb URI)
//! 2. timeout = [1ms, 1h]
//! 3. max_size = [1KB, 1TB]
//! 4. persist = bool (true = MongoDB collection 持久化)
//! 5. cache_ttl = [0ms, 7d] (0 = 永不过期)
//! 6. scope = Global (MongoDB 集群共享)

use async_trait::async_trait;

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{MemoryProvider, ProviderConfig, ProviderKind, ProviderScope};

/// **MongoDB schema**: 单 collection `kv` (key TEXT PRIMARY KEY, value BYTEA-style BinData).
const MONGODB_COLLECTION: &str = "kv";

/// **MongoDbProvider**: 外部 MongoDB server provider (per R23 #6 派工 mongodb skeleton).
#[derive(Debug, Clone)]
pub struct MongoDbProvider {
    /// MongoDB connection string (e.g. `mongodb://localhost:27017`).
    /// R23+ 续真接时传给 `mongodb::Client::with_uri_str`.
    connection_string: String,
    /// Database name.
    database: String,
    /// Collection name.
    collection: String,
    /// 6 K-1 强校验过的 config.
    config: ProviderConfig,
}

impl MongoDbProvider {
    /// 新建 MongoDbProvider (skeleton, 0 接 client).
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::MongoDb)?;
        let conn = &config.connection_string;
        let (db_name, coll_name) = parse_mongodb_uri(conn)?;
        Ok(Self {
            connection_string: conn.clone(),
            database: db_name,
            collection: coll_name,
            config,
        })
    }

    /// MongoDB connection string (read-only).
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Database name (parsed from URI).
    pub fn database(&self) -> &str {
        &self.database
    }

    /// Collection name (parsed from URI or default `kv`).
    pub fn collection(&self) -> &str {
        &self.collection
    }

    /// 6 K-1 字段 hardcoded: persist = true (MongoDB 持久化默认).
    pub fn is_persistent(&self) -> bool {
        self.config.persist
    }

    /// 6 K-1 字段 hardcoded: scope = Global (MongoDB 集群共享).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Global
    }

    /// R23+ 续补标记: 当前 0 接 client, 显式失败.
    fn not_wired(&self, op: &str) -> MemoryProviderError {
        MemoryProviderError::Connection {
            provider: ProviderKind::MongoDb,
            reason: format!(
                "{op}: mongodb client not wired (R23+ 续补: 引入 mongodb crate,                  URI={}, db={}, coll={}, target_coll={})",
                self.connection_string, self.database, self.collection, MONGODB_COLLECTION
            ),
        }
    }
}

/// 解析 mongodb:// URI, 提取 db name (path) + collection (query `?collection=` 或默认 `kv`).
fn parse_mongodb_uri(conn: &str) -> MemoryProviderResult<(String, String)> {
    let prefix = "mongodb://";
    if !conn.starts_with(prefix) {
        return Err(MemoryProviderError::Config {
            field: crate::memory_provider::ProviderConfigField::ConnectionString,
            reason: format!("must start with `{prefix}`, got `{conn}`"),
        });
    }
    let after_scheme = &conn[prefix.len()..];
    let (path_part, query_part) = match after_scheme.find('?') {
        Some(idx) => (&after_scheme[..idx], Some(&after_scheme[idx + 1..])),
        None => (after_scheme, None),
    };
    let db = match path_part.find('/') {
        Some(idx) => &path_part[idx + 1..],
        None => "",
    };
    let db_name = if db.is_empty() {
        "apeireth".to_string()
    } else {
        db.to_string()
    };
    let coll_name = match query_part {
        Some(q) => {
            let mut coll = MONGODB_COLLECTION.to_string();
            for kv in q.split('&') {
                if let Some((k, v)) = kv.split_once('=') {
                    if k == "collection" && !v.is_empty() {
                        coll = v.to_string();
                    }
                }
            }
            coll
        }
        None => MONGODB_COLLECTION.to_string(),
    };
    if db_name.is_empty() {
        return Err(MemoryProviderError::Config {
            field: crate::memory_provider::ProviderConfigField::ConnectionString,
            reason: format!("db name missing in URI `{conn}`"),
        });
    }
    Ok((db_name, coll_name))
}

#[async_trait]
impl MemoryProvider for MongoDbProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::MongoDb
    }

    async fn set(&self, _key: &str, _value: &[u8]) -> MemoryProviderResult<()> {
        Err(self.not_wired("set"))
    }

    async fn get(&self, _key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        Err(self.not_wired("get"))
    }

    async fn delete(&self, _key: &str) -> MemoryProviderResult<()> {
        Err(self.not_wired("delete"))
    }

    async fn exists(&self, _key: &str) -> MemoryProviderResult<bool> {
        Err(self.not_wired("exists"))
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        Err(self.not_wired("clear"))
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        Err(self.not_wired("size"))
    }
}

/// **MongoDbConfigDefault**: MongoDbProvider 6 K-1 默认 config 构造器.
pub struct MongoDbConfigDefault;

impl MongoDbConfigDefault {
    /// 默认 config (connection_string = `mongodb://localhost:27017/apeireth?collection=kv`,
    /// timeout = 1s, max_size = 1MB, persist = true, cache_ttl = 0, scope = Global).
    pub fn build() -> ProviderConfig {
        ProviderConfig::new(
            "mongodb://localhost:27017/apeireth?collection=kv",
            std::time::Duration::from_secs(1),
            1024 * 1024,
            true,
            std::time::Duration::ZERO,
            ProviderScope::Global,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skeleton() -> MongoDbProvider {
        MongoDbProvider::new(MongoDbConfigDefault::build()).unwrap()
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn kind_is_mongodb() {
        let p = skeleton();
        assert_eq!(p.kind(), ProviderKind::MongoDb);
    }

    #[test]
    fn set_errors_skeleton() {
        let p = skeleton();
        runtime().block_on(async {
            let err = p.set("k", b"v").await.unwrap_err();
            match err {
                MemoryProviderError::Connection { provider, reason } => {
                    assert_eq!(provider, ProviderKind::MongoDb);
                    assert!(reason.contains("mongodb client not wired"));
                    assert!(reason.contains("set"));
                }
                _ => panic!("expected MemoryProviderError::Connection"),
            }
        });
    }

    #[test]
    fn get_errors_skeleton() {
        let p = skeleton();
        runtime().block_on(async {
            let err = p.get("k").await.unwrap_err();
            match err {
                MemoryProviderError::Connection { provider, reason } => {
                    assert_eq!(provider, ProviderKind::MongoDb);
                    assert!(reason.contains("mongodb client not wired"));
                    assert!(reason.contains("get"));
                }
                _ => panic!("expected MemoryProviderError::Connection"),
            }
        });
    }

    #[test]
    fn delete_errors_skeleton() {
        let p = skeleton();
        runtime().block_on(async {
            let err = p.delete("k").await.unwrap_err();
            match err {
                MemoryProviderError::Connection { reason, .. } => {
                    assert!(reason.contains("delete"));
                }
                _ => panic!("expected Connection error"),
            }
        });
    }

    #[test]
    fn exists_errors_skeleton() {
        let p = skeleton();
        runtime().block_on(async {
            let err = p.exists("k").await.unwrap_err();
            match err {
                MemoryProviderError::Connection { reason, .. } => {
                    assert!(reason.contains("exists"));
                }
                _ => panic!("expected Connection error"),
            }
        });
    }

    #[test]
    fn clear_errors_skeleton() {
        let p = skeleton();
        runtime().block_on(async {
            let err = p.clear().await.unwrap_err();
            match err {
                MemoryProviderError::Connection { reason, .. } => {
                    assert!(reason.contains("clear"));
                }
                _ => panic!("expected Connection error"),
            }
        });
    }

    #[test]
    fn size_errors_skeleton() {
        let p = skeleton();
        runtime().block_on(async {
            let err = p.size().await.unwrap_err();
            match err {
                MemoryProviderError::Connection { reason, .. } => {
                    assert!(reason.contains("size"));
                }
                _ => panic!("expected Connection error"),
            }
        });
    }

    #[test]
    fn accessors_return_parsed_uri() {
        let cfg = ProviderConfig::new(
            "mongodb://example-host:27018/my_db?collection=my_coll",
            std::time::Duration::from_millis(100),
            1024 * 1024,
            true,
            std::time::Duration::ZERO,
            ProviderScope::Global,
        );
        let p = MongoDbProvider::new(cfg).unwrap();
        assert_eq!(
            p.connection_string(),
            "mongodb://example-host:27018/my_db?collection=my_coll"
        );
        assert_eq!(p.database(), "my_db");
        assert_eq!(p.collection(), "my_coll");
    }

    #[test]
    fn skeleton_error_message_includes_uri_and_collection() {
        let cfg = ProviderConfig::new(
            "mongodb://audit-host:27017/audit_db?collection=audit_coll",
            std::time::Duration::from_millis(100),
            1024 * 1024,
            true,
            std::time::Duration::ZERO,
            ProviderScope::Global,
        );
        let p = MongoDbProvider::new(cfg).unwrap();
        runtime().block_on(async {
            let err = p.set("k", b"v").await.unwrap_err();
            let msg = format!("{err:?}");
            assert!(
                msg.contains("audit-host"),
                "URI 必须在 skeleton msg 中: {msg}"
            );
            assert!(msg.contains("audit_db"));
            assert!(msg.contains("audit_coll"));
            assert!(msg.contains("kv"), "target collection 默认 kv 在 msg 中");
        });
    }

    #[test]
    fn scope_is_global() {
        let p = skeleton();
        assert_eq!(p.scope(), ProviderScope::Global);
    }

    #[test]
    fn default_config_parses() {
        let cfg = MongoDbConfigDefault::build();
        let p = MongoDbProvider::new(cfg).unwrap();
        assert_eq!(p.database(), "apeireth");
        assert_eq!(p.collection(), "kv");
    }
}
