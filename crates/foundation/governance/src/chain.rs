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

use apeireth_core::kernel::Timestamp;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{AuditHashChain, GovernanceHook, GovernanceRequest};

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use apeireth_core::kernel::{SessionId, TraceId};
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
}
