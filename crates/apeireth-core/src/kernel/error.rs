//! Errors shared by every canonical subsystem.
//!
//! [`CoreError`] covers only failures that are meaningful at the primitive layer:
//! a malformed identifier, an illegal lifecycle transition, a missing entry, a
//! duplicate registration, a rejected precondition. Subsystem-specific failure
//! modes belong to the subsystem's own error type, which may wrap this one.
//!
//! Deliberately absent: HTTP status codes, provider error payloads, SQL errors.
//! Those belong to the layers that own those concerns.

use thiserror::Error;

/// Result alias for the canonical primitive layer.
pub type CoreResult<T> = Result<T, CoreError>;

/// A failure at the canonical primitive layer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// An identifier did not satisfy its grammar.
    #[error("invalid {kind} {value:?}: {reason}")]
    InvalidId {
        /// Which identifier type rejected the value.
        kind: &'static str,
        /// The offending value.
        value: String,
        /// Why it was rejected.
        reason: String,
    },

    /// A lifecycle transition was not permitted from the current state.
    #[error("illegal lifecycle transition: {subject} cannot go {from} -> {to}")]
    IllegalTransition {
        /// What was being transitioned, e.g. a plugin id.
        subject: String,
        /// The state it was in.
        from: &'static str,
        /// The state that was requested.
        to: &'static str,
    },

    /// A lookup found nothing.
    #[error("{kind} {id:?} not found")]
    NotFound {
        /// What was being looked up.
        kind: &'static str,
        /// The identifier that missed.
        id: String,
    },

    /// A registration collided with an existing entry.
    ///
    /// This is an error rather than a silent overwrite on purpose: two components
    /// claiming one identifier is exactly the "second source of truth" defect the
    /// canonical registries exist to prevent.
    #[error("{kind} {id:?} is already registered by {owner:?}")]
    Duplicate {
        /// What was being registered.
        kind: &'static str,
        /// The contested identifier.
        id: String,
        /// Who already holds it.
        owner: String,
    },

    /// A precondition for an operation did not hold.
    #[error("precondition failed: {0}")]
    Precondition(String),
}

impl CoreError {
    /// An identifier failed validation.
    pub fn invalid_id(
        kind: &'static str,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidId {
            kind,
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// A lookup missed.
    pub fn not_found(kind: &'static str, id: impl Into<String>) -> Self {
        Self::NotFound {
            kind,
            id: id.into(),
        }
    }

    /// A registration collided.
    pub fn duplicate(kind: &'static str, id: impl Into<String>, owner: impl Into<String>) -> Self {
        Self::Duplicate {
            kind,
            id: id.into(),
            owner: owner.into(),
        }
    }

    /// A precondition did not hold.
    pub fn precondition(reason: impl Into<String>) -> Self {
        Self::Precondition(reason.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_name_the_offending_value() {
        let e = CoreError::invalid_id("CapabilityId", "Tool.Shell", "must be lowercase");
        let msg = e.to_string();
        assert!(msg.contains("CapabilityId"), "{msg}");
        assert!(msg.contains("Tool.Shell"), "{msg}");
        assert!(msg.contains("must be lowercase"), "{msg}");
    }

    #[test]
    fn duplicate_names_the_incumbent_owner() {
        let e = CoreError::duplicate("capability", "tool.shell", "builtin.shell");
        let msg = e.to_string();
        assert!(msg.contains("tool.shell"), "{msg}");
        assert!(msg.contains("builtin.shell"), "{msg}");
    }
}
