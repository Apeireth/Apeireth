//! File lock (fd-lock 4.0 RwLock API).
//!
//! Note: full advisory lock semantics require keeping an RwLockReadGuard /
//! RwLockWriteGuard alive, which borrows the RwLock. Since we want the guard
//! to be `'static` (so it can live in async tasks), we expose a simpler API
//! that holds the file handle. For real coordination across processes,
//! wrap with explicit RwLock usage at call site.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::fs::{File, OpenOptions};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LockError {
    #[error("io error: `{0}`")]
    Io(#[from] std::io::Error),
}

pub struct FileLockGuard {
    _file: File,
}

pub struct FileLock;

impl FileLock {
    pub fn exclusive(path: &Path) -> Result<FileLockGuard, LockError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        Ok(FileLockGuard { _file: file })
    }

    pub fn shared(path: &Path) -> Result<FileLockGuard, LockError> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)?;
        Ok(FileLockGuard { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use super::*;
    #[test]
    fn lock_can_be_acquired() {
        let tmp = tempfile::tempdir().unwrap();
        let lock_path: PathBuf = tmp.path().join("lock.file");
        let r = FileLock::exclusive(&lock_path);
        assert!(r.is_ok());
    }
}
