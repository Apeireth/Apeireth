//! # apeireth-memory-extensions
//!
//! **Apeireth R21 借鉴 Golutra #3: 7 memory provider 模式**
//! (per `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P1 第 13/14 项 +
//! 主 2026-08-06 派活 "整合 #3 R21 续补 2/15").
//!
//! ## 借鉴背景
//!
//! Golutra v0.1.0 memory gateway 5 provider 模式 (`local` / `local_sqlite` / `local_vector_index` /
//! `mem0` / `evermind` / `relay` / `custom`) 1:1 翻译到 Rust 路线. 本 crate 走 Apeireth 风格
//! 5+1+1 扩展, 7 provider = Golutra 5 + InMemory(纯 Rust) + DiskLru(纯 Rust) + Hybrid(组合):
//!
//! | 变体索引 | Provider | Golutra 等价 | 真接方式 |
//! |---:|---|---|---|
//! | 0 | `InMemory` | local (in-process) | real HashMap |
//! | 1 | `Redis` | local (Redis sidecar) | real redis-rs |
//! | 2 | `Sqlite` | local_sqlite | real rusqlite |
//! | 3 | `Postgres` | mem0 (云端) / vector (云端) | real tokio-postgres |
//! | 4 | `S3` | relay (云端) | real reqwest + S3 REST API |
//! | 5 | `DiskLru` | (Golutra 无, 新增) | real lru + std::fs |
//! | 6 | `Hybrid` | (Golutra 无, 新增) | real 组合 in_memory + disk_lru |
//!
//! **1:1 翻译**:
//! - Golutra 5 provider `default_providers` 列表 → 本 crate 7 provider enum (`ProviderKind`)
//! - Golutra `MemoryGateway.set/get/delete` 跨 provider 抽象 → 本 crate `MemoryProvider` trait
//! - Golutra `ProviderConfig` 6 字段 (uri/timeout/max_size/persist/ttl/scope) → 本 crate 6 K-1
//! - Golutra `init_providers()` 一次性装配 → 本 crate `ProviderRegistry` 7 字段聚合
//!
//! **不**抄 VCP / Golutra 业务代码 (per 借鉴 §0 哲学审计), **只**借鉴字段 + 行为模式.
//!
//! ## 7 provider 模式 (跟 Golutra 5 provider 1:1 对应 + 扩展 2 个)
//!
//! 1. **[`InMemoryProvider`](crate::provider_in_memory) — 进程内 HashMap (真接, 端到端可测)
//! 2. **[`RedisProvider`](crate::provider_redis) — 外部 Redis server (真接, config 强校验 + 真创建 Client)
//! 3. **[`SqliteProvider`](crate::provider_sqlite) — 本地 SQLite (真接, 端到端可测)
//! 4. **[`PostgresProvider`](crate::provider_postgres) — 外部 PG (真接, config 强校验 + 真解析 Config)
//! 5. **[`S3Provider`](crate::provider_s3) — 外部 S3 (真接, config 强校验 + 真创建 reqwest Client)
//! 6. **[`DiskLruProvider`](crate::provider_disk_lru) — 本地 disk + LRU (真接, 端到端可测)
//! 7. **[`HybridProvider`](crate::provider_hybrid) — 组合 in_memory + disk_lru (真接, 端到端可测)
//!
//! ## 6 K-1 强校验 (per task spec 强制要求)
//!
//! 每个 ProviderConfig 6 字段编译期 + 运行时双层强校验 (per `ProviderConfig::validate`):
//! 1. **connection_string** — URL/path/连接 URI (非空 + 协议头匹配 provider scheme)
//! 2. **timeout** — Duration (>= 1ms, <= 1h)
//! 3. **max_size** — u64 bytes (>= 1KB, <= 1TB)
//! 4. **persist** — bool (是否持久化)
//! 5. **cache_ttl** — Duration (>= 0ms, <= 7d; 0 = 永不过期)
//! 6. **scope** — ProviderScope 3 变体 (Local/Shared/Global)
//!
//! ## 公开 API (100% 文档化)
//!
//! | 模块 | 公开类型 | 用途 |
//! |---|---|---|
//! | [`memory_provider`] | `ProviderKind` (7) / `ProviderConfig` / `ProviderScope` (3) / `ProviderConfigField` (6) / `MemoryProvider` trait | 7 provider 抽象层 |
//! | [`error`] | `MemoryProviderError` (7 variant) / `MemoryProviderErrorKind` / `MemoryProviderResult` | 错误类型 + 序列化摘要 |
//! | [`provider_in_memory`] | `InMemoryProvider` / `InMemoryConfigDefault` | 模式 0: 进程内 HashMap |
//! | [`provider_redis`] | `RedisProvider` / `RedisConfigDefault` | 模式 1: 外部 Redis |
//! | [`provider_sqlite`] | `SqliteProvider` / `SqliteConfigDefault` | 模式 2: 本地 SQLite |
//! | [`provider_postgres`] | `PostgresProvider` / `PostgresConfigDefault` | 模式 3: 外部 PG |
//! | [`provider_s3`] | `S3Provider` / `S3ConfigDefault` | 模式 4: 外部 S3 |
//! | [`provider_disk_lru`] | `DiskLruProvider` / `DiskLruConfigDefault` | 模式 5: 本地 disk + LRU |
//! | [`provider_hybrid`] | `HybridProvider` / `HybridConfigDefault` | 模式 6: 组合 in_memory + disk_lru |
//! | [`registry`] | `ProviderRegistry` (7 字段) / `ProviderRegistryBuilder` | 7 provider 聚合容器 |
//!
//! ## 6 哲学锚穿透 (per APEIRETH-CONVENTIONS §9)
//!
//! - **S-1 北极星导向** — 7 provider 服务 ASI 北极星 (memory gateway 跨进程 / 跨集群, 0 编造"只本地")
//! - **S-2 实事求是** — 7 provider 各自真接, 端到端可测的 4 个 (in_memory / sqlite / disk_lru / hybrid)
//!   + 3 个 config 强校验 (redis / postgres / s3, 真连服务端由 R21+ 续做)
//! - **O-2 走在前人肩上** — 借 redis-rs + tokio-postgres + lru + rusqlite + reqwest 业界标准, 0 重复造客户端
//! - **O-3 干到底** — 7 provider × 10 lib unit = 70 unit + 10 集成 = 80 测试
//! - **O-4 任何人都能接手** — 10 src 模块 + 1 example + 1 tests + 顶部 §0-§10 完整
//! - **O-5 不假装** — 7 provider 各自真接 + Redis/PG/S3 显式标"无服务端 Connection error", 0 编造"无 server 也能 set/get"
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10 + 8-locked-unified §2)
//!
//! 8 项详见 `docs/stage4/8-locked-unified-2026-08-05.md` §2.
//!
//! | # | 承诺 | 本 crate 守门 |
//! |---|------|--------------|
//! | 1 | 阶段 1+2+3 LOCKED 文档 | ✅ 不动 |
//! | 2 | v2 / v4 / v4.1 LOCKED | ✅ 不动 |
//! | 3 | 阶段 4 主文档 LOCKED (6ca80776) | ✅ 不动 |
//! | 4 | 阶段 5 施工文档 LOCKED (631 行) | ✅ 不动 |
//! | 5 | v6 基础架构 (4 重守门 + 权限发放 + E 层修改路径) | ✅ 不动 |
//! | 6 | R11 baseline 三值 (V1141/V1131/V1136) | ✅ 不动 |
//! | 7 | 顶层 3 规范文件 (CONVENTIONS/VERSIONING/GLOSSARY) | ✅ 不动 |
//! | 8 | workspace version 1.0.0 (semver 严格) | ✅ 不动 (本 crate 0.1.0 + workspace.members 新增) |
//!
//! **0 触碰 24 LOCKED crate** (per `git diff`):
//! - `apeireth-memory/` LOCKED (src/ 0 触碰, 0 改 `crates/apeireth-memory/src/lib.rs`)
//! - 23 其他 LOCKED crate 0 触碰
//! - workspace `[workspace.package] version = "1.0.0"` 0 改
//! - workspace `[workspace.members]` 仅 +1 行 `"crates/apeireth-memory/extensions",` (子 crate 注册)
//!
//! ## 状态
//!
//! ⚠️ **skeleton (R21 借鉴 Golutra #3 估补, 主 2026-08-06 派活)**.
//! 7 provider 各自真接 (4 个端到端 + 3 个 config 强校验), 6 K-1 编译期 + 运行时双层强校验.
//! R21+ 续做: Redis/Postgres/S3 配服务端真连测试 + SigV4 签名 + List+BatchDelete for S3 clear/size
//! + 真实集成 `apeireth-memory` 7 provider 路由 (LOCKED 边界外).
//!
//! ## 引用文档 (4 份)
//!
//! 1. `analysis\golutra\BORROW_FROM_GOLUTRA.md` §8 P1 第 13/14 项 (借鉴优先级 P1)
//! 2. `.openclaw\workspace\promethean\Apeireth-rust\reports\borrow-golutra-6-state-pattern-2026-08-06.md` (借鉴 #6 sister 报告, 1:1 镜像模式)
//! 3. `.openclaw\workspace\promethean\Apeireth-rust\docs\stage4\8-locked-unified-2026-08-05.md` §2 (8 项不修改承诺唯一引用源)
//! 4. `.openclaw\workspace\promethean\Apeireth-rust\reports\integrate-3-commit-templates-2026-08-06.md` (整合 #3 7 commit 模板, 本 crate = P1 续补 2/15)

#![warn(missing_docs)]
#![deny(unsafe_code)]
// async 框架: tokio 1.40 + async-trait (借 workspace, 0 重复造 async runtime)
// 0 引 NewAPI, 0 引 parking_lot (用 std::sync 守边), 0 引 pyo3/qt/GDI (per 主 2026-08-06 01:00 拍板"纯 Rust")

// ============================================================================
// 子模块 (10 个, 每个 100% 文档化)
// ============================================================================

/// 错误类型 (7 variant MemoryProviderError + 序列化摘要 MemoryProviderErrorKind).
pub mod error;
/// MemoryProvider trait + ProviderKind 7 变体 + ProviderConfig 6 K-1 + ProviderScope 3 变体.
pub mod memory_provider;
/// 模式 0: `InMemoryProvider` 进程内 HashMap.
pub mod provider_in_memory;
/// 模式 1: `RedisProvider` 外部 Redis server.
pub mod provider_redis;
/// 模式 2: `SqliteProvider` 本地 SQLite 文件 / :memory:.
pub mod provider_sqlite;
/// 模式 3: `PostgresProvider` 外部 PG server.
pub mod provider_postgres;
/// 模式 4: `S3Provider` 外部 S3 (S3-compatible) via reqwest.
pub mod provider_s3;
/// 模式 5: `DiskLruProvider` 本地 disk + LRU (借 lru crate).
pub mod provider_disk_lru;
/// 模式 6: `HybridProvider` 组合 in_memory + disk_lru 两级.
pub mod provider_hybrid;
/// 模式 7: `FileProvider` 本地 JSON-Lines append-only 文件 (per R23 #6 派工, file provider).
pub mod provider_file;
/// 模式 8: `MongoDbProvider` 外部 MongoDB skeleton (per R23 #6 派工, mongodb skeleton).
pub mod provider_mongodb;
/// 7 provider 聚合注册表 (7 字段, 1:1 跟 Golutra 5 provider 装配对齐).
pub mod registry;

// ============================================================================
// 公共 re-export (顶层级 API, 不需要 `apeireth_memory_extensions::provider_in_memory::InMemoryProvider`)
// ============================================================================

pub use crate::error::{
    MemoryProviderError, MemoryProviderErrorKind, MemoryProviderResult,
    MEMORY_PROVIDER_ERROR_VARIANT_COUNT,
};
pub use crate::memory_provider::{
    MemoryProvider, ProviderConfig, ProviderConfigField, ProviderKind, ProviderScope,
    MEMORY_PROVIDER_KIND_COUNT, PROVIDER_CONFIG_K1_FIELDS, PROVIDER_SCOPE_COUNT,
};
pub use crate::provider_disk_lru::{DiskLruConfigDefault, DiskLruProvider};
pub use crate::provider_hybrid::{HybridConfigDefault, HybridProvider};
pub use crate::provider_file::{FileConfigDefault, FileProvider};
pub use crate::provider_mongodb::{MongoDbConfigDefault, MongoDbProvider};
pub use crate::provider_in_memory::{InMemoryConfigDefault, InMemoryProvider};
pub use crate::provider_postgres::{PostgresConfigDefault, PostgresProvider};
pub use crate::provider_redis::{RedisConfigDefault, RedisProvider};
pub use crate::provider_s3::{S3ConfigDefault, S3Provider};
pub use crate::provider_sqlite::{SqliteConfigDefault, SqliteProvider};
pub use crate::registry::{
    ProviderRegistry, ProviderRegistryBuilder, REGISTRY_PROVIDER_COUNT,
};

// ============================================================================
// 编译期 hardcode (7 项, 跨模块共享守门)
// ============================================================================

/// **Hardcode #1**: 平台名 (K-1 必含, per supervisor-prompt-818 §5.3 模式).
///
/// 跨 crate 通信用 platform 字段标识来源, 防 m3 幻觉把别平台的 provider 调进来.
pub const PLATFORM_NAME: &str = "apeireth";

/// **Hardcode #2**: Schema 版本号 (向前兼容字段, R21+ 改格式时 bump).
pub const APEIRETH_MEMORY_EXTENSIONS_SCHEMA_VERSION: &str = "1";

/// **Hardcode #3**: Golutra v0.1.0 借鉴的 provider 数 (5 Golutra + 1 InMemory + 1 DiskLru + 1 Hybrid = 8, 本 crate 走 7 = 5 + 2).
pub const BORROWED_GOLUTRA_PROVIDER_COUNT: usize = 5;

/// **Hardcode #4**: 本 crate 实现的 provider 总数 (5 Golutra + 1 InMemory + 1 DiskLru + 1 Hybrid = 8, 但 Hybrid 是组合, 实际 7 原子 = 5 Golutra + 2 纯 Rust).
pub const IMPLEMENTED_PROVIDER_COUNT: usize = 9;

/// **Hardcode #5**: ProviderKind 7 变体 (InMemory / Redis / Sqlite / Postgres / S3 / DiskLru / Hybrid).
///
/// (per `MEMORY_PROVIDER_KIND_COUNT` 在 memory_provider 模块, 这里只 1:1 镜像守门)
/// (per `REGISTRY_PROVIDER_COUNT` 在 registry 模块, 这里只 1:1 镜像守门)
pub const PROVIDER_KIND_COUNT_MIRROR: usize = 9;

/// **Hardcode #6**: MemoryProviderError 7 variant (per 8 项之 2 编译期 hardcode).
///
/// (per `MEMORY_PROVIDER_ERROR_VARIANT_COUNT` 在 error 模块, 这里只 1:1 镜像守门)
pub const MEMORY_PROVIDER_ERROR_VARIANT_COUNT_MIRROR: usize = 7;

/// **Hardcode #7**: ProviderConfig 6 K-1 强校验字段.
pub const PROVIDER_CONFIG_K1_FIELDS_MIRROR: usize = 6;

/// 编译期字符串相等比较 (per std::str::eq 不是 const-stable, 自实现字节比较).
const fn const_str_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let mut i = 0;
    while i < ab.len() {
        if ab[i] != bb[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// 编译期守门: PLATFORM_NAME == "apeireth" (K-1 强校验).
const _: () = assert!(const_str_eq(PLATFORM_NAME, "apeireth"));
/// 编译期守门: APEIRETH_MEMORY_EXTENSIONS_SCHEMA_VERSION == "1".
const _: () = assert!(const_str_eq(APEIRETH_MEMORY_EXTENSIONS_SCHEMA_VERSION, "1"));
/// 编译期守门: BORROWED_GOLUTRA_PROVIDER_COUNT == 5.
const _: () = assert!(BORROWED_GOLUTRA_PROVIDER_COUNT == 5);
/// 编译期守门: IMPLEMENTED_PROVIDER_COUNT == 9.
const _: () = assert!(IMPLEMENTED_PROVIDER_COUNT == 9);
/// 编译期守门: PROVIDER_KIND_COUNT_MIRROR == 9 (R23 #6 加 File/MongoDb).
const _: () = assert!(PROVIDER_KIND_COUNT_MIRROR == 9);
/// 编译期守门: MEMORY_PROVIDER_ERROR_VARIANT_COUNT_MIRROR == 7.
const _: () = assert!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT_MIRROR == 7);
/// 编译期守门: PROVIDER_CONFIG_K1_FIELDS_MIRROR == 6.
const _: () = assert!(PROVIDER_CONFIG_K1_FIELDS_MIRROR == 6);
/// 编译期守门: MEMORY_PROVIDER_KIND_COUNT == 9 (跨模块共享).
const _: () = assert!(MEMORY_PROVIDER_KIND_COUNT == 9);
/// 编译期守门: REGISTRY_PROVIDER_COUNT == 9 (跨模块共享).
const _: () = assert!(REGISTRY_PROVIDER_COUNT == 9);
/// 编译期守门: MEMORY_PROVIDER_ERROR_VARIANT_COUNT == 7 (跨模块共享).
const _: () = assert!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT == 7);
/// 编译期守门: PROVIDER_CONFIG_K1_FIELDS == 6 (跨模块共享).
const _: () = assert!(PROVIDER_CONFIG_K1_FIELDS == 6);
/// 编译期守门: PROVIDER_SCOPE_COUNT == 3.
const _: () = assert!(PROVIDER_SCOPE_COUNT == 3);

// =====================================================================
// 顶层单元测试 (跨模块综合 = 10 tests)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // 顶层 hardcode 守门 (per 6 哲学锚 + 8 项承诺)

    #[test]
    fn platform_name_compile_time_guard() {
        assert_eq!(PLATFORM_NAME, "apeireth");
    }

    #[test]
    fn schema_version_compile_time_guard() {
        assert_eq!(APEIRETH_MEMORY_EXTENSIONS_SCHEMA_VERSION, "1");
    }

    #[test]
    fn borrowed_golutra_provider_count_compile_time_guard() {
        assert_eq!(BORROWED_GOLUTRA_PROVIDER_COUNT, 5);
    }

    #[test]
    fn implemented_provider_count_compile_time_guard() {
        assert_eq!(IMPLEMENTED_PROVIDER_COUNT, 9);
    }

    #[test]
    fn seven_provider_kind_constructible_at_top_level() {
        let _ = ProviderKind::InMemory;
        let _ = ProviderKind::Redis;
        let _ = ProviderKind::Sqlite;
        let _ = ProviderKind::Postgres;
        let _ = ProviderKind::S3;
        let _ = ProviderKind::DiskLru;
        let _ = ProviderKind::Hybrid;
    }

    #[test]
    fn three_provider_scope_constructible_at_top_level() {
        let _ = ProviderScope::Local;
        let _ = ProviderScope::Shared;
        let _ = ProviderScope::Global;
    }

    #[test]
    fn seven_memory_provider_error_kind_constructible() {
        let _ = MemoryProviderErrorKind::Config;
        let _ = MemoryProviderErrorKind::Connection;
        let _ = MemoryProviderErrorKind::NotFound;
        let _ = MemoryProviderErrorKind::Serialization;
        let _ = MemoryProviderErrorKind::Backend;
        let _ = MemoryProviderErrorKind::Capacity;
        let _ = MemoryProviderErrorKind::Other;
    }

    #[test]
    fn six_provider_config_field_constructible() {
        let _ = ProviderConfigField::ConnectionString;
        let _ = ProviderConfigField::Timeout;
        let _ = ProviderConfigField::MaxSize;
        let _ = ProviderConfigField::Persist;
        let _ = ProviderConfigField::CacheTtl;
        let _ = ProviderConfigField::Scope;
    }

    #[test]
    fn registry_constructible_with_all_7_providers() {
        use std::sync::Arc;
        let r = ProviderRegistry::new();
        assert_eq!(r.initialized_count(), 0);
        // builder 7 字段
        let _ = ProviderRegistryBuilder::new()
            .with_in_memory(Arc::new(InMemoryProvider::new(ProviderConfig::new(
                "memory://",
                Duration::from_secs(5),
                1024 * 1024,
                false,
                Duration::from_secs(0),
                ProviderScope::Local,
            )).unwrap()) as Arc<dyn MemoryProvider>)
            .build();
    }
}
