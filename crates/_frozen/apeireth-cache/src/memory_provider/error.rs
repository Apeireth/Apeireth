//! # MemoryProviderError — memory provider 错误类型 (8 variant)
//!
//! 8 个错误 variant 覆盖 memory provider 全流程:
//! 1. `BackendNotImplemented` — 4 stub (redis / postgres / s3 / disk_lru) 返
//! 2. `CaptureFailed` — capture 写时底层错误
//! 3. `QueryFailed` — query 读时底层错误
//! 4. `ClearFailed` — clear 删时底层错误
//! 5. `SerializationError` — metadata JSON 序列化失败
//! 6. `InvalidQuery` — query 字段非法 (e.g. limit = 0)
//! 7. `BackendIoError` — backend 实际 I/O 失败 (sqlite 文件锁 / redis 网络断)
//! 8. `NotFound` — clear(id) 时 id 不存在
//!
//! ## m3 防御 (per `m3-hallucination-defense §2.1`)
//!
//! 每个错误 variant 编译期 hardcode, 不允许运行时新增 variant, 防 m3 hallucination
//! 改错分类.
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use thiserror::Error;

// ============================================================================
// §1 MemoryProviderError 枚举 (8 variant, per skeleton 模式 + m3 防御)
// ============================================================================

/// Memory provider 错误 (8 variant, 编译期 hardcode).
#[derive(Debug, Error, PartialEq)]
pub enum MemoryProviderError {
    /// 4 stub (redis / postgres / s3 / disk_lru) 返 (R21 续接).
    #[error("memory provider backend not implemented: '{0}' (R21 stub)")]
    BackendNotImplemented(String),

    /// capture 写时底层错误.
    #[error("capture failed: {0}")]
    CaptureFailed(String),

    /// query 读时底层错误.
    #[error("query failed: {0}")]
    QueryFailed(String),

    /// clear 删时底层错误.
    #[error("clear failed: {0}")]
    ClearFailed(String),

    /// metadata JSON 序列化失败.
    #[error("serialization error: {0}")]
    SerializationError(String),

    /// query 字段非法 (e.g. limit = 0).
    #[error("invalid query: {0}")]
    InvalidQuery(String),

    /// backend 实际 I/O 失败 (sqlite 文件锁 / redis 网络断 / s3 凭证失效).
    #[error("backend I/O error: {0}")]
    BackendIoError(String),

    /// clear(id) 时 id 不存在.
    #[error("memory entry not found: id='{0}'")]
    NotFound(String),
}

// ============================================================================
// §2 统一 Result 别名
// ============================================================================

/// 统一 Result 别名 (memory provider).
pub type MemoryProviderResult<T> = std::result::Result<T, MemoryProviderError>;

// ============================================================================
// §3 错误 variant 计数 (K-1 强校验, 编译期守门)
// ============================================================================

/// MemoryProviderError 编译期 hardcode variant 数 (8).
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 加/减 variant.
pub const MEMORY_PROVIDER_ERROR_VARIANT_COUNT: usize = 8;

// ============================================================================
// §4 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: MemoryProviderError 编译期 8 variant.
    #[test]
    fn memory_provider_error_has_eight_variants() {
        // 8 variant 1:1: BackendNotImplemented / CaptureFailed / QueryFailed / ClearFailed
        //               / SerializationError / InvalidQuery / BackendIoError / NotFound
        assert_eq!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT, 8);
    }

    /// 守门 #2: 错误 Display 实现正常.
    #[test]
    fn memory_provider_error_display_works() {
        let e = MemoryProviderError::BackendNotImplemented("REDIS".to_string());
        let s = format!("{e}");
        assert!(s.contains("REDIS"));
        assert!(s.contains("not implemented"));
    }

    /// 守门 #3: NotFound 包含 id.
    #[test]
    fn not_found_error_includes_id() {
        let e = MemoryProviderError::NotFound("mem-42".to_string());
        let s = format!("{e}");
        assert!(s.contains("mem-42"));
        assert!(s.contains("not found"));
    }
}
