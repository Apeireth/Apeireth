//! # MemoryProvider trait + MemoryEntry + MemoryQuery — 3 流 (capture / query / clear)
//!
//! 借鉴 Golutra memory gateway 3 流设计 (per `analysis/golutra/BORROW_FROM_GOLUTRA.md` §3):
//! - **capture** — 写 (id, content, metadata)
//! - **query** — 读 (id / content 模糊 / limit)
//! - **clear** — 删 (id 单删 / all 全删)
//!
//! 3 流全 async + Send + Sync, 跨任务 / 跨线程共享.
//!
//! ## 1:1 借鉴 Golutra 映射
//!
//! | apeireth-cache memory_provider | Golutra memory gateway 商业版        |
//! |--------------------------------|---------------------------------------|
//! | `MemoryProvider::capture`      | `MemoryProvider.capture(id, content)` |
//! | `MemoryProvider::query`        | `MemoryProvider.query(query)`         |
//! | `MemoryProvider::clear`        | `MemoryProvider.clear(id)`            |
//!
//! ## 6 哲学 anchor + 8 项不修改承诺
//!
//! S-1 北极星 / S-2 实事求是 / O-2 走在前人肩上 / O-3 干到底 / O-4 任何人都能接手 / O-5 不假装.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::error::MemoryProviderResult;
use super::provider_kind::ProviderKind;

// ============================================================================
// §1 MemoryEntry — 一条 memory 记录 (id + content + metadata + created_at_secs)
// ============================================================================

/// 一条 memory 记录.
///
/// 借鉴 Golutra memory gateway entry 商业版 (id + content + metadata + created_at).
/// `id` 唯一; `content` 主内容; `metadata` JSON 自由扩展; `created_at_secs` UNIX 秒.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// 唯一 id (调用方生成, 推荐 ULID / UUID).
    pub id: String,

    /// 主内容.
    pub content: String,

    /// 自由扩展字段 (JSON object, e.g. `{"tag": "x", "score": 0.9}`).
    pub metadata: HashMap<String, String>,

    /// 创建时间 (UNIX 秒).
    pub created_at_secs: u64,
}

impl MemoryEntry {
    /// 构造新 entry (自动填 `created_at_secs` = now).
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        let created_at_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            id: id.into(),
            content: content.into(),
            metadata,
            created_at_secs,
        }
    }

    /// 构造空 metadata 的 entry (便捷).
    pub fn with_id_and_content(id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::new(id, content, HashMap::new())
    }
}

// ============================================================================
// §2 MemoryQuery — query 条件 (id / content_contains / limit)
// ============================================================================

/// query 条件 (id 精确 / content 模糊 / limit 截断).
///
/// 借鉴 Golutra memory gateway query 商业版. 至少给一个过滤条件, 否则返 InvalidQuery.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryQuery {
    /// 精确 id (Optional).
    pub id: Option<String>,

    /// content 子串 (Optional, 大小写敏感 LIKE '%contains%').
    pub content_contains: Option<String>,

    /// 最大返回数 (Optional, 默认 100, 上限 1000).
    pub limit: Option<usize>,
}

impl MemoryQuery {
    /// 构造空 query.
    pub fn new() -> Self {
        Self::default()
    }

    /// 按 id 精确查询.
    pub fn by_id(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            content_contains: None,
            limit: None,
        }
    }

    /// 按 content 子串查询.
    pub fn by_content_contains(contains: impl Into<String>) -> Self {
        Self {
            id: None,
            content_contains: Some(contains.into()),
            limit: None,
        }
    }

    /// 设置 limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 是否至少给一个过滤条件 (K-1 强校验, 返 InvalidQuery).
    pub fn has_any_filter(&self) -> bool {
        self.id.is_some() || self.content_contains.is_some()
    }

    /// 实际 limit (默认 100, 上限 1000, 0 视为默认).
    pub fn effective_limit(&self) -> usize {
        match self.limit {
            None => 100,
            Some(0) => 100,
            Some(n) if n > 1000 => 1000,
            Some(n) => n,
        }
    }
}

// ============================================================================
// §3 MemoryProvider trait — 3 流 (capture / query / clear) + 2 元 (kind / is_implemented)
// ============================================================================

/// Memory provider trait — 3 流 + 2 元 (per Golutra memory gateway 借鉴).
///
/// 1:1 借鉴 Golutra 公开仓库 memory gateway `MemoryProvider` 商业版接口.
///
/// 所有方法 `Send + Sync` 以便跨任务 / 跨线程共享. 4 stub (Redis / Postgres / S3 /
/// DiskLru) 全部方法返 `BackendNotImplemented` (per task spec, 留 R21 续接).
#[async_trait]
pub trait MemoryProvider: Send + Sync {
    /// capture (写一条 memory).
    ///
    /// - 成功: 返 `Ok(id)` (与入参 id 一致)
    /// - 重复 id: 实现选择 (覆写 or 返错, 当前 3 真接 provider 都覆写)
    /// - stub: 返 `Err(BackendNotImplemented)`
    async fn capture(&self, entry: MemoryEntry) -> MemoryProviderResult<String>;

    /// query (按条件读).
    ///
    /// - 命中: 返 `Ok(Vec<MemoryEntry>)` (按 `created_at_secs` 倒序)
    /// - 未命中: 返 `Ok(vec![])`
    /// - 无过滤条件: 返 `Err(InvalidQuery)` (K-1 强校验)
    /// - stub: 返 `Err(BackendNotImplemented)`
    async fn query(&self, q: MemoryQuery) -> MemoryProviderResult<Vec<MemoryEntry>>;

    /// clear (删).
    ///
    /// - `Some(id)`: 删该 id, 不存在返 `Err(NotFound)`
    /// - `None`: 删全部
    /// - stub: 返 `Err(BackendNotImplemented)`
    async fn clear(&self, id: Option<&str>) -> MemoryProviderResult<()>;

    /// 当前 provider 类型 (元信息, 编译期 hardcode).
    fn kind(&self) -> ProviderKind;

    /// 当前 provider 是否完整实现 (元信息, 编译期 hardcode).
    fn is_implemented(&self) -> bool {
        self.kind().is_implemented()
    }
}
