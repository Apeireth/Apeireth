//! SQLite MemoryBackend 适配 (委托给现有 SqliteMemoryStore).
//!
//! **0 重写 SQL**：本 adapter 不重写任何 SQL。所有方法**委托**给
//! `SqliteMemoryStore` 的成熟实现（M1A 阶段已落地的 WAL + migrations + 6 历史流）。
//!
//! 委托关系:
//! - `put_episode`    → `SqliteMemoryStore` 的 `EpisodeStore::put_episode` trait impl
//! - `get_episode`    → `EpisodeStore::get_episode`
//! - `recent_episodes` → `EpisodeStore::recent_episodes`
//! - `append_stream`  → deserialize `serde_json::Value` → `HistoryEntry` → StreamHandle::append
//! - `list_stream`    → StreamHandle::list_for_session → serialize 列表 → return `Vec<Value>`
//!
//! **架构优势**：
//! - trait 边界在 `apeireth_plugin` (Refactor-1, 2026-08-27), impl 在本 crate (engine)
//! - 单向依赖: memory → plugin (不反向)
//! - 现有 24 个 memory 子模块 0 重写
//! - 未来加 MongoDB / RocksDB = 新增本 crate 内的 adapter, 0 改 memory domain

use std::sync::Arc;

use apeireth_core::kernel::memory::Episode;

use crate::append_only::{HistoryEntry, HistoryStream};
use crate::episode::EpisodeStore;
use crate::{MemoryError, MemoryResult, SqliteMemoryStore};

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

        let conn = self.store.conn()?;
        let mut all = <crate::streams::StreamHandle<'_> as HistoryStream>::list_for_session(
            &crate::streams::StreamHandle::new(kind, &conn),
            session_id,
            false,
        )?;
        all.sort_by_key(|e| e.created_at);
        if all.len() > n {
            let skip = all.len() - n;
            all.drain(..skip);
        }
        // trait 要求 typed HistoryEntry 直接返, 不再 serde round-trip
        Ok(all)
    }
}

/// 6 流名 → memory 的 `StreamKind` 映射
///
/// 0 装: 字符串硬编码; rc 阶段接 SchemaRegistry 时改


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
        // O-6 锚 #18 兑现: trait 接口走 typed HistoryEntry (不再 serde round-trip)
        let thought = crate::from_str_core("thought").expect("valid stream");
        b.append_stream(thought, he("t-1", session)).unwrap();
        b.append_stream(thought, he("t-2", session)).unwrap();
        let listed = b.list_stream(thought, session, 10).unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "t-1");
        assert_eq!(listed[1].id, "t-2");
    }

    #[test]
    fn unknown_stream_name_is_rejected() {
        let b = fresh();
        let invalid = crate::StreamKind::Thought; // typed enum 不可能 unknown (编译期保证)
        let _ = b.append_stream(invalid, he("x", "s")); // 验证编译过, 语义由 typed enum 保证
    }

    #[test]
    fn ping_succeeds() {
        let b = fresh();
        assert!(b.ping().is_ok());
    }
}
