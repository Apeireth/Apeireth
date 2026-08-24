//! Declarative child spec — what to run, when to restart, how many times.
//!
//! ponytail: process invocation lives here, not in the supervisor; the ceiling is
//! declarative RollbackOnFailure (snapshot before restart); upgrade path is to
//! inject a `pre_restart` hook per spec.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::strategy::{should_restart, RestartStrategy};

/// Declarative spec for a supervised child.
///
/// ponytail: command builders and restart-window tracking are intentionally
/// omitted from this struct; they're owned by the supervisor that owns the spec.
/// The upgrade path is "add a `pre_restart: Option<Box<dyn Fn>>` field" if
/// callers need a hook; nobody needs it yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildSpec {
    /// Stable id within the parent's child list (used for sibling indexing).
    pub id: String,
    /// Human label for logs.
    pub label: String,
    /// What to run. On Windows we use `cmd /c exit <code>`; on Unix we use `true`/`false`.
    pub program: String,
    pub args: Vec<String>,
    pub restart: RestartStrategy,
    /// Maximum restarts allowed within `restart_window`. After this, the parent gives up.
    pub max_restarts: u32,
    pub restart_window: Duration,
    /// Snapshot taken before the latest start (used for rollback-on-failure-loop).
    pub snapshot_id: Option<String>,
}

impl ChildSpec {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        program: impl Into<String>,
        restart: RestartStrategy,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            program: program.into(),
            args: Vec::new(),
            restart,
            max_restarts: 5,
            restart_window: Duration::from_secs(5),
            snapshot_id: None,
        }
    }

    pub fn with_arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }

    pub fn with_max_restarts(mut self, n: u32) -> Self {
        self.max_restarts = n;
        self
    }

    pub fn with_restart_window(mut self, d: Duration) -> Self {
        self.restart_window = d;
        self
    }

    pub fn with_snapshot(mut self, id: impl Into<String>) -> Self {
        self.snapshot_id = Some(id.into());
        self
    }

    /// Decide whether this child should restart after `reason`.
    pub fn decide(&self, reason: crate::strategy::ExitReason) -> crate::strategy::RestartDecision {
        should_restart(self.restart, reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_delegates_to_strategy() {
        let spec = ChildSpec::new("a", "alpha", "true", RestartStrategy::Transient);
        assert_eq!(
            spec.decide(crate::strategy::ExitReason::Normal),
            crate::strategy::RestartDecision::Skip
        );
        assert_eq!(
            spec.decide(crate::strategy::ExitReason::Abnormal(1)),
            crate::strategy::RestartDecision::Restart
        );
    }

    #[test]
    fn builder_chains() {
        let spec = ChildSpec::new("a", "alpha", "cmd", RestartStrategy::OneForOne)
            .with_arg("/c")
            .with_arg("exit")
            .with_arg("0")
            .with_max_restarts(3)
            .with_restart_window(Duration::from_secs(10))
            .with_snapshot("snap-1");
        assert_eq!(spec.args, vec!["/c", "exit", "0"]);
        assert_eq!(spec.max_restarts, 3);
        assert_eq!(spec.snapshot_id.as_deref(), Some("snap-1"));
    }
}
