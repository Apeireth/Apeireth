//! Durable session event vocabulary; re-exported at the existing session path.
use super::event_log::SurfaceOp;
use super::session::Session;
use apeireth_core::kernel::{ApprovalId, CapabilityId, SessionId};
use apeireth_orchestration::plan_mode::PlanModeEvent;
use apeireth_protocol::canonical::NormalizedMessage;
use serde::{Deserialize, Serialize};

/// Persisted execution outcomes. No variant carries model chain-of-thought.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum SessionEventKind {
    /// A fixed-version hook attempt, ordered at commit, with no payload in errors.
    RegisteredHook {
        module: apeireth_core::kernel::PluginId,
        hook: apeireth_protocol::canonical::ModuleHookV1,
        status: String,
    },
    /// Durable correlated fact, including provenance and independent execution id.
    ToolEventRecorded {
        fact: apeireth_protocol::canonical::ToolEventV1,
    },
    /// A module fact's paired messages were appended exactly once.
    ToolEventProjected {
        call_id: apeireth_core::kernel::TaskId,
    },
    /// Automatic observation had no model-visible data; its V1 fact remains auditable.
    ModuleToolEventObserved {
        call_id: apeireth_core::kernel::TaskId,
    },
    /// Host explicitly closed the session after draining its turn.
    SessionEnded,
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
