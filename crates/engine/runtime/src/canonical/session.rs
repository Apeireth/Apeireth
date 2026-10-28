//! Sessions: the conversation the runtime is orchestrating over.
//!
//! A session owns the transcript and nothing else. It has no emotion, no memory
//! policy, no persona — those are companion concerns that consume a session, not
//! parts of one. The nested `reconstruction_v2` prototype fused them, which is
//! why its session type could not be reused by anything that was not the
//! companion.
//!
//! [`SessionStore`] is async because every real backend is. Retrofitting async
//! into a synchronous trait later would break every caller in the runtime, so it
//! is async from the start even though the only implementation here is in-memory.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use apeireth_core::kernel::{
    ApprovalId, CapabilityId, Clock, RequestId, SessionId, Timestamp, TraceId,
};
use apeireth_orchestration::compaction_checkpoint::{
    CompactionLogEntry, CompactionMessage, CompactionRole,
};
use apeireth_orchestration::plan_mode::PlanModeEvent;
use apeireth_orchestration::CognitiveQuota;
use apeireth_protocol::canonical::{ContentPart, MessageRole, NormalizedMessage};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, OwnedMutexGuard};

use super::approval::PendingApproval;
use super::error::{RuntimeError, RuntimeResult};
use super::event_log::{
    compaction_entries, fold_surface, fork_placeholder, open_tool_calls, ForkRecord, LogEntry,
    SurfaceOp, SurfaceView,
};

/// Session-level permission posture applied to subsequent turns.
///
/// This is part of the durable session record, not a live runtime policy: it
/// changes what the agent loop does on the *next* turn, and it never rewrites
/// an approval that is already in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionPreset {
    /// Refuse every write/execute tool call with a human-readable explanation.
    ReadOnly,
    /// Dangerous tools require human approval (the default behaviour).
    Standard,
    /// Dangerous tools are allowed per policy without approval, but keep the
    /// full audit log.
    Full,
}

impl Default for PermissionPreset {
    fn default() -> Self {
        Self::Standard
    }
}

/// Durable, session-scoped model and permission settings.
///
/// `model: None` means "follow the runtime/global default". `permission_preset`
/// defaults to [`PermissionPreset::Standard`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSettings {
    /// Session-level model override. `None` = follow the global default.
    #[serde(default)]
    pub model: Option<String>,
    /// Session-level permission posture for subsequent turns.
    #[serde(default)]
    pub permission_preset: PermissionPreset,
    /// Remember a human approval for a `(session, capability)` pair and skip
    /// the next identical approval prompt in this session.
    ///
    /// Defaults to `false` (every occurrence still requires approval). The
    /// memory itself lives in-process in the governance hook, so it is cleared
    /// on restart; this flag only controls whether that memory is consulted.
    #[serde(default)]
    pub approval_remember: bool,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            model: None,
            permission_preset: PermissionPreset::Standard,
            approval_remember: false,
        }
    }
}

/// One conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "SessionRepr")]
pub struct Session {
    /// Stable identity.
    pub id: SessionId,
    /// Immutable Host-authorized memory/profile identity; absent in legacy records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<apeireth_plugin::agents::AgentPrincipal>,
    /// Persisted scope ceiling for identified profiles; a fork cannot widen it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_scope: Option<Vec<CapabilityId>>,
    /// Durable session-scoped settings (model override + permission preset).
    ///
    /// `#[serde(default)]` is the on-disk migration path: sessions persisted
    /// before this field existed deserialize with [`SessionSettings::default`]
    /// and lose nothing from their transcript or approval history.
    #[serde(default)]
    pub settings: SessionSettings,
    /// Session-level cognitive quota ledger: the durable consumption
    /// accounting behind the opt-in spend gate (token usage accumulated from
    /// per-round usage metering, tool steps counted per dispatched call).
    ///
    /// `#[serde(default)]` is the on-disk migration path: sessions persisted
    /// before this field existed load an unlimited, empty ledger and lose
    /// nothing from their transcript or approval history.
    #[serde(default)]
    pub quota: CognitiveQuota,
    /// The transcript, in order. Includes assistant tool-call messages and
    /// tool-result messages, so a resumed session can continue mid-tool-loop.
    ///
    /// This is the materialized projection of [`Session::log`], kept in
    /// lockstep with it by [`Session::append`] / [`Session::extend`]. It is
    /// never rewritten or truncated: surface changes happen in the derived
    /// view only, so every original stays replayable here.
    pub messages: Vec<NormalizedMessage>,
    /// Structured execution facts needed to reconstruct denied and failed
    /// turns without polluting the provider transcript.
    pub events: Vec<SessionEvent>,
    /// The append-only event log: the sole authority over this session's
    /// surface. Message appends, surface changes, masks, and fork evidence are
    /// recorded here and never rewritten or deleted — masked and replaced
    /// originals stay in the log forever. Provider messages are derived from
    /// it by one pure fold ([`Session::surface_view`]), so the same log always
    /// replays to the same view.
    ///
    /// `#[serde(default)]` is the on-disk migration path: sessions persisted
    /// before the log existed rebuild it from their transcript and facts on
    /// load (see [`SessionRepr`]).
    #[serde(default)]
    pub log: Vec<LogEntry>,
    /// For a forked session: how many records of its parent's log it inherited
    /// — the exact fork cut. `None` for a session that was not forked.
    #[serde(default)]
    pub inherited_event_count: Option<usize>,
    /// Every approval this session has produced, keyed by stable [`ApprovalId`].
    /// Terminal approvals are retained for audit and idempotency.
    pub approvals: BTreeMap<ApprovalId, PendingApproval>,
    /// The currently active pending approval, when the session is paused.
    pub active_approval_id: Option<ApprovalId>,
    /// Monotonic state revision. It advances whenever a message or event is
    /// appended, including on a failed turn.
    pub revision: u64,
    /// When the session was created.
    pub created_at: Timestamp,
    /// When it was last written to.
    pub updated_at: Timestamp,
}

impl Session {
    /// A new, empty session.
    pub fn new(id: SessionId, clock: &dyn Clock) -> Self {
        let now = Timestamp::from_clock(clock);
        Self {
            id,
            principal: None,
            tool_scope: None,
            messages: Vec::new(),
            events: Vec::new(),
            log: Vec::new(),
            inherited_event_count: None,
            approvals: BTreeMap::new(),
            active_approval_id: None,
            settings: SessionSettings::default(),
            quota: CognitiveQuota::default(),
            revision: 0,
            created_at: now,
            updated_at: now,
        }
    }

    /// Append a message and update the modification time.
    ///
    /// The transcript and the event log advance together: the log gains the
    /// append record that is the authority for this message, so the log is
    /// complete for every message the session has ever held.
    pub fn append(&mut self, message: NormalizedMessage, clock: &dyn Clock) {
        let seq = self.messages.len();
        self.messages.push(message.clone());
        self.log.push(LogEntry {
            at: Timestamp::from_clock(clock),
            request: None,
            trace: None,
            event: SessionEventKind::MessageAppended { seq, message },
        });
        self.touch(clock);
    }

    /// Append several messages, touching the modification time once.
    pub fn extend(
        &mut self,
        messages: impl IntoIterator<Item = NormalizedMessage>,
        clock: &dyn Clock,
    ) {
        let at = Timestamp::from_clock(clock);
        for message in messages {
            let seq = self.messages.len();
            self.messages.push(message.clone());
            self.log.push(LogEntry {
                at,
                request: None,
                trace: None,
                event: SessionEventKind::MessageAppended { seq, message },
            });
        }
        self.touch(clock);
    }

    /// Append one structured execution event.
    ///
    /// The event is mirrored into the authoritative log with its provenance,
    /// so the log carries the session's whole event stream — facts and surface
    /// records alike — in one order. A fork cuts that one stream.
    pub fn record(
        &mut self,
        request: RequestId,
        trace: TraceId,
        event: SessionEventKind,
        clock: &dyn Clock,
    ) {
        let at = Timestamp::from_clock(clock);
        self.events.push(SessionEvent {
            at,
            request,
            trace,
            event: event.clone(),
        });
        self.log.push(LogEntry {
            at,
            request: Some(request),
            trace: Some(trace),
            event,
        });
        self.touch(clock);
    }

    /// Number of messages in the transcript.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the transcript is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// The append-only compaction log, in event order: marker-pair entries and
    /// the checkpoints they bracket. Deriving provider messages folds this log
    /// over the transcript.
    ///
    /// The projection comes from the authoritative event log, which carries
    /// the same compaction records in the same order (with provenance) — one
    /// mapping serves both this planner-facing read and the derived-view fold.
    pub fn compaction_log(&self) -> Vec<CompactionLogEntry> {
        compaction_entries(&self.log)
    }

    /// The append-only plan-mode ledger entries of this session, in event
    /// order. Folding them rebuilds the session's collaborative plan-mode
    /// projection (posture, `state_version`, parked switch), so a resumed or
    /// rebuilt session recovers its mode from the log alone.
    pub fn plan_mode_log(&self) -> Vec<PlanModeEvent> {
        self.events
            .iter()
            .filter_map(|event| match &event.event {
                SessionEventKind::PlanMode { entry } => Some(entry.clone()),
                _ => None,
            })
            .collect()
    }

    /// The provider-facing message view, derived from the event log by one
    /// pure fold: closed compaction checkpoints surface-replace their spans
    /// with their summaries, explicit surface changes replace their spans with
    /// new content, and masked messages drop out — nothing else moves. The
    /// transcript and the log are never modified, so this is a deterministic
    /// replay: folding the same log twice yields the same view.
    pub fn provider_view(&self) -> Vec<NormalizedMessage> {
        self.surface_view().messages()
    }

    /// The full derived surface behind [`Session::provider_view`]: segments,
    /// retained originals (masked and replaced ones included), and every log
    /// record the fold refused.
    pub fn surface_view(&self) -> SurfaceView {
        fold_surface(&self.log)
    }

    /// Mask message `seq` out of every derived view.
    ///
    /// Masking hides, it never deletes: the append record and the original
    /// message stay in the log and the transcript, queryable at any later
    /// time. The record-time check only validates the sequence number; the
    /// fold additionally refuses a mask that lands inside a standing
    /// replacement or on an already masked message.
    pub fn mask_message(
        &mut self,
        seq: usize,
        reason: impl Into<String>,
        clock: &dyn Clock,
    ) -> RuntimeResult<()> {
        if seq >= self.messages.len() {
            return Err(RuntimeError::Session {
                session: self.id,
                operation: "masked",
                reason: format!(
                    "message {seq} is past the transcript ({})",
                    self.messages.len()
                ),
            });
        }
        self.log.push(LogEntry {
            at: Timestamp::from_clock(clock),
            request: None,
            trace: None,
            event: SessionEventKind::Masked {
                seq,
                reason: reason.into(),
            },
        });
        self.touch(clock);
        Ok(())
    }

    /// Apply the one legal surface change: replace `[start_seq, end_seq)` of
    /// the derived view with `replacement` — empty truncates the span away,
    /// new content rewrites it. The replaced originals are kept on record in
    /// the log and the transcript.
    pub fn replace_surface(
        &mut self,
        op: SurfaceOp,
        replacement: Vec<NormalizedMessage>,
        note: impl Into<String>,
        clock: &dyn Clock,
    ) -> RuntimeResult<()> {
        let (start_seq, end_seq) = op.span();
        if start_seq >= end_seq {
            return Err(RuntimeError::Session {
                session: self.id,
                operation: "replaced",
                reason: format!("surface change [{start_seq}, {end_seq}) covers an empty span"),
            });
        }
        if end_seq > self.messages.len() {
            return Err(RuntimeError::Session {
                session: self.id,
                operation: "replaced",
                reason: format!(
                    "surface change [{start_seq}, {end_seq}) reaches past the transcript ({})",
                    self.messages.len()
                ),
            });
        }
        self.log.push(LogEntry {
            at: Timestamp::from_clock(clock),
            request: None,
            trace: None,
            event: SessionEventKind::SurfaceReplaced {
                op,
                replacement,
                note: note.into(),
            },
        });
        self.touch(clock);
        Ok(())
    }

    /// Fork this session at `prefix_end_seq`: the exact prefix of the event
    /// log becomes a new session, `inherited_event_count` records the cut, and
    /// no record after the cut is carried over.
    ///
    /// Tool calls the inherited prefix leaves open get a placeholder result
    /// ([`fork_placeholder`]) so the new session's view is complete; this
    /// session keeps the fork as children evidence ([`Session::children`]).
    /// Approval state is live turn state and stays with this session.
    pub fn fork_session(
        &mut self,
        prefix_end_seq: usize,
        clock: &dyn Clock,
    ) -> RuntimeResult<Session> {
        if prefix_end_seq > self.log.len() {
            return Err(RuntimeError::Session {
                session: self.id,
                operation: "forked",
                reason: format!(
                    "fork point {prefix_end_seq} is past the event log ({})",
                    self.log.len()
                ),
            });
        }
        let mut child = Session::new(SessionId::new(), clock);
        child.settings = self.settings.clone();
        child.principal = self.principal.clone();
        child.tool_scope = self.tool_scope.clone();
        child.inherited_event_count = Some(prefix_end_seq);
        child.log = self.log[..prefix_end_seq].to_vec();
        // The facts of the inherited prefix: the records that carry execution
        // provenance, exactly as they happened before the cut.
        child.events = child
            .log
            .iter()
            .filter_map(|entry| {
                Some(SessionEvent {
                    at: entry.at,
                    request: entry.request?,
                    trace: entry.trace?,
                    event: entry.event.clone(),
                })
            })
            .collect();
        // The transcript of the inherited prefix: every appended original in
        // it, in order — nothing appended after the cut.
        child.messages = child
            .log
            .iter()
            .filter_map(|entry| match &entry.event {
                SessionEventKind::MessageAppended { message, .. } => Some(message.clone()),
                _ => None,
            })
            .collect();
        // Close the tool calls the prefix left open so the child's view is
        // complete.
        for call_id in open_tool_calls(&child.messages) {
            child.append(fork_placeholder(&call_id), clock);
        }
        // Children evidence on the parent, appended after the cut so the
        // child's inherited prefix stays exact.
        self.log.push(LogEntry {
            at: Timestamp::from_clock(clock),
            request: None,
            trace: None,
            event: SessionEventKind::SessionForked {
                child_id: child.id,
                prefix_end_seq,
            },
        });
        self.touch(clock);
        Ok(child)
    }

    /// Fork evidence: every child session forked from this one, in fork order.
    pub fn children(&self) -> Vec<ForkRecord> {
        self.log
            .iter()
            .filter_map(|entry| match &entry.event {
                SessionEventKind::SessionForked {
                    child_id,
                    prefix_end_seq,
                } => Some(ForkRecord {
                    child_id: *child_id,
                    prefix_end_seq: *prefix_end_seq,
                }),
                _ => None,
            })
            .collect()
    }

    /// The planner-level shape of one transcript message.
    pub fn compaction_shape(message: &NormalizedMessage) -> CompactionMessage {
        CompactionMessage {
            role: match message.role {
                MessageRole::System => CompactionRole::System,
                MessageRole::User => CompactionRole::User,
                MessageRole::Assistant => CompactionRole::Assistant,
                MessageRole::Tool => CompactionRole::Tool,
            },
            text: ContentPart::join_text(&message.content),
            tool_call_ids: message
                .tool_calls
                .iter()
                .map(|call| call.id.clone())
                .collect(),
            tool_result_id: message.tool_call_id.clone(),
        }
    }

    fn touch(&mut self, clock: &dyn Clock) {
        self.revision = self.revision.saturating_add(1);
        self.updated_at = Timestamp::from_clock(clock);
    }
}

/// One persisted execution fact for a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEvent {
    /// When it happened according to the runtime clock.
    pub at: Timestamp,
    /// Inbound request that caused it.
    pub request: RequestId,
    /// Execution trace that caused it.
    pub trace: TraceId,
    /// Structured event payload.
    pub event: SessionEventKind,
}

pub use super::session_events::SessionEventKind;

/// The on-disk shape of a [`Session`], and the migration path into one.
///
/// Sessions persisted before the event log existed carry their transcript and
/// facts without log records. Deserialization rebuilds the log from them once,
/// at the load boundary, so from then on the log is the complete authority and
/// every derived view replays exactly. Fields mirror [`Session`] one for one;
/// when a field is added to [`Session`], add it here too.
#[derive(Debug, Clone, Deserialize)]
struct SessionRepr {
    id: SessionId,
    #[serde(default)]
    principal: Option<apeireth_plugin::agents::AgentPrincipal>,
    #[serde(default)]
    tool_scope: Option<Vec<CapabilityId>>,
    #[serde(default)]
    settings: SessionSettings,
    #[serde(default)]
    quota: CognitiveQuota,
    messages: Vec<NormalizedMessage>,
    events: Vec<SessionEvent>,
    #[serde(default)]
    log: Vec<LogEntry>,
    #[serde(default)]
    inherited_event_count: Option<usize>,
    approvals: BTreeMap<ApprovalId, PendingApproval>,
    active_approval_id: Option<ApprovalId>,
    revision: u64,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl From<SessionRepr> for Session {
    fn from(repr: SessionRepr) -> Self {
        let mut session = Self {
            id: repr.id,
            principal: repr.principal,
            tool_scope: repr.tool_scope,
            settings: repr.settings,
            quota: repr.quota,
            messages: repr.messages,
            events: repr.events,
            log: repr.log,
            inherited_event_count: repr.inherited_event_count,
            approvals: repr.approvals,
            active_approval_id: repr.active_approval_id,
            revision: repr.revision,
            created_at: repr.created_at,
            updated_at: repr.updated_at,
        };
        if session.log.is_empty() {
            session.log =
                rebuilt_event_stream(&session.messages, &session.events, session.created_at);
        }
        session
    }
}

/// Rebuild the event log of a session persisted before the log existed: every
/// transcript message stands in as its own append record, then every recorded
/// fact with its provenance, in record order.
///
/// The old shape kept the two apart, and the fold does not depend on how
/// appends and facts interleave — appends establish the numbering, facts apply
/// in their own order — so the rebuilt log replays to exactly the view the
/// session had before its migration.
fn rebuilt_event_stream(
    messages: &[NormalizedMessage],
    events: &[SessionEvent],
    at: Timestamp,
) -> Vec<LogEntry> {
    let mut log = Vec::with_capacity(messages.len() + events.len());
    for (seq, message) in messages.iter().enumerate() {
        log.push(LogEntry {
            at,
            request: None,
            trace: None,
            event: SessionEventKind::MessageAppended {
                seq,
                message: message.clone(),
            },
        });
    }
    for event in events {
        log.push(LogEntry {
            at: event.at,
            request: Some(event.request),
            trace: Some(event.trace),
            event: event.event.clone(),
        });
    }
    log
}

/// Where sessions live.
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Load a session, or `None` if it does not exist.
    async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>>;

    /// Persist a session, creating or replacing it.
    async fn save(&self, session: &Session) -> RuntimeResult<()>;

    /// List every stored session, most recently updated first.
    ///
    /// Panel/introspection surface (`GET /v1/panel/sessions`). Ordering is part
    /// of the contract: the frontend renders the newest conversation first.
    async fn list(&self) -> RuntimeResult<Vec<Session>>;

    /// Remove the stored session record.
    ///
    /// Returns `Ok(true)` when a record was removed, `Ok(false)` when the id
    /// was not present. This deletes the durable transcript record — callers
    /// own the retention decision.
    async fn delete(&self, id: &SessionId) -> RuntimeResult<bool>;

    /// Whether `id` is present in this store (membership probe, W3-③ 域事件
    /// 过滤面: 本域库在册 = 本域会话, 分库之下他域会话按本域库读作不存在)。
    ///
    /// 默认实现经 [`SessionStore::load`] 判定; 存储后端可覆写成只查存在性的
    /// 轻量实现 (不载入会话 JSON)。**存在性本身不含内容**, 探测口只回答在册与否。
    async fn contains(&self, id: &SessionId) -> RuntimeResult<bool> {
        Ok(self.load(id).await?.is_some())
    }
}

/// A session store held in process memory.
///
/// Process-local only. For process-restart durability, compose the runtime with
/// [`SqliteSessionStore`] or another durable [`SessionStore`] implementation.
#[derive(Debug, Default)]
pub struct InMemorySessionStore {
    sessions: Mutex<BTreeMap<SessionId, Session>>,
}

impl InMemorySessionStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many sessions are held.
    pub async fn len(&self) -> usize {
        self.sessions.lock().await.len()
    }

    /// Whether the store is empty.
    pub async fn is_empty(&self) -> bool {
        self.sessions.lock().await.is_empty()
    }
}

#[async_trait]
impl SessionStore for InMemorySessionStore {
    async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>> {
        Ok(self.sessions.lock().await.get(id).cloned())
    }

    async fn save(&self, session: &Session) -> RuntimeResult<()> {
        self.sessions
            .lock()
            .await
            .insert(session.id, session.clone());
        Ok(())
    }

    async fn list(&self) -> RuntimeResult<Vec<Session>> {
        let mut all: Vec<Session> = self.sessions.lock().await.values().cloned().collect();
        all.sort_by(|a, b| {
            b.updated_at
                .epoch_millis()
                .cmp(&a.updated_at.epoch_millis())
        });
        Ok(all)
    }

    async fn delete(&self, id: &SessionId) -> RuntimeResult<bool> {
        Ok(self.sessions.lock().await.remove(id).is_some())
    }
}

/// 一次会话写的排队事实 (W2-③ 多客户端会话复用, 审计/观测用)。
///
/// 这是**事实记录**, 不是同步原语: 由 [`SessionWritePermit`] 在入场时测得,
/// 供调用方写审计行 (例如 `gateway.session_client`) 或排障。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionWriteAdmission {
    /// 入场时排在前面的写者数 (持闸者 + 排队者)。0 = 直接持闸, 未排队。
    pub queue_position: usize,
    /// 实际等待持闸的时长。
    pub waited: Duration,
}

impl SessionWriteAdmission {
    /// 这次写是否排过队 (入场时前面有写者)。
    pub fn was_queued(&self) -> bool {
        self.queue_position > 0
    }
}

/// 同会话写闸的一条车道: 一把先到先服务的互斥锁 + 在场写者计数。
#[derive(Debug)]
struct WriteLane {
    /// 先到先服务 (tokio 互斥锁按请求顺序放行): 同会话写者串行, 不交叠。
    /// `Arc` 形态让通行证持有**属主守卫**而非借用守卫 (通行证自持车道)。
    lock: Arc<Mutex<()>>,
    /// 已入场未退场的写者 (持闸者 + 排队者), 排队事实的来源。
    writers: AtomicUsize,
}

impl WriteLane {
    fn new() -> Self {
        Self {
            lock: Arc::new(Mutex::new(())),
            writers: AtomicUsize::new(0),
        }
    }
}

/// 同时驻留的写车道上限。语义与运行时的 per-session 回合锁一致 (M20):
/// 超限时只驱逐"无持有者"的车道, 正在持闸/排队的车道永不丢失。
const MAX_SESSION_WRITE_LANES: usize = 4096;

/// 会话写闸 (W2-③ 多客户端会话复用): **同一会话的账本写先到先服务串行**,
/// 会话之间互不阻塞。
///
/// ## 这把闸承诺什么
///
/// 持有 [`SessionWritePermit`] 期间, 同一会话不会有第二个写者交叠写账本
/// (消息/事件日志/设置): 回合提交、审批续跑、设置补丁、会话删除都从
/// [`SessionManager::write_permit`] 入场, 先到先服务排队 (第二回合**排队**,
/// 不发 busy 帧 —— 排队语义见 `docs/04-internal/backend-enhancement/`
/// W2-3 交付记录的决策表)。
///
/// ## 这把闸不承诺什么
///
/// 绕过会话写入口、直接调用 `Runtime`/`SessionStore` 写面的调用方不经此闸;
/// 回合之间的互斥由运行时自身的 per-session 回合锁兜底, 但与本闸覆盖的
/// 设置写之间的交叠窗口是如实登记的残余 (见 W2-3 交付记录残余表)。
#[derive(Debug, Default)]
pub struct SessionWriteGate {
    lanes: Mutex<BTreeMap<SessionId, Arc<WriteLane>>>,
}

impl SessionWriteGate {
    /// 入场一个写者: 同会话先到先服务串行, 会话之间并发。
    ///
    /// 返回的通行证携带排队事实 (排在前面几位 / 等了多久); 释放即让位
    /// (RAII, 不留悬持)。
    pub async fn admit(&self, session: SessionId) -> SessionWritePermit {
        let lane = self.lane(session).await;
        let started = std::time::Instant::now();
        // 在场写者先计数再排队: 差值即"排在我前面的写者数", 确定性排队事实。
        let queue_position = lane.writers.fetch_add(1, Ordering::SeqCst);
        let guard = lane.lock.clone().lock_owned().await;
        SessionWritePermit {
            session,
            admission: SessionWriteAdmission {
                queue_position,
                waited: started.elapsed(),
            },
            lane,
            guard: Some(guard),
        }
    }

    /// 该会话当前在场写者数 (持闸者 + 排队者) —— 排队深度观测口。
    pub async fn writers_in_flight(&self, session: &SessionId) -> usize {
        let map = self.lanes.lock().await;
        map.get(session)
            .map_or(0, |lane| lane.writers.load(Ordering::SeqCst))
    }

    /// 取 (或建) 一条车道。有界 (M20 同语义): 超限只驱逐空闲车道。
    async fn lane(&self, session: SessionId) -> Arc<WriteLane> {
        let mut map = self.lanes.lock().await;
        if let Some(existing) = map.get(&session) {
            return Arc::clone(existing);
        }
        // 无持有者 (仅 map 自己持有一份 Arc) 才可驱逐: 持闸或排队中的车道
        // 不会因为淘汰而变成另一把锁。
        if map.len() >= MAX_SESSION_WRITE_LANES {
            let evictable = map
                .iter()
                .find(|(_, lane)| Arc::strong_count(lane) == 1)
                .map(|(id, _)| *id);
            if let Some(evictable) = evictable {
                map.remove(&evictable);
            }
        }
        Arc::clone(
            map.entry(session)
                .or_insert_with(|| Arc::new(WriteLane::new())),
        )
    }
}

/// 一张会话写通行证: 持有期间该会话账本写串行 (先到先服务), 释放即让位。
///
/// 每个写者一张、RAII 管理: 进程内不设手动释放口, 悬持不可能由 API 造成。
pub struct SessionWritePermit {
    session: SessionId,
    admission: SessionWriteAdmission,
    lane: Arc<WriteLane>,
    guard: Option<OwnedMutexGuard<()>>,
}

impl std::fmt::Debug for SessionWritePermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionWritePermit")
            .field("session", &self.session)
            .field("admission", &self.admission)
            .finish_non_exhaustive()
    }
}

impl SessionWritePermit {
    /// 这张通行证把守的会话。
    pub fn session(&self) -> SessionId {
        self.session
    }

    /// 排队事实 (排在前面几位 / 等了多久)。
    pub fn admission(&self) -> SessionWriteAdmission {
        self.admission
    }
}

impl Drop for SessionWritePermit {
    fn drop(&mut self) {
        // 先放闸再退位: 后一位写者拿到锁时, 计数已让出它的位置。
        self.guard.take();
        self.lane.writers.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Loads, creates, and persists sessions against a [`SessionStore`].
///
/// Owns the clock so that a session's timestamps come from the same source as
/// the rest of the runtime's, rather than from `Utc::now()` scattered across
/// call sites.
pub struct SessionManager {
    store: Arc<dyn SessionStore>,
    clock: Arc<dyn Clock>,
    /// 会话写闸 (W2-③): 同会话账本写先到先服务串行, 会话间并发。
    writes: SessionWriteGate,
}

impl SessionManager {
    /// Build a manager over a store.
    pub fn new(store: Arc<dyn SessionStore>, clock: Arc<dyn Clock>) -> Self {
        Self {
            store,
            clock,
            writes: SessionWriteGate::default(),
        }
    }

    /// 取一张会话写通行证 (W2-③ 排队语义): 同会话写者先到先服务串行,
    /// 持证期间该会话账本写不交叠。回合提交 / 审批续跑 / 设置补丁 / 删除
    /// 等一切会话写入口都从这里入场。
    pub async fn write_permit(&self, session: SessionId) -> SessionWritePermit {
        self.writes.admit(session).await
    }

    /// 写闸观测口 (排队深度等; 面板/测试读数用)。
    pub fn write_gate(&self) -> &SessionWriteGate {
        &self.writes
    }

    /// 原子读-改-写一段会话账本: 持证跨 `load → edit → save`, 与同会话任何
    /// 在途写者零交叠 (绝不交叠写会话账本)。
    ///
    /// 返回 `Ok(None)` = 会话不存在 (未创建、未改动); 否则返回编辑结果与
    /// 这次写的排队事实。与 [`SessionManager::write_permit`] 同一把闸 —— 持证
    /// 调用本方法会自锁, 二选一。
    pub async fn update<R>(
        &self,
        session: SessionId,
        edit: impl FnOnce(&mut Session) -> R,
    ) -> RuntimeResult<Option<(R, SessionWriteAdmission)>> {
        let permit = self.write_permit(session).await;
        let admission = permit.admission();
        let Some(mut loaded) = self.load(&session).await? else {
            return Ok(None);
        };
        let result = edit(&mut loaded);
        self.save(&loaded).await?;
        Ok(Some((result, admission)))
    }

    /// Load `id`, returning `None` when it does not exist.
    pub async fn load(&self, id: &SessionId) -> RuntimeResult<Option<Session>> {
        self.store
            .load(id)
            .await
            .map_err(|e| RuntimeError::session_load(*id, e.to_string()))
    }

    /// Whether `id` is present in this manager's store (本域在册探测:
    /// 语义见 [`SessionStore::contains`])。
    pub async fn contains(&self, id: &SessionId) -> RuntimeResult<bool> {
        self.store
            .contains(id)
            .await
            .map_err(|e| RuntimeError::session_load(*id, e.to_string()))
    }

    /// The store this manager writes through (W3-③ 组合根给每域伙伴运行时
    /// 共用同一把写闸时, 治理面/预设面需要同一存储句柄)。
    pub fn store(&self) -> &Arc<dyn SessionStore> {
        &self.store
    }

    /// Load `id`, creating an empty session if it does not exist yet.
    pub async fn load_or_create(&self, id: SessionId) -> RuntimeResult<Session> {
        match self.store.load(&id).await {
            Ok(Some(session)) => Ok(session),
            Ok(None) => Ok(Session::new(id, self.clock.as_ref())),
            Err(e) => Err(RuntimeError::session_load(id, e.to_string())),
        }
    }

    /// Persist a session.
    pub async fn save(&self, session: &Session) -> RuntimeResult<()> {
        self.store
            .save(session)
            .await
            .map_err(|e| RuntimeError::session_save(session.id, e.to_string()))
    }

    /// Remove `id` from the store, returning whether a record was removed.
    pub async fn delete(&self, id: &SessionId) -> RuntimeResult<bool> {
        self.store
            .delete(id)
            .await
            .map_err(|e| RuntimeError::Session {
                session: *id,
                operation: "deleted",
                reason: e.to_string(),
            })
    }

    /// The clock this manager stamps sessions with.
    pub fn clock(&self) -> &Arc<dyn Clock> {
        &self.clock
    }
}

#[cfg(test)]
#[path = "session/tests.rs"]
mod tests;
