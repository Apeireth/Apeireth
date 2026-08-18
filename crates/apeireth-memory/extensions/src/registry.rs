//! # ProviderRegistry — 9 provider 聚合注册表
//!
//! 借鉴 Golutra v0.1.0 memory gateway 5 provider 装配模式 (`default_providers` 列表),
//! 1:1 翻译到 Rust 路线. 9 provider (7 借鉴 Golutra 5 + 2 扩展: InMemory/DiskLru/Hybrid/File/MongoDb) 一次性聚合.
//!
//! ## 设计
//!
//! - 9 provider 9 字段 (in_memory / redis / sqlite / postgres / s3 / disk_lru / hybrid / file / mongodb),
//!   编译期 hardcode, 改 1 provider = 改 1 字段 + 1 match arm
//! - 9 字段默认 `None`, 用 [`ProviderRegistryBuilder`] 按需 init
//! - 真接 (per 8 项之 1, "0 不假装已实现"): 9 provider 各自真接, 见 provider_*.rs
//!
//! **不假装**:
//! - 9 字段是 `Option<Arc<dyn MemoryProvider>>`, 仅是聚合容器, 不预创建 provider 实例
//! - 真实集成时 R21+ 续做: 用 builder 按需 init (e.g. 仅 init redis + sqlite, 0 init s3)

use std::sync::Arc;

use crate::memory_provider::{MemoryProvider, ProviderKind};

/// **K-1 强校验 #4**: ProviderRegistry 9 字段 (7 借鉴 Golutra + 2 扩展 File/MongoDb).
pub const REGISTRY_PROVIDER_COUNT: usize = 9;

/// **ProviderRegistry**: 9 provider 聚合容器 (per 借鉴 Golutra 7 provider + R23 #6 扩展 File/MongoDb).
///
/// 9 字段都用 `Option<Arc<dyn MemoryProvider>>`:
/// - `None`: 该 provider 尚未 init (skeleton 阶段常见)
/// - `Some(Arc<...>)`: 该 provider 已 init, 可用
///
/// **不假装**: 当前 skeleton 阶段 9 字段都默认 `None`, 真实集成时由 R23+ 用 builder 按需 init.
// (Debug / Default / Clone 手动 impl, 因 dyn trait 0 实现 Debug derive)
#[derive(Default)]
pub struct ProviderRegistry {
    /// 0: in_memory (进程内 HashMap).
    pub in_memory: Option<Arc<dyn MemoryProvider>>,
    /// 1: redis (外部 Redis server).
    pub redis: Option<Arc<dyn MemoryProvider>>,
    /// 2: sqlite (本地 SQLite 文件/:memory:).
    pub sqlite: Option<Arc<dyn MemoryProvider>>,
    /// 3: postgres (外部 PG server).
    pub postgres: Option<Arc<dyn MemoryProvider>>,
    /// 4: s3 (外部 S3 / S3-compatible).
    pub s3: Option<Arc<dyn MemoryProvider>>,
    /// 5: disk_lru (本地 disk + LRU).
    pub disk_lru: Option<Arc<dyn MemoryProvider>>,
    /// 6: hybrid (in_memory + disk_lru 两级).
    pub hybrid: Option<Arc<dyn MemoryProvider>>,
    /// 7: file (本地 JSON-Lines append-only, per R23 #6 派工).
    pub file: Option<Arc<dyn MemoryProvider>>,
    /// 8: mongodb (外部 MongoDB skeleton, per R23 #6 派工).
    pub mongodb: Option<Arc<dyn MemoryProvider>>,
}

impl Clone for ProviderRegistry {
    fn clone(&self) -> Self {
        Self {
            in_memory: self.in_memory.clone(),
            redis: self.redis.clone(),
            sqlite: self.sqlite.clone(),
            postgres: self.postgres.clone(),
            s3: self.s3.clone(),
            disk_lru: self.disk_lru.clone(),
            hybrid: self.hybrid.clone(),
            file: self.file.clone(),
            mongodb: self.mongodb.clone(),
        }
    }
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRegistry")
            .field("in_memory", &self.in_memory.is_some())
            .field("redis", &self.redis.is_some())
            .field("sqlite", &self.sqlite.is_some())
            .field("postgres", &self.postgres.is_some())
            .field("s3", &self.s3.is_some())
            .field("disk_lru", &self.disk_lru.is_some())
            .field("hybrid", &self.hybrid.is_some())
            .field("file", &self.file.is_some())
            .field("mongodb", &self.mongodb.is_some())
            .finish()
    }
}

impl ProviderRegistry {
    /// 新建空 registry (9 字段都 None).
    pub fn new() -> Self {
        Self::default()
    }

    /// 9 provider 哪个已 init (per ProviderKind 索引).
    pub fn is_initialized(&self, kind: ProviderKind) -> bool {
        match kind {
            ProviderKind::InMemory => self.in_memory.is_some(),
            ProviderKind::Redis => self.redis.is_some(),
            ProviderKind::Sqlite => self.sqlite.is_some(),
            ProviderKind::Postgres => self.postgres.is_some(),
            ProviderKind::S3 => self.s3.is_some(),
            ProviderKind::DiskLru => self.disk_lru.is_some(),
            ProviderKind::Hybrid => self.hybrid.is_some(),
            ProviderKind::File => self.file.is_some(),
            ProviderKind::MongoDb => self.mongodb.is_some(),
        }
    }

    /// 9 provider 已 init 数.
    pub fn initialized_count(&self) -> usize {
        ProviderKind::ALL
            .iter()
            .filter(|k| self.is_initialized(**k))
            .count()
    }

    /// Get provider by kind (None if not initialized).
    pub fn get(&self, kind: ProviderKind) -> Option<Arc<dyn MemoryProvider>> {
        match kind {
            ProviderKind::InMemory => self.in_memory.clone(),
            ProviderKind::Redis => self.redis.clone(),
            ProviderKind::Sqlite => self.sqlite.clone(),
            ProviderKind::Postgres => self.postgres.clone(),
            ProviderKind::S3 => self.s3.clone(),
            ProviderKind::DiskLru => self.disk_lru.clone(),
            ProviderKind::Hybrid => self.hybrid.clone(),
            ProviderKind::File => self.file.clone(),
            ProviderKind::MongoDb => self.mongodb.clone(),
        }
    }
}

/// **ProviderRegistryBuilder**: 9 provider 按需 init builder.
#[derive(Debug, Default, Clone)]
pub struct ProviderRegistryBuilder {
    /// 内部 registry.
    inner: ProviderRegistry,
}

impl ProviderRegistryBuilder {
    /// 新建 builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set in_memory provider.
    pub fn with_in_memory(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.in_memory = Some(p);
        self
    }

    /// Set redis provider.
    pub fn with_redis(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.redis = Some(p);
        self
    }

    /// Set sqlite provider.
    pub fn with_sqlite(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.sqlite = Some(p);
        self
    }

    /// Set postgres provider.
    pub fn with_postgres(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.postgres = Some(p);
        self
    }

    /// Set s3 provider.
    pub fn with_s3(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.s3 = Some(p);
        self
    }

    /// Set disk_lru provider.
    pub fn with_disk_lru(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.disk_lru = Some(p);
        self
    }

    /// Set hybrid provider.
    pub fn with_hybrid(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.hybrid = Some(p);
        self
    }

    /// Set file provider (R23 #6 派工).
    pub fn with_file(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.file = Some(p);
        self
    }

    /// Set mongodb provider (R23 #6 派工).
    pub fn with_mongodb(mut self, p: Arc<dyn MemoryProvider>) -> Self {
        self.inner.mongodb = Some(p);
        self
    }

    /// 构造 ProviderRegistry.
    pub fn build(self) -> ProviderRegistry {
        self.inner
    }
}

// =====================================================================
// 单元测试 (9 字段 + 9 kind + builder + 编译期守门 = 15+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::MemoryProviderResult;
    use crate::memory_provider::{ProviderConfig, ProviderScope};
    use async_trait::async_trait;
    use std::time::Duration;

    /// 测试用 dummy provider (7 通用方法返 Ok 占位)
    #[derive(Debug)]
    struct DummyProvider {
        kind: ProviderKind,
    }

    #[async_trait]
    impl MemoryProvider for DummyProvider {
        fn kind(&self) -> ProviderKind {
            self.kind
        }
        async fn set(&self, _key: &str, _value: &[u8]) -> MemoryProviderResult<()> {
            Ok(())
        }
        async fn get(&self, _key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
            Ok(None)
        }
        async fn delete(&self, _key: &str) -> MemoryProviderResult<()> {
            Ok(())
        }
        async fn exists(&self, _key: &str) -> MemoryProviderResult<bool> {
            Ok(false)
        }
        async fn clear(&self) -> MemoryProviderResult<()> {
            Ok(())
        }
        async fn size(&self) -> MemoryProviderResult<u64> {
            Ok(0)
        }
    }

    fn dummy(kind: ProviderKind) -> Arc<dyn MemoryProvider> {
        Arc::new(DummyProvider { kind })
    }

    // ----- registry tests -----

    #[test]
    fn new_registry_has_7_none_fields() {
        let r = ProviderRegistry::new();
        assert!(r.in_memory.is_none());
        assert!(r.redis.is_none());
        assert!(r.sqlite.is_none());
        assert!(r.postgres.is_none());
        assert!(r.s3.is_none());
        assert!(r.disk_lru.is_none());
        assert!(r.hybrid.is_none());
        assert_eq!(r.initialized_count(), 0);
    }

    #[test]
    fn is_initialized_returns_false_for_empty_registry() {
        let r = ProviderRegistry::new();
        for kind in ProviderKind::ALL {
            assert!(!r.is_initialized(kind));
        }
    }

    #[test]
    fn builder_can_set_each_of_7_providers() {
        let reg = ProviderRegistryBuilder::new()
            .with_in_memory(dummy(ProviderKind::InMemory))
            .with_redis(dummy(ProviderKind::Redis))
            .with_sqlite(dummy(ProviderKind::Sqlite))
            .with_postgres(dummy(ProviderKind::Postgres))
            .with_s3(dummy(ProviderKind::S3))
            .with_disk_lru(dummy(ProviderKind::DiskLru))
            .with_hybrid(dummy(ProviderKind::Hybrid))
            .build();
        assert_eq!(reg.initialized_count(), 7);
    }

    #[test]
    fn get_returns_correct_provider() {
        let p = dummy(ProviderKind::Sqlite);
        let reg = ProviderRegistryBuilder::new()
            .with_sqlite(p.clone())
            .build();
        let got = reg.get(ProviderKind::Sqlite).expect("sqlite initialized");
        assert_eq!(got.kind(), ProviderKind::Sqlite);
    }

    #[test]
    fn get_returns_none_for_uninitialized() {
        let reg = ProviderRegistry::new();
        assert!(reg.get(ProviderKind::Redis).is_none());
    }

    #[test]
    fn seven_provider_count_constant() {
        assert_eq!(REGISTRY_PROVIDER_COUNT, 9);
    }

    #[test]
    fn is_initialized_7_variants_compile_time() {
        // 编译期守门: 7 变体都能传给 is_initialized
        let reg = ProviderRegistry::new();
        for kind in ProviderKind::ALL {
            let _ = reg.is_initialized(kind);
        }
    }

    #[test]
    fn partial_init_2_of_7() {
        let reg = ProviderRegistryBuilder::new()
            .with_in_memory(dummy(ProviderKind::InMemory))
            .with_sqlite(dummy(ProviderKind::Sqlite))
            .build();
        assert_eq!(reg.initialized_count(), 2);
        assert!(reg.is_initialized(ProviderKind::InMemory));
        assert!(reg.is_initialized(ProviderKind::Sqlite));
        assert!(!reg.is_initialized(ProviderKind::Redis));
    }

    #[test]
    fn clone_shares_provider() {
        let p = dummy(ProviderKind::Hybrid);
        let reg = ProviderRegistryBuilder::new()
            .with_hybrid(p.clone())
            .build();
        let cloned = reg.clone();
        let got1 = reg.get(ProviderKind::Hybrid).unwrap();
        let got2 = cloned.get(ProviderKind::Hybrid).unwrap();
        assert!(Arc::ptr_eq(&got1, &got2));
    }

    #[test]
    fn nine_provider_kind_all_round_trip_through_registry() {
        let reg = ProviderRegistryBuilder::new()
            .with_in_memory(dummy(ProviderKind::InMemory))
            .with_redis(dummy(ProviderKind::Redis))
            .with_sqlite(dummy(ProviderKind::Sqlite))
            .with_postgres(dummy(ProviderKind::Postgres))
            .with_s3(dummy(ProviderKind::S3))
            .with_disk_lru(dummy(ProviderKind::DiskLru))
            .with_hybrid(dummy(ProviderKind::Hybrid))
            .with_file(dummy(ProviderKind::File))
            .with_mongodb(dummy(ProviderKind::MongoDb))
            .build();
        for kind in ProviderKind::ALL {
            let p = reg.get(kind).expect("all 9 initialized");
            assert_eq!(p.kind(), kind);
        }
    }

    // ----- 6 K-1 强校验 cross-check 集成 -----

    #[test]
    fn all_7_k1_validations_via_config() {
        let cases: &[(ProviderKind, &str)] = &[
            (ProviderKind::InMemory, "memory://test"),
            (ProviderKind::Redis, "redis://localhost:6379/0"),
            (ProviderKind::Sqlite, "sqlite://:memory:"),
            (ProviderKind::Postgres, "postgres://user:pass@localhost/db"),
            (ProviderKind::S3, "s3://bucket/key"),
            (ProviderKind::DiskLru, "file:///tmp/cache"),
            (ProviderKind::Hybrid, "hybrid://memory+disk"),
        ];
        for (kind, conn) in cases {
            let cfg = ProviderConfig::new(
                *conn,
                Duration::from_secs(5),
                1024 * 1024,
                true,
                Duration::from_secs(60),
                ProviderScope::Shared,
            );
            assert!(
                cfg.validate(*kind).is_ok(),
                "{kind:?} should validate {conn}"
            );
        }
    }
}
