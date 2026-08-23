//! Failures at the orchestration layer.

use apeireth_core::kernel::{CoreError, SessionId};
use apeireth_plugin::{PluginError, ProviderError};
use thiserror::Error;

/// Result alias for the canonical runtime.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// A failure composing, or executing against, the runtime.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RuntimeError {
    /// A primitive-layer failure.
    #[error(transparent)]
    Core(#[from] CoreError),

    /// A plugin-layer failure: registration, lifecycle, or capability lookup.
    #[error(transparent)]
    Plugin(#[from] PluginError),

    /// Every candidate provider failed with a retryable error.
    #[error("all providers failed for model {model:?}; last error: {source}")]
    ProvidersExhausted {
        /// The model that could not be served.
        model: String,
        /// The final failure.
        #[source]
        source: ProviderError,
    },

    /// A provider failed in a way that makes falling back pointless.
    #[error(transparent)]
    Provider(#[from] ProviderError),

    /// No registered provider claims to serve this model.
    #[error("no provider serves model {model:?}; registered providers: {available}")]
    NoProvider {
        /// The model that was requested.
        model: String,
        /// Which providers are registered, so the mismatch is visible without
        /// a second lookup.
        available: String,
    },

    /// Providers claim the model, but each has been sidelined by health state.
    #[error("all providers serving model {model:?} are unhealthy: {unhealthy}")]
    NoHealthyProvider {
        /// The model that was requested.
        model: String,
        /// Providers excluded from this routing attempt.
        unhealthy: String,
    },

    /// Governance refused the turn.
    #[error("governance denied the turn: {reason}")]
    Denied {
        /// Which hook decided.
        hook: String,
        /// The stated reason.
        reason: String,
    },

    /// Governance requires a human decision before this turn can proceed.
    #[error("governance requires approval: {reason}")]
    ApprovalRequired {
        /// Which hook decided.
        hook: String,
        /// What a human is being asked to approve.
        reason: String,
    },

    /// The session store could not serve or persist a session.
    #[error("session {session} could not be {operation}: {reason}")]
    Session {
        /// The session involved.
        session: SessionId,
        /// `loaded` or `saved`.
        operation: &'static str,
        /// What went wrong.
        reason: String,
    },

    /// The turn made too many provider round-trips without reaching an answer.
    ///
    /// Distinct from a governance denial: this is the runtime's own structural
    /// guard, and it fires even when no policy is configured at all.
    #[error("turn did not converge within {limit} rounds")]
    RoundLimitExceeded {
        /// The configured limit.
        limit: u32,
    },

    /// The runtime was assembled in an unusable state.
    #[error("runtime misconfigured: {0}")]
    Misconfigured(String),
}

impl RuntimeError {
    /// A session could not be loaded.
    pub fn session_load(session: SessionId, reason: impl Into<String>) -> Self {
        Self::Session {
            session,
            operation: "loaded",
            reason: reason.into(),
        }
    }

    /// A session could not be saved.
    pub fn session_save(session: SessionId, reason: impl Into<String>) -> Self {
        Self::Session {
            session,
            operation: "saved",
            reason: reason.into(),
        }
    }

    /// The runtime was assembled in an unusable state.
    pub fn misconfigured(reason: impl Into<String>) -> Self {
        Self::Misconfigured(reason.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_provider_lists_what_is_registered() {
        let e = RuntimeError::NoProvider {
            model: "absent-model".into(),
            available: "provider.fake".into(),
        };
        let msg = e.to_string();
        assert!(msg.contains("absent-model"), "{msg}");
        assert!(
            msg.contains("provider.fake"),
            "the message must show the mismatch without a second lookup: {msg}"
        );
    }

    #[test]
    fn a_denial_and_an_approval_requirement_read_differently() {
        let denied = RuntimeError::Denied {
            hook: "deny_capabilities".into(),
            reason: "not permitted".into(),
        };
        let approval = RuntimeError::ApprovalRequired {
            hook: "human_gate".into(),
            reason: "needs a human".into(),
        };
        assert!(denied.to_string().contains("denied"), "{denied}");
        assert!(approval.to_string().contains("approval"), "{approval}");
    }
}
