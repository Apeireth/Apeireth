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
    /// Stable identifier suitable for a file name.
    pub id: String,
    /// Schema version for forward-compatible readers.
    pub version: u32,
    /// Creation time in milliseconds since the Unix epoch.
    pub created_at_unix_ms: u128,
    /// Nodes present when this snapshot was taken.
    pub graph_nodes: Vec<NodeId>,
    /// Captured shared state.
    pub state: State,
}

impl Checkpoint {
    pub(crate) fn new(graph_nodes: Vec<NodeId>, state: State) -> Result<Self> {
        let created_at_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| GraphError::Clock(error.to_string()))?
            .as_millis();
        let sequence = CHECKPOINT_SEQUENCE.fetch_add(1, Ordering::Relaxed);

        Ok(Self {
            id: format!("checkpoint-{created_at_unix_ms}-{sequence}"),
            version: 1,
            created_at_unix_ms,
            graph_nodes,
            state,
        })
    }

    /// Writes this checkpoint as pretty JSON.
    ///
    /// Existing files are rejected rather than overwritten, preventing accidental
    /// checkpoint loss.
    pub async fn write_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            tokio::fs::create_dir_all(parent).await?;
        }

        let bytes = serde_json::to_vec_pretty(self)?;
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = options.open(path).await?;

        use tokio::io::AsyncWriteExt;
        file.write_all(&bytes).await?;
        file.sync_all().await?;
        Ok(())
    }

    /// Reads and validates a checkpoint JSON file.
    pub async fn read_from(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        let checkpoint: Self = serde_json::from_slice(&bytes)?;
        if checkpoint.version != 1 {
            return Err(GraphError::UnsupportedCheckpointVersion(checkpoint.version));
        }
        Ok(checkpoint)
    }
}

/// Directory-backed checkpoint storage.
#[derive(Debug, Clone)]
pub struct CheckpointStore {
    directory: PathBuf,
}

impl CheckpointStore {
    /// Creates a store rooted at `directory`.
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    /// Persists a checkpoint and returns its path.
    pub async fn save(&self, checkpoint: &Checkpoint) -> Result<PathBuf> {
        let path = self.directory.join(format!("{}.json", checkpoint.id));
        checkpoint.write_to(&path).await?;
        Ok(path)
    }

    /// Loads a checkpoint by ID.
    pub async fn load(&self, checkpoint_id: &str) -> Result<Checkpoint> {
        validate_checkpoint_id(checkpoint_id)?;
        Checkpoint::read_from(self.directory.join(format!("{checkpoint_id}.json"))).await
    }
}

fn validate_checkpoint_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(GraphError::InvalidCheckpointId(id.to_owned()));
    }
    Ok(())
}
