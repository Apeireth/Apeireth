//! Governance chain: one ordered execution of the six governance phases.
//!
//! ADR-0010 (R8) found six governance steps implemented as separate pieces —
//! gate check, deliberation rubric, human-authority multi-signature,
//! approval-closure audit pair, tamper-evident audit append, and self-disable
//! audit — with **no single function that runs them in order**. This module is
//! that single function's skeleton (R8-A).
//!
//! # What is wired today (honest status)
//!
//! - [`GovernancePhase::Gate`]: runs the injected [`GovernanceHook`] through
//!   [`GovernanceHook::evaluate_verbose`]; `Deny` short-circuits.
//! - [`GovernancePhase::HumanAuthority`]: only runs when
//!   [`ChainConfig::require_human_authority`] is set; verification is delegated
//!   to [`MultiSignVerifier`]. The default verifier is an explicit
//!   [`ChainSignatureMode::Placeholder`] stub that does **not** accept any
//!   signature (cryptographic multi-signature is v2.1, NOT IMPLEMENTED).
//! - [`GovernancePhase::ApprovalClosure`]: a `RequireApproval` verdict is
//!   reported as [`ChainOutcome::RequireApproval`] and audited; it does **not**
//!   consume an approval (that remains the runtime's approval loop, R8-B).
//! - [`GovernancePhase::AuditAppend`]: appends one record through
//!   [`AuditSink`]; a sink failure is fail-closed (denied at this phase).
//! - [`GovernancePhase::Reflection`]: scheduled through [`ReflectionScheduler`]
//!   after a successful audit append. Default scheduler is in-process
//!   bookkeeping only; no daemon exists (R8-D).
//! - [`GovernancePhase::SelfDisable`]: records that a self-disable audit trail
//!   would be required; the `SelfDisableAudit` hookup is NOT IMPLEMENTED here.
//!
//! Deliberation ([`Ballot`](crate::Ballot)/[`synthesize`](crate::synthesize))
//! stays a library helper; it is not invoked by this chain yet (R8-B).
//!
//! # Layering
//!
//! `apeireth-governance` depends on `apeireth-core` only. This module therefore
//! defines its own injected ports ([`MultiSignVerifier`], [`AuditSink`],
//! [`ReflectionScheduler`], [`ChainClock`]) so the runtime/gateway assemble them
//! without a cross-layer dependency.

use std::sync::{Arc, Mutex};

use apeireth_core::kernel::{CapabilityId, SessionId, Timestamp};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{AuditHashChain, Decision, GovernanceHook, GovernanceRequest, GovernanceVerdict};

/// Schema version of [`ChainRecord`]. Bump only for additive/breaking changes
/// that a reader must be able to tell apart; v1 is the R8-A skeleton.
pub const GOVERNANCE_CHAIN_VERSION: u32 = 1;

/// Why the chain could not complete a phase.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChainError {
    /// The audit sink rejected the record (fail-closed).
    #[error("governance audit append failed: {0}")]
    AuditAppend(String),
}

/// The six phases, in execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GovernancePhase {
    /// Ask the installed [`GovernanceHook`].
    Gate,
    /// Deliberation rubric (Ballot/synthesize) — library helper, not invoked
    /// by this chain yet.
    Deliberation,
    /// Human-authority multi-signature verification.
    HumanAuthority,
    /// Approval-closure record (asked/decision pair).
    ApprovalClosure,
    /// Tamper-evident audit append.
    AuditAppend,
    /// Schedule the reflection audit.
    Reflection,
    /// Self-disable audit trail.
    SelfDisable,
}

impl GovernancePhase {
    /// All phases in execution order.
    pub const ALL: [Self; 7] = [
        Self::Gate,
        Self::Deliberation,
        Self::HumanAuthority,
        Self::ApprovalClosure,
        Self::AuditAppend,
        Self::Reflection,
        Self::SelfDisable,
    ];

    /// Stable label for logs and records.
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Gate => "gate",
            Self::Deliberation => "deliberation",
            Self::HumanAuthority => "human_authority",
            Self::ApprovalClosure => "approval_closure",
            Self::AuditAppend => "audit_append",
            Self::Reflection => "reflection",
            Self::SelfDisable => "self_disable",
        }
    }
}

/// How signatures were verified.
///
/// [`Self::Placeholder`] is explicitly not cryptographic; a record carrying it
/// must never be read as "proven".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChainSignatureMode {
    /// String signatures checked by structure/identity only (v2.0).
    Placeholder,
    /// Real cryptographic verification (v2.1, NOT IMPLEMENTED).
    Cryptographic,
}

/// Outcome of one chain run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainOutcome {
    /// Every configured phase passed.
    Allowed(ChainRecord),
    /// A phase refused; the reason names the phase.
    Denied {
        /// The phase that refused.
        phase: GovernancePhase,
        /// Why.
        reason: String,
    },
    /// A human must decide; the chain stops here.
    RequireApproval(ChainRecord),
    /// A required phase has no configuration at all (not "allowed").
    NotConfigured {
        /// The missing phase.
        phase: GovernancePhase,
    },
}

impl ChainOutcome {
    /// Whether the run ended in [`Self::Allowed`].
    pub const fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed(_))
    }

    /// The record, when the run got far enough to build one.
    pub const fn record(&self) -> Option<&ChainRecord> {
        match self {
            Self::Allowed(record) | Self::RequireApproval(record) => Some(record),
            Self::Denied { .. } | Self::NotConfigured { .. } => None,
        }
    }
}

/// One immutable record of what the chain did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainRecord {
    /// Schema version of this record.
    pub version: u32,
    /// Free-form subject (action label + optional action id).
    pub subject: String,
    /// Phases actually traversed, in order.
    pub trail: Vec<GovernancePhase>,
    /// Phases that were configured but skipped because they are not
    /// implemented. Never silently omitted.
    pub skipped: Vec<GovernancePhase>,
    /// Stable decision label (`allowed` / `require_approval`).
    pub decision: String,
    /// When the run happened (injected clock, never ambient).
    pub timestamp_ms: i64,
    /// Whether signatures were placeholder or cryptographic.
    pub signature_mode: ChainSignatureMode,
    /// Audit record hash, when the audit phase appended one.
    pub audit_hash: Option<String>,
    /// Reflection token, when the reflection phase scheduled one.
    pub reflection_token: Option<String>,
}

impl ChainRecord {
    /// The trail ends with `phase`.
    pub fn ended_at(&self, phase: GovernancePhase) -> bool {
        self.trail.last() == Some(&phase)
    }
}

/// One signature verification result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultiSignVerification {
    /// Enough distinct signatures.
    Accepted,
    /// Too few signatures.
    Insufficient {
        /// Required count.
        required: u8,
        /// Received count.
        received: u8,
    },
    /// No verifier is configured; the chain must fail closed.
    NotConfigured,
}

/// Injected port: verify human-authority signatures.
pub trait MultiSignVerifier: Send + Sync {
    /// Which mode this verifier operates in; recorded in the chain record.
    fn mode(&self) -> ChainSignatureMode;

    /// Verify one signature set.
    fn verify(&self, signatures: &[String]) -> MultiSignVerification;
}

/// Default verifier: placeholder mode, rejects everything (fail-closed).
///
/// Cryptographic multi-signature is v2.1 work; pretending a string is proven
/// would be exactly the dishonesty this project's 0 装 rule forbids.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlaceholderMultiSignVerifier;

impl MultiSignVerifier for PlaceholderMultiSignVerifier {
    fn mode(&self) -> ChainSignatureMode {
        ChainSignatureMode::Placeholder
    }

    fn verify(&self, _signatures: &[String]) -> MultiSignVerification {
        MultiSignVerification::NotConfigured
    }
}

/// Injected port: append one tamper-evident audit record.
pub trait AuditSink: Send + Sync {
    /// Append, returning the new record's hash.
    fn append(
        &self,
        event_kind: &str,
        subject: &str,
        timestamp: Timestamp,
    ) -> Result<String, String>;

    /// Number of appended records so far.
    fn len(&self) -> usize;

    /// Whether no record was appended yet.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// In-memory audit sink backed by [`AuditHashChain`].
#[derive(Debug, Default)]
pub struct InMemoryAuditSink {
    chain: Mutex<AuditHashChain>,
}

impl InMemoryAuditSink {
    /// Empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify the underlying chain.
    pub fn verify(&self) -> Result<(), crate::AuditChainError> {
        let chain = self.chain.lock().expect("audit chain mutex poisoned");
        chain.verify()
    }

    /// The appended records (for tests and inspection).
    pub fn records(&self) -> Vec<crate::AuditRecord> {
        let chain = self.chain.lock().expect("audit chain mutex poisoned");
        chain.records().to_vec()
    }
}

impl AuditSink for InMemoryAuditSink {
    fn append(
        &self,
        event_kind: &str,
        subject: &str,
        timestamp: Timestamp,
    ) -> Result<String, String> {
        let mut chain = self
            .chain
            .lock()
            .map_err(|_| "audit chain mutex poisoned".to_string())?;
        let record = chain.append(event_kind, subject, timestamp);
        Ok(record.current_hash.clone())
    }

    fn len(&self) -> usize {
        self.chain
            .lock()
            .map(|chain| chain.len())
            .unwrap_or_default()
    }
}

/// One reflection-scheduling result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReflectionSchedule {
    /// Scheduled with an opaque token.
    Scheduled {
        /// Opaque token the runtime can later cancel.
        token: String,
    },
    /// No scheduler is configured; nothing was scheduled.
    NotConfigured,
}

/// Injected port: schedule the (asynchronous) reflection audit.
pub trait ReflectionScheduler: Send + Sync {
    /// Schedule reflection for one audit hash.
    fn schedule(&self, audit_hash: &str, timestamp: Timestamp) -> ReflectionSchedule;

    /// Take the reflections whose delay has elapsed (R8-C).
    ///
    /// Default: nothing is ever due (matches the immediate scheduler, whose
    /// reflection is bookkeeping only). A delayed scheduler returns due items
    /// once; the chain audits each one it receives.
    fn take_due(&self, _now: Timestamp) -> Vec<ReflectionDue> {
        Vec::new()
    }
}

/// One reflection whose in-process delay has elapsed (R8-C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflectionDue {
    /// Opaque token handed out when the reflection was scheduled.
    pub token: String,
    /// Audit hash the reflection is about.
    pub audit_hash: String,
    /// When the reflection was scheduled.
    pub scheduled_at: Timestamp,
    /// When the delay elapsed.
    pub due_at: Timestamp,
}

/// Default scheduler: in-process bookkeeping only.
///
/// There is no background thread, daemon, or persistence: a token is handed out
/// and the caller decides what to do. An actual reflection runner is R8-D and
/// is NOT IMPLEMENTED.
#[derive(Debug, Default)]
pub struct ImmediateReflectionScheduler {
    counter: Mutex<u64>,
}

impl ImmediateReflectionScheduler {
    /// Fresh scheduler.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ReflectionScheduler for ImmediateReflectionScheduler {
    fn schedule(&self, audit_hash: &str, timestamp: Timestamp) -> ReflectionSchedule {
        let mut counter = self.counter.lock().expect("reflection counter poisoned");
        *counter += 1;
        let token = format!(
            "reflection-{}-{}-{}",
            counter,
            timestamp.epoch_millis(),
            &audit_hash[..audit_hash.len().min(12)]
        );
        ReflectionSchedule::Scheduled { token }
    }
}

/// In-process delayed reflection queue (R8-C).
///
/// `schedule` records a pending reflection due `delay_ms` after the supplied
/// instant; `take_due` hands the elapsed ones back exactly once so the chain can
/// audit them. No thread, daemon, persistence, or autopilot: it is a queue, not
/// a runner (autopilot stays NOT IMPLEMENTED, ADR-0010 D5).
pub struct DelayedReflectionScheduler {
    clock: Arc<dyn ChainClock>,
    delay_ms: i64,
    counter: Mutex<u64>,
    pending: Mutex<Vec<PendingReflection>>,
}

impl std::fmt::Debug for DelayedReflectionScheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DelayedReflectionScheduler")
            .field("delay_ms", &self.delay_ms)
            .field("pending", &self.pending_len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
struct PendingReflection {
    token: String,
    audit_hash: String,
    scheduled_at: Timestamp,
    due_at: Timestamp,
}

impl DelayedReflectionScheduler {
    /// A queue with `delay_ms` (clamped to non-negative) at the given clock.
    pub fn new(clock: Arc<dyn ChainClock>, delay_ms: i64) -> Self {
        Self {
            clock,
            delay_ms: delay_ms.max(0),
            counter: Mutex::new(0),
            pending: Mutex::new(Vec::new()),
        }
    }

    /// Configured delay in milliseconds.
    pub const fn delay_ms(&self) -> i64 {
        self.delay_ms
    }

    /// Number of reflections still waiting for their delay.
    pub fn pending_len(&self) -> usize {
        self.pending
            .lock()
            .map(|pending| pending.len())
            .unwrap_or(0)
    }

    /// [`ReflectionScheduler::take_due`] evaluated at the scheduler's own clock.
    pub fn take_due_now(&self) -> Vec<ReflectionDue> {
        <Self as ReflectionScheduler>::take_due(self, self.clock.now())
    }
}

impl ReflectionScheduler for DelayedReflectionScheduler {
    fn schedule(&self, audit_hash: &str, timestamp: Timestamp) -> ReflectionSchedule {
        let mut counter = self.counter.lock().expect("reflection counter poisoned");
        *counter += 1;
        let token = format!(
            "reflection-{}-{}-{}",
            counter,
            timestamp.epoch_millis(),
            &audit_hash[..audit_hash.len().min(12)]
        );
        let due_at =
            Timestamp::from_epoch_millis(timestamp.epoch_millis().saturating_add(self.delay_ms))
                .unwrap_or(timestamp);
        self.pending
            .lock()
            .expect("reflection queue poisoned")
            .push(PendingReflection {
                token: token.clone(),
                audit_hash: audit_hash.to_string(),
                scheduled_at: timestamp,
                due_at,
            });
        ReflectionSchedule::Scheduled { token }
    }

    fn take_due(&self, now: Timestamp) -> Vec<ReflectionDue> {
        let now_ms = now.epoch_millis();
        let mut pending = self.pending.lock().expect("reflection queue poisoned");
        let mut due = Vec::new();
        pending.retain(|item| {
            if item.due_at.epoch_millis() <= now_ms {
                due.push(ReflectionDue {
                    token: item.token.clone(),
                    audit_hash: item.audit_hash.clone(),
                    scheduled_at: item.scheduled_at,
                    due_at: item.due_at,
                });
                false
            } else {
                true
            }
        });
        due
    }
}

/// Injected port: the clock the chain reads.
pub trait ChainClock: Send + Sync {
    /// Current instant.
    fn now(&self) -> Timestamp;
}

/// Default clock: system time (production).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemChainClock;

impl ChainClock for SystemChainClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// Fixed clock (tests / deterministic replay).
#[derive(Debug, Clone, Copy)]
pub struct FixedChainClock {
    epoch_millis: i64,
}

impl FixedChainClock {
    /// Clock pinned to one epoch-millis instant.
    pub const fn new(epoch_millis: i64) -> Self {
        Self { epoch_millis }
    }
}

impl ChainClock for FixedChainClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_epoch_millis(self.epoch_millis).expect("fixed instant must be valid")
    }
}

/// Configuration of the chain's optional phases.
#[derive(Debug, Clone, Default)]
pub struct ChainConfig {
    /// Run the human-authority phase.
    pub require_human_authority: bool,
    /// Signatures offered to the verifier (R8-A keeps them in config; R8-B
    /// moves them onto the request once the runtime owns the approval loop).
    pub human_authority_signatures: Vec<String>,
    /// Record that a self-disable audit trail is required.
    pub self_disable_requested: bool,
}

/// The ordered governance chain.
pub struct GovernanceChain {
    gate: Option<Arc<dyn GovernanceHook>>,
    multi_sign: Arc<dyn MultiSignVerifier>,
    audit: Arc<dyn AuditSink>,
    reflection: Arc<dyn ReflectionScheduler>,
    clock: Arc<dyn ChainClock>,
    config: ChainConfig,
}

impl GovernanceChain {
    /// A chain whose gate phase runs `gate`.
    pub fn new(gate: Arc<dyn GovernanceHook>) -> Self {
        Self {
            gate: Some(gate),
            multi_sign: Arc::new(PlaceholderMultiSignVerifier),
            audit: Arc::new(InMemoryAuditSink::new()),
            reflection: Arc::new(ImmediateReflectionScheduler::new()),
            clock: Arc::new(SystemChainClock),
            config: ChainConfig::default(),
        }
    }

    /// A chain with no gate configured (every run reports `NotConfigured`).
    pub fn without_gate() -> Self {
        Self {
            gate: None,
            multi_sign: Arc::new(PlaceholderMultiSignVerifier),
            audit: Arc::new(InMemoryAuditSink::new()),
            reflection: Arc::new(ImmediateReflectionScheduler::new()),
            clock: Arc::new(SystemChainClock),
            config: ChainConfig::default(),
        }
    }

    /// Replace the signature verifier.
    #[must_use]
    pub fn with_multi_sign_verifier(mut self, verifier: Arc<dyn MultiSignVerifier>) -> Self {
        self.multi_sign = verifier;
        self
    }

    /// Replace the audit sink.
    #[must_use]
    pub fn with_audit_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.audit = sink;
        self
    }

    /// Replace the reflection scheduler.
    #[must_use]
    pub fn with_reflection_scheduler(mut self, scheduler: Arc<dyn ReflectionScheduler>) -> Self {
        self.reflection = scheduler;
        self
    }

    /// Replace the clock.
    #[must_use]
    pub fn with_clock(mut self, clock: Arc<dyn ChainClock>) -> Self {
        self.clock = clock;
        self
    }

    /// Replace the configuration.
    #[must_use]
    pub fn with_config(mut self, config: ChainConfig) -> Self {
        self.config = config;
        self
    }

    /// The installed audit sink (inspection/tests).
    pub fn audit_sink(&self) -> &Arc<dyn AuditSink> {
        &self.audit
    }

    /// The injected gate hook, when one is configured.
    ///
    /// Exposed so the R8-B1 [`ChainGovernanceHook`] adapter can forward
    /// [`GovernanceHook::approval_resolved`] to the gate.
    pub fn gate(&self) -> Option<&Arc<dyn GovernanceHook>> {
        self.gate.as_ref()
    }

    /// Run the chain for one request.
    ///
    /// `request` already carries the turn's `TurnSecurityContext`, so no
    /// separate context argument is taken (ADR-0010 §9 records this narrowing).
    pub async fn run(&self, request: &GovernanceRequest<'_>) -> ChainOutcome {
        let Some(gate) = self.gate.as_ref() else {
            return ChainOutcome::NotConfigured {
                phase: GovernancePhase::Gate,
            };
        };

        let timestamp = self.clock.now();
        // R8-C: before deciding anything, audit reflections whose in-process
        // delay elapsed. A broken audit sink stays fail-closed.
        if let Err(reason) = self.sweep_due_reflections(timestamp) {
            return ChainOutcome::Denied {
                phase: GovernancePhase::Reflection,
                reason,
            };
        }
        let subject = Self::subject_of(request);
        let signature_mode = self.multi_sign.mode();
        let mut record = ChainRecord {
            version: GOVERNANCE_CHAIN_VERSION,
            subject: subject.clone(),
            trail: vec![GovernancePhase::Gate],
            skipped: Vec::new(),
            decision: String::new(),
            timestamp_ms: timestamp.epoch_millis(),
            signature_mode,
            audit_hash: None,
            reflection_token: None,
        };

        // Phase: Gate.
        let verdict = gate.evaluate_verbose(request).await;
        match &verdict.decision {
            crate::Decision::Allow => {}
            crate::Decision::Deny { reason } => {
                let reason = reason.clone();
                return self.finish_denied(record, GovernancePhase::Gate, reason, timestamp);
            }
            crate::Decision::RequireApproval { reason: _ } => {
                record.decision = "require_approval".to_string();
                record.trail.push(GovernancePhase::ApprovalClosure);
                if self.config.require_human_authority {
                    match self.verify_human_authority(&mut record) {
                        HumanAuthorityStep::Accepted => {}
                        HumanAuthorityStep::Denied(reason) => {
                            return self.finish_denied(
                                record,
                                GovernancePhase::HumanAuthority,
                                reason,
                                timestamp,
                            );
                        }
                    }
                }
                // The approval-closure record is a pair of asked/decision events;
                // producing the "asked" half is R8-B, so it is honestly skipped.
                record.skipped.push(GovernancePhase::ApprovalClosure);
                record
                    .skipped
                    .extend(Self::unimplemented_phases(&self.config));
                return self.finish_recorded(
                    record,
                    ChainOutcome::RequireApproval,
                    "governance.approval_required",
                    timestamp,
                );
            }
        }

        // Phase: HumanAuthority (only when configured).
        if self.config.require_human_authority {
            record.trail.push(GovernancePhase::HumanAuthority);
            match self.verify_human_authority(&mut record) {
                HumanAuthorityStep::Accepted => {}
                HumanAuthorityStep::Denied(reason) => {
                    return self.finish_denied(
                        record,
                        GovernancePhase::HumanAuthority,
                        reason,
                        timestamp,
                    );
                }
            }
        }

        // Phase: Deliberation — library helper only, not invoked (R8-B).
        record.skipped.push(GovernancePhase::Deliberation);
        record
            .skipped
            .extend(Self::unimplemented_phases(&self.config));

        record.decision = "allowed".to_string();
        self.finish_recorded(
            record,
            ChainOutcome::Allowed,
            "governance.allowed",
            timestamp,
        )
    }

    fn subject_of(request: &GovernanceRequest<'_>) -> String {
        match request.action_id {
            Some(action_id) => format!("{}:{}", request.action.label(), action_id),
            None => request.action.label().to_string(),
        }
    }

    fn verify_human_authority(&self, record: &mut ChainRecord) -> HumanAuthorityStep {
        if !record.trail.contains(&GovernancePhase::HumanAuthority) {
            record.trail.push(GovernancePhase::HumanAuthority);
        }
        match self
            .multi_sign
            .verify(&self.config.human_authority_signatures)
        {
            MultiSignVerification::Accepted => HumanAuthorityStep::Accepted,
            MultiSignVerification::Insufficient { required, received } => {
                HumanAuthorityStep::Denied(format!(
                    "human authority signatures insufficient: required {required}, received {received}"
                ))
            }
            MultiSignVerification::NotConfigured => HumanAuthorityStep::Denied(
                "human authority verifier is not configured (placeholder mode fails closed)"
                    .to_string(),
            ),
        }
    }

    fn unimplemented_phases(config: &ChainConfig) -> Vec<GovernancePhase> {
        let mut phases = Vec::new();
        if !config.self_disable_requested {
            // The self-disable phase has no trail hookup yet; record it as
            // skipped either way so it is never silently omitted.
            phases.push(GovernancePhase::SelfDisable);
        }
        phases
    }

    fn finish_denied(
        &self,
        mut record: ChainRecord,
        phase: GovernancePhase,
        reason: String,
        timestamp: Timestamp,
    ) -> ChainOutcome {
        Self::note_denial(&mut record, phase);
        record.decision = format!("denied_at_{}", phase.label());
        let event_kind = format!("governance.denied.{}", phase.label());
        match self.audit.append(&event_kind, &record.subject, timestamp) {
            Ok(hash) => {
                record.audit_hash = Some(hash);
                self.schedule_reflection(&mut record, timestamp);
            }
            Err(error) => {
                record.skipped.push(GovernancePhase::AuditAppend);
                return ChainOutcome::Denied {
                    phase: GovernancePhase::AuditAppend,
                    reason: format!(
                        "{} (denial at {} could not be audited: {error})",
                        error,
                        phase.label()
                    ),
                };
            }
        }
        ChainOutcome::Denied { phase, reason }
    }

    fn finish_recorded(
        &self,
        mut record: ChainRecord,
        outcome: fn(ChainRecord) -> ChainOutcome,
        event_kind: &str,
        timestamp: Timestamp,
    ) -> ChainOutcome {
        match self.audit.append(event_kind, &record.subject, timestamp) {
            Ok(hash) => {
                record.trail.push(GovernancePhase::AuditAppend);
                record.audit_hash = Some(hash);
                self.schedule_reflection(&mut record, timestamp);
            }
            Err(error) => {
                return ChainOutcome::Denied {
                    phase: GovernancePhase::AuditAppend,
                    reason: format!("{error} (fail-closed: unaudited outcome is not allowed)"),
                };
            }
        }
        outcome(record)
    }

    fn schedule_reflection(&self, record: &mut ChainRecord, timestamp: Timestamp) {
        record.trail.push(GovernancePhase::Reflection);
        if let Some(audit_hash) = record.audit_hash.as_deref() {
            if let ReflectionSchedule::Scheduled { token } =
                self.reflection.schedule(audit_hash, timestamp)
            {
                record.reflection_token = Some(token);
            }
        }
    }

    /// Audit every reflection whose in-process delay has elapsed (R8-C).
    ///
    /// The default scheduler never reports due items, so this is a no-op unless
    /// a [`DelayedReflectionScheduler`] is injected. Each due reflection is
    /// appended exactly once; a sink failure is returned as a reason so `run`
    /// can fail closed at [`GovernancePhase::Reflection`].
    fn sweep_due_reflections(&self, now: Timestamp) -> Result<(), String> {
        for due in self.reflection.take_due(now) {
            self.audit
                .append("governance.reflection_due", &due.audit_hash, now)
                .map_err(|error| format!("{}: {error}", GovernancePhase::Reflection.label()))?;
        }
        Ok(())
    }

    fn note_denial(record: &mut ChainRecord, phase: GovernancePhase) {
        if !record.trail.contains(&phase) {
            record.trail.push(phase);
        }
    }
}

/// Internal human-authority step result.
enum HumanAuthorityStep {
    Accepted,
    Denied(String),
}

// ---------------------------------------------------------------------------
// R8-B1: chain as a governance hook
// ---------------------------------------------------------------------------

/// Adapter that exposes [`GovernanceChain`] wherever a [`GovernanceHook`] is
/// expected (ADR-0010 §9, R8-B1).
///
/// One evaluation = one chain run, so every action decided through this hook
/// goes through the six ordered phases. The runtime keeps its
/// `Arc<dyn GovernanceHook>` interface, which is why R8-B2 needs no runtime
/// change (design candidate §3, 方案 B).
///
/// **PRODUCTION WIRED**（2026-10-25 核对）：生产治理入口就是它 ——
/// `cli::governance_chain::production()` 构造本类型并交给 runtime 的
/// `with_governance`（`cli/lib.rs` 装配根调用）。原文曾写「no production
/// assembly constructs this type yet」，已过期。
pub struct ChainGovernanceHook {
    chain: Arc<GovernanceChain>,
}

impl ChainGovernanceHook {
    /// Wrap a chain.
    pub fn new(chain: Arc<GovernanceChain>) -> Self {
        Self { chain }
    }

    /// The wrapped chain.
    pub fn chain(&self) -> &Arc<GovernanceChain> {
        &self.chain
    }
}

impl std::fmt::Debug for ChainGovernanceHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChainGovernanceHook")
            .field("gate", &self.chain.gate().map(|gate| gate.name()))
            .finish()
    }
}

#[async_trait::async_trait]
impl GovernanceHook for ChainGovernanceHook {
    fn name(&self) -> &str {
        "governance_chain"
    }

    async fn evaluate(&self, request: &GovernanceRequest<'_>) -> Decision {
        self.evaluate_verbose(request).await.decision
    }

    async fn evaluate_verbose(&self, request: &GovernanceRequest<'_>) -> GovernanceVerdict {
        let decision = match self.chain.run(request).await {
            ChainOutcome::Allowed(_) => Decision::Allow,
            ChainOutcome::Denied { phase, reason } => {
                Decision::deny(format!("{}: {reason}", phase.label()))
            }
            ChainOutcome::RequireApproval(_) => {
                Decision::require_approval("governance chain requires human approval")
            }
            ChainOutcome::NotConfigured { phase } => Decision::deny(format!(
                "governance chain not configured at {}",
                phase.label()
            )),
        };
        GovernanceVerdict::new(self.name(), decision)
    }

    fn approval_resolved(&self, session: &SessionId, capability: &CapabilityId) {
        if let Some(gate) = self.chain.gate() {
            gate.approval_resolved(session, capability);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
    use async_trait::async_trait;

    use super::*;
    use crate::{Decision, GovernanceVerdict};

    struct FixedDecision(Decision, &'static str);

    #[async_trait]
    impl GovernanceHook for FixedDecision {
        fn name(&self) -> &str {
            self.1
        }

        async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
            self.0.clone()
        }

        async fn evaluate_verbose(&self, _request: &GovernanceRequest<'_>) -> GovernanceVerdict {
            GovernanceVerdict::new(self.1, self.0.clone())
        }
    }

    struct AcceptingVerifier;

    impl MultiSignVerifier for AcceptingVerifier {
        fn mode(&self) -> ChainSignatureMode {
            ChainSignatureMode::Placeholder
        }

        fn verify(&self, _signatures: &[String]) -> MultiSignVerification {
            MultiSignVerification::Accepted
        }
    }

    struct FailingAudit;

    impl AuditSink for FailingAudit {
        fn append(
            &self,
            _event_kind: &str,
            _subject: &str,
            _timestamp: Timestamp,
        ) -> Result<String, String> {
            Err("disk full".to_string())
        }

        fn len(&self) -> usize {
            0
        }
    }

    fn request<'a>(action: crate::Action<'a>) -> GovernanceRequest<'a> {
        GovernanceRequest::new(action, SessionId::new(), TraceId::new(), 1)
    }

    fn completion_request<'a>(model: &'a str) -> GovernanceRequest<'a> {
        request(crate::Action::Completion {
            model,
            message_count: 0,
        })
    }

    #[tokio::test]
    async fn phases_are_ordered_and_labelled() {
        let labels: Vec<&str> = GovernancePhase::ALL.iter().map(|p| p.label()).collect();
        assert_eq!(
            labels,
            vec![
                "gate",
                "deliberation",
                "human_authority",
                "approval_closure",
                "audit_append",
                "reflection",
                "self_disable"
            ]
        );
        assert_eq!(GovernancePhase::ALL.len(), 7);
    }

    #[tokio::test]
    async fn no_gate_is_not_configured_and_audits_nothing() {
        let chain = GovernanceChain::without_gate();
        let outcome = chain.run(&completion_request("m")).await;
        assert_eq!(
            outcome,
            ChainOutcome::NotConfigured {
                phase: GovernancePhase::Gate
            }
        );
        assert!(chain.audit_sink().is_empty());
    }

    #[tokio::test]
    async fn allow_traverses_gate_audit_and_reflection() {
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_clock(Arc::new(FixedChainClock::new(1_700_000_000_000)));
        let outcome = chain.run(&completion_request("m")).await;
        let ChainOutcome::Allowed(record) = outcome else {
            panic!("expected allowed, got {outcome:?}");
        };
        assert_eq!(record.version, GOVERNANCE_CHAIN_VERSION);
        assert_eq!(record.decision, "allowed");
        assert_eq!(
            record.trail,
            vec![
                GovernancePhase::Gate,
                GovernancePhase::AuditAppend,
                GovernancePhase::Reflection,
            ]
        );
        assert!(record.skipped.contains(&GovernancePhase::Deliberation));
        assert!(record.audit_hash.is_some());
        assert!(record.reflection_token.is_some());
        assert_eq!(record.signature_mode, ChainSignatureMode::Placeholder);
        assert_eq!(chain.audit_sink().len(), 1);
    }

    #[tokio::test]
    async fn deny_short_circuits_and_is_audited() {
        let chain = GovernanceChain::new(Arc::new(FixedDecision(
            Decision::deny("not allowed"),
            "fixed_deny",
        )));
        let outcome = chain.run(&completion_request("m")).await;
        assert_eq!(
            outcome,
            ChainOutcome::Denied {
                phase: GovernancePhase::Gate,
                reason: "not allowed".to_string(),
            }
        );
        assert_eq!(chain.audit_sink().len(), 1);
    }

    #[tokio::test]
    async fn require_approval_is_distinct_from_deny() {
        let chain = GovernanceChain::new(Arc::new(FixedDecision(
            Decision::require_approval("needs a human"),
            "fixed_approval",
        )));
        let outcome = chain.run(&completion_request("m")).await;
        let ChainOutcome::RequireApproval(record) = outcome else {
            panic!("expected require_approval, got {outcome:?}");
        };
        assert_eq!(record.decision, "require_approval");
        assert!(record.trail.contains(&GovernancePhase::ApprovalClosure));
        assert!(record.skipped.contains(&GovernancePhase::ApprovalClosure));
        assert_eq!(chain.audit_sink().len(), 1);
    }

    #[tokio::test]
    async fn placeholder_verifier_fails_closed() {
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_config(ChainConfig {
                require_human_authority: true,
                ..ChainConfig::default()
            });
        let outcome = chain.run(&completion_request("m")).await;
        assert_eq!(
            outcome,
            ChainOutcome::Denied {
                phase: GovernancePhase::HumanAuthority,
                reason:
                    "human authority verifier is not configured (placeholder mode fails closed)"
                        .to_string(),
            }
        );
    }

    #[tokio::test]
    async fn human_authority_accepts_with_configured_verifier() {
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_multi_sign_verifier(Arc::new(AcceptingVerifier))
            .with_config(ChainConfig {
                require_human_authority: true,
                ..ChainConfig::default()
            });
        let outcome = chain.run(&completion_request("m")).await;
        assert!(outcome.is_allowed(), "got {outcome:?}");
    }

    #[tokio::test]
    async fn audit_failure_fails_closed() {
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_audit_sink(Arc::new(FailingAudit));
        let outcome = chain.run(&completion_request("m")).await;
        assert!(matches!(
            outcome,
            ChainOutcome::Denied {
                phase: GovernancePhase::AuditAppend,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn audit_chain_records_verify_and_link() {
        let sink = Arc::new(InMemoryAuditSink::new());
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_audit_sink(sink.clone());
        assert!(chain.run(&completion_request("m")).await.is_allowed());
        assert!(chain.run(&completion_request("m")).await.is_allowed());
        assert_eq!(sink.len(), 2);
        sink.verify().expect("chain must verify");
        let records = sink.records();
        assert_eq!(records[1].previous_hash, records[0].current_hash);
    }

    #[tokio::test]
    async fn records_serialize_stably() {
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_clock(Arc::new(FixedChainClock::new(1_700_000_000_000)));
        let ChainOutcome::Allowed(record) = chain.run(&completion_request("m")).await else {
            panic!("expected allowed");
        };
        let json = serde_json::to_string(&record).expect("serialize");
        let back: ChainRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, record);
        assert!(json.contains("\"decision\":\"allowed\""));
    }

    // -- R8-B1 adapter: chain as a governance hook --------------------------

    #[tokio::test]
    async fn chain_hook_allowed_path_maps_to_allow_and_audits() {
        let sink = Arc::new(InMemoryAuditSink::new());
        let chain = Arc::new(
            GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
                .with_audit_sink(sink.clone()),
        );
        let hook = ChainGovernanceHook::new(chain);
        assert_eq!(hook.name(), "governance_chain");
        let verdict = hook.evaluate_verbose(&completion_request("m")).await;
        assert_eq!(verdict.decision, Decision::Allow);
        assert_eq!(verdict.hook, "governance_chain");
        assert_eq!(sink.len(), 1);
    }

    #[tokio::test]
    async fn chain_hook_deny_carries_the_phase_label() {
        let chain = Arc::new(GovernanceChain::new(Arc::new(FixedDecision(
            Decision::deny("blocked by policy"),
            "fixed_deny",
        ))));
        let verdict = ChainGovernanceHook::new(chain)
            .evaluate_verbose(&completion_request("m"))
            .await;
        let Decision::Deny { reason } = verdict.decision else {
            panic!("expected deny");
        };
        assert!(reason.starts_with("gate: "), "reason = {reason}");
        assert!(reason.contains("blocked by policy"));
    }

    #[tokio::test]
    async fn chain_hook_require_approval_stays_distinct() {
        let chain = Arc::new(GovernanceChain::new(Arc::new(FixedDecision(
            Decision::require_approval("needs owner"),
            "fixed_approval",
        ))));
        let hook = ChainGovernanceHook::new(chain);
        let decision = GovernanceHook::evaluate(&hook, &completion_request("m")).await;
        assert!(matches!(decision, Decision::RequireApproval { .. }));
    }

    #[tokio::test]
    async fn chain_hook_without_gate_fails_closed() {
        let hook = ChainGovernanceHook::new(Arc::new(GovernanceChain::without_gate()));
        let decision = GovernanceHook::evaluate(&hook, &completion_request("m")).await;
        let Decision::Deny { reason } = decision else {
            panic!("expected deny");
        };
        assert!(
            reason.contains("not configured at gate"),
            "reason = {reason}"
        );
    }

    #[tokio::test]
    async fn chain_hook_forwards_approval_resolved_to_gate() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct CountingApproval(Arc<AtomicUsize>);

        #[async_trait]
        impl GovernanceHook for CountingApproval {
            fn name(&self) -> &str {
                "counting_approval"
            }

            async fn evaluate(&self, _request: &GovernanceRequest<'_>) -> Decision {
                Decision::Allow
            }

            fn approval_resolved(&self, _session: &SessionId, _capability: &CapabilityId) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let chain = Arc::new(GovernanceChain::new(Arc::new(CountingApproval(
            counter.clone(),
        ))));
        let hook = ChainGovernanceHook::new(chain);
        hook.approval_resolved(&SessionId::new(), &CapabilityId::new("tool.shell").unwrap());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    // -- R8-C delayed reflection queue --------------------------------------

    const T0: i64 = 1_700_000_000_000;

    fn fixed_clock() -> Arc<FixedChainClock> {
        Arc::new(FixedChainClock::new(T0))
    }

    fn timestamp_ms(epoch_millis: i64) -> Timestamp {
        Timestamp::from_epoch_millis(epoch_millis).expect("valid instant")
    }

    #[test]
    fn delayed_scheduler_releases_each_due_reflection_once() {
        let scheduler = DelayedReflectionScheduler::new(fixed_clock(), 5_000);
        assert_eq!(scheduler.delay_ms(), 5_000);
        let ReflectionSchedule::Scheduled { token } =
            scheduler.schedule("hash-abc", timestamp_ms(T0))
        else {
            panic!("expected scheduled");
        };
        assert!(token.starts_with("reflection-1-"), "token = {token}");
        assert_eq!(scheduler.pending_len(), 1);

        assert!(ReflectionScheduler::take_due(&scheduler, timestamp_ms(T0 + 4_999)).is_empty());
        let due = ReflectionScheduler::take_due(&scheduler, timestamp_ms(T0 + 5_000));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].token, token);
        assert_eq!(due[0].audit_hash, "hash-abc");
        assert_eq!(due[0].scheduled_at, timestamp_ms(T0));
        assert_eq!(due[0].due_at, timestamp_ms(T0 + 5_000));
        assert_eq!(scheduler.pending_len(), 0);
        assert!(ReflectionScheduler::take_due(&scheduler, timestamp_ms(T0 + 60_000)).is_empty());
    }

    #[test]
    fn delayed_scheduler_take_due_now_uses_its_clock() {
        let scheduler = DelayedReflectionScheduler::new(fixed_clock(), 0);
        let _ = scheduler.schedule("hash-now", timestamp_ms(T0));
        let due = scheduler.take_due_now();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].audit_hash, "hash-now");
        assert!(scheduler.take_due_now().is_empty());
    }

    #[tokio::test]
    async fn chain_audits_due_reflections_before_deciding() {
        let sink = Arc::new(InMemoryAuditSink::new());
        let clock = fixed_clock();
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_clock(clock.clone())
            .with_reflection_scheduler(Arc::new(DelayedReflectionScheduler::new(clock, 0)))
            .with_audit_sink(sink.clone());

        assert!(chain.run(&completion_request("m")).await.is_allowed());
        assert_eq!(sink.len(), 1);
        // The first run scheduled a reflection due immediately; the next run
        // audits it before deciding.
        assert!(chain.run(&completion_request("m")).await.is_allowed());
        assert_eq!(sink.len(), 3);
        let records = sink.records();
        assert_eq!(records[1].event_kind, "governance.reflection_due");
        assert_eq!(records[2].event_kind, "governance.allowed");
        sink.verify().expect("chain must verify");
    }

    #[tokio::test]
    async fn default_scheduler_never_reports_due_reflections() {
        let sink = Arc::new(InMemoryAuditSink::new());
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_clock(fixed_clock())
            .with_audit_sink(sink.clone());
        assert!(chain.run(&completion_request("m")).await.is_allowed());
        assert!(chain.run(&completion_request("m")).await.is_allowed());
        assert_eq!(sink.len(), 2);
        for record in sink.records() {
            assert_eq!(record.event_kind, "governance.allowed");
        }
    }

    #[tokio::test]
    async fn chain_fails_closed_when_due_reflection_audit_fails() {
        let clock = fixed_clock();
        let scheduler = Arc::new(DelayedReflectionScheduler::new(clock.clone(), 0));
        let _ = scheduler.schedule("hash-pending", timestamp_ms(T0));
        let chain = GovernanceChain::new(Arc::new(FixedDecision(Decision::Allow, "fixed_allow")))
            .with_clock(clock)
            .with_reflection_scheduler(scheduler)
            .with_audit_sink(Arc::new(FailingAudit));
        let outcome = chain.run(&completion_request("m")).await;
        assert!(matches!(
            outcome,
            ChainOutcome::Denied {
                phase: GovernancePhase::Reflection,
                ..
            }
        ));
    }
}
