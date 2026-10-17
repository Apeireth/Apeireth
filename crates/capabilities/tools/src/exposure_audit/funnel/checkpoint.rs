//! 守望巡检漏斗 · 巡检状态落库 (断点续查)。
//!
//! 巡检是长跑: 每件处理完**立即落库** ([`PatrolStateStore::save`]), 中途猝死
//! 重启后 [`PatrolCheckpoint`] 记着每段做到哪 —— 已筛查的不重扫、已精查的
//! 不重查、已复核的不重复 (断点续查 = 只做余下)。落库的是**巡检自己的工作区**
//! (`patrol_state.json`, 调用方指定路径), 被测面仍然只读零写 (两件事分开:
//! 状态落库不进被测面)。
//!
//! 进度只记「做过什么」不记「做没做完」: 完成与否由计数**推导** (成本闸截断
//! 后接着做的时候, 结论不会被旧标记钉死)。损坏的落库档**拒绝即帧**
//! ([`crate::exposure_audit::funnel::PatrolError::Checkpoint`]), 不静默重建 ——
//! 静默重建会把「上次巡检做到哪」悄悄丢掉。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::exposure_audit::funnel::evidence::{
    risk_from_word, Candidate, DeepEvidence, DeepEvidenceKind,
};
use crate::exposure_audit::funnel::task_seam::PatrolTaskAccount;
use crate::exposure_audit::funnel::PatrolError;

/// 落库档的稳定身份 (`schema` 字段)。
pub const CHECKPOINT_SCHEMA: &str = "apeireth.patrol_checkpoint";

/// 落库档的形状版本 (`schema_version` 字段)。
pub const CHECKPOINT_SCHEMA_VERSION: u32 = 1;

/// 巡检状态落库默认文件名。
pub const PATROL_STATE_FILE: &str = "patrol_state.json";

/// 巡检任务的执行者词面 (巡检是守望族自己的活; 队列指派后以队列记录为准)。
pub(crate) const PATROL_ASSIGNEE: &str = "exposure-patrol";

/// 筛查段进度 (全量候选清单 + 已过成本闸的准入位)。
///
/// 候选清单整份落库 (含未准入部分), 成本闸截断的余量在续跑会话里继续准入 ——
/// 截断不丢件, 只是「这一次没轮到」。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScreenProgress {
    /// 六项速扫是否已跑过 (空面也要显式记「扫过」, 不重扫)。
    pub scanned: bool,
    /// 去重后的全量候选 (筛查段产出)。
    pub candidates: Vec<Candidate>,
    /// 已过成本闸的准入数 (下游各段只见前 `admitted` 件)。
    pub admitted: usize,
    /// 六项速扫的原始发现总数 (含聚合/截断面, 如实计数)。
    pub total_findings: usize,
}

impl ScreenProgress {
    /// 已准入的候选 (进漏斗下游的部分)。
    pub fn admitted_candidates(&self) -> &[Candidate] {
        &self.candidates[..self.admitted.min(self.candidates.len())]
    }

    /// 筛查段是否完成 (已扫 + 全部候选都已准入)。
    pub fn is_done(&self) -> bool {
        self.scanned && self.admitted >= self.candidates.len()
    }
}

/// 精查段进度 (`done_keys` = 已查过的候选键, 续跑不重做)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeepProgress {
    /// 已精查过的候选键 (断点续查的跳过账)。
    pub done_keys: Vec<String>,
    /// 已产出的取证 (一键一份)。
    pub evidence: Vec<DeepEvidence>,
}

/// 复核段进度 (二次测量快照 + 逐键复验结论)。
///
/// 二次测量**只采一次** (跨续跑同一把尺子, 不换尺子)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecheckProgress {
    /// 二次测量是否已采集。
    pub started: bool,
    /// 二次测量的候选快照 (键 + 风险级)。
    pub second: Vec<Candidate>,
    /// 已复验过的候选键 (断点续查的跳过账)。
    pub done_keys: Vec<String>,
    /// 二次测量一致 (确认上报)。
    pub confirmed: Vec<String>,
    /// 瞬时态 (二次测量不见) —— 降噪剔除。
    pub dropped_transient: Vec<String>,
    /// 风险级漂移 (二次测量不一致) —— 降噪剔除。
    pub dropped_risk_shift: Vec<String>,
}

/// 巡检状态落库 (三段进度 + 任务账 + 增量 diff 的基线键账)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatrolCheckpoint {
    /// 巡检 id (幂等键同源)。
    pub patrol_id: String,
    /// 本次巡检是否已完成 (未完成 = 可续跑)。
    pub completed: bool,
    /// 巡检任务账 (队列同账)。
    pub task: PatrolTaskAccount,
    /// 筛查段进度。
    pub screen: ScreenProgress,
    /// 精查段进度。
    pub deep: DeepProgress,
    /// 复核段进度。
    pub recheck: RecheckProgress,
    /// 增量 diff 基线: 上一次完成巡检的 id (首次 = None)。
    pub baseline_patrol_id: Option<String>,
    /// 增量 diff 基线: 上一次完成巡检的确认键账。
    pub baseline_keys: Vec<String>,
}

impl PatrolCheckpoint {
    /// 新巡检的空档 (基线沿用上次完成巡检的键账)。
    pub fn fresh(
        patrol_id: impl Into<String>,
        baseline_patrol_id: Option<String>,
        baseline_keys: Vec<String>,
    ) -> Self {
        let patrol_id = patrol_id.into();
        Self {
            task: PatrolTaskAccount::new(patrol_id.clone(), PATROL_ASSIGNEE),
            patrol_id,
            completed: false,
            screen: ScreenProgress::default(),
            deep: DeepProgress::default(),
            recheck: RecheckProgress::default(),
            baseline_patrol_id,
            baseline_keys,
        }
    }

    /// 落库档 → 线上形状 (风险级/取证种类落词面)。
    fn to_wire(&self) -> CheckpointWire {
        CheckpointWire {
            schema: CHECKPOINT_SCHEMA.to_string(),
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            patrol_id: self.patrol_id.clone(),
            completed: self.completed,
            task: self.task.clone(),
            screen: ScreenWire {
                scanned: self.screen.scanned,
                candidates: self
                    .screen
                    .candidates
                    .iter()
                    .map(CandidateWire::from)
                    .collect(),
                admitted: self.screen.admitted,
                total_findings: self.screen.total_findings,
            },
            deep: DeepWire {
                done_keys: self.deep.done_keys.clone(),
                evidence: self.deep.evidence.iter().map(EvidenceWire::from).collect(),
            },
            recheck: RecheckWire {
                started: self.recheck.started,
                second: self
                    .recheck
                    .second
                    .iter()
                    .map(CandidateWire::from)
                    .collect(),
                done_keys: self.recheck.done_keys.clone(),
                confirmed: self.recheck.confirmed.clone(),
                dropped_transient: self.recheck.dropped_transient.clone(),
                dropped_risk_shift: self.recheck.dropped_risk_shift.clone(),
            },
            baseline_patrol_id: self.baseline_patrol_id.clone(),
            baseline_keys: self.baseline_keys.clone(),
        }
    }

    /// 线上形状 → 落库档 (未知词面 = 坏档, 拒绝即帧)。
    fn from_wire(wire: CheckpointWire) -> Result<Self, String> {
        if wire.schema != CHECKPOINT_SCHEMA {
            return Err(format!("落库档身份不符: {}", wire.schema));
        }
        if wire.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(format!(
                "落库档形状版本不支持: {} (当前 {CHECKPOINT_SCHEMA_VERSION})",
                wire.schema_version
            ));
        }
        let mut screen = ScreenProgress {
            scanned: wire.screen.scanned,
            admitted: wire.screen.admitted,
            total_findings: wire.screen.total_findings,
            ..ScreenProgress::default()
        };
        for candidate in wire.screen.candidates {
            screen.candidates.push(candidate.try_into_domain()?);
        }
        let mut deep = DeepProgress::default();
        deep.done_keys = wire.deep.done_keys;
        for evidence in wire.deep.evidence {
            deep.evidence.push(evidence.try_into_domain()?);
        }
        let mut recheck = RecheckProgress {
            started: wire.recheck.started,
            done_keys: wire.recheck.done_keys,
            confirmed: wire.recheck.confirmed,
            dropped_transient: wire.recheck.dropped_transient,
            dropped_risk_shift: wire.recheck.dropped_risk_shift,
            ..RecheckProgress::default()
        };
        for candidate in wire.recheck.second {
            recheck.second.push(candidate.try_into_domain()?);
        }
        Ok(Self {
            patrol_id: wire.patrol_id,
            completed: wire.completed,
            task: wire.task,
            screen,
            deep,
            recheck,
            baseline_patrol_id: wire.baseline_patrol_id,
            baseline_keys: wire.baseline_keys,
        })
    }
}

/// 候选的线上形状 (风险级落词面)。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CandidateWire {
    key: String,
    item_id: String,
    surface: String,
    risk: String,
    detail: String,
}

impl From<&Candidate> for CandidateWire {
    fn from(candidate: &Candidate) -> Self {
        Self {
            key: candidate.key.clone(),
            item_id: candidate.item_id.clone(),
            surface: candidate.surface.clone(),
            risk: candidate.risk.as_str().to_string(),
            detail: candidate.detail.clone(),
        }
    }
}

impl CandidateWire {
    fn try_into_domain(self) -> Result<Candidate, String> {
        let risk = risk_from_word(&self.risk)
            .ok_or_else(|| format!("落库档风险级词面未知: {}", self.risk))?;
        Ok(Candidate {
            key: self.key,
            item_id: self.item_id,
            surface: self.surface,
            risk,
            detail: self.detail,
        })
    }
}

/// 取证的线上形状 (种类落词面)。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvidenceWire {
    key: String,
    kind: String,
    summary: String,
    lines: Vec<String>,
    truncated: bool,
}

impl From<&DeepEvidence> for EvidenceWire {
    fn from(evidence: &DeepEvidence) -> Self {
        Self {
            key: evidence.key.clone(),
            kind: evidence.kind.as_str().to_string(),
            summary: evidence.summary.clone(),
            lines: evidence.lines.clone(),
            truncated: evidence.truncated,
        }
    }
}

impl EvidenceWire {
    fn try_into_domain(self) -> Result<DeepEvidence, String> {
        let kind = DeepEvidenceKind::parse(&self.kind)
            .ok_or_else(|| format!("落库档取证种类词面未知: {}", self.kind))?;
        Ok(DeepEvidence {
            key: self.key,
            kind,
            summary: self.summary,
            lines: self.lines,
            truncated: self.truncated,
        })
    }
}

/// 三段进度的线上形状。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScreenWire {
    scanned: bool,
    candidates: Vec<CandidateWire>,
    admitted: usize,
    total_findings: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeepWire {
    done_keys: Vec<String>,
    evidence: Vec<EvidenceWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecheckWire {
    started: bool,
    second: Vec<CandidateWire>,
    done_keys: Vec<String>,
    confirmed: Vec<String>,
    dropped_transient: Vec<String>,
    dropped_risk_shift: Vec<String>,
}

/// 落库档的线上形状 (serde 合同)。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckpointWire {
    schema: String,
    schema_version: u32,
    patrol_id: String,
    completed: bool,
    task: PatrolTaskAccount,
    screen: ScreenWire,
    deep: DeepWire,
    recheck: RecheckWire,
    baseline_patrol_id: Option<String>,
    baseline_keys: Vec<String>,
}

/// 巡检状态落库端口 (每件一落; 损坏拒绝即帧)。
pub trait PatrolStateStore {
    /// 读档 (无档 = None; 坏档 = `Err`, 不静默重建)。
    fn load(&self) -> Result<Option<PatrolCheckpoint>, PatrolError>;
    /// 落档 (一次 save = 一份完整档)。
    fn save(&self, checkpoint: &PatrolCheckpoint) -> Result<(), PatrolError>;
}

/// 文件落库 (`patrol_state.json`, serde JSON)。
#[derive(Debug, Clone)]
pub struct JsonPatrolStore {
    path: PathBuf,
}

impl JsonPatrolStore {
    /// 指定落库路径 (巡检自己的工作区, 与被测面分开)。
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 默认文件名落库到指定目录。
    pub fn in_dir(dir: impl AsRef<Path>) -> Self {
        Self::new(dir.as_ref().join(PATROL_STATE_FILE))
    }

    /// 落库路径。
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl PatrolStateStore for JsonPatrolStore {
    fn load(&self) -> Result<Option<PatrolCheckpoint>, PatrolError> {
        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(PatrolError::Store {
                    operation: "read_checkpoint",
                    reason: error.to_string(),
                });
            }
        };
        let wire: CheckpointWire =
            serde_json::from_slice(&bytes).map_err(|error| PatrolError::Checkpoint {
                reason: format!("落库档不可解析 (拒绝即帧, 不静默重建): {error}"),
            })?;
        let checkpoint =
            PatrolCheckpoint::from_wire(wire).map_err(|reason| PatrolError::Checkpoint {
                reason: format!("落库档形状不合法 (拒绝即帧, 不静默重建): {reason}"),
            })?;
        Ok(Some(checkpoint))
    }

    fn save(&self, checkpoint: &PatrolCheckpoint) -> Result<(), PatrolError> {
        let wire = checkpoint.to_wire();
        let text = serde_json::to_string_pretty(&wire).map_err(|error| PatrolError::Store {
            operation: "serialize_checkpoint",
            reason: error.to_string(),
        })?;
        std::fs::write(&self.path, text).map_err(|error| PatrolError::Store {
            operation: "write_checkpoint",
            reason: error.to_string(),
        })
    }
}

/// 内存落库 (测试口/无盘场景; 语义与文件落库逐字一致)。
#[derive(Debug, Default)]
pub struct MemoryPatrolStore {
    state: Mutex<Option<PatrolCheckpoint>>,
}

impl MemoryPatrolStore {
    /// 空内存落库。
    pub fn new() -> Self {
        Self::default()
    }
}

impl PatrolStateStore for MemoryPatrolStore {
    fn load(&self) -> Result<Option<PatrolCheckpoint>, PatrolError> {
        let guard = self.state.lock().map_err(|error| PatrolError::Store {
            operation: "lock_checkpoint",
            reason: error.to_string(),
        })?;
        Ok(guard.clone())
    }

    fn save(&self, checkpoint: &PatrolCheckpoint) -> Result<(), PatrolError> {
        let mut guard = self.state.lock().map_err(|error| PatrolError::Store {
            operation: "lock_checkpoint",
            reason: error.to_string(),
        })?;
        *guard = Some(checkpoint.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exposure_audit::report::RiskLevel;

    #[test]
    fn checkpoint_roundtrips_through_the_wire_shape() {
        let mut checkpoint = PatrolCheckpoint::fresh("patrol-9", None, Vec::new());
        checkpoint.screen.scanned = true;
        checkpoint.screen.total_findings = 3;
        checkpoint.screen.candidates.push(Candidate::new(
            "plaintext_points",
            "app.toml",
            RiskLevel::High,
            "fact",
        ));
        checkpoint.screen.admitted = 1;
        checkpoint
            .deep
            .done_keys
            .push("plaintext_points|app.toml".to_string());
        checkpoint.deep.evidence.push(DeepEvidence {
            key: "plaintext_points|app.toml".to_string(),
            kind: DeepEvidenceKind::PlaintextFields,
            summary: "s".to_string(),
            lines: vec!["api_key = \"[redacted]\"".to_string()],
            truncated: false,
        });

        let dir = tempfile::tempdir().expect("temp dir");
        let store = JsonPatrolStore::in_dir(dir.path());
        store.save(&checkpoint).expect("save");
        let loaded = store.load().expect("load").expect("some");
        assert_eq!(loaded, checkpoint);
    }

    #[test]
    fn corrupt_store_fails_loud_instead_of_silent_reset() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = JsonPatrolStore::in_dir(dir.path());
        std::fs::write(store.path(), b"{ not json").unwrap();
        let result = store.load();
        assert!(
            matches!(result, Err(PatrolError::Checkpoint { .. })),
            "坏档必须拒绝即帧: {result:?}"
        );
    }

    #[test]
    fn absent_store_reads_as_none() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = JsonPatrolStore::in_dir(dir.path());
        assert!(store.load().expect("load").is_none());
    }
}
