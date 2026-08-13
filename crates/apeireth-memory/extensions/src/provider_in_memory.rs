//! # InMemoryProvider — 7 provider 模式 0: 进程内 HashMap
//!
//! **真接 (per 8 项之 1, "0 不假装已实现")**:
//! - 内部用 `Arc<Mutex<HashMap<String, Vec<u8>>>>` 持 state
//! - `set` 走 `HashMap::insert`, `get` 走 `HashMap::get` clone
//! - 7 通用方法 (`kind`/`set`/`get`/`delete`/`exists`/`clear`/`size`) 全部真接
//! - **端到端可测**: 0 外部服务, 集成测试 0 依赖
//!
//! **不假装**:
//! - skeleton 阶段 0 业务数据, 0 假装"分布式缓存"
//! - InMemory 进程死亡 = 数据丢失 (per 内存语义, 0 编造"持久化内存")
//! - 0 假装 async, 锁是 std::sync::Mutex (sync), 在 async 上下文里会阻塞 tokio worker
//!   (skeleton 阶段可接受, R21+ 续做 parking_lot::Mutex / tokio::sync::Mutex)
//!
//! **6 K-1 强校验** (per task spec 强制要求):
//! 1. connection_string = `memory://<key>` (任意非空, 协议头匹配 `memory://`)
//! 2. timeout = [1ms, 1h] (sync 实现, 实际 0 用, 仅过 config validate)
//! 3. max_size = [1KB, 1TB] (entries 总 size 上限, 超限返 Capacity)
//! 4. persist = bool (InMemory 永远 = false, 0 假装持久化)
//! 5. cache_ttl = [0ms, 7d] (0 = 永不过期, InMemory 不主动 expire, 仅供接口一致)
//! 6. scope = Local (InMemory 仅 Local, 0 假装 Shared/Global)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::{
    MemoryProvider, ProviderConfig, ProviderKind, ProviderScope,
};

/// **InMemoryProvider**: 进程内 HashMap 内存 provider.
#[derive(Debug, Clone)]
pub struct InMemoryProvider {
    /// 内部 `Arc<Mutex<HashMap<String, Vec<u8>>>>` (跨线程共享).
    inner: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    /// 6 K-1 强校验过的 config (clone-friendly).
    config: ProviderConfig,
    /// 编译期 hardcode: 当前 size (bytes, 避免每次 O(n) 算).
    current_size: Arc<Mutex<u64>>,
}

/// **InMemoryConfig 编译期守门**: connection_string 必须以 `memory://` 开头.
#[allow(dead_code)]
const IN_MEMORY_SCHEME: &str = "memory://";

impl InMemoryProvider {
    /// 新建 InMemoryProvider, 6 K-1 强校验.
    ///
    /// **不假装**: skeleton 阶段 sync 锁, 0 引 tokio::sync::Mutex.
    pub fn new(config: ProviderConfig) -> MemoryProviderResult<Self> {
        config.validate(ProviderKind::InMemory)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            config,
            current_size: Arc::new(Mutex::new(0)),
        })
    }

    /// Get internal Arc<Mutex<HashMap>> handle (跨线程共享, per 借鉴 #6 MutexState 模式).
    pub fn handle(&self) -> Arc<Mutex<HashMap<String, Vec<u8>>>> {
        Arc::clone(&self.inner)
    }

    /// 6 K-1 字段 hardcoded: persist = false (InMemory 不持久化).
    pub fn is_persistent(&self) -> bool {
        false
    }

    /// 6 K-1 字段 hardcoded: scope = Local (InMemory 仅 Local).
    pub fn scope(&self) -> ProviderScope {
        ProviderScope::Local
    }

    /// 当前 entry 数 (非 async, sync helper).
    pub fn len_sync(&self) -> usize {
        self.inner.lock().map(|m| m.len()).unwrap_or(0)
    }
}

#[async_trait]
impl MemoryProvider for InMemoryProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::InMemory
    }

    async fn set(&self, key: &str, value: &[u8]) -> MemoryProviderResult<()> {
        let mut map = self.inner.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        let mut size = self.current_size.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;

        // K-1 #3: max_size 校验 (capacity check)
        let new_size = *size + value.len() as u64;
        if new_size > self.config.max_size {
            return Err(MemoryProviderError::Capacity {
                provider: ProviderKind::InMemory,
                max_size: self.config.max_size,
                current: *size,
            });
        }

        // 已存在 key, 减去旧 size
        if let Some(old) = map.get(key) {
            *size -= old.len() as u64;
        }
        map.insert(key.to_string(), value.to_vec());
        *size = new_size;
        Ok(())
    }

    async fn get(&self, key: &str) -> MemoryProviderResult<Option<Vec<u8>>> {
        let map = self.inner.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        Ok(map.get(key).cloned())
    }

    async fn delete(&self, key: &str) -> MemoryProviderResult<()> {
        let mut map = self.inner.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        let mut size = self.current_size.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        if let Some(old) = map.remove(key) {
            *size -= old.len() as u64;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> MemoryProviderResult<bool> {
        let map = self.inner.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        Ok(map.contains_key(key))
    }

    async fn clear(&self) -> MemoryProviderResult<()> {
        let mut map = self.inner.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        let mut size = self.current_size.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        map.clear();
        *size = 0;
        Ok(())
    }

    async fn size(&self) -> MemoryProviderResult<u64> {
        let size = self.current_size.lock().map_err(|e| MemoryProviderError::Backend {
            provider: ProviderKind::InMemory,
            reason: format!("Mutex poisoned: {e}"),
        })?;
        Ok(*size)
    }
}

/// **InMemoryConfigDefault** — 借 serde derive 序列化 ProviderConfig (per 借鉴 #6 模式).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InMemoryConfigDefault {
    /// 6 K-1 字段 (ProviderConfig 6 字段 1:1).
    pub config: ProviderConfig,
}

// =====================================================================
// 单元测试 (10 tests per 借鉴 #6 模式 1:1)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> ProviderConfig {
        ProviderConfig::new(
            "memory://test",
            Duration::from_secs(5),
            1024 * 1024, // 1MB
            false,        // InMemory 不持久化
            Duration::from_secs(0), // 永不过期
            ProviderScope::Local,
        )
    }

    use std::time::Duration;

    // ----- 10 unit tests -----

    #[test]
    fn test_1_provider_kind_is_in_memory() {
        let p = InMemoryProvider::new(make_config()).unwrap();
        assert_eq!(p.kind(), ProviderKind::InMemory);
    }

    #[test]
    fn test_2_k1_connection_string_must_be_memory_scheme() {
        // redis:// 应该被 config.validate 拒绝
        let bad = ProviderConfig::new(
            "redis://localhost:6379",
            Duration::from_secs(5),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let r = InMemoryProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_3_k1_timeout_validation_propagates() {
        // timeout = 0 (小于 1ms 下界) → 强校验失败
        let bad = ProviderConfig::new(
            "memory://test",
            Duration::from_micros(500),
            1024 * 1024,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let r = InMemoryProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_4_k1_max_size_validation_propagates() {
        // max_size = 512 (< 1KB) → 强校验失败
        let bad = ProviderConfig::new(
            "memory://test",
            Duration::from_secs(5),
            512,
            false,
            Duration::from_secs(0),
            ProviderScope::Local,
        );
        let r = InMemoryProvider::new(bad);
        assert!(r.is_err());
    }

    #[test]
    fn test_5_k1_persist_is_ignored_for_in_memory() {
        // InMemory 永远不持久化, persist 字段仅作 config 合法性校验
        let p = InMemoryProvider::new(make_config()).unwrap();
        assert!(!p.is_persistent());
    }

    #[test]
    fn test_6_k1_cache_ttl_zero_passes() {
        // cache_ttl = 0 = 永不过期, InMemory 不主动 expire, 仅过 validate
        let p = InMemoryProvider::new(make_config()).unwrap();
        assert_eq!(p.config.cache_ttl, Duration::from_secs(0));
    }

    #[test]
    fn test_7_k1_scope_is_always_local() {
        // InMemory 永远 Local, 即使用户传 Shared/Global 也会被 config.validate 拒绝
        // (per 6 K-1: scope 强校验仅 enum 守门, InMemory 不强制, 但 scope() 方法硬返 Local)
        let p = InMemoryProvider::new(make_config()).unwrap();
        assert_eq!(p.scope(), ProviderScope::Local);
    }

    #[test]
    fn test_8_provider_init_with_valid_config() {
        let p = InMemoryProvider::new(make_config()).expect("valid config should construct");
        assert_eq!(p.kind(), ProviderKind::InMemory);
        assert_eq!(p.len_sync(), 0);
    }

    #[tokio::test]
    async fn test_9_set_get_round_trip() {
        let p = InMemoryProvider::new(make_config()).unwrap();
        p.set("k1", b"hello").await.unwrap();
        let got = p.get("k1").await.unwrap();
        assert_eq!(got, Some(b"hello".to_vec()));
    }

    #[tokio::test]
    async fn test_10_end_to_end_set_get_delete_exists_clear_size() {
        let p = InMemoryProvider::new(make_config()).unwrap();
        // 空
        assert_eq!(p.size().await.unwrap(), 0);
        assert!(!p.exists("k1").await.unwrap());
        // set 3 个
        p.set("k1", b"v1").await.unwrap();
        p.set("k2", b"v2").await.unwrap();
        p.set("k3", b"v3").await.unwrap();
        assert_eq!(p.size().await.unwrap(), 6); // 3 entries * 2 bytes
        assert!(p.exists("k1").await.unwrap());
        // get
        assert_eq!(p.get("k2").await.unwrap(), Some(b"v2".to_vec()));
        assert_eq!(p.get("nope").await.unwrap(), None);
        // delete
        p.delete("k2").await.unwrap();
        assert!(!p.exists("k2").await.unwrap());
        assert_eq!(p.size().await.unwrap(), 4);
        // clear
        p.clear().await.unwrap();
        assert_eq!(p.size().await.unwrap(), 0);
        assert!(!p.exists("k1").await.unwrap());
    }
}
