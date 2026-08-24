//! Serializable graph checkpoints and file persistence.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::{GraphError, NodeId, Result, State};

static CHECKPOINT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A versioned snapshot of graph state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub version: u32,
    pub created_at_unix_ms: u128,
    pub graph_nodes: Vec<NodeId>,
    pub state: State,
}

impl Checkpoint {
    pub(crate) fn new(graph_nodes: Vec<NodeId>, state: State) -> Result<Self> {
        let created_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| GraphError::Clock(e.to_string()))?
            .as_millis();
        let seq = CHECKPOINT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            id: format!("checkpoint-{created_at_unix_ms}-{seq}"),
            version: 1,
            created_at_unix_ms,
            graph_nodes,
            state,
        })
    }

    pub async fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            tokio::fs::create_dir_all(parent).await?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        let mut opts = tokio::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        let mut f = opts.open(path).await?;
        use tokio::io::AsyncWriteExt;
        f.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn read_from(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = tokio::fs::read(path.as_ref()).await?;
        let cp: Self = serde_json::from_slice(&bytes)?;
        if cp.version != 1 {
            return Err(GraphError::UnsupportedCheckpointVersion(cp.version));
        }
        Ok(cp)
    }
}

/// In-memory checkpoint store.
#[derive(Debug, Default)]
pub struct CheckpointStore {
    by_id: std::sync::RwLock<Vec<Checkpoint>>,
}

impl CheckpointStore {
    pub fn new() -> Self { Self::default() }
    pub async fn save(&self, cp: Checkpoint) -> NodeId {
        let id = cp.id.clone();
        self.by_id.write().unwrap().push(cp);
        id
    }
    pub async fn list(&self) -> Vec<Checkpoint> {
        self.by_id.read().unwrap().clone()
    }
    pub async fn latest(&self) -> Option<Checkpoint> {
        self.by_id.read().unwrap().last().cloned()
    }
    pub fn path_for(&self, id: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{id}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn checkpoint_roundtrip() {
        let cp = Checkpoint::new(vec!["a".into()], State::with("k", "v")).unwrap();
        let path = std::env::temp_dir().join(format!("{}.json", cp.id));
        cp.write_to(&path).await.unwrap();
        let restored = Checkpoint::read_from(&path).await.unwrap();
        tokio::fs::remove_file(path).await.ok();
        assert_eq!(restored.state, cp.state);
        assert_eq!(restored.version, 1);
    }

    #[tokio::test]
    async fn store_save_and_latest() {
        let store = CheckpointStore::new();
        let cp1 = Checkpoint::new(vec![], State::new()).unwrap();
        let cp2 = Checkpoint::new(vec![], State::with("k", "v")).unwrap();
        store.save(cp1).await;
        store.save(cp2).await;
        let latest = store.latest().await.unwrap();
        assert_eq!(latest.state.get("k"), Some(&serde_json::json!("v")));
        assert_eq!(store.list().await.len(), 2);
    }
}
