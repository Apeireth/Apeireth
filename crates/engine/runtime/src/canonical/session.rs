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

/// Persisted execution outcomes. No variant carries model chain-of-thought.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum SessionEventKind {
    /// A user turn entered canonical execution.
    TurnStarted,
    /// Governance denied an action.
    GovernanceDenied {
        /// Hook that decided.
        hook: String,
        /// Stable action label.
        action: String,
        /// Stated denial reason.
        reason: String,
        /// Runtime round.
        round: u32,
    },
    /// Governance suspended an action for human approval.
    ///
    /// Only capability dispatch uses this variant. The `approval_id` always
    /// corresponds to a real [`PendingApproval`] stored in the session.
    ApprovalRequired {
        /// Hook that decided.
        hook: String,
        /// Stable action label.
        action: String,
        /// What needs approval.
        reason: String,
        /// Runtime round.
        round: u32,
        /// The stable approval id that was minted for this pause.
        approval_id: ApprovalId,
    },
    /// Governance requires completion-level approval. This path is not
    /// resumable in the current phase and intentionally does not mint a
    /// stable approval id, because no pending approval entity exists.
    CompletionApprovalRequired {
        /// Hook that decided.
        hook: String,
        /// Stable action label.
        action: String,
        /// What needs approval.
        reason: String,
        /// Runtime round.
        round: u32,
    },
    /// A pending approval reached a terminal decision.
    ApprovalResolved {
        /// The stable approval id.
        approval_id: ApprovalId,
        /// `approved` / `rejected` / `expired`.
        decision: String,
        /// Round that produced the approval.
        round: u32,
        /// Optional human reason recorded at resolution time.
        human_reason: Option<String>,
    },
    /// The model repeated, in the same turn, a proposal that governance had
    /// already frozen for human approval. The repeat is folded onto the same
    /// approval item ("视为同一待批准项"): no second approval is minted, and
    /// the turn collapses directly instead of re-entering the freeze loop.
    ApprovalReentryCollapsed {
        /// Model-facing tool name of the repeated proposal.
        tool_name: String,
        /// The approval item the repeat was folded into.
        approval_id: ApprovalId,
        /// Status that item had reached when the repeat arrived.
        prior_status: String,
        /// Round of the repeated proposal.
        round: u32,
    },
    /// An approved action's binding digest did not match the live action at the
    /// execution boundary and the dispatch was refused (G2.5 拒绝即帧).
    /// Stable audit event kind: `approval.binding_mismatch`.
    ApprovalBindingMismatch {
        /// The stable approval id that authorized the frozen action.
        approval_id: ApprovalId,
        /// Model tool-call correlation id of the refused dispatch.
        tool_call_id: String,
        /// 漂移面 (`operation` / `agent` / `target` / `tool_schema_version` /
        /// `parameters` / `argument_order`): 哪一个被改了。
        drift: String,
        /// 批准时冻结的动作摘要。
        expected_digest: String,
        /// 执行边界重算的动作摘要。
        actual_digest: String,
        /// Runtime round.
        round: u32,
    },
    /// Provider routing could not serve a round.
    ProviderFailed {
        /// Legible terminal routing/provider error.
        error: String,
        /// Runtime round.
        round: u32,
    },
    /// A requested tool did not complete successfully.
    ToolFailed {
        /// Resolved capability, absent when the model named no known tool.
        capability: Option<CapabilityId>,
        /// Model tool-call correlation id.
        tool_call_id: String,
        /// Legible failure.
        error: String,
        /// Runtime round.
        round: u32,
    },
    /// Execution failed outside provider/tool/governance handling.
    ExecutionFailed {
        /// Stable phase label.
        phase: String,
        /// Legible failure.
        error: String,
    },
    /// A runtime invariant observed a violation and recorded it here
    /// (log-only mode). This is an observation record, never a verdict change.
    InvariantViolation {
        /// The invariant that fired.
        invariant: String,
        /// Module attribution for follow-up.
        module: String,
        /// What was observed.
        detail: String,
    },
    /// The turn reached a final assistant response.
    TurnCompleted {
        /// Provider round-trips taken.
        rounds: u32,
    },
    /// A compaction checkpoint write opened its marker pair.
    ///
    /// The pair (`start -> checkpoint -> closed`) is a crash-detectable lock:
    /// if the closing entry never arrives, the checkpoint is reported as
    /// unclosed and never folded into any derived view.
    CompactionStarted {
        /// Marker identity shared with the checkpoint and the closing entry.
        marker: String,
    },
    /// A compaction checkpoint: the derived provider view surface-replaces the
    /// messages `[start_seq, end_seq)` with `summary`.
    ///
    /// The transcript itself is untouched: the original messages stay in
    /// `messages` and remain replayable at any later time.
    CompactionCheckpoint {
        /// First replaced message index (inclusive).
        start_seq: usize,
        /// First unreplaced message index (exclusive).
        end_seq: usize,
        /// The summary that replaces the span in the derived view.
        summary: String,
        /// Marker identity shared with its bracketing pair.
        marker: String,
    },
    /// The compaction marker pair closed.
    CompactionClosed {
        /// Marker identity shared with the opening entry.
        marker: String,
    },
    /// One plan-mode collaborative-state ledger entry.
    ///
    /// The session's plan-mode posture is a fold over these entries (init /
    /// apply / state-version); only an `Apply` entry changes the projected
    /// posture, and one is written exclusively at the pre-step of an accepted
    /// turn. See [`Session::plan_mode_log`].
    PlanMode {
        /// The ledger entry to fold.
        entry: PlanModeEvent,
    },
    /// A message entered the transcript: the append-only record the derived
    /// views replay from. Written by [`Session::append`] / [`Session::extend`]
    /// only, so the transcript and this record advance together and the log
    /// covers every message a session has ever held.
    MessageAppended {
        /// Sequence number of the message in the append-only transcript.
        seq: usize,
        /// The message exactly as appended.
        message: NormalizedMessage,
    },
    /// A surface change on the derived view.
    ///
    /// [`SurfaceOp::Replace`] is the only legal form: truncation, compaction,
    /// and rewriting all route through it — a rewrite is "replace the span
    /// with new content" — and the replaced originals are kept on record in
    /// the log and the transcript. Compaction checkpoints are the same
    /// operation carrying a summary as replacement content.
    SurfaceReplaced {
        /// The replacement interval.
        op: SurfaceOp,
        /// New content standing in for the interval; empty truncates it away.
        replacement: Vec<NormalizedMessage>,
        /// Why the surface changed.
        note: String,
    },
    /// The message at `seq` is masked out of every derived view.
    ///
    /// The append record and the original message are never deleted — they
    /// stay replayable through the log and the transcript.
    Masked {
        /// Sequence number of the masked message.
        seq: usize,
        /// Why it was masked.
        reason: String,
    },
    /// A child session was forked from this one at `prefix_end_seq`: children
    /// evidence kept on the parent (see [`Session::children`]).
    SessionForked {
        /// The child session that was created.
        child_id: SessionId,
        /// Where the child's inherited prefix ended: it inherited exactly this
        /// many records of this session's log.
        prefix_end_seq: usize,
    },
}

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
mod tests {
    use super::*;
    use apeireth_core::kernel::VirtualClock;

    fn clock() -> Arc<dyn Clock> {
        Arc::new(VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        ))
    }

    #[tokio::test]
    async fn a_missing_session_is_created_rather_than_erroring() {
        let manager = SessionManager::new(Arc::new(InMemorySessionStore::new()), clock());
        let id = SessionId::new();

        let session = manager.load_or_create(id).await.unwrap();
        assert_eq!(session.id, id);
        assert!(session.is_empty());
    }

    #[tokio::test]
    async fn a_saved_session_round_trips_with_its_transcript() {
        let store = Arc::new(InMemorySessionStore::new());
        let manager = SessionManager::new(store.clone(), clock());
        let id = SessionId::new();

        let mut session = manager.load_or_create(id).await.unwrap();
        session.append(NormalizedMessage::user("hello"), clock().as_ref());
        session.append(NormalizedMessage::assistant("hi"), clock().as_ref());
        manager.save(&session).await.unwrap();

        let reloaded = manager.load_or_create(id).await.unwrap();
        assert_eq!(reloaded.len(), 2);
        assert!(
            matches!(
                &reloaded.messages[0].content[0],
                apeireth_protocol::canonical::ContentPart::Text { text } if text == "hello"
            ),
            "the transcript must survive the round trip intact"
        );
        assert_eq!(store.len().await, 1);
    }

    #[tokio::test]
    async fn saving_twice_replaces_rather_than_duplicating() {
        let store = Arc::new(InMemorySessionStore::new());
        let manager = SessionManager::new(store.clone(), clock());
        let id = SessionId::new();

        let mut session = manager.load_or_create(id).await.unwrap();
        session.append(NormalizedMessage::user("one"), clock().as_ref());
        manager.save(&session).await.unwrap();
        session.append(NormalizedMessage::user("two"), clock().as_ref());
        manager.save(&session).await.unwrap();

        assert_eq!(store.len().await, 1);
        assert_eq!(manager.load_or_create(id).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn messages_and_events_advance_one_monotonic_revision() {
        let clock = clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        let request = RequestId::new();
        let trace = TraceId::new();

        session.append(NormalizedMessage::user("attempt"), clock.as_ref());
        session.record(
            request,
            trace,
            SessionEventKind::ProviderFailed {
                error: "offline".into(),
                round: 1,
            },
            clock.as_ref(),
        );

        assert_eq!(session.revision, 2);
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.events.len(), 1);
        assert_eq!(session.events[0].request, request);
        assert_eq!(session.events[0].trace, trace);
    }

    #[tokio::test]
    async fn appending_advances_the_modification_time_from_the_injected_clock() {
        let virtual_clock = VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        );
        let clock: Arc<dyn Clock> = Arc::new(virtual_clock.clone());
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        let created = session.created_at;

        virtual_clock.advance(chrono::Duration::seconds(30));
        session.append(NormalizedMessage::user("later"), clock.as_ref());

        assert_eq!(session.created_at, created, "creation time must not move");
        assert_eq!(
            session.updated_at.epoch_millis() - created.epoch_millis(),
            30_000
        );
    }

    #[tokio::test]
    async fn two_sessions_do_not_share_a_transcript() {
        let store = Arc::new(InMemorySessionStore::new());
        let manager = SessionManager::new(store.clone(), clock());

        let mut a = manager.load_or_create(SessionId::new()).await.unwrap();
        let b = manager.load_or_create(SessionId::new()).await.unwrap();
        a.append(NormalizedMessage::user("only in a"), clock().as_ref());
        manager.save(&a).await.unwrap();
        manager.save(&b).await.unwrap();

        assert_eq!(manager.load_or_create(a.id).await.unwrap().len(), 1);
        assert_eq!(manager.load_or_create(b.id).await.unwrap().len(), 0);
        assert_eq!(store.len().await, 2);
    }

    #[test]
    fn legacy_session_json_without_settings_migrates_with_defaults_and_no_loss() {
        // A session persisted before the `settings` field existed: serialize a
        // real session, strip the new `settings` key, and reload it. The
        // transcript must survive and settings must fall back to defaults.
        let clock = clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("hello"), clock.as_ref());

        let mut legacy = serde_json::to_value(&session).unwrap();
        let object = legacy
            .as_object_mut()
            .expect("session serializes as object");
        assert!(object.remove("settings").is_some());

        let migrated: Session = serde_json::from_value(legacy).expect("legacy session must load");

        assert_eq!(migrated.settings, SessionSettings::default());
        assert_eq!(migrated.settings.model, None);
        assert_eq!(
            migrated.settings.permission_preset,
            PermissionPreset::Standard
        );
        assert_eq!(
            migrated.messages.len(),
            1,
            "transcript must survive migration"
        );
        assert_eq!(migrated.revision, 1);
    }

    #[test]
    fn session_settings_without_approval_remember_migrates_to_false_without_loss() {
        // A settings blob persisted before `approval_remember` existed must
        // deserialize with `approval_remember == false` and keep every other
        // field intact.
        let settings = SessionSettings {
            model: Some("some/model".into()),
            permission_preset: PermissionPreset::Full,
            approval_remember: true,
        };

        let mut json = serde_json::to_value(&settings).unwrap();
        let object = json.as_object_mut().expect("settings serializes as object");
        assert!(object.remove("approval_remember").is_some());

        let migrated: SessionSettings = serde_json::from_value(json).unwrap();
        assert!(!migrated.approval_remember);
        assert_eq!(migrated.model, Some("some/model".into()));
        assert_eq!(migrated.permission_preset, PermissionPreset::Full);
    }

    #[test]
    fn session_settings_survive_json_roundtrip_for_each_preset() {
        // Select a tier → serialize → deserialize: a populated session (with a
        // transcript and a recorded fact) must read its settings back as the
        // same tier, with the model override and the remember flag untouched —
        // for every tier and every switch between tiers.
        let clock = clock();
        let mut session = Session::new(SessionId::new(), clock.as_ref());
        session.append(NormalizedMessage::user("hello"), clock.as_ref());
        session.record(
            RequestId::new(),
            TraceId::new(),
            SessionEventKind::TurnStarted,
            clock.as_ref(),
        );
        session.settings.model = Some("some/model".into());

        for (preset, remember) in [
            (PermissionPreset::ReadOnly, true),
            (PermissionPreset::Standard, false),
            (PermissionPreset::Full, true),
        ] {
            session.settings.permission_preset = preset;
            session.settings.approval_remember = remember;

            let json = serde_json::to_string(&session).unwrap();
            let reloaded: Session = serde_json::from_str(&json).unwrap();

            assert_eq!(reloaded.settings.permission_preset, preset);
            assert_eq!(reloaded.settings.approval_remember, remember);
            assert_eq!(reloaded.settings.model.as_deref(), Some("some/model"));
        }
    }

    #[tokio::test]
    async fn list_returns_sessions_most_recently_updated_first() {
        let virtual_clock = VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        );
        let clock: Arc<dyn Clock> = Arc::new(virtual_clock.clone());
        let store = InMemorySessionStore::new();

        let older = Session::new(SessionId::new(), clock.as_ref());
        virtual_clock.advance(chrono::Duration::seconds(30));
        let newer = Session::new(SessionId::new(), clock.as_ref());

        store.save(&older).await.unwrap();
        store.save(&newer).await.unwrap();

        let listed = store.list().await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, newer.id, "newest updated first");
        assert_eq!(listed[1].id, older.id);
    }

    #[tokio::test]
    async fn delete_removes_the_stored_session_and_reports_it() {
        let store = InMemorySessionStore::new();
        let id = SessionId::new();
        store
            .save(&Session::new(id, clock().as_ref()))
            .await
            .unwrap();
        assert_eq!(store.len().await, 1);

        assert!(store.delete(&id).await.unwrap(), "record was present");
        assert_eq!(store.len().await, 0);
        assert!(store.load(&id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn deleting_an_unknown_session_reports_nothing_removed() {
        let store = InMemorySessionStore::new();

        assert!(!store.delete(&SessionId::new()).await.unwrap());
    }

    // ------------------------------------------------------------------
    // W2-③ 会话写闸: 同会话写串行 (排队事实) / 会话间并发 / 读改写原子
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn write_permits_serialize_one_session_and_stay_concurrent_across_sessions() {
        let gate = Arc::new(SessionWriteGate::default());
        let a = SessionId::new();
        let b = SessionId::new();

        let held = gate.admit(a).await;
        assert_eq!(held.admission().queue_position, 0, "首位写者不排队");
        assert!(!held.admission().was_queued());

        // 同会话第二位写者: 排队 (不拒绝、不交叠) —— 排队事实确定性。
        let waiter_gate = Arc::clone(&gate);
        let waiter = tokio::spawn(async move { waiter_gate.admit(a).await });
        for _ in 0..100 {
            if gate.writers_in_flight(&a).await == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert_eq!(
            gate.writers_in_flight(&a).await,
            2,
            "第二位写者在场排队, 不是被拒 (排队语义钉死)"
        );

        // 不同会话不共车道: 直接持闸, 不受会话 a 的在场写者影响。
        let other = gate.admit(b).await;
        assert!(!other.admission().was_queued(), "会话之间互不阻塞");

        drop(held);
        let queued = waiter.await.unwrap();
        assert!(queued.admission().was_queued(), "排队事实必须留痕");
        assert_eq!(queued.admission().queue_position, 1, "排在原持闸者之后");

        drop(queued);
        drop(other);
        assert_eq!(gate.writers_in_flight(&a).await, 0, "退场即清计数");
        assert_eq!(gate.writers_in_flight(&b).await, 0);
    }

    #[tokio::test]
    async fn a_queued_writer_really_waits_until_the_holder_releases() {
        let gate = Arc::new(SessionWriteGate::default());
        let session = SessionId::new();

        let held = gate.admit(session).await;
        let waiter_gate = Arc::clone(&gate);
        let waiter = tokio::spawn(async move { waiter_gate.admit(session).await });

        // 持闸者不放行, 排队者绝拿不到闸 (串行不交叠)。
        for _ in 0..100 {
            assert!(!waiter.is_finished(), "持闸期间排队者不得入场 (回合串行化)");
            tokio::task::yield_now().await;
        }

        drop(held);
        let admitted = waiter.await.unwrap();
        assert!(admitted.admission().was_queued());
    }

    #[tokio::test]
    async fn concurrent_updates_never_lose_a_write_on_one_session() {
        // 对抗压测: 16 个写者并发读-改-写同一会话, 每人追加一条消息 ——
        // 交叠写会丢更新, 全量到齐即证零交叠。
        let clock = clock();
        let manager = Arc::new(SessionManager::new(
            Arc::new(InMemorySessionStore::new()),
            Arc::clone(&clock),
        ));
        let id = SessionId::new();
        manager
            .save(&manager.load_or_create(id).await.unwrap())
            .await
            .unwrap();

        let mut writers = Vec::new();
        for n in 0..16u32 {
            let manager = Arc::clone(&manager);
            let clock = Arc::clone(&clock);
            writers.push(tokio::spawn(async move {
                manager
                    .update(id, |session| {
                        session.append(NormalizedMessage::user(format!("msg-{n}")), clock.as_ref());
                    })
                    .await
                    .unwrap()
                    .expect("session exists")
            }));
        }
        for writer in writers {
            let (_, admission) = writer.await.unwrap();
            assert!(admission.queue_position < 16, "排队事实有界");
        }

        let session = manager.load_or_create(id).await.unwrap();
        assert_eq!(session.len(), 16, "16 次追加一条不丢 (零交叠写)");
        let appended = session
            .log
            .iter()
            .filter(|entry| matches!(entry.event, SessionEventKind::MessageAppended { .. }))
            .count();
        assert_eq!(appended, 16, "账本与转写 lockstep: 每条消息恰有一笔日志");
    }

    #[tokio::test]
    async fn update_reports_missing_sessions_without_creating_them() {
        let manager = SessionManager::new(Arc::new(InMemorySessionStore::new()), clock());
        let id = SessionId::new();

        let outcome = manager.update(id, |_| ()).await.unwrap();
        assert!(outcome.is_none(), "不存在的会话不因读改写而凭空出现");
    }

    #[tokio::test]
    async fn a_stored_session_list_is_one_coherent_snapshot_under_concurrent_writes() {
        // 面板读快照语义 (W2-③): `list()` 一次读 = 一个自洽快照 —— 写者并发
        // 期间, 每个快照行必须是某个时刻的完整会话 (转写与账本永不撕裂)。
        let clock = clock();
        let store = Arc::new(InMemorySessionStore::new());
        let manager = Arc::new(SessionManager::new(
            Arc::clone(&store) as Arc<dyn SessionStore>,
            Arc::clone(&clock),
        ));
        let ids: Vec<SessionId> = (0..4).map(|_| SessionId::new()).collect();
        for id in &ids {
            manager
                .save(&manager.load_or_create(*id).await.unwrap())
                .await
                .unwrap();
        }

        let mut writers = Vec::new();
        for id in &ids {
            for n in 0..8usize {
                let manager = Arc::clone(&manager);
                let clock = Arc::clone(&clock);
                let id = *id;
                writers.push(tokio::spawn(async move {
                    manager
                        .update(id, |session| {
                            session
                                .append(NormalizedMessage::user(format!("w-{n}")), clock.as_ref());
                        })
                        .await
                        .unwrap();
                }));
            }
        }

        let mut snapshots = Vec::new();
        for _ in 0..32 {
            snapshots.push(store.list().await.unwrap());
            tokio::task::yield_now().await;
        }
        for writer in writers {
            writer.await.unwrap();
        }
        snapshots.push(store.list().await.unwrap());

        for snapshot in &snapshots {
            assert_eq!(snapshot.len(), ids.len(), "快照不缺行不重行");
            for session in snapshot {
                let appended = session
                    .log
                    .iter()
                    .filter(|entry| matches!(entry.event, SessionEventKind::MessageAppended { .. }))
                    .count();
                assert_eq!(
                    appended,
                    session.messages.len(),
                    "每行自洽: 账本追加记录与转写同刻 (无撕裂读)"
                );
                assert!(session.revision >= session.messages.len() as u64);
            }
        }
        // 写者收尾后的最终快照: 每会话 8 位写者 × 1 条全到齐。
        let final_snapshot = snapshots.last().unwrap();
        for session in final_snapshot {
            assert_eq!(session.messages.len(), 8, "零丢失更新");
        }
    }
}
