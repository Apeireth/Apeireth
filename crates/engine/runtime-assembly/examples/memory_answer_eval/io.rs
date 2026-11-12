//! Append-only evaluation artifacts in an explicitly selected output directory.
//!
//! This example uses standard-library file I/O to avoid adding a dependency.
//! Files are created exclusively; it never deletes or reuses an existing run.
#![allow(
    clippy::disallowed_methods,
    reason = "isolated example artifacts use std-only exclusive creation; no new file dependency"
)]

use std::io::Write;
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

pub type EvalResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}

pub fn write_new(path: &Path, value: &impl Serialize) -> EvalResult<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

pub fn read<T: serde::de::DeserializeOwned>(path: &Path) -> EvalResult<T> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

pub fn create_run(path: &Path) -> EvalResult<()> {
    std::fs::create_dir(path)?;
    std::fs::create_dir(path.join("calls"))?;
    Ok(())
}

pub fn prepare_memory_root(db: &Path) -> EvalResult<std::path::PathBuf> {
    let root = db.with_extension("isolated-memory-data");
    std::fs::create_dir_all(root.join("wiki"))?;
    Ok(root)
}

pub fn git_output(argument: &str) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", argument])
        .output()
        .ok()
        .filter(|r| r.status.success())
        .and_then(|r| String::from_utf8(r.stdout).ok())
        .map(|s| s.trim().into())
}

pub fn dirty_diff_hash() -> Option<String> {
    std::process::Command::new("git")
        .args(["diff", "--binary", "HEAD"])
        .output()
        .ok()
        .filter(|r| r.status.success())
        .map(|r| hash(r.stdout))
}
