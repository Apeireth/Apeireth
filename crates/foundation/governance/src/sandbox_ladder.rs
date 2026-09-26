//! Monotonic sandbox escalation ladder with mandatory justification and
//! one-call ("one-shot") authorization.
//!
//! # Why
//!
//! The sandbox boundary used to be a static switch: a tool call that needed to
//! exceed it once (for example a single write outside the workspace directory)
//! forced the operator to widen the boundary permanently in configuration.
//! This module makes the boundary *temporarily* exceedable per call, without
//! touching persistent configuration and without creating permanent authority.
//!
//! # The rules
//!
//! 1. **Total order, monotonic widening.** [`SandboxMode`] is totally ordered
//!    `Strict < Standard < Relaxed < Permissive`. [`WIDER_MODES`] lists the
//!    admissible escalation steps — one rung at a time, each step moving to the
//!    narrowest wider rung. Narrowing is allowed at any time without
//!    authorization; skipping rungs is refused at evaluation time.
//! 2. **Mandatory justification.** An [`EscalationRequest`] must carry a
//!    non-empty `justification`. Requests without one are refused.
//! 3. **One-call authorization.** An approved escalation covers exactly one
//!    call, identified by its [`CallFingerprint`] (tool + arguments hash). It
//!    never enters persistent configuration. A later call with the same
//!    fingerprint goes through the approval channel again.
//! 4. **In-place guidance.** Every refusal carries a structured
//!    [`EscalationHint`] — which mode is missing and which reason field the
//!    caller must fill in — so the next step is visible at the decision point
//!    instead of being buried in a settings surface.
//! 5. **Compatible with the proven approval invariants.** An escalation is a
//!    normal [`Decision::RequireApproval`] on the existing approval channel;
//!    the runtime's single `approval_resolved` notification per approved
//!    decision turns it into one authorization event, recorded in the audit
//!    chain with the existing event vocabulary.
//!
//! The module is default-off: nothing is installed into a
//! [`crate::GovernancePipeline`] here, and the canonical decision semantics of
//! the inner policy are never rewritten — a one-call grant can promote an
//! approval but can never widen a real denial.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, Clock, SessionId, Timestamp};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::audit::AuditHashChain;
use crate::{Action, AuditRecord, Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict};

/// The argument key carrying the structured escalation request on a call.
pub const ESCALATION_ARGUMENT_KEY: &str = "sandbox_escalation";

/// The mandatory reason field inside an escalation request.
pub const JUSTIFICATION_FIELD: &str = "justification";

/// What an approved escalation covers: exactly one call.
pub const GRANT_SCOPE_ONE_CALL: &str = "one_call";

/// Audit event kind for an authorization event, reused from the existing
/// audit vocabulary (`approval.resolved`).
pub const AUDIT_EVENT_APPROVAL_RESOLVED: &str = "approval.resolved";

/// The sandbox restriction level, totally ordered from narrowest to widest.
///
/// The rungs are cut along the restriction kinds that already exist at the
/// tool boundary: the workspace-directory limit, the command-family rules, and
/// the network cut. Each predicate below names one of those existing
/// restrictions, so a rung is described by what it relaxes rather than by any
/// new mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    /// Narrowest: workspace-directory-only files, command-family allowlist in
    /// force, no network egress.
    Strict,
    /// The default sandboxed execution semantics: workspace-directory-only
    /// files, no network egress, content guardrails still apply.
    Standard,
    /// The filesystem boundary is lifted (out-of-workspace access, including
    /// one-time writes outside the workspace directory); still no egress.
    Relaxed,
    /// Widest: the user account authority, network egress on, every command
    /// family — the unsandboxed execution semantics.
    Permissive,
}

impl SandboxMode {
    /// Position in the ladder, counting from 0 at [`SandboxMode::Strict`].
    pub const fn rank(self) -> u8 {
        match self {
            Self::Strict => 0,
            Self::Standard => 1,
            Self::Relaxed => 2,
            Self::Permissive => 3,
        }
    }

    /// Stable wire label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Standard => "standard",
            Self::Relaxed => "relaxed",
            Self::Permissive => "permissive",
        }
    }

    /// Parse a wire label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "strict" => Some(Self::Strict),
            "standard" => Some(Self::Standard),
            "relaxed" => Some(Self::Relaxed),
            "permissive" => Some(Self::Permissive),
            _ => None,
        }
    }

    /// The narrowest wider rung, looked up in [`WIDER_MODES`] at call time.
    pub const fn next_wider_mode(self) -> Option<SandboxMode> {
        let mut index = 0;
        while index < WIDER_MODES.len() {
            let (from, to) = WIDER_MODES[index];
            if from.rank() == self.rank() {
                return Some(to);
            }
            index += 1;
        }
        None
    }

    /// Whether filesystem access is limited to the workspace directory
    /// (the existing directory restriction).
    pub const fn workspace_only_files(self) -> bool {
        matches!(self, Self::Strict | Self::Standard)
    }

    /// Whether a write outside the workspace directory is permitted at this
    /// rung (the motivating one-time escape).
    pub const fn permits_out_of_workspace_write(self) -> bool {
        !self.workspace_only_files()
    }

    /// Whether network egress is permitted (the existing network cut).
    pub const fn allows_network_egress(self) -> bool {
        matches!(self, Self::Permissive)
    }

    /// Whether a command-family allowlist is in force (the existing static
    /// command-family boundary, tightened to its narrowest form).
    pub const fn enforces_command_family_allowlist(self) -> bool {
        matches!(self, Self::Strict)
    }

    /// Whether high-risk command families (destructive / privilege utilities)
    /// are permitted at this rung.
    pub const fn allows_high_risk_command_family(self) -> bool {
        matches!(self, Self::Permissive)
    }
}

impl fmt::Display for SandboxMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// The admissible escalation steps: every rung except the widest maps to the
/// narrowest wider rung. Escalation walks this table one step at a time;
/// widening several rungs in one request is refused.
pub const WIDER_MODES: &[(SandboxMode, SandboxMode)] = &[
    (SandboxMode::Strict, SandboxMode::Standard),
    (SandboxMode::Standard, SandboxMode::Relaxed),
    (SandboxMode::Relaxed, SandboxMode::Permissive),
];

/// What a validated mode change means for authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModeTransition {
    /// The requested mode is narrower: always allowed, no authorization.
    Narrowing,
    /// The requested mode equals the current one: nothing changes.
    Unchanged,
    /// Exactly one rung wider, via [`WIDER_MODES`]: needs justification and a
    /// one-call authorization.
    WideningOneStep,
}

/// Validate a requested mode against the current one.
///
/// Narrowing and unchanged are always allowed; widening must land exactly on
/// the next rung from [`WIDER_MODES`]; anything wider is a refused `SkippedRung`
/// carrying the rung to request first.
pub fn check_mode_transition(
    current: SandboxMode,
    requested: SandboxMode,
) -> Result<ModeTransition, EscalationRefusal> {
    if requested.rank() < current.rank() {
        Ok(ModeTransition::Narrowing)
    } else if requested.rank() == current.rank() {
        Ok(ModeTransition::Unchanged)
    } else if current.next_wider_mode() == Some(requested) {
        Ok(ModeTransition::WideningOneStep)
    } else {
        let required_next = current.next_wider_mode().unwrap_or(SandboxMode::Permissive);
        Err(EscalationRefusal::SkippedRung {
            current,
            requested,
            required_next,
        })
    }
}

/// A structured request to widen the sandbox boundary for one call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EscalationRequest {
    /// The one-rung-wider mode being requested.
    pub mode: SandboxMode,
    /// Why this call must exceed the current boundary. Mandatory: a request
    /// without a non-empty reason is refused.
    pub justification: String,
}

impl EscalationRequest {
    /// A request for `mode`, justified by `justification`.
    pub fn new(mode: SandboxMode, justification: impl Into<String>) -> Self {
        Self {
            mode,
            justification: justification.into(),
        }
    }

    /// Refuse a request whose reason is missing, empty, or whitespace only.
    pub fn validate_justification(&self) -> Result<(), EscalationRefusal> {
        if self.justification.trim().is_empty() {
            return Err(EscalationRefusal::MissingJustification {
                requested_mode: self.mode,
            });
        }
        Ok(())
    }

    /// Read the structured envelope from call arguments.
    ///
    /// Returns `Ok(None)` when the call carries no envelope — an ordinary call
    /// is not an escalation. A present but unusable envelope is a refusal with
    /// in-place guidance, never a silent drop.
    pub fn from_arguments(
        arguments: &serde_json::Value,
    ) -> Result<Option<Self>, EscalationRefusal> {
        let Some(envelope) = arguments.get(ESCALATION_ARGUMENT_KEY) else {
            return Ok(None);
        };
        let Some(object) = envelope.as_object() else {
            return Err(EscalationRefusal::MalformedRequest {
                detail: format!("{ESCALATION_ARGUMENT_KEY} must be a JSON object"),
            });
        };
        let Some(mode_value) = object.get("mode") else {
            return Err(EscalationRefusal::MalformedRequest {
                detail: "missing the \"mode\" field".to_string(),
            });
        };
        let Some(mode_label) = mode_value.as_str() else {
            return Err(EscalationRefusal::MalformedRequest {
                detail: "the \"mode\" field must be a string".to_string(),
            });
        };
        let Some(mode) = SandboxMode::from_label(mode_label) else {
            return Err(EscalationRefusal::MalformedRequest {
                detail: format!("unknown mode {mode_label:?}"),
            });
        };
        let Some(justification_value) = object.get(JUSTIFICATION_FIELD) else {
            return Err(EscalationRefusal::MissingJustification {
                requested_mode: mode,
            });
        };
        let Some(justification) = justification_value.as_str() else {
            return Err(EscalationRefusal::MalformedRequest {
                detail: format!("the {JUSTIFICATION_FIELD:?} field must be a string"),
            });
        };
        if justification.trim().is_empty() {
            return Err(EscalationRefusal::MissingJustification {
                requested_mode: mode,
            });
        }
        Ok(Some(Self::new(mode, justification)))
    }
}

/// Why an escalation request was refused. Every variant carries enough
/// structure for [`EscalationHint`] to explain the fix at the call site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "refusal", rename_all = "snake_case")]
pub enum EscalationRefusal {
    /// The envelope could not be parsed at all.
    MalformedRequest { detail: String },
    /// The reason field is missing, empty, or whitespace only.
    MissingJustification { requested_mode: SandboxMode },
    /// The request widens more than one rung in a single step.
    SkippedRung {
        current: SandboxMode,
        requested: SandboxMode,
        required_next: SandboxMode,
    },
}

impl EscalationRefusal {
    /// The structured in-place guidance for this refusal.
    pub fn hint(&self) -> EscalationHint {
        match self {
            Self::MalformedRequest { .. } => EscalationHint::for_malformed_request(),
            Self::MissingJustification { requested_mode } => {
                EscalationHint::for_missing_justification(*requested_mode)
            }
            Self::SkippedRung {
                required_next,
                current,
                requested,
            } => EscalationHint::for_skipped_rung(*current, *requested, *required_next),
        }
    }
}

impl fmt::Display for EscalationRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hint = self.hint();
        match self {
            Self::MalformedRequest { detail } => write!(
                f,
                "sandbox escalation refused: malformed {ESCALATION_ARGUMENT_KEY} envelope ({detail}); {hint}"
            ),
            Self::MissingJustification { requested_mode } => write!(
                f,
                "sandbox escalation refused: the {JUSTIFICATION_FIELD:?} field is missing or empty, \
                 a one-call upgrade to {requested_mode} needs a non-empty reason; {hint}"
            ),
            Self::SkippedRung {
                current,
                requested,
                required_next,
            } => write!(
                f,
                "sandbox escalation refused: {requested} skips rungs above {current}; request \
                 {required_next} first (the ladder widens one rung at a time); {hint}"
            ),
        }
    }
}

/// Structured in-place guidance attached to a refusal: which sandbox mode is
/// missing, which reason field must be filled in, and what an approved grant
/// would cover.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscalationHint {
    /// The ladder rung that would permit the refused operation, when the
    /// refusal is escapable at all.
    pub missing_mode: Option<SandboxMode>,
    /// The argument key the caller uses to attach the structured request.
    pub request_key: String,
    /// The mandatory reason field inside that request.
    pub justification_field: String,
    /// Whether the reason field must be non-empty.
    pub justification_required: bool,
    /// What an approved grant covers.
    pub grant_scope: String,
    /// How to fix the refusal at the call site.
    pub guidance: String,
}

impl EscalationHint {
    /// Guidance for a request whose reason is missing or empty.
    pub fn for_missing_justification(requested_mode: SandboxMode) -> Self {
        Self {
            missing_mode: Some(requested_mode),
            request_key: ESCALATION_ARGUMENT_KEY.to_string(),
            justification_field: JUSTIFICATION_FIELD.to_string(),
            justification_required: true,
            grant_scope: GRANT_SCOPE_ONE_CALL.to_string(),
            guidance: format!(
                "set {ESCALATION_ARGUMENT_KEY} = {{\"mode\":\"{requested_mode}\",\"{JUSTIFICATION_FIELD}\":\"<non-empty reason>\"}} and retry; the grant covers one call"
            ),
        }
    }

    /// Guidance for a request that skips one or more rungs.
    pub fn for_skipped_rung(
        current: SandboxMode,
        requested: SandboxMode,
        required_next: SandboxMode,
    ) -> Self {
        Self {
            missing_mode: Some(required_next),
            request_key: ESCALATION_ARGUMENT_KEY.to_string(),
            justification_field: JUSTIFICATION_FIELD.to_string(),
            justification_required: true,
            grant_scope: GRANT_SCOPE_ONE_CALL.to_string(),
            guidance: format!(
                "widen one rung at a time: from {current} request {required_next} first \
                 ({ESCALATION_ARGUMENT_KEY} = {{\"mode\":\"{required_next}\",\"{JUSTIFICATION_FIELD}\":\"<non-empty reason>\"}}), \
                 then repeat for {requested}"
            ),
        }
    }

    /// Guidance for an envelope that could not be parsed.
    pub fn for_malformed_request() -> Self {
        Self {
            missing_mode: None,
            request_key: ESCALATION_ARGUMENT_KEY.to_string(),
            justification_field: JUSTIFICATION_FIELD.to_string(),
            justification_required: true,
            grant_scope: GRANT_SCOPE_ONE_CALL.to_string(),
            guidance: format!(
                "{ESCALATION_ARGUMENT_KEY} must be an object like \
                 {{\"mode\":\"<mode label>\",\"{JUSTIFICATION_FIELD}\":\"<non-empty reason>\"}}"
            ),
        }
    }

    /// The hint as one compact JSON object, ready to embed in a refusal.
    pub fn render(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.guidance.clone())
    }
}

impl fmt::Display for EscalationHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

/// Identity of one tool call: the tool plus a hash of its arguments.
///
/// The hash is computed over canonical JSON (object keys sorted), so equal
/// arguments in different key order produce the same fingerprint. Only the
/// digest is stored, so neither the argument text nor secrets inside it leak
/// into authorization or audit records.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallFingerprint {
    /// The capability being dispatched.
    pub tool: String,
    /// First 16 hex characters of the SHA-256 over the canonical arguments.
    pub arguments_hash: String,
}

impl CallFingerprint {
    /// Fingerprint one call.
    pub fn new(tool: impl Into<String>, arguments: &serde_json::Value) -> Self {
        let mut canonical = String::new();
        canonical_json(arguments, &mut canonical);
        let digest = Sha256::digest(canonical.as_bytes());
        Self {
            tool: tool.into(),
            arguments_hash: format!("{digest:x}")[..16].to_string(),
        }
    }
}

impl fmt::Display for CallFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.tool, self.arguments_hash)
    }
}

fn canonical_json(value: &serde_json::Value, out: &mut String) {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key.as_str()).unwrap_or_default());
                out.push(':');
                canonical_json(&map[*key], out);
            }
            out.push('}');
        }
        serde_json::Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                canonical_json(item, out);
            }
            out.push(']');
        }
        other => out.push_str(&serde_json::to_string(other).unwrap_or_default()),
    }
}

/// Why the authorization ledger refused an operation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AuthorizationLedgerError {
    #[error("no escalation review is pending for this call fingerprint")]
    NoPendingReview,
}

/// Outcome of opening a review for one escalation request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    /// A new review is now waiting for a human decision.
    Opened,
    /// The same request is already waiting; no second review was opened.
    AlreadyPending,
}

/// One-call escalation authorizations.
///
/// A grant is bound to one [`CallFingerprint`], is spent by exactly one call,
/// and is never persisted: the ledger is in-process memory, so a restart drops
/// every grant and every pending review. Authorization events are appended to
/// a [`AuditHashChain`] with the existing `approval.resolved` vocabulary, one
/// record per human approval.
#[derive(Debug, Default)]
pub struct OneShotAuthorizationLedger {
    state: Mutex<LedgerState>,
}

#[derive(Debug, Default)]
struct LedgerState {
    /// Reviews waiting for a human decision, keyed by fingerprint.
    pending: HashMap<CallFingerprint, SandboxMode>,
    /// Approved but not yet spent grants, keyed by (fingerprint, target).
    grants: HashSet<(CallFingerprint, SandboxMode)>,
    /// Authorization events, in order.
    audit: AuditHashChain,
}

impl OneShotAuthorizationLedger {
    /// An empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the review for one escalation request.
    ///
    /// A request whose fingerprint already waits for a human decision does not
    /// open a second review — the pending request is handled by the existing
    /// approval state machine, not re-approved on the side.
    pub fn begin_review(&self, fingerprint: CallFingerprint, target: SandboxMode) -> ReviewStatus {
        let mut state = lock_or_recover(&self.state);
        if state.pending.contains_key(&fingerprint) {
            return ReviewStatus::AlreadyPending;
        }
        state.pending.insert(fingerprint, target);
        ReviewStatus::Opened
    }

    /// Record that a human approved the pending review for `fingerprint`.
    ///
    /// Produces exactly one authorization event in the audit chain. Approving
    /// a fingerprint that has no pending review is refused, so a duplicated
    /// approval notification can never duplicate an authorization.
    pub fn grant(
        &self,
        fingerprint: &CallFingerprint,
        timestamp: Timestamp,
    ) -> Result<SandboxMode, AuthorizationLedgerError> {
        let mut state = lock_or_recover(&self.state);
        let target = state
            .pending
            .remove(fingerprint)
            .ok_or(AuthorizationLedgerError::NoPendingReview)?;
        state.grants.insert((fingerprint.clone(), target));
        state.audit.append(
            AUDIT_EVENT_APPROVAL_RESOLVED,
            format!("sandbox_escalation:{fingerprint}"),
            timestamp,
        );
        Ok(target)
    }

    /// Spend the one-call authorization.
    ///
    /// Returns `true` at most once per grant: the call that consumes it is the
    /// single side effect the approval covers.
    pub fn consume(&self, fingerprint: &CallFingerprint, target: SandboxMode) -> bool {
        lock_or_recover(&self.state)
            .grants
            .remove(&(fingerprint.clone(), target))
    }

    /// Whether a review is waiting for a human decision.
    pub fn is_pending(&self, fingerprint: &CallFingerprint) -> bool {
        lock_or_recover(&self.state)
            .pending
            .contains_key(fingerprint)
    }

    /// Whether an unspent grant exists for this fingerprint and target.
    pub fn has_grant(&self, fingerprint: &CallFingerprint, target: SandboxMode) -> bool {
        lock_or_recover(&self.state)
            .grants
            .contains(&(fingerprint.clone(), target))
    }

    /// The authorization events recorded so far, in order.
    pub fn audit_records(&self) -> Vec<AuditRecord> {
        lock_or_recover(&self.state).audit.records().to_vec()
    }
}

fn lock_or_recover<T>(lock: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Governance hook decorator that adds the escalation ladder around an inner
/// policy.
///
/// The inner policy is consulted first and its verdict is preserved; this
/// decorator only adds the authorization surface:
///
/// * a call with no escalation envelope gets the inner verdict verbatim;
/// * a narrowing or unchanged mode request also gets the inner verdict;
/// * a one-rung widening with a non-empty reason reaches the existing approval
///   channel as [`Decision::RequireApproval`] — unless a one-call grant for
///   this exact fingerprint is still unspent, in which case the grant is spent
///   and the call proceeds;
/// * anything else is refused with a structured [`EscalationHint`].
///
/// A one-call grant never widens a real denial from the inner policy.
pub struct SandboxEscalationGate {
    inner: Arc<dyn GovernanceHook>,
    current_mode: SandboxMode,
    clock: Arc<dyn Clock>,
    awaiting: Mutex<HashMap<(String, String), (CallFingerprint, SandboxMode)>>,
    ledger: OneShotAuthorizationLedger,
}

impl SandboxEscalationGate {
    /// Wrap `inner`, escalating from `current_mode` one rung at a time.
    pub fn new(
        inner: Arc<dyn GovernanceHook>,
        current_mode: SandboxMode,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            inner,
            current_mode,
            clock,
            awaiting: Mutex::new(HashMap::new()),
            ledger: OneShotAuthorizationLedger::new(),
        }
    }

    /// The mode the ladder widens from.
    pub const fn current_mode(&self) -> SandboxMode {
        self.current_mode
    }

    /// The one-call authorization ledger (introspection surface).
    pub const fn ledger(&self) -> &OneShotAuthorizationLedger {
        &self.ledger
    }

    fn approve_pending(&self, session: &SessionId, capability: &CapabilityId) {
        let key = (session.to_string(), capability.as_str().to_string());
        let Some((fingerprint, _target)) = lock_or_recover(&self.awaiting).remove(&key) else {
            return;
        };
        let timestamp = Timestamp::from_clock(self.clock.as_ref());
        // A duplicated notification finds no pending review and records
        // nothing, so one approval stays one authorization event.
        let _ = self.ledger.grant(&fingerprint, timestamp);
    }

    async fn judge(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let Action::CapabilityDispatch {
            capability,
            arguments,
        } = &request.action
        else {
            return self.inner.evaluate_verbose(request).await;
        };

        let escalation = match EscalationRequest::from_arguments(arguments) {
            // No envelope: an ordinary call. The inner verdict is returned
            // untouched — turning the ladder off changes nothing.
            Ok(None) => return self.inner.evaluate_verbose(request).await,
            Ok(Some(escalation)) => escalation,
            Err(refusal) => {
                return GovernanceVerdict::new(self.name(), Decision::deny(refusal.to_string()))
            }
        };

        let transition = match check_mode_transition(self.current_mode, escalation.mode) {
            Ok(transition) => transition,
            Err(refusal) => {
                return GovernanceVerdict::new(self.name(), Decision::deny(refusal.to_string()))
            }
        };

        match transition {
            // Narrowing is always allowed; nothing is escalated, so the inner
            // verdict stands.
            ModeTransition::Narrowing | ModeTransition::Unchanged => {
                self.inner.evaluate_verbose(request).await
            }
            ModeTransition::WideningOneStep => {
                if let Err(refusal) = escalation.validate_justification() {
                    return GovernanceVerdict::new(
                        self.name(),
                        Decision::deny(refusal.to_string()),
                    );
                }
                let fingerprint = CallFingerprint::new(capability.as_str(), arguments);
                let inner = self.inner.evaluate_verbose(request).await;
                if matches!(inner.decision, Decision::Deny { .. }) {
                    // Fail closed: a one-call grant is not a denial bypass, and
                    // a call that never runs must not spend its grant.
                    return inner;
                }
                if self.ledger.consume(&fingerprint, escalation.mode) {
                    return GovernanceVerdict::new(self.name(), Decision::Allow);
                }
                self.ledger
                    .begin_review(fingerprint.clone(), escalation.mode);
                let key = (request.session.to_string(), capability.as_str().to_string());
                lock_or_recover(&self.awaiting).insert(key, (fingerprint.clone(), escalation.mode));
                GovernanceVerdict::new(
                    self.name(),
                    Decision::require_approval(format!(
                        "one-call sandbox escalation to {} for {fingerprint} requires human \
                         approval ({GRANT_SCOPE_ONE_CALL}); {JUSTIFICATION_FIELD}: {}",
                        escalation.mode, escalation.justification
                    )),
                )
            }
        }
    }
}

#[async_trait::async_trait]
impl GovernanceHook for SandboxEscalationGate {
    fn name(&self) -> &str {
        "sandbox_escalation"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        self.judge(request).await.decision
    }

    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        self.judge(request).await
    }

    fn approval_resolved(&self, session: &SessionId, capability: &CapabilityId) {
        // The runtime calls this exactly once per approved decision; it is the
        // existing approval channel, reused unchanged for escalations.
        self.approve_pending(session, capability);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::{system_clock, TraceId, VirtualClock};
    use serde_json::json;

    fn dispatch_request<'a>(
        session: SessionId,
        capability: &'a CapabilityId,
        arguments: &'a serde_json::Value,
    ) -> GovernanceRequest<'a> {
        GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability,
                arguments,
            },
            session,
            TraceId::new(),
            1,
        )
    }

    struct FixedDecision {
        name: &'static str,
        decision: Decision,
    }

    #[async_trait::async_trait]
    impl GovernanceHook for FixedDecision {
        fn name(&self) -> &str {
            self.name
        }

        async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
            self.decision.clone()
        }
    }

    fn args_with(mode: &str, justification: &str) -> serde_json::Value {
        json!({
            "command": "write-file ../notes.txt",
            "sandbox_escalation": {"mode": mode, "justification": justification}
        })
    }

    #[test]
    fn wider_modes_table_is_a_single_monotone_chain() {
        assert_eq!(WIDER_MODES.len(), 3, "one entry per non-top rung");
        for (from, to) in WIDER_MODES {
            assert_eq!(
                to.rank(),
                from.rank() + 1,
                "each step widens exactly one rung"
            );
            assert_eq!(from.next_wider_mode(), Some(*to), "{from}");
        }
        assert_eq!(SandboxMode::Permissive.next_wider_mode(), None);
        // The rung order is the declared total order.
        assert!(SandboxMode::Strict < SandboxMode::Standard);
        assert!(SandboxMode::Standard < SandboxMode::Relaxed);
        assert!(SandboxMode::Relaxed < SandboxMode::Permissive);
    }

    #[test]
    fn narrowing_is_allowed_and_step_skipping_is_refused() {
        // 收窄随时允许.
        assert_eq!(
            check_mode_transition(SandboxMode::Standard, SandboxMode::Strict),
            Ok(ModeTransition::Narrowing)
        );
        assert_eq!(
            check_mode_transition(SandboxMode::Permissive, SandboxMode::Standard),
            Ok(ModeTransition::Narrowing)
        );
        // Same rung is not an escalation.
        assert_eq!(
            check_mode_transition(SandboxMode::Relaxed, SandboxMode::Relaxed),
            Ok(ModeTransition::Unchanged)
        );
        // One rung wider is the only admissible widening step.
        assert_eq!(
            check_mode_transition(SandboxMode::Standard, SandboxMode::Relaxed),
            Ok(ModeTransition::WideningOneStep)
        );
        // 越级: skipping rungs is refused and names the rung to request first.
        let refusal =
            check_mode_transition(SandboxMode::Strict, SandboxMode::Permissive).unwrap_err();
        assert_eq!(
            refusal,
            EscalationRefusal::SkippedRung {
                current: SandboxMode::Strict,
                requested: SandboxMode::Permissive,
                required_next: SandboxMode::Standard,
            }
        );
        let second = check_mode_transition(SandboxMode::Strict, SandboxMode::Relaxed).unwrap_err();
        assert_eq!(
            second,
            EscalationRefusal::SkippedRung {
                current: SandboxMode::Strict,
                requested: SandboxMode::Relaxed,
                required_next: SandboxMode::Standard,
            }
        );
    }

    #[test]
    fn escalation_without_a_non_empty_justification_is_refused() {
        for justification in ["", "   ", "\t\n"] {
            let request = EscalationRequest::new(SandboxMode::Relaxed, justification);
            assert_eq!(
                request.validate_justification(),
                Err(EscalationRefusal::MissingJustification {
                    requested_mode: SandboxMode::Relaxed
                }),
                "justification {justification:?} must be refused"
            );
        }
        // The structured envelope follows the same rule.
        let empty = json!({ "sandbox_escalation": {"mode": "relaxed", "justification": ""} });
        assert!(matches!(
            EscalationRequest::from_arguments(&empty),
            Err(EscalationRefusal::MissingJustification { .. })
        ));
        let missing_field = json!({ "sandbox_escalation": {"mode": "relaxed"} });
        assert!(matches!(
            EscalationRequest::from_arguments(&missing_field),
            Err(EscalationRefusal::MissingJustification { .. })
        ));
        // A non-empty reason parses.
        let good = json!({
            "sandbox_escalation": {"mode": "relaxed", "justification": "one export file"}
        });
        assert_eq!(
            EscalationRequest::from_arguments(&good),
            Ok(Some(EscalationRequest::new(
                SandboxMode::Relaxed,
                "one export file"
            )))
        );
    }

    #[test]
    fn refusal_hint_names_the_missing_mode_and_the_justification_field() {
        let refusal = EscalationRefusal::MissingJustification {
            requested_mode: SandboxMode::Relaxed,
        };
        let hint = refusal.hint();
        let rendered = hint.render();
        assert_eq!(hint.missing_mode, Some(SandboxMode::Relaxed));
        assert!(
            rendered.contains("\"missing_mode\":\"relaxed\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("\"justification_field\":\"justification\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("\"request_key\":\"sandbox_escalation\""),
            "{rendered}"
        );
        assert!(
            rendered.contains("\"grant_scope\":\"one_call\""),
            "{rendered}"
        );
        // 就地提示: the message says how to fill the reason field and retry.
        assert!(
            hint.guidance.contains("sandbox_escalation")
                && hint.guidance.contains("justification")
                && hint.guidance.contains("non-empty"),
            "{}",
            hint.guidance
        );
        // The refusal message itself carries the structured hint.
        let message = refusal.to_string();
        assert!(
            message.contains("\"missing_mode\":\"relaxed\""),
            "{message}"
        );
        assert!(message.contains("justification"), "{message}");
    }

    #[test]
    fn skipped_rung_hint_points_at_the_next_wider_mode() {
        let refusal = EscalationRefusal::SkippedRung {
            current: SandboxMode::Strict,
            requested: SandboxMode::Permissive,
            required_next: SandboxMode::Standard,
        };
        let hint = refusal.hint();
        assert_eq!(hint.missing_mode, Some(SandboxMode::Standard));
        assert!(
            hint.guidance.contains("standard") && hint.guidance.contains("one rung at a time"),
            "{}",
            hint.guidance
        );
        let message = refusal.to_string();
        assert!(message.contains("standard"), "{message}");
        assert!(message.contains("skips rungs"), "{message}");
    }

    #[tokio::test]
    async fn one_call_grant_is_valid_for_exactly_one_call() {
        let capability = CapabilityId::new("tool.shell").unwrap();
        let arguments = args_with("relaxed", "one export file outside the workspace");
        let gate = SandboxEscalationGate::new(
            Arc::new(FixedDecision {
                name: "inner_policy",
                decision: Decision::Allow,
            }),
            SandboxMode::Standard,
            system_clock(),
        );

        // First call: the escalation reaches the existing approval channel.
        let session = SessionId::new();
        let request = dispatch_request(session, &capability, &arguments);
        let first = gate.evaluate_verbose(&request).await;
        assert!(
            matches!(first.decision, Decision::RequireApproval { .. }),
            "before a human decides, the call waits: {:?}",
            first.decision
        );
        // While the request waits, the identical retry does not slip through.
        let retry = gate.evaluate_verbose(&request).await;
        assert!(matches!(retry.decision, Decision::RequireApproval { .. }));

        // Human approves once; the runtime delivers the single notification.
        gate.approval_resolved(&session, &capability);
        let granted = gate.evaluate_verbose(&request).await;
        assert!(
            granted.decision.is_allowed(),
            "the approved call runs once: {:?}",
            granted.decision
        );

        // 同指纹下次仍要批: the grant is spent, so the identical call asks again.
        let third = gate.evaluate_verbose(&request).await;
        assert!(
            matches!(third.decision, Decision::RequireApproval { .. }),
            "a one-call grant must not survive its one call: {:?}",
            third.decision
        );
    }

    #[test]
    fn repeated_request_while_pending_does_not_open_a_second_review() {
        let capability = CapabilityId::new("tool.shell").unwrap();
        let arguments = args_with("relaxed", "one export file");
        let fingerprint = CallFingerprint::new(capability.as_str(), &arguments);
        let ledger = OneShotAuthorizationLedger::new();

        assert_eq!(
            ledger.begin_review(fingerprint.clone(), SandboxMode::Relaxed),
            ReviewStatus::Opened
        );
        assert_eq!(
            ledger.begin_review(fingerprint.clone(), SandboxMode::Relaxed),
            ReviewStatus::AlreadyPending
        );
        assert_eq!(
            ledger.begin_review(fingerprint.clone(), SandboxMode::Relaxed),
            ReviewStatus::AlreadyPending
        );
        assert!(ledger.is_pending(&fingerprint));
    }

    #[tokio::test]
    async fn one_authorization_produces_exactly_one_audit_event() {
        let capability = CapabilityId::new("tool.filesystem").unwrap();
        let arguments = args_with("relaxed", "single write outside the workspace");
        let session = SessionId::new();
        let gate = SandboxEscalationGate::new(
            Arc::new(FixedDecision {
                name: "inner_policy",
                decision: Decision::Allow,
            }),
            SandboxMode::Standard,
            system_clock(),
        );

        // Three identical requests while pending: one review, one approval.
        for _ in 0..3 {
            gate.evaluate_verbose(&dispatch_request(session, &capability, &arguments))
                .await;
        }
        gate.approval_resolved(&session, &capability);
        // A duplicated notification must not record a second authorization.
        gate.approval_resolved(&session, &capability);

        let records = gate.ledger().audit_records();
        assert_eq!(records.len(), 1, "authorization events are single-shot");
        assert_eq!(records[0].event_kind, AUDIT_EVENT_APPROVAL_RESOLVED);
        assert!(
            records[0]
                .subject
                .contains(&CallFingerprint::new(capability.as_str(), &arguments).to_string()),
            "the record names the authorized call: {}",
            records[0].subject
        );
    }

    #[tokio::test]
    async fn one_call_grant_never_bypasses_a_denial() {
        let capability = CapabilityId::new("tool.shell").unwrap();
        let arguments = args_with("relaxed", "one export file");
        let fingerprint = CallFingerprint::new(capability.as_str(), &arguments);
        let gate = SandboxEscalationGate::new(
            Arc::new(FixedDecision {
                name: "inner_policy",
                decision: Decision::deny("capability tool.shell is not permitted"),
            }),
            SandboxMode::Standard,
            system_clock(),
        );

        // Seed an unspent grant the same way an approval would.
        gate.ledger()
            .begin_review(fingerprint.clone(), SandboxMode::Relaxed);
        gate.ledger()
            .grant(&fingerprint, Timestamp::from_clock(system_clock().as_ref()))
            .unwrap();

        let verdict = gate
            .evaluate_verbose(&dispatch_request(SessionId::new(), &capability, &arguments))
            .await;
        assert!(matches!(verdict.decision, Decision::Deny { .. }));
        assert_eq!(verdict.hook, "inner_policy", "the denial keeps its owner");
        assert!(
            gate.ledger().has_grant(&fingerprint, SandboxMode::Relaxed),
            "a call that never ran must not spend its grant"
        );
    }

    #[tokio::test]
    async fn without_an_escalation_request_the_inner_decision_is_unchanged() {
        let capability = CapabilityId::new("tool.shell").unwrap();
        let session = SessionId::new();

        for decision in [
            Decision::Allow,
            Decision::deny("blocked by policy"),
            Decision::require_approval("needs a human"),
        ] {
            let gate = SandboxEscalationGate::new(
                Arc::new(FixedDecision {
                    name: "inner_policy",
                    decision: decision.clone(),
                }),
                SandboxMode::Standard,
                system_clock(),
            );
            let arguments = json!({"command": "ls"});
            let verdict = gate
                .evaluate_verbose(&dispatch_request(session, &capability, &arguments))
                .await;
            assert_eq!(verdict.decision, decision, "no envelope, no change");
            assert_eq!(verdict.hook, "inner_policy", "attribution is preserved");

            // A narrowing request is not an escalation either.
            let narrowing = args_with("strict", "tightening for this call");
            let narrowed = gate
                .evaluate_verbose(&dispatch_request(session, &capability, &narrowing))
                .await;
            assert_eq!(narrowed.decision, decision);
            assert_eq!(narrowed.hook, "inner_policy");
        }
    }

    #[test]
    fn call_fingerprint_is_key_order_independent_and_never_plaintext() {
        let a = json!({"command": "echo hi", "cwd": "/tmp"});
        let b = json!({"cwd": "/tmp", "command": "echo hi"});
        assert_eq!(
            CallFingerprint::new("tool.shell", &a),
            CallFingerprint::new("tool.shell", &b)
        );
        assert_ne!(
            CallFingerprint::new("tool.shell", &a),
            CallFingerprint::new("tool.shell", &json!({"command": "echo bye"}))
        );
        assert_ne!(
            CallFingerprint::new("tool.shell", &a),
            CallFingerprint::new("tool.filesystem", &a)
        );
        let secret = CallFingerprint::new("tool.shell", &json!({"token": "sk-live-secret-value"}));
        assert!(
            !secret.to_string().contains("sk-live-secret-value"),
            "{secret}"
        );
    }

    #[test]
    fn escalation_request_envelope_is_optional_and_strict() {
        // No envelope: ordinary call.
        assert_eq!(
            EscalationRequest::from_arguments(&json!({"command": "ls"})),
            Ok(None)
        );
        // Unknown mode / wrong shapes are refusals with guidance, never silent.
        for bad in [
            json!({ ESCALATION_ARGUMENT_KEY: {"mode": "wide", JUSTIFICATION_FIELD: "why"} }),
            json!({ ESCALATION_ARGUMENT_KEY: {"mode": 3, JUSTIFICATION_FIELD: "why"} }),
            json!({ ESCALATION_ARGUMENT_KEY: [1, 2] }),
            json!({ ESCALATION_ARGUMENT_KEY: {"JUSTIFICATION_FIELD": "why"} }),
        ] {
            let refusal = EscalationRequest::from_arguments(&bad).unwrap_err();
            assert!(matches!(
                refusal,
                EscalationRefusal::MalformedRequest { .. }
            ));
            assert!(refusal.to_string().contains("sandbox_escalation"));
        }
    }

    #[test]
    fn mode_predicates_follow_the_existing_restriction_kinds() {
        // Directory restriction: workspace-only on the two narrow rungs.
        assert!(SandboxMode::Strict.workspace_only_files());
        assert!(SandboxMode::Standard.workspace_only_files());
        assert!(!SandboxMode::Relaxed.workspace_only_files());
        assert!(SandboxMode::Relaxed.permits_out_of_workspace_write());
        // Network cut: only the widest rung has egress.
        assert!(!SandboxMode::Strict.allows_network_egress());
        assert!(!SandboxMode::Relaxed.allows_network_egress());
        assert!(SandboxMode::Permissive.allows_network_egress());
        // Command-family boundary: allowlist on the narrowest rung only.
        assert!(SandboxMode::Strict.enforces_command_family_allowlist());
        assert!(!SandboxMode::Standard.enforces_command_family_allowlist());
        assert!(!SandboxMode::Strict.allows_high_risk_command_family());
        assert!(SandboxMode::Permissive.allows_high_risk_command_family());
    }

    #[tokio::test]
    async fn virtual_clock_makes_authorization_timestamps_injectable() {
        let capability = CapabilityId::new("tool.shell").unwrap();
        let arguments = args_with("relaxed", "one export file");
        let session = SessionId::new();
        let start = Timestamp::from_epoch_millis(1_767_225_600_000).unwrap();
        let gate = SandboxEscalationGate::new(
            Arc::new(FixedDecision {
                name: "inner_policy",
                decision: Decision::Allow,
            }),
            SandboxMode::Standard,
            Arc::new(VirtualClock::new(start.as_datetime())),
        );
        gate.evaluate_verbose(&dispatch_request(session, &capability, &arguments))
            .await;
        gate.approval_resolved(&session, &capability);
        let records = gate.ledger().audit_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].timestamp.epoch_millis(), 1_767_225_600_000);
    }
}
