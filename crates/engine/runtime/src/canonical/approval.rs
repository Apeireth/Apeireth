//! Canonical pending-approval lifecycle types.
//!
//! Governance owns the decision *should this operation require approval?*.
//! Runtime owns *what is waiting, what exact operation is frozen, how it is
//! resumed, and whether it was already resolved*. This module contains the
//! runtime-owned domain objects, not a second governance policy.
//!
//! There is intentionally no `ApprovalManager`, `ApprovalRuntime`, or
//! `ApprovalExecutor` here. The types are inert data; `Runtime` is the only
//! orchestration root that acts on them.

use apeireth_core::kernel::{ApprovalId, CapabilityId, RequestId, SessionId, Timestamp, TraceId};
use apeireth_plugin::FrozenInvocation;
use apeireth_protocol::canonical::ToolCall;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A human decision about one pending approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Execute the frozen operation exactly once.
    Approve,
    /// Do not execute the frozen operation.
    Reject {
        /// Optional human reason. Private by default; the model-facing result
        /// uses a fixed rejection message unless a future contract says
        /// otherwise.
        reason: Option<String>,
    },
    /// Cancel the pending operation without executing it.
    ///
    /// Semantically equivalent to rejection for transcript pairing: the frozen
    /// tool call receives a synthetic result and the turn may continue.
    Cancel {
        /// Optional human reason.
        reason: Option<String>,
    },
}

impl ApprovalDecision {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject { .. } => "rejected",
            Self::Cancel { .. } => "cancelled",
        }
    }
}

/// The lifecycle of one pending approval.
///
/// `Pending -> Claimed -> Consumed` is the approval path. `Claimed` is not
/// final: it means a human approved the operation and the runtime has durably
/// claimed it, but the external effect may not have happened (or may have
/// happened and the final `Consumed` save was interrupted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    /// Waiting for a human decision.
    Pending,
    /// A human approved the operation and the runtime atomically claimed it.
    /// No other resolver may execute the operation again. Execution may or may
    /// not have occurred; only `Consumed` records that the result was appended.
    Claimed,
    /// A human rejected the operation. It will never execute.
    Rejected,
    /// The approval expired before a human resolved it. It will never execute.
    Expired,
    /// The approved operation was executed and its result was appended to the
    /// transcript.
    Consumed,
    /// A resolver re-opened a `Claimed` approval whose result was never
    /// recorded. The external effect is unknown and must not be retried
    /// automatically.
    Interrupted,
}

impl ApprovalStatus {
    /// True while the approval is waiting for a human decision.
    pub const fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }

    /// True once a human decision or an expiry transition has been reached.
    /// This includes `Claimed`, whose external effect is not yet final.
    pub const fn is_resolved(self) -> bool {
        !self.is_pending()
    }

    /// True when this approval can no longer move to `Consumed`.
    pub const fn is_final(self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Expired | Self::Consumed | Self::Interrupted
        )
    }

    /// Backwards-compatible alias for [`Self::is_final`].
    pub const fn is_terminal(self) -> bool {
        self.is_final()
    }
}

/// The exact point inside a turn where a pending approval must resume.
///
/// The provider transcript already contains the assistant tool-call message.
/// [`Self::next_tool_index`] points at the first tool call that has not been
/// executed yet. When the list is empty the turn is at the start of a round.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrozenTurnContinuation {
    pub request_id: RequestId,
    pub trace_id: TraceId,
    /// The model that was serving this turn when it paused.
    pub model: String,
    /// The round that was being executed.
    pub round: u32,
    /// The original assistant tool-call batch for the round. The model is
    /// never re-queried to regenerate these calls.
    pub tool_calls: Vec<ToolCall>,
    /// Index of the next tool call to process.
    pub next_tool_index: usize,
    /// When set, the tool call at this index was already approved and must be
    /// dispatched without a second governance evaluation.
    pub approved_tool_index: Option<usize>,
    /// When `approved_tool_index` is set, this is the approval that authorized
    /// it. The approval is moved to [`ApprovalStatus::Consumed`] after the tool
    /// has been invoked and its result appended.
    pub approved_approval_id: Option<ApprovalId>,
    /// Number of isolated module calls already spent by this turn.
    ///
    /// This is persisted with an approval pause so resolving the approval
    /// cannot reset the turn's side-call budget.
    #[serde(default)]
    pub module_invocations: usize,
}

impl FrozenTurnContinuation {
    /// A fresh continuation at the start of a round.
    pub fn start_of_round(
        request_id: RequestId,
        trace_id: TraceId,
        model: impl Into<String>,
        round: u32,
    ) -> Self {
        Self {
            request_id,
            trace_id,
            model: model.into(),
            round,
            tool_calls: Vec::new(),
            next_tool_index: 0,
            approved_tool_index: None,
            approved_approval_id: None,
            module_invocations: 0,
        }
    }
}

/// One durable pending approval.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingApproval {
    pub approval_id: ApprovalId,
    pub session_id: SessionId,
    pub request_id: RequestId,
    pub trace_id: TraceId,
    /// The runtime round that produced the pending operation.
    pub round: u32,
    /// The capability whose dispatch required approval.
    pub capability_id: CapabilityId,
    /// The model-facing tool name.
    pub tool_name: String,
    /// The frozen, immutable tool call. Resolvers may not substitute a new
    /// tool name, arguments, cwd, or any other execution parameter.
    pub tool_call: ToolCall,
    /// Capability-frozen effective invocation, when the capability supplied
    /// one. This binds derived security-relevant values (for example shell
    /// cwd, shell executable, timeout, and environment profile) beyond the raw
    /// provider tool arguments. This is the executable payload and may contain
    /// environment values required to execute; it is never shown directly in
    /// approval views.
    pub effective_invocation: Option<FrozenInvocation>,
    /// Governance hook that produced `RequireApproval`.
    pub governance_hook: String,
    /// Governance reason that is shown to the human.
    pub governance_reason: String,
    /// Canonical fingerprint over every operation-relevant field.
    pub operation_fingerprint: String,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    pub status: ApprovalStatus,
    /// Exact point the turn will resume from when this approval is resolved.
    pub continuation: FrozenTurnContinuation,
    /// Optional human reason recorded at resolution time.
    pub human_reason: Option<String>,
}

impl PendingApproval {
    pub fn is_expired(&self, now: Timestamp) -> bool {
        now > self.expires_at
    }
}

/// A provider- and adapter-facing view of one pending approval.
///
/// This is deliberately smaller than [`PendingApproval`]: it exposes the exact
/// operation that must be approved and nothing else. It never exposes secret
/// values beyond the operation itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingApprovalView {
    pub approval_id: ApprovalId,
    pub session_id: SessionId,
    pub request_id: RequestId,
    pub trace_id: TraceId,
    pub round: u32,
    pub capability_id: CapabilityId,
    pub tool_name: String,
    pub tool_call: ToolCall,
    /// Redacted, human-facing capability-frozen effective invocation, when one
    /// was supplied. This is [`PendingApproval::effective_invocation`]'s
    /// display payload, never the raw executable payload.
    pub effective_invocation: Option<serde_json::Value>,
    pub governance_hook: String,
    pub governance_reason: String,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
    pub operation_fingerprint: String,
}

impl From<&PendingApproval> for PendingApprovalView {
    fn from(value: &PendingApproval) -> Self {
        Self {
            approval_id: value.approval_id,
            session_id: value.session_id,
            request_id: value.request_id,
            trace_id: value.trace_id,
            round: value.round,
            capability_id: value.capability_id.clone(),
            tool_name: value.tool_name.clone(),
            tool_call: value.tool_call.clone(),
            effective_invocation: value
                .effective_invocation
                .as_ref()
                .map(|frozen| frozen.display.clone()),
            governance_hook: value.governance_hook.clone(),
            governance_reason: value.governance_reason.clone(),
            created_at: value.created_at,
            expires_at: value.expires_at,
            operation_fingerprint: value.operation_fingerprint.clone(),
        }
    }
}

/// Canonical deterministic JSON representation used for fingerprints.
pub fn canonical_json_bytes(value: &serde_json::Value) -> Vec<u8> {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(&String, &serde_json::Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let mut out = Vec::with_capacity(64);
            out.push(b'{');
            for (i, (key, value)) in entries.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                out.extend_from_slice(&canonical_json_bytes(&serde_json::Value::String(
                    (*key).clone(),
                )));
                out.push(b':');
                out.extend_from_slice(&canonical_json_bytes(value));
            }
            out.push(b'}');
            out
        }
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(64);
            out.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(b',');
                }
                out.extend_from_slice(&canonical_json_bytes(item));
            }
            out.push(b']');
            out
        }
        serde_json::Value::String(s) => {
            serde_json::to_vec(s).expect("string serialization is infallible")
        }
        serde_json::Value::Null => b"null".to_vec(),
        serde_json::Value::Bool(b) => b.to_string().into_bytes(),
        serde_json::Value::Number(n) => n.to_string().into_bytes(),
    }
}

/// Compute the canonical operation fingerprint for one tool dispatch.
///
/// The field set is intentionally generic so every capability dispatch —
/// including a future shell invocation — binds the same core identity.
pub fn operation_fingerprint(
    action_kind: &str,
    capability_id: &CapabilityId,
    tool_name: &str,
    tool_call_id: &str,
    arguments: &serde_json::Value,
    session_id: SessionId,
    request_id: RequestId,
    round: u32,
) -> String {
    operation_fingerprint_with_invocation(
        action_kind,
        capability_id,
        tool_name,
        tool_call_id,
        arguments,
        None,
        session_id,
        request_id,
        round,
    )
}

/// Compute the canonical operation fingerprint including a capability-frozen
/// effective invocation, when one exists.
pub fn operation_fingerprint_with_invocation(
    action_kind: &str,
    capability_id: &CapabilityId,
    tool_name: &str,
    tool_call_id: &str,
    arguments: &serde_json::Value,
    effective_invocation: Option<&serde_json::Value>,
    session_id: SessionId,
    request_id: RequestId,
    round: u32,
) -> String {
    let mut hasher = Sha256::new();
    update_len_prefixed(&mut hasher, action_kind.as_bytes());
    update_len_prefixed(&mut hasher, capability_id.as_str().as_bytes());
    update_len_prefixed(&mut hasher, tool_name.as_bytes());
    update_len_prefixed(&mut hasher, tool_call_id.as_bytes());
    update_len_prefixed(&mut hasher, &canonical_json_bytes(arguments));
    if let Some(effective) = effective_invocation {
        update_len_prefixed(&mut hasher, &canonical_json_bytes(effective));
    } else {
        update_len_prefixed(&mut hasher, b"null");
    }
    update_len_prefixed(&mut hasher, session_id.to_string().as_bytes());
    update_len_prefixed(&mut hasher, request_id.to_string().as_bytes());
    hasher.update(round.to_be_bytes());
    format!("{:x}", hasher.finalize())
}

fn update_len_prefixed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    #[test]
    fn canonical_json_object_key_order_is_irrelevant() {
        let a = serde_json::json!({ "b": 1, "a": [true, null, "x"] });
        let b = serde_json::json!({ "a": [true, null, "x"], "b": 1 });
        assert_eq!(canonical_json_bytes(&a), canonical_json_bytes(&b));
    }

    #[test]
    fn canonical_json_array_order_is_preserved() {
        let a = serde_json::json!([1, 2]);
        let b = serde_json::json!([2, 1]);
        assert_ne!(canonical_json_bytes(&a), canonical_json_bytes(&b));
    }

    #[test]
    fn fingerprint_is_deterministic_for_equivalent_objects() {
        let session = SessionId::new();
        let request = RequestId::new();
        let cap = CapabilityId::new("tool.example").unwrap();
        let args_a = serde_json::json!({ "command": "echo hi", "cwd": "/tmp" });
        let args_b = serde_json::json!({ "cwd": "/tmp", "command": "echo hi" });

        let a = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &args_a,
            session,
            request,
            2,
        );
        let b = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &args_b,
            session,
            request,
            2,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_changes_with_any_operation_field() {
        let session = SessionId::new();
        let request = RequestId::new();
        let cap = CapabilityId::new("tool.example").unwrap();
        let args = serde_json::json!({ "command": "echo hi" });

        let base = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &args,
            session,
            request,
            1,
        );

        let changed_scalar = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_1",
            &serde_json::json!({ "command": "echo bye" }),
            session,
            request,
            1,
        );
        assert_ne!(base, changed_scalar);

        let changed_capability = operation_fingerprint(
            "capability_dispatch",
            &CapabilityId::new("tool.other").unwrap(),
            "example",
            "call_1",
            &args,
            session,
            request,
            1,
        );
        assert_ne!(base, changed_capability);

        let changed_call = operation_fingerprint(
            "capability_dispatch",
            &cap,
            "example",
            "call_2",
            &args,
            session,
            request,
            1,
        );
        assert_ne!(base, changed_call);
    }

    #[test]
    fn approval_status_terminals_are_clear() {
        assert!(ApprovalStatus::Pending.is_pending());
        assert!(!ApprovalStatus::Pending.is_resolved());
        assert!(!ApprovalStatus::Pending.is_final());
        assert!(!ApprovalStatus::Pending.is_terminal());

        assert!(!ApprovalStatus::Claimed.is_pending());
        assert!(ApprovalStatus::Claimed.is_resolved());
        assert!(
            !ApprovalStatus::Claimed.is_final(),
            "Claimed is not final: it must still be able to move to Consumed"
        );
        assert!(!ApprovalStatus::Claimed.is_terminal());

        assert!(ApprovalStatus::Rejected.is_final());
        assert!(ApprovalStatus::Expired.is_final());
        assert!(ApprovalStatus::Consumed.is_final());
        assert!(ApprovalStatus::Interrupted.is_final());
    }
}
