//! ASI scoring (V0.5 24-dim + V1136 9 sub-measures).
//!
//! This module provides 5+ pub fn for scoring cognition organ inputs:
//! - `score_v05` — 5-dim scoring main entry
//! - `score_v1136` — 7 sub-measure main entry
//! - 5 sub-dimension scoring functions (continuity/salience/identity/philosophy_guard/transferability)

use apeireth_asi::{AsiV05Scores, V1136Submeasures};
use chrono::Utc;

use crate::{CognitionError, CognitionResult, CognitiveInput};

/// Score a cognitive input with ASI V0.5 24-dim scoring.
pub fn score_v05(input: &CognitiveInput) -> AsiV05Scores {
    AsiV05Scores {
        continuity: continuity_score(input),
        salience: salience_score(input),
        identity: identity_score(input),
        philosophy_guard: philosophy_guard_score(input),
        transferability: transferability_score(input),
    }
}

/// Score a cognitive input with ASI V1136 9 sub-measure scoring.
pub fn score_v1136(input: &CognitiveInput) -> V1136Submeasures {
    // 5 continuity + 2 transferability - simplified: each dim derived from V0.5.
    let v05 = score_v05(input);
    V1136Submeasures {
        continuity_5: [
            v05.continuity,
            v05.identity,
            v05.salience * 0.5,
            v05.philosophy_guard * 0.5,
            (v05.continuity + v05.identity) / 2.0,
        ],
        transferability_2: [v05.transferability, v05.transferability * 0.8],
    }
}

/// Continuity dim score - cross-session continuity.
pub fn continuity_score(input: &CognitiveInput) -> f64 {
    // session_id present -> high continuity; absent -> low.
    if input.session_id.is_some() {
        0.85
    } else {
        0.45
    }
}

/// Salience dim score - memory salience.
pub fn salience_score(input: &CognitiveInput) -> f64 {
    // more candidate actions = richer signal.
    match input.candidate_targets.len() {
        0 => 0.0,
        1 => 0.50,
        2..=5 => 0.70,
        _ => 0.90,
    }
}

/// Identity dim score - identity stability.
pub fn identity_score(input: &CognitiveInput) -> f64 {
    // context_tag length approximates identity stability (heuristic).
    let len = input.context_tag.chars().count();
    ((len as f64) / 64.0).min(1.0).max(0.1)
}

/// Philosophy-guard dim score - philosophy gate pass rate (this cycle).
///
/// Approximated by input legality - full version would coordinate with V3 9-key + v4.1 12-key verdict.
pub fn philosophy_guard_score(input: &CognitiveInput) -> f64 {
    if input.validate().is_ok() {
        0.95
    } else {
        0.20
    }
}

/// Transferability dim score - knowledge transfer capability.
pub fn transferability_score(input: &CognitiveInput) -> f64 {
    // timestamp closer to now -> higher transfer value.
    let now = Utc::now().timestamp();
    let age = (now - input.timestamp).abs() as f64;
    (1.0 / (1.0 + age / 3600.0)).clamp(0.1, 1.0)
}

/// Validate that an ASI score is in [0.0, 1.0].
pub fn validate_asi_score(score: f64) -> CognitionResult<()> {
    if !(0.0..=1.0).contains(&score) {
        return Err(CognitionError::AsiOutOfRange(score));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::ActionTarget;

    fn sample_input() -> CognitiveInput {
        let target = ActionTarget::NormalAction("noop".to_string());
        CognitiveInput::new(vec![target], "scoring_test")
    }

    #[test]
    fn continuity_score_depends_on_session_id() {
        let mut input = sample_input();
        assert!(continuity_score(&input) < 0.5);
        input.session_id = Some(uuid::Uuid::new_v4());
        assert!(continuity_score(&input) > 0.5);
    }

    #[test]
    fn salience_score_handles_empty() {
        let input = CognitiveInput::new(vec![], "x");
        // validate fails but salience is purely len-based, doesn't depend on validate.
        assert_eq!(salience_score(&input), 0.0);
    }

    #[test]
    fn salience_score_handles_single_target() {
        let input = sample_input();
        assert!((salience_score(&input) - 0.50).abs() < 0.01);
    }

    #[test]
    fn identity_score_bounds_in_unit_interval() {
        let input = sample_input();
        let s = identity_score(&input);
        assert!((0.0..=1.0).contains(&s));
    }

    #[test]
    fn philosophy_guard_score_high_for_valid_input() {
        let input = sample_input();
        assert!(philosophy_guard_score(&input) > 0.5);
    }

    #[test]
    fn transferability_score_recent_input_is_high() {
        let input = sample_input();
        assert!(transferability_score(&input) > 0.5);
    }

    #[test]
    fn validate_asi_score_accepts_unit_interval() {
        assert!(validate_asi_score(0.0).is_ok());
        assert!(validate_asi_score(0.5).is_ok());
        assert!(validate_asi_score(1.0).is_ok());
    }

    #[test]
    fn validate_asi_score_rejects_out_of_range() {
        assert!(validate_asi_score(-0.1).is_err());
        assert!(validate_asi_score(1.1).is_err());
        assert!(validate_asi_score(2.0).is_err());
    }

    #[test]
    fn score_v05_returns_full_struct() {
        let input = sample_input();
        let v05 = score_v05(&input);
        assert_eq!(v05.continuity.is_finite(), true);
        assert_eq!(v05.salience.is_finite(), true);
        assert_eq!(v05.identity.is_finite(), true);
        assert_eq!(v05.philosophy_guard.is_finite(), true);
        assert_eq!(v05.transferability.is_finite(), true);
    }

    #[test]
    fn score_v1136_returns_full_struct() {
        let input = sample_input();
        let v1136 = score_v1136(&input);
        assert_eq!(v1136.continuity_5.len(), 5);
        assert_eq!(v1136.transferability_2.len(), 2);
    }
}
