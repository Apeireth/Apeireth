//! Council adapters, compatibility exports, and the opt-in quorum gate.
//!
//! The typed Council implementation lives at the crate root so the runtime
//! and legacy callers share one aggregation contract. This module exposes the
//! real LLM advisor slots and keeps the historical `council::Council` path.
//!
//! # Quorum gate (SG-24 → Council, W3.1)
//!
//! [`QuorumGate`] wraps the [`crate::quorum`] accumulator and answers one
//! question before a deliberation starts: is there enough evidence to convene
//! the Council right now? It never answers what the Council should conclude.
//!
//! - **Default off.** [`QuorumGate::default`] is disabled and
//!   [`QuorumGate::admit`] returns [`GateAdmission::Open`] without reading or
//!   mutating the accumulator. Callers that leave the gate at its default see
//!   the same Council results and zero quorum side effects.
//! - **Existing entry points unchanged.** [`Council::decide`] and
//!   [`Council::decide_with_invoker`] keep their signatures and behaviour.
//!   Gated callers opt in through [`Council::decide_gated`] and
//!   [`Council::decide_with_invoker_gated`], which consult the gate *before*
//!   any advisor is invoked and take `now_ms` as a caller-injected `u64`
//!   milliseconds value (no clock is read here).
//! - **No fabricated verdicts.** `Hold` / `Cooldown` are returned as
//!   [`GatedCouncilVerdict::NotConvened`] / [`GatedCouncilResult::NotConvened`]
//!   carrying the exact [`QuorumDecision`]; no advisor runs and no
//!   `CouncilVerdict` / `CouncilResult` is synthesized. `CouncilDecision` is
//!   deliberately not extended, so existing exhaustive matches keep compiling.
//! - **Wiring boundary.** IMPLEMENTED at the crate API level and covered by
//!   unit tests; DEFAULT OFF; not yet assembled into `runtime-assembly`
//!   (W3.2 owns injection). The gate adds no clock, I/O, task, or dependency.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::quorum::{QuorumAccumulator, QuorumDecision, QuorumEscalation, QuorumSignal};
use crate::{CouncilInvoker, CouncilResult, CouncilVerdict, Proposal};

pub mod advisors_llm;

pub use crate::Council;
pub use advisors_llm::{default_seven_advisors, seven_system_prompts, LlmAdvisor};

/// Default overall deadline for the compatibility LLM Council path.
pub const DEFAULT_COUNCIL_TIMEOUT: Duration = Duration::from_secs(60);

/// Admission result of a [`QuorumGate`] for one prospective Council call.
///
/// Produced by [`QuorumGate::admit`]. `Denied` is the only non-convening state
/// and always carries `Hold` or `Cooldown` (never `Escalate`): `consume` only
/// returns `None` when the same deterministic check is not `Escalate`, and no
/// state can change between the two calls on one `&mut self` borrow.
#[derive(Debug, Clone, PartialEq)]
pub enum GateAdmission {
    /// The gate is disabled: convene without consulting the accumulator.
    Open,
    /// The gate is enabled, the threshold was crossed, and the escalation was
    /// consumed (signals cleared, refractory window opened) exactly once.
    Escalated(QuorumEscalation),
    /// The gate is enabled but declined: below threshold (`Hold`) or inside the
    /// post-escalation refractory window (`Cooldown`). No state was changed.
    Denied(QuorumDecision),
}

/// Opt-in quorum gate in front of a Council deliberation.
///
/// Holds a [`QuorumAccumulator`] plus an enable flag. [`Default`] is disabled,
/// which reproduces the pre-wiring behaviour exactly: [`Self::admit`] returns
/// [`GateAdmission::Open`] without reading or mutating the accumulator. Callers
/// opt in with [`Self::enabled`] (or the explicit [`Self::new`]).
///
/// All time is injected as `u64` milliseconds; the gate never reads the system
/// clock, performs I/O, spawns tasks, or adds a dependency.
#[derive(Debug, Clone)]
pub struct QuorumGate {
    enabled: bool,
    accumulator: QuorumAccumulator,
}

impl Default for QuorumGate {
    /// A disabled gate over a default accumulator: the zero-behaviour-change default.
    fn default() -> Self {
        Self::disabled()
    }
}

impl QuorumGate {
    /// An enabled gate over the given accumulator.
    #[must_use]
    pub fn enabled(accumulator: QuorumAccumulator) -> Self {
        Self {
            enabled: true,
            accumulator,
        }
    }

    /// A disabled gate over a default accumulator.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            accumulator: QuorumAccumulator::default(),
        }
    }

    /// Explicit constructor for call sites that carry the flag from configuration.
    #[must_use]
    pub fn new(accumulator: QuorumAccumulator, enabled: bool) -> Self {
        Self {
            enabled,
            accumulator,
        }
    }

    /// Whether the gate participates in admission decisions.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Read-only view of the accumulator (signals, score, cooldown).
    #[must_use]
    pub fn accumulator(&self) -> &QuorumAccumulator {
        &self.accumulator
    }

    /// Mutable view for recording signals and tuning the accumulator.
    pub fn accumulator_mut(&mut self) -> &mut QuorumAccumulator {
        &mut self.accumulator
    }

    /// Record one quorum signal at `now_ms`.
    ///
    /// Recording is intentionally independent of [`Self::is_enabled`]: evidence
    /// can be accumulated before the gate is switched on, and recording alone
    /// never convenes a Council. `now_ms` is injected by the caller.
    pub fn add_signal(&mut self, signal: QuorumSignal, now_ms: u64) {
        self.accumulator.add(signal, now_ms);
    }

    /// Decide whether one Council call may convene at `now_ms`.
    ///
    /// - disabled: [`GateAdmission::Open`], accumulator neither read nor changed;
    /// - enabled + `Escalate`: [`QuorumAccumulator::consume`] runs, clearing the
    ///   consumed signals and opening the refractory window, and the bounded
    ///   reasons/score are returned in [`GateAdmission::Escalated`];
    /// - enabled + `Hold` / `Cooldown`: [`GateAdmission::Denied`] with the exact
    ///   decision; no state change.
    ///
    /// A single wave can convene at most once: after consumption the signals are
    /// gone and (with a non-zero cooldown) the next call reports `Cooldown`.
    #[must_use]
    pub fn admit(&mut self, now_ms: u64) -> GateAdmission {
        if !self.enabled {
            return GateAdmission::Open;
        }
        match self.accumulator.consume(now_ms) {
            Some(escalation) => GateAdmission::Escalated(escalation),
            None => GateAdmission::Denied(self.accumulator.should_escalate(now_ms)),
        }
    }
}

/// Outcome of [`Council::decide_gated`] (compatibility path).
///
/// `Convened` wraps the unchanged legacy [`CouncilVerdict`]; `NotConvened`
/// means no advisor was consulted and no verdict was fabricated.
#[derive(Debug, Clone, PartialEq)]
pub enum GatedCouncilVerdict {
    /// The gate allowed the Council to convene.
    Convened {
        /// The unchanged verdict from [`Council::decide`].
        verdict: CouncilVerdict,
        /// `Some` exactly when an enabled gate consumed an escalation for this
        /// call; `None` means the gate was disabled.
        escalation: Option<QuorumEscalation>,
    },
    /// The gate declined to convene; `decision` is `Hold` or `Cooldown`.
    NotConvened {
        /// Why the Council did not convene (never `Escalate`).
        decision: QuorumDecision,
    },
}

/// Outcome of [`Council::decide_with_invoker_gated`] (runtime-backed path).
///
/// `Convened` carries the unchanged [`CouncilResult`] plus the escalation
/// snapshot (score and bounded reasons) that admitted it. `NotConvened` carries
/// the exact `Hold` / `Cooldown` decision and no result at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatedCouncilResult {
    /// The gate allowed the Council to convene.
    Convened {
        /// The unchanged result from [`Council::decide_with_invoker`].
        result: CouncilResult,
        /// `Some` exactly when an enabled gate consumed an escalation for this
        /// call; `None` means the gate was disabled.
        escalation: Option<QuorumEscalation>,
    },
    /// The gate declined to convene; `decision` is `Hold` or `Cooldown`.
    NotConvened {
        /// Why the Council did not convene (never `Escalate`).
        decision: QuorumDecision,
    },
}

impl Council {
    /// Gated compatibility path: consult `gate` before any advisor is invoked.
    ///
    /// With a disabled gate this delegates straight to the unchanged
    /// [`Self::decide`] and returns the same verdict with `escalation: None`;
    /// no quorum state is touched. With an enabled gate, `now_ms` is the
    /// caller-injected millisecond timestamp used for decay and cooldown.
    ///
    /// `Hold` / `Cooldown` do not invoke any advisor and return
    /// [`GatedCouncilVerdict::NotConvened`] carrying the exact decision. No
    /// `CouncilVerdict` is fabricated for a Council that never sat.
    pub async fn decide_gated(
        &self,
        proposal: &Proposal,
        gate: &mut QuorumGate,
        now_ms: u64,
    ) -> GatedCouncilVerdict {
        let escalation = match gate.admit(now_ms) {
            GateAdmission::Denied(decision) => {
                return GatedCouncilVerdict::NotConvened { decision };
            }
            GateAdmission::Open => None,
            GateAdmission::Escalated(escalation) => Some(escalation),
        };
        GatedCouncilVerdict::Convened {
            verdict: self.decide(proposal).await,
            escalation,
        }
    }

    /// Gated runtime-backed path: same admission rules as
    /// [`Self::decide_gated`], delegating to the unchanged
    /// [`Self::decide_with_invoker`].
    ///
    /// On `Convened` the returned [`GatedCouncilResult`] carries the full
    /// [`CouncilResult`] plus the consumed escalation's score and bounded
    /// `reasons`, so callers can log *why* this deliberation was convened.
    pub async fn decide_with_invoker_gated(
        &self,
        proposal: &Proposal,
        invoker: &dyn CouncilInvoker,
        gate: &mut QuorumGate,
        now_ms: u64,
    ) -> GatedCouncilResult {
        let escalation = match gate.admit(now_ms) {
            GateAdmission::Denied(decision) => {
                return GatedCouncilResult::NotConvened { decision };
            }
            GateAdmission::Open => None,
            GateAdmission::Escalated(escalation) => Some(escalation),
        };
        GatedCouncilResult::Convened {
            result: self.decide_with_invoker(proposal, invoker).await,
            escalation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quorum::{QuorumConfig, QuorumSignalKind};
    use crate::{
        Advisor, AdvisorDecision, AdvisorKind, AdvisorVerdict, CouncilCallError, CouncilConfig,
        CouncilDecision,
    };
    use apeireth_core::kernel::SessionId;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn proposal() -> Proposal {
        Proposal {
            id: "p-gated".into(),
            proposer: "runtime".into(),
            payload: serde_json::json!({"action": "deploy"}),
            submitted_at: 1_700_000_000,
            session_id: SessionId::new(),
        }
    }

    fn config(threshold: f32, decay_per_second: f32, cooldown_ms: u64) -> QuorumConfig {
        QuorumConfig {
            threshold,
            decay_per_second,
            cooldown_ms,
            max_accumulation: 10.0,
        }
    }

    fn enabled_gate(config: QuorumConfig) -> QuorumGate {
        QuorumGate::enabled(QuorumAccumulator::new(config))
    }

    fn risk_signal(weight: f32, at_ms: u64, source: &str) -> QuorumSignal {
        QuorumSignal::weighted(QuorumSignalKind::RiskEscalation, weight, at_ms, source)
    }

    struct CountingInvoker {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl CouncilInvoker for CountingInvoker {
        async fn invoke(
            &self,
            _advisor: Arc<dyn Advisor>,
            _proposal: &Proposal,
        ) -> Result<AdvisorVerdict, CouncilCallError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            AdvisorVerdict::new(1.0, AdvisorDecision::Allow, "gated script", Some(0.9))
                .map_err(CouncilCallError::Malformed)
        }
    }

    fn counting_invoker() -> (CountingInvoker, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            CountingInvoker {
                calls: Arc::clone(&calls),
            },
            calls,
        )
    }

    fn bounded_council() -> Council {
        Council::default_llm().with_config(CouncilConfig {
            max_advisors: 3,
            per_advisor_timeout: Duration::from_millis(100),
            overall_timeout: Duration::from_secs(1),
        })
    }

    #[test]
    fn default_gate_is_disabled_and_admission_has_no_side_effects() {
        let mut gate = QuorumGate::default();
        assert!(!gate.is_enabled(), "the default gate must stay off");
        gate.add_signal(risk_signal(1.0, 0, "risk"), 0);
        assert_eq!(gate.accumulator().score(0), 1.0);

        assert_eq!(gate.admit(0), GateAdmission::Open);
        assert_eq!(
            gate.accumulator().score(0),
            1.0,
            "a disabled gate must not consume signals"
        );
        assert_eq!(gate.accumulator().cooldown_until_ms(), None);

        assert!(!QuorumGate::disabled().is_enabled());
        assert!(QuorumGate::new(QuorumAccumulator::default(), true).is_enabled());
    }

    #[tokio::test]
    async fn disabled_gate_reproduces_ungated_results_byte_for_byte() {
        let proposal = proposal();
        let council = bounded_council();
        let (invoker, calls) = counting_invoker();
        let mut gate = QuorumGate::default();

        let base = council.decide_with_invoker(&proposal, &invoker).await;
        let gated = council
            .decide_with_invoker_gated(&proposal, &invoker, &mut gate, 42)
            .await;
        let GatedCouncilResult::Convened { result, escalation } = gated else {
            panic!("a disabled gate must never block the Council");
        };
        assert_eq!(escalation, None);
        assert_eq!(
            serde_json::to_string(&result).unwrap(),
            serde_json::to_string(&base).unwrap(),
            "disabled-gate output must be byte-identical to the ungated result"
        );
        assert_eq!(calls.load(Ordering::SeqCst), 6);

        let legacy = Council::default_allow();
        let base_verdict = legacy.decide(&proposal).await;
        let gated_verdict = legacy.decide_gated(&proposal, &mut gate, 42).await;
        assert_eq!(
            gated_verdict,
            GatedCouncilVerdict::Convened {
                verdict: base_verdict,
                escalation: None,
            }
        );
        assert_eq!(gate.accumulator().signals().len(), 0);
    }

    #[tokio::test]
    async fn escalate_convenes_with_reasons_and_consumes_one_wave() {
        let mut gate = enabled_gate(config(1.0, 0.0, 10_000));
        gate.add_signal(risk_signal(0.6, 0, "risk-a"), 0);
        gate.add_signal(risk_signal(0.6, 0, "risk-b"), 0);
        let (invoker, calls) = counting_invoker();

        let outcome = Council::default_llm()
            .decide_with_invoker_gated(&proposal(), &invoker, &mut gate, 0)
            .await;
        let GatedCouncilResult::Convened { result, escalation } = outcome else {
            panic!("score 1.2 must clear the 1.0 threshold");
        };
        let escalation = escalation.expect("an enabled gate records the consumed escalation");
        assert_eq!(escalation.triggered_at_ms, 0);
        assert_eq!(escalation.score, 1.2);
        assert_eq!(escalation.reasons.len(), 2);
        assert!(
            escalation
                .reasons
                .iter()
                .all(|reason| reason.starts_with("risk_escalation weight=0.600")),
            "reasons must carry the signal kind and weight: {:?}",
            escalation.reasons
        );
        assert_eq!(result.decision, CouncilDecision::Continue);
        assert_eq!(calls.load(Ordering::SeqCst), 7);
        assert_eq!(gate.accumulator().cooldown_until_ms(), Some(10_000));
        assert!(gate.accumulator().signals().is_empty());
    }

    #[tokio::test]
    async fn legacy_decide_gated_convenes_and_carries_reasons() {
        let mut gate = enabled_gate(config(1.0, 0.0, 5_000));
        gate.add_signal(risk_signal(1.0, 0, "risk"), 0);
        let outcome = Council::default_allow()
            .decide_gated(&proposal(), &mut gate, 0)
            .await;
        let GatedCouncilVerdict::Convened {
            verdict,
            escalation,
        } = outcome
        else {
            panic!("a clearing wave must convene");
        };
        assert_eq!(verdict, CouncilVerdict::Approved);
        let escalation = escalation.expect("escalation is carried on the legacy path");
        assert_eq!(escalation.reasons.len(), 1);
        assert!(escalation.reasons[0].contains("source=risk"));
    }

    #[tokio::test]
    async fn hold_does_not_convene_and_calls_no_advisor() {
        let mut gate = enabled_gate(config(1.0, 0.0, 10_000));
        gate.add_signal(risk_signal(0.5, 0, "risk"), 0);
        let (invoker, calls) = counting_invoker();

        let outcome = Council::default_llm()
            .decide_with_invoker_gated(&proposal(), &invoker, &mut gate, 0)
            .await;
        match outcome {
            GatedCouncilResult::NotConvened { decision } => match decision {
                QuorumDecision::Hold { score, need } => {
                    assert_eq!(score, 0.5);
                    assert_eq!(need, 0.5);
                }
                other => panic!("expected Hold, got {other:?}"),
            },
            other @ GatedCouncilResult::Convened { .. } => {
                panic!("below-threshold wave must not convene: {other:?}")
            }
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "no advisor may be invoked on Hold"
        );
        assert_eq!(
            gate.accumulator().signals().len(),
            1,
            "Hold must not consume signals"
        );
    }

    #[tokio::test]
    async fn cooldown_after_a_consumed_wave_blocks_the_next_one() {
        let mut gate = enabled_gate(config(1.0, 0.0, 10_000));
        gate.add_signal(risk_signal(1.0, 0, "first-wave"), 0);
        let (invoker, calls) = counting_invoker();
        let council = Council::default_llm();

        let first = council
            .decide_with_invoker_gated(&proposal(), &invoker, &mut gate, 0)
            .await;
        assert!(matches!(first, GatedCouncilResult::Convened { .. }));
        assert_eq!(calls.load(Ordering::SeqCst), 7);

        gate.add_signal(risk_signal(1.0, 1_000, "second-wave"), 1_000);
        let second = council
            .decide_with_invoker_gated(&proposal(), &invoker, &mut gate, 1_000)
            .await;
        match second {
            GatedCouncilResult::NotConvened {
                decision: QuorumDecision::Cooldown { until_ms, score },
            } => {
                assert_eq!(until_ms, 10_000);
                assert_eq!(score, 1.0);
            }
            other => panic!("expected Cooldown inside the refractory window, got {other:?}"),
        }
        assert_eq!(
            calls.load(Ordering::SeqCst),
            7,
            "cooldown must not invoke advisors again"
        );
    }

    #[tokio::test]
    async fn the_same_wave_can_convene_at_most_once() {
        let (invoker, calls) = counting_invoker();
        let council = Council::default_llm();

        let mut gated = enabled_gate(config(1.0, 0.0, 10_000));
        gated.add_signal(risk_signal(1.0, 0, "wave"), 0);
        let first = council
            .decide_with_invoker_gated(&proposal(), &invoker, &mut gated, 0)
            .await;
        assert!(matches!(first, GatedCouncilResult::Convened { .. }));
        let second = council
            .decide_with_invoker_gated(&proposal(), &invoker, &mut gated, 0)
            .await;
        assert!(matches!(
            second,
            GatedCouncilResult::NotConvened {
                decision: QuorumDecision::Cooldown { .. }
            }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 7);

        // Without a refractory window the consumed signals are simply gone, so
        // the second call is a Hold rather than a second escalation.
        let mut no_cooldown = enabled_gate(config(1.0, 0.0, 0));
        no_cooldown.add_signal(risk_signal(1.0, 0, "wave"), 0);
        assert!(matches!(
            council
                .decide_with_invoker_gated(&proposal(), &invoker, &mut no_cooldown, 0)
                .await,
            GatedCouncilResult::Convened { .. }
        ));
        assert!(matches!(
            council
                .decide_with_invoker_gated(&proposal(), &invoker, &mut no_cooldown, 0)
                .await,
            GatedCouncilResult::NotConvened {
                decision: QuorumDecision::Hold { .. }
            }
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 14);
    }

    #[tokio::test]
    async fn threshold_boundary_is_inclusive() {
        let council = Council::default_allow();

        let mut exact = enabled_gate(config(1.0, 0.0, 0));
        exact.add_signal(risk_signal(0.5, 0, "a"), 0);
        exact.add_signal(risk_signal(0.5, 0, "b"), 0);
        match council.decide_gated(&proposal(), &mut exact, 0).await {
            GatedCouncilVerdict::Convened {
                escalation: Some(escalation),
                ..
            } => assert_eq!(escalation.score, 1.0),
            other => panic!("score equal to the threshold must convene, got {other:?}"),
        }

        let mut below = enabled_gate(config(1.0, 0.0, 0));
        below.add_signal(risk_signal(0.5, 0, "a"), 0);
        below.add_signal(risk_signal(0.25, 0, "b"), 0);
        match council.decide_gated(&proposal(), &mut below, 0).await {
            GatedCouncilVerdict::NotConvened {
                decision: QuorumDecision::Hold { score, need },
            } => {
                assert_eq!(score, 0.75);
                assert_eq!(need, 0.25);
            }
            other => panic!("score below the threshold must hold, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn now_ms_injection_drives_decay_decisions() {
        // One full-weight signal at t=0; 0.5/s decay, threshold 1.0.
        let mut gate = enabled_gate(config(1.0, 0.5, 0));
        gate.add_signal(risk_signal(1.0, 0, "sensor"), 0);
        let council = Council::default_allow();

        // One second later the evidence has decayed below the threshold.
        match council.decide_gated(&proposal(), &mut gate, 1_000).await {
            GatedCouncilVerdict::NotConvened {
                decision: QuorumDecision::Hold { score, .. },
            } => assert_eq!(score, 0.5),
            other => panic!("decayed score must hold, got {other:?}"),
        }

        // The same accumulator evaluated at t=0 still convenes: admission is a
        // pure function of the injected now_ms, never of the wall clock.
        match council.decide_gated(&proposal(), &mut gate, 0).await {
            GatedCouncilVerdict::Convened {
                escalation: Some(escalation),
                ..
            } => {
                assert_eq!(escalation.triggered_at_ms, 0);
                assert_eq!(escalation.score, 1.0);
            }
            other => panic!("time-reversed now_ms must still convene, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn gated_decisions_are_deterministic_across_identical_runs() {
        async fn run() -> (String, usize) {
            let council = Council::default_llm();
            let (invoker, calls) = counting_invoker();
            let mut gate = enabled_gate(config(0.9, 0.05, 2_000));
            gate.add_signal(risk_signal(0.6, 0, "risk"), 0);
            gate.add_signal(risk_signal(0.5, 0, "distrust"), 0);
            let outcome = council
                .decide_with_invoker_gated(&proposal(), &invoker, &mut gate, 100)
                .await;
            (
                serde_json::to_string(&outcome).unwrap(),
                calls.load(Ordering::SeqCst),
            )
        }

        let first = run().await;
        let second = run().await;
        assert_eq!(first, second);
        assert_eq!(first.1, 7);
    }
}
