//! 操作幂等去重表 (C2-1).
//!
//! 目的: 同一会话内的同一个操作键只执行一次。客户端重发同一请求 (网络重试、
//! 双发) 时, 第二次不再执行, 直接回放第一次的终态结果; 载荷不同或上一次
//! 执行中断未定则返回冲突, 由客户端换新操作键重试。
//!
//! # 语义边界 (诚实边界)
//!
//! - 进程内、单 Runtime 实例内有效: 重启即失, 多进程不共享。
//! - 只有显式携带 session 的请求才进出本表 (作用域是 (session, operation)),
//!   未给 session 的请求每次都按新会话处理, 不去重。
//! - 回放是终态重放: 回放 Completed 得到同一份 TurnResponse, 回放
//!   PendingApproval 得到同一份审批视图; 过程 delta 不重放。
//! - 未到终态就中断 (RuntimeError, 或被取消) 的条目落 Indeterminate:
//!   同键重试一律返回冲突并提示换新操作键, 不猜测上一次的结果。
//! - 容量 1024 条; 淘汰只挑最早完成 (或已中断) 的条目, 在飞条目绝不淘汰,
//!   故瞬时条目数可超过容量。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use apeireth_core::kernel::SessionId;

use super::execute::TurnOutcome;
use super::runtime::Runtime;

/// 操作键最长字节数 (与稳定标识同一上限)。
pub const MAX_OPERATION_KEY_BYTES: usize = 128;

/// 默认容量: 每个 Runtime 进程内最多跟踪的操作条数。
pub const DEFAULT_OPERATION_CAPACITY: usize = 1024;

/// 同键不同载荷的冲突原因。
const PAYLOAD_MISMATCH_REASON: &str =
    "the same operation key was first used with a different payload";

/// 上次中断未定的冲突原因。
const INDETERMINATE_REASON: &str =
    "the previous attempt was interrupted before reaching a final outcome; retry with a new operation key";

/// 操作键不满足稳定标识文法。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidOperationKey {
    key: String,
    reason: &'static str,
}

impl InvalidOperationKey {
    /// 被拒的操作键。
    pub fn key(&self) -> &str {
        &self.key
    }

    /// 稳定的英文拒绝原因。
    pub fn reason(&self) -> &'static str {
        self.reason
    }
}

impl std::fmt::Display for InvalidOperationKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "operation key {:?} is invalid: {}",
            self.key, self.reason
        )
    }
}

impl std::error::Error for InvalidOperationKey {}

/// 校验操作键文法: 与核心稳定标识同规则。
///
/// 非空、最多 128 字节、首字符 ASCII 小写字母、只含 a-z 0-9 . - _、
/// 点分段不得为空。
pub fn validate_operation_key(key: &str) -> Result<(), InvalidOperationKey> {
    let invalid = |reason: &'static str| InvalidOperationKey {
        key: key.to_string(),
        reason,
    };
    if key.is_empty() {
        return Err(invalid("must not be empty"));
    }
    if key.len() > MAX_OPERATION_KEY_BYTES {
        return Err(invalid("must be at most 128 bytes"));
    }
    if !key.starts_with(|c: char| c.is_ascii_lowercase()) {
        return Err(invalid("must start with an ASCII lowercase letter"));
    }
    for c in key.chars() {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_');
        if !ok {
            return Err(invalid("contains a character outside a-z 0-9 . - _"));
        }
    }
    if key.split('.').any(str::is_empty) {
        return Err(invalid("must not contain an empty dot-separated segment"));
    }
    Ok(())
}

/// 一次已解析、已校验的操作键 + 载荷摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRequest {
    id: String,
    payload_digest: [u8; 32],
}

impl OperationRequest {
    /// 校验键文法并登记载荷摘要 (SHA-256, 32 字节)。
    pub fn new(
        id: impl Into<String>,
        payload_digest: [u8; 32],
    ) -> Result<Self, InvalidOperationKey> {
        let id = id.into();
        validate_operation_key(&id)?;
        Ok(Self { id, payload_digest })
    }

    /// 操作键原文。
    pub fn id(&self) -> &str {
        &self.id
    }

    /// 载荷摘要。
    pub fn payload_digest(&self) -> &[u8; 32] {
        &self.payload_digest
    }
}

/// 幂等表的一次登记裁决。
#[derive(Debug)]
pub enum OperationAdmission {
    /// 首次见到该键: 执行方持守卫跑这一回合, 完成后交回终态。
    Fresh(OperationGuard),
    /// 该键已有终态: 回放这份结果, 不再执行。
    Replay(TurnOutcome),
    /// 同键不同载荷, 或上次中断未定: 调用方应换新操作键。
    Conflict {
        /// 被拒的操作键。
        operation: String,
        /// 稳定的英文原因。
        reason: &'static str,
    },
    /// 同键同载荷仍在执行中。
    InProgress {
        /// 在飞的操作键。
        operation: String,
    },
}

/// 一次新鲜执行的所有权守卫。
///
/// 只有 complete 会写入终态; 守卫被丢弃 (回合报错、请求被取消、任务被
/// abort) 时条目落 Indeterminate, 同键重试得到冲突。
#[derive(Debug)]
pub struct OperationGuard {
    inner: Arc<Mutex<Inner>>,
    key: OperationKey,
    digest: [u8; 32],
    finished: bool,
}

impl OperationGuard {
    /// 记录本次执行的终态 (Completed 或 PendingApproval)。
    pub fn complete(mut self, outcome: TurnOutcome) {
        self.record(outcome);
    }

    fn record(&mut self, outcome: TurnOutcome) {
        let mut inner = lock(&self.inner);
        match inner.entries.get_mut(&self.key) {
            Some(entry) if entry.digest == self.digest => {
                entry.state = EntryState::Complete(outcome);
            }
            _ => {
                // 防御性回填: 条目被外部清掉时仍留下终态, 不丢回放能力。
                inner.entries.insert(
                    self.key.clone(),
                    Entry {
                        digest: self.digest,
                        state: EntryState::Complete(outcome),
                    },
                );
                inner.order.push_back(self.key.clone());
            }
        }
        self.finished = true;
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let mut inner = lock(&self.inner);
        if let Some(entry) = inner.entries.get_mut(&self.key) {
            if entry.digest == self.digest && matches!(entry.state, EntryState::InFlight) {
                entry.state = EntryState::Indeterminate;
            }
        }
    }
}

type OperationKey = (String, String);

/// 进程内操作幂等表。
#[derive(Debug, Clone)]
pub struct OperationIdempotency {
    capacity: usize,
    inner: Arc<Mutex<Inner>>,
}

impl Default for OperationIdempotency {
    fn default() -> Self {
        Self::new(DEFAULT_OPERATION_CAPACITY)
    }
}

impl OperationIdempotency {
    /// 建表并给定容量 (0 按 1 处理, 至少留住一条)。
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            inner: Arc::new(Mutex::new(Inner::default())),
        }
    }

    /// 当前跟踪的条目数 (含在飞条目, 可瞬时超过容量)。
    pub fn len(&self) -> usize {
        lock(&self.inner).entries.len()
    }

    /// 是否一条都没跟踪。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 登记一次操作; 返回该走哪条路。
    pub fn begin(&self, session: SessionId, operation: OperationRequest) -> OperationAdmission {
        let OperationRequest { id, payload_digest } = operation;
        let key = (session.to_string(), id);
        let mut inner = lock(&self.inner);

        if let Some(entry) = inner.entries.get_mut(&key) {
            if entry.digest != payload_digest {
                return OperationAdmission::Conflict {
                    operation: key.1,
                    reason: PAYLOAD_MISMATCH_REASON,
                };
            }
            match &entry.state {
                EntryState::Complete(outcome) => {
                    return OperationAdmission::Replay(outcome.clone());
                }
                EntryState::InFlight => {
                    return OperationAdmission::InProgress { operation: key.1 };
                }
                EntryState::Indeterminate => {
                    return OperationAdmission::Conflict {
                        operation: key.1,
                        reason: INDETERMINATE_REASON,
                    };
                }
            }
        }

        inner.entries.insert(
            key.clone(),
            Entry {
                digest: payload_digest,
                state: EntryState::InFlight,
            },
        );
        inner.order.push_back(key.clone());
        inner.evict_over_capacity(self.capacity);

        OperationAdmission::Fresh(OperationGuard {
            inner: Arc::clone(&self.inner),
            key,
            digest: payload_digest,
            finished: false,
        })
    }
}

impl Runtime {
    /// 登记一次操作 (C2-1): 新鲜请求给出执行守卫, 终态回放/冲突/在飞在此裁决。
    ///
    /// 调用方在拿到的 Fresh 守卫上跑回合并交回终态。
    pub fn begin_operation(
        &self,
        session: SessionId,
        operation: OperationRequest,
    ) -> OperationAdmission {
        self.operation_idempotency.begin(session, operation)
    }
}

#[derive(Debug)]
struct Entry {
    digest: [u8; 32],
    state: EntryState,
}

#[derive(Debug)]
enum EntryState {
    InFlight,
    Complete(TurnOutcome),
    Indeterminate,
}

#[derive(Debug, Default)]
struct Inner {
    entries: HashMap<OperationKey, Entry>,
    order: VecDeque<OperationKey>,
}

impl Inner {
    fn evict_over_capacity(&mut self, capacity: usize) {
        while self.entries.len() > capacity {
            let evictable = self
                .order
                .iter()
                .position(|key| match self.entries.get(key) {
                    Some(entry) => !matches!(entry.state, EntryState::InFlight),
                    // 顺序队列里的陈旧键 (已不在表中) 一并清掉。
                    None => true,
                });
            let Some(index) = evictable else {
                // 只剩在飞条目: 容量守则让位于在飞保护。
                break;
            };
            if let Some(key) = self.order.remove(index) {
                self.entries.remove(&key);
            }
        }
    }
}

fn lock(inner: &Mutex<Inner>) -> MutexGuard<'_, Inner> {
    inner
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{ApprovalId, CapabilityId, RequestId, Timestamp, TraceId};
    use apeireth_protocol::canonical::{NormalizedUsage, ToolCall};

    use crate::canonical::approval::PendingApprovalView;
    use crate::canonical::execute::TurnResponse;
    use crate::canonical::trace::ExecutionTrace;

    fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn request(key: &str, payload: [u8; 32]) -> OperationRequest {
        OperationRequest::new(key, payload).expect("the test key is valid")
    }

    fn completed(text: &str) -> TurnOutcome {
        let session = SessionId::new();
        let request = RequestId::new();
        TurnOutcome::Completed(TurnResponse {
            session,
            request,
            text: text.to_string(),
            served_by: CapabilityId::new("test.provider").expect("valid capability id"),
            usage: NormalizedUsage::default(),
            rounds: 1,
            trace: ExecutionTrace::new(TraceId::new(), session, request),
        })
    }

    fn pending(approval: ApprovalId) -> TurnOutcome {
        let at = Timestamp::from_epoch_millis(1).expect("valid timestamp");
        TurnOutcome::PendingApproval(PendingApprovalView {
            approval_id: approval,
            session_id: SessionId::new(),
            request_id: RequestId::new(),
            trace_id: TraceId::new(),
            round: 1,
            capability_id: CapabilityId::new("tool.gated").expect("valid capability id"),
            tool_name: "gated".into(),
            tool_call: ToolCall {
                id: "call_1".into(),
                name: "gated".into(),
                arguments: serde_json::json!({}),
            },
            effective_invocation: None,
            governance_hook: "policy".into(),
            governance_reason: "needs a human".into(),
            command_text: "gated".into(),
            arguments_summary: "{}".into(),
            authorization_families: Vec::new(),
            action_binding_digest: String::new(),
            created_at: at,
            expires_at: at,
            operation_fingerprint: "fp".into(),
            generated_text: String::new(),
            rounds_used: 1,
        })
    }

    #[test]
    fn operation_keys_follow_the_stable_id_grammar() {
        let longest = "x".repeat(MAX_OPERATION_KEY_BYTES);
        let too_long = "x".repeat(MAX_OPERATION_KEY_BYTES + 1);
        for key in ["a", "run-1", "chat.turn_2", longest.as_str()] {
            assert!(
                validate_operation_key(key).is_ok(),
                "expected {key:?} to be accepted"
            );
        }
        for key in [
            "",
            "-run",
            "Run",
            "run@1",
            too_long.as_str(),
            "run..twice",
            "run.",
            ".run",
            "run with space",
        ] {
            assert!(
                validate_operation_key(key).is_err(),
                "expected {key:?} to be rejected"
            );
        }
    }

    #[test]
    fn a_completed_operation_replays_its_stored_outcome() {
        let table = OperationIdempotency::new(8);
        let session = SessionId::new();
        let OperationAdmission::Fresh(guard) = table.begin(session, request("op-once", digest(7)))
        else {
            panic!("the first admission must be fresh");
        };
        guard.complete(completed("first answer"));

        match table.begin(session, request("op-once", digest(7))) {
            OperationAdmission::Replay(TurnOutcome::Completed(response)) => {
                assert_eq!(response.text, "first answer");
            }
            other => panic!("expected a replay, got {other:?}"),
        }
    }

    #[test]
    fn a_pending_approval_replays_the_same_approval_view() {
        let table = OperationIdempotency::new(8);
        let session = SessionId::new();
        let approval = ApprovalId::new();
        let OperationAdmission::Fresh(guard) =
            table.begin(session, request("op-pending", digest(9)))
        else {
            panic!("the first admission must be fresh");
        };
        guard.complete(pending(approval));

        match table.begin(session, request("op-pending", digest(9))) {
            OperationAdmission::Replay(TurnOutcome::PendingApproval(view)) => {
                assert_eq!(view.approval_id, approval);
                assert_eq!(view.tool_name, "gated");
            }
            other => panic!("expected the approval view to replay, got {other:?}"),
        }
    }

    #[test]
    fn a_second_session_with_the_same_key_is_fresh() {
        let table = OperationIdempotency::new(8);
        let first = SessionId::new();
        let second = SessionId::new();
        let OperationAdmission::Fresh(guard) = table.begin(first, request("op-shared", digest(1)))
        else {
            panic!("the first admission must be fresh");
        };
        guard.complete(completed("session one"));

        assert!(matches!(
            table.begin(second, request("op-shared", digest(1))),
            OperationAdmission::Fresh(_)
        ));
    }

    #[test]
    fn the_same_key_in_flight_is_in_progress() {
        let table = OperationIdempotency::new(8);
        let session = SessionId::new();
        let OperationAdmission::Fresh(_guard) = table.begin(session, request("op-live", digest(2)))
        else {
            panic!("the first admission must be fresh");
        };
        match table.begin(session, request("op-live", digest(2))) {
            OperationAdmission::InProgress { operation } => assert_eq!(operation, "op-live"),
            other => panic!("expected in progress, got {other:?}"),
        }
    }

    #[test]
    fn a_different_payload_conflicts_in_flight_and_after_completion() {
        let table = OperationIdempotency::new(8);
        let session = SessionId::new();
        let OperationAdmission::Fresh(guard) =
            table.begin(session, request("op-payload", digest(3)))
        else {
            panic!("the first admission must be fresh");
        };

        match table.begin(session, request("op-payload", digest(4))) {
            OperationAdmission::Conflict { operation, reason } => {
                assert_eq!(operation, "op-payload");
                assert_eq!(reason, PAYLOAD_MISMATCH_REASON);
            }
            other => panic!("expected a payload conflict, got {other:?}"),
        }

        guard.complete(completed("done"));
        assert!(matches!(
            table.begin(session, request("op-payload", digest(4))),
            OperationAdmission::Conflict { .. }
        ));
    }

    #[test]
    fn dropping_the_guard_makes_the_key_indeterminate() {
        let table = OperationIdempotency::new(8);
        let session = SessionId::new();
        {
            let OperationAdmission::Fresh(guard) =
                table.begin(session, request("op-crashed", digest(5)))
            else {
                panic!("the first admission must be fresh");
            };
            drop(guard);
        }

        match table.begin(session, request("op-crashed", digest(5))) {
            OperationAdmission::Conflict { operation, reason } => {
                assert_eq!(operation, "op-crashed");
                assert_eq!(reason, INDETERMINATE_REASON);
            }
            other => panic!("expected an indeterminate conflict, got {other:?}"),
        }
    }

    #[test]
    fn capacity_evicts_the_oldest_finished_operation() {
        let table = OperationIdempotency::new(2);
        let session = SessionId::new();
        for key in ["op-a", "op-b"] {
            let OperationAdmission::Fresh(guard) = table.begin(session, request(key, digest(6)))
            else {
                panic!("{key} must start fresh");
            };
            guard.complete(completed(key));
        }
        assert_eq!(table.len(), 2);

        let OperationAdmission::Fresh(_guard) = table.begin(session, request("op-c", digest(6)))
        else {
            panic!("op-c must start fresh");
        };
        assert_eq!(table.len(), 2);

        // op-a was evicted, so it starts over; op-c is still in flight and stays.
        assert!(matches!(
            table.begin(session, request("op-a", digest(6))),
            OperationAdmission::Fresh(_)
        ));
    }

    #[test]
    fn capacity_never_evicts_an_in_flight_operation() {
        let table = OperationIdempotency::new(1);
        let session = SessionId::new();
        let OperationAdmission::Fresh(_first) = table.begin(session, request("op-a", digest(8)))
        else {
            panic!("op-a must start fresh");
        };
        let OperationAdmission::Fresh(_second) = table.begin(session, request("op-b", digest(8)))
        else {
            panic!("op-b must start fresh");
        };
        assert_eq!(table.len(), 2);
        assert!(matches!(
            table.begin(session, request("op-a", digest(8))),
            OperationAdmission::InProgress { .. }
        ));
    }
}
