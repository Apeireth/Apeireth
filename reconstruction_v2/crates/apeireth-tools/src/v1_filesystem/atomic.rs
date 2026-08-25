//! Atomic write (tmp + rename).

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AtomicWriteError {
    #[error("io error: `{0}`")]
    Io(#[from] std::io::Error),
    #[error("parent dir not found: `{0}`")]
    ParentNotFound(PathBuf),
    #[error("rename failed: `{0}`")]
    RenameFailed(String),
    #[error("task join error: `{0}`")]
    Join(#[from] tokio::task::JoinError),
}

pub async fn atomic_write(path: &Path, content: &[u8]) -> Result<(), AtomicWriteError> {
    let parent = path
        .parent()
        .ok_or_else(|| AtomicWriteError::ParentNotFound(path.to_path_buf()))?
        .to_path_buf();
    let file_name = path
        .file_name()
        .ok_or_else(|| AtomicWriteError::ParentNotFound(path.to_path_buf()))?
        .to_owned();
    let target = path.to_path_buf();
    let bytes = content.to_vec();
    let tmp_name = format!("apeireth_atomic_{}.tmp", file_name.to_string_lossy());

    let inner_parent = parent.clone();
    let inner_tmp_name = tmp_name.clone();

    let res: Result<(), AtomicWriteError> =
        tokio::task::spawn_blocking(move || -> Result<(), AtomicWriteError> {
            use std::io::Write;
            let tmp_path = inner_parent.join(&inner_tmp_name);
            {
                let mut f = std::fs::File::create(&tmp_path)?;
                f.write_all(&bytes)?;
                f.sync_all()?;
            }
            std::fs::rename(&tmp_path, &target)
                .map_err(|e| AtomicWriteError::RenameFailed(e.to_string()))?;
            Ok(())
        })
        .await?;

    if res.is_err() {
        let _ = std::fs::remove_file(parent.join(&tmp_name));
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn atomic_write_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("test.txt");
        atomic_write(&target, b"hello atomic").await.unwrap();
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "hello atomic");
    }

    #[tokio::test]
    async fn atomic_write_overwrites_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("test.txt");
        std::fs::write(&target, "old").unwrap();
        atomic_write(&target, b"new").await.unwrap();
        let content = std::fs::read_to_string(&target).unwrap();
        assert_eq!(content, "new");
    }
}
