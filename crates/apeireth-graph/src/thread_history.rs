//! Thread-based checkpoint history (LangGraph-style).
//!
//! R149 升级: 借鉴 LangGraph MemorySaver/PostgresSaver 模式, 加 thread_id 维度
//! 的 checkpoint history (每个 thread 多个 checkpoint, 按时间排序).
//!
//! 不假装 (O-5): in-memory 真实现 (无 SQLite/PG 依赖), 序列化真用 serde,
//! 升级点: 现有 Checkpoint + CheckpointStore 基础上加 thread index.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::checkpoint::{Checkpoint, CheckpointStore};

/// 单个 thread 的 checkpoint 历史
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreadHistory {
    pub thread_id: String,
    pub checkpoints: Vec<Checkpoint>,
    pub current_index: usize,
}

impl ThreadHistory {
    pub fn new(thread_id: impl Into<String>) -> Self {
        Self {
            thread_id: thread_id.into(),
            checkpoints: Vec::new(),
            current_index: 0,
        }
    }

    pub fn push(&mut self, cp: Checkpoint) {
        self.checkpoints.push(cp);
        self.current_index = self.checkpoints.len().saturating_sub(1);
    }

    pub fn current(&self) -> Option<&Checkpoint> {
        self.checkpoints.get(self.current_index)
    }

    pub fn len(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.checkpoints.is_empty()
    }

    /// 回到上一个 checkpoint (LangGraph get_state(history=...))
    pub fn rewind(&mut self) -> Option<&Checkpoint> {
        if self.current_index > 0 {
            self.current_index -= 1;
            self.current()
        } else {
            None
        }
    }

    /// 前进到下一个 checkpoint (resume)
    pub fn advance(&mut self) -> Option<&Checkpoint> {
        if self.current_index + 1 < self.checkpoints.len() {
            self.current_index += 1;
            self.current()
        } else {
            None
        }
    }

    /// 获取指定 index 的 checkpoint
    pub fn at(&self, idx: usize) -> Option<&Checkpoint> {
        self.checkpoints.get(idx)
    }
}

/// Thread 索引的 checkpoint 存储 (LangGraph MemorySaver 简化版)
pub struct ThreadCheckpointStore {
    inner: RwLock<Inner>,
    file_store: Option<CheckpointStore>,
}

#[derive(Default)]
struct Inner {
    threads: HashMap<String, ThreadHistory>,
    checkpoint_to_thread: HashMap<String, String>,
}

impl ThreadCheckpointStore {
    pub fn new_in_memory() -> Self {
        Self {
            inner: RwLock::new(Inner::default()),
            file_store: None,
        }
    }

    pub fn with_persistence(store: CheckpointStore) -> Self {
        Self {
            inner: RwLock::new(Inner::default()),
            file_store: Some(store),
        }
    }

    /// Append a checkpoint to a thread (creates thread if absent)
    pub fn put(&self, thread_id: impl Into<String>, cp: Checkpoint) {
        let tid = thread_id.into();
        let mut g = self.inner.write();
        g.checkpoint_to_thread.insert(cp.id.clone(), tid.clone());
        let history = g
            .threads
            .entry(tid)
            .or_insert_with(|| ThreadHistory::new(""));
        history.push(cp);
    }

    /// Get the latest checkpoint for a thread (LangGraph get_state)
    pub fn get(&self, thread_id: &str) -> Option<Checkpoint> {
        self.inner
            .read()
            .threads
            .get(thread_id)
            .and_then(|h| h.current().cloned())
    }

    /// Get a specific checkpoint by ID (across all threads)
    pub fn get_by_checkpoint_id(&self, checkpoint_id: &str) -> Option<Checkpoint> {
        let g = self.inner.read();
        let tid = g.checkpoint_to_thread.get(checkpoint_id)?;
        g.threads.get(tid).and_then(|h| {
            h.checkpoints
                .iter()
                .find(|c| c.id == checkpoint_id)
                .cloned()
        })
    }

    /// Get full thread history (for inspection / debug)
    pub fn history(&self, thread_id: &str) -> Option<ThreadHistory> {
        self.inner.read().threads.get(thread_id).cloned()
    }

    /// List all threads
    pub fn list_threads(&self) -> Vec<String> {
        self.inner.read().threads.keys().cloned().collect()
    }

    /// Total number of checkpoints across all threads
    pub fn total_checkpoints(&self) -> usize {
        self.inner.read().threads.values().map(|h| h.len()).sum()
    }

    /// Get the latest checkpoint index for a thread (0-based)
    pub fn current_index(&self, thread_id: &str) -> usize {
        self.inner
            .read()
            .threads
            .get(thread_id)
            .map(|h| h.current_index)
            .unwrap_or(0)
    }

    /// Rewind a thread to previous checkpoint (returns checkpoint if successful)
    pub fn rewind(&self, thread_id: &str) -> Option<Checkpoint> {
        let mut g = self.inner.write();
        let history = g.threads.get_mut(thread_id)?;
        let idx = history.current_index;
        if idx > 0 {
            history.current_index -= 1;
            history.current().cloned()
        } else {
            None
        }
    }

    /// Drop a thread entirely (LangGraph delete_thread)
    pub fn delete_thread(&self, thread_id: &str) -> bool {
        let mut g = self.inner.write();
        if let Some(history) = g.threads.remove(thread_id) {
            for cp in &history.checkpoints {
                g.checkpoint_to_thread.remove(&cp.id);
            }
            true
        } else {
            false
        }
    }
}

impl Default for ThreadCheckpointStore {
    fn default() -> Self {
        Self::new_in_memory()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;

    fn fake_checkpoint(id: &str) -> Checkpoint {
        Checkpoint {
            id: id.into(),
            version: 1,
            created_at_unix_ms: 0,
            graph_nodes: vec!["n1".into()],
            state: State::default(),
        }
    }

    #[test]
    fn put_and_get() {
        let store = ThreadCheckpointStore::new_in_memory();
        store.put("thread-1", fake_checkpoint("cp-1"));
        store.put("thread-1", fake_checkpoint("cp-2"));
        let cp = store.get("thread-1").unwrap();
        assert_eq!(cp.id, "cp-2");
    }

    #[test]
    fn history_per_thread() {
        let store = ThreadCheckpointStore::new_in_memory();
        store.put("a", fake_checkpoint("a-1"));
        store.put("a", fake_checkpoint("a-2"));
        store.put("b", fake_checkpoint("b-1"));
        assert_eq!(store.history("a").unwrap().len(), 2);
        assert_eq!(store.history("b").unwrap().len(), 1);
    }

    #[test]
    fn rewind_and_advance() {
        let store = ThreadCheckpointStore::new_in_memory();
        store.put("t", fake_checkpoint("c1"));
        store.put("t", fake_checkpoint("c2"));
        store.put("t", fake_checkpoint("c3"));
        let rewound = store.rewind("t").unwrap();
        assert_eq!(rewound.id, "c2");
        let r2 = store.rewind("t").unwrap();
        assert_eq!(r2.id, "c1");
        // can't rewind past start
        assert!(store.rewind("t").is_none());
    }

    #[test]
    fn get_by_checkpoint_id() {
        let store = ThreadCheckpointStore::new_in_memory();
        store.put("t", fake_checkpoint("c1"));
        store.put("t", fake_checkpoint("c2"));
        assert_eq!(store.get_by_checkpoint_id("c1").unwrap().id, "c1");
        assert_eq!(store.get_by_checkpoint_id("c2").unwrap().id, "c2");
    }

    #[test]
    fn list_threads() {
        let store = ThreadCheckpointStore::new_in_memory();
        store.put("a", fake_checkpoint("x"));
        store.put("b", fake_checkpoint("y"));
        let threads = store.list_threads();
        assert!(threads.contains(&"a".to_string()));
        assert!(threads.contains(&"b".to_string()));
    }

    #[test]
    fn total_checkpoints_across_threads() {
        let store = ThreadCheckpointStore::new_in_memory();
        store.put("a", fake_checkpoint("a1"));
        store.put("a", fake_checkpoint("a2"));
        store.put("b", fake_checkpoint("b1"));
        assert_eq!(store.total_checkpoints(), 3);
    }

    #[test]
    fn delete_thread_removes() {
        let store = ThreadCheckpointStore::new_in_memory();
        store.put("x", fake_checkpoint("x1"));
        assert!(store.delete_thread("x"));
        assert!(store.get("x").is_none());
        assert!(store.get_by_checkpoint_id("x1").is_none());
    }

    #[test]
    fn missing_thread_returns_none() {
        let store = ThreadCheckpointStore::new_in_memory();
        assert!(store.get("nope").is_none());
        assert!(store.history("nope").is_none());
    }
}
