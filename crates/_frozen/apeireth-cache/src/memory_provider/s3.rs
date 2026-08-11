//! # S3Provider — R21 stub
//!
//! 1:1 翻译 Golutra `evermind` 云端备份 商业版 (object storage memory gateway).
//!
//! ## 状态: ❌ R21 stub (per task spec)
//!
//! 全部方法返 `BackendNotImplemented`. 真接计划 (R21):
//! - 加 `aws-sdk-s3 = "1.x"` + `aws-config = "1.x"` workspace dep
//! - 用 `aws_sdk_s3::Client::new(&config)` (凭证 + region 来自 env / IAM role)
//! - 存储约定: `mem:{id}.json` (key) + 整个 MemoryEntry JSON serialize
//! - capture → `PutObject` (ContentType: application/json)
//! - query → `ListObjectsV2` + `GetObject` (全量拉, 内存过滤, 或加 metadata index)
//! - clear → `DeleteObject` / `DeleteObjects` (batch)
//!
//! ## K-1 强校验
//!
//! 全部方法守门 `ProviderKind::S3.check_implemented()`, 不允许"装作能写 S3".
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use async_trait::async_trait;

use crate::memory_provider::error::{MemoryProviderError, MemoryProviderResult};
use crate::memory_provider::provider_kind::ProviderKind;
use crate::memory_provider::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 S3Provider 结构
// ============================================================================

/// S3 object-storage memory provider (R21 stub).
///
/// 全部方法返 `BackendNotImplemented` (per task spec). 留 R21 续接.
#[derive(Debug, Clone, Default)]
pub struct S3Provider {
    /// 占位 (R21 真接时存 `aws_sdk_s3::Client` + bucket name).
    _placeholder: (),
}

impl S3Provider {
    /// 构造 stub.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryProvider for S3Provider {
    async fn capture(&self, _entry: MemoryEntry) -> MemoryProviderResult<String> {
        // TODO R21: 接 aws-sdk-s3, PutObject key=mem:{id}.json body=entry JSON
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::S3.as_str().to_string(),
        ))
    }

    async fn query(&self, _q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        // TODO R21: ListObjectsV2 + GetObject (batch)
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::S3.as_str().to_string(),
        ))
    }

    async fn clear(&self, _id: Option<&str>) -> MemoryProviderResult<()> {
        // TODO R21: DeleteObject / DeleteObjects (batch)
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::S3.as_str().to_string(),
        ))
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::S3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门: R21 stub — 全部方法返 BackendNotImplemented.
    #[tokio::test]
    async fn all_methods_return_not_implemented() {
        let p = S3Provider::new();
        assert!(!p.is_implemented());
        assert_eq!(p.kind(), ProviderKind::S3);

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
