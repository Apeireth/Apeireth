//! # ProviderFactory — 9 provider 统一构造器 + from_env 接线工厂
//!
//! "telemetry cache 接线"派工的构造入口: 一个函数按 [`ProviderKind`] 构造任意 provider
//! (统一 `Arc<dyn MemoryProvider>`), 一个 `from_env` 从环境变量装配 (接线入口, 供
//! telemetry/cache 或高级用户按配置选后端, 0 硬编码 provider 选择).
//!
//! ## from_env 环境变量契约 (全可选, 0 配置默认 `in_memory`)
//!
//! | 变量 | 默认 | 说明 |
//! |---|---|---|
//! | `APEIRETH_MEMORY_PROVIDER` | `in_memory` | provider 名 (per [`ProviderKind::from_name`]) |
//! | `APEIRETH_MEMORY_CONNECTION` | per-kind 默认 | connection_string (协议头须匹配 kind) |
//! | `APEIRETH_MEMORY_TIMEOUT_MS` | `5000` | 超时 (1ms..=1h) |
//! | `APEIRETH_MEMORY_MAX_SIZE` | `1048576` | 容量上限 bytes (1KB..=1TB) |
//! | `APEIRETH_MEMORY_PERSIST` | per-kind (持久化 kind=true) | `true`/`false` |
//! | `APEIRETH_MEMORY_CACHE_TTL_MS` | `0` | 缓存 TTL (0=永不过期, ≤7d) |
//! | `APEIRETH_MEMORY_SCOPE` | per-kind | `local`/`shared`/`global` |
//!
//! per-kind 默认 connection (仅保证 6 K-1 校验通过; 网络类 provider 连通由调用方配置负责):
//! `in_memory`→`memory://`, `redis`→`redis://localhost:6379/0`, `sqlite`→`sqlite://:memory:`,
//! `postgres`→`postgres://localhost:5432/apeireth`, `s3`→`s3://apeireth/apeireth`,
//! `disk_lru`→`file://./apeireth-memory-cache`, `hybrid`→`hybrid://memory+disk`,
//! `file`→`file-jsonl://./apeireth-memory.jsonl`, `mongodb`→`mongodb://localhost:27017/apeireth`.
//!
//! **不假装**: `from_env` 只负责**构造** (6 K-1 校验 + provider 实例化), 0 假装连通:
//! Redis/Postgres/S3/MongoDb 无服务端时后续 `set/get` 仍返 Connection 错 (per provider_*.rs 现状).

use std::sync::Arc;
use std::time::Duration;

use crate::cache_layer::is_persistent;
use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{
    MemoryProvider, ProviderConfig, ProviderConfigField, ProviderKind, ProviderScope,
};
use crate::provider_disk_lru::DiskLruProvider;
use crate::provider_file::FileProvider;
use crate::provider_hybrid::HybridProvider;
use crate::provider_in_memory::InMemoryProvider;
use crate::provider_mongodb::MongoDbProvider;
use crate::provider_postgres::PostgresProvider;
use crate::provider_redis::RedisProvider;
use crate::provider_s3::S3Provider;
use crate::provider_sqlite::SqliteProvider;

/// env: provider 名 (per `ProviderKind::from_name`).
pub const ENV_PROVIDER: &str = "APEIRETH_MEMORY_PROVIDER";
/// env: connection_string (per-kind 协议头).
pub const ENV_CONNECTION: &str = "APEIRETH_MEMORY_CONNECTION";
/// env: timeout ms.
pub const ENV_TIMEOUT_MS: &str = "APEIRETH_MEMORY_TIMEOUT_MS";
/// env: max_size bytes.
pub const ENV_MAX_SIZE: &str = "APEIRETH_MEMORY_MAX_SIZE";
/// env: persist bool.
pub const ENV_PERSIST: &str = "APEIRETH_MEMORY_PERSIST";
/// env: cache_ttl ms.
pub const ENV_CACHE_TTL_MS: &str = "APEIRETH_MEMORY_CACHE_TTL_MS";
/// env: scope (local/shared/global).
pub const ENV_SCOPE: &str = "APEIRETH_MEMORY_SCOPE";

/// **ProviderFactory**: 9 provider 统一构造器 (0 state, 纯函数集).
#[derive(Debug, Clone, Copy, Default)]
pub struct ProviderFactory;

impl ProviderFactory {
    /// 按 kind 构造 provider (统一 `Arc<dyn MemoryProvider>`).
    ///
    /// 9 kind 全覆盖 (InMemory/Redis/Sqlite/Postgres/S3/DiskLru/Hybrid/File/MongoDb),
    /// 全部走各自 6 K-1 强校验构造器 (per provider_*.rs).
    pub fn build(
        kind: ProviderKind,
        config: ProviderConfig,
    ) -> MemoryProviderResult<Arc<dyn MemoryProvider>> {
        let provider: Arc<dyn MemoryProvider> = match kind {
            ProviderKind::InMemory => Arc::new(InMemoryProvider::new(config)?),
            ProviderKind::Redis => Arc::new(RedisProvider::new(config)?),
            ProviderKind::Sqlite => Arc::new(SqliteProvider::new(config)?),
            ProviderKind::Postgres => Arc::new(PostgresProvider::new(config)?),
            ProviderKind::S3 => Arc::new(S3Provider::new(config)?),
            ProviderKind::DiskLru => Arc::new(DiskLruProvider::new(config)?),
            ProviderKind::Hybrid => Arc::new(HybridProvider::new(config)?),
            ProviderKind::File => Arc::new(FileProvider::new(config)?),
            ProviderKind::MongoDb => Arc::new(MongoDbProvider::new(config)?),
        };
        Ok(provider)
    }

    /// 按名称构造 provider (per [`ProviderKind::from_name`]).
    ///
    /// 未知名称 → `Err(MemoryProviderError::Other)` (带合法名称列表).
    pub fn build_named(
        name: &str,
        config: ProviderConfig,
    ) -> MemoryProviderResult<Arc<dyn MemoryProvider>> {
        let kind = Self::parse_kind_name(name)?;
        Self::build(kind, config)
    }

    /// 解析 provider 名称 → kind (未知名称报错, 合法列表随错误给出).
    pub fn parse_kind_name(name: &str) -> MemoryProviderResult<ProviderKind> {
        ProviderKind::from_name(name).ok_or_else(|| {
            let valid = ProviderKind::ALL
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            MemoryProviderError::Other {
                msg: format!("unknown memory provider name `{name}` (valid: {valid})"),
            }
        })
    }

    /// 从真实进程环境变量装配 (per 顶部契约表).
    pub fn from_env() -> MemoryProviderResult<(ProviderKind, Arc<dyn MemoryProvider>)> {
        Self::from_env_with(|key| std::env::var(key).ok())
    }

    /// 从可注入 env 装配 (测试用; `from_env` = 本函数 + 真实 env).
    ///
    /// `env` 闭包: 变量名 → `Some(value)` / `None` (未设).
    /// 返回 `(kind, provider)`; 0 env 配置时默认 `in_memory`.
    pub fn from_env_with<F>(env: F) -> MemoryProviderResult<(ProviderKind, Arc<dyn MemoryProvider>)>
    where
        F: Fn(&str) -> Option<String>,
    {
        let (kind, config) = Self::from_env_config(&env)?;
        let provider = Self::build(kind, config)?;
        Ok((kind, provider))
    }

    /// 从可注入 env 只装配 config (0 构造 provider, 测试/预检用).
    ///
    /// 返回 `(kind, config)`; config 已过 6 K-1 强校验.
    pub fn from_env_config<F>(env: &F) -> MemoryProviderResult<(ProviderKind, ProviderConfig)>
    where
        F: Fn(&str) -> Option<String>,
    {
        // 1. kind (默认 in_memory)
        let kind_name = env(ENV_PROVIDER).unwrap_or_else(|| "in_memory".to_string());
        let kind = Self::parse_kind_name(&kind_name)?;

        // 2. 6 K-1 字段 (默认 per-kind, 覆盖值解析失败报 Config)
        let connection =
            env(ENV_CONNECTION).unwrap_or_else(|| default_connection(kind).to_string());
        let timeout_ms = parse_u64(
            env(ENV_TIMEOUT_MS),
            5000,
            ENV_TIMEOUT_MS,
            ProviderConfigField::Timeout,
        )?;
        let max_size = parse_u64(
            env(ENV_MAX_SIZE),
            1024 * 1024,
            ENV_MAX_SIZE,
            ProviderConfigField::MaxSize,
        )?;
        let persist = parse_bool(env(ENV_PERSIST), default_persist(kind), ENV_PERSIST)?;
        let cache_ttl_ms = parse_u64(
            env(ENV_CACHE_TTL_MS),
            0,
            ENV_CACHE_TTL_MS,
            ProviderConfigField::CacheTtl,
        )?;
        let scope = parse_scope(env(ENV_SCOPE), default_scope(kind))?;

        let config = ProviderConfig::new(
            connection,
            Duration::from_millis(timeout_ms),
            max_size,
            persist,
            Duration::from_millis(cache_ttl_ms),
            scope,
        );
        // 6 K-1 强校验 (timeout>=1ms / max_size>=1KB / ttl<=7d / scheme 匹配)
        config.validate(kind)?;
        Ok((kind, config))
    }
}

/// per-kind 默认 connection_string (0 保证连通, 只保证 6 K-1 校验通过).
fn default_connection(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::InMemory => "memory://",
        ProviderKind::Redis => "redis://localhost:6379/0",
        ProviderKind::Sqlite => "sqlite://:memory:",
        ProviderKind::Postgres => "postgres://localhost:5432/apeireth",
        ProviderKind::S3 => "s3://apeireth/apeireth",
        ProviderKind::DiskLru => "file://./apeireth-memory-cache",
        ProviderKind::Hybrid => "hybrid://memory+disk",
        ProviderKind::File => "file-jsonl://./apeireth-memory.jsonl",
        ProviderKind::MongoDb => "mongodb://localhost:27017/apeireth",
    }
}

/// per-kind 默认 persist (持久化能力 = true, cache 层 = false).
fn default_persist(kind: ProviderKind) -> bool {
    is_persistent(kind)
}

/// per-kind 默认 scope (本地/共享/全局, 跟集成测试映射一致).
fn default_scope(kind: ProviderKind) -> ProviderScope {
    match kind {
        ProviderKind::InMemory | ProviderKind::DiskLru | ProviderKind::File => ProviderScope::Local,
        ProviderKind::Sqlite | ProviderKind::Hybrid => ProviderScope::Shared,
        ProviderKind::Redis | ProviderKind::Postgres | ProviderKind::S3 | ProviderKind::MongoDb => {
            ProviderScope::Global
        }
    }
}

/// 解析 u64 env 值 (缺省返默认; 非法值报 Config, 定位到具体 K-1 字段).
fn parse_u64(
    v: Option<String>,
    default: u64,
    var: &str,
    field: ProviderConfigField,
) -> MemoryProviderResult<u64> {
    match v {
        None => Ok(default),
        Some(s) => s
            .trim()
            .parse::<u64>()
            .map_err(|e| MemoryProviderError::Config {
                field,
                reason: format!("{var} must be an unsigned integer, got `{s}` ({e})"),
            }),
    }
}

/// 解析 bool env 值 (`true`/`1`/`false`/`0`; 缺省返默认; 其他报 Config).
fn parse_bool(v: Option<String>, default: bool, var: &str) -> MemoryProviderResult<bool> {
    match v.as_deref() {
        None => Ok(default),
        Some("true") | Some("1") => Ok(true),
        Some("false") | Some("0") => Ok(false),
        Some(other) => Err(MemoryProviderError::Config {
            field: crate::memory_provider::ProviderConfigField::Persist,
            reason: format!("{var} must be true/false, got `{other}`"),
        }),
    }
}

/// 解析 scope env 值 (`local`/`shared`/`global`; 缺省返默认; 其他报 Config).
fn parse_scope(v: Option<String>, default: ProviderScope) -> MemoryProviderResult<ProviderScope> {
    match v.as_deref() {
        None => Ok(default),
        Some("local") => Ok(ProviderScope::Local),
        Some("shared") => Ok(ProviderScope::Shared),
        Some("global") => Ok(ProviderScope::Global),
        Some(other) => Err(MemoryProviderError::Config {
            field: crate::memory_provider::ProviderConfigField::Scope,
            reason: format!("scope must be local/shared/global, got `{other}`"),
        }),
    }
}

// =====================================================================
// 单元测试 (9 kind 构造 + env 装配 + 解析失败 = 15+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 测试用注入 env (闭包查表).
    fn env_of<'a>(map: &'a HashMap<&'static str, String>) -> impl Fn(&str) -> Option<String> + 'a {
        move |k: &str| map.get(k).cloned()
    }

    fn empty_env() -> impl Fn(&str) -> Option<String> {
        |_| None
    }

    #[test]
    fn build_all_9_kinds_constructible() {
        let cases: &[(ProviderKind, &str)] = &[
            (ProviderKind::InMemory, "memory://"),
            (ProviderKind::Redis, "redis://localhost:6379/0"),
            (ProviderKind::Sqlite, "sqlite://:memory:"),
            (ProviderKind::Postgres, "postgres://u:p@localhost/db"),
            (ProviderKind::S3, "s3://bucket/key"),
            (ProviderKind::DiskLru, "file://./target/factory-test-disk"),
            (ProviderKind::Hybrid, "hybrid://memory+disk"),
            (
                ProviderKind::File,
                "file-jsonl://./target/factory-test.jsonl",
            ),
            (ProviderKind::MongoDb, "mongodb://localhost:27017/apeireth"),
        ];
        for (kind, conn) in cases {
            let cfg = ProviderConfig::new(
                *conn,
                Duration::from_secs(5),
                1024 * 1024,
                true,
                Duration::from_secs(0),
                default_scope(*kind),
            );
            let p = ProviderFactory::build(*kind, cfg).expect("constructible");
            assert_eq!(p.kind(), *kind, "{kind:?} should build");
        }
    }

    #[test]
    fn build_named_dispatch_and_unknown_errors() {
        let cfg = ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let p = ProviderFactory::build_named("in_memory", cfg).unwrap();
        assert_eq!(p.kind(), ProviderKind::InMemory);

        let err = ProviderFactory::build_named(
            "nonexistent",
            ProviderConfig::new(
                "memory://",
                Duration::from_secs(5),
                1024 * 1024,
                false,
                Duration::from_secs(0),
                ProviderScope::Local,
            ),
        )
        .err()
        .unwrap();
        assert!(matches!(err, MemoryProviderError::Other { .. }));
        assert!(err.to_string().contains("valid"), "错误应带合法列表: {err}");
    }

    #[test]
    fn from_env_config_defaults_to_in_memory() {
        let (kind, cfg) = ProviderFactory::from_env_config(&empty_env()).unwrap();
        assert_eq!(kind, ProviderKind::InMemory);
        assert_eq!(cfg.connection_string, "memory://");
        assert_eq!(cfg.timeout, Duration::from_millis(5000));
        assert_eq!(cfg.max_size, 1024 * 1024);
        assert!(!cfg.persist, "InMemory 默认 persist=false (cache 层)");
        assert_eq!(cfg.cache_ttl, Duration::from_millis(0));
        assert_eq!(cfg.scope, ProviderScope::Local);
    }

    #[test]
    fn from_env_with_selects_kind() {
        let map: HashMap<&'static str, String> =
            HashMap::from([(ENV_PROVIDER, "sqlite".to_string())]);
        let (kind, p) = ProviderFactory::from_env_with(env_of(&map)).unwrap();
        assert_eq!(kind, ProviderKind::Sqlite);
        assert_eq!(p.kind(), ProviderKind::Sqlite);
    }

    #[test]
    fn from_env_with_connection_override() {
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "sqlite".to_string()),
            (
                ENV_CONNECTION,
                "sqlite:///tmp/apeireth-factory.db".to_string(),
            ),
        ]);
        let (kind, cfg) = ProviderFactory::from_env_config(&env_of(&map)).unwrap();
        assert_eq!(kind, ProviderKind::Sqlite);
        assert_eq!(cfg.connection_string, "sqlite:///tmp/apeireth-factory.db");
    }

    #[test]
    fn from_env_with_full_override() {
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "redis".to_string()),
            (ENV_CONNECTION, "redis://127.0.0.1:6380/2".to_string()),
            (ENV_TIMEOUT_MS, "1000".to_string()),
            (ENV_MAX_SIZE, "4096".to_string()),
            (ENV_PERSIST, "false".to_string()),
            (ENV_CACHE_TTL_MS, "60000".to_string()),
            (ENV_SCOPE, "shared".to_string()),
        ]);
        let (kind, cfg) = ProviderFactory::from_env_config(&env_of(&map)).unwrap();
        assert_eq!(kind, ProviderKind::Redis);
        assert_eq!(cfg.connection_string, "redis://127.0.0.1:6380/2");
        assert_eq!(cfg.timeout, Duration::from_millis(1000));
        assert_eq!(cfg.max_size, 4096);
        assert!(!cfg.persist);
        assert_eq!(cfg.cache_ttl, Duration::from_millis(60000));
        assert_eq!(cfg.scope, ProviderScope::Shared);
    }

    #[test]
    fn from_env_unknown_kind_errors() {
        let map: HashMap<&'static str, String> =
            HashMap::from([(ENV_PROVIDER, "oracle".to_string())]);
        let err = ProviderFactory::from_env_config(&env_of(&map)).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Other { .. }));
        assert!(err.to_string().contains("oracle"));
    }

    #[test]
    fn from_env_invalid_timeout_errors() {
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "in_memory".to_string()),
            (ENV_TIMEOUT_MS, "abc".to_string()),
        ]);
        let err = ProviderFactory::from_env_config(&env_of(&map)).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { .. }));
    }

    #[test]
    fn from_env_timeout_zero_fails_k1_validation() {
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "in_memory".to_string()),
            (ENV_TIMEOUT_MS, "0".to_string()),
        ]);
        let err = ProviderFactory::from_env_config(&env_of(&map)).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { .. }));
    }

    #[test]
    fn from_env_max_size_zero_fails_k1_validation() {
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "in_memory".to_string()),
            (ENV_MAX_SIZE, "0".to_string()),
        ]);
        let err = ProviderFactory::from_env_config(&env_of(&map)).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { .. }));
    }

    #[test]
    fn from_env_invalid_persist_errors() {
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "in_memory".to_string()),
            (ENV_PERSIST, "maybe".to_string()),
        ]);
        let err = ProviderFactory::from_env_config(&env_of(&map)).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { .. }));
    }

    #[test]
    fn from_env_invalid_scope_errors() {
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "in_memory".to_string()),
            (ENV_SCOPE, "cluster".to_string()),
        ]);
        let err = ProviderFactory::from_env_config(&env_of(&map)).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { .. }));
    }

    #[test]
    fn from_env_scheme_mismatch_fails_k1() {
        // redis:// 配 in_memory → scheme 不匹配 → Config err
        let map: HashMap<&'static str, String> = HashMap::from([
            (ENV_PROVIDER, "in_memory".to_string()),
            (ENV_CONNECTION, "redis://localhost:6379/0".to_string()),
        ]);
        let err = ProviderFactory::from_env_config(&env_of(&map)).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { .. }));
    }

    #[test]
    fn per_kind_default_connections_pass_validation() {
        for kind in ProviderKind::ALL {
            let (parsed, cfg) = ProviderFactory::from_env_config(&|k: &str| {
                if k == ENV_PROVIDER {
                    Some(kind.as_str().to_string())
                } else {
                    None
                }
            })
            .unwrap();
            assert_eq!(parsed, kind, "{kind:?} should be selectable with defaults");
            assert!(cfg.validate(kind).is_ok(), "{kind:?} default config valid");
        }
    }

    #[tokio::test]
    async fn from_env_default_returns_usable_provider() {
        let (kind, p) = ProviderFactory::from_env_with(empty_env()).unwrap();
        assert_eq!(kind, ProviderKind::InMemory);
        p.set("k", b"v").await.unwrap();
        assert_eq!(p.get("k").await.unwrap(), Some(b"v".to_vec()));
        assert_eq!(p.size().await.unwrap(), 1);
    }
}
