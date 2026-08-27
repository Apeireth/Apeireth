//! SQLite MemoryBackend 适配 (委托给现有 SqliteMemoryStore).
//!
//! **0 重写 SQL**：本 adapter 不重写任何 SQL。所有方法**委托**给
//! `SqliteMemoryStore` 的成熟实现（M1A 阶段已落地的 WAL + migrations + 6 历史流）。
//!
//! 委托关系:
//! - `put_episode`    → `SqliteMemoryStore` 的 `EpisodeStore::put_episode` trait impl
//! - `get_episode`    → `EpisodeStore::get_episode`
//! - `recent_episodes` → `EpisodeStore::recent_episodes`
//! - `append_stream`  → `HistoryStream::insert` (每流有自己 const KIND)
//! - `list_stream`    → `HistoryStream::query_by_session`
//!
//! **架构优势**：
//! - trait 边界清晰（知道"何时调后端"）
//! - 不破现有 24 个 memory 子模块（零重写 SQL，零迁移风险）
//! - 未来加 MongoDB / RocksDB 后端时，**只写新 adapter**，不改 memory domain

use std::sync::Arc;

use apeireth_core::Episode;

use crate::append_only::{HistoryEntry, HistoryStream};
use crate::episode::EpisodeStore;
use crate::{MemoryError, MemoryResult, SqliteMemoryStore, StreamKind};

use super::{BackendKind, MemoryBackend};

/// SQLite 后端（默认，v1 compat）。
///
/// 内部持 `Arc<SqliteMemoryStore>` 共享连接（v1 的 `SqliteMemoryStore::conn()`
/// 拿锁是 `Mutex<Connection>`，所以 clone store 共享同一连接——多线程要同步）。
///
/// **Send + Sync 实现**：通过 `Mutex` 内部互斥（来自 `SqliteMemoryStore` 自身）。
pub struct SqliteBackend {
    store: Arc<SqliteMemoryStore>,
}

impl SqliteBackend {
    /// 从 `SqliteMemoryStore` 创建。
    pub fn new(store: SqliteMemoryStore) -> Self {
        Self {
            store: Arc::new(store),
        }
    }

    /// 从 `Arc<SqliteMemoryStore>` 创建（共享场景）。
    pub fn from_arc(store: Arc<SqliteMemoryStore>) -> Self {
        Self { store }
    }

    /// 拿到内部 store（用于走完整 v1 API，如 EpisodeQuery 复合查询）。
    pub fn store(&self) -> &SqliteMemoryStore {
        &self.store
    }
}

impl MemoryBackend for SqliteBackend {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::Sqlite
    }

    fn put_episode(&self, ep: &Episode) -> MemoryResult<()> {
        <SqliteMemoryStore as EpisodeStore>::put_episode(&*self.store, ep)
    }

    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>> {
        <SqliteMemoryStore as EpisodeStore>::get_episode(&*self.store, id)
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>> {
        <SqliteMemoryStore as EpisodeStore>::recent_episodes(&*self.store, session_id, n)
    }

    fn append_stream(&self, kind: StreamKind, entry: HistoryEntry) -> MemoryResult<()> {
        // StreamHandle::new(kind, &conn) 路由到正确的 *Stream
        // 6 个流都实现 HistoryStream trait (const KIND 不同)
        let conn = self.store.conn()?;
        <crate::streams::StreamHandle<'_> as HistoryStream>::append(
            &crate::streams::StreamHandle::new(kind, &conn),
            &entry,
        )
    }

    fn list_stream(
        &self,
        kind: StreamKind,
        session_id: &str,
        n: usize,
    ) -> MemoryResult<Vec<HistoryEntry>> {
        // 通过 StreamHandle::list_for_session 委托
        // 然后客户端按时间升序取末尾 N 条（与 InMemory/File 行为一致）
        let conn = self.store.conn()?;
        let mut all = <crate::streams::StreamHandle<'_> as HistoryStream>::list_for_session(
            &crate::streams::StreamHandle::new(kind, &conn),
            session_id,
            false, // 默认不过滤 tombstone
        )?;
        all.sort_by_key(|e| e.created_at);
        if all.len() > n {
            let skip = all.len() - n;
            all.drain(..skip);
        }
        Ok(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::append_only::HistoryEntry;
    use crate::SqliteMemoryStore;

    fn fresh() -> SqliteBackend {
        SqliteBackend::new(SqliteMemoryStore::open_in_memory().unwrap())
    }

    fn ep(id: &str, session: &str) -> Episode {
        Episode {
            id: id.to_string(),
            timestamp: 1_700_000_000,
            role: "user".to_string(),
            content: format!("content of {id}"),
            session_id: session.to_string(),
        }
    }

    fn he(id: &str, session: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            subject_id: "subj-1".to_string(),
            subject_rev: 1,
            session_id: Some(session.to_string()),
            created_at: 1_700_000_100,
            payload: serde_json::json!({"kind": "test"}),
            source: "test".to_string(),
            tags: vec!["unit".to_string()],
            tombstoned_at: None,
        }
    }

    #[test]
    fn name_and_kind() {
        let b = fresh();
        assert_eq!(b.name(), "sqlite");
        assert_eq!(b.kind(), BackendKind::Sqlite);
    }

    #[test]
    fn episode_roundtrip() {
        let b = fresh();
        let e = ep("ep-1", "sess-1");
        b.put_episode(&e).unwrap();
        // Episode 没有 PartialEq，字段级对比
        let got = b.get_episode("ep-1").unwrap().expect("episode exists");
        assert_eq!(got.id, e.id);
        assert_eq!(got.timestamp, e.timestamp);
        assert_eq!(got.role, e.role);
        assert_eq!(got.content, e.content);
        assert_eq!(got.session_id, e.session_id);
        let recent = b.recent_episodes("sess-1", 10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "ep-1");
    }

    #[test]
    fn append_and_list_stream_through_trait() {
        let b = fresh();
        let session = "sess-stream";
        b.append_stream(StreamKind::Thought, he("t-1", session)).unwrap();
        b.append_stream(StreamKind::Thought, he("t-2", session)).unwrap();
        let listed = b.list_stream(StreamKind::Thought, session, 10).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "t-1");
        assert_eq!(listed[1].id, "t-2");
    }

    #[test]
    fn ping_succeeds() {
        let b = fresh();
        assert!(b.ping().is_ok());
    }
}
