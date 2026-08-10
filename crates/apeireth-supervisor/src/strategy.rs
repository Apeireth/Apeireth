//! Restart strategies for child supervision.
//!
//! - [`RestartStrategy::OneForOne`]: only the failed child restarts
//! - [`RestartStrategy::RestForOne`]: failed child + all subsequent siblings restart
//! - [`RestartStrategy::Transient`]: only abnormal exits trigger restart

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RestartStrategy {
    OneForOne,
    RestForOne,
    Transient,
}

impl Default for RestartStrategy {
    fn default() -> Self {
        Self::OneForOne
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    Normal,
    Abnormal(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartDecision {
    Restart,
    Skip,
}

/// Decide whether to restart based on strategy + exit reason.
///
/// ponytail: trivial dispatch; the ceiling is a policy DSL, the upgrade path is
/// a per-supervisor restart budget. Not implemented because no caller needs it yet.
pub fn should_restart(strategy: RestartStrategy, reason: ExitReason) -> RestartDecision {
    match strategy {
        RestartStrategy::OneForOne | RestartStrategy::RestForOne => RestartDecision::Restart,
        RestartStrategy::Transient => match reason {
            ExitReason::Normal => RestartDecision::Skip,
            ExitReason::Abnormal(_) => RestartDecision::Restart,
        },
    }
}

/// Inclusive index range of siblings affected by a restart.
///
/// ponytail: trivial arithmetic; the ceiling is "per-child rest-rate limit", not needed yet.
pub fn affected_indices(
    strategy: RestartStrategy,
    failed: usize,
    total: usize,
) -> std::ops::RangeInclusive<usize> {
    debug_assert!(failed < total);
    match strategy {
        RestartStrategy::OneForOne => failed..=failed,
        RestartStrategy::RestForOne => failed..=(total - 1),
        // Transient never asks "who else" — only the failed child is in scope
        RestartStrategy::Transient => failed..=failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_for_one_restarts_any() {
        assert_eq!(
            should_restart(RestartStrategy::OneForOne, ExitReason::Normal),
            RestartDecision::Restart
        );
        assert_eq!(
            should_restart(RestartStrategy::OneForOne, ExitReason::Abnormal(1)),
            RestartDecision::Restart
        );
    }

    #[test]
    fn rest_for_one_restarts_any() {
        assert_eq!(
            should_restart(RestartStrategy::RestForOne, ExitReason::Normal),
            RestartDecision::Restart
        );
    }

    #[test]
    fn transient_skips_normal() {
        assert_eq!(
            should_restart(RestartStrategy::Transient, ExitReason::Normal),
            RestartDecision::Skip
        );
        assert_eq!(
            should_restart(RestartStrategy::Transient, ExitReason::Abnormal(0)),
            RestartDecision::Restart
        );
    }

    #[test]
    fn affected_range_one_for_one_isolates_failed() {
        assert_eq!(
            affected_indices(RestartStrategy::OneForOne, 0, 3).collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            affected_indices(RestartStrategy::OneForOne, 2, 4).collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn affected_range_rest_for_one_takes_subsequent() {
        assert_eq!(
            affected_indices(RestartStrategy::RestForOne, 0, 3).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            affected_indices(RestartStrategy::RestForOne, 1, 4).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
    }
}
