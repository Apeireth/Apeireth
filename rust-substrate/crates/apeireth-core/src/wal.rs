//! WAL — Write-Ahead Log (借鉴 DeltaMemory WAL 设计)
//!
//! 主人 14:52 "最高深度" → 持久化 + crash recovery 是基础
//! 主人 12:14 "中央 AI 是永恒身份" → 不能崩
//!
//! 借鉴:
//! - DeltaMemory: CRC32 checksum + WAL replay
//! - LSM-tree: Write-Ahead Log → MemTable → SSTable

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Checksum mismatch — corrupted entry")]
    ChecksumMismatch,
}

/// WAL entry — 一次写入操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    pub sequence: u64,
    pub operation: WalOperation,
    pub timestamp: DateTime<Utc>,
    /// CRC32 of operation payload
    pub checksum: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalOperation {
    AppendEpisode { eid: String, actor: String, content: String },
    AddNote { nid: String, topic: String, claim: String, confidence: f64 },
    UpdateIdentity { field: String, value: String },
    Forget { target_id: String, reason: String },
    Reconsolidate { note_id: String, path: String, delta: f64 },
    TierTransition { target_id: String, from: String, to: String },
}

/// WAL — Write-Ahead Log (DeltaMemory 借鉴)
pub struct Wal {
    file: File,
    sequence: u64,
    path: std::path::PathBuf,
}

impl Wal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WalError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        let sequence = Self::recover_last_sequence(&path)?;
        Ok(Self { file, sequence, path })
    }

    /// Recover last sequence from existing WAL file
    fn recover_last_sequence(path: &Path) -> Result<u64, WalError> {
        if !path.exists() {
            return Ok(0);
        }
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut last_seq = 0u64;
        for line in reader.lines().map_while(|r| r.ok()) {
            if let Ok(entry) = serde_json::from_str::<WalEntry>(&line) {
                if entry.sequence > last_seq {
                    last_seq = entry.sequence;
                }
            }
            // 损坏 entry 跳过 (DeltaMemory: skip damaged entry)
        }
        Ok(last_seq)
    }

    /// 写一条 entry
    pub fn append(&mut self, operation: WalOperation) -> Result<u64, WalError> {
        self.sequence += 1;
        let payload = serde_json::to_string(&operation)?;
        let checksum = crc32fast::hash(payload.as_bytes());

        let entry = WalEntry {
            sequence: self.sequence,
            operation,
            timestamp: Utc::now(),
            checksum,
        };

        let line = serde_json::to_string(&entry)?;
        writeln!(self.file, "{}", line)?;
        self.file.flush()?;
        self.file.sync_all()?; // 强持久化 (DeltaMemory: fsync before ack)

        Ok(self.sequence)
    }

    /// Replay — crash recovery
    pub fn replay<F>(&self, mut handler: F) -> Result<u64, WalError>
    where
        F: FnMut(&WalEntry) -> Result<(), WalError>,
    {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut count = 0u64;
        for line in reader.lines().map_while(|r| r.ok()) {
            let entry: WalEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue, // skip corrupted
            };

            // 验证 checksum
            let payload = serde_json::to_string(&entry.operation)?;
            let computed = crc32fast::hash(payload.as_bytes());
            if computed != entry.checksum {
                return Err(WalError::ChecksumMismatch);
            }

            handler(&entry)?;
            count += 1;
        }
        Ok(count)
    }

    pub fn current_sequence(&self) -> u64 {
        self.sequence
    }
}

// CRC32 — 借 crate
// 备注: 实际 Cargo.toml 需加 crc32fast = "1.3"

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wal_append_and_replay() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.wal");
        let mut wal = Wal::open(&path).unwrap();

        let s1 = wal.append(WalOperation::AppendEpisode {
            eid: "ep1".to_string(),
            actor: "master".to_string(),
            content: "test".to_string(),
        }).unwrap();
        assert_eq!(s1, 1);

        let s2 = wal.append(WalOperation::AddNote {
            nid: "n1".to_string(),
            topic: "t".to_string(),
            claim: "c".to_string(),
            confidence: 0.5,
        }).unwrap();
        assert_eq!(s2, 2);
    }
}