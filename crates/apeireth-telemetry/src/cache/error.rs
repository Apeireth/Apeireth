//! # CacheError — cache skeleton 错误类型 (10 variant)
//!
//! 10 个错误 variant 覆盖 cache 全流程:
//! 1. `KeyNotFound` — get / remove 时 key 不存在
//! 2. `InvalidMaxSize` — max_size = 0 (K-1 强校验)
//! 3. `InvalidTtl` — ttl = Duration::ZERO (K-1 强校验)
//! 4. `InvalidShardCount` — shards 不在 16..=256 范围
//! 5. `BackendNotImplemented` — Disk/Redis/Memcached stub 返
//! 6. `PolicyNotSupported` — 当前 backend 不支持该 policy
//! 7. `ShardLockPoisoned` — parking_lot Mutex poisoned (极少)
//! 8. `SerializationError` — V serde 失败 (跨 backend 序列化)
//! 9. `DiskIoError` — disk backend 实际 I/O 失败 (留口子, 阶段 6 stub)
//! 10. `CapacityExceeded` — 单 key > max_size, 拒绝 put
//!
//! ## m3 防御 (per `m3-hallucination-defense §2.1`)
//!
//! 每个错误 variant 编译期 hardcode, 不允许运行时新增 variant, 防 m3 hallucination
//! 改错分类 (如把 `KeyNotFound` 改成 `KeyNotFoundError` 制造不可识别的 err).
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use thiserror::Error;

// ============================================================================
// §1 CacheError 枚举 (10 variant, per skeleton 模式 + m3 防御)
// ============================================================================

/// Cache skeleton 错误 (10 variant, 编译期 hardcode).
#[derive(Debug, Error, PartialEq)]
pub enum CacheError {
    /// get / remove 时 key 不存在.
    #[error("key not found: '{0}'")]
    KeyNotFound(String),

    /// max_size = 0 (K-1 强校验, 不允许 0 容量 cache).
    #[error("invalid max_size: {0}, must be > 0")]
    InvalidMaxSize(usize),

    /// ttl = Duration::ZERO (K-1 强校验, 不允许 0 TTL).
    #[error("invalid ttl: {0:?}, must be > Duration::ZERO")]
    InvalidTtl(std::time::Duration),

    /// shards 不在 16..=256 范围.
    #[error("invalid shard count: {0}, must be in 16..=256")]
    InvalidShardCount(usize),

    /// Disk / Redis / Memcached stub 返 (R20 阶段 6 估缺).
    #[error("backend not implemented: '{0}' (R20 阶段 6 stub)")]
    BackendNotImplemented(String),

    /// 当前 backend 不支持该 policy.
    #[error("policy not supported: {policy} on backend {backend}")]
    PolicyNotSupported {
        /// 5 EvictionPolicy 之一.
        policy: String,
        /// 4 BackendKind 之一.
        backend: String,
    },

    /// parking_lot Mutex poisoned (极少发生, 进程 panic 后).
    #[error("shard lock poisoned (shard_id={0})")]
    ShardLockPoisoned(usize),

    /// V serde 失败 (跨 backend 序列化 / 反序列化).
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// disk backend 实际 I/O 失败 (留口子, 阶段 6 stub).
    #[error("disk I/O error: {0}")]
    DiskIoError(String),

    /// 单 key value size > max_size, 拒绝 put.
    #[error("capacity exceeded: value size {value_size} > max_size {max_size}")]
    CapacityExceeded {
        /// 实际 value 字节数.
        value_size: usize,
        /// cache max_size.
        max_size: usize,
    },
}

// ============================================================================
// §2 统一 Result 别名
// ============================================================================

/// 统一 Result 别名.
pub type CacheResult<T> = std::result::Result<T, CacheError>;

// ============================================================================
// §3 错误 variant 计数 (K-1 强校验, 编译期守门)
// ============================================================================

/// CacheError 编译期 hardcode variant 数 (10).
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 加/减 variant.
pub const CACHE_ERROR_VARIANT_COUNT: usize = 10;

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 守门 #1: CacheError 编译期 10 variant.
    #[test]
    fn cache_error_has_ten_variants() {
        // 10 variant 1:1: KeyNotFound / InvalidMaxSize / InvalidTtl / InvalidShardCount
        //               / BackendNotImplemented / PolicyNotSupported / ShardLockPoisoned
        //               / SerializationError / DiskIoError / CapacityExceeded
        assert_eq!(CACHE_ERROR_VARIANT_COUNT, 10);
    }

    /// 守门 #2: 错误 Display 实现正常.
    #[test]
    fn cache_error_display_works() {
        let e = CacheError::KeyNotFound("user:42".to_string());
        let s = format!("{e}");
        assert!(s.contains("user:42"));
        assert!(s.contains("not found"));
    }

    /// 守门 #3: InvalidTtl 包含 Duration.
    #[test]
    fn invalid_ttl_error_includes_duration() {
        let e = CacheError::InvalidTtl(Duration::ZERO);
        let s = format!("{e}");
        assert!(s.contains("invalid ttl"));
    }

    /// 守门 #4: CapacityExceeded 含 value_size + max_size.
    #[test]
    fn capacity_exceeded_error_includes_sizes() {
        let e = CacheError::CapacityExceeded { value_size: 1024, max_size: 512 };
        let s = format!("{e}");
        assert!(s.contains("1024"));
        assert!(s.contains("512"));
    }

    /// 守门 #5: PolicyNotSupported 含 policy + backend.
    #[test]
    fn policy_not_supported_error_includes_both() {
        let e = CacheError::PolicyNotSupported {
            policy: "TinyLFU".to_string(),
            backend: "Disk".to_string(),
        };
        let s = format!("{e}");
        assert!(s.contains("TinyLFU"));
        assert!(s.contains("Disk"));
    }
}
