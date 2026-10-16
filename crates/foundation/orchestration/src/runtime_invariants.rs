//! Runtime invariant registry: online guards over the audit event stream.
//!
//! Offline model-level proofs establish properties up to a model boundary; the
//! integration layer can still violate them without anyone noticing. This
//! module turns a small set of those proven properties into live guards: every
//! event that flows through an audit chain or event stream is checked, item by
//! item, against the registered invariants.
//!
//! # Observation layer
//!
//! Nothing here changes a governance verdict. A violation is a structured
//! record (`InvariantViolation`) that names the invariant, the module it
//! belongs to, and a legible detail. Two consumption modes exist:
//!
//! - **fail-fast** — the first violation is raised as an error to the call
//!   site, which decides whether to block;
//! - **log-only** — violations are collected and recorded for audit while the
//!   stream continues.
//!
//! Each invariant can be enabled or disabled by name (the configuration
//! surface), so a noisy guard is silenced without unregistering it.
//!
//! # Audit event vocabulary
//!
//! [`AuditEvent`] is the normalized observation vocabulary. Constructors
//! document the emit contract for each observable stream position:
//!
//! | constructor | stream position | observable fact |
//! |---|---|---|
//! | [`AuditEvent::side_effect`] | approved capability dispatch | one side effect executed for one request |
//! | [`AuditEvent::forget`] | memory governance forget | one forget applied to one subject |
//! | [`AuditEvent::state_change`] / [`AuditEvent::state_change_for_forget`] | memory governance write | one state-change record written |
//! | [`AuditEvent::protect`] / [`AuditEvent::unprotect`] | memory governance protect | a protect marker set or cleared on one subject |
//! | [`AuditEvent::cleanup`] | retention sweep / cleanup pass | one subject entered the cleanup stream |
//!
//! # Order independence
//!
//! Checks are pure functions of one event plus stream-wide folded facts
//! ([`StreamFacts`]: identity totals and per-subject presence across the whole
//! stream), so the violation set does not depend on the order in which the
//! stream arrives.
//!
//! # First batch
//!
//! The registered defaults mirror the offline-proved property families by name
//! and assertion semantics, restricted to what an audit event can actually
//! observe:
//!
//! - `inv_a_no_double_side_effect` — one approval produces exactly one side
//!   effect: side-effect events sharing one request id and one operation must
//!   not repeat;
//! - `inv_forget_idempotent` — repeated forget is idempotent: repeat forget
//!   events must not accompany new forget-attributable state-change records;
//! - `inv_sweep_keeps_protected` — protected memory never enters the cleanup
//!   stream: a live protect marker (set and not cleared) and cleanup events are
//!   mutually exclusive per subject;
//! - `inv_forget_confined_to_target` — a forget only touches its target: a
//!   state-change record attributable to a forget must carry the target
//!   subject.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

/// Separator inside folded identity keys; unrepresentable in the identifiers
/// below so two keys can never alias across their halves.
const KEY_SEPARATOR: char = '\u{1f}';

/// The observable vocabulary of the runtime audit stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditEventKind {
    /// One side effect was executed for one request.
    SideEffect,
    /// A forget operation was applied to one subject.
    Forget,
    /// A state-change record was written for one subject.
    StateChange,
    /// A protect marker is set on one subject.
    Protect,
    /// A protect marker is cleared on one subject.
    Unprotect,
    /// A cleanup pass touched one subject.
    Cleanup,
}

impl AuditEventKind {
    /// Stable wire label for this kind.
    pub const fn label(self) -> &'static str {
        match self {
            Self::SideEffect => "side_effect",
            Self::Forget => "forget",
            Self::StateChange => "state_change",
            Self::Protect => "protect",
            Self::Unprotect => "unprotect",
            Self::Cleanup => "cleanup",
        }
    }
}

/// Stream-folded identity facts the consumer attaches before checks run.
///
/// Every field is a whole-stream aggregate keyed by the event's own identity,
/// which is what makes verdicts independent of stream order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StreamFacts {
    /// Events in the stream sharing this event's identity key.
    pub identity_total: u64,
    /// `Forget` events in the stream sharing this event's subject.
    pub subject_forget_total: u64,
    /// `StateChange` records in the stream attributable to a forget of this
    /// event's subject (their correlation names the forget target).
    pub subject_forget_state_change_total: u64,
    /// Whether the subject is protected at stream level: protect markers
    /// minus unprotect markers is positive.
    pub subject_protected: bool,
    /// Whether the stream holds a `Cleanup` event for this event's subject.
    pub subject_cleaned: bool,
}

/// One event observed on the runtime audit stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// What happened.
    pub kind: AuditEventKind,
    /// Module that emitted the event; carried into violations for attribution.
    pub module: String,
    /// Primary subject: a request-scoped operation for side effects, a memory
    /// key for memory-governance events.
    pub subject: String,
    /// Correlation identity: the request id for side effects, the forget cause
    /// for forget-attributable state changes.
    pub correlation: String,
    /// Human-readable detail.
    pub detail: String,
    /// Stream-folded facts, filled by the consumer before checks run.
    pub facts: StreamFacts,
}

impl AuditEvent {
    /// One side effect executed for `request` under `operation`.
    pub fn side_effect(
        module: impl Into<String>,
        request: impl Into<String>,
        operation: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: AuditEventKind::SideEffect,
            module: module.into(),
            subject: operation.into(),
            correlation: request.into(),
            detail: detail.into(),
            facts: StreamFacts::default(),
        }
    }

    /// One forget applied to `subject`. The correlation records the forget
    /// cause so later state-change records can be attributed to it.
    pub fn forget(
        module: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject = subject.into();
        let correlation = forget_cause(&subject);
        Self {
            kind: AuditEventKind::Forget,
            module: module.into(),
            subject,
            correlation,
            detail: detail.into(),
            facts: StreamFacts::default(),
        }
    }

    /// One state-change record written for `subject` by `cause`.
    pub fn state_change(
        module: impl Into<String>,
        subject: impl Into<String>,
        cause: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: AuditEventKind::StateChange,
            module: module.into(),
            subject: subject.into(),
            correlation: cause.into(),
            detail: detail.into(),
            facts: StreamFacts::default(),
        }
    }

    /// One state-change record written by a forget whose target was `target`.
    /// A conforming write carries `target == subject`; anything else is an
    /// out-of-target write.
    pub fn state_change_for_forget(
        module: impl Into<String>,
        target: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let target = target.into();
        Self::state_change(module, subject, forget_cause(&target), detail)
    }

    /// A protect marker set on `subject`.
    pub fn protect(
        module: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: AuditEventKind::Protect,
            module: module.into(),
            subject: subject.into(),
            correlation: String::new(),
            detail: detail.into(),
            facts: StreamFacts::default(),
        }
    }

    /// A protect marker cleared on `subject`.
    pub fn unprotect(
        module: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: AuditEventKind::Unprotect,
            module: module.into(),
            subject: subject.into(),
            correlation: String::new(),
            detail: detail.into(),
            facts: StreamFacts::default(),
        }
    }

    /// A cleanup pass touched `subject`.
    pub fn cleanup(
        module: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: AuditEventKind::Cleanup,
            module: module.into(),
            subject: subject.into(),
            correlation: String::new(),
            detail: detail.into(),
            facts: StreamFacts::default(),
        }
    }

    /// The folded identity key: side effects are identified per (request,
    /// operation); memory-governance events per (kind, subject).
    pub fn identity_key(&self) -> String {
        match self.kind {
            AuditEventKind::SideEffect => format!(
                "side_effect{KEY_SEPARATOR}{}{KEY_SEPARATOR}{}",
                self.correlation, self.subject
            ),
            other => format!("{}{KEY_SEPARATOR}{}", other.label(), self.subject),
        }
    }
}

fn forget_cause(target: &str) -> String {
    format!("forget:{target}")
}

/// An event feed for observation points that only emit audit events and do not
/// decide anything themselves (the memory-governance surface, for example).
pub type AuditEventSink = std::sync::Arc<dyn Fn(AuditEvent) + Send + Sync>;

/// Bridge one [`InvariantAuditor`] into an [`AuditEventSink`].
///
/// Every event delivered to the sink is fed through the auditor's stream
/// consumption point. The sink never blocks its emitter: in fail-fast mode the
/// raised violation is left with the consuming call site (which is where
/// blocking belongs), and in log-only mode the violation is recorded as usual.
pub fn auditor_sink(auditor: std::sync::Arc<InvariantAuditor>) -> AuditEventSink {
    std::sync::Arc::new(move |event: AuditEvent| {
        let _ = auditor.observe(std::slice::from_ref(&event));
    })
}

/// A structured invariant violation: which invariant fired, which module owns
/// it, and what was observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantViolation {
    /// Stable invariant name.
    pub invariant: String,
    /// Module attribution for follow-up.
    pub module: String,
    /// Legible observation detail.
    pub detail: String,
}

/// The check shape every registered invariant implements: one event in,
/// `Ok(())` or a structured violation out.
pub type InvariantCheck = fn(&AuditEvent) -> Result<(), InvariantViolation>;

/// One registered runtime invariant.
#[derive(Debug, Clone)]
pub struct RuntimeInvariant {
    /// Stable name; also the configuration key for enable/disable.
    pub name: String,
    /// Module attribution recorded on every violation this invariant raises.
    pub module: String,
    /// The per-event check.
    pub check: InvariantCheck,
}

impl RuntimeInvariant {
    /// Register an invariant under `name`, attributed to `module`.
    pub fn new(name: impl Into<String>, module: impl Into<String>, check: InvariantCheck) -> Self {
        Self {
            name: name.into(),
            module: module.into(),
            check,
        }
    }
}

/// Registry of runtime invariants with per-name enable/disable.
#[derive(Debug, Clone, Default)]
pub struct InvariantRegistry {
    invariants: BTreeMap<String, RuntimeInvariant>,
    disabled: BTreeSet<String>,
}

impl InvariantRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one invariant. Empty and duplicate names are rejected.
    pub fn register(&mut self, invariant: RuntimeInvariant) -> Result<(), String> {
        if invariant.name.is_empty() {
            return Err("registered invariants must have a non-empty name".to_string());
        }
        if self.invariants.contains_key(&invariant.name) {
            return Err(format!("duplicate invariant name {:?}", invariant.name));
        }
        self.invariants.insert(invariant.name.clone(), invariant);
        Ok(())
    }

    /// Unregister one invariant by name. Returns whether it existed.
    pub fn unregister(&mut self, name: &str) -> bool {
        self.disabled.remove(name);
        self.invariants.remove(name).is_some()
    }

    /// Enable or disable one invariant by name without unregistering it.
    /// Returns whether the name is registered.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if !self.invariants.contains_key(name) {
            return false;
        }
        if enabled {
            self.disabled.remove(name);
        } else {
            self.disabled.insert(name.to_string());
        }
        true
    }

    /// Whether a registered invariant is currently enabled.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.invariants.contains_key(name) && !self.disabled.contains(name)
    }

    /// Whether a name is registered (enabled or not).
    pub fn contains(&self, name: &str) -> bool {
        self.invariants.contains_key(name)
    }

    /// Registered names in sorted order.
    pub fn names(&self) -> Vec<String> {
        self.invariants.keys().cloned().collect()
    }

    /// Number of registered invariants.
    pub fn len(&self) -> usize {
        self.invariants.len()
    }

    /// Whether nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.invariants.is_empty()
    }

    /// Registered invariants that are currently enabled, in sorted name order.
    pub fn enabled(&self) -> Vec<&RuntimeInvariant> {
        self.invariants
            .values()
            .filter(|invariant| self.is_enabled(&invariant.name))
            .collect()
    }
}

/// How the consumption point reacts to violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvariantMode {
    /// Raise the first violation as an error to the call site.
    FailFast,
    /// Record violations and keep consuming.
    LogOnly,
}

/// The stream consumption point.
///
/// Events are retained so that stream-wide facts (duplicate identity totals,
/// cross-kind subject presence) are visible to every check regardless of when
/// the violating event arrived. Violations are reported once per offending
/// identity; re-observing an unchanged stream reports nothing new.
#[derive(Debug)]
pub struct InvariantAuditor {
    inner: Mutex<AuditorState>,
}

#[derive(Debug)]
struct AuditorState {
    registry: InvariantRegistry,
    mode: InvariantMode,
    stream: Vec<AuditEvent>,
    violations: Vec<InvariantViolation>,
}

impl InvariantAuditor {
    /// An auditor over one registry in one mode.
    pub fn new(registry: InvariantRegistry, mode: InvariantMode) -> Self {
        Self {
            inner: Mutex::new(AuditorState {
                registry,
                mode,
                stream: Vec::new(),
                violations: Vec::new(),
            }),
        }
    }

    /// The configured consumption mode.
    pub fn mode(&self) -> InvariantMode {
        self.lock().mode
    }

    /// Events retained so far.
    pub fn stream_len(&self) -> usize {
        self.lock().stream.len()
    }

    /// Violations recorded so far (log-only mode accumulates).
    pub fn recorded(&self) -> Vec<InvariantViolation> {
        self.lock().violations.clone()
    }

    /// Registered invariant names in sorted order.
    pub fn names(&self) -> Vec<String> {
        self.lock().registry.names()
    }

    /// Register one invariant at the configuration surface.
    pub fn register(&self, invariant: RuntimeInvariant) -> Result<(), String> {
        self.lock().registry.register(invariant)
    }

    /// Unregister one invariant by name.
    pub fn unregister(&self, name: &str) -> bool {
        self.lock().registry.unregister(name)
    }

    /// Enable or disable one invariant by name.
    pub fn set_enabled(&self, name: &str, enabled: bool) -> bool {
        self.lock().registry.set_enabled(name, enabled)
    }

    /// Whether a registered invariant is enabled.
    pub fn is_enabled(&self, name: &str) -> bool {
        self.lock().registry.is_enabled(name)
    }

    /// The stream consumption point: append `events`, re-fold the stream, and
    /// check every event against every enabled invariant.
    ///
    /// Log-only returns the newly detected violations and records them.
    /// Fail-fast returns the first violation as an error and records nothing.
    pub fn observe(
        &self,
        events: &[AuditEvent],
    ) -> Result<Vec<InvariantViolation>, InvariantViolation> {
        let mut state = self.lock();
        state.stream.extend_from_slice(events);
        let all = run_checks(&state.registry, &state.stream);
        let fresh: Vec<InvariantViolation> = all
            .into_iter()
            .filter(|violation| !state.violations.contains(violation))
            .collect();
        match state.mode {
            InvariantMode::FailFast => match fresh.into_iter().next() {
                Some(violation) => Err(violation),
                None => Ok(Vec::new()),
            },
            InvariantMode::LogOnly => {
                state.violations.extend(fresh.iter().cloned());
                Ok(fresh)
            }
        }
    }

    /// Batch check without retaining: every violation `events` produces under
    /// `registry`, computed over the given slice only.
    pub fn check_stream(
        registry: &InvariantRegistry,
        events: &[AuditEvent],
    ) -> Vec<InvariantViolation> {
        run_checks(registry, events)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, AuditorState> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Fold stream-wide facts onto every event.
fn enrich(events: &[AuditEvent]) -> Vec<AuditEvent> {
    let mut identity_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut forget_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut state_change_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut protected: BTreeMap<String, i64> = BTreeMap::new();
    let mut cleaned: BTreeSet<String> = BTreeSet::new();

    for event in events {
        *identity_totals.entry(event.identity_key()).or_insert(0) += 1;
        match event.kind {
            AuditEventKind::Forget => {
                *forget_totals.entry(event.subject.clone()).or_insert(0) += 1;
            }
            AuditEventKind::StateChange => {
                if event.correlation == forget_cause(&event.subject) {
                    *state_change_totals
                        .entry(event.subject.clone())
                        .or_insert(0) += 1;
                }
            }
            AuditEventKind::Protect => {
                *protected.entry(event.subject.clone()).or_insert(0) += 1;
            }
            AuditEventKind::Unprotect => {
                *protected.entry(event.subject.clone()).or_insert(0) -= 1;
            }
            AuditEventKind::Cleanup => {
                cleaned.insert(event.subject.clone());
            }
            AuditEventKind::SideEffect => {}
        }
    }

    events
        .iter()
        .map(|event| {
            let mut enriched = event.clone();
            enriched.facts = StreamFacts {
                identity_total: identity_totals
                    .get(&event.identity_key())
                    .copied()
                    .unwrap_or(0),
                subject_forget_total: forget_totals.get(&event.subject).copied().unwrap_or(0),
                subject_forget_state_change_total: state_change_totals
                    .get(&event.subject)
                    .copied()
                    .unwrap_or(0),
                subject_protected: protected.get(&event.subject).copied().unwrap_or(0) > 0,
                subject_cleaned: cleaned.contains(&event.subject),
            };
            enriched
        })
        .collect()
}

/// Run every enabled invariant over every event, attributing each violation to
/// its registered entry and collapsing repeats of the same observation.
///
/// The result is sorted by (invariant, module, detail), so the report itself is
/// independent of the order the stream arrived in.
fn run_checks(registry: &InvariantRegistry, events: &[AuditEvent]) -> Vec<InvariantViolation> {
    let enriched = enrich(events);
    let enabled = registry.enabled();
    let mut violations: Vec<InvariantViolation> = Vec::new();
    for event in &enriched {
        for invariant in &enabled {
            if let Err(mut violation) = (invariant.check)(event) {
                // Attribution follows the registration, never the check body.
                violation.invariant = invariant.name.clone();
                violation.module = invariant.module.clone();
                if !violations.contains(&violation) {
                    violations.push(violation);
                }
            }
        }
    }
    violations.sort_by(|left, right| {
        (&left.invariant, &left.module, &left.detail).cmp(&(
            &right.invariant,
            &right.module,
            &right.detail,
        ))
    });
    violations
}

// ---------------------------------------------------------------------------
// First batch: runtime assertions mirroring the offline-proved property
// families, restricted to audit-event-observable semantics.
// ---------------------------------------------------------------------------

/// Name of the single-side-effect invariant.
pub const INV_A_NO_DOUBLE_SIDE_EFFECT: &str = "inv_a_no_double_side_effect";
/// Name of the forget-idempotence invariant.
pub const INV_FORGET_IDEMPOTENT: &str = "inv_forget_idempotent";
/// Name of the protected-memory cleanup exclusion invariant.
pub const INV_SWEEP_KEEPS_PROTECTED: &str = "inv_sweep_keeps_protected";
/// Name of the forget confinement invariant.
pub const INV_FORGET_CONFINED_TO_TARGET: &str = "inv_forget_confined_to_target";

/// `inv_a_no_double_side_effect`: one approval produces exactly one side
/// effect. Side-effect events sharing one request id and one operation must not
/// repeat — a repeat is a second execution of an already-executed effect.
fn check_no_double_side_effect(event: &AuditEvent) -> Result<(), InvariantViolation> {
    if event.kind == AuditEventKind::SideEffect && event.facts.identity_total > 1 {
        return Err(InvariantViolation {
            invariant: INV_A_NO_DOUBLE_SIDE_EFFECT.to_string(),
            module: "approval".to_string(),
            detail: format!(
                "request {} produced {} side-effect events for operation {} (at most one allowed)",
                event.correlation, event.facts.identity_total, event.subject
            ),
        });
    }
    Ok(())
}

/// `inv_forget_idempotent`: repeated forget is idempotent. Once a subject has
/// seen more than one forget event, no new state-change record may be
/// attributable to a forget of that subject: the repeat of a forget changes
/// nothing.
fn check_forget_idempotent(event: &AuditEvent) -> Result<(), InvariantViolation> {
    let forget_attributable = event.kind == AuditEventKind::StateChange
        && event.correlation == forget_cause(&event.subject);
    if forget_attributable
        && event.facts.subject_forget_total >= 2
        && event.facts.subject_forget_state_change_total > 1
    {
        return Err(InvariantViolation {
            invariant: INV_FORGET_IDEMPOTENT.to_string(),
            module: "memory_governance".to_string(),
            detail: format!(
                "subject {} saw {} forget events but carries {} forget-attributable \
                 state-change records (a repeated forget must not record new state changes)",
                event.subject,
                event.facts.subject_forget_total,
                event.facts.subject_forget_state_change_total
            ),
        });
    }
    Ok(())
}

/// `inv_sweep_keeps_protected`: protected memory never enters the cleanup
/// stream. A protect marker and a cleanup event on one subject are mutually
/// exclusive; whichever event kind of the pair is being checked, the pair
/// itself is the violation.
fn check_sweep_keeps_protected(event: &AuditEvent) -> Result<(), InvariantViolation> {
    let in_scope = matches!(
        event.kind,
        AuditEventKind::Protect | AuditEventKind::Cleanup
    );
    if in_scope && event.facts.subject_protected && event.facts.subject_cleaned {
        return Err(InvariantViolation {
            invariant: INV_SWEEP_KEEPS_PROTECTED.to_string(),
            module: "retention".to_string(),
            detail: format!(
                "subject {} carries a protect marker and a cleanup event at the same time \
                 (protected memory must not enter the cleanup stream)",
                event.subject
            ),
        });
    }
    Ok(())
}

/// `inv_forget_confined_to_target`: a forget only touches its target subject. A
/// state-change record attributable to a forget must carry the target as its
/// own subject.
fn check_forget_confined_to_target(event: &AuditEvent) -> Result<(), InvariantViolation> {
    const CAUSE_PREFIX: &str = "forget:";
    if event.kind == AuditEventKind::StateChange {
        if let Some(target) = event.correlation.strip_prefix(CAUSE_PREFIX) {
            if target != event.subject {
                return Err(InvariantViolation {
                    invariant: INV_FORGET_CONFINED_TO_TARGET.to_string(),
                    module: "memory_governance".to_string(),
                    detail: format!(
                        "forget of {target} recorded a state change for non-target subject {}",
                        event.subject
                    ),
                });
            }
        }
    }
    Ok(())
}

/// The first batch of runtime invariants, ready to register.
pub fn first_batch_invariants() -> Vec<RuntimeInvariant> {
    vec![
        RuntimeInvariant::new(
            INV_A_NO_DOUBLE_SIDE_EFFECT,
            "approval",
            check_no_double_side_effect,
        ),
        RuntimeInvariant::new(
            INV_FORGET_IDEMPOTENT,
            "memory_governance",
            check_forget_idempotent,
        ),
        RuntimeInvariant::new(
            INV_SWEEP_KEEPS_PROTECTED,
            "retention",
            check_sweep_keeps_protected,
        ),
        RuntimeInvariant::new(
            INV_FORGET_CONFINED_TO_TARGET,
            "memory_governance",
            check_forget_confined_to_target,
        ),
    ]
}

/// An auditor pre-loaded with the first batch.
pub fn first_batch_auditor(mode: InvariantMode) -> InvariantAuditor {
    let mut registry = InvariantRegistry::new();
    for invariant in first_batch_invariants() {
        let _ = registry.register(invariant);
    }
    InvariantAuditor::new(registry, mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn register_first_batch(registry: &mut InvariantRegistry) {
        for invariant in first_batch_invariants() {
            registry.register(invariant).expect("first batch registers");
        }
    }

    #[test]
    fn registry_registers_and_unregisters_invariants() {
        let mut registry = InvariantRegistry::new();
        assert!(registry.is_empty());

        register_first_batch(&mut registry);
        assert_eq!(registry.len(), 4);
        assert_eq!(
            registry.names(),
            [
                INV_A_NO_DOUBLE_SIDE_EFFECT,
                INV_FORGET_CONFINED_TO_TARGET,
                INV_FORGET_IDEMPOTENT,
                INV_SWEEP_KEEPS_PROTECTED,
            ]
            .map(str::to_string)
        );

        let duplicate = RuntimeInvariant::new(
            INV_A_NO_DOUBLE_SIDE_EFFECT,
            "approval",
            check_no_double_side_effect,
        );
        assert!(
            registry.register(duplicate).is_err(),
            "duplicates are rejected"
        );
        assert!(
            registry
                .register(RuntimeInvariant::new("", "m", check_no_double_side_effect))
                .is_err(),
            "empty names are rejected"
        );

        assert!(registry.unregister(INV_FORGET_IDEMPOTENT));
        assert!(!registry.unregister(INV_FORGET_IDEMPOTENT));
        assert!(!registry.contains(INV_FORGET_IDEMPOTENT));
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn per_name_toggle_controls_which_invariants_run() {
        let auditor = first_batch_auditor(InvariantMode::LogOnly);
        let duplicate = vec![
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "first"),
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "second"),
        ];

        let fired = auditor.observe(&duplicate).expect("log-only never errors");
        assert_eq!(fired.len(), 1);

        assert!(auditor.set_enabled(INV_A_NO_DOUBLE_SIDE_EFFECT, false));
        assert!(!auditor.is_enabled(INV_A_NO_DOUBLE_SIDE_EFFECT));
        assert!(
            !auditor.set_enabled("not_registered", false),
            "toggling an unknown name reports false"
        );

        let quiet = first_batch_auditor(InvariantMode::LogOnly);
        assert!(quiet.set_enabled(INV_A_NO_DOUBLE_SIDE_EFFECT, false));
        let fired = quiet.observe(&duplicate).expect("log-only never errors");
        assert!(
            fired.is_empty(),
            "a disabled invariant must stay silent: {fired:?}"
        );

        assert!(quiet.set_enabled(INV_A_NO_DOUBLE_SIDE_EFFECT, true));
        let fired = quiet.observe(&[]).expect("log-only never errors");
        assert_eq!(fired.len(), 1, "re-enabling restores the guard");
    }

    #[test]
    fn duplicate_side_effect_for_one_request_is_detected_and_attributed() {
        let registry = {
            let mut registry = InvariantRegistry::new();
            register_first_batch(&mut registry);
            registry
        };

        let stream = vec![
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "first"),
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "second"),
        ];
        let violations = InvariantAuditor::check_stream(&registry, &stream);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert_eq!(violations[0].invariant, INV_A_NO_DOUBLE_SIDE_EFFECT);
        assert_eq!(violations[0].module, "approval");
        assert!(
            violations[0].detail.contains("req-1"),
            "{:?}",
            violations[0]
        );

        // One request may legitimately run several *different* operations.
        let healthy = vec![
            AuditEvent::side_effect("dispatch", "req-2", "call-1", "first"),
            AuditEvent::side_effect("dispatch", "req-2", "call-2", "second"),
        ];
        assert!(InvariantAuditor::check_stream(&registry, &healthy).is_empty());
    }

    #[test]
    fn repeated_forget_with_new_state_changes_is_detected() {
        let registry = {
            let mut registry = InvariantRegistry::new();
            register_first_batch(&mut registry);
            registry
        };

        let offending = vec![
            AuditEvent::forget("memory", "ep-1", "first forget"),
            AuditEvent::forget("memory", "ep-1", "repeat forget"),
            AuditEvent::state_change_for_forget("memory", "ep-1", "ep-1", "change one"),
            AuditEvent::state_change_for_forget("memory", "ep-1", "ep-1", "change two"),
        ];
        let violations = InvariantAuditor::check_stream(&registry, &offending);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert_eq!(violations[0].invariant, INV_FORGET_IDEMPOTENT);
        assert_eq!(violations[0].module, "memory_governance");
        assert!(violations[0].detail.contains("ep-1"));

        // A repeat forget that records no new state change is conforming.
        let idempotent = vec![
            AuditEvent::forget("memory", "ep-1", "first forget"),
            AuditEvent::state_change_for_forget("memory", "ep-1", "ep-1", "one change"),
            AuditEvent::forget("memory", "ep-1", "repeat forget"),
        ];
        assert!(InvariantAuditor::check_stream(&registry, &idempotent).is_empty());
    }

    #[test]
    fn protect_marker_and_cleanup_events_are_mutually_exclusive() {
        let registry = {
            let mut registry = InvariantRegistry::new();
            register_first_batch(&mut registry);
            registry
        };

        for stream in [
            vec![
                AuditEvent::protect("memory", "ep-1", "protect"),
                AuditEvent::cleanup("retention", "ep-1", "sweep"),
            ],
            vec![
                AuditEvent::cleanup("retention", "ep-1", "sweep"),
                AuditEvent::protect("memory", "ep-1", "protect"),
            ],
        ] {
            let violations = InvariantAuditor::check_stream(&registry, &stream);
            assert_eq!(violations.len(), 1, "{violations:?}");
            assert_eq!(violations[0].invariant, INV_SWEEP_KEEPS_PROTECTED);
            assert_eq!(violations[0].module, "retention");
            assert!(violations[0].detail.contains("ep-1"));
        }

        let disjoint = vec![
            AuditEvent::protect("memory", "ep-1", "protect"),
            AuditEvent::cleanup("retention", "ep-2", "sweep"),
        ];
        assert!(InvariantAuditor::check_stream(&registry, &disjoint).is_empty());

        // Clearing the marker before the cleanup pass is conforming: the
        // subject is no longer protected when the sweep touches it.
        let cleared = vec![
            AuditEvent::protect("memory", "ep-1", "protect"),
            AuditEvent::unprotect("memory", "ep-1", "unprotect"),
            AuditEvent::cleanup("retention", "ep-1", "sweep"),
        ];
        assert!(InvariantAuditor::check_stream(&registry, &cleared).is_empty());
    }

    #[test]
    fn forget_state_changes_outside_the_target_subject_are_detected() {
        let registry = {
            let mut registry = InvariantRegistry::new();
            register_first_batch(&mut registry);
            registry
        };

        let escaping = vec![AuditEvent::state_change_for_forget(
            "memory",
            "ep-1",
            "ep-2",
            "derived write",
        )];
        let violations = InvariantAuditor::check_stream(&registry, &escaping);
        assert_eq!(violations.len(), 1, "{violations:?}");
        assert_eq!(violations[0].invariant, INV_FORGET_CONFINED_TO_TARGET);
        assert!(violations[0].detail.contains("ep-1") && violations[0].detail.contains("ep-2"));

        let confined = vec![AuditEvent::state_change_for_forget(
            "memory",
            "ep-1",
            "ep-1",
            "target write",
        )];
        assert!(InvariantAuditor::check_stream(&registry, &confined).is_empty());
    }

    #[test]
    fn fail_fast_raises_while_log_only_records() {
        let duplicate = vec![
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "first"),
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "second"),
        ];

        let strict = first_batch_auditor(InvariantMode::FailFast);
        let err = strict
            .observe(&duplicate)
            .expect_err("fail-fast must raise");
        assert_eq!(err.invariant, INV_A_NO_DOUBLE_SIDE_EFFECT);
        assert!(
            strict.recorded().is_empty(),
            "fail-fast hands the violation to the call site instead of recording it"
        );

        let lenient = first_batch_auditor(InvariantMode::LogOnly);
        let recorded = lenient.observe(&duplicate).expect("log-only never errors");
        assert_eq!(recorded.len(), 1);
        assert_eq!(lenient.recorded().len(), 1, "log-only records for audit");

        // Re-observing the same stream reports nothing new.
        let again = lenient.observe(&[]).expect("log-only never errors");
        assert!(again.is_empty(), "{again:?}");
        assert_eq!(lenient.recorded().len(), 1);
    }

    #[test]
    fn the_violation_set_is_independent_of_stream_order() {
        let registry = {
            let mut registry = InvariantRegistry::new();
            register_first_batch(&mut registry);
            registry
        };

        let stream = vec![
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "a"),
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "b"),
            AuditEvent::forget("memory", "ep-1", "f1"),
            AuditEvent::forget("memory", "ep-1", "f2"),
            AuditEvent::state_change_for_forget("memory", "ep-1", "ep-1", "s1"),
            AuditEvent::state_change_for_forget("memory", "ep-1", "ep-1", "s2"),
            AuditEvent::protect("memory", "ep-2", "p"),
            AuditEvent::cleanup("retention", "ep-2", "c"),
        ];

        let reference = InvariantAuditor::check_stream(&registry, &stream);
        assert_eq!(reference.len(), 3, "{reference:?}");

        for rotation in 1..stream.len() {
            let mut rotated = stream.clone();
            rotated.rotate_left(rotation);
            let verdict = InvariantAuditor::check_stream(&registry, &rotated);
            assert_eq!(
                verdict, reference,
                "rotation by {rotation} must not change the violation set"
            );
        }

        let mut reversed = stream.clone();
        reversed.reverse();
        assert_eq!(
            InvariantAuditor::check_stream(&registry, &reversed),
            reference
        );
    }

    #[test]
    fn a_normal_stream_produces_zero_violations() {
        let auditor = first_batch_auditor(InvariantMode::LogOnly);
        let healthy = vec![
            // Distinct operations under one request are ordinary tool chains.
            AuditEvent::side_effect("dispatch", "req-1", "call-1", "read"),
            AuditEvent::side_effect("dispatch", "req-1", "call-2", "write"),
            AuditEvent::side_effect("dispatch", "req-2", "call-3", "read"),
            // One forget, one state change, confined to the target.
            AuditEvent::forget("memory", "ep-1", "forget"),
            AuditEvent::state_change_for_forget("memory", "ep-1", "ep-1", "change"),
            // Protected subject never enters the cleanup stream.
            AuditEvent::protect("memory", "ep-2", "protect"),
            // An unprotected subject is cleaned normally.
            AuditEvent::cleanup("retention", "ep-3", "sweep"),
        ];

        let fired = auditor.observe(&healthy).expect("log-only never errors");
        assert!(
            fired.is_empty(),
            "a healthy stream must be silent: {fired:?}"
        );
        assert!(auditor.recorded().is_empty());
    }
}
