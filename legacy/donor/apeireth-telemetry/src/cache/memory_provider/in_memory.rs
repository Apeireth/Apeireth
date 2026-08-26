//! # InMemoryProvider — 进程内 HashMap (R20 完整实现)
//!
//! 1:1 翻译 Golutra `local` 商业版 (in-memory memory provider).
//!
//! ## 实现
//!
//! - `Arc<parking_lot::RwLock<HashMap<String, MemoryEntry>>>` — 单进程多读少写
//! - `capture` — HashMap 插入 / 覆写
//! - `query` — 过滤 (id 精确 / content 子串) + 按 created_at_secs 倒序 + limit 截断
//! - `clear` — `Some(id)` 删单条 / `None` 全清
//!
//! ## K-1 强校验
//!
//! - `query` 无过滤条件: 返 `Err(InvalidQuery)` (防全表扫)
//! - `clear(Some(id))` id 不存在: 返 `Err(NotFound)`
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use super::error::{
    MemoryProviderError, MemoryProviderResult, MEMORY_PROVIDER_ERROR_VARIANT_COUNT,
};
use super::provider_kind::{ProviderKind, PROVIDER_KIND_VARIANT_COUNT};
use super::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 InMemoryProvider 结构
// ============================================================================

/// 进程内 memory provider (R20 完整实现).
///
/// `Send + Sync` (Arc + RwLock).
#[derive(Debug, Clone)]
pub struct InMemoryProvider {
    /// 内存存储.
    inner: Arc<RwLock<HashMap<String, MemoryEntry>>>,
}

impl InMemoryProvider {
    /// 构造空 provider.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 当前 entry 数 (sync, 锁内读).
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// 是否空.
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }
}

impl Default for InMemoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryProvider for InMemoryProvider {
    async fn capture(&self, entry: MemoryEntry) -> MemoryProviderResult<String> {
        let id = entry.id.clone();
        self.inner.write().insert(entry.id.clone(), entry);
        Ok(id)
    }

    async fn query(&self, q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        // K-1 强校验: 至少给一个过滤条件
        if !q.has_any_filter() {
            return Err(MemoryProviderError::InvalidQuery(
                "at least one of `id` or `content_contains` must be set".to_string(),
            ));
        }
        let limit = q.effective_limit();
        let guard = self.inner.read();

        // 1) 过滤
        let mut hits: Vec<MemoryEntry> = guard
            .values()
            .filter(|e| match &q.id {
                Some(id) => &e.id == id,
                None => true,
            })
            .filter(|e| match &q.content_contains {
                Some(needle) => e.content.contains(needle.as_str()),
                None => true,
            })
            .cloned()
            .collect();

        // 2) 按 created_at_secs 倒序 (新 → 旧)
        hits.sort_by(|a, b| b.created_at_secs.cmp(&a.created_at_secs));

        // 3) limit 截断
        hits.truncate(limit);

        Ok(hits)
    }

    async fn clear(&self, id: Option<&str>) -> MemoryProviderResult<()> {
        match id {
            Some(id) => {
                let mut guard = self.inner.write();
                match guard.remove(id) {
                    Some(_) => Ok(()),
                    None => Err(MemoryProviderError::NotFound(id.to_string())),
                }
            }
            None => {
                self.inner.write().clear();
                Ok(())
            }
        }
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::InMemory
    }
}

// ============================================================================
// §2 in-module 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门 #1: 8 error variant + 7 provider variant (跨子模块守门).
    #[test]
    fn cross_module_variant_counts() {
        assert_eq!(MEMORY_PROVIDER_ERROR_VARIANT_COUNT, 8);
        assert_eq!(PROVIDER_KIND_VARIANT_COUNT, 7);
    }

    /// 守门 #2: 默认构造空.
    #[test]
    fn new_is_empty() {
        let p = InMemoryProvider::new();
        assert!(p.is_empty());
        assert_eq!(p.len(), 0);
        assert_eq!(p.kind(), ProviderKind::InMemory);
        assert!(p.is_implemented());
    }

    /// 守门 #3: capture 后 len = 1.
    #[tokio::test]
    async fn capture_increments_len() {
        let p = InMemoryProvider::new();
        let id = p
            .capture(MemoryEntry::with_id_and_content("a", "hello"))
            .await
            .unwrap();
        assert_eq!(id, "a");
        assert_eq!(p.len(), 1);
    }

    /// 守门 #4: query 无过滤返 InvalidQuery.
    #[tokio::test]
    async fn query_no_filter_returns_invalid() {
        let p = InMemoryProvider::new();
        let err = p.query(MemoryQuery::new()).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::InvalidQuery(_)));
    }

    /// 守门 #5: clear 单条 id 不存在返 NotFound.
    #[tokio::test]
    async fn clear_unknown_id_returns_not_found() {
        let p = InMemoryProvider::new();
        let err = p.clear(Some("nope")).await.unwrap_err();
        assert!(matches!(err, MemoryProviderError::NotFound(_)));
    }
}
