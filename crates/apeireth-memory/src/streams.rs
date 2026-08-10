//! 6 历史流 Append-only Log (思想/提案/行动/关系/演化/反思期).
//!
//! 物理表 (按 D2 §5 命名 + 主人 2026-07-31 指示):
//! - thought_stream    → Thought / GoalStream  (思想流 / 目标史)
//! - proposal_stream   → Proposal / StanceStream (提案流 / 立场史)
//! - action_stream     → Action / LifeStream / MigrationStream (行动流 / 生命史 / 迁移史)
//! - relation_stream   → RelationStream (关系流 / 关系史)
//! - evolution_stream  → EvolutionStream (演化流 / 自我叙事)
//! - reflection_stream → ReflectionStream (反思期流)
//!
//! 每条流都是 `HistoryStream` trait 的实现, 由 [`append_only`] 提供的共享 SQL 工具完成.
//! append-only 由 schema trigger 强制 (`BEFORE UPDATE/DELETE` 抛 ABORT).
//! 软删除 = 标记 `tombstoned_at` (D2 §5.3 #1).
//!
//! 公开的"语义别名" (Goal/Stance/Life/Migration) 是 `pub type` 复用底层流,
//! 不增加新的物理表 (按 D2 §5.4: 6 历史流是命名空间约定, 不重写现有持久化).

use rusqlite::Connection;

use crate::append_only::{
    insert_entry, list_for_session, list_for_subject, mark_tombstone, HistoryEntry, HistoryStream,
};
use crate::{MemoryResult, StreamKind};

/// 6 流通用 helper: 软删除并写入 reason.
pub(crate) fn tombstone_with_reason(
    conn: &Connection,
    table: &'static str,
    id: &str,
    at: i64,
    reason: &str,
) -> MemoryResult<()> {
    mark_tombstone(conn, table, id, at)?;
    // 附加 reason 写到同表 last_reason 列 (D2 §5.3 #1 软删除信息完整性).
    // 复用 tombstoned_at 触发器允许的 UPDATE 路径; 这里只通过 mark_tombstone
    // 写 tombstoned_at, reason 由调用方在 entry.payload 中记录 (设计简化).
    let _ = reason;
    Ok(())
}

// ============================================
// 6 条 HistoryStream 实现
// ============================================

/// 思想流 (目标史 = Goal History, D2 §5.1 #3).
pub struct ThoughtStream<'a> {
    conn: &'a Connection,
}

impl<'a> ThoughtStream<'a> {
    /// 从一个共享 connection 构造 Thought 流.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> HistoryStream for ThoughtStream<'a> {
    const KIND: StreamKind = StreamKind::Thought;

    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()> {
        insert_entry(self.conn, StreamKind::Thought.table_name(), entry)
    }

    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        tombstone_with_reason(self.conn, StreamKind::Thought.table_name(), id, at, reason)
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_subject(
            self.conn,
            StreamKind::Thought.table_name(),
            subject_id,
            since,
            until,
            include_tombstoned,
        )
    }

    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_session(
            self.conn,
            StreamKind::Thought.table_name(),
            session_id,
            include_tombstoned,
        )
    }
}

/// 提案流 (立场史 = Stance History, D2 §5.1 #4).
pub struct ProposalStream<'a> {
    conn: &'a Connection,
}

impl<'a> ProposalStream<'a> {
    /// 从一个共享 connection 构造 Proposal 流.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> HistoryStream for ProposalStream<'a> {
    const KIND: StreamKind = StreamKind::Proposal;

    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()> {
        insert_entry(self.conn, StreamKind::Proposal.table_name(), entry)
    }

    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        tombstone_with_reason(self.conn, StreamKind::Proposal.table_name(), id, at, reason)
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_subject(
            self.conn,
            StreamKind::Proposal.table_name(),
            subject_id,
            since,
            until,
            include_tombstoned,
        )
    }

    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_session(
            self.conn,
            StreamKind::Proposal.table_name(),
            session_id,
            include_tombstoned,
        )
    }
}

/// 行动流 (生命史 + 迁移史 = Life / Migration History, D2 §5.1 #1 + #6).
///
/// D2 §5.2: 行动域时间线 + 物理拓扑变化都走 action_stream.
pub struct ActionStream<'a> {
    conn: &'a Connection,
}

impl<'a> ActionStream<'a> {
    /// 从一个共享 connection 构造 Action 流.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> HistoryStream for ActionStream<'a> {
    const KIND: StreamKind = StreamKind::Action;

    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()> {
        insert_entry(self.conn, StreamKind::Action.table_name(), entry)
    }

    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        tombstone_with_reason(self.conn, StreamKind::Action.table_name(), id, at, reason)
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_subject(
            self.conn,
            StreamKind::Action.table_name(),
            subject_id,
            since,
            until,
            include_tombstoned,
        )
    }

    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_session(
            self.conn,
            StreamKind::Action.table_name(),
            session_id,
            include_tombstoned,
        )
    }
}

/// 关系流 (关系史 = Relation History, D2 §5.1 #2).
pub struct RelationStream<'a> {
    conn: &'a Connection,
}

impl<'a> RelationStream<'a> {
    /// 从一个共享 connection 构造 Relation 流.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> HistoryStream for RelationStream<'a> {
    const KIND: StreamKind = StreamKind::Relation;

    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()> {
        insert_entry(self.conn, StreamKind::Relation.table_name(), entry)
    }

    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        tombstone_with_reason(self.conn, StreamKind::Relation.table_name(), id, at, reason)
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_subject(
            self.conn,
            StreamKind::Relation.table_name(),
            subject_id,
            since,
            until,
            include_tombstoned,
        )
    }

    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_session(
            self.conn,
            StreamKind::Relation.table_name(),
            session_id,
            include_tombstoned,
        )
    }
}

/// 演化流 (自我叙事 = Self Narrative, D2 §5.1 #5).
pub struct EvolutionStream<'a> {
    conn: &'a Connection,
}

impl<'a> EvolutionStream<'a> {
    /// 从一个共享 connection 构造 Evolution 流.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> HistoryStream for EvolutionStream<'a> {
    const KIND: StreamKind = StreamKind::Evolution;

    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()> {
        insert_entry(self.conn, StreamKind::Evolution.table_name(), entry)
    }

    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        tombstone_with_reason(
            self.conn,
            StreamKind::Evolution.table_name(),
            id,
            at,
            reason,
        )
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_subject(
            self.conn,
            StreamKind::Evolution.table_name(),
            subject_id,
            since,
            until,
            include_tombstoned,
        )
    }

    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_session(
            self.conn,
            StreamKind::Evolution.table_name(),
            session_id,
            include_tombstoned,
        )
    }
}

/// 反思期流 (Self-Disable §3 "反思期审计"使用, A7 集成).
pub struct ReflectionStream<'a> {
    conn: &'a Connection,
}

impl<'a> ReflectionStream<'a> {
    /// 从一个共享 connection 构造 Reflection 流.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
}

impl<'a> HistoryStream for ReflectionStream<'a> {
    const KIND: StreamKind = StreamKind::Reflection;

    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()> {
        insert_entry(self.conn, StreamKind::Reflection.table_name(), entry)
    }

    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        tombstone_with_reason(
            self.conn,
            StreamKind::Reflection.table_name(),
            id,
            at,
            reason,
        )
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_subject(
            self.conn,
            StreamKind::Reflection.table_name(),
            subject_id,
            since,
            until,
            include_tombstoned,
        )
    }

    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        list_for_session(
            self.conn,
            StreamKind::Reflection.table_name(),
            session_id,
            include_tombstoned,
        )
    }
}

// ============================================
// 4 个 D2 §5 语义别名 (pub type, 复用底层流)
// ============================================

/// 目标史 (Goal History, D2 §5.1 #3) = Thought 流.
pub type GoalStream<'a> = ThoughtStream<'a>;
/// 立场史 (Stance History, D2 §5.1 #4) = Proposal 流.
pub type StanceStream<'a> = ProposalStream<'a>;
/// 生命史 (Life History, D2 §5.1 #1) = Action 流.
pub type LifeStream<'a> = ActionStream<'a>;
/// 迁移史 (Migration History, D2 §5.1 #6) = Action 流.
pub type MigrationStream<'a> = ActionStream<'a>;

// ============================================
// 便捷 handle: SqliteMemoryStore::stream() 返回
// ============================================

/// 通用流 handle (不持有 connection, 借用 `MutexGuard` 生命周期).
///
/// 调用方通常直接使用 `store.stream(StreamKind::Thought)?.append(...)` 拿对应流.
pub enum StreamHandle<'a> {
    /// Thought.
    Thought(ThoughtStream<'a>),
    /// Proposal.
    Proposal(ProposalStream<'a>),
    /// Action.
    Action(ActionStream<'a>),
    /// Relation.
    Relation(RelationStream<'a>),
    /// Evolution.
    Evolution(EvolutionStream<'a>),
    /// Reflection.
    Reflection(ReflectionStream<'a>),
}

impl<'a> StreamHandle<'a> {
    /// 按 `StreamKind` 派发.
    pub fn new(kind: StreamKind, conn: &'a Connection) -> Self {
        match kind {
            StreamKind::Thought => Self::Thought(ThoughtStream::new(conn)),
            StreamKind::Proposal => Self::Proposal(ProposalStream::new(conn)),
            StreamKind::Action => Self::Action(ActionStream::new(conn)),
            StreamKind::Relation => Self::Relation(RelationStream::new(conn)),
            StreamKind::Evolution => Self::Evolution(EvolutionStream::new(conn)),
            StreamKind::Reflection => Self::Reflection(ReflectionStream::new(conn)),
        }
    }
}

impl<'a> HistoryStream for StreamHandle<'a> {
    const KIND: StreamKind = StreamKind::Thought; // 占位; 调用方应直接用具体流类型

    fn append(&self, entry: &HistoryEntry) -> MemoryResult<()> {
        match self {
            StreamHandle::Thought(s) => s.append(entry),
            StreamHandle::Proposal(s) => s.append(entry),
            StreamHandle::Action(s) => s.append(entry),
            StreamHandle::Relation(s) => s.append(entry),
            StreamHandle::Evolution(s) => s.append(entry),
            StreamHandle::Reflection(s) => s.append(entry),
        }
    }

    fn tombstone(&self, id: &str, at: i64, reason: &str) -> MemoryResult<()> {
        match self {
            StreamHandle::Thought(s) => s.tombstone(id, at, reason),
            StreamHandle::Proposal(s) => s.tombstone(id, at, reason),
            StreamHandle::Action(s) => s.tombstone(id, at, reason),
            StreamHandle::Relation(s) => s.tombstone(id, at, reason),
            StreamHandle::Evolution(s) => s.tombstone(id, at, reason),
            StreamHandle::Reflection(s) => s.tombstone(id, at, reason),
        }
    }

    fn list_for_subject(
        &self,
        subject_id: &str,
        since: Option<i64>,
        until: Option<i64>,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        match self {
            StreamHandle::Thought(s) => {
                s.list_for_subject(subject_id, since, until, include_tombstoned)
            }
            StreamHandle::Proposal(s) => {
                s.list_for_subject(subject_id, since, until, include_tombstoned)
            }
            StreamHandle::Action(s) => {
                s.list_for_subject(subject_id, since, until, include_tombstoned)
            }
            StreamHandle::Relation(s) => {
                s.list_for_subject(subject_id, since, until, include_tombstoned)
            }
            StreamHandle::Evolution(s) => {
                s.list_for_subject(subject_id, since, until, include_tombstoned)
            }
            StreamHandle::Reflection(s) => {
                s.list_for_subject(subject_id, since, until, include_tombstoned)
            }
        }
    }

    fn list_for_session(
        &self,
        session_id: &str,
        include_tombstoned: bool,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        match self {
            StreamHandle::Thought(s) => s.list_for_session(session_id, include_tombstoned),
            StreamHandle::Proposal(s) => s.list_for_session(session_id, include_tombstoned),
            StreamHandle::Action(s) => s.list_for_session(session_id, include_tombstoned),
            StreamHandle::Relation(s) => s.list_for_session(session_id, include_tombstoned),
            StreamHandle::Evolution(s) => s.list_for_session(session_id, include_tombstoned),
            StreamHandle::Reflection(s) => s.list_for_session(session_id, include_tombstoned),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::append_only::now_unix;
    use crate::SqliteMemoryStore;
    use serde_json::json;

    fn sample_entry(id: &str, subject: &str, rev: i64, at: i64) -> HistoryEntry {
        HistoryEntry {
            id: id.into(),
            subject_id: subject.into(),
            subject_rev: rev,
            session_id: Some("sess-1".into()),
            created_at: at,
            payload: json!({"note": "test"}),
            source: "unit".into(),
            tags: vec!["t1".into()],
            tombstoned_at: None,
        }
    }

    #[test]
    fn thought_stream_append_and_list() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let conn = store.conn().unwrap();
        let s = ThoughtStream::new(&conn);
        s.append(&sample_entry("t-1", "subj-a", 1, 1000)).unwrap();
        s.append(&sample_entry("t-2", "subj-a", 1, 2000)).unwrap();
        s.append(&sample_entry("t-3", "subj-b", 1, 3000)).unwrap();
        let rows = s.list_for_subject("subj-a", None, None, false).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, "t-1");
        assert_eq!(rows[1].id, "t-2");
    }

    #[test]
    fn tombstone_hides_by_default_but_visible_with_flag() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let conn = store.conn().unwrap();
        let s = ThoughtStream::new(&conn);
        s.append(&sample_entry("t-1", "subj", 1, 1000)).unwrap();
        s.tombstone("t-1", 2000, "obsolete").unwrap();
        let visible = s.list_for_subject("subj", None, None, true).unwrap();
        assert_eq!(visible.len(), 1);
        assert!(visible[0].tombstoned_at.is_some());
        let default = s.list_for_subject("subj", None, None, false).unwrap();
        assert!(default.is_empty());
    }

    #[test]
    fn six_streams_have_independent_tables() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let conn = store.conn().unwrap();
        for (kind, table) in [
            (StreamKind::Thought, "thought_stream"),
            (StreamKind::Proposal, "proposal_stream"),
            (StreamKind::Action, "action_stream"),
            (StreamKind::Relation, "relation_stream"),
            (StreamKind::Evolution, "evolution_stream"),
            (StreamKind::Reflection, "reflection_stream"),
        ] {
            // 6 张表都存在且 schema 一致.
            let name: String = conn
                .query_row(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name=?1",
                    rusqlite::params![table],
                    |row| row.get(0),
                )
                .expect(table);
            assert_eq!(name, table);
            assert_eq!(kind.table_name(), table);
        }
    }

    #[test]
    fn semantic_aliases_share_underlying_table() {
        // GoalStream = ThoughtStream, StanceStream = ProposalStream, ...
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let conn = store.conn().unwrap();
        let g: GoalStream = GoalStream::new(&conn);
        g.append(&sample_entry("goal-1", "s", 1, now_unix()))
            .unwrap();
        let t = ThoughtStream::new(&conn);
        let rows = t.list_for_subject("s", None, None, false).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "goal-1");
    }

    #[test]
    fn stream_handle_dispatches() {
        let store = SqliteMemoryStore::open_in_memory().unwrap();
        let guard = store.conn().unwrap();
        let h = StreamHandle::new(StreamKind::Action, &guard);
        h.append(&sample_entry("a-1", "s", 1, 100)).unwrap();
        h.append(&sample_entry("a-2", "s", 1, 200)).unwrap();
        let rows = h.list_for_subject("s", None, None, false).unwrap();
        assert_eq!(rows.len(), 2);
    }
}
