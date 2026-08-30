//! Motivation / value score formula (V0.5 §13) — algorithm only.
//!
//! Recovered from `legacy/donor/apeireth-motivation/src/lib.rs` `motivation_score`.
//!
//! **Not ported** (agent 15 owns proactive / cron / goals):
//! - SGI write-flow / C-SGI-1..7 uniqueness machine
//! - `MotivationDrive` as a second goal owner
//! - ReflectionAuditor / Uuid history
//!
//! Weights are a proposed starting point (`0.35 / 0.35 / 0.30`), not frozen.
//! Default-off library primitive.

/// Proposed §13 weights: autonomy / value / intrinsic. Not frozen.
pub const MOTIVATION_WEIGHTS: (f64, f64, f64) = (0.35, 0.35, 0.30);

/// Hard threshold used by the donor (`MIN_EVIDENCE_SCORE` = 0.85).
pub const MOTIVATION_THRESHOLD: f64 = 0.85;

/// Autonomy-consistency inputs (internal drive vs history share).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutonomyConsistency {
    /// Current internal-drive intensity `[0, 1]`.
    pub internal_intensity: f64,
    /// Share of history that is internal `[0, 1]`.
    pub internal_history_ratio: f64,
}

/// Value-stability inputs (turnover + deadline variance; lower is better).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueStability {
    /// Goal-turnover rate `[0, 1]`.
    pub goal_turnover: f64,
    /// Deadline-span variance, normalized `[0, 1]`.
    pub deadline_variance: f64,
}

/// Intrinsic-intensity inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntrinsicIntensity {
    /// Current SGI internal intensity `[0, 1]`.
    pub current_internal: f64,
    /// Historical internal-intensity peak `[0, 1]`.
    pub historical_peak: f64,
}

/// Weighted motivation / value score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotivationScore {
    pub total: f64,
    pub autonomy: f64,
    pub value: f64,
    pub intrinsic: f64,
    pub passes_threshold: bool,
}

/// `motivation_score = w1*autonomy + w2*value + w3*intrinsic`.
///
/// - autonomy = sqrt(intensity × history_ratio)  (geometric; either low pulls down)
/// - value    = mean(1 - turnover, 1 - deadline_variance)
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
        passes_threshold: total >= MOTIVATION_THRESHOLD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_agent_passes_threshold() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 0.95,
                internal_history_ratio: 0.9,
            },
            ValueStability {
                goal_turnover: 0.05,
                deadline_variance: 0.1,
            },
            IntrinsicIntensity {
                current_internal: 0.9,
                historical_peak: 0.95,
            },
        );
        assert!(score.passes_threshold, "healthy total={}", score.total);
        assert!(score.total >= MOTIVATION_THRESHOLD);
        assert!((score.autonomy - (0.95 * 0.9_f64).sqrt()).abs() < 1e-12);
    }

    #[test]
    fn low_intrinsic_fails_threshold() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 0.95,
                internal_history_ratio: 0.9,
            },
            ValueStability {
                goal_turnover: 0.05,
                deadline_variance: 0.1,
            },
            IntrinsicIntensity {
                current_internal: 0.0,
                historical_peak: 0.0,
            },
        );
        assert!(!score.passes_threshold);
        assert_eq!(score.intrinsic, 0.0);
    }

    #[test]
    fn zero_inputs_zero_total() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 0.0,
                internal_history_ratio: 0.0,
            },
            ValueStability {
                goal_turnover: 1.0,
                deadline_variance: 1.0,
            },
            IntrinsicIntensity {
                current_internal: 0.0,
                historical_peak: 0.0,
            },
        );
        assert_eq!(score.total, 0.0);
        assert!(!score.passes_threshold);
    }

    #[test]
    fn mid_weights_are_convex_combination() {
        let score = motivation_score(
            AutonomyConsistency {
                internal_intensity: 1.0,
                internal_history_ratio: 1.0,
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
        assert!((score.total - 1.0).abs() < 1e-12);
        assert_eq!(score.autonomy, 1.0);
        assert_eq!(score.value, 1.0);
        assert_eq!(score.intrinsic, 1.0);
    }
}
