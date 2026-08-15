//! Agent workspace: per-Node file + memory + skill roots.
//!
//! Borrowed from OpenClaw: every Node has a working area where skills,
//! episodic memory, and transient state live. The workspace is rooted at
//! `<workspace_root>/<node_id>/<kind>/...` and the gateway refuses to mount
//! paths that escape the root (path traversal defense).

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
use crate::node::NodeKind;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace: path traversal rejected: `{0}`")]
    PathTraversal(String),
    #[error("workspace: missing root: `{0}`")]
    MissingRoot(String),
    #[error("workspace: io error: {0}")]
    Io(String),
}

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

/// Defensive path resolver: refuses any `..` component or absolute input.
pub fn safe_join(root: &Path, rel: &str) -> WorkspaceResult<PathBuf> {
    let p = Path::new(rel);
    for c in p.components() {
        if matches!(c, Component::ParentDir) {
            return Err(WorkspaceError::PathTraversal(rel.into()));
        }
        if matches!(c, Component::RootDir | Component::Prefix(_) | Component::CurDir) {
            // CurDir is harmless on its own but we reject it for consistency.
            // RootDir + Prefix indicate absolute paths which we must reject.
            return Err(WorkspaceError::PathTraversal(rel.into()));
        }
    }
    let _ = Component::RootDir; // ensure the symbol is in scope
    Ok(root.join(rel))
}

/// Per-Node workspace layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentWorkspace {
    pub root: PathBuf,
    pub node_id: Uuid,
    pub node_kind: NodeKind,
}

impl AgentWorkspace {
    pub fn new(root: impl Into<PathBuf>, node_id: Uuid, node_kind: NodeKind) -> Self {
        Self { root: root.into(), node_id, node_kind }
    }

    pub fn skills_dir(&self) -> WorkspaceResult<PathBuf> {
        let p = format!("{}/{}/skills", self.node_id, self.node_kind.as_str());
        safe_join(&self.root, &p)
    }

    pub fn memory_dir(&self) -> WorkspaceResult<PathBuf> {
        let p = format!("{}/{}/memory", self.node_id, self.node_kind.as_str());
        safe_join(&self.root, &p)
    }

    pub fn state_file(&self) -> WorkspaceResult<PathBuf> {
        let p = format!("{}/{}/state.json", self.node_id, self.node_kind.as_str());
        safe_join(&self.root, &p)
    }

    pub fn relative_in(&self, kind: WorkspaceSlot, rel: &str) -> WorkspaceResult<PathBuf> {
        let dir = match kind {
            WorkspaceSlot::Skills => self.skills_dir()?,
            WorkspaceSlot::Memory => self.memory_dir()?,
        };
        safe_join(&dir, rel)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceSlot {
    Skills,
    Memory,
}

impl WorkspaceSlot {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Memory => "memory",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn safe_join_relative_works() {
        let r = PathBuf::from("/tmp/ws");
        let p = safe_join(&r, "a/b/c.txt").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/ws/a/b/c.txt"));
    }

    #[test]
    fn safe_join_rejects_parent_dir() {
        let r = PathBuf::from("/tmp/ws");
        assert!(matches!(safe_join(&r, "../etc"), Err(WorkspaceError::PathTraversal(_))));
        assert!(matches!(safe_join(&r, "a/../../b"), Err(WorkspaceError::PathTraversal(_))));
    }

    #[test]
    fn safe_join_rejects_absolute() {
        let r = PathBuf::from("/tmp/ws");
        assert!(matches!(safe_join(&r, "/etc/passwd"), Err(WorkspaceError::PathTraversal(_))));
    }

    #[test]
    fn workspace_paths_use_node_id_and_kind() {
        let ws = AgentWorkspace::new("/tmp/ws", Uuid::nil(), NodeKind::Tui);
        let s = ws.skills_dir().unwrap();
        assert!(s.to_string_lossy().contains("tui"));
        assert!(s.to_string_lossy().contains("skills"));
        let m = ws.memory_dir().unwrap();
        assert!(m.to_string_lossy().contains("memory"));
        let st = ws.state_file().unwrap();
        assert!(st.to_string_lossy().ends_with("state.json"));
    }

    #[test]
    fn workspace_relative_in_traversal_blocked() {
        let ws = AgentWorkspace::new("/tmp/ws", Uuid::nil(), NodeKind::Http);
        assert!(matches!(
            ws.relative_in(WorkspaceSlot::Skills, "../escape"),
            Err(WorkspaceError::PathTraversal(_))
        ));
    }

    #[test]
    fn workspace_slot_str() {
        assert_eq!(WorkspaceSlot::Skills.as_str(), "skills");
        assert_eq!(WorkspaceSlot::Memory.as_str(), "memory");
    }
}
