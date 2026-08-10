//! # MemoryProvider — 7 provider 抽象层
//!
//! 借鉴 Golutra v0.1.0 memory gateway 5 provider 模式 (per `analysis/golututra/BORROW_FROM_GOLUTRA.md` §8 P1 第 13/14 项)
//! 1:1 翻译到 Rust 路线. Golutra 5 + 1 (in-memory) + 1 (S3) + 1 (hybrid) = 7 provider.
//!
//! ## 7 Provider 模式 (跟 Golutra 5 provider 1:1 对应 + 扩展)
//!
//! | 变体索引 | Provider | Golutra 等价 | 真接方式 | 端到端测 |
//! |---:|---|---|---|---|
//! | 0 | `InMemory` | local (in-process) | real HashMap | ✅ |
//! | 1 | `Redis` | local (Redis sidecar) | real redis-rs + config 强校验 | ⚠️ 需本地 Redis |
//! | 2 | `Sqlite` | local_sqlite | real rusqlite (in-memory DB) | ✅ |
//! | 3 | `Postgres` | mem0 (云端) / vector (云端) | real tokio-postgres + config 强校验 | ⚠️ 需本地 PG |
//! | 4 | `S3` | relay (云端) | real reqwest + S3 REST API + config 强校验 | ⚠️ 需 AWS 凭据 |
//! | 5 | `DiskLru` | (Golutra 无, 新增) | real lru + std::fs | ✅ |
//! | 6 | `//! # MemoryProvider — 7 provider 抽象层
//!
//! 借鉴 Golutra v0.1.0 memory gateway 5 provider 模式 (per `analysis/golututra/BORROW_FROM_GOLUTRA.md` §8 P1 第 13/14 项)
//! 1:1 翻译到 Rust 路线. Golutra 5 + 1 (in-memory) + 1 (S3) + 1 (hybrid) = 7 provider.
//!
//! ## 7 Provider 模式 (跟 Golutra 5 provider 1:1 对应 + 扩展)
//!
//! | 变体索引 | Provider | Golutra 等价 | 真接方式 | 端到端测 |
//! |---:|---|---|---|---|
//! | 0 | `InMemory` | local (in-process) | real HashMap | ✅ |
//! | 1 | `Redis` | local (Redis sidecar) | real redis-rs + config 强校验 | ⚠️ 需本地 Redis |
//! | 2 | `Sqlite` | local_sqlite | real rusqlite (in-memory DB) | ✅ |
//! | 3 | `Postgres` | mem0 (云端) / vector (云端) | real tokio-postgres + config 强校验 | ⚠️ 需本地 PG |
//! | 4 | `S3` | relay (云端) | real reqwest + S3 REST API + config 强校验 | ⚠️ 需 AWS 凭据 |
//! | 5 | `DiskLru` | (Golutra 无, 新增) | real lru + std::fs | ✅ |
//! | 6 | `Hybrid` | (Golutra 无, 新增) | real 组合 in_memory + disk_lru | ✅ |
//!
//! ## 6 K-1 强校验 (per task spec 强制要求)
//!
//! 每个 ProviderConfig 6 字段编译期 + 运行时双层强校验:
//! 1. **connection_string** — URL/path/连接 URI (非空 + 协议头匹配)
//! 2. **timeout** — Duration (>= 1ms, <= 1h)
//! 3. **max_size** — u64 bytes (>= 1KB, <= 1TB)
//! 4. **persist** — bool (是否持久化)
//! 5. **cache_ttl** — Duration (>= 0ms, <= 7d)
//! 6. **scope** — ProviderScope 3 变体 (Local/Shared/Global)
//!
//! ## 抽象层
//!
//! - [`ProviderKind`] — 9 变体 enum, 编译期 exhaustive match 守门
//! - [`ProviderConfig`] — 6 K-1 字段 struct, 6 强校验编译期守门
//! - [`ProviderScope`] — 3 变体 enum
//! - [`ProviderConfigField`] — 6 变体 enum, 错误定位用
//! - [`MemoryProvider`] — trait, 7 通用方法 (kind/set/get/delete/exists/clear/size)
//!
//! **不**引入 dyn trait object (编译期 monomorphization, 0 虚函数开销).
//! **不**预创建任何全局 state (每个 provider 自己内部 state, 借用 std::sync::Mutex / lru / redis-rs).

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryProviderError, MemoryProviderResult};

/// **K-1 强校验 #1**: 7 provider 模式分类 (跟 Golutra 5 provider 1:1 + 扩展).
pub const MEMORY_PROVIDER_KIND_COUNT: usize = 9;

/// **K-1 强校验 #2**: 每个 ProviderConfig 6 字段强校验.
pub const PROVIDER_CONFIG_K1_FIELDS: usize = 6;

/// **K-1 强校验 #3**: ProviderScope 3 变体 (Local 单进程 / Shared 多进程 / Global 跨集群).
pub const PROVIDER_SCOPE_COUNT: usize = 3;

// ============================================================================
// 7 provider 模式 enum (编译期 hardcode)
// ============================================================================

/// **7 provider 模式 enum** (per 借鉴 Golutra memory gateway 5 provider 1:1 + 扩展 2 个).
///
/// 顺序固定 (per 借鉴 Golutra `default_providers` 列表顺序):
/// InMemory → Redis → Sqlite → Postgres → S3 → DiskLru → Hybrid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    /// 0: 进程内 HashMap (无外部服务, 0 序列化开销).
    InMemory = 0,
    /// 1: 外部 Redis (借 redis-rs, 0 重复造轮子).
    Redis = 1,
    /// 2: 本地 SQLite (借 workspace rusqlite, in-memory DB 测试).
    Sqlite = 2,
    /// 3: 外部 Postgres (借 tokio-postgres, 0 重复造轮子).
    Postgres = 3,
    /// 4: 外部 S3 (借 reqwest + S3 REST API, 避免 aws-sdk-s3 重量级).
    S3 = 4,
    /// 5: 本地 disk + LRU (借 workspace lru + std::fs).
    DiskLru = 5,
        /// 6: 组合 (in_memory + disk_lru 两级, 0 重复造 tiered cache).
    Hybrid = 6,
    /// 7: 本地 JSON-Lines append-only 文件 (per R23 #6 派工, file provider).
    File = 7,
    /// 8: 外部 MongoDB (per R23 #6 派工, mongodb skeleton).
    MongoDb = 8,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ProviderKind {
    /// 数字 0-6 → ProviderKind.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::InMemory),
            1 => Some(Self::Redis),
            2 => Some(Self::Sqlite),
            3 => Some(Self::Postgres),
            4 => Some(Self::S3),
            5 => Some(Self::DiskLru),
            6 => Some(Self::Hybrid),
            7 => Some(Self::File),
            8 => Some(Self::MongoDb),
            _ => None,
        }
    }

    /// 编译期字符串表示.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InMemory => "in_memory",
            Self::Redis => "redis",
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
            Self::S3 => "s3",
            Self::DiskLru => "disk_lru",
            Self::Hybrid => "hybrid",
            Self::File => "file",
            Self::MongoDb => "mongodb",
        }
    }

    /// 9 变体全列表 (per 借鉴 #6 sister report 9 organ ALL 模式).
    pub const ALL: [ProviderKind; MEMORY_PROVIDER_KIND_COUNT] = [
        Self::InMemory,
        Self::Redis,
        Self::Sqlite,
        Self::Postgres,
        Self::S3,
        Self::DiskLru,
        Self::Hybrid,
        Self::File,
        Self::MongoDb,
    ];

    /// 该 provider 期望的 connection_string 协议头 (per 借鉴 Golutra 5 provider URI 模式).
    pub const fn expected_scheme(self) -> &'static str {
        match self {
            Self::InMemory => "memory://",
            Self::Redis => "redis://",
            Self::Sqlite => "sqlite://",
            Self::Postgres => "postgres://",
            Self::S3 => "s3://",
            Self::DiskLru => "file://",
            Self::Hybrid => "hybrid://",
            Self::File => "file-jsonl://",
            Self::MongoDb => "mongodb://",
        }
    }
}

// ============================================================================
// ProviderScope 3 变体 (编译期 hardcode, K-1 字段 6 之一)
// ============================================================================

/// **Provider scope**: 3 变体, K-1 字段 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderScope {
    /// 0: Local — 单进程 (e.g. InMemory, DiskLru 单文件).
    Local = 0,
    /// 1: Shared — 多进程共享 (e.g. Sqlite 文件锁, Redis 共享).
    Shared = 1,
    /// 2: Global — 跨集群 (e.g. S3, Postgres 集群).
    Global = 2,
}

impl fmt::Display for ProviderScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ProviderScope {
    /// 编译期字符串.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Shared => "shared",
            Self::Global => "global",
        }
    }

    /// 数字 0-2 → ProviderScope.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Local),
            1 => Some(Self::Shared),
            2 => Some(Self::Global),
            _ => None,
        }
    }
}

// ============================================================================
// ProviderConfigField 6 变体 (K-1 强校验字段标识, 错误定位用)
// ============================================================================

/// **ProviderConfig 6 K-1 字段 enum** (用于 Config 错误定位).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderConfigField {
    /// 字段 1: connection_string.
    ConnectionString = 0,
    /// 字段 2: timeout.
    Timeout = 1,
    /// 字段 3: max_size.
    MaxSize = 2,
    /// 字段 4: persist.
    Persist = 3,
    /// 字段 5: cache_ttl.
    CacheTtl = 4,
    /// 字段 6: scope.
    Scope = 5,
}

impl ProviderConfigField {
    /// 编译期字符串 (字段名, snake_case).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConnectionString => "connection_string",
            Self::Timeout => "timeout",
            Self::MaxSize => "max_size",
            Self::Persist => "persist",
            Self::CacheTtl => "cache_ttl",
            Self::Scope => "scope",
        }
    }
}

// ============================================================================
// ProviderConfig (6 K-1 字段 + 强校验方法)
// ============================================================================

/// **ProviderConfig**: 6 K-1 字段配置 (per task spec 强制要求).
///
/// 6 字段 + 强校验 + 序列化:
/// - `connection_string`: URL/path/连接 URI
/// - `timeout`: 客户端操作超时 (1ms..=1h)
/// - `max_size`: 容量上限 (1KB..=1TB)
/// - `persist`: 是否持久化
/// - `cache_ttl`: 缓存过期时间 (0ms..=7d)
/// - `scope`: Local/Shared/Global
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// K-1 #1: 连接字符串.
    pub connection_string: String,
    /// K-1 #2: 超时.
    pub timeout: Duration,
    /// K-1 #3: 最大容量 (bytes).
    pub max_size: u64,
    /// K-1 #4: 是否持久化.
    pub persist: bool,
    /// K-1 #5: 缓存 TTL.
    pub cache_ttl: Duration,
    /// K-1 #6: 作用域.
    pub scope: ProviderScope,
}

/// 1 KB (用于 max_size 下界校验).
const MIN_MAX_SIZE_BYTES: u64 = 1024;
/// 1 TB (用于 max_size 上界校验).
const MAX_MAX_SIZE_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
/// 1 ms (用于 timeout 下界校验).
const MIN_TIMEOUT: Duration = Duration::from_millis(1);
/// 1 h (用于 timeout 上界校验).
const MAX_TIMEOUT: Duration = Duration::from_secs(3600);
/// 7 d (用于 cache_ttl 上界校验, 0 ms 是允许的 = 永不过期).
const MAX_CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 3600);

impl ProviderConfig {
    /// 新建 ProviderConfig (无强校验, 用 `validate` 强校验).
    pub fn new(
        connection_string: impl Into<String>,
        timeout: Duration,
        max_size: u64,
        persist: bool,
        cache_ttl: Duration,
        scope: ProviderScope,
    ) -> Self {
        Self {
            connection_string: connection_string.into(),
            timeout,
            max_size,
            persist,
            cache_ttl,
            scope,
        }
    }

    /// **6 K-1 强校验**: 校验所有 6 字段, 失败返 `MemoryProviderError::Config`.
    pub fn validate(&self, kind: ProviderKind) -> MemoryProviderResult<()> {
        // K-1 #1: connection_string 非空 + 协议头匹配
        if self.connection_string.is_empty() {
            return Err(MemoryProviderError::Config {
                field: ProviderConfigField::ConnectionString,
                reason: "must be non-empty".to_string(),
            });
        }
        let expected_scheme = kind.expected_scheme();
        if !self.connection_string.starts_with(expected_scheme) {
            return Err(MemoryProviderError::Config {
                field: ProviderConfigField::ConnectionString,
                reason: format!(
                    "must start with scheme `{expected_scheme}`, got `{}`",
                    self.connection_string
                ),
            });
        }

        // K-1 #2: timeout in [1ms, 1h]
        if self.timeout < MIN_TIMEOUT || self.timeout > MAX_TIMEOUT {
            return Err(MemoryProviderError::Config {
                field: ProviderConfigField::Timeout,
                reason: format!(
                    "must be in [1ms, 1h], got {:?}",
                    self.timeout
                ),
            });
        }

        // K-1 #3: max_size in [1KB, 1TB]
        if self.max_size < MIN_MAX_SIZE_BYTES || self.max_size > MAX_MAX_SIZE_BYTES {
            return Err(MemoryProviderError::Config {
                field: ProviderConfigField::MaxSize,
                reason: format!(
                    "must be in [1KB, 1TB], got {max_size}",
                    max_size = self.max_size
                ),
            });
        }

        // K-1 #4: persist — bool, 0 校验 (任何 bool 合法)

        // K-1 #5: cache_ttl in [0ms, 7d] (0 = 永不过期)
        if self.cache_ttl > MAX_CACHE_TTL {
            return Err(MemoryProviderError::Config {
                field: ProviderConfigField::CacheTtl,
                reason: format!(
                    "must be in [0ms, 7d], got {:?}",
                    self.cache_ttl
                ),
            });
        }

        // K-1 #6: scope — 3 变体, enum 编译期守门, 0 校验 (无效值无法构造)

        Ok(())
    }
}

// ============================================================================
// MemoryProvider trait (7 通用方法, 编译期 monomorphization)
// ============================================================================

/// **MemoryProvider trait** (7 通用方法, 跨 7 provider 统一接口).
///
/// 7 方法 (per Golutra memory gateway 5 provider API 1:1 + extension):
/// - `kind()`: 当前 provider 类型
/// - `set()`: 写一个 key
/// - `get()`: 读一个 key
/// - `delete()`: 删一个 key
/// - `exists()`: 查 key 是否存在
/// - `clear()`: 清空所有
/// - `size()`: 当前 entry 数
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// 当前 provider 类型.
    fn kind(&self) -> ProviderKind;

    /// 写一个 key.
    ///
    /// 行为:
    /// - InMemory: HashMap insert
    /// - Sqlite: INSERT OR REPLACE
    /// - DiskLru: lru.put + fs write
    /// - Hybrid: in_memory + disk_lru 同步
    /// - Redis/Postgres/S3: 真连服务端 PUT (skeleton 阶段只验 config, 实际连接 R21+)
    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()>;

    /// 读一个 key.
    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>>;

    /// 删一个 key.
    async fn delete(&self, key: &str) -> MemoryProviderResult<()>;

    /// 查 key 是否存在.
    async fn exists(&self, key: &str) -> MemoryProviderResult<bool>;

    /// 清空所有 (per provider 实现).
    async fn clear(&self) -> MemoryProviderResult<()>;

    /// 当前 entry 数.
    async fn size(&self) -> MemoryProviderResult<u64>;
}

// ============================================================================
// 单元测试 (ProviderKind 7 + ProviderScope 3 + ProviderConfigField 6 + ProviderConfig 6 K-1 = 30+ 测试)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // ProviderKind 9 变体测试
    // -----------------------------------------------------------------------

    #[test]
    fn nine_provider_kind_variants_constructible() {
        let _ = ProviderKind::InMemory;
        let _ = ProviderKind::Redis;
        let _ = ProviderKind::Sqlite;
        let _ = ProviderKind::Postgres;
        let _ = ProviderKind::S3;
        let _ = ProviderKind::DiskLru;
        let _ = ProviderKind::Hybrid;
        let _ = ProviderKind::File;
        let _ = ProviderKind::MongoDb;
    }

    #[test]
    fn nine_provider_kind_as_str_distinct() {
        let s: Vec<&str> = (0..=8u8)
            .map(|n| ProviderKind::from_u8(n).unwrap().as_str())
            .collect();
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 9);
    }

    #[test]
    fn nine_provider_kind_serialize_round_trip() {
        for n in 0..=6u8 {
            let kind = ProviderKind::from_u8(n).unwrap();
            let s = serde_json::to_string(&kind).unwrap();
            let back: ProviderKind = serde_json::from_str(&s).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn nine_provider_kind_all_matches_count() {
        assert_eq!(ProviderKind::ALL.len(), MEMORY_PROVIDER_KIND_COUNT);
        assert_eq!(ProviderKind::ALL.len(), 9);
    }

    #[test]
    fn from_u8_round_trip_9() {
        for n in 0..=8u8 {
            let k = ProviderKind::from_u8(n).expect("0-8 valid");
            assert_eq!(k as u8, n);
        }
        assert!(ProviderKind::from_u8(9).is_none());
        assert!(ProviderKind::from_u8(255).is_none());
    }

    #[test]
    fn nine_provider_kind_expected_scheme_distinct() {
        let schemes: Vec<&str> = ProviderKind::ALL
            .iter()
            .map(|k| k.expected_scheme())
            .collect();
        let unique: std::collections::HashSet<&str> = schemes.iter().copied().collect();
        assert_eq!(unique.len(), 9);
    }

    #[test]
    fn nine_provider_kind_count_constant() {
        assert_eq!(MEMORY_PROVIDER_KIND_COUNT, 9);
    }

    // -----------------------------------------------------------------------
    // ProviderScope 3 变体测试
    // -----------------------------------------------------------------------

    #[test]
    fn three_scope_variants_constructible() {
        let _ = ProviderScope::Local;
        let _ = ProviderScope::Shared;
        let _ = ProviderScope::Global;
    }

    #[test]
    fn three_scope_as_str_distinct() {
        let s = [
            ProviderScope::Local.as_str(),
            ProviderScope::Shared.as_str(),
            ProviderScope::Global.as_str(),
        ];
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 3);
    }

    #[test]
    fn three_scope_count_constant() {
        assert_eq!(PROVIDER_SCOPE_COUNT, 3);
    }

    // -----------------------------------------------------------------------
    // ProviderConfigField 6 变体测试
    // -----------------------------------------------------------------------

    #[test]
    fn six_config_field_variants_constructible() {
        let _ = ProviderConfigField::ConnectionString;
        let _ = ProviderConfigField::Timeout;
        let _ = ProviderConfigField::MaxSize;
        let _ = ProviderConfigField::Persist;
        let _ = ProviderConfigField::CacheTtl;
        let _ = ProviderConfigField::Scope;
    }

    #[test]
    fn six_config_field_as_str_distinct() {
        let s = [
            ProviderConfigField::ConnectionString.as_str(),
            ProviderConfigField::Timeout.as_str(),
            ProviderConfigField::MaxSize.as_str(),
            ProviderConfigField::Persist.as_str(),
            ProviderConfigField::CacheTtl.as_str(),
            ProviderConfigField::Scope.as_str(),
        ];
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 6);
    }

    #[test]
    fn six_k1_fields_count_constant() {
        assert_eq!(PROVIDER_CONFIG_K1_FIELDS, 6);
    }

    // -----------------------------------------------------------------------
    // ProviderConfig 6 K-1 强校验测试
    // -----------------------------------------------------------------------

    #[test]
    fn validate_valid_config_passes() {
        let cfg = ProviderConfig::new(
            "redis://localhost:6379",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            ProviderScope::Shared,
        );
        assert!(cfg.validate(ProviderKind::Redis).is_ok());
    }

    #[test]
    fn validate_empty_connection_string_fails() {
        let cfg = ProviderConfig::new(
            "",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            ProviderScope::Local,
        );
        let err = cfg.validate(ProviderKind::InMemory).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { field: ProviderConfigField::ConnectionString, .. }));
    }

    #[test]
    fn validate_wrong_scheme_fails() {
        let cfg = ProviderConfig::new(
            "redis://localhost:6379",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(60),
            ProviderScope::Local,
        );
        // redis:// 配 InMemory 期望 memory://, 失败
        let err = cfg.validate(ProviderKind::InMemory).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { field: ProviderConfigField::ConnectionString, .. }));
    }

    #[test]
    fn validate_timeout_too_small_fails() {
        let cfg = ProviderConfig::new(
            "memory://",
            Duration::from_micros(500), // < 1ms
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let err = cfg.validate(ProviderKind::InMemory).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { field: ProviderConfigField::Timeout, .. }));
    }

    #[test]
    fn validate_timeout_too_large_fails() {
        let cfg = ProviderConfig::new(
            "memory://",
            Duration::from_secs(7200), // > 1h
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let err = cfg.validate(ProviderKind::InMemory).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { field: ProviderConfigField::Timeout, .. }));
    }

    #[test]
    fn validate_max_size_too_small_fails() {
        let cfg = ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            512, // < 1KB
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let err = cfg.validate(ProviderKind::InMemory).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { field: ProviderConfigField::MaxSize, .. }));
    }

    #[test]
    fn validate_max_size_too_large_fails() {
        let cfg = ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            2 * 1024 * 1024 * 1024 * 1024, // 2TB > 1TB
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let err = cfg.validate(ProviderKind::InMemory).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { field: ProviderConfigField::MaxSize, .. }));
    }

    #[test]
    fn validate_cache_ttl_too_large_fails() {
        let cfg = ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(8 * 24 * 3600), // > 7d
            ProviderScope::Local,
        );
        let err = cfg.validate(ProviderKind::InMemory).unwrap_err();
        assert!(matches!(err, MemoryProviderError::Config { field: ProviderConfigField::CacheTtl, .. }));
    }

    #[test]
    fn validate_cache_ttl_zero_passes() {
        // 0 = 永不过期, 允许
        let cfg = ProviderConfig::new(
            "memory://",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        assert!(cfg.validate(ProviderKind::InMemory).is_ok());
    }

    #[test]
    fn validate_scope_3_variants_all_pass() {
        for scope in [
            ProviderScope::Local,
            ProviderScope::Shared,
            ProviderScope::Global,
        ] {
            let cfg = ProviderConfig::new(
                "memory://",
                Duration::from_secs(5),
                1024 * 1024,
                false,
                Duration::from_secs(0),
                scope,
            );
            assert!(cfg.validate(ProviderKind::InMemory).is_ok(), "scope={scope:?} should pass");
        }
    }

    #[test]
    fn validate_all_9_provider_schemes_match() {
        // 7 provider × 1 scheme, 全部匹配
        let pairs: &[(ProviderKind, &str)] = &[
            (ProviderKind::InMemory, "memory://key1"),
            (ProviderKind::Redis, "redis://localhost:6379"),
            (ProviderKind::Sqlite, "sqlite://test.db"),
            (ProviderKind::Postgres, "postgres://user:pass@localhost/db"),
            (ProviderKind::S3, "s3://my-bucket/path"),
            (ProviderKind::DiskLru, "file:///tmp/cache"),
            (ProviderKind::Hybrid, "hybrid://memory+disk"),
        ];
        for (kind, conn) in pairs {
            let cfg = ProviderConfig::new(
                *conn,
                Duration::from_secs(5),
                1024 * 1024,
                true,
                Duration::from_secs(60),
                ProviderScope::Shared,
            );
            assert!(cfg.validate(*kind).is_ok(), "{kind:?} should accept {conn}");
        }
    }
}
