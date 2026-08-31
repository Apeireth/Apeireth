//! Motivation / value scoring formula (library, default-off).
//!
//! Canonical implementation module. §13
//! `motivation_score`. Pure f64 math: three named components, proposed
//! weights `(0.35, 0.35, 0.30)`, hard threshold `0.85`.
//!
//! **Not recovered** (ownership / architecture):
//! - `SGI` write-flow + C-SGI-1..7 as a second goal owner (v2 Goal SM is
//!   [`crate::goal::GoalService`]; SGI uniqueness is a policy vocabulary, not
//!   a second store).
//! - `MotivationDrive` trait as a runtime authority.
//! - Consciousness / life-force bridges, Kani stubs, Uuid-timestamped entries.
//!
//! Production wiring: none. Scoring is a function. Callers inject the three
//! component structs; this module never reads a clock or a store.

use serde::{Deserialize, Serialize};

/// Proposed §13 weights (autonomy, value-stability, intrinsic). Not frozen.
pub const MOTIVATION_WEIGHTS: (f64, f64, f64) = (0.35, 0.35, 0.30);

/// Hard threshold the canonical used (`MIN_EVIDENCE_SCORE` = 0.85).
pub const MIN_MOTIVATION_THRESHOLD: f64 = 0.85;

/// Alias kept for salvage-07 re-exports (`MOTIVATION_THRESHOLD`).
pub const MOTIVATION_THRESHOLD: f64 = MIN_MOTIVATION_THRESHOLD;

/// Autonomy consistency (internal drive × history ratio).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutonomyConsistency {
    /// Current internal-drive intensity `[0, 1]`.
    pub internal_intensity: f64,
    /// Share of history that was internal `[0, 1]`.
    pub internal_history_ratio: f64,
}

/// Value-orientation stability. Lower turnover / variance is more stable.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ValueStability {
    /// Goal-text turnover `[0, 1]` (lower is more stable).
    pub goal_turnover: f64,
    /// Deadline-span variance, normalized `[0, 1]` (lower is more stable).
    pub deadline_variance: f64,
}

/// Intrinsic intensity (current internal × historical peak).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IntrinsicIntensity {
    /// Current internal intensity `[0, 1]`.
    pub current_internal: f64,
    /// Historical internal peak `[0, 1]`.
    pub historical_peak: f64,
}

/// Composite motivation / value score.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MotivationScore {
    /// Weighted total `[0, 1]`.
    pub total: f64,
    /// Autonomy component.
    pub autonomy: f64,
    /// Value-stability component.
    pub value: f64,
    /// Intrinsic-intensity component.
    pub intrinsic: f64,
    /// Whether `total >= MIN_MOTIVATION_THRESHOLD`.
    pub passes_threshold: bool,
}

/// `motivation_score = w1*autonomy + w2*value + w3*intrinsic`.
///
/// - autonomy = sqrt(intensity × history_ratio)  (geometric; either low pulls down)
/// - value    = mean(1 − turnover, 1 − deadline_variance)
/// - intrinsic = mean(current, historical_peak)
pub fn motivation_score(
    autonomy: AutonomyConsistency,
    value: ValueStability,
    intrinsic: IntrinsicIntensity,
) -> MotivationScore {
    let (w1, w2, w3) = MOTIVATION_WEIGHTS;

    let autonomy_score =
        ((autonomy.internal_intensity * autonomy.internal_history_ratio).clamp(0.0, 1.0)).sqrt();

    let value_score = ((1.0 - value.goal_turnover).clamp(0.0, 1.0)
        + (1.0 - value.deadline_variance).clamp(0.0, 1.0))
        / 2.0;

    let intrinsic_score =
        ((intrinsic.current_internal + intrinsic.historical_peak) / 2.0).clamp(0.0, 1.0);

    let total = (w1 * autonomy_score + w2 * value_score + w3 * intrinsic_score).clamp(0.0, 1.0);

    MotivationScore {
        total,
        autonomy: autonomy_score,
        value: value_score,
        intrinsic: intrinsic_score,
        passes_threshold: total >= MIN_MOTIVATION_THRESHOLD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_agent_passes_threshold() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 0.9,
                internal_history_ratio: 0.85,
            },
            ValueStability {
                goal_turnover: 0.1,
                deadline_variance: 0.1,
            },
            IntrinsicIntensity {
                current_internal: 0.9,
                historical_peak: 0.95,
            },
        );
        assert!(score.passes_threshold);
        assert!(score.total >= MIN_MOTIVATION_THRESHOLD);
        assert!(score.autonomy > 0.8);
        assert!(score.value > 0.8);
        assert!(score.intrinsic > 0.8);
    }

    #[test]
    fn low_intrinsic_fails_threshold() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 0.5,
                internal_history_ratio: 0.5,
            },
            ValueStability {
                goal_turnover: 0.8,
                deadline_variance: 0.8,
            },
            IntrinsicIntensity {
                current_internal: 0.2,
                historical_peak: 0.3,
            },
        );
        assert!(!score.passes_threshold);
        assert!(score.total < MIN_MOTIVATION_THRESHOLD);
    }

    #[test]
    fn geometric_autonomy_collapses_when_either_factor_is_zero() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 1.0,
                internal_history_ratio: 0.0,
            },
            ValueStability {
                goal_turnover: 0.0,
                deadline_variance: 0.0,
            },
            IntrinsicIntensity {
                current_internal: 1.0,
                historical_peak: 1.0,
            },
        );
        assert_eq!(score.autonomy, 0.0);
        // weights 0.35*0 + 0.35*1 + 0.30*1 = 0.65 < 0.85
        assert!(!score.passes_threshold);
        assert!((score.total - 0.65).abs() < 1e-12);
    }

    #[test]
    fn clamps_out_of_range_inputs() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 2.0,
                internal_history_ratio: 2.0,
            },
            ValueStability {
                goal_turnover: -1.0,
                deadline_variance: -1.0,
            },
            IntrinsicIntensity {
                current_internal: 4.0,
                historical_peak: 4.0,
            },
        );
        assert_eq!(score.autonomy, 1.0);
        assert_eq!(score.value, 1.0);
        assert_eq!(score.intrinsic, 1.0);
        assert_eq!(score.total, 1.0);
        assert!(score.passes_threshold);
    }
}
