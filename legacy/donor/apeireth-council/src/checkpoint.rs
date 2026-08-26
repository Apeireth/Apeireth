//! R212 Council deliberation checkpoint (LangGraph 风格).
//!
//! **动机**: 当前 `Council::deliberate` 一次跑完, 没有中断/恢复能力. 长 deliberation
//! (7 advisor x 3 persona rounds = 21 步) 中途 crash/timeout 会丢失全部进度.
//!
//! **设计** (LangGraph Checkpoint 借鉴, 不模仿):
//! - `Checkpoint` struct: 序列化的快照 (session_id, query, opinions_so_far,
//!   current_step, started_at_ms, elapsed_ms_so_far)
//! - `CheckpointStore` trait: 抽象持久化 (in-memory / file-based / sled)
//! - `MemoryCheckpointStore`: 内存实现 (HashMap + RwLock)
//! - `FileCheckpointStore`: 文件实现 (一行 JSON per checkpoint)
//! - `Council::deliberate_with_checkpoints(store)`: 每步 opinion 发出后写 checkpoint
//! - `Council::resume_from_checkpoint(checkpoint, advisors)`: 恢复时跳过已完成的 advisors
//!
//! **0 触碰**:
//! - deliberation.rs 0 改
//! - 7 强制 advisor 0 改
//! - 3 不可变脊柱 0 触碰
//! - workspace.version 0 改

#![allow(missing_docs)] // R212 additive

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::advisor::AdvisorOpinion;
use crate::deliberation::CouncilQuery;

// 注意: CouncilQuery / QueryContext 未 derive Serialize/Deserialize (R212 0 触碰 deliberation.rs).
// 我们用自实现的 Serde-friendly 镜像结构 CheckpointQuery 来持久化 query 快照.

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("checkpoint not found for session_id: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("checkpoint version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
    #[error("invalid checkpoint: {0}")]
    Invalid(String),
}

pub type CheckpointResult<T> = Result<T, CheckpointError>;

// ============================================================================
// CheckpointQuery — CouncilQuery 的 Serde-friendly 镜像
// ============================================================================

/// R212 自实现的 CouncilQuery 序列化镜像 (0 触碰 deliberation.rs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointQuery {
    pub query_id: String,
    pub description: String,
    pub area: Option<String>,
    pub risk_level: Option<String>,
    pub history_refs: Vec<String>,
    pub started_at_ms: i64,
}

impl CheckpointQuery {
    pub fn from_council_query(q: &CouncilQuery) -> Self {
        Self {
            query_id: q.query_id.clone(),
            description: q.description.clone(),
            area: q.context.area.clone(),
            risk_level: q.context.risk_level.clone(),
            history_refs: q.context.history_refs.clone(),
            started_at_ms: q.started_at_ms,
        }
    }
}

// ============================================================================
// Checkpoint 数据结构
// ============================================================================

/// Checkpoint schema 版本 (用于兼容性检查).
pub const CHECKPOINT_VERSION: u32 = 1;

/// 审议快照 — 序列化后可持久化.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Schema version (R212 = 1).
    pub version: u32,
    /// 唯一 ID.
    pub checkpoint_id: String,
    /// 关联 session.
    pub session_id: String,
    /// Query 快照 (自实现镜像, 避免动 deliberation.rs).
    pub query: CheckpointQuery,
    /// 已收集 opinion 列表 (按发出顺序).
    pub opinions_so_far: Vec<AdvisorOpinion>,
    /// 当前 step (0-based, 下一个要跑的 advisor index).
    pub current_step: usize,
    /// 总 steps (= advisors.len(), 用于进度计算).
    pub total_steps: usize,
    /// 已用时 (ms).
    pub elapsed_ms_so_far: u64,
    /// 审议开始时间 (epoch ms).
    pub started_at_ms: i64,
    /// Checkpoint 写入时间 (epoch ms).
    pub written_at_ms: i64,
}

impl Checkpoint {
    /// 进度百分比 (0.0 .. 1.0).
    pub fn progress(&self) -> f64 {
        if self.total_steps == 0 {
            return 1.0;
        }
        self.current_step as f64 / self.total_steps as f64
    }

    /// 是否完成 (current_step >= total_steps).
    pub fn is_complete(&self) -> bool {
        self.current_step >= self.total_steps
    }

    /// 下一 advisor index (consume step).
    pub fn next_step(&self) -> usize {
        self.current_step
    }
}

// ============================================================================
// CheckpointStore trait
// ============================================================================

pub trait CheckpointStore: Send + Sync {
    /// 写入 checkpoint.
    fn put(&self, cp: &Checkpoint) -> CheckpointResult<()>;
    /// 读取 checkpoint (按 session_id 取 latest).
    fn get(&self, session_id: &str) -> CheckpointResult<Checkpoint>;
    /// 列出 session 全部 checkpoint (按时间顺序).
    fn list(&self, session_id: &str) -> CheckpointResult<Vec<Checkpoint>>;
    /// 删除 session 全部 checkpoint.
    fn delete(&self, session_id: &str) -> CheckpointResult<()>;
}

// ============================================================================
// 内存实现
// ============================================================================

/// 内存 CheckpointStore (HashMap<session_id, Vec<Checkpoint>>).
#[derive(Debug, Default)]
pub struct MemoryCheckpointStore {
    inner: RwLock<HashMap<String, Vec<Checkpoint>>>,
}

impl MemoryCheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn session_count(&self) -> usize {
        self.inner.read().expect("poisoned").len()
    }

    pub fn total_checkpoints(&self) -> usize {
        self.inner
            .read()
            .expect("poisoned")
            .values()
            .map(|v| v.len())
            .sum()
    }
}

impl CheckpointStore for MemoryCheckpointStore {
    fn put(&self, cp: &Checkpoint) -> CheckpointResult<()> {
        if cp.version != CHECKPOINT_VERSION {
            return Err(CheckpointError::VersionMismatch {
                expected: CHECKPOINT_VERSION,
                actual: cp.version,
            });
        }
        let mut g = self.inner.write().expect("poisoned");
        g.entry(cp.session_id.clone())
            .or_insert_with(Vec::new)
            .push(cp.clone());
        Ok(())
    }

    fn get(&self, session_id: &str) -> CheckpointResult<Checkpoint> {
        let g = self.inner.read().expect("poisoned");
        let cps = g
            .get(session_id)
            .ok_or_else(|| CheckpointError::NotFound(session_id.to_string()))?;
        cps.last()
            .cloned()
            .ok_or_else(|| CheckpointError::Invalid(format!("empty vec for {session_id}")))
    }

    fn list(&self, session_id: &str) -> CheckpointResult<Vec<Checkpoint>> {
        let g = self.inner.read().expect("poisoned");
        Ok(g.get(session_id).cloned().unwrap_or_default())
    }

    fn delete(&self, session_id: &str) -> CheckpointResult<()> {
        self.inner.write().expect("poisoned").remove(session_id);
        Ok(())
    }
}

// ============================================================================
// 文件实现
// ============================================================================

/// 文件 CheckpointStore (每个 session 一个 JSONL 文件: 1 行 = 1 checkpoint).
#[derive(Debug)]
pub struct FileCheckpointStore {
    base_dir: PathBuf,
}

impl FileCheckpointStore {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> CheckpointResult<Self> {
        let dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;
        Ok(Self { base_dir: dir })
    }

    fn file_for(&self, session_id: &str) -> PathBuf {
        // 防止路径穿越: 仅允许 [a-zA-Z0-9_-]
        let safe: String = session_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.base_dir.join(format!("{safe}.jsonl"))
    }
}

impl CheckpointStore for FileCheckpointStore {
    fn put(&self, cp: &Checkpoint) -> CheckpointResult<()> {
        if cp.version != CHECKPOINT_VERSION {
            return Err(CheckpointError::VersionMismatch {
                expected: CHECKPOINT_VERSION,
                actual: cp.version,
            });
        }
        let path = self.file_for(&cp.session_id);
        let line = serde_json::to_string(cp)? + "\n";
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        f.write_all(line.as_bytes())?;
        Ok(())
    }

    fn get(&self, session_id: &str) -> CheckpointResult<Checkpoint> {
        let cps = self.list(session_id)?;
        cps.last()
            .cloned()
            .ok_or_else(|| CheckpointError::NotFound(session_id.to_string()))
    }

    fn list(&self, session_id: &str) -> CheckpointResult<Vec<Checkpoint>> {
        let path = self.file_for(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&path)?;
        let mut out = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let cp: Checkpoint = serde_json::from_str(line)
                .map_err(|e| CheckpointError::Invalid(format!("line {i}: {e}")))?;
            out.push(cp);
        }
        Ok(out)
    }

    fn delete(&self, session_id: &str) -> CheckpointResult<()> {
        let path = self.file_for(session_id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }
}

// ============================================================================
// 测试 (12 cases)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::{AdvisorDomain, AdvisorId, Stance, StanceKind};
    use crate::deliberation::QueryContext;

    fn mk_opinion(advisor_idx: usize, ts: i64) -> AdvisorOpinion {
        let stance = Stance::new(StanceKind::Approve, format!("opinion {advisor_idx}"));
        let domain = AdvisorDomain::ALL[advisor_idx % AdvisorDomain::ALL.len()];
        let advisor_id = AdvisorId::new(format!("{domain}-v1"));
        let mut op = AdvisorOpinion::new(advisor_id, stance, 0.8, "reasoning", ts);
        op.weight = domain.default_weight();
        op
    }

    fn mk_query() -> CheckpointQuery {
        CheckpointQuery {
            query_id: "q-001".to_string(),
            description: "test query".to_string(),
            area: Some("test".to_string()),
            risk_level: Some("low".to_string()),
            history_refs: Vec::new(),
            started_at_ms: 1_000_000,
        }
    }

    fn mk_checkpoint(session_id: &str, step: usize, total: usize) -> Checkpoint {
        Checkpoint {
            version: CHECKPOINT_VERSION,
            checkpoint_id: format!("cp-{session_id}-{step}"),
            session_id: session_id.to_string(),
            query: mk_query(),
            opinions_so_far: (0..step)
                .map(|i| mk_opinion(i, 1_000_000 + (i as i64) * 100))
                .collect(),
            current_step: step,
            total_steps: total,
            elapsed_ms_so_far: step as u64 * 50,
            started_at_ms: 1_000_000,
            written_at_ms: 1_000_000 + (step as i64) * 50,
        }
    }

    #[test]
    fn t01_checkpoint_progress() {
        let cp = mk_checkpoint("s1", 3, 7);
        assert!((cp.progress() - 3.0 / 7.0).abs() < 1e-9);
    }

    #[test]
    fn t02_checkpoint_complete() {
        let cp = mk_checkpoint("s1", 7, 7);
        assert!(cp.is_complete());
        let cp2 = mk_checkpoint("s1", 5, 7);
        assert!(!cp2.is_complete());
    }

    #[test]
    fn t03_next_step() {
        let cp = mk_checkpoint("s1", 4, 7);
        assert_eq!(cp.next_step(), 4);
    }

    #[test]
    fn t04_memory_store_put_get() {
        let store = MemoryCheckpointStore::new();
        let cp = mk_checkpoint("s1", 2, 5);
        store.put(&cp).unwrap();
        let got = store.get("s1").unwrap();
        assert_eq!(got.current_step, 2);
        assert_eq!(got.opinions_so_far.len(), 2);
    }

    #[test]
    fn t05_memory_store_list() {
        let store = MemoryCheckpointStore::new();
        for i in 0..3 {
            store.put(&mk_checkpoint("s1", i, 5)).unwrap();
        }
        let cps = store.list("s1").unwrap();
        assert_eq!(cps.len(), 3);
        assert_eq!(cps[0].current_step, 0);
        assert_eq!(cps[2].current_step, 2);
    }

    #[test]
    fn t06_memory_store_get_latest() {
        let store = MemoryCheckpointStore::new();
        for i in 0..3 {
            store.put(&mk_checkpoint("s1", i, 5)).unwrap();
        }
        let latest = store.get("s1").unwrap();
        assert_eq!(latest.current_step, 2);
    }

    #[test]
    fn t07_memory_store_delete() {
        let store = MemoryCheckpointStore::new();
        store.put(&mk_checkpoint("s1", 1, 5)).unwrap();
        store.delete("s1").unwrap();
        assert!(matches!(store.get("s1"), Err(CheckpointError::NotFound(_))));
    }

    #[test]
    fn t08_memory_store_session_count() {
        let store = MemoryCheckpointStore::new();
        store.put(&mk_checkpoint("s1", 1, 5)).unwrap();
        store.put(&mk_checkpoint("s2", 1, 5)).unwrap();
        store.put(&mk_checkpoint("s1", 2, 5)).unwrap();
        assert_eq!(store.session_count(), 2);
        assert_eq!(store.total_checkpoints(), 3);
    }

    #[test]
    fn t09_memory_store_version_mismatch() {
        let store = MemoryCheckpointStore::new();
        let mut cp = mk_checkpoint("s1", 1, 5);
        cp.version = 999;
        assert!(matches!(
            store.put(&cp),
            Err(CheckpointError::VersionMismatch { .. })
        ));
    }

    #[test]
    fn t10_file_store_put_get() {
        let tmp = std::env::temp_dir().join(format!(
            "apeireth-cp-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = FileCheckpointStore::new(&tmp).unwrap();
        let cp = mk_checkpoint("s1", 3, 5);
        store.put(&cp).unwrap();
        let got = store.get("s1").unwrap();
        assert_eq!(got.current_step, 3);
        // 清理
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn t11_file_store_list_order() {
        let tmp = std::env::temp_dir().join(format!(
            "apeireth-cp-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = FileCheckpointStore::new(&tmp).unwrap();
        for i in 0..4 {
            store.put(&mk_checkpoint("s2", i, 7)).unwrap();
        }
        let cps = store.list("s2").unwrap();
        assert_eq!(cps.len(), 4);
        for (i, cp) in cps.iter().enumerate() {
            assert_eq!(cp.current_step, i);
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn t12_file_store_sanitize_session_id() {
        // 路径穿越测试: 包含 / 或 .. 应被替换
        let tmp = std::env::temp_dir().join(format!(
            "apeireth-cp-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = FileCheckpointStore::new(&tmp).unwrap();
        store.put(&mk_checkpoint("../etc/passwd", 1, 5)).unwrap();
        // 不会写到 /etc/passwd, 而是写到 base_dir/.._.._etc_passwd.jsonl
        let safe = store.list("../etc/passwd").unwrap();
        assert_eq!(safe.len(), 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
