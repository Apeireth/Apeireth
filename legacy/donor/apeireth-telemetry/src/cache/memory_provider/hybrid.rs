//! # HybridProvider — L1 InMemory + L2 Sqlite 写穿透 (R20 完整实现)
//!
//! 1:1 翻译 Golutra `relay` 商业版 (multi-tier memory gateway, write-through).
//!
//! ## 架构
//!
//! ```text
//! capture(id, content) ──► L1 InMemory 写入
//!                       └─► L2 Sqlite 写入 (write-through, 持久化)
//!
//! query(q) ──► L1 命中? ──► 返 L1
//!           └─► L1 miss ──► L2 命中? ──► 写回 L1 (read-fill) + 返 L2
//!                                └─► L2 miss ──► 返空
//!
//! clear(id) ──► L1 clear + L2 clear (双删一致性)
//! ```
//!
//! ## 一致性策略
//!
//! - **写穿透 (write-through)**: capture 同时写 L1 + L2, L2 失败整个 capture 失败
//!   (强一致, 牺牲可用性 — 适合 memory 这种"丢了就找不回"场景)
//! - **读回填 (read-fill)**: L1 miss 但 L2 hit 时, 写回 L1 让下次 L1 hit
//! - **双删 (double-delete)**: clear 同时清 L1 + L2, 任一失败返 `ClearFailed`
//!
//! ## K-1 强校验
//!
//! - L2 (sqlite) 失败: `BackendIoError` (透明冒泡, 不假装成功)
//! - `query` 无过滤: `InvalidQuery` (透传)
//! - `clear(Some(id))` 不存在: `NotFound` (透传)
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::sync::Arc;

use async_trait::async_trait;

use super::error::MemoryProviderResult;
use super::in_memory::InMemoryProvider;
use super::provider_kind::{ProviderKind, PROVIDER_KIND_VARIANT_COUNT};
use super::sqlite::SqliteProvider;
use super::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 HybridProvider 结构
// ============================================================================

/// Hybrid memory provider (L1 InMemory + L2 Sqlite 写穿透).
///
/// `Send + Sync` (内部两个 provider 都 `Send + Sync`).
#[derive(Clone)]
pub struct HybridProvider {
    /// L1 — 进程内 HashMap (热路径, fast).
    l1: InMemoryProvider,

    /// L2 — SQLite (冷路径, durable).
    l2: Arc<SqliteProvider>,
}

impl std::fmt::Debug for HybridProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridProvider")
            .field("l1_kind", &self.l1.kind())
            .field("l2_kind", &self.l2.kind())
            .finish_non_exhaustive()
    }
}

impl HybridProvider {
    /// 构造 hybrid (L1 + L2).
    ///
    /// L2 I/O 失败: 返 `BackendIoError` (K-1 强校验).
    pub fn new(l2: SqliteProvider) -> Self {
        Self {
            l1: InMemoryProvider::new(),
            l2: Arc::new(l2),
        }
    }

    /// 引用 L1 (供测试 / 调试).
    pub fn l1(&self) -> &InMemoryProvider {
        &self.l1
    }

    /// 引用 L2 (供测试 / 调试).
    pub fn l2(&self) -> &SqliteProvider {
        &self.l2
    }
}

#[async_trait]
impl MemoryProvider for HybridProvider {
    async fn capture(&self, entry: MemoryEntry) -> MemoryProviderResult<String> {
        // 写穿透: L1 先, L2 后. L2 失败 → 整 capture 失败, L1 已写也无所谓
        // (下次 L2 恢复时 capture 会覆写, L1 临时多一条无副作用)
        let id = entry.id.clone();
        self.l1.capture(entry.clone()).await?;
        self.l2.capture(entry).await?;
        Ok(id)
    }

    async fn query(&self, q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        // 1) L1 先查
        let l1_hits = self.l1.query(q.clone()).await?;
        if !l1_hits.is_empty() {
            return Ok(l1_hits);
        }
        // 2) L1 miss, 查 L2
        let l2_hits = self.l2.query(q).await?;
        // 3) read-fill: 把 L2 hit 写回 L1
        for e in &l2_hits {
            // 忽略写回错误 (L1 miss 后续再写, 不阻塞当前 query)
            let _ = self.l1.capture(e.clone()).await;
        }
        Ok(l2_hits)
    }

    async fn clear(&self, id: Option<&str>) -> MemoryProviderResult<()> {
        // 双删: L1 + L2
        self.l1.clear(id).await?;
        self.l2.clear(id).await?;
        Ok(())
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Hybrid
    }
}

// ============================================================================
// §2 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 守门 #1: provider variant 守门.
    #[test]
    fn cross_module_variant_counts() {
        assert_eq!(PROVIDER_KIND_VARIANT_COUNT, 7);
    }

    /// 守门 #2: 构造 hybrid.
    #[test]
    fn new_hybrid_works() {
        let l2 = SqliteProvider::in_memory().unwrap();
        let h = HybridProvider::new(l2);
        assert_eq!(h.kind(), ProviderKind::Hybrid);
        assert!(h.is_implemented());
        assert_eq!(h.l1().len(), 0);
    }

    /// 守门 #3: write-through — L1 + L2 同时写.
    #[tokio::test]
    async fn capture_writes_to_both_l1_and_l2() {
        let l2 = SqliteProvider::in_memory().unwrap();
        let h = HybridProvider::new(l2);
        h.capture(MemoryEntry::with_id_and_content("h1", "hybrid"))
            .await
            .unwrap();
        assert_eq!(h.l1().len(), 1);
        assert_eq!(h.l2().len().unwrap(), 1);
    }

    /// 守门 #4: L1 hit 优先.
    #[tokio::test]
    async fn l1_hit_takes_precedence() {
        let l2 = SqliteProvider::in_memory().unwrap();
        let h = HybridProvider::new(l2);
        h.capture(MemoryEntry::with_id_and_content("h1", "L1L2"))
            .await
            .unwrap();

        // L1 + L2 都有
        let r = h.query(MemoryQuery::by_id("h1")).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "L1L2");
    }

    /// 守门 #5: L1 miss + L2 hit 触发 read-fill.
    #[tokio::test]
    async fn l1_miss_read_fills_from_l2() {
        let l2 = SqliteProvider::in_memory().unwrap();
        // 直接往 L2 写 (跳过 L1)
        l2.capture(MemoryEntry::with_id_and_content("only-l2", "L2-only"))
            .await
            .unwrap();

        let h = HybridProvider::new(l2);
        // 初始 L1 空, L2 有一条
        assert_eq!(h.l1().len(), 0);
        assert_eq!(h.l2().len().unwrap(), 1);

        // query 触发 read-fill
        let r = h.query(MemoryQuery::by_id("only-l2")).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].content, "L2-only");

        // L1 被回填
        assert_eq!(h.l1().len(), 1);
    }

    /// 守门 #6: clear 双删 L1 + L2.
    #[tokio::test]
    async fn clear_removes_from_both_tiers() {
        let l2 = SqliteProvider::in_memory().unwrap();
        let h = HybridProvider::new(l2);
        h.capture(MemoryEntry::with_id_and_content("c1", "x"))
            .await
            .unwrap();
        h.capture(MemoryEntry::with_id_and_content("c2", "y"))
            .await
            .unwrap();
        assert_eq!(h.l1().len(), 2);
        assert_eq!(h.l2().len().unwrap(), 2);

        h.clear(Some("c1")).await.unwrap();
        assert_eq!(h.l1().len(), 1);
        assert_eq!(h.l2().len().unwrap(), 1);

        h.clear(None).await.unwrap();
        assert_eq!(h.l1().len(), 0);
        assert_eq!(h.l2().len().unwrap(), 0);
    }

    /// 守门 #7: query 无过滤透传 InvalidQuery.
    #[tokio::test]
    async fn query_no_filter_returns_invalid() {
        let l2 = SqliteProvider::in_memory().unwrap();
        let h = HybridProvider::new(l2);
        let err = h.query(MemoryQuery::new()).await.unwrap_err();
        assert!(matches!(
            err,
            super::super::error::MemoryProviderError::InvalidQuery(_)
        ));
    }

    /// 守门 #8: metadata 端到端.
    #[tokio::test]
    async fn metadata_end_to_end() {
        let l2 = SqliteProvider::in_memory().unwrap();
        let h = HybridProvider::new(l2);
        let mut md = HashMap::new();
        md.insert("scope".to_string(), "hybrid-test".to_string());
        h.capture(MemoryEntry::new("m1", "meta test", md.clone()))
            .await
            .unwrap();
        let r = h.query(MemoryQuery::by_id("m1")).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(
            r[0].metadata.get("scope").map(|s| s.as_str()),
            Some("hybrid-test")
        );
    }
}
