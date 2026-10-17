//! 散列增量门 (V4-b 记忆保鲜回路三件套第①件): 摄入面内容散列 + 同源同散列去重。
//!
//! # 机制
//!
//! 摄入面 (整编 / 记忆写入) 在动算力之前先查门: 对来源内容做 **FNV-1a 64**
//! 内容散列 (自含实现, 0 新外部依赖), 以 `(source, content_hash)` 查散列账
//! ([`source_hash`] 表):
//!
//! - **同源同散列已在账上** = 重复摄入 → 跳过 (省算力防重复记忆), 记去重计数;
//! - **同源新散列** = 内容有变 → 放行重摄 (变内容重摄)。
//!
//! # 记账语义 (失败不污染账)
//!
//! [`IngestGate::check`] 是**只读判定**; 落账是摄入成功后的显式动作
//! ([`IngestGate::mark_ingested`]), 失败路径不记账 —— 同源同散列下次仍会被
//! 放行重试, 不会因一次失败被永久跳过。重复摄入的跳过经
//! [`IngestGate::mark_skipped`] 记进 `skip_count` (去重计数真值来源)。

use rusqlite::OptionalExtension;

use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// FNV-1a 64 位内容散列 (自含实现, 0 新外部依赖; 与人格层确定性 id 同族算法)。
pub fn fnv1a64(input: &str) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// 内容散列 (FNV-1a 64 的 16 字符小写十六进制)。
pub fn content_hash(content: &str) -> String {
    format!("{:016x}", fnv1a64(content))
}

/// 散列增量门判定 (摄入面查门结果)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    /// 放行: 同源下该内容散列未入账 = 新内容 (或变内容重摄), 走摄入。
    Ingest {
        /// 本次内容散列。
        content_hash: String,
    },
    /// 跳过: 同源同散列已在账上 = 重复摄入 (省算力防重复记忆)。
    SkipDuplicate {
        /// 本次内容散列 (与账上一致)。
        content_hash: String,
        /// 该 (源, 散列) 首次入账时刻 (epoch 毫秒)。
        first_seen_ms: i64,
        /// 该 (源, 散列) 已有的去重计数 (不含本次跳过)。
        skip_count: u64,
    },
}

impl GateDecision {
    /// 是否放行摄入。
    pub fn should_ingest(&self) -> bool {
        matches!(self, Self::Ingest { .. })
    }
}

/// 摄入散列门 (保鲜回路第①件): 查门 + 两笔显式记账。
///
/// 实现体: [`SqliteMemoryStore`] (散列账与记忆库同库, 迁移 append-only)。
pub trait IngestGate: Send + Sync {
    /// 查门 (只读判定, 不记账; 无时间语义): 同源同散列已在账上 =
    /// [`GateDecision::SkipDuplicate`]。
    fn check(&self, source: &str, content: &str) -> Result<GateDecision, String>;
    /// 摄取**成功后**记账: `(source, content_hash)` 入散列账 (重复调用累加
    /// `ingest_count`, 不产生第二行)。
    fn mark_ingested(&self, source: &str, content: &str, now_ms: i64) -> Result<(), String>;
    /// 重复摄入记账: `skip_count` +1 (去重计数真值), 回累计去重计数。
    fn mark_skipped(&self, source: &str, content: &str, now_ms: i64) -> Result<u64, String>;
}

impl IngestGate for SqliteMemoryStore {
    fn check(&self, source: &str, content: &str) -> Result<GateDecision, String> {
        self.ingest_gate_check(source, content)
            .map_err(|error| error.to_string())
    }

    fn mark_ingested(&self, source: &str, content: &str, now_ms: i64) -> Result<(), String> {
        self.ingest_gate_mark_ingested(source, content, now_ms)
            .map_err(|error| error.to_string())
    }

    fn mark_skipped(&self, source: &str, content: &str, now_ms: i64) -> Result<u64, String> {
        self.ingest_gate_mark_skipped(source, content, now_ms)
            .map_err(|error| error.to_string())
    }
}

impl SqliteMemoryStore {
    /// 查门 (只读判定, 不记账; 无时间语义)。空源/空内容拒绝即帧。
    pub fn ingest_gate_check(&self, source: &str, content: &str) -> MemoryResult<GateDecision> {
        let conn = self.conn()?;
        check_conn(&conn, source, content)
    }

    /// 摄取成功记账: `(source, content_hash)` 入账, `ingest_count` 累加。
    ///
    /// 每次成功摄入都记一笔 (账如实反映摄入次数), 但**不产生第二行**。
    pub fn ingest_gate_mark_ingested(
        &self,
        source: &str,
        content: &str,
        now_ms: i64,
    ) -> MemoryResult<()> {
        let conn = self.conn()?;
        mark_ingested_conn(&conn, source, content, now_ms)
    }

    /// 重复摄入记账: `skip_count` +1, 回累计去重计数 (含本次)。
    pub fn ingest_gate_mark_skipped(
        &self,
        source: &str,
        content: &str,
        now_ms: i64,
    ) -> MemoryResult<u64> {
        let conn = self.conn()?;
        mark_skipped_conn(&conn, source, content, now_ms)
    }
}

/// 查门原语 (同一连接/事务内可用)。空源/空内容拒绝即帧。
pub(crate) fn check_conn(
    conn: &rusqlite::Connection,
    source: &str,
    content: &str,
) -> MemoryResult<GateDecision> {
    validate_source_content(source, content)?;
    let hash = content_hash(content);
    let row: Option<i64> = conn
        .query_row(
            "SELECT first_seen_ms FROM source_hash WHERE source = ?1 AND content_hash = ?2",
            rusqlite::params![source, hash],
            |row| row.get(0),
        )
        .optional()?;
    match row {
        Some(first_seen_ms) => {
            let skip_count: i64 = conn.query_row(
                "SELECT skip_count FROM source_hash WHERE source = ?1 AND content_hash = ?2",
                rusqlite::params![source, hash],
                |row| row.get(0),
            )?;
            Ok(GateDecision::SkipDuplicate {
                content_hash: hash,
                first_seen_ms,
                skip_count: skip_count.max(0) as u64,
            })
        }
        None => Ok(GateDecision::Ingest { content_hash: hash }),
    }
}

/// 摄取成功记账原语 (同一连接/事务内可用)。
pub(crate) fn mark_ingested_conn(
    conn: &rusqlite::Connection,
    source: &str,
    content: &str,
    now_ms: i64,
) -> MemoryResult<()> {
    validate_source_content(source, content)?;
    let hash = content_hash(content);
    upsert_hash_row(conn, source, &hash, now_ms, HashHit::Ingested)
}

/// 重复摄入记账原语 (同一连接/事务内可用), 回累计去重计数 (含本次)。
pub(crate) fn mark_skipped_conn(
    conn: &rusqlite::Connection,
    source: &str,
    content: &str,
    now_ms: i64,
) -> MemoryResult<u64> {
    validate_source_content(source, content)?;
    let hash = content_hash(content);
    upsert_hash_row(conn, source, &hash, now_ms, HashHit::Skipped)?;
    let skip_count: i64 = conn.query_row(
        "SELECT skip_count FROM source_hash WHERE source = ?1 AND content_hash = ?2",
        rusqlite::params![source, hash],
        |row| row.get(0),
    )?;
    Ok(skip_count.max(0) as u64)
}

/// 散列账一笔命中是摄入还是跳过 (决定累加哪个计数)。
enum HashHit {
    Ingested,
    Skipped,
}

/// 散列账 upsert: 首见插行, 再见累加计数 (`first_seen_ms` 永不改写)。
fn upsert_hash_row(
    conn: &rusqlite::Connection,
    source: &str,
    hash: &str,
    now_ms: i64,
    hit: HashHit,
) -> MemoryResult<()> {
    let (ingest_delta, skip_delta) = match hit {
        HashHit::Ingested => (1i64, 0i64),
        HashHit::Skipped => (0i64, 1i64),
    };
    conn.execute(
        "INSERT INTO source_hash \
             (source, content_hash, first_seen_ms, last_seen_ms, ingest_count, skip_count) \
         VALUES (?1, ?2, ?3, ?3, ?4, ?5) \
         ON CONFLICT(source, content_hash) DO UPDATE SET \
             last_seen_ms = excluded.last_seen_ms, \
             ingest_count = source_hash.ingest_count + excluded.ingest_count, \
             skip_count = source_hash.skip_count + excluded.skip_count",
        rusqlite::params![source, hash, now_ms, ingest_delta, skip_delta],
    )?;
    Ok(())
}

/// 空源/空内容拒绝即帧 (错误即帧, 不静默吞)。
pub(crate) fn validate_source_content(source: &str, content: &str) -> MemoryResult<()> {
    if source.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "freshness source must not be empty".to_string(),
        ));
    }
    if content.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "freshness content must not be empty".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("in-memory store")
    }

    /// FNV-1a 64 标准测试向量 (自含实现的正确性锚)。
    #[test]
    fn fnv1a64_matches_the_standard_vectors() {
        assert_eq!(fnv1a64(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64("a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64("foobar"), 0x8594_4171_f739_67e8);
    }

    /// 内容散列稳定且随内容变化 (变内容 = 变散列)。
    #[test]
    fn content_hash_is_stable_and_tracks_content() {
        assert_eq!(content_hash("同一内容"), content_hash("同一内容"));
        assert_ne!(content_hash("同一内容"), content_hash("变了的内容"));
        assert_eq!(content_hash("x").len(), 16);
    }

    /// 散列去重 (同内容跳过): 同源同散列第二次查门 = 跳过重复摄入。
    #[test]
    fn same_source_same_content_is_skipped_as_duplicate() {
        let store = store();
        let first = store.ingest_gate_check("src-a", "内容甲").expect("check");
        assert!(first.should_ingest(), "首见必须放行: {first:?}");
        store
            .ingest_gate_mark_ingested("src-a", "内容甲", 1_000)
            .expect("mark ingested");

        let second = store.ingest_gate_check("src-a", "内容甲").expect("check");
        match second {
            GateDecision::SkipDuplicate {
                content_hash: hash,
                first_seen_ms,
                skip_count,
            } => {
                assert_eq!(hash, content_hash("内容甲"));
                assert_eq!(first_seen_ms, 1_000, "首见时刻不被重写");
                assert_eq!(skip_count, 0, "尚未记过跳过");
            }
            other => panic!("同源同散列必须跳过: {other:?}"),
        }
    }

    /// 变内容重摄: 同源内容有变 = 新散列 = 放行。
    #[test]
    fn changed_content_is_reingested() {
        let store = store();
        store
            .ingest_gate_mark_ingested("src-a", "内容甲", 1_000)
            .expect("mark");
        let decision = store.ingest_gate_check("src-a", "内容乙").expect("check");
        assert!(decision.should_ingest(), "变内容必须重摄: {decision:?}");
    }

    /// 同内容不同源不互相去重 (同源同散列才跳过)。
    #[test]
    fn same_content_from_another_source_is_not_deduplicated() {
        let store = store();
        store
            .ingest_gate_mark_ingested("src-a", "同一内容", 1_000)
            .expect("mark");
        let decision = store.ingest_gate_check("src-b", "同一内容").expect("check");
        assert!(decision.should_ingest(), "异源不去重: {decision:?}");
    }

    /// 去重计数真值: 每次跳过 +1, 累计回来 (self_status 记忆节的去重计数来源)。
    #[test]
    fn skip_count_accumulates_per_duplicate() {
        let store = store();
        store
            .ingest_gate_mark_ingested("src-a", "内容甲", 1_000)
            .expect("mark");
        assert_eq!(
            store
                .ingest_gate_mark_skipped("src-a", "内容甲", 2_000)
                .expect("skip 1"),
            1
        );
        assert_eq!(
            store
                .ingest_gate_mark_skipped("src-a", "内容甲", 3_000)
                .expect("skip 2"),
            2
        );
    }

    /// 记账语义 (失败不污染账): 只查门不记账, 下次仍放行。
    #[test]
    fn check_alone_does_not_poison_the_ledger() {
        let store = store();
        assert!(store
            .ingest_gate_check("src-a", "内容甲")
            .expect("check")
            .should_ingest());
        assert!(
            store
                .ingest_gate_check("src-a", "内容甲")
                .expect("check again")
                .should_ingest(),
            "查门只读: 未记账不得跳过"
        );
    }

    /// 空源/空内容拒绝即帧 (不静默吞)。
    #[test]
    fn empty_source_or_content_is_refused() {
        let store = store();
        assert!(store.ingest_gate_check("", "内容").is_err());
        assert!(store.ingest_gate_check("src", "  ").is_err());
    }
}
