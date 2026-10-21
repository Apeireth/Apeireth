//! R8-B3 (ADR-0010 D4) 治理审计落盘面: 链审计持久 [`AuditSink`] + IM 审批对经链落账。
//!
//! 两件东西:
//!
//! 1. [`FileChainAuditSink`] —— 链审计的持久 sink: append-only jsonl (每行一个
//!    [`AuditRecord`]) + 打开时全量重放重建 [`AuditHashChain`] 并逐条比对哈希
//!    (账本被篡改 = 拒开, fail-closed) + `verify()` 收口; 追加失败向上游报错,
//!    chain 的 `run()` 据此 `ChainOutcome::Denied` fail-closed。
//! 2. [`ChainApprovalAudit`] —— IM 审批对经链落账 (R8-B3 第二半): 实现
//!    [`ApprovalAuditSink`] + [`ImApprovalAuditCommit`], 把 `approval.asked` /
//!    `approval.decision` 一对记录写进**同一条哈希链** (审计单一账本, 可回答
//!    「这次审批进了哪条链」); savepoint 回滚 = 文件截断 + 链重建, 与
//!    [`commit_approval_audit_pair`] 的「整对回滚」语义一致。
//!
//! 0 装口径: 单写者假设 —— 链 sequence = 记录数, 多进程并发追加会分叉;
//! 生产装配单进程单 sink。`approve` 事件的 subject 编码见 [`ChainApprovalAudit`]
//! 模块内 [`encode_pair_record`], 解析严格 (解析失败 = 拒开/拒读, 不冒充)。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::Timestamp;
use apeireth_governance::approval_closure::{
    commit_approval_audit_pair, ApprovalAuditPair, ApprovalAuditRecord, ApprovalAuditSink,
    ApprovalOutcome, AUDIT_EVENT_ASKED, AUDIT_EVENT_DECISION,
};
use apeireth_governance::{
    AuditChainError, AuditHashChain, AuditRecord, AuditSink, ChainClock, SystemChainClock,
};

use crate::im_approval::ImApprovalAuditCommit;

/// 治理链的持久审计 sink (jsonl 追加 + 重放重建哈希链)。
pub struct FileChainAuditSink {
    path: PathBuf,
    state: Arc<Mutex<ChainState>>,
}

struct ChainState {
    chain: AuditHashChain,
    /// 字节边界: `boundaries[0] = 0`, `boundaries[i+1]` = 前 i 条记录落盘后的
    /// 文件长度 —— savepoint 回滚据此截断。
    boundaries: Vec<u64>,
}

impl FileChainAuditSink {
    /// 打开/创建持久审计文件。
    ///
    /// 逐行严格解析 [`AuditRecord`]、重放重建哈希链并逐条比对
    /// `previous_hash`/`current_hash` (任何篡改/断链 = 拒开), 最后 `verify()`。
    pub fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs_err::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        if !path.exists() {
            fs_err::write(&path, b"").map_err(|error| error.to_string())?;
        }
        let bytes = fs_err::read(&path).map_err(|error| error.to_string())?;
        let mut chain = AuditHashChain::new();
        let mut boundaries = vec![0u64];
        let mut rest = &bytes[..];
        while !rest.is_empty() {
            let (line, tail, consumed) = match rest.iter().position(|byte| *byte == b'\n') {
                Some(position) => (
                    &rest[..position],
                    &rest[position + 1..],
                    position as u64 + 1,
                ),
                None => (rest, &rest[..0], rest.len() as u64),
            };
            if !line.is_empty() {
                let record: AuditRecord = serde_json::from_slice(line).map_err(|error| {
                    format!(
                        "governance audit record {} is malformed: {error}",
                        boundaries.len() - 1
                    )
                })?;
                let appended = chain.append(
                    record.event_kind.clone(),
                    record.subject.clone(),
                    record.timestamp,
                );
                if appended.previous_hash != record.previous_hash
                    || appended.current_hash != record.current_hash
                {
                    return Err(format!(
                        "governance audit record {} hash mismatch (tamper detected)",
                        boundaries.len() - 1
                    ));
                }
                boundaries.push(boundaries[boundaries.len() - 1] + consumed);
            }
            rest = tail;
        }
        chain
            .verify()
            .map_err(|error| format!("governance audit chain does not verify: {error}"))?;
        Ok(Self {
            path,
            state: Arc::new(Mutex::new(ChainState { chain, boundaries })),
        })
    }

    /// 全链校验 (重启收口条件)。
    pub fn verify(&self) -> Result<(), AuditChainError> {
        let state = self
            .state
            .lock()
            .map_err(|_| AuditChainError::GenesisHashMismatch { index: 0 })?;
        state.chain.verify()
    }

    /// 记录读面 (测试/自述; 顺序即链序)。
    pub fn records(&self) -> Vec<AuditRecord> {
        self.state
            .lock()
            .map(|state| state.chain.records().to_vec())
            .unwrap_or_default()
    }

    /// savepoint 回滚: 文件截断回第 `committed_len` 条末尾 + 链按保留记录重建
    /// (哈希重算确定性, 重建链与原链逐字节同哈希)。
    fn rollback_to(&self, committed_len: usize) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "governance audit state poisoned".to_string())?;
        if committed_len >= state.chain.len() {
            return Ok(());
        }
        let offset = state.boundaries[committed_len];
        let file = fs_err::OpenOptions::new()
            .write(true)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        file.set_len(offset).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        let kept: Vec<AuditRecord> = state.chain.records()[..committed_len].to_vec();
        let mut chain = AuditHashChain::new();
        for record in kept {
            chain.append(record.event_kind, record.subject, record.timestamp);
        }
        state.chain = chain;
        state.boundaries.truncate(committed_len + 1);
        Ok(())
    }
}

impl AuditSink for FileChainAuditSink {
    fn append(
        &self,
        event_kind: &str,
        subject: &str,
        timestamp: Timestamp,
    ) -> Result<String, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "governance audit state poisoned".to_string())?;
        let record = state.chain.append(event_kind, subject, timestamp).clone();
        let line = serde_json::to_string(&record).map_err(|error| error.to_string())?;
        let mut file = fs_err::OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        use std::io::Write as _;
        file.write_all(line.as_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        let previous_end = state.boundaries[state.boundaries.len() - 1];
        state.boundaries.push(previous_end + line.len() as u64 + 1);
        Ok(record.current_hash)
    }

    fn len(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.chain.len())
            .unwrap_or_default()
    }
}

/// 审批对记录的链内 subject 编码: `{pair_id}#{round}:{subject}` (asked) /
/// `{pair_id}#{round}:{subject}:{outcome}` (decision)。
///
/// 契约: `pair_id`/`subject` 不含 `#` 与 `:` (approval id 与 capability id
/// 满足); 解析严格, 遇未知形状即错 (不猜、不冒充)。
fn encode_pair_record(record: &ApprovalAuditRecord) -> (&'static str, String) {
    match record {
        ApprovalAuditRecord::Asked {
            pair_id,
            round,
            subject,
        } => (AUDIT_EVENT_ASKED, format!("{pair_id}#{round}:{subject}")),
        ApprovalAuditRecord::Decision {
            pair_id,
            round,
            subject,
            outcome,
        } => (
            AUDIT_EVENT_DECISION,
            format!("{pair_id}#{round}:{subject}:{}", outcome_label(*outcome)),
        ),
    }
}

fn outcome_label(outcome: ApprovalOutcome) -> &'static str {
    match outcome {
        ApprovalOutcome::AllowedOnce => "allowed_once",
        ApprovalOutcome::Rejected => "rejected",
        ApprovalOutcome::Cancelled => "cancelled",
        ApprovalOutcome::Unavailable => "unavailable",
    }
}

fn outcome_from_label(label: &str) -> Result<ApprovalOutcome, String> {
    match label {
        "allowed_once" => Ok(ApprovalOutcome::AllowedOnce),
        "rejected" => Ok(ApprovalOutcome::Rejected),
        "cancelled" => Ok(ApprovalOutcome::Cancelled),
        "unavailable" => Ok(ApprovalOutcome::Unavailable),
        other => Err(format!("unknown approval outcome label {other:?}")),
    }
}

fn decode_pair_record(
    event_kind: &str,
    subject: &str,
) -> Result<Option<ApprovalAuditRecord>, String> {
    if event_kind != AUDIT_EVENT_ASKED && event_kind != AUDIT_EVENT_DECISION {
        return Ok(None);
    }
    let (pair_part, rest) = subject
        .split_once('#')
        .ok_or_else(|| format!("approval audit subject missing '#': {subject:?}"))?;
    let (round_part, tail) = rest
        .split_once(':')
        .ok_or_else(|| format!("approval audit subject missing ':': {subject:?}"))?;
    let round: u64 = round_part
        .parse()
        .map_err(|_| format!("approval audit round not numeric: {subject:?}"))?;
    if event_kind == AUDIT_EVENT_ASKED {
        Ok(Some(ApprovalAuditRecord::Asked {
            pair_id: pair_part.to_string(),
            round,
            subject: tail.to_string(),
        }))
    } else {
        let (subject_part, outcome_part) = tail
            .rsplit_once(':')
            .ok_or_else(|| format!("approval decision subject missing outcome: {subject:?}"))?;
        Ok(Some(ApprovalAuditRecord::Decision {
            pair_id: pair_part.to_string(),
            round,
            subject: subject_part.to_string(),
            outcome: outcome_from_label(outcome_part)?,
        }))
    }
}

/// IM 审批对经链落账 (R8-B3): pair 的两半写进同一条哈希链。
///
/// 读面 [`committed()`](ImApprovalAuditCommit::committed) = 会话内在账
/// (open 时从链重建 + 每次写入追加; savepoint 回滚同步裁剪)。
#[derive(Clone)]
pub struct ChainApprovalAudit {
    sink: Arc<FileChainAuditSink>,
    clock: Arc<dyn ChainClock>,
    /// 会话内已落账对记录 (chain 记录下标, 记录) —— 回滚按存档点裁剪。
    committed: Arc<Mutex<Vec<(usize, ApprovalAuditRecord)>>>,
}

impl ChainApprovalAudit {
    /// 打开经链审批账: 从链上已有 approval.* 事件重建读面 (解析失败 = 拒开)。
    pub fn open(sink: Arc<FileChainAuditSink>, clock: Arc<dyn ChainClock>) -> Result<Self, String> {
        let mut committed = Vec::new();
        for (index, record) in sink.records().into_iter().enumerate() {
            if let Some(pair_record) = decode_pair_record(&record.event_kind, &record.subject)? {
                committed.push((index, pair_record));
            }
        }
        Ok(Self {
            sink,
            clock,
            committed: Arc::new(Mutex::new(committed)),
        })
    }

    /// 系统时钟口径 (生产)。
    pub fn with_system_clock(sink: Arc<FileChainAuditSink>) -> Result<Self, String> {
        Self::open(sink, Arc::new(SystemChainClock))
    }
}

impl ApprovalAuditSink for ChainApprovalAudit {
    type Error = String;

    fn committed_len(&self) -> usize {
        self.sink.len()
    }

    fn write(&mut self, record: ApprovalAuditRecord) -> Result<(), Self::Error> {
        let (event_kind, subject) = encode_pair_record(&record);
        let index = self.sink.len();
        self.sink.append(event_kind, &subject, self.clock.now())?;
        self.committed
            .lock()
            .map_err(|_| "approval audit state poisoned".to_string())?
            .push((index, record));
        Ok(())
    }

    fn rollback(&mut self, committed_len: usize) {
        self.sink.rollback_to(committed_len).ok();
        if let Ok(mut committed) = self.committed.lock() {
            committed.retain(|(index, _)| *index < committed_len);
        }
    }
}

impl ImApprovalAuditCommit for ChainApprovalAudit {
    fn commit(&self, pair: &ApprovalAuditPair) -> Result<(), String> {
        let mut handle = self.clone();
        commit_approval_audit_pair(&mut handle, pair).map_err(|error| error.to_string())
    }

    fn committed(&self) -> Vec<ApprovalAuditRecord> {
        self.committed
            .lock()
            .map(|committed| committed.iter().map(|(_, record)| record.clone()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
    use apeireth_governance::{
        Action, ChainOutcome, Decision, FixedChainClock, GovernanceChain, GovernanceHook,
        GovernanceRequest, GovernanceVerdict,
    };

    struct AllowGate;

    #[async_trait::async_trait]
    impl GovernanceHook for AllowGate {
        fn name(&self) -> &str {
            "test.allow_gate"
        }

        async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
            Decision::Allow
        }

        async fn evaluate_verbose(&self, _request: &GovernanceRequest<'_>) -> GovernanceVerdict {
            GovernanceVerdict::new(self.name(), Decision::Allow)
        }
    }

    fn temp_path(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("chain-audit-test-{}", std::process::id()));
        fs_err::create_dir_all(&dir).expect("temp dir");
        dir.join(format!("{label}.jsonl"))
    }

    /// 验收②: 追加失败 fail-closed —— sink 报错, chain 的 run() 出
    /// `ChainOutcome::Denied`。
    #[tokio::test]
    async fn append_failure_fails_the_chain_closed() {
        let path = temp_path("failclosed");
        let _ = fs_err::remove_file(&path);
        let _ = fs_err::remove_dir(&path);
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
        // 把账本文件换成同名目录: 追加必然失败 (盘满/只读的可测替身)。
        fs_err::remove_file(&path).expect("remove");
        fs_err::create_dir(&path).expect("dir in the way");
        let chain = GovernanceChain::new(Arc::new(AllowGate)).with_audit_sink(sink);
        let capability = CapabilityId::new("tool.shell").expect("valid capability");
        let arguments = serde_json::Value::Null;
        let request = GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &capability,
                arguments: &arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        );
        let outcome = chain.run(&request).await;
        assert!(
            matches!(outcome, ChainOutcome::Denied { .. }),
            "审计追加失败必须 fail-closed: {outcome:?}"
        );
        let _ = fs_err::remove_dir(&path);
    }

    /// 验收①: 重启后审计链 verify() 通过, 并且链能继续追加 (哈希链完整)。
    #[test]
    fn reopened_chain_verifies_and_keeps_appending() {
        let path = temp_path("roundtrip");
        let _ = fs_err::remove_file(&path);
        let sink = FileChainAuditSink::open(path.clone()).expect("open");
        sink.append("governance.allowed", "tool.repo", Timestamp::now())
            .expect("append 1");
        sink.append("governance.denied.gate", "tool.shell", Timestamp::now())
            .expect("append 2");
        drop(sink);

        let reopened = FileChainAuditSink::open(path.clone()).expect("reopen");
        reopened.verify().expect("chain verifies after restart");
        assert_eq!(reopened.len(), 2);
        reopened
            .append(
                "governance.approval_required",
                "tool.filesystem",
                Timestamp::now(),
            )
            .expect("append after restart");
        reopened.verify().expect("chain still verifies");
        assert_eq!(reopened.len(), 3);
        let _ = fs_err::remove_file(&path);
    }

    /// 账本被篡改 = 拒开 (fail-closed, 不冒充)。
    #[test]
    fn tampered_file_refuses_to_open() {
        let path = temp_path("tamper");
        let _ = fs_err::remove_file(&path);
        let sink = FileChainAuditSink::open(path.clone()).expect("open");
        sink.append("governance.allowed", "tool.repo", Timestamp::now())
            .expect("append");
        drop(sink);
        let bytes = fs_err::read(&path).expect("read");
        let tampered = bytes
            .windows(9)
            .position(|window| window == b"tool.repo")
            .expect("subject present");
        let mut fixed = bytes;
        fixed[tampered..tampered + 9].copy_from_slice(b"tool.root");
        fs_err::write(&path, fixed).expect("write tampered");
        assert!(
            FileChainAuditSink::open(path.clone()).is_err(),
            "被篡改的账本必须拒开"
        );
        let _ = fs_err::remove_file(&path);
    }

    /// 整对回滚: decision 写失败时 asked 半对也回滚 (账本无半对状态)。
    #[test]
    fn pair_commit_rolls_back_to_savepoint() {
        let path = temp_path("rollback");
        let _ = fs_err::remove_file(&path);
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
        sink.append("governance.allowed", "tool.repo", Timestamp::now())
            .expect("prior record");
        let mut audit = ChainApprovalAudit::open(
            sink.clone(),
            Arc::new(FixedChainClock::new(1_700_000_000_000)),
        )
        .expect("open pair ledger");
        let savepoint = audit.committed_len();
        let asked = ApprovalAuditRecord::Asked {
            pair_id: "p1".to_string(),
            round: 1,
            subject: "tool.shell".to_string(),
        };
        audit.write(asked.clone()).expect("asked write");
        assert_eq!(sink.len(), savepoint + 1);
        audit.rollback(savepoint);
        assert_eq!(sink.len(), savepoint, "回滚后链回到存档点");
        sink.verify().expect("回滚后链仍完整");
        assert!(audit.committed().is_empty(), "读面无半对状态");
        let _ = fs_err::remove_file(&path);
    }

    /// IM 审批对经链落账: asked ↔ decision 一对进同一条哈希链, 重启后读面重建。
    #[test]
    fn pair_commit_lands_in_the_chain_and_survives_restart() {
        let path = temp_path("pair");
        let _ = fs_err::remove_file(&path);
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
        let mut audit = ChainApprovalAudit::open(
            sink.clone(),
            Arc::new(FixedChainClock::new(1_700_000_000_000)),
        )
        .expect("open pair ledger");
        let pair = ApprovalAuditPair::new("pair-1", 2, "tool.shell", ApprovalOutcome::AllowedOnce);
        audit.commit(&pair).expect("pair commit");
        let records = sink.records();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].event_kind, AUDIT_EVENT_ASKED);
        assert_eq!(records[0].subject, "pair-1#2:tool.shell");
        assert_eq!(records[1].event_kind, AUDIT_EVENT_DECISION);
        assert_eq!(records[1].subject, "pair-1#2:tool.shell:allowed_once");
        sink.verify().expect("chain verifies");
        assert_eq!(
            audit.committed(),
            vec![pair.asked().clone(), pair.decision().clone()]
        );

        // 重启: 链 verify + 读面从链重建。
        drop(audit);
        drop(sink);
        let reopened = Arc::new(FileChainAuditSink::open(path.clone()).expect("reopen"));
        reopened.verify().expect("chain verifies after restart");
        let rebuilt =
            ChainApprovalAudit::open(reopened, Arc::new(FixedChainClock::new(1_700_000_000_001)))
                .expect("rebuild pair ledger");
        assert_eq!(
            rebuilt.committed(),
            vec![pair.asked().clone(), pair.decision().clone()]
        );
        let _ = fs_err::remove_file(&path);
    }
}
