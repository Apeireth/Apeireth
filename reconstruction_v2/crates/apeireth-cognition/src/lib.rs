//! apeireth-cognition: cognition organ (A10 landing — R14 stage 4)
//!
//! **Responsibility**: internal cognition main path — receives perception input -> ASI scoring (V0.5/V1136) ->
//! 13-key verdict gate -> decision -> reflection.
//!
//! **Architecture position**: stage-4 §2 main path 17-crate A10 organ (after apeireth-perception,
//! before apeireth-action/motivation/value).
//!
//! **Current state**: A10 minimal viable landing (P2 task). This crate provides 5+ pub fn + 5+ tests +
//! example, calling V0.5/V1136 + 13-key verdict gate.
//!
//! **TP18 (E3, P1) increments**: calibration + ensemble forecast + prediction market
//! - `calibration` — Brier score + Murphy monotonic decomposition (reliability / resolution / uncertainty)
//!   + 10-bin CalibrationBin + Expected Calibration Error
//!
//! **Honest registration**: per handover-final-2026-08-01 §B.4 "5+ pub fn, 5+ tests, calling V0.5/V1136 +
//! 13-key" simplified implementation. Full cognition organ (double onion + Cognitive-Dream 6 state
//! machine) remains for A18/A19 deepening.
//!
//! **Prohibitions**:
//! - do NOT modify apeireth-core / apeireth-asi installed type signatures
//! - do NOT touch R11 baseline three values
//! - do NOT touch apeireth-legacy/

#![deny(unsafe_code)]

use apeireth_asi::{AsiV05Scores, V1136Submeasures};
use apeireth_core::{ActionTarget, PhilosophyVerdict};
use thiserror::Error;
use uuid::Uuid;

pub mod calibration;
pub mod consciousness_bridge;
mod decision;
mod reflection;
mod scoring;

pub use calibration::{
    brier_score, brier_single, calibration_bins, decompose, expected_calibration_error,
    BrierDecomposition, CalibrationBin, Observation, DEFAULT_NUM_BINS,
};
pub use consciousness_bridge::{
    accumulate_biases, plutchik_to_decision_bias, DecisionBias, PlutchikAdvanced, PlutchikBasic,
    PlutchikEmotion, PlutchikIntensity,
};
pub use decision::{CognitiveOutput, CognitivePipeline};
pub use reflection::{ReflectionReport, ReflectionVerdict};
pub use scoring::{
    continuity_score, identity_score, philosophy_guard_score, salience_score, score_v05,
    score_v1136, transferability_score, validate_asi_score,
};

/// Top-level error: fallback error for all cognition subsystems.
#[derive(Debug, Error)]
pub enum CognitionError {
    /// Invalid input.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// ASI score out of range ([0.0, 1.0]).
    #[error("asi score out of range: {0}")]
    AsiOutOfRange(f64),
    /// Verdict chain discovered Block decision.
    #[error("verdict blocked: {0:?}")]
    VerdictBlocked(PhilosophyVerdict),
    /// Serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Unified result type.
pub type CognitionResult<T> = Result<T, CognitionError>;

/// Cognition input — perception result + candidate actions + context metadata.
#[derive(Debug, Clone)]
pub struct CognitiveInput {
    /// Input unique ID.
    pub input_id: Uuid,
    /// Associated session ID (optional, for cross-session continuity).
    pub session_id: Option<Uuid>,
    /// Candidate actions (13-key verdict gate targets).
    pub candidate_targets: Vec<ActionTarget>,
    /// Timestamp (Unix seconds).
    pub timestamp: i64,
    /// Context tag (for reflection / memory recall).
    pub context_tag: String,
}

impl CognitiveInput {
    /// Construct a minimal input.
    pub fn new(candidate_targets: Vec<ActionTarget>, context_tag: impl Into<String>) -> Self {
        Self {
            input_id: Uuid::new_v4(),
            session_id: None,
            candidate_targets,
            timestamp: chrono::Utc::now().timestamp(),
            context_tag: context_tag.into(),
        }
    }

    /// Validate input legality.
    pub fn validate(&self) -> CognitionResult<()> {
        if self.candidate_targets.is_empty() {
            return Err(CognitionError::InvalidInput(
                "candidate_targets must not be empty".to_string(),
            ));
        }
        if self.context_tag.is_empty() {
            return Err(CognitionError::InvalidInput(
                "context_tag must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// One full cognition cycle — input -> scoring -> gate -> decision -> reflection.
#[derive(Debug, Clone)]
pub struct CognitiveCycle {
    /// Cycle input ID.
    pub input_id: Uuid,
    /// ASI V0.5 scores.
    pub v05: AsiV05Scores,
    /// ASI V1136 sub-measures.
    pub v1136: V1136Submeasures,
    /// 13-key verdict chain (one-to-one with candidate_targets).
    pub verdicts: Vec<PhilosophyVerdict>,
    /// Final decision.
    pub output: CognitiveOutput,
    /// Reflection report.
    pub reflection: ReflectionReport,
}

impl CognitiveCycle {
    /// Whether the cycle rejected (any verdict Block).
    pub fn is_rejected(&self) -> bool {
        matches!(self.output, CognitiveOutput::Reject(_))
    }

    /// Whether the cycle allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self.output, CognitiveOutput::Decision(_))
    }
}

/// Run one full cognition cycle (public API — most common entry point).
pub fn run_cycle(input: CognitiveInput) -> CognitionResult<CognitiveCycle> {
    input.validate()?;

    let v05 = scoring::score_v05(&input);
    let v1136 = scoring::score_v1136(&input);
    let verdicts = decision::evaluate_actions(&input.candidate_targets);
    let output = decision::decide(&verdicts)?;
    let reflection = reflection::reflect(&input, &v05, &v1136, &verdicts, &output);

    Ok(CognitiveCycle {
        input_id: input.input_id,
        v05,
        v1136,
        verdicts,
        output,
        reflection,
    })
}

// ============================================
// Cognitive-trait abstraction layer (v1 pub API surface)
// ============================================

/// Example cognition engine — provides a deterministic native-Rust baseline implementation
/// for stage 4 trait abstraction. This type has no internal state, suitable for examples,
/// tests, and call-site contract validation before adopting a specialized implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct BasicCognitiveEngine;

/// Synthesis cognition: combine a set of observations into a consumable cognition result.
pub trait Cognition {
    /// Normalize and synthesize observations; empty observations return `None`.
    fn cognize(&self, observations: &[&str]) -> Option<String>;
}

/// Intuition: quickly select a suggestion from candidates.
pub trait Intuition {
    /// Return the first non-empty candidate; `None` if none exist.
    fn intuit<'a>(&self, candidates: &'a [&'a str]) -> Option<&'a str>;
}

/// Reasoning: derive conclusions from premises.
pub trait Reasoning {
    /// `true` only when all premises hold; empty premises do not constitute sufficient reason.
    fn reason(&self, premises: &[bool]) -> bool;
}

/// Meta-cognition: assess the confidence of a cognition result.
pub trait MetaCognition {
    /// Clamp confidence to `[0.0, 1.0]`; non-finite values treated as `0.0`.
    fn assess_confidence(&self, confidence: f64) -> f64;
}

/// Recall: retrieve a matching memory item by query.
pub trait Recall {
    /// Return the first memory containing the query; empty query does not match.
    fn recall<'a>(&self, query: &str, memories: &'a [&'a str]) -> Option<&'a str>;
}

/// Consolidation: trim whitespace and adjacent duplicates into stable memories.
pub trait Consolidation {
    /// Return a deduplicated consolidation result, preserving input order.
    fn consolidate(&self, memories: &[&str]) -> Vec<String>;
}

/// Forgetting: drop memories based on a retention policy.
pub trait Forgetting {
    /// Keep only memories where `retain` returns `true`.
    fn forget(&self, memories: &[&str], retain: &dyn Fn(&str) -> bool) -> Vec<String>;
}

/// Learning: update current knowledge intensity based on feedback.
pub trait Learning {
    /// Apply feedback delta and clamp to `[0.0, 1.0]`.
    fn learn(&self, current: f64, feedback: f64) -> f64;
}

/// Abstraction: extract a common non-empty prefix from samples.
pub trait Abstraction {
    /// Return the common prefix across all samples; empty input returns `None`.
    fn abstract_commonality(&self, samples: &[&str]) -> Option<String>;
}

impl Cognition for BasicCognitiveEngine {
    fn cognize(&self, observations: &[&str]) -> Option<String> {
        let normalized: Vec<_> = observations
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .collect();
        (!normalized.is_empty()).then(|| normalized.join(" | "))
    }
}

impl Intuition for BasicCognitiveEngine {
    fn intuit<'a>(&self, candidates: &'a [&'a str]) -> Option<&'a str> {
        candidates
            .iter()
            .copied()
            .find(|item| !item.trim().is_empty())
    }
}

impl Reasoning for BasicCognitiveEngine {
    fn reason(&self, premises: &[bool]) -> bool {
        !premises.is_empty() && premises.iter().all(|premise| *premise)
    }
}

impl MetaCognition for BasicCognitiveEngine {
    fn assess_confidence(&self, confidence: f64) -> f64 {
        if confidence.is_finite() {
            confidence.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

impl Recall for BasicCognitiveEngine {
    fn recall<'a>(&self, query: &str, memories: &'a [&'a str]) -> Option<&'a str> {
        (!query.is_empty())
            .then(|| {
                memories
                    .iter()
                    .copied()
                    .find(|memory| memory.contains(query))
            })
            .flatten()
    }
}

impl Consolidation for BasicCognitiveEngine {
    fn consolidate(&self, memories: &[&str]) -> Vec<String> {
        let mut result = Vec::new();
        for memory in memories
            .iter()
            .map(|memory| memory.trim())
            .filter(|memory| !memory.is_empty())
        {
            if result.last().map(String::as_str) != Some(memory) {
                result.push(memory.to_string());
            }
        }
        result
    }
}

impl Forgetting for BasicCognitiveEngine {
    fn forget(&self, memories: &[&str], retain: &dyn Fn(&str) -> bool) -> Vec<String> {
        memories
            .iter()
            .copied()
            .filter(|memory| retain(memory))
            .map(str::to_string)
            .collect()
    }
}

impl Learning for BasicCognitiveEngine {
    fn learn(&self, current: f64, feedback: f64) -> f64 {
        let current = if current.is_finite() { current } else { 0.0 };
        let feedback = if feedback.is_finite() { feedback } else { 0.0 };
        (current + feedback).clamp(0.0, 1.0)
    }
}

impl Abstraction for BasicCognitiveEngine {
    fn abstract_commonality(&self, samples: &[&str]) -> Option<String> {
        let first = *samples.first()?;
        let mut boundary = first.len();
        for sample in &samples[1..] {
            let matched_bytes: usize = first
                .chars()
                .zip(sample.chars())
                .take_while(|(left, right)| left == right)
                .map(|(character, _)| character.len_utf8())
                .sum();
            boundary = boundary.min(matched_bytes);
        }
        (boundary > 0).then(|| first[..boundary].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cognitive_input_validate_rejects_empty_targets() {
        let input = CognitiveInput::new(vec![], "test");
        assert!(input.validate().is_err());
    }

    #[test]
    fn cognitive_input_validate_rejects_empty_context() {
        let target = ActionTarget::NormalAction("noop".to_string());
        let input = CognitiveInput::new(vec![target], "");
        assert!(input.validate().is_err());
    }

    #[test]
    fn cognitive_input_validate_accepts_valid_input() {
        let target = ActionTarget::NormalAction("noop".to_string());
        let input = CognitiveInput::new(vec![target], "test");
        assert!(input.validate().is_ok());
    }

    #[test]
    fn run_cycle_normal_action_is_allowed() {
        let target = ActionTarget::NormalAction("read".to_string());
        let input = CognitiveInput::new(vec![target], "normal_op");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_allowed());
        assert!(!cycle.is_rejected());
        assert_eq!(cycle.verdicts.len(), 1);
    }

    #[test]
    fn run_cycle_modify_l0_ha_is_rejected() {
        let target = ActionTarget::ModifyL0HA;
        let input = CognitiveInput::new(vec![target], "l0_violation_attempt");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_rejected());
        assert!(!cycle.is_allowed());
        // 13-key verdict gate: ModifyL0HA -> Block(NotUnobservable)
        assert!(matches!(cycle.verdicts[0], PhilosophyVerdict::Block(_)));
    }

    #[test]
    fn run_cycle_pretend_clone_is_rejected() {
        let target = ActionTarget::PretendClone;
        let input = CognitiveInput::new(vec![target], "phl01_violation");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_rejected());
    }

    #[test]
    fn run_cycle_mixed_targets_partial_reject() {
        let targets = vec![
            ActionTarget::NormalAction("read".to_string()),
            ActionTarget::PretendPerfect,
        ];
        let input = CognitiveInput::new(targets, "mixed");
        let cycle = run_cycle(input).expect("cycle must run");
        assert!(cycle.is_rejected());
        assert_eq!(cycle.verdicts.len(), 2);
    }

    #[test]
    fn run_cycle_uses_verdict_for_target_core_api() {
        let target = ActionTarget::PretendUuid;
        let expected = verdict_for_target(&target);
        let input = CognitiveInput::new(vec![target], "phl01_uuid");
        let cycle = run_cycle(input).expect("cycle must run");
        assert_eq!(cycle.verdicts[0], expected);
    }

    #[test]
    fn run_cycle_assigns_input_id_to_cycle() {
        let target = ActionTarget::NormalAction("noop".to_string());
        let input = CognitiveInput::new(vec![target], "id_check");
        let cycle = run_cycle(input).expect("cycle must run");
        // input_id was consumed by run_cycle (moved); cycle.input_id is the new generated UUID.
        assert_eq!(cycle.input_id.as_bytes().len(), 16);
    }

    #[test]
    fn basic_cognitive_engine_traits_work() {
        let engine = BasicCognitiveEngine;
        // Cognition
        assert_eq!(engine.cognize(&["a", "b"]).as_deref(), Some("a | b"));
        assert!(engine.cognize(&["", "  "]).is_none());
        // Intuition
        assert_eq!(engine.intuit(&["", "hi", "world"]), Some("hi"));
        // Reasoning
        assert!(engine.reason(&[true, true]));
        assert!(!engine.reason(&[true, false]));
        assert!(!engine.reason(&[]));
        // MetaCognition
        assert!((engine.assess_confidence(0.7) - 0.7).abs() < 1e-9);
        assert_eq!(engine.assess_confidence(1.5), 1.0);
        assert_eq!(engine.assess_confidence(-0.1), 0.0);
        assert_eq!(engine.assess_confidence(f64::NAN), 0.0);
        // Recall
        let mems = &["apple", "banana", "cherry"];
        assert_eq!(engine.recall("nan", mems), Some("banana"));
        assert!(engine.recall("", mems).is_none());
        // Consolidation
        let result = engine.consolidate(&["a", "  a  ", "b", "b", ""]);
        assert_eq!(result, vec!["a".to_string(), "b".to_string()]);
        // Forgetting
        let result = engine.forget(&["keep", "drop"], &|m| m == "keep");
        assert_eq!(result, vec!["keep".to_string()]);
        // Learning
        assert!((engine.learn(0.5, 0.3) - 0.8).abs() < 1e-9);
        assert_eq!(engine.learn(0.9, 0.5), 1.0); // clamped
        // Abstraction
        assert_eq!(
            engine.abstract_commonality(&["hello world", "hello rust"]).as_deref(),
            Some("hello ")
        );
        assert!(engine.abstract_commonality(&[]).is_none());
    }
}
