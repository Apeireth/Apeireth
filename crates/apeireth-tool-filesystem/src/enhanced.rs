//! EnhancedFileOps - R137 5-dimension extension entrypoint.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::path::{Path, PathBuf};
use apeireth_tools::{StdFileOps, FileOps};
use crate::{atomic, lock, sandbox};
use crate::sandbox::{Sandbox, SandboxPolicy};
use crate::atomic::AtomicWriteError;
use crate::lock::LockError;

#[async_trait::async_trait]
pub trait EnhancedFileOps: Send + Sync {
    async fn read_sandboxed(&self, path: &Path) -> Result<String, EnhancedError>;
    async fn write_atomic(&self, path: &Path, content: &[u8]) -> Result<(), EnhancedError>;
    async fn read_with_lock(&self, path: &Path) -> Result<(String, lock::FileLockGuard), EnhancedError>;
}

#[derive(Debug, thiserror::Error)]
pub enum EnhancedError {
    #[error("sandbox: `{0}`")]
    Sandbox(#[from] sandbox::SandboxError),
    #[error("atomic: `{0}`")]
    Atomic(#[from] AtomicWriteError),
    #[error("lock: `{0}`")]
    Lock(#[from] LockError),
    #[error("io: `{0}`")]
    Io(#[from] std::io::Error),
}

pub struct StdEnhancedFileOps {
    inner: StdFileOps,
    sandbox: Sandbox,
}

impl StdEnhancedFileOps {
    pub fn new(allowed_roots: Vec<PathBuf>) -> Self {
        Self {
            inner: StdFileOps::new(),
            sandbox: Sandbox::new(SandboxPolicy::new(allowed_roots)),
        }
    }
}

#[async_trait::async_trait]
impl EnhancedFileOps for StdEnhancedFileOps {
    async fn read_sandboxed(&self, path: &Path) -> Result<String, EnhancedError> {
        let canonical = self.sandbox.resolve(path).await?;
        let content = self.inner.read(&canonical).await
            .map_err(|e| EnhancedError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        Ok(content)
    }

    async fn write_atomic(&self, path: &Path, content: &[u8]) -> Result<(), EnhancedError> {
        // 写入允许"新建文件", 而 std::fs::canonicalize 对新文件在 Windows 上失败.
        // 改为解析父目录 (确保父在沙盒允许根) + 拼文件名 — 沙盒语义: 写权限 = 父目录允许根.
        let canonical_parent = self
            .sandbox
            .resolve(path.parent().unwrap_or(Path::new(".")))
            .await?;
        let file_name = path
            .file_name()
            .ok_or_else(|| EnhancedError::Atomic(AtomicWriteError::ParentNotFound(path.to_path_buf())))?;
        let target = canonical_parent.join(file_name);
        Ok(atomic::atomic_write(&target, content).await?)
    }

    async fn read_with_lock(&self, path: &Path) -> Result<(String, lock::FileLockGuard), EnhancedError> {
        let canonical = self.sandbox.resolve(path).await?;
        let guard = lock::FileLock::shared(&canonical)?;
        let content = self.inner.read(&canonical).await
            .map_err(|e| EnhancedError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        Ok((content, guard))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn enhanced_read_sandboxed_works() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "hi").unwrap();
        let ops = StdEnhancedFileOps::new(vec![tmp.path().to_path_buf()]);
        let r = ops.read_sandboxed(&tmp.path().join("a.txt")).await;
        assert!(r.is_ok(), "read_sandboxed failed: `{r:?}`");
        assert_eq!(r.unwrap(), "hi");
    }
}
