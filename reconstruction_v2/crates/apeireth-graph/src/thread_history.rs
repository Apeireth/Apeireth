//! Thread-based checkpoint history (LangGraph-style).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::checkpoint::Checkpoint;

/// A thread's checkpoint history (per session/thread).
#[derive(Debug, Default, Clone)]
pub struct ThreadHistory {
    pub thread_id: String,
    pub checkpoints: Vec<Checkpoint>,
}

impl ThreadHistory {
    pub fn new(thread_id: impl Into<String>) -> Self {
        Self { thread_id: thread_id.into(), checkpoints: Vec::new() }
    }

    pub fn append(&mut self, cp: Checkpoint) { self.checkpoints.push(cp); }
    pub fn latest(&self) -> Option<&Checkpoint> { self.checkpoints.last() }
    pub fn len(&self) -> usize { self.checkpoints.len() }
    pub fn is_empty(&self) -> bool { self.checkpoints.is_empty() }
}

/// Per-thread checkpoint store.
#[derive(Default, Clone)]
pub struct ThreadCheckpointStore {
    inner: Arc<RwLock<HashMap<String, ThreadHistory>>>,
}

impl ThreadCheckpointStore {
    pub fn new() -> Self { Self::default() }

    pub fn save(&self, thread_id: impl Into<String>, cp: Checkpoint) {
        let id = thread_id.into();
        let mut guard = self.inner.write().unwrap();
        let entry = guard.entry(id.clone()).or_insert_with(|| ThreadHistory::new(id.clone()));
        entry.thread_id = id;
        entry.append(cp);
    }

    pub fn get(&self, thread_id: &str) -> Option<ThreadHistory> {
        self.inner.read().unwrap().get(thread_id).cloned()
    }

    pub fn list_threads(&self) -> Vec<String> {
        self.inner.read().unwrap().keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::State;

    #[tokio::test]
    async fn thread_history_append() {
        let cp1 = Checkpoint::new(vec!["a".into()], State::new()).unwrap();
        let mut h = ThreadHistory::new("t1");
        h.append(cp1);
        assert_eq!(h.len(), 1);
        assert!(h.latest().is_some());
        assert!(!h.is_empty());
    }

    #[tokio::test]
    async fn thread_store_save_get() {
        let store = ThreadCheckpointStore::new();
        let cp = Checkpoint::new(vec![], State::new()).unwrap();
        store.save("t1", cp);
        let h = store.get("t1").unwrap();
        assert_eq!(h.thread_id, "t1");
        assert_eq!(h.len(), 1);
        assert!(!store.list_threads().is_empty());
    }
}
