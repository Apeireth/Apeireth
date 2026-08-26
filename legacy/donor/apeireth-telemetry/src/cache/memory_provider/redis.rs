//! # RedisProvider — R21 stub
//!
//! 1:1 翻译 Golutra `redis` 商业版 (network memory gateway).
//!
//! ## 状态: ❌ R21 stub (per task spec)
//!
//! 全部方法返 `BackendNotImplemented`. 真接计划 (R21):
//! - 加 `redis = { version = "0.27", features = ["tokio-comp"] }` workspace dep
//! - 用 `redis::Client::open(url)` + `redis::aio::ConnectionManager`
//! - capture → `HSET mem:{id} content <s> meta <json> ts <i>`
//! - query → `HGETALL` / `SCAN` + 内存过滤 (或 Lua 脚本)
//! - clear → `DEL mem:{id}` / `FLUSHDB` (单 prefix)
//!
//! ## K-1 强校验
//!
//! 全部方法守门 `ProviderKind::Redis.check_implemented()`, 不允许"装作能写 Redis".
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use async_trait::async_trait;

use super::error::{MemoryProviderError, MemoryProviderResult};
use super::provider_kind::ProviderKind;
use super::traits::{MemoryEntry, MemoryProvider, MemoryQuery};

// ============================================================================
// §1 RedisProvider 结构
// ============================================================================

/// Redis memory provider (R21 stub).
///
/// 全部方法返 `BackendNotImplemented` (per task spec). 留 R21 续接.
#[derive(Debug, Clone, Default)]
pub struct RedisProvider {
    /// 占位 (R21 真接时存 `redis::Client`).
    _placeholder: (),
}

impl RedisProvider {
    /// 构造 stub.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryProvider for RedisProvider {
    async fn capture(&self, _entry: MemoryEntry) -> MemoryProviderResult<String> {
        // TODO R21: 接 redis crate, 用 HSET 写 mem:{id}
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Redis.as_str().to_string(),
        ))
    }

    async fn query(&self, _q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>> {
        // TODO R21: HGETALL / SCAN
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Redis.as_str().to_string(),
        ))
    }

    async fn clear(&self, _id: Option<&str>) -> MemoryProviderResult<()> {
        // TODO R21: DEL mem:{id} / FLUSHDB
        Err(MemoryProviderError::BackendNotImplemented(
            ProviderKind::Redis.as_str().to_string(),
        ))
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Redis
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 守门: R21 stub — 全部方法返 BackendNotImplemented.
    #[tokio::test]
    async fn all_methods_return_not_implemented() {
        let p = RedisProvider::new();
        assert!(!p.is_implemented());
        assert_eq!(p.kind(), ProviderKind::Redis);

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
