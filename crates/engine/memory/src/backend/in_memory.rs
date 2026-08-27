use apeireth_core::kernel::StreamKind;
//! InMemory MemoryBackend（仅测试，进程重启数据丢失）。
//!
//! 0 装 PASS：进程结束 = 数据全无。**不**用于生产。`BackendKind::InMemory` 明确标识此点。
//!
//! 实现用 `std::sync::Mutex<HashMap<...>>` 包裹所有状态，**Send + Sync**。
//! 不阻塞 I/O（纯内存）。

use std::collections::HashMap;
use std::sync::Mutex;

use apeireth_core::kernel::memory::Episode;

use crate::append_only::HistoryEntry;
use crate::MemoryResult;
use crate::StreamKind;

use super::{BackendKind, MemoryBackend};

/// 进程内 HashMap 后端（测试用）。
pub struct InMemoryBackend {
    episodes_by_id: Mutex<HashMap<String, Episode>>,
    /// `(stream_kind, session_id) -> ordered list of entries`
    streams: Mutex<HashMap<(StreamKind, String), Vec<HistoryEntry>>>,
}

impl InMemoryBackend {
    pub fn new() -> Self {
        Self {
            episodes_by_id: Mutex::new(HashMap::new()),
            streams: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBackend for InMemoryBackend {
    fn name(&self) -> &'static str {
        "in_memory"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::InMemory
    }

    fn put_episode(&self, ep: &Episode) -> MemoryResult<()> {
        let mut map = self.episodes_by_id.lock().expect("InMemoryBackend poisoned");
        // append-only 语义：拒绝覆盖已存在 id（与 SQLite 行为一致）
        if map.contains_key(&ep.id) {
            return Err(crate::MemoryError::Invalid(format!(
                "episode id already exists: {}",
                ep.id
            )));
        }
        map.insert(ep.id.clone(), ep.clone());
        Ok(())
    }

    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>> {
        let map = self.episodes_by_id.lock().expect("InMemoryBackend poisoned");
        Ok(map.get(id).cloned())
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>> {
        let map = self.episodes_by_id.lock().expect("InMemoryBackend poisoned");
        let mut all: Vec<Episode> = map
            .values()
            .filter(|e| e.session_id == session_id)
            .cloned()
            .collect();
        all.sort_by_key(|e| e.timestamp);
        if all.len() > n {
            let skip = all.len() - n;
            all.drain(..skip);
        }
        Ok(all)
    }

    fn append_stream(&self, kind: StreamKind, entry: HistoryEntry) -> MemoryResult<()> {
        let mut streams = self.streams.lock().expect("InMemoryBackend poisoned");
        let key = (kind, entry.session_id.clone().unwrap_or_default());
        let list = streams.entry(key).or_insert_with(Vec::new);
        list.push(entry);
        Ok(())
    }

    fn list_stream(
        &self,
        kind: StreamKind,
        session_id: &str,
        n: usize,
    ) -> MemoryResult<Vec<HistoryEntry>> {

        let streams = self.streams.lock().expect("InMemoryBackend poisoned");
        let key = (kind, session_id.to_string());
        let list = match streams.get(&key) {
            Some(l) => l,
            None => return Ok(Vec::new()),
        };
        let mut alive: Vec<HistoryEntry> = list
            .iter()
            .filter(|e| e.tombstoned_at.is_none())
            .cloned()
            .collect();
        alive.sort_by_key(|e| e.created_at);
        if alive.len() > n {
            let skip = alive.len() - n;
            alive.drain(..skip);
        }
        Ok(alive)
    }
}

fn stream_name_to_kind(name: &str) -> MemoryResult<StreamKind> {
    match name {
        "thought" => Ok(StreamKind::Thought),
        "proposal" => Ok(StreamKind::Proposal),
        "action" => Ok(StreamKind::Action),
        "relation" => Ok(StreamKind::Relation),
        "evolution" => Ok(StreamKind::Evolution),
        "reflection" => Ok(StreamKind::Reflection),
        other => Err(crate::MemoryError::Invalid(format!(
            "unknown stream name: {other}; expected one of 6: thought/proposal/action/relation/evolution/reflection"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::append_only::HistoryEntry;

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
        let b = InMemoryBackend::new();
        assert_eq!(b.name(), "in_memory");
        assert_eq!(b.kind(), BackendKind::InMemory);
    }

    #[test]
    fn episode_roundtrip() {
        let b = InMemoryBackend::new();
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
    }

    #[test]
    fn append_only_rejects_duplicate_episode_id() {
        let b = InMemoryBackend::new();
        let e = ep("dup", "s");
        b.put_episode(&e).unwrap();
        let r = b.put_episode(&e);
        assert!(r.is_err());
    }

    #[test]
    fn stream_roundtrip() {
        let b = InMemoryBackend::new();
        let session = "s1";
        b.append_stream(crate::from_str_core("thought").expect("valid stream"), serde_json::to_value(he("t1", session)).unwrap())
            .unwrap();
        b.append_stream(crate::from_str_core("thought").expect("valid stream"), serde_json::to_value(he("t2", session)).unwrap())
            .unwrap();
        let list = b.list_stream(crate::from_str_core("thought").expect("valid stream"), session, 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["id"], "t1");
        assert_eq!(list[1]["id"], "t2");
    }

    #[test]
    fn list_stream_filters_tombstoned() {
        let b = InMemoryBackend::new();
        let session = "s";
        let mut alive = he("a", session);
        alive.tombstoned_at = None;
        let mut dead = he("d", session);
        dead.tombstoned_at = Some(1_700_000_500);
        b.append_stream(crate::from_str_core("action").expect("valid stream"), serde_json::to_value(alive).unwrap())
            .unwrap();
        b.append_stream(crate::from_str_core("action").expect("valid stream"), serde_json::to_value(dead).unwrap())
            .unwrap();
        let list = b.list_stream(crate::from_str_core("action").expect("valid stream"), session, 10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], "a");
    }

    #[test]
    fn unknown_stream_name_is_rejected() {
        let b = InMemoryBackend::new();
        let r = b.append_stream("not-a-stream", serde_json::json!({}));
        assert!(r.is_err());
    }
}
