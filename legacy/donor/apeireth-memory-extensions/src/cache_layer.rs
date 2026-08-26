//! # cache_layer — "cache 语义" 接线层 (ProviderRole + CachedMemoryProvider)
//!
//! "telemetry cache 接线"派工的 cache 语义落地:
//!
//! > **若 provider 支持持久化则直接读写, 否则经主 store 中转缓存 (内存 provider 视为 cache 层).**
//!
//! ## 两个部件
//!
//! 1. **`ProviderRole` / `provider_role(kind)` / `is_persistent(kind)`** — provider 能力分类表:
//!    - `Persistent` — Sqlite / Postgres / S3 / DiskLru / Hybrid / File / Redis / MongoDb
//!      (配置语义上可持久化; 选它们时**直接读写**后端)
//!    - `CacheLayer` — InMemory (进程内, 死亡即失; **视为 cache 层**, 不应作为唯一真相源)
//! 2. **`CachedMemoryProvider`** — 装饰器 (decorator): 在**持久化后端**前挂一个 **L1 cache**
//!    (内存 provider), 实现 write-through + read-fill + invalidate:
//!    - `set`  → 写穿: cache 尽力写 (失败忽略) + 后端权威写 (失败则回滚 cache 并报错)
//!    - `get`  → 读填充: cache 命中直接返; miss 读后端, 命中则回填 cache
//!    - `delete` / `clear` → cache 失效 + 后端删除
//!    - `exists` → cache 快路径, miss 再问后端
//!    - `size`  → 后端权威
//!
//! **不假装**:
//! - 本层是**语义接线**, 0 假装"所有 provider 都有服务端": Redis/Postgres/S3/MongoDb 的
//!   `set/get` 在无服务端时仍返 `Connection` 错 (per provider_*.rs 现状), 分类表只说明
//!   配置语义上"可持久化", 不担保连通.
//! - 0 TTL 实现 (cache_ttl 是 config 旋钮, 本层不 pretend 过期语义, R+ 续做).

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::MemoryProviderResult;
use crate::memory_provider::{MemoryProvider, ProviderKind};

/// **Provider 能力分类**: 持久化后端 vs cache 层.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderRole {
    /// 持久化后端 (可作权威真相源; 接线时直接读写).
    Persistent,
    /// 进程内 cache 层 (死亡即失; 只能作 L1, 0 作唯一真相源).
    CacheLayer,
}

impl ProviderRole {
    /// 编译期字符串.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Persistent => "persistent",
            Self::CacheLayer => "cache_layer",
        }
    }
}

/// **能力分类表**: `InMemory` = cache 层; 其余 8 个 = 持久化后端.
pub const fn provider_role(kind: ProviderKind) -> ProviderRole {
    match kind {
        ProviderKind::InMemory => ProviderRole::CacheLayer,
        ProviderKind::Redis
        | ProviderKind::Sqlite
        | ProviderKind::Postgres
        | ProviderKind::S3
        | ProviderKind::DiskLru
        | ProviderKind::Hybrid
        | ProviderKind::File
        | ProviderKind::MongoDb => ProviderRole::Persistent,
    }
}

/// 该 provider 是否可持久化 (per [`provider_role`]).
pub const fn is_persistent(kind: ProviderKind) -> bool {
    matches!(provider_role(kind), ProviderRole::Persistent)
}

/// **CachedMemoryProvider**: 持久化后端 + L1 内存 cache 的装饰器.
///
/// 实现 "cache 语义":
/// - 后端 (`backend`) 必须是持久化 provider (构造时 debug_assert, 0 强制运行时校验 =
///   调用方负责, 文档明示).
/// - cache (`cache`) 通常是 `InMemoryProvider` (cache 层); 任何实现 `MemoryProvider` 的
///   provider 都可作 cache, 但只有内存级才有意义 (0 假装 disk 作 cache 更快).
///
/// 语义 (per "若 provider 支持持久化则直接读写, 否则经主 store 中转缓存"):
/// - 持久化后端 → 读写最终落后端 (write-through / read-fill).
/// - 内存 provider → 作为 L1 cache 层 (0 假装它能替代后端持久化).
#[derive(Clone)]
pub struct CachedMemoryProvider {
    /// L1 cache (内存 provider, cache 层).
    cache: Arc<dyn MemoryProvider>,
    /// L2 持久化后端 (权威真相源).
    backend: Arc<dyn MemoryProvider>,
}

impl std::fmt::Debug for CachedMemoryProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // dyn MemoryProvider 0 Debug → 只打两层 kind.
        f.debug_struct("CachedMemoryProvider")
            .field("cache_kind", &self.cache.kind())
            .field("backend_kind", &self.backend.kind())
            .finish()
    }
}

impl CachedMemoryProvider {
    /// 新建装饰器: cache 在前, backend 在后.
    ///
    /// `backend` 应为持久化 provider (per [`is_persistent`]); 违反时 debug 构建 panic
    /// (release 0 校验 — 调用方自担, 0 假装).
    pub fn new(cache: Arc<dyn MemoryProvider>, backend: Arc<dyn MemoryProvider>) -> Self {
        debug_assert!(
            is_persistent(backend.kind()),
            "CachedMemoryProvider backend must be persistent, got {:?}",
            backend.kind()
        );
        Self { cache, backend }
    }

    /// L1 cache 引用.
    pub fn cache(&self) -> &Arc<dyn MemoryProvider> {
        &self.cache
    }

    /// L2 持久化后端引用.
    pub fn backend(&self) -> &Arc<dyn MemoryProvider> {
        &self.backend
    }
}

#[async_trait]
impl MemoryProvider for CachedMemoryProvider {
    fn kind(&self) -> ProviderKind {
        // 对外呈现后端类型 (cache 层 0 假装自己是独立 provider).
        self.backend.kind()
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        // cache 尽力写 (capacity 满等失败 0 阻断 — cache 非权威)
        let _ = self.cache.set(key, value).await;
        // 后端权威写
        if let Err(e) = self.backend.set(key, value).await {
            // 后端失败 → 回滚 cache (防 cache 与后端不一致)
            let _ = self.cache.delete(key).await;
            return Err(e);
        }
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        // L1 命中直接返 (快路径)
        if let Some(v) = self.cache.get(key).await? {
            return Ok(Some(v));
        }
        // L1 miss → 读后端 (read-fill)
        let v = self.backend.get(key).await?;
        if let Some(value) = &v {
            let _ = self.cache.set(key, value).await; // 回填尽力而为
        }
        Ok(v)
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        // cache 失效先行 (0 阻断 — 失效失败不该让删除失败)
        let _ = self.cache.delete(key).await;
        self.backend.delete(key).await
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        // cache 快路径
        if self.cache.exists(key).await? {
            return Ok(true);
        }
        self.backend.exists(key).await
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        let _ = self.cache.clear().await;
        self.backend.clear().await
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        // 权威 size = 后端 (cache 只是镜像)
        self.backend.size().await
    }
}

// =====================================================================
// 单元测试 (分类表 + 装饰器 4 语义 = 12+ 测试, 全用真 provider)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_provider::{ProviderConfig, ProviderScope};
    use crate::provider_in_memory::InMemoryProvider;
    use crate::provider_sqlite::SqliteProvider;
    use std::time::Duration;

    fn in_memory() -> Arc<dyn MemoryProvider> {
        Arc::new(
            InMemoryProvider::new(ProviderConfig::new(
                "memory://",
                Duration::from_secs(5),
                1024 * 1024,
                false,
                Duration::from_secs(0),
                ProviderScope::Local,
            ))
            .unwrap(),
        )
    }

    fn sqlite() -> Arc<dyn MemoryProvider> {
        Arc::new(
            SqliteProvider::new(ProviderConfig::new(
                "sqlite://:memory:",
                Duration::from_secs(5),
                1024 * 1024,
                false,
                Duration::from_secs(0),
                ProviderScope::Shared,
            ))
            .unwrap(),
        )
    }

    // ----- 分类表 -----

    #[test]
    fn in_memory_is_cache_layer() {
        assert_eq!(
            provider_role(ProviderKind::InMemory),
            ProviderRole::CacheLayer
        );
        assert!(!is_persistent(ProviderKind::InMemory));
    }

    #[test]
    fn eight_providers_are_persistent() {
        for kind in [
            ProviderKind::Redis,
            ProviderKind::Sqlite,
            ProviderKind::Postgres,
            ProviderKind::S3,
            ProviderKind::DiskLru,
            ProviderKind::Hybrid,
            ProviderKind::File,
            ProviderKind::MongoDb,
        ] {
            assert_eq!(provider_role(kind), ProviderRole::Persistent, "{kind:?}");
            assert!(is_persistent(kind), "{kind:?}");
        }
    }

    #[test]
    fn role_as_str_distinct() {
        assert_eq!(ProviderRole::Persistent.as_str(), "persistent");
        assert_eq!(ProviderRole::CacheLayer.as_str(), "cache_layer");
    }

    // ----- 装饰器语义 (真 InMemory cache + 真 Sqlite backend) -----

    #[tokio::test]
    async fn set_write_through_writes_both_layers() {
        let c = CachedMemoryProvider::new(in_memory(), sqlite());
        c.set("k1", b"v1").await.unwrap();
        assert!(c.cache().exists("k1").await.unwrap());
        assert!(c.backend().exists("k1").await.unwrap());
        assert_eq!(c.get("k1").await.unwrap(), Some(b"v1".to_vec()));
    }

    #[tokio::test]
    async fn get_miss_fills_cache() {
        let cache = in_memory();
        let backend = sqlite();
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        // 直接写后端 (绕过 cache, 模拟"别处写入")
        backend.set("cold", b"hot").await.unwrap();
        assert!(!cache.exists("cold").await.unwrap(), "cache 应初始 miss");
        // get → read-fill
        assert_eq!(c.get("cold").await.unwrap(), Some(b"hot".to_vec()));
        assert!(cache.exists("cold").await.unwrap(), "miss 后应回填 cache");
    }

    #[tokio::test]
    async fn get_cache_hit_skips_backend() {
        let cache = in_memory();
        let backend = sqlite();
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        c.set("k", b"v").await.unwrap();
        // 删后端数据 → cache 命中仍能读到 (证明快路径)
        backend.delete("k").await.unwrap();
        assert_eq!(c.get("k").await.unwrap(), Some(b"v".to_vec()));
        assert!(!backend.exists("k").await.unwrap());
    }

    #[tokio::test]
    async fn delete_invalidates_cache_and_backend() {
        let cache = in_memory();
        let backend = sqlite();
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        c.set("k1", b"v1").await.unwrap();
        c.delete("k1").await.unwrap();
        assert!(!cache.exists("k1").await.unwrap());
        assert!(!backend.exists("k1").await.unwrap());
        assert_eq!(c.get("k1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn exists_cache_fast_path() {
        let cache = in_memory();
        let backend = sqlite();
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        c.set("k", b"v").await.unwrap();
        backend.delete("k").await.unwrap();
        // cache 快路径 → 仍 exists (证明没问后端)
        assert!(c.exists("k").await.unwrap());
    }

    #[tokio::test]
    async fn clear_clears_both_layers() {
        let cache = in_memory();
        let backend = sqlite();
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        c.set("k1", b"v1").await.unwrap();
        c.set("k2", b"v2").await.unwrap();
        c.clear().await.unwrap();
        assert_eq!(c.size().await.unwrap(), 0);
        assert_eq!(cache.size().await.unwrap(), 0);
        assert_eq!(backend.size().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn size_is_backend_authoritative() {
        let cache = in_memory();
        let backend = sqlite();
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        // 后端直写 1 条 + 装饰器写 1 条 = 后端 2 条 (cache 1 条)
        backend.set("a", b"1").await.unwrap();
        c.set("b", b"2").await.unwrap();
        assert_eq!(backend.size().await.unwrap(), 2);
        assert_eq!(cache.size().await.unwrap(), 1);
        assert_eq!(c.size().await.unwrap(), 2, "size 应取后端权威值");
    }

    #[tokio::test]
    async fn cache_capacity_failure_does_not_fail_set() {
        // cache max_size = 1KB; 写 2KB value → cache Capacity 失败, 后端仍成功
        let cache: Arc<dyn MemoryProvider> = Arc::new(
            InMemoryProvider::new(ProviderConfig::new(
                "memory://",
                Duration::from_secs(5),
                1024,
                false,
                Duration::from_secs(0),
                ProviderScope::Local,
            ))
            .unwrap(),
        );
        let backend = sqlite();
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        let big = vec![b'x'; 2048];
        c.set("big", &big).await.unwrap();
        // 后端权威有数据; cache 尽力失败但整体不报错
        assert_eq!(backend.get("big").await.unwrap(), Some(big.clone()));
        assert_eq!(c.get("big").await.unwrap(), Some(big));
    }

    #[tokio::test]
    async fn backend_failure_rolls_back_cache() {
        let cache = in_memory();
        // 用无服务端 Redis 作 backend → set 必 Connection 错
        let backend: Arc<dyn MemoryProvider> = Arc::new(
            crate::provider_redis::RedisProvider::new(ProviderConfig::new(
                "redis://localhost:6399/0",
                Duration::from_secs(5),
                1024 * 1024,
                true,
                Duration::from_secs(0),
                ProviderScope::Global,
            ))
            .unwrap(),
        );
        let c = CachedMemoryProvider::new(cache.clone(), backend.clone());
        assert!(
            c.set("k", b"v").await.is_err(),
            "无 Redis server 后端 set 必须 err"
        );
        assert!(!cache.exists("k").await.unwrap(), "后端失败应回滚 cache");
        assert!(c.get("k").await.is_err());
    }

    #[test]
    fn kind_exposes_backend_kind() {
        let c = CachedMemoryProvider::new(in_memory(), sqlite());
        assert_eq!(c.kind(), ProviderKind::Sqlite);
    }
}
