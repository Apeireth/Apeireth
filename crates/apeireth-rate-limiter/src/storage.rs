//! `apeireth-rate-limiter` 存储后端。
//!
//! 5 种 storage: `InMemoryStorage`（完整实现, parking_lot + HashMap）+ 4 个 stub。
//!
//! **设计原则**:
//! - 抽象 trait `Storage` — 4 个方法（get / set / delete / exists）
//! - `async_trait` — 未来 Redis stub 是真异步网络, 接口先异步化
//! - 4 个 stub 全部返回 `RateLimiterError::NotImplemented` — 不假装已实现
//!
//! **8 项不修改承诺**:
//! - 0 真接 Redis SDK / Memcached SDK — 留 R21+ 续真接
//! - 0 真接 Raft / Paxos consensus — 留 R22+ 续真接
//! - 0 持久化 file backend — 留 R21+ 续真接

use crate::error::{RateLimiterError, Result};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub use crate::config::StorageKind;

// =====================================================================
// Trait
// =====================================================================

/// 通用存储后端接口。
///
/// 4 个方法: get / set / delete / exists。所有方法都接受 `&str` key,
/// value 是 `Vec<u8>`（上层负责 serde 序列化）。
#[async_trait]
pub trait Storage: Send + Sync {
    /// 读 value + TTL。返回 `None` 表示 key 不存在或已过期。
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// 写 value + 可选 TTL。覆盖已有 key。
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()>;

    /// 删除 key。key 不存在不报错。
    async fn delete(&self, key: &str) -> Result<()>;

    /// 检查 key 是否存在（未过期）。
    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    /// 存储后端类型（用于 stats 报告 / 调试）。
    fn kind(&self) -> StorageKind;
}

// =====================================================================
// InMemoryStorage — 完整实现
// =====================================================================

/// 内存存储 — parking_lot::Mutex + HashMap, 完整实现。
///
/// **特点**:
/// - 内部 `Arc<Mutex<HashMap<...>>>` — 单锁, 简单
/// - TTL 用 `Instant` 单调时钟, 避免系统时间回拨
/// - `lazy TTL` — `get` 时检查过期, `set` 不立即清理（清理留给上层）
/// - 0 异步 — 直接阻塞, 但通过 `async_trait` 适配
pub struct InMemoryStorage {
    inner: Arc<Mutex<HashMap<String, Entry>>>,
}

/// 内部条目: value + 过期时间。
struct Entry {
    value: Vec<u8>,
    /// `None` = 不过期; `Some(instant)` = 在 instant 之前有效。
    expires_at: Option<Instant>,
}

impl InMemoryStorage {
    /// 新建空内存存储。
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 当前条目数（含已过期但未清理的）。
    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }

    /// 主动清理所有已过期条目。
    pub fn purge_expired(&self) -> usize {
        let now = Instant::now();
        let mut map = self.inner.lock();
        let before = map.len();
        map.retain(|_, entry| match entry.expires_at {
            None => true,
            Some(deadline) => deadline > now,
        });
        before - map.len()
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let map = self.inner.lock();
        match map.get(key) {
            None => Ok(None),
            Some(entry) => match entry.expires_at {
                Some(deadline) if deadline <= Instant::now() => Ok(None),
                _ => Ok(Some(entry.value.clone())),
            },
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl: Option<Duration>) -> Result<()> {
        let expires_at = ttl.map(|d| Instant::now() + d);
        let mut map = self.inner.lock();
        map.insert(
            key.to_string(),
            Entry {
                value,
                expires_at,
            },
        );
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut map = self.inner.lock();
        map.remove(key);
        Ok(())
    }

    fn kind(&self) -> StorageKind {
        StorageKind::InMemory
    }
}

// =====================================================================
// RedisStorage — stub
// =====================================================================

/// Redis 存储 — stub, 0 真接 redis-rs。
pub struct RedisStorage {
    _connection_string: Option<String>,
}

impl RedisStorage {
    /// 新建 Redis stub。
    pub fn new(connection_string: Option<String>) -> Self {
        Self {
            _connection_string: connection_string,
        }
    }
}

#[async_trait]
impl Storage for RedisStorage {
    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Err(RateLimiterError::NotImplemented(
            "RedisStorage::get — 留 R21+ 续真接 redis-rs".to_string(),
        ))
    }
    async fn set(&self, _key: &str, _value: Vec<u8>, _ttl: Option<Duration>) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "RedisStorage::set — 留 R21+ 续真接 redis-rs".to_string(),
        ))
    }
    async fn delete(&self, _key: &str) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "RedisStorage::delete — 留 R21+ 续真接 redis-rs".to_string(),
        ))
    }
    fn kind(&self) -> StorageKind {
        StorageKind::Redis
    }
}

// =====================================================================
// MemcachedStorage — stub
// =====================================================================

/// Memcached 存储 — stub, 0 真接 memcache-rs。
pub struct MemcachedStorage {
    _connection_string: Option<String>,
}

impl MemcachedStorage {
    /// 新建 Memcached stub。
    pub fn new(connection_string: Option<String>) -> Self {
        Self {
            _connection_string: connection_string,
        }
    }
}

#[async_trait]
impl Storage for MemcachedStorage {
    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Err(RateLimiterError::NotImplemented(
            "MemcachedStorage::get — 留 R21+ 续真接 memcache-rs".to_string(),
        ))
    }
    async fn set(&self, _key: &str, _value: Vec<u8>, _ttl: Option<Duration>) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "MemcachedStorage::set — 留 R21+ 续真接 memcache-rs".to_string(),
        ))
    }
    async fn delete(&self, _key: &str) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "MemcachedStorage::delete — 留 R21+ 续真接 memcache-rs".to_string(),
        ))
    }
    fn kind(&self) -> StorageKind {
        StorageKind::Memcached
    }
}

// =====================================================================
// FileStorage — stub
// =====================================================================

/// 文件存储 — stub, 0 真接 fs_err / sled。
pub struct FileStorage {
    _path: Option<String>,
}

impl FileStorage {
    /// 新建文件 stub。
    pub fn new(path: Option<String>) -> Self {
        Self { _path: path }
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Err(RateLimiterError::NotImplemented(
            "FileStorage::get — 留 R21+ 续真接 fs_err/sled".to_string(),
        ))
    }
    async fn set(&self, _key: &str, _value: Vec<u8>, _ttl: Option<Duration>) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "FileStorage::set — 留 R21+ 续真接 fs_err/sled".to_string(),
        ))
    }
    async fn delete(&self, _key: &str) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "FileStorage::delete — 留 R21+ 续真接 fs_err/sled".to_string(),
        ))
    }
    fn kind(&self) -> StorageKind {
        StorageKind::File
    }
}

// =====================================================================
// DistributedStorage — stub
// =====================================================================

/// 分布式存储 — stub, 0 真接 Raft / Paxos。
pub struct DistributedStorage {
    _connection_string: Option<String>,
}

impl DistributedStorage {
    /// 新建分布式 stub。
    pub fn new(connection_string: Option<String>) -> Self {
        Self {
            _connection_string: connection_string,
        }
    }
}

#[async_trait]
impl Storage for DistributedStorage {
    async fn get(&self, _key: &str) -> Result<Option<Vec<u8>>> {
        Err(RateLimiterError::NotImplemented(
            "DistributedStorage::get — 留 R22+ 续真接 Raft/Paxos".to_string(),
        ))
    }
    async fn set(&self, _key: &str, _value: Vec<u8>, _ttl: Option<Duration>) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "DistributedStorage::set — 留 R22+ 续真接 Raft/Paxos".to_string(),
        ))
    }
    async fn delete(&self, _key: &str) -> Result<()> {
        Err(RateLimiterError::NotImplemented(
            "DistributedStorage::delete — 留 R22+ 续真接 Raft/Paxos".to_string(),
        ))
    }
    fn kind(&self) -> StorageKind {
        StorageKind::Distributed
    }
}

// =====================================================================
// 工厂
// =====================================================================

/// 根据 `StorageConfig` 构造对应 storage。
///
/// in-memory 返回完整实现, 其他 4 种返回 stub。
pub fn build_storage(cfg: &crate::config::StorageConfig) -> Box<dyn Storage> {
    match cfg.kind {
        StorageKind::InMemory => Box::new(InMemoryStorage::new()),
        StorageKind::Redis => Box::new(RedisStorage::new(cfg.connection_string.clone())),
        StorageKind::Memcached => {
            Box::new(MemcachedStorage::new(cfg.connection_string.clone()))
        }
        StorageKind::File => Box::new(FileStorage::new(cfg.file_path.clone())),
        StorageKind::Distributed => {
            Box::new(DistributedStorage::new(cfg.connection_string.clone()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_set_get_round_trip() {
        let s = InMemoryStorage::new();
        s.set("k1", b"v1".to_vec(), None).await.unwrap();
        let v = s.get("k1").await.unwrap();
        assert_eq!(v, Some(b"v1".to_vec()));
    }

    #[tokio::test]
    async fn in_memory_ttl_expires() {
        let s = InMemoryStorage::new();
        s.set("k1", b"v1".to_vec(), Some(Duration::from_millis(10)))
            .await
            .unwrap();
        assert!(s.get("k1").await.unwrap().is_some());
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(s.get("k1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn in_memory_delete() {
        let s = InMemoryStorage::new();
        s.set("k1", b"v1".to_vec(), None).await.unwrap();
        s.delete("k1").await.unwrap();
        assert!(s.get("k1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn in_memory_purge_expired_removes_dead() {
        let s = InMemoryStorage::new();
        s.set("alive", b"a".to_vec(), None).await.unwrap();
        s.set("dead", b"d".to_vec(), Some(Duration::from_millis(1)))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
        let removed = s.purge_expired();
        assert_eq!(removed, 1);
        assert_eq!(s.len(), 1);
    }

    #[tokio::test]
    async fn redis_storage_get_returns_not_implemented() {
        let s = RedisStorage::new(None);
        let r = s.get("k").await;
        assert!(matches!(r, Err(RateLimiterError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn memcached_storage_set_returns_not_implemented() {
        let s = MemcachedStorage::new(None);
        let r = s.set("k", b"v".to_vec(), None).await;
        assert!(matches!(r, Err(RateLimiterError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn file_storage_delete_returns_not_implemented() {
        let s = FileStorage::new(None);
        let r = s.delete("k").await;
        assert!(matches!(r, Err(RateLimiterError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn distributed_storage_set_returns_not_implemented() {
        let s = DistributedStorage::new(None);
        let r = s.set("k", b"v".to_vec(), None).await;
        assert!(matches!(r, Err(RateLimiterError::NotImplemented(_))));
    }

    #[tokio::test]
    async fn build_storage_returns_correct_kind() {
        let cfg = crate::config::StorageConfig::default();
        let s = build_storage(&cfg);
        assert_eq!(s.kind(), StorageKind::InMemory);
    }
}
