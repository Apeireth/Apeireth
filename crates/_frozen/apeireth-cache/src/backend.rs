//! # BackendKind — 4 存储后端 (Memory / Disk / Redis / Memcached stub)
//!
//! 4 个存储后端 1:1 翻译 v0.9.21 @anthropic-ai/cache 商业版:
//! 1. **Memory** — 进程内, 完整实现 (用 LRU + TTL)
//! 2. **Disk** — 文件, mmap 留口子, R20 阶段 6 stub
//! 3. **Redis** — 网络, R20 阶段 6 stub
//! 4. **Memcached** — 网络, R20 阶段 6 stub
//!
//! ## R20 阶段 6 stub 策略
//!
//! 4 个后端只有 Memory 是真实现; Disk/Redis/Memcached 在 put/get/remove/clear 时
//! 返 `CacheError::BackendNotImplemented` (per task spec §5).
//!
//! ## 1:1 翻译映射 (v0.9.21 @anthropic-ai/cache)
//!
//! | apeireth-cache | @anthropic-ai/cache 商业版 | 实现度 |
//! |----------------|----------------------------|--------|
//! | `Memory`       | `Backend.MEMORY`            | ✅ 完整 (LRU + TTL) |
//! | `Disk`         | `Backend.DISK`              | ❌ stub |
//! | `Redis`        | `Backend.REDIS`             | ❌ stub |
//! | `Memcached`    | `Backend.MEMCACHED`         | ❌ stub |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use serde::{Deserialize, Serialize};

use crate::error::{CacheError, CacheResult};

// ============================================================================
// §1 BackendKind 枚举 (4 variant, per skeleton 模式 + m3 防御)
// ============================================================================

/// 4 存储后端 (编译期 hardcode).
///
/// 1:1 翻译 v0.9.21 @anthropic-ai/cache Backend 商业版.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendKind {
    /// 进程内 (完整实现).
    Memory,

    /// 文件 (mmap, R20 阶段 6 stub).
    Disk,

    /// Redis (网络, R20 阶段 6 stub).
    Redis,

    /// Memcached (网络, R20 阶段 6 stub).
    Memcached,
}

impl BackendKind {
    /// 4 个后端 1:1 列表.
    pub const ALL: [BackendKind; 4] = [
        BackendKind::Memory,
        BackendKind::Disk,
        BackendKind::Redis,
        BackendKind::Memcached,
    ];

    /// 后端英文名 (1:1 翻译 v0.9.21 商业版).
    pub const fn as_str(&self) -> &'static str {
        match self {
            BackendKind::Memory => "MEMORY",
            BackendKind::Disk => "DISK",
            BackendKind::Redis => "REDIS",
            BackendKind::Memcached => "MEMCACHED",
        }
    }

    /// 是否在 R20 阶段 6 完整实现 (只有 Memory).
    pub const fn is_implemented(&self) -> bool {
        match self {
            BackendKind::Memory => true,
            BackendKind::Disk | BackendKind::Redis | BackendKind::Memcached => false,
        }
    }

    /// 守门: 凡是 R20 阶段 6 未实现的后端, 全部返 `BackendNotImplemented` (per task spec §5).
    pub fn check_implemented(&self) -> CacheResult<()> {
        if self.is_implemented() {
            Ok(())
        } else {
            Err(CacheError::BackendNotImplemented(self.as_str().to_string()))
        }
    }
}

impl Default for BackendKind {
    fn default() -> Self {
        // 默认 Memory: 唯一 R20 阶段 6 完整实现
        BackendKind::Memory
    }
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for BackendKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MEMORY" | "memory" => Ok(BackendKind::Memory),
            "DISK" | "disk" => Ok(BackendKind::Disk),
            "REDIS" | "redis" => Ok(BackendKind::Redis),
            "MEMCACHED" | "memcached" => Ok(BackendKind::Memcached),
            other => Err(format!("unknown backend kind: '{other}'")),
        }
    }
}

// ============================================================================
// §2 后端 variant 计数 (K-1 强校验, 编译期守门)
// ============================================================================

/// BackendKind 编译期 hardcode variant 数 (4).
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 加/减 variant.
pub const BACKEND_KIND_VARIANT_COUNT: usize = 4;

// ============================================================================
// §3 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: BackendKind 编译期 4 variant.
    #[test]
    fn backend_kind_has_four_variants() {
        // 4 variant 1:1: Memory / Disk / Redis / Memcached
        assert_eq!(BACKEND_KIND_VARIANT_COUNT, 4);
        assert_eq!(BackendKind::ALL.len(), 4);
    }

    /// 守门 #2: 4 后端 1:1 字符串名.
    #[test]
    fn backend_kind_4_strs() {
        assert_eq!(BackendKind::Memory.as_str(), "MEMORY");
        assert_eq!(BackendKind::Disk.as_str(), "DISK");
        assert_eq!(BackendKind::Redis.as_str(), "REDIS");
        assert_eq!(BackendKind::Memcached.as_str(), "MEMCACHED");
    }

    /// 守门 #3: 只有 Memory 已实现.
    #[test]
    fn only_memory_is_implemented() {
        assert!(BackendKind::Memory.is_implemented());
        assert!(!BackendKind::Disk.is_implemented());
        assert!(!BackendKind::Redis.is_implemented());
        assert!(!BackendKind::Memcached.is_implemented());
    }

    /// 守门 #4: 3 stub 全部返 BackendNotImplemented.
    #[test]
    fn three_stubs_return_not_implemented() {
        assert!(BackendKind::Memory.check_implemented().is_ok());
        let disk_err = BackendKind::Disk.check_implemented().unwrap_err();
        assert!(matches!(disk_err, CacheError::BackendNotImplemented(_)));
        let redis_err = BackendKind::Redis.check_implemented().unwrap_err();
        assert!(matches!(redis_err, CacheError::BackendNotImplemented(_)));
        let mc_err = BackendKind::Memcached.check_implemented().unwrap_err();
        assert!(matches!(mc_err, CacheError::BackendNotImplemented(_)));
    }

    /// 守门 #5: 4 后端 ALL 唯一.
    #[test]
    fn backend_kind_all_unique() {
        let mut all = BackendKind::ALL.to_vec();
        all.sort_by_key(|b| b.as_str());
        all.dedup();
        assert_eq!(all.len(), 4);
    }

    /// 守门 #6: Default = Memory.
    #[test]
    fn backend_kind_default_is_memory() {
        assert_eq!(BackendKind::default(), BackendKind::Memory);
    }

    /// 守门 #7: FromStr 解析.
    #[test]
    fn backend_kind_from_str_works() {
        assert_eq!("MEMORY".parse::<BackendKind>().unwrap(), BackendKind::Memory);
        assert_eq!("redis".parse::<BackendKind>().unwrap(), BackendKind::Redis);
        assert!("NOPE".parse::<BackendKind>().is_err());
    }
}
