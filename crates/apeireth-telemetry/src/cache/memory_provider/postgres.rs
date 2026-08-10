//! # PostgresProvider — R21 stub
//!
//! 1:1 翻译 Golutra `vector` / `mem0` 后端 server 商业版 (network SQL).
//!
//! ## 状态: ❌ R21 stub (per task spec)
//!
//! 全部方法返 `BackendNotImplemented`. 真接计划 (R21):
//! - 加 `sqlx = { version = "0.8", features = ["postgres", "runtime-tokio-rustls"] }` workspace dep
//! - 用 `sqlx::PgPool::connect(url)` + migration (跟 SqliteProvider schema 1:1 镜像)
//! - capture → `INSERT ... ON CONFLICT (id) DO UPDATE` (PostgreSQL upsert)
//! - query → `SELECT ... WHERE ... ORDER BY created_at DESC LIMIT`
//! - clear → `DELETE FROM memory_entries WHERE id = $1`
//!
//! ## K-1 强校验
//!
//! 全部方法守门 `ProviderKind::Postgres.check_implemented()`, 不允许"装作能写 Postgres".
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use async_trait::async_trait;

use super::error::{MemoryProviderError, MemoryProviderResult};
use super::provider_kind::ProviderKind;
use super::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 PostgresProvider 结构
// ============================================================================

/// PostgreSQL memory provider (R21 stub).
///
/// 全部方法返 `BackendNotImplemented` (per task spec). 留 R21 续接.
#[derive(Debug, Clone, Default)]
pub struct PostgresProvider {
    /// 占位 (R21 真接时存 `sqlx::PgPool`).
    _placeholder: (),
}

impl PostgresProvider {
    /// 构造 stub.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryProvider for PostgresProvider {
    async fn capture(&self, _entry: MemoryEntry) -> MemoryProviderResult<String> {
        // TODO R21: 接 sqlx + PgPool, 用 ON CONFLICT (id) DO UPDATE
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Postgres.as_str().to_string(),
        ))
    }

    async fn query(&self, _q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        // TODO R21: SELECT ... WHERE ... ORDER BY created_at DESC LIMIT
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Postgres.as_str().to_string(),
        ))
    }

    async fn clear(&self, _id: Option<&str>) -> MemoryProviderResult<()> {
        // TODO R21: DELETE FROM memory_entries WHERE id = $1
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Postgres.as_str().to_string(),
        ))
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Postgres
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门: R21 stub — 全部方法返 BackendNotImplemented.
    #[tokio::test]
    async fn all_methods_return_not_implemented() {
        let p = PostgresProvider::new();
        assert!(!p.is_implemented());
        assert_eq!(p.kind(), ProviderKind::Postgres);

        let err = p
            .capture(MemoryEntry::with_id_and_content("a", "x"))
            .await
            .unwrap_err();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));

        let err = p.query(MemoryQuery::by_id("a")).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));

        let err = p.clear(None).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::BackendNotImplemented(_)));
    }
}
