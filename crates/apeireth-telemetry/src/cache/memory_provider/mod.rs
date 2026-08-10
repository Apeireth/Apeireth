//! # memory_provider — 7-mode memory provider (借鉴 Golutra memory gateway)
//!
//! 1:1 翻译 Golutra 公开仓库 memory gateway 7 provider 商业版 (per
//! `analysis/golutra/BORROW_FROM_GOLUTRA.md` §3 借鉴 + §8 P1 优先级矩阵).
//!
//! ## 背景
//!
//! R20 战役: 主人 2026-08-05 01:00 拍板"借 Golutra memory gateway 7 provider 抽象",
//! 但要走 Apeireth 哲学骨架 (5 项不假装 / 双洋葱 / 7 器官主权), 不抄 Golutra 闭源 40%.
//! 本模块是 R20 阶段 6 skeleton 后续: 给现有 4 BackendKind (Memory/Disk/Redis/Memcached)
//! 之上, 加 7 ProviderKind (InMemory/Redis/Sqlite/Postgres/S3/DiskLru/Hybrid) 维度的
//! memory provider 抽象, 让 `apeireth-memory` / `apeireth-cognition` 等 7 器官可灵活选择
//! memory 后端.
//!
//! ## 7 Provider (1:1 翻译 Golutra)
//!
//! | ProviderKind | 实现度 | Golutra 对位          | 真实依赖                |
//! |--------------|--------|----------------------|-------------------------|
//! | `InMemory`   | ✅ 完整 | `local`              | 无 (HashMap + parking_lot RwLock) |
//! | `Sqlite`     | ✅ 完整 | `local_sqlite`       | workspace `rusqlite = "0.32"` |
//! | `Hybrid`     | ✅ 完整 | `relay` (多后端组合)  | InMemory + Sqlite (write-through + read-fill) |
//! | `Redis`      | ❌ R21 stub | `redis`         | (留 R21)                |
//! | `Postgres`   | ❌ R21 stub | `vector` / `mem0` server | (留 R21)         |
//! | `S3`         | ❌ R21 stub | `evermind` 云端备份  | (留 R21)                |
//! | `DiskLru`    | ❌ R21 stub | `disk` 落盘         | (留 R21, 复用 crate 内 `lru` 0.12) |
//!
//! ## 3 流 (capture / query / clear)
//!
//! 借鉴 Golutra memory gateway 商业版 3 流设计:
//! - `capture(entry)` → 写一条 (id + content + metadata + created_at_secs)
//! - `query(q)` → 按 (id / content_contains) + limit 读
//! - `clear(id)` → 删单条 (id = Some) 或 全清 (id = None)
//!
//! ## 哲学适配 (per APEIRETH-CONVENTIONS §9)
//!
//! - **不假装已实现 (O-5)**: 4 stub 显式标 `TODO R21`, 全部方法返 `BackendNotImplemented`,
//!   不允许"装作能写 Redis/Postgres/S3/DiskLru"
//! - **走在前人肩上 (O-2)**: 借 Golutra 7 provider 抽象 + workspace `rusqlite 0.32` 硬锁
//! - **中央 AI 主权**: 7 provider 是"工具外圈", 不动摇 7 器官主权 (apeireth-memory 等
//!   内部可以选哪个 provider, 但不影响主 AI 决策)
//! - **不假装灵魂同一 (S-1)**: 7 provider 是不同后端, 不假装"它们是同一个 memory 的多视角",
//!   每个 provider 独立 identity
//!
//! ## 8 项不修改承诺 (per APEIRETH-CONVENTIONS §10)
//!
//! 1. **阶段 1+2+3 LOCKED** — 不动 ✅
//! 2. **v2 / v4 / v4.1 LOCKED** — 不动 ✅
//! 3. **24 LOCKED crate** — 不触碰 (本模块在 `apeireth-cache` 内, 该 crate 不在 LOCKED list) ✅
//! 4. **workspace version 1.0.0** — 不动 ✅
//! 5. **v6 修正** (4 重守门 + 权限发放 + E 层修改路径) — 不动 ✅
//! 6. **R11 baseline 三值** — 不动 ✅
//! 7. **v1 → v5 历史链** — 不删除 ✅
//! 8. **Goroutra 7 模式 1:1 翻译** — 7 variant 编译期 hardcode, 不加不减 ✅
//!
//! ## 公开 API 一览
//!
//! - [`MemoryProvider`] trait — 3 流 + 2 元 (kind / is_implemented)
//! - [`MemoryEntry`] struct — id + content + metadata + created_at_secs
//! - [`MemoryQuery`] struct — id (Optional) + content_contains (Optional) + limit (Optional)
//! - [`ProviderKind`] enum — 7 variant 1:1 翻译
//! - [`MemoryProviderError`] enum — 8 variant
//! - 3 真接 provider: [`InMemoryProvider`], [`SqliteProvider`], [`HybridProvider`]
//! - 4 stub provider: [`RedisProvider`], [`PostgresProvider`], [`S3Provider`], [`DiskLruProvider`]
//! - [`build_provider`] fn — 按 ProviderKind 分发构造 (3 真接 / 4 返 Err)
//!
//! ## 用法示例
//!
//! ```ignore
//! use apeireth_cache::memory_provider::{build_provider, MemoryEntry, MemoryQuery, ProviderKind};
//!
//! // 1) 选 provider (InMemory / Sqlite / Hybrid 真接, 其他 R21 stub)
//! let provider = build_provider(ProviderKind::InMemory, None).unwrap();
//!
//! // 2) capture (写一条)
//! let id = provider.capture(MemoryEntry::with_id_and_content("u1", "hello")).await.unwrap();
//!
//! // 3) query (按 id 精确)
//! let entries = provider.query(MemoryQuery::by_id("u1")).await.unwrap();
//! assert_eq!(entries.len(), 1);
//!
//! // 4) clear (删一条)
//! provider.clear(Some("u1")).await.unwrap();
//! ```

#![allow(clippy::module_inception)]

// ============================================================================
// §1 子模块声明
// ============================================================================

mod disk_lru;
pub mod error;
mod hybrid;
mod in_memory;
mod postgres;
mod provider_kind;
mod redis;
mod s3;
mod sqlite;
pub mod traits;

// ============================================================================
// §2 Re-export (主入口便捷)
// ============================================================================

pub use disk_lru::DiskLruProvider;
pub use error::{MemoryProviderError, MemoryProviderResult, MEMORY_PROVIDER_ERROR_VARIANT_COUNT};
pub use hybrid::HybridProvider;
pub use in_memory::InMemoryProvider;
pub use postgres::PostgresProvider;
pub use provider_kind::{ProviderKind, PROVIDER_KIND_VARIANT_COUNT};
pub use redis::RedisProvider;
pub use s3::S3Provider;
pub use sqlite::SqliteProvider;
pub use traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §3 build_provider factory (按 ProviderKind 分发)
// ============================================================================

/// 构造 memory provider (按 ProviderKind 分发).
///
/// - `InMemory` / `Sqlite` / `Hybrid`: 返真接 provider
/// - `Redis` / `Postgres` / `S3` / `DiskLru`: 返 `Err(BackendNotImplemented)` (R21 续接)
///
/// `sqlite_path`:
/// - `Some(path)`: Sqlite / Hybrid 打开该文件 (不存在则创建)
/// - `None`: Sqlite / Hybrid 走 `:memory:` (测试用, 不落盘)
/// - 其他 provider: 忽略
pub fn build_provider(
    kind: ProviderKind,
    sqlite_path: Option<&str>,
) -> MemoryProviderResult<Box<dyn MemoryProvider>> {
    match kind {
        ProviderKind::InMemory => Ok(Box::new(InMemoryProvider::new())),
        ProviderKind::Sqlite => {
            let p = match sqlite_path {
                Some(path) => SqliteProvider::open(path)?,
                None => SqliteProvider::in_memory()?,
            };
            Ok(Box::new(p))
        }
        ProviderKind::Hybrid => {
            let l2 = match sqlite_path {
                Some(path) => SqliteProvider::open(path)?,
                None => SqliteProvider::in_memory()?,
            };
            Ok(Box::new(HybridProvider::new(l2)))
        }
        ProviderKind::Redis => Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Redis.as_str().to_string(),
        )),
        ProviderKind::Postgres => Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Postgres.as_str().to_string(),
        )),
        ProviderKind::S3 => Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::S3.as_str().to_string(),
        )),
        ProviderKind::DiskLru => Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::DiskLru.as_str().to_string(),
        )),
    }
}

// ============================================================================
// §4 in-module 测试 (factory 守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 7 variant 编译期 hardcode.
    #[test]
    fn provider_kind_seven_variants() {
        assert_eq!(PROVIDER_KIND_VARIANT_COUNT, 7);
        assert_eq!(ProviderKind::ALL.len(), 7);
    }

    /// 守门 #2: 8 error variant 编译期 hardcode.
    #[test]
    fn error_eight_variants() {
        assert_eq!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT, 8);
    }

    /// 守门 #3: build_provider — 3 真接都 OK, 4 stub 全部返 BackendNotImplemented.
    #[tokio::test]
    async fn build_provider_dispatch() {
        // 3 真接
        let p1 = build_provider(ProviderKind::InMemory, None).unwrap();
        assert_eq!(p1.kind(), ProviderKind::InMemory);
        let p2 = build_provider(ProviderKind::Sqlite, None).unwrap();
        assert_eq!(p2.kind(), ProviderKind::Sqlite);
        let p3 = build_provider(ProviderKind::Hybrid, None).unwrap();
        assert_eq!(p3.kind(), ProviderKind::Hybrid);

        // 4 stub 全部返错
        for stub in [
            ProviderKind::Redis,
            ProviderKind::Postgres,
            ProviderKind::S3,
            ProviderKind::DiskLru,
        ] {
            let result = build_provider(stub, None);
            assert!(result.is_err(), "stub {stub:?} should return Err");
            let err = result.err().unwrap();
            assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
        }
    }

    /// 守门 #4: build_provider with sqlite_path 落盘 + reopen.
    #[tokio::test]
    async fn build_provider_sqlite_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mem.db");
        let path_str = path.to_str().unwrap();

        // 第一次: 写
        let p1 = build_provider(ProviderKind::Sqlite, Some(path_str)).unwrap();
        p1.capture(MemoryEntry::with_id_and_content("p1", "persist"))
            .await
            .unwrap();
        assert_eq!(p1.query(MemoryQuery::by_id("p1")).await.unwrap().len(), 1);

        // 第二次: 重新打开, 应该读得到
        let p2 = build_provider(ProviderKind::Sqlite, Some(path_str)).unwrap();
        assert_eq!(p2.query(MemoryQuery::by_id("p1")).await.unwrap().len(), 1);
    }
}
