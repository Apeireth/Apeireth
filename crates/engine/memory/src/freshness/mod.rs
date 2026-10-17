//! V4-b 记忆保鲜回路: **保鲜三件套** (散列增量门 / 可恢复摄取队列 / 增量折入)。
//!
//! # 为什么有这一页
//!
//! 记忆库建成后新素材、修改、删除无法同步, 知识会过期; 而每次摄入都全量重算
//! 又贵又会产生重复记忆。保鲜回路的立场: **变更驱动的增量摄入是常态路径** ——
//! 运行成本随「变更量」而不是随「摄入次数」增长。
//!
//! # 三件套管线图
//!
//! ```text
//!  摄取请求 (source, topic, content)
//!      │
//!      ▼
//!  ① 散列增量门 (hash_gate): FNV-1a 64 内容散列, 查 source_hash 散列账
//!      ├─ 同源同散列已在账上 ──▶ 跳过重复摄取 (省算力防重复记忆)
//!      │                        └─ 去重计数 skip_count +1 (真值)
//!      └─ 同源新散列 (新内容 / 变内容) ──▶ 放行
//!      │
//!      ▼
//!  ② 可恢复摄取队列 (ingest_queue): 待摄取清单落库 (ingest_queue 表)
//!      ├─ 串行处理 (一次泵一条, 并发不放大下游)
//!      ├─ 失败留队列 + 尝试计数 (attempts / last_error; 不静默丢)
//!      └─ 进程重启续摄 (清单落库 = 重启即续; pending 真值即「待摄取数」)
//!      │
//!      ▼
//!  ③ 增量折入 (fold): 同主题追加新段 (memory_fold_segments 表)
//!      ├─ 追加不重写 (旧行字节原样; 触发器钉死 append-only)
//!      └─ 折入后散列记账 (mark_ingested) —— 失败不污染账
//!      │
//!      ▼
//!  自述记忆节真值: 待摄取数 / 最近折入 / 去重计数 (缺源显式 null)
//! ```
//!
//! # 三件分工 (与保真分级同魂)
//!
//! - **散列增量门** ([`hash_gate`]): 「进不进」的前置判定 —— 同源同散列跳过,
//!   变内容重摄; 记账是摄入成功后的显式动作, 失败路径不污染账。
//! - **可恢复摄取队列** ([`ingest_queue`]): 「什么时候进」的耐久清单 —— 待摄取
//!   落库、重启续摄、失败留队列带尝试计数。
//! - **增量折入** ([`fold`]): 「怎么进」的合并策略 —— 新内容折入既有主题记忆
//!   = 同主题**追加**新段, 不整段重写 (保真: 既有内容字节级不动)。
//!
//! # 自述节真值 (只报数字, 不回内容)
//!
//! [`FreshnessStatus`] 是 `tool.self_status` 记忆节三字段 (待摄取数 / 最近折入 /
//! 去重计数) 的真值来源; 尚未折入 = `None` (显式 null, 不编时间)。

pub mod fold;
pub mod hash_gate;
pub mod ingest_queue;

pub use fold::{FoldReport, FoldSegment};
pub use hash_gate::{content_hash, fnv1a64, GateDecision, IngestGate};
pub use ingest_queue::{EnqueueOutcome, IngestState, IngestTask, PumpOutcome};

use rusqlite::OptionalExtension;

use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 保鲜回路状态真值 (self_status 记忆节三字段的真值来源; 只报数字, 不回内容)。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FreshnessStatus {
    /// 待摄取数 (`ingest_queue` 表 `state = 'pending'` 行数)。
    pub pending_ingest: u64,
    /// 最近折入时刻 (epoch 毫秒; 尚未折入 = `None`, 显式 null 不编时间)。
    pub last_fold_at_ms: Option<i64>,
    /// 去重计数 (`source_hash` 散列账 `skip_count` 累计 = 跳过重复摄取次数)。
    pub dedup_skipped: u64,
}

impl SqliteMemoryStore {
    /// 保鲜回路状态真值 (每次现探测; 只回计数与时刻, 不回内容)。
    ///
    /// 口径: 待摄取数 = 队列 `pending` 行数; 去重计数 = 散列账 `skip_count` 总和;
    /// 最近折入 = 折入段账最新 `folded_at_ms` (无折入段 = `None`)。
    pub fn freshness_status(&self) -> MemoryResult<FreshnessStatus> {
        let conn = self.conn()?;
        let pending_ingest: i64 = conn.query_row(
            "SELECT COUNT(*) FROM ingest_queue WHERE state = 'pending'",
            [],
            |row| row.get(0),
        )?;
        let last_fold_at_ms: Option<i64> = conn
            .query_row(
                "SELECT MAX(folded_at_ms) FROM memory_fold_segments",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let dedup_skipped: i64 = conn.query_row(
            "SELECT COALESCE(SUM(skip_count), 0) FROM source_hash",
            [],
            |row| row.get(0),
        )?;
        Ok(FreshnessStatus {
            pending_ingest: pending_ingest.max(0) as u64,
            last_fold_at_ms,
            dedup_skipped: dedup_skipped.max(0) as u64,
        })
    }
}

/// 空主题拒绝即帧 (折入/摄取共用)。
pub(crate) fn validate_topic(topic: &str) -> MemoryResult<()> {
    if topic.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "freshness topic must not be empty".to_string(),
        ));
    }
    Ok(())
}
