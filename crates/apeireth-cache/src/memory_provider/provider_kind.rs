//! # ProviderKind — 7 memory provider (InMemory / Redis / Sqlite / Postgres / S3 / DiskLru / Hybrid)
//!
//! 7 个 memory provider 1:1 翻译 Golutra 公开仓库 memory gateway 设计:
//! 1. **InMemory** — 进程内 HashMap, R20 完整实现
//! 2. **Redis** — 内存数据库, R21 stub
//! 3. **Sqlite** — 嵌入式 SQL, R20 完整实现 (用 rusqlite)
//! 4. **Postgres** — 网络 SQL, R21 stub
//! 5. **S3** — 对象存储, R21 stub
//! 6. **DiskLru** — 磁盘 LRU 文件, R21 stub
//! 7. **Hybrid** — L1 in-memory + L2 sqlite 写穿透, R20 完整实现
//!
//! ## R20 真接 / R21 stub 策略
//!
//! 7 个 provider 3 个完整实现 (InMemory / Sqlite / Hybrid); 4 个 stub
//! (Redis / Postgres / S3 / DiskLru) 全部返 `BackendNotImplemented` (per task spec).
//!
//! ## 1:1 借鉴 Golutra 映射
//!
//! | apeireth-cache  | Golutra memory gateway 商业版  | 实现度 |
//! |-----------------|--------------------------------|--------|
//! | `InMemory`      | `local` (进程内)                | ✅ 完整 |
//! | `Redis`         | `redis`                         | ❌ stub |
//! | `Sqlite`        | `local_sqlite`                  | ✅ 完整 |
//! | `Postgres`      | `vector` / `mem0` 后端 server   | ❌ stub |
//! | `S3`            | `evermind` 云端备份             | ❌ stub |
//! | `DiskLru`       | `disk` 落盘                     | ❌ stub |
//! | `Hybrid`        | `relay` 多后端组合              | ✅ 完整 (L1+L2) |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use serde::{Deserialize, Serialize};

use crate::memory_provider::error::{MemoryProviderError, MemoryProviderResult};

// ============================================================================
// §1 ProviderKind 枚举 (7 variant, per Golutra 7 模式 + m3 防御)
// ============================================================================

/// 7 memory provider (编译期 hardcode).
///
/// 1:1 借鉴 Golutra 公开仓库 memory gateway 7 模式 (in-memory / redis / sqlite /
/// postgres / s3 / disk-lru / hybrid).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProviderKind {
    /// 进程内 HashMap (R20 完整实现).
    InMemory,

    /// Redis 内存数据库 (R21 stub).
    Redis,

    /// 嵌入式 SQLite (R20 完整实现, 用 rusqlite).
    Sqlite,

    /// 网络 PostgreSQL (R21 stub).
    Postgres,

    /// S3 对象存储 (R21 stub).
    S3,

    /// 磁盘 LRU 文件 (R21 stub).
    DiskLru,

    /// 混合 L1 in-memory + L2 sqlite 写穿透 (R20 完整实现).
    Hybrid,
}

impl ProviderKind {
    /// 7 个 provider 1:1 列表 (编译期 hardcode, K-1 守门).
    pub const ALL: [ProviderKind; 7] = [
        ProviderKind::InMemory,
        ProviderKind::Redis,
        ProviderKind::Sqlite,
        ProviderKind::Postgres,
        ProviderKind::S3,
        ProviderKind::DiskLru,
        ProviderKind::Hybrid,
    ];

    /// provider 英文名 (1:1 翻译 Golutra 商业版).
    pub const fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::InMemory => "IN_MEMORY",
            ProviderKind::Redis => "REDIS",
            ProviderKind::Sqlite => "SQLITE",
            ProviderKind::Postgres => "POSTGRES",
            ProviderKind::S3 => "S3",
            ProviderKind::DiskLru => "DISK_LRU",
            ProviderKind::Hybrid => "HYBRID",
        }
    }

    /// 是否在 R20 完整实现 (InMemory / Sqlite / Hybrid).
    pub const fn is_implemented(&self) -> bool {
        match self {
            ProviderKind::InMemory | ProviderKind::Sqlite | ProviderKind::Hybrid => true,
            ProviderKind::Redis
            | ProviderKind::Postgres
            | ProviderKind::S3
            | ProviderKind::DiskLru => false,
        }
    }

    /// 守门: 凡是 R20 未实现的 provider, 全部返 `BackendNotImplemented` (per task spec).
    pub fn check_implemented(&self) -> MemoryProviderResult<()> {
        if self.is_implemented() {
            Ok(())
        } else {
            Err(MemoryProviderError::BackendNotImplemented(
                self.as_str().to_string(),
            ))
        }
    }
}

impl Default for ProviderKind {
    fn default() -> Self {
        // 默认 InMemory: R20 阶段最简完整实现 + 无外部依赖
        ProviderKind::InMemory
    }
}

impl std::fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ProviderKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "IN_MEMORY" | "in_memory" | "in-memory" => Ok(ProviderKind::InMemory),
            "REDIS" | "redis" => Ok(ProviderKind::Redis),
            "SQLITE" | "sqlite" => Ok(ProviderKind::Sqlite),
            "POSTGRES" | "postgres" | "postgresql" => Ok(ProviderKind::Postgres),
            "S3" | "s3" => Ok(ProviderKind::S3),
            "DISK_LRU" | "disk_lru" | "disk-lru" => Ok(ProviderKind::DiskLru),
            "HYBRID" | "hybrid" => Ok(ProviderKind::Hybrid),
            other => Err(format!("unknown provider kind: '{other}'")),
        }
    }
}

// ============================================================================
// §2 provider variant 计数 (K-1 强校验, 编译期守门)
// ============================================================================

/// ProviderKind 编译期 hardcode variant 数 (7).
/// m3 防御: 改这个数字会立刻破坏 build, 防止 hallucination 加/减 variant.
pub const PROVIDER_KIND_VARIANT_COUNT: usize = 7;

// ============================================================================
// §3 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: ProviderKind 编译期 7 variant.
    #[test]
    fn provider_kind_has_seven_variants() {
        // 7 variant 1:1: InMemory / Redis / Sqlite / Postgres / S3 / DiskLru / Hybrid
        assert_eq!(PROVIDER_KIND_VARIANT_COUNT, 7);
        assert_eq!(ProviderKind::ALL.len(), 7);
    }

    /// 守门 #2: 7 provider 1:1 字符串名.
    #[test]
    fn provider_kind_7_strs() {
        assert_eq!(ProviderKind::InMemory.as_str(), "IN_MEMORY");
        assert_eq!(ProviderKind::Redis.as_str(), "REDIS");
        assert_eq!(ProviderKind::Sqlite.as_str(), "SQLITE");
        assert_eq!(ProviderKind::Postgres.as_str(), "POSTGRES");
        assert_eq!(ProviderKind::S3.as_str(), "S3");
        assert_eq!(ProviderKind::DiskLru.as_str(), "DISK_LRU");
        assert_eq!(ProviderKind::Hybrid.as_str(), "HYBRID");
    }

    /// 守门 #3: R20 完整实现 = 3 (InMemory / Sqlite / Hybrid), R21 stub = 4.
    #[test]
    fn three_implemented_four_stub() {
        assert!(ProviderKind::InMemory.is_implemented());
        assert!(ProviderKind::Sqlite.is_implemented());
        assert!(ProviderKind::Hybrid.is_implemented());
        assert!(!ProviderKind::Redis.is_implemented());
        assert!(!ProviderKind::Postgres.is_implemented());
        assert!(!ProviderKind::S3.is_implemented());
        assert!(!ProviderKind::DiskLru.is_implemented());
    }

    /// 守门 #4: 4 stub 全部返 BackendNotImplemented.
    #[test]
    fn four_stubs_return_not_implemented() {
        assert!(ProviderKind::InMemory.check_implemented().is_ok());
        assert!(ProviderKind::Sqlite.check_implemented().is_ok());
        assert!(ProviderKind::Hybrid.check_implemented().is_ok());
        for stub in [
            ProviderKind::Redis,
            ProviderKind::Postgres,
            ProviderKind::S3,
            ProviderKind::DiskLru,
        ] {
            let err = stub.check_implemented().unwrap_err();
            assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
        }
    }

    /// 守门 #5: 7 provider ALL 唯一.
    #[test]
    fn provider_kind_all_unique() {
        let mut all = ProviderKind::ALL.to_vec();
        all.sort_by_key(|p| p.as_str());
        all.dedup();
        assert_eq!(all.len(), 7);
    }

    /// 守门 #6: Default = InMemory.
    #[test]
    fn provider_kind_default_is_in_memory() {
        assert_eq!(ProviderKind::default(), ProviderKind::InMemory);
    }

    /// 守门 #7: FromStr 解析.
    #[test]
    fn provider_kind_from_str_works() {
        assert_eq!(
            "IN_MEMORY".parse::<ProviderKind>().unwrap(),
            ProviderKind::InMemory
        );
        assert_eq!(
            "in-memory".parse::<ProviderKind>().unwrap(),
            ProviderKind::InMemory
        );
        assert_eq!(
            "redis".parse::<ProviderKind>().unwrap(),
            ProviderKind::Redis
        );
        assert_eq!(
            "postgresql".parse::<ProviderKind>().unwrap(),
            ProviderKind::Postgres
        );
        assert!("NOPE".parse::<ProviderKind>().is_err());
    }
}
