//! The canonical lifecycle of a managed component.
//!
//! Note the distinction from [`crate::lifecycle`], which models the *cognitive*
//! loop (perception, retrieval, dreaming). This one models *managed component
//! state*: registered, starting, running, stopping, stopped, failed. They share a
//! word and nothing else, which is why this one lives under `kernel`.
//!
//! Transitions are checked rather than assigned. A component that reports
//! `Active` without having passed through `Initializing` is a bug that should
//! surface at the transition, not three subsystems later.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::{CoreError, CoreResult};

/// State of a managed component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    /// Known to the registry; not yet started. Declared capabilities are visible
    /// but must not be dispatched to.
    Registered,
    /// Start-up in progress.
    Initializing,
    /// Running; capabilities may be dispatched to.
    Active,
    /// Shutdown in progress; no new dispatch is accepted.
    Stopping,
    /// Shut down cleanly. Terminal for this run.
    Stopped,
    /// Start-up or operation failed. Terminal for this run.
    Failed,
}

impl Lifecycle {
    /// Stable lowercase name, used in errors, logs, and wire formats.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Registered => "registered",
            Self::Initializing => "initializing",
            Self::Active => "active",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }

    /// Whether the component may currently serve dispatch.
    pub const fn is_dispatchable(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Whether no further transition is possible in this run.
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed)
    }

    /// Whether `self -> next` is a permitted transition.
    ///
    /// `Failed` is reachable from every non-terminal state: a component can break
    /// while starting, while running, or while shutting down.
    pub const fn can_transition_to(&self, next: Self) -> bool {
        match (self, next) {
            (Self::Registered, Self::Initializing)
            | (Self::Initializing, Self::Active)
            | (Self::Active, Self::Stopping)
            | (Self::Stopping, Self::Stopped) => true,

            // A component that never started still needs to reach a terminal
            // state when the manager shuts down mid-boot.
            (Self::Registered, Self::Stopped) => true,

            // Failure can interrupt any non-terminal state.
            (
                Self::Registered | Self::Initializing | Self::Active | Self::Stopping,
                Self::Failed,
            ) => true,

            _ => false,
        }
    }

    /// Perform a checked transition.
    ///
    /// `subject` names the thing being transitioned so the error is actionable.
    pub fn transition_to(self, subject: &str, next: Self) -> CoreResult<Self> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(CoreError::IllegalTransition {
                subject: subject.to_string(),
                from: self.as_str(),
                to: next.as_str(),
            })
        }
    }
}

impl fmt::Display for Lifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_happy_path_runs_end_to_end() {
        let mut state = Lifecycle::Registered;
        for next in [
            Lifecycle::Initializing,
            Lifecycle::Active,
            Lifecycle::Stopping,
            Lifecycle::Stopped,
        ] {
            state = state.transition_to("plugin.example", next).unwrap();
        }
        assert_eq!(state, Lifecycle::Stopped);
        assert!(state.is_terminal());
    }

    #[test]
    fn only_active_is_dispatchable() {
        for state in [
            Lifecycle::Registered,
            Lifecycle::Initializing,
            Lifecycle::Stopping,
            Lifecycle::Stopped,
            Lifecycle::Failed,
        ] {
            assert!(!state.is_dispatchable(), "{state} must not be dispatchable");
        }
        assert!(Lifecycle::Active.is_dispatchable());
    }

    #[test]
    fn skipping_initialization_is_rejected() {
        let err = Lifecycle::Registered
            .transition_to("plugin.example", Lifecycle::Active)
            .unwrap_err();

        match err {
            CoreError::IllegalTransition { subject, from, to } => {
                assert_eq!(subject, "plugin.example");
                assert_eq!(from, "registered");
                assert_eq!(to, "active");
            }
            other => panic!("expected IllegalTransition, got {other:?}"),
        }
    }

    #[test]
    fn failure_can_interrupt_any_non_terminal_state() {
        for state in [
            Lifecycle::Registered,
            Lifecycle::Initializing,
            Lifecycle::Active,
            Lifecycle::Stopping,
        ] {
            assert!(
                state.transition_to("s", Lifecycle::Failed).is_ok(),
                "{state} -> failed must be permitted"
            );
        }
    }

    #[test]
    fn terminal_states_do_not_transition() {
        for state in [Lifecycle::Stopped, Lifecycle::Failed] {
            for next in [
                Lifecycle::Registered,
                Lifecycle::Initializing,
                Lifecycle::Active,
                Lifecycle::Stopping,
                Lifecycle::Stopped,
                Lifecycle::Failed,
            ] {
                assert!(
                    !state.can_transition_to(next),
                    "{state} is terminal but allowed -> {next}"
                );
            }
        }
    }

    #[test]
    fn serializes_to_its_stable_name() {
        assert_eq!(
            serde_json::to_string(&Lifecycle::Initializing).unwrap(),
            r#""initializing""#
        );
    }
}
