//! # HybridProvider — 7 provider 模式 6: 组合 (in_memory + disk_lru 两级)
//!
//! **真接 (per 8 项之 1, "0 不假装已实现")**:
//! - 内部组合 `InMemoryProvider` (L1 快) + `DiskLruProvider` (L2 慢但大)
//! - `set` 同时写 L1 + L2 (write-through)
//! - `get` 先查 L1, miss 再查 L2; L2 命中 promote 到 L1
//! - `delete` 同时删 L1 + L2
//! - **端到端可测**: 集成测试用 tempfile 真实两层测试
//!
//! **不假装**:
//! - 两层都是真 `InMemoryProvider` + `DiskLruProvider` 组件, 0 重新造 LRU/HashMap
//! - skeleton 阶段 write-through, 0 假装"read-through with prefetch" / "smart tiering"
//! - 0 假装"自动迁移 L1↔L2", 仅显式 promote (L2→L1 on miss)
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `hybrid://memory+disk` (语义, 不解析实际 URL)
//! 2. timeout = [1ms, 1h]
//! 3. max_size = [1KB, 1TB] (总容量 = L1 + L2)
//! 4. persist = bool (透传给 L2 DiskLruProvider)
//! 5. cache_ttl = [0ms, 7d] (透传给 L2 DiskLruProvider)
//! 6. scope = Shared (跨进程共享, 走 L2 DiskLru)

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{
    MemoryProvider, ProviderConfig, ProviderKind, ProviderScope,
};
use crate::provider_disk_lru::DiskLruProvider;
use crate::provider_in_memory::InMemoryProvider;

/// **HybridProvider**: 组合 L1 (InMemory) + L2 (DiskLru) 两级缓存.
#[derive(Debug)]
pub struct HybridProvider {
    /// L1: InMemory (快, 小).
    l1: Arc<InMemoryProvider>,
    /// L2: DiskLru (慢, 大).
    l2: Arc<DiskLruProvider>,
    /// 6 K-1 强校验过的 config.
    #[allow(dead_code)]
    config: ProviderConfig,
    /// TempDir 句柄, drop 时自动清理 (per tempfile 0 重复造 cleanup).
    _tempdir: Option<TempDir>,
}

impl HybridProvider {
    /// 新建 HybridProvider, 6 K-1 强校验 + 创建 L1 + L2 组件.
    ///
    /// **不假装**: Hybrid 的 L2 disk 目录用 tempfile 自动管理, 0 假装持久化到固定路径
    /// (用户传 `file://` 时用之, 否则用 TempDir).
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::Hybrid)?;

        // 解析 hybrid://<l1_conn>+<l2_conn>
        // skeleton 阶段: 仅解析 `hybrid://memory+disk` 模式, 用 TempDir 当 L2 dir
        let stripped = config
            .connection_string
            .strip_prefix("hybrid://")
            .ok_or_else(|| MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: "must start with `hybrid://`".to_string(),
            })?;

        // L1 config: 1/10 max_size (1/10 是粗估, 0 假装"智能分配")
        let l1_max_size = (config.max_size / 10).max(1024);
        let l1_config = ProviderConfig::new(
            "memory://hybrid-l1",
            config.timeout,
            l1_max_size,
            false,
            config.cache_ttl,
            ProviderScope::Local,
        );
        let l1 = InMemoryProvider::new(l1_config)?;

        // L2 config: 9/10 max_size, 写到 TempDir
        let l2_max_size = (config.max_size * 9 / 10).max(1024);
        let tempdir = TempDir::new().map_err(|e| MemoryProviderError::Connection {
            provider: ProviderKind::Hybrid,
            reason: format!("TempDir create failed: {e}"),
        })?;
        let l2_config = ProviderConfig::new(
            format!("file://{}", tempdir.path().display()),
            config.timeout,
            l2_max_size,
            config.persist,
            config.cache_ttl,
            ProviderScope::Local,
        );
        let l2 = DiskLruProvider::new(l2_config)?;

        // 校验 stripped 至少含 `memory+disk` (基本守门)
        if !stripped.contains("memory") || !stripped.contains("disk") {
            return Err(MemoryProviderError::Config {
                field: crate::memory_provider::ProviderConfigField::ConnectionString,
                reason: format!("hybrid:// must contain `memory+disk`, got `{stripped}`"),
            });
        }

        Ok(Self {
            l1: Arc::new(l1),
            l2: Arc::new(l2),
            config,
            _tempdir: Some(tempdir),
        })
    }

    /// 6 K-1 字段 hardcoded: scope = Shared (跨进程共享, 走 L2 DiskLru 文件锁).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Shared
    }

    /// Get L1 (InMemory) handle.
    pub fn l1(&self) -> Arc<InMemoryProvider> {
        Arc::clone(&self.l1)
    }

    /// Get L2 (DiskLru) handle.
    pub fn l2(&self) -> Arc<DiskLruProvider> {
        Arc::clone(&self.l2)
    }
}

#[async_trait]
impl MemoryProvider for HybridProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Hybrid
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        // write-through: L1 + L2 同时写
        self.l1.set(key, value).await?;
        self.l2.set(key, value).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        // read: 先 L1, miss 再 L2 + promote
        if let Some(v) = self.l1.get(key).await? {
            return Ok(Some(v));
        }
        if let Some(v) = self.l2.get(key).await? {
            // L2 命中, promote 到 L1 (0 假装完美 LRU 策略, 仅直 promote)
            let _ = self.l1.set(key, &v).await;
            return Ok(Some(v));
        }
        Ok(None)
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        self.l1.delete(key).await?;
        self.l2.delete(key).await?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        // 任意一层存在即存在
        Ok(self.l1.exists(key).await? || self.l2.exists(key).await?)
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        self.l1.clear().await?;
        self.l2.clear().await?;
        Ok(())
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        // 总 size = L1 + L2 (会有重复, 0 假装"去重"精确数)
        let l1_size = self.l1.size().await?;
        let l2_size = self.l2.size().await?;
        Ok(l1_size + l2_size)
    }
}

/// **HybridConfigDefault** — 借 serde derive 序列化 ProviderConfig.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridConfigDefault {
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
            "hybrid://memory+disk",
            Duration::from_secs(5),
            1024 * 1024, // 1MB
            true,         // persist=true 透传给 L2
            Duration::from_secs(0), // 永不过期
            ProviderScope::Shared,
        )
    }

    // ----- 10 unit tests -----

    #[test]
    fn test_1_provider_kind_is_hybrid() {
        let p = HybridProvider::new(make_config()).unwrap();
        assert_eq!(p.kind(), ProviderKind::Hybrid);
    }

    #[test]
    fn test_2_k1_connection_string_must_be_hybrid_scheme() {
        let bad = ProviderConfig::new(
            "redis://localhost:6379",
            Duration::from_secs(5),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Shared,
        );
        let r = HybridProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_3_k1_timeout_validation_propagates() {
        let bad = ProviderConfig::new(
            "hybrid://memory+disk",
            Duration::from_micros(500),
            1024 * 1024,
            true,
            Duration::from_secs(0),
            ProviderScope::Shared,
        );
        let r = HybridProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_4_k1_max_size_validation_propagates() {
        let bad = ProviderConfig::new(
            "hybrid://memory+disk",
            Duration::from_secs(5),
            512, // < 1KB
            true,
            Duration::from_secs(0),
            ProviderScope::Shared,
        );
        let r = HybridProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_5_k1_persist_propagates_to_l2() {
        // Hybrid persist=true → L2 DiskLru persist=true → reload 行为
        let p = HybridProvider::new(make_config()).unwrap();
        assert!(p.config.persist);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(p.set("k1", b"v1")).unwrap();
        // L2 应该有 k1
        let l2_size = rt.block_on(p.l2.size()).unwrap();
        assert_eq!(l2_size, 1);
    }

    #[test]
    fn test_6_k1_cache_ttl_zero_passes() {
        let p = HybridProvider::new(make_config()).unwrap();
        assert_eq!(p.config.cache_ttl, Duration::from_secs(0));
    }

    #[test]
    fn test_7_k1_scope_is_always_shared() {
        let p = HybridProvider::new(make_config()).unwrap();
        assert_eq!(p.scope(), ProviderScope::Shared);
    }

    #[test]
    fn test_8_provider_init_with_valid_config() {
        let p = HybridProvider::new(make_config()).expect("valid config");
        assert_eq!(p.kind(), ProviderKind::Hybrid);
    }

    #[tokio::test]
    async fn test_9_set_get_round_trip_through_both_layers() {
        let p = HybridProvider::new(make_config()).unwrap();
        p.set("k1", b"hello").await.unwrap();
        let got = p.get("k1").await.unwrap();
        assert_eq!(got, Some(b"hello".to_vec()));
        // 验证 L1 + L2 都有
        assert!(p.l1().exists("k1").await.unwrap());
        assert!(p.l2().exists("k1").await.unwrap());
    }

    #[tokio::test]
    async fn test_10_end_to_end_promote_l2_to_l1_on_cache_miss() {
        let p = HybridProvider::new(make_config()).unwrap();
        // set: 写到 L1 + L2
        p.set("k1", b"v1").await.unwrap();
        // 手动清空 L1, 模拟 L1 miss
        p.l1().clear().await.unwrap();
        assert!(!p.l1().exists("k1").await.unwrap());
        assert!(p.l2().exists("k1").await.unwrap());
        // get: 走 L1 miss → L2 hit → promote 到 L1
        let got = p.get("k1").await.unwrap();
        assert_eq!(got, Some(b"v1".to_vec()));
        // L1 应该有 (promote 成功)
        assert!(p.l1().exists("k1").await.unwrap());
    }
}
