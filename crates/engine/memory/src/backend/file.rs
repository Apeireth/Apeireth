//! File MemoryBackend — JSON Lines append-only。
//!
//! **0 装 PASS（v2.0 关键标注）**：
//! - 文件**明文 JSON**（无加密）
//! - 无 keyring 集成
//! - 无压缩
//! - 无并发保护（assume single writer per backend instance；多 writer 需外加文件锁）
//!
//! **v2.1 路线**（per `v2-unabsorbed-features.md` §A4 + `scene-d-v2-plan.md`）：
//! - keyring 加密（passphrase from `apeireth_credentials::KeyringSelector`）
//! - 文件锁（`fs2` crate）支持多 writer
//! - schema versioning（文件头 magic + 版本号）
//!
//! **文件布局**（每个后端实例一个 root directory）：
//!
//! ```text
//! <root>/
//!   episodes.jsonl         # 一行一个 Episode
//!   streams/
//!     thought.jsonl        # 6 流各一个文件
//!     proposal.jsonl
//!     action.jsonl
//!     relation.jsonl
//!     evolution.jsonl
//!     reflection.jsonl
//!   index/
//!     episodes/<id>.id     # 空文件作为 id index（占位，未来可加更多索引）
//! ```
//!
//! **append-only 保证**：所有写入用 `OpenOptions::append`；从不修改已写行。
//! tombstoned 条目保留在文件中（与 SQLite 行为一致），列出时过滤。

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use apeireth_core::Episode;

use crate::append_only::HistoryEntry;
use crate::MemoryResult;

use super::{BackendKind, MemoryBackend};

/// 文件后端（JSON Lines，明文 append-only）。
pub struct FileBackend {
    root: PathBuf,
    /// episode 写入的全局互斥（保证单 writer 内 append 不交错）
    episode_write_lock: Mutex<()>,
    /// stream 写入的全局互斥（同上，单 writer 简化）
    stream_write_lock: Mutex<()>,
}

impl FileBackend {
    /// 在指定 root directory 创建后端。
    /// root 目录如不存在会被创建。
    pub fn new(root: impl AsRef<Path>) -> MemoryResult<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir_all(root.join("streams"))?;
        std::fs::create_dir_all(root.join("index").join("episodes"))?;
        Ok(Self {
            root,
            episode_write_lock: Mutex::new(()),
            stream_write_lock: Mutex::new(()),
        })
    }

    /// root 路径
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn episodes_path(&self) -> PathBuf {
        self.root.join("episodes.jsonl")
    }

    fn stream_path(&self, stream_name: &str) -> PathBuf {
        self.root.join("streams").join(format!("{stream_name}.jsonl"))
    }
}

impl MemoryBackend for FileBackend {
    fn name(&self) -> &'static str {
        "file"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::File
    }

    fn put_episode(&self, ep: &Episode) -> MemoryResult<()> {
        // append-only：先扫文件确认 id 不重复
        if self.episode_exists(&ep.id)? {
            return Err(crate::MemoryError::Invalid(format!(
                "episode id already exists: {}",
                ep.id
            )));
        }
        let _guard = self.episode_write_lock.lock().expect("FileBackend poisoned");
        let line = serde_json::to_string(ep)?;
        let path = self.episodes_path();
        let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(f, "{line}")?;
        f.sync_all()?;
        // id 索引（占位 file，未来可加更多索引）
        let idx_path = self.root.join("index").join("episodes").join(format!("{}.id", ep.id));
        std::fs::write(idx_path, b"")?;
        Ok(())
    }

    fn get_episode(&self, id: &str) -> MemoryResult<Option<Episode>> {
        let path = self.episodes_path();
        if !path.exists() {
            return Ok(None);
        }
        let f = File::open(&path)?;
        for line in BufReader::new(f).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let ep: Episode = serde_json::from_str(&line)?;
            if ep.id == id {
                return Ok(Some(ep));
            }
        }
        Ok(None)
    }

    fn recent_episodes(&self, session_id: &str, n: usize) -> MemoryResult<Vec<Episode>> {
        let path = self.episodes_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let f = File::open(&path)?;
        let mut all: Vec<Episode> = Vec::new();
        for line in BufReader::new(f).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let ep: Episode = serde_json::from_str(&line)?;
            if ep.session_id == session_id {
                all.push(ep);
            }
        }
        all.sort_by_key(|e| e.timestamp);
        if all.len() > n {
            let skip = all.len() - n;
            all.drain(..skip);
        }
        Ok(all)
    }

    fn append_stream(&self, stream_name: &str, entry: serde_json::Value) -> MemoryResult<()> {
        // 0 装: stream_name 必须已是 6 个固定值之一 (caller 责任)
        // rc 阶段加 SchemaRegistry 验证
        let _ = serde_json::from_value::<HistoryEntry>(entry.clone())?; // schema check
        let _guard = self.stream_write_lock.lock().expect("FileBackend poisoned");
        let line = serde_json::to_string(&entry)?;
        let path = self.stream_path(stream_name);
        let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
        writeln!(f, "{line}")?;
        f.sync_all()?;
        Ok(())
    }

    fn list_stream(
        &self,
        stream_name: &str,
        session_id: &str,
        n: usize,
    ) -> MemoryResult<Vec<serde_json::Value>> {
        let path = self.stream_path(stream_name);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let f = File::open(&path)?;
        let mut alive: Vec<HistoryEntry> = Vec::new();
        for line in BufReader::new(f).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: HistoryEntry = serde_json::from_str(&line)?;
            if entry.tombstoned_at.is_some() {
                continue;
            }
            let matches = match &entry.session_id {
                None => true,
                Some(s) => s == session_id,
            };
            if matches {
                alive.push(entry);
            }
        }
        alive.sort_by_key(|e| e.created_at);
        if alive.len() > n {
            let skip = alive.len() - n;
            alive.drain(..skip);
        }
        // 序列化为 trait 要求的 JSON Value
        alive
            .into_iter()
            .map(|e| serde_json::to_value(e).map_err(crate::MemoryError::Json))
            .collect()
    }
}

impl FileBackend {
    fn episode_exists(&self, id: &str) -> MemoryResult<bool> {
        let path = self.episodes_path();
        if !path.exists() {
            return Ok(false);
        }
        let f = File::open(&path)?;
        for line in BufReader::new(f).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let ep: Episode = serde_json::from_str(&line)?;
            if ep.id == id {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::append_only::HistoryEntry;
    use tempfile::TempDir;

    fn fresh() -> (FileBackend, TempDir) {
        let dir = TempDir::new().expect("create tempdir");
        let backend = FileBackend::new(dir.path()).expect("create FileBackend");
        (backend, dir)
    }

    fn ep(id: &str, session: &str) -> Episode {
        Episode {
            id: id.to_string(),
            timestamp: 1_700_000_000,
            role: "user".to_string(),
            content: format!("content of {id}"),
            session_id: session.to_string(),
        }
    }

    fn he(id: &str, session: &str) -> HistoryEntry {
        HistoryEntry {
            id: id.to_string(),
            subject_id: "subj-1".to_string(),
            subject_rev: 1,
            session_id: Some(session.to_string()),
            created_at: 1_700_000_100,
            payload: serde_json::json!({"kind": "test"}),
            source: "test".to_string(),
            tags: vec!["unit".to_string()),
            tombstoned_at: None,
        }
    }

    #[test]
    fn name_and_kind() {
        let (b, _d) = fresh();
        assert_eq!(b.name(), "file");
        assert_eq!(b.kind(), BackendKind::File);
    }

    #[test]
    fn episode_roundtrip() {
        let (b, _d) = fresh();
        let e = ep("ep-1", "sess-1");
        b.put_episode(&e).unwrap();
        let got = b.get_episode("ep-1").unwrap().expect("episode exists");
        assert_eq!(got.id, e.id);
        assert_eq!(got.timestamp, e.timestamp);
        assert_eq!(got.content, e.content);
        assert_eq!(got.session_id, e.session_id);
        let recent = b.recent_episodes("sess-1", 10).unwrap();
        assert_eq!(recent.len(), 1);
    }

    #[test]
    fn append_only_rejects_duplicate_episode_id() {
        let (b, _d) = fresh();
        b.put_episode(&ep("dup", "s")).unwrap();
        let r = b.put_episode(&ep("dup", "s"));
        assert!(r.is_err());
    }

    #[test]
    fn stream_roundtrip() {
        let (b, _d) = fresh();
        let session = "s1";
        b.append_stream("thought", serde_json::to_value(he("t1", session)).unwrap())
            .unwrap();
        b.append_stream("thought", serde_json::to_value(he("t2", session)).unwrap())
            .unwrap();
        let list = b.list_stream("thought", session, 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["id"], "t1");
        assert_eq!(list[1]["id"], "t2");
    }

    #[test]
    fn list_stream_filters_tombstoned() {
        let (b, _d) = fresh();
        let session = "s";
        let mut alive = he("a", session);
        alive.tombstoned_at = None;
        let mut dead = he("d", session);
        dead.tombstoned_at = Some(1_700_000_500);
        b.append_stream("action", serde_json::to_value(alive).unwrap())
            .unwrap();
        b.append_stream("action", serde_json::to_value(dead).unwrap())
            .unwrap();
        let list = b.list_stream("action", session, 10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], "a");
    }

    #[test]
    fn persistence_across_instances() {
        let dir = TempDir::new().expect("create tempdir");
        {
            let b = FileBackend::new(dir.path()).unwrap();
            b.put_episode(&ep("persist-1", "s")).unwrap();
            b.append_stream("action", serde_json::to_value(he("p-1", "s")).unwrap())
                .unwrap();
        }
        let b2 = FileBackend::new(dir.path()).unwrap();
        let got = b2
            .get_episode("persist-1")
            .unwrap()
            .expect("persisted episode");
        assert_eq!(got.id, "persist-1");
        assert_eq!(got.session_id, "s");
        let list = b2.list_stream("action", "s", 10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], "p-1");
    }
}
