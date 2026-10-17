//! 增量折入 (V4-b 记忆保鲜回路三件套第③件): 同主题**追加**新段, 追加不重写。
//!
//! # 机制
//!
//! 新内容折入既有记忆 = **增量合并**: 同主题在 [`memory_fold_segments`] 追加
//! 新段 (`seq` 递增), 既有段字节级不动 —— 不整段重写 (与保真分级同魂: 已落库
//! 的内容不因后来的合并走样)。主题视图 [`SqliteMemoryStore::topic_text`] 把
//! 段按 `seq` 拼接, 查询期只读。
//!
//! # 追加不重写是机器钉死的, 不是约定
//!
//! `memory_fold_segments` 表带触发器: **UPDATE / DELETE 一律 ABORT** ——
//! 「追加不重写」不是调用方自律, 是数据库层结构上不可能 (见
//! [`SqliteMemoryStore::fold_topic`] 的测试)。

use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

/// 折入段 (主题记忆的最小增量单位)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldSegment {
    /// 主题 (同主题折入同一条目)。
    pub topic: String,
    /// 段序 (每主题从 1 递增)。
    pub seq: i64,
    /// 段内容 (逐字保留, 不改写)。
    pub content: String,
    /// 段内容散列 (FNV-1a 64)。
    pub content_hash: String,
    /// 折入时刻 (epoch 毫秒)。
    pub folded_at_ms: i64,
}

/// 一次增量折入的真账。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldReport {
    /// 主题。
    pub topic: String,
    /// 新段序号。
    pub seq: i64,
    /// 新段内容散列。
    pub content_hash: String,
}

impl SqliteMemoryStore {
    /// 增量折入: 同主题追加新段 (既有段永不改写)。空主题/空内容拒绝即帧。
    pub fn fold_topic(&self, topic: &str, content: &str, now_ms: i64) -> MemoryResult<FoldReport> {
        let conn = self.conn()?;
        fold_conn(&conn, topic, content, now_ms)
    }

    /// 主题的全部折入段 (按 `seq` 升序)。
    pub fn topic_segments(&self, topic: &str) -> MemoryResult<Vec<FoldSegment>> {
        crate::freshness::validate_topic(topic)?;
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT topic, seq, content, content_hash, folded_at_ms \
             FROM memory_fold_segments WHERE topic = ?1 ORDER BY seq ASC",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![topic], |row| {
                Ok(FoldSegment {
                    topic: row.get(0)?,
                    seq: row.get(1)?,
                    content: row.get(2)?,
                    content_hash: row.get(3)?,
                    folded_at_ms: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// 主题记忆视图: 段按 `seq` 拼接 (`\n` 分隔), 只读。
    pub fn topic_text(&self, topic: &str) -> MemoryResult<String> {
        let segments = self.topic_segments(topic)?;
        Ok(segments
            .into_iter()
            .map(|segment| segment.content)
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

/// 折入原语 (同一连接/事务内可用; 泵在事务里调它, 与队列记账同事务原子落库)。
pub(crate) fn fold_conn(
    conn: &rusqlite::Connection,
    topic: &str,
    content: &str,
    now_ms: i64,
) -> MemoryResult<FoldReport> {
    crate::freshness::validate_topic(topic)?;
    if content.trim().is_empty() {
        return Err(MemoryError::Invalid(
            "freshness content must not be empty".to_string(),
        ));
    }
    let hash = crate::freshness::content_hash(content);
    conn.execute(
        "INSERT INTO memory_fold_segments (topic, seq, content, content_hash, folded_at_ms) \
         SELECT ?1, COALESCE(MAX(seq), 0) + 1, ?2, ?3, ?4 \
         FROM memory_fold_segments WHERE topic = ?1",
        rusqlite::params![topic, content, hash, now_ms],
    )?;
    let seq: i64 = conn.query_row(
        "SELECT MAX(seq) FROM memory_fold_segments WHERE topic = ?1",
        rusqlite::params![topic],
        |row| row.get(0),
    )?;
    Ok(FoldReport {
        topic: topic.to_string(),
        seq,
        content_hash: hash,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SqliteMemoryStore {
        SqliteMemoryStore::open_in_memory().expect("in-memory store")
    }

    /// 增量合并 (追加不重写): 第二次折入后第一段字节原样, 只追加新段。
    #[test]
    fn folding_appends_without_rewriting_existing_segments() {
        let store = store();
        let first = store
            .fold_topic("主题甲", "第一段内容", 1_000)
            .expect("fold 1");
        let second = store
            .fold_topic("主题甲", "第二段内容", 2_000)
            .expect("fold 2");
        assert_eq!((first.seq, second.seq), (1, 2), "同主题 seq 递增");

        let segments = store.topic_segments("主题甲").expect("segments");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].content, "第一段内容", "旧段字节原样 (不重写)");
        assert_eq!(
            segments[0].content_hash,
            crate::freshness::content_hash("第一段内容")
        );
        assert_eq!(segments[1].content, "第二段内容", "新内容以追加方式合入");
        assert_eq!(
            store.topic_text("主题甲").expect("text"),
            "第一段内容\n第二段内容",
            "主题视图 = 各段按 seq 拼接"
        );
    }

    /// 追加不重写是机器钉死的: 直接 UPDATE / DELETE 折入段被触发器拒绝。
    #[test]
    fn rewrite_and_delete_are_structurally_impossible() {
        let store = store();
        store.fold_topic("主题甲", "原始段", 1_000).expect("fold");
        {
            let conn = store.conn().expect("conn");
            let update = conn.execute(
                "UPDATE memory_fold_segments SET content = '篡改' WHERE topic = '主题甲'",
                [],
            );
            assert!(update.is_err(), "改写折入段必须被拒: {update:?}");
            let delete = conn.execute(
                "DELETE FROM memory_fold_segments WHERE topic = '主题甲'",
                [],
            );
            assert!(delete.is_err(), "删除折入段必须被拒: {delete:?}");
        }
        assert_eq!(
            store.topic_text("主题甲").expect("text"),
            "原始段",
            "被拒的操作不得留下任何痕迹"
        );
    }

    /// 折入按主题独立编号 (两主题互不串号)。
    #[test]
    fn fold_sequences_are_per_topic() {
        let store = store();
        store.fold_topic("主题甲", "甲一段", 1_000).expect("fold");
        store.fold_topic("主题甲", "甲二段", 2_000).expect("fold");
        let report = store.fold_topic("主题乙", "乙一段", 3_000).expect("fold");
        assert_eq!(report.seq, 1, "主题乙从 1 起编号");
    }

    /// 空主题/空内容拒绝即帧。
    #[test]
    fn empty_topic_or_content_is_refused() {
        let store = store();
        assert!(store.fold_topic("", "内容", 0).is_err());
        assert!(store.fold_topic("主题", "  ", 0).is_err());
    }
}
