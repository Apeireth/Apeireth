//! # DiskLruProvider — R21 stub
//!
//! 1:1 翻译 Golutra `disk` 落盘 商业版 (file-based LRU memory gateway).
//!
//! ## 状态: ❌ R21 stub (per task spec)
//!
//! 全部方法返 `BackendNotImplemented`. 真接计划 (R21):
//! - 复用 crate 内 `lru` 0.12 (已 deps) 做容量守门
//! - 文件: `<base_dir>/mem/{id}.json` (per-entry 落盘, fs_err 守护)
//! - index: `<base_dir>/mem.idx` (id → offset + size, 启动时 rebuild)
//! - capture → write entry + append index
//! - query → load index + read file + parse JSON + 过滤
//! - clear → unlink file + remove index
//!
//! ## K-1 强校验
//!
//! 全部方法守门 `ProviderKind::DiskLru.check_implemented()`, 不允许"装作能写磁盘 LRU".
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use async_trait::async_trait;

use super::error::{MemoryProviderError, MemoryProviderResult};
use super::provider_kind::ProviderKind;
use super::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 DiskLruProvider 结构
// ============================================================================

/// Disk LRU file-based memory provider (R21 stub).
///
/// 全部方法返 `BackendNotImplemented` (per task spec). 留 R21 续接.
#[derive(Debug, Clone, Default)]
pub struct DiskLruProvider {
    /// 占位 (R21 真接时存 base_dir + Lru index).
    _placeholder: (),
}

impl DiskLruProvider {
    /// 构造 stub.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryProvider for DiskLruProvider {
    async fn capture(&self, _entry: MemoryEntry) -> MemoryProviderResult<String> {
        // TODO R21: 复用 lru 0.12 + 文件 <base>/mem/{id}.json + index 文件
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::DiskLru.as_str().to_string(),
        ))
    }

    async fn query(&self, _q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        // TODO R21: load idx + read file + parse
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::DiskLru.as_str().to_string(),
        ))
    }

    async fn clear(&self, _id: Option<&str>) -> MemoryProviderResult<()> {
        // TODO R21: unlink file + remove index
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::DiskLru.as_str().to_string(),
        ))
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::DiskLru
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门: R21 stub — 全部方法返 BackendNotImplemented.
    #[tokio::test]
    async fn all_methods_return_not_implemented() {
        let p = DiskLruProvider::new();
        assert!(!p.is_implemented());
        assert_eq!(p.kind(), ProviderKind::DiskLru);

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
