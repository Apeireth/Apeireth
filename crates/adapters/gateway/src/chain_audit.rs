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
        // 已消费字节数 (物理偏移): 边界必须记**物理偏移**, 不能靠上一条边界累加 ——
        // 累加会在账本含空行 (非记录行) 时与真实偏移错位, savepoint 回滚就会截在
        // 记录中间, 把账本截坏 (链面 fuzz 回归: rollback_with_blank_line_*)。
        let mut offset = 0u64;
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
            offset += consumed;
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
                // 落盘的 `sequence` 必须在信封内: 它是记录的位置声明
                // (`AuditRecord::sequence` 文档 "Position in the chain, counting
                // from zero"), 而重放哈希用的是**链下标** —— 不比对落盘值的话,
                // 改写落盘 sequence 查不出来 (顺序被伪造)。链面 fuzz 回归:
                // tampered_sequence_field_is_refused。
                if appended.sequence != record.sequence {
                    return Err(format!(
                        "governance audit record {} sequence mismatch (tamper detected)",
                        boundaries.len() - 1
                    ));
                }
                if appended.previous_hash != record.previous_hash
                    || appended.current_hash != record.current_hash
                {
                    return Err(format!(
                        "governance audit record {} hash mismatch (tamper detected)",
                        boundaries.len() - 1
                    ));
                }
                // 边界 = 该条记录所在行的物理末尾 (含其换行符)。
                boundaries.push(offset);
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
            .read(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| error.to_string())?;
        use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
        // 记录分隔符修复: 账本末行缺 '\n' (撕裂写 —— 上个进程死在「写行体」与
        // 「写换行」之间) 时, 新记录会粘在旧行尾 ⇒ 账本从此再也开不出来
        // (「每行一个 AuditRecord」被破坏 ⇒ 审计永久降级到内存)。
        // 此处只**追加**一个换行 (append-only 不变), 让账本继续合法。
        let file_len = file.metadata().map_err(|error| error.to_string())?.len();
        let mut separator = 0u64;
        if file_len > 0 {
            file.seek(SeekFrom::End(-1))
                .map_err(|error| error.to_string())?;
            let mut last = [0u8; 1];
            file.read_exact(&mut last)
                .map_err(|error| error.to_string())?;
            if last[0] != b'\n' {
                file.write_all(b"\n").map_err(|error| error.to_string())?;
                separator = 1;
            }
        }
        file.write_all(line.as_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        state
            .boundaries
            .push(file_len + separator + line.len() as u64 + 1);
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
///
/// **编码面强制 (链面 fuzz 实测)**: `pair_id`/`subject` 自带 `#` 时, 编码结果
/// **不可唯一解码** —— 解码按第一个 `#` 切, 于是 pair_id 被截断、round 串位、
/// subject 吞掉后半段, 读面**冒充**另一条记录 (且不报错)。故编码面直接拒收
/// 带 `#` 的 `pair_id`/`subject` (fail-closed: 写不进去 ⇒ 账本永远可读, 不留
/// 半截冒充记录; 写面与读面口径一致 ⇒ 不会出现「写成功但读不回来」)。
fn encode_pair_record(record: &ApprovalAuditRecord) -> Result<(&'static str, String), String> {
    let pair_id = record.pair_id();
    if pair_id.contains('#') {
        return Err(format!(
            "approval audit pair id must not contain '#' (chain subject encoding is \
             ambiguous): {pair_id:?}"
        ));
    }
    let subject = record.subject();
    if subject.contains('#') {
        return Err(format!(
            "approval audit subject must not contain '#' (chain subject encoding is \
             ambiguous): {subject:?}"
        ));
    }
    Ok(match record {
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
    })
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
        // 契约: subject 不含 `#` ⇒ 解出来的 subject 带 `#` ⇒ 形状未知 ⇒ 报错
        // (不猜、不冒充)。这一条正是「pair_id 自带 '#' ⇒ 字段串位」的护栏:
        // 串位后 round 与 subject 会错位, 而错位后的 subject 必带 '#'。
        if tail.contains('#') {
            return Err(format!(
                "approval audit subject carries '#' in the subject part (ambiguous \
                 shape): {subject:?}"
            ));
        }
        Ok(Some(ApprovalAuditRecord::Asked {
            pair_id: pair_part.to_string(),
            round,
            subject: tail.to_string(),
        }))
    } else {
        let (subject_part, outcome_part) = tail
            .rsplit_once(':')
            .ok_or_else(|| format!("approval decision subject missing outcome: {subject:?}"))?;
        if subject_part.contains('#') {
            return Err(format!(
                "approval audit subject carries '#' in the subject part (ambiguous \
                 shape): {subject:?}"
            ));
        }
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
        let (event_kind, subject) = encode_pair_record(&record)?;
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

    /// 本次运行独有的账本路径 (pid + 纳秒 + 标签): 共享临时名会互撞。
    fn unique_path(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or_default();
        let dir = std::env::temp_dir().join(format!(
            "chain-audit-regress-{}-{nanos}-{label}",
            std::process::id()
        ));
        fs_err::create_dir_all(&dir).expect("temp dir");
        dir.join("governance-audit.jsonl")
    }

    /// 回归 (链面 fuzz 实测): `pair_id` 自带 `#` ⇒ 链内 subject **不可唯一解码**
    /// ⇒ 编码面必须拒收 (fail-closed), 解码面必须拒读 (不猜、不冒充)。
    ///
    /// 缺陷原文 (修复前): 编码面照写, 解码面按第一个 `#` 切 ⇒ pair_id 被截断、
    /// round 串位、subject 吞掉后半段 ⇒ 读面**冒充**另一条记录且不报错。
    #[test]
    fn ambiguous_pair_id_is_refused_on_write_and_on_read() {
        let path = unique_path("ambiguous");
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
        let mut audit = ChainApprovalAudit::open(
            sink.clone(),
            Arc::new(FixedChainClock::new(1_700_000_000_000)),
        )
        .expect("open pair ledger");
        let pair = ApprovalAuditPair::new("a#7:b", 5, "tool.shell", ApprovalOutcome::AllowedOnce);
        // 写面: 拒收 (契约: pair_id 不含 '#')。
        assert!(
            audit.commit(&pair).is_err(),
            "含 '#' 的 pair_id 必须拒写 (链内编码不可唯一解码)"
        );
        // 写面: subject 带 '#' 同样拒收 (否则「写成功但读面拒读」= 账本中毒)。
        let poisoned =
            ApprovalAuditPair::new("pair-2", 5, "tool#shell", ApprovalOutcome::AllowedOnce);
        assert!(
            audit.commit(&poisoned).is_err(),
            "含 '#' 的 subject 必须拒写 (写读口径必须一致)"
        );
        assert_eq!(sink.len(), 0, "拒写后链里不得留下半截记录");
        assert!(audit.committed().is_empty(), "拒写后读面必须为空");
        sink.verify().expect("拒写后链仍完整");
        drop(audit);
        drop(sink);

        // 读面: 手工塞一条同形状的链内记录 (模拟被伪造的账本) ⇒ 必须拒读。
        let sink = FileChainAuditSink::open(path.clone()).expect("reopen");
        sink.append("approval.asked", "a#7:b#5:tool.shell", Timestamp::now())
            .expect("append forged shape");
        drop(sink);
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("reopen"));
        let parsed =
            ChainApprovalAudit::open(sink, Arc::new(FixedChainClock::new(1_700_000_000_000)));
        assert!(parsed.is_err(), "串位形状必须拒读 (不猜、不冒充)");
        let _ = fs_err::remove_file(&path);
    }

    /// 回归 (链面 fuzz 实测): 落盘的 `sequence` 字段在信封内 —— 改写它
    /// (顺序被伪造) 必须拒开。
    ///
    /// 缺陷原文 (修复前): 重放哈希用的是**链下标**, 落盘 `sequence` 从不比对
    /// ⇒ `"sequence":1` 改成 `"sequence":999` 后账本照开, 顺序声明可被伪造。
    #[test]
    fn tampered_sequence_field_is_refused() {
        let path = unique_path("seq");
        let sink = FileChainAuditSink::open(path.clone()).expect("open");
        sink.append("governance.allowed", "tool.repo", Timestamp::now())
            .expect("append 0");
        sink.append("governance.denied.gate", "tool.shell", Timestamp::now())
            .expect("append 1");
        drop(sink);
        let text = fs_err::read_to_string(&path).expect("read");
        for (from, to) in [
            ("\"sequence\":0", "\"sequence\":7"),
            ("\"sequence\":1", "\"sequence\":999"),
        ] {
            let tampered = text.replacen(from, to, 1);
            assert_ne!(tampered, text, "篡改必须落到文件里: {from}");
            fs_err::write(&path, tampered.as_bytes()).expect("write tampered");
            assert!(
                FileChainAuditSink::open(path.clone()).is_err(),
                "改写落盘 sequence 必须拒开 (顺序被伪造)"
            );
        }
        // 未改动的原账本仍开得出 (自检: 拒开不是因为别的坏)。
        fs_err::write(&path, text.as_bytes()).expect("restore");
        let sink = FileChainAuditSink::open(path.clone()).expect("reopen pristine ledger");
        assert_eq!(sink.len(), 2);
        sink.verify().expect("pristine chain verifies");
        let _ = fs_err::remove_file(&path);
    }

    /// 回归 (链面 fuzz 实测): 撕裂写 (末行缺换行) 之后追加, 账本必须仍是
    /// 「每行一个 AuditRecord」⇒ 追加后仍开得出、链仍完整。
    ///
    /// 缺陷原文 (修复前): 追加直接接在末行尾 ⇒ 两记录挤成一行 ⇒
    /// `open` 永久报 "malformed: trailing characters" ⇒ 审计永久降级内存。
    #[test]
    fn append_after_a_torn_line_keeps_the_ledger_openable() {
        let path = unique_path("torn");
        let sink = FileChainAuditSink::open(path.clone()).expect("open");
        sink.append("governance.allowed", "tool.repo", Timestamp::now())
            .expect("append");
        drop(sink);
        // 模拟撕裂写: 去掉末行换行 (上个进程死在写行体与写换行之间)。
        let mut bytes = fs_err::read(&path).expect("read");
        bytes.pop();
        fs_err::write(&path, &bytes).expect("write torn");
        let reopened = FileChainAuditSink::open(path.clone()).expect("reopen torn ledger");
        assert_eq!(reopened.len(), 1, "撕裂写末行仍是一条完整记录");
        reopened
            .append("governance.denied.gate", "tool.shell", Timestamp::now())
            .expect("append after torn");
        drop(reopened);
        let after = FileChainAuditSink::open(path.clone()).expect("reopen after append");
        assert_eq!(after.len(), 2, "撕裂写后追加必须仍是两条记录 (每行一个)");
        after.verify().expect("chain verifies");
        let _ = fs_err::remove_file(&path);
    }

    /// 回归 (链面 fuzz 实测): 账本里的空行 (非记录行) 不得让 savepoint 回滚
    /// 截在记录中间 —— 回滚必须精确落在第 `committed_len` 条末尾, 且回滚后
    /// 账本仍开得出、追加后仍开得出。
    ///
    /// 缺陷原文 (修复前): 边界按「上一条边界 + 本行长度」累加 ⇒ 空行字节被
    /// 漏计 ⇒ 回滚截断偏移少 ⇒ 记录被截半 ⇒ 账本永久开不出。
    #[test]
    fn rollback_with_a_blank_line_truncates_at_the_record_boundary() {
        let path = unique_path("blank");
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("open"));
        sink.append("governance.allowed", "tool.repo", Timestamp::now())
            .expect("r0");
        sink.append("governance.denied.gate", "tool.shell", Timestamp::now())
            .expect("r1");
        sink.append("governance.allowed", "tool.filesystem", Timestamp::now())
            .expect("r2");
        drop(sink);
        // 插入一个空行 (外部改动: 空行不是 AuditRecord, 但链不因它失效)。
        let text = fs_err::read_to_string(&path).expect("read");
        let mut rebuilt = String::new();
        for (index, line) in text.lines().enumerate() {
            if index == 1 {
                rebuilt.push('\n');
            }
            rebuilt.push_str(line);
            rebuilt.push('\n');
        }
        fs_err::write(&path, rebuilt.as_bytes()).expect("write blank");
        let sink = Arc::new(FileChainAuditSink::open(path.clone()).expect("reopen blank"));
        assert_eq!(sink.len(), 3, "空行不是记录, 链长不变");
        let mut audit = ChainApprovalAudit::open(
            sink.clone(),
            Arc::new(FixedChainClock::new(1_700_000_000_000)),
        )
        .expect("open pair ledger");
        audit.rollback(2);
        assert_eq!(sink.len(), 2, "回滚后链回到存档点");
        sink.verify().expect("回滚后链仍完整");
        let reopened = FileChainAuditSink::open(path.clone()).expect("reopen after rollback");
        assert_eq!(reopened.len(), 2, "回滚后账本必须仍开得出");
        reopened
            .append("governance.allowed", "tool.repo", Timestamp::now())
            .expect("append after rollback");
        let after = FileChainAuditSink::open(path.clone()).expect("reopen after append");
        assert_eq!(after.len(), 3, "回滚后追加必须仍是三条记录");
        after.verify().expect("chain verifies");
        let _ = fs_err::remove_file(&path);
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
