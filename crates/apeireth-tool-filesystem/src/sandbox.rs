//! Real sandbox (canonicalize + allowed roots whitelist).
//!
//! Per VCP gap analysis §2.1: VCP ALLOWED_DIRECTORIES is string filter,
//! bypassable via symlink. We use std::fs::canonicalize to resolve symlinks
//! + path traversal, then check whitelist.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("path not found: `{0}`")]
    PathNotFound(PathBuf),
    #[error("path outside allowed roots: `{path}`")]
    OutsideAllowedRoots { path: PathBuf },
    #[error("symlink target not in allowed roots: `{target}`")]
    SymlinkEscape { target: PathBuf },
    #[error("io error: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("task join error: `{0}`")]
    Join(#[from] tokio::task::JoinError),
}

#[derive(Debug, Clone)]
pub struct SandboxPolicy {
    pub allowed_roots: Vec<PathBuf>,
    pub follow_symlinks: bool,
}

impl SandboxPolicy {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        Self { allowed_roots, follow_symlinks: false }
    }
}

#[derive(Debug, Clone)]
pub struct Sandbox {
    policy: SandboxPolicy,
}

impl Sandbox {
    pub fn new(policy: SandboxPolicy) -> Self {
        Self { policy }
    }
    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    pub async fn resolve(&self, path: &Path) -> Result<PathBuf, SandboxError> {
        let p = path.to_path_buf();
        let policy = self.policy.clone();
        let canonical = tokio::task::spawn_blocking(move || std::fs::canonicalize(&p))
            .await
            .map_err(SandboxError::Join)?
            .map_err(SandboxError::Io)?;

        // Canonicalize each allowed root too, then compare on canonical form
        // (Windows: canonicalize adds `\\?\` UNC prefix which breaks naive
        // `starts_with` between non-canonical allowed_roots and canonical
        // target paths).
        let canon_target = canonical.clone();
        let allowed_canonicals: Vec<PathBuf> = policy
            .allowed_roots
            .iter()
            .filter_map(|r| std::fs::canonicalize(r).ok())
            .collect();

        for allowed in &allowed_canonicals {
            if canon_target.starts_with(allowed) {
                return Ok(canonical);
            }
        }
        Err(SandboxError::OutsideAllowedRoots { path: canonical })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn resolve_normal_path_works() {
        let sandbox = Sandbox::new(SandboxPolicy::new(vec![std::env::temp_dir()]));
        let result = sandbox.resolve(&std::env::temp_dir()).await;
        assert!(result.is_ok(), "normal path should resolve: `{result:?}`");
    }

    #[tokio::test]
    async fn resolve_nonexistent_path_errors() {
        let sandbox = Sandbox::new(SandboxPolicy::new(vec![std::env::temp_dir()]));
        let bad = std::env::temp_dir().join("nonexistent_apeireth_test_xyz");
        let result = sandbox.resolve(&bad).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn resolve_outside_allowed_roots_errors() {
        let sandbox = Sandbox::new(SandboxPolicy::new(vec![PathBuf::from("/tmp")]));
        if cfg!(unix) {
            let result = sandbox.resolve(Path::new("/etc/passwd")).await;
            assert!(result.is_err());
        }
    }
}
