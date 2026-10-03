//! Failures of a mesh call, and the execution-side failures that map onto them.
//!
//! Two error surfaces on purpose. [`MeshExecError`] is what the *executing*
//! node's canonical dispatch produces; [`MeshError`] is what the *calling* side
//! observes. The wire carries [`CapabilityError`] between them, and the mapping
//! is total in both directions so a remote refusal reads like a local one.

use apeireth_core::kernel::CapabilityId;
use apeireth_mesh_protocol::{CapabilityError, MeshErrorCode, MeshProtocolError};

/// A failure on the calling side of a mesh operation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MeshError {
    /// The underlying connection failed or closed.
    #[error("mesh transport failure: {0}")]
    Transport(String),

    /// The handshake was refused or invalid. Fail closed.
    #[error(transparent)]
    Handshake(#[from] MeshProtocolError),

    /// The node is not in this runtime's known peers, or is not trusted.
    #[error("node {node} is not trusted for mesh use: {reason}")]
    Untrusted {
        /// The offending node.
        node: String,
        /// Why it was refused.
        reason: String,
    },

    /// The peer exists but cannot accept the call right now.
    #[error("peer {node} is not available for dispatch (state: {state})")]
    PeerUnavailable {
        /// Which peer.
        node: String,
        /// Its current lifecycle state.
        state: &'static str,
    },

    /// No route for this capability under the requested selection.
    #[error("no {selection} route for capability {capability}: {detail}")]
    RouteUnavailable {
        /// Which capability could not be routed.
        capability: CapabilityId,
        /// The scope that was explicitly selected.
        selection: String,
        /// Why the route is absent or unusable.
        detail: String,
    },

    /// A wire message violated the protocol contract (e.g. a capability kind
    /// that disagrees with its id prefix). Refused rather than trusted.
    ///
    /// The sender field is deliberately not named `source`: thiserror treats a
    /// field of that name as the error's causal source.
    #[error("malformed mesh message from {sender}: {reason}")]
    Malformed {
        /// Who sent it.
        sender: String,
        /// What is wrong with it.
        reason: String,
    },

    /// The executing node refused or failed the invocation; it did not
    /// complete as a capability outcome.
    #[error("remote capability error ({code:?}): {message}")]
    Remote {
        /// Capability identity, when the failure is tied to one.
        capability: Option<CapabilityId>,
        /// What kind of remote failure.
        code: MeshErrorCode,
        /// Legible detail.
        message: String,
    },

    /// Too many calls are already in flight to this peer. Nothing was sent.
    #[error("peer {node} is saturated: {in_flight} calls in flight (cap {cap})")]
    Backpressure {
        /// Which peer.
        node: String,
        /// Calls currently pending against it.
        in_flight: usize,
        /// The configured cap.
        cap: usize,
    },

    /// A newer connection to the same peer superseded this one (deterministic
    /// duplicate-connection policy). The surviving connection is named.
    #[error("connection superseded by a duplicate; kept connection to {kept}")]
    Superseded {
        /// The peer whose surviving connection was kept.
        kept: String,
    },

    /// The call was cancelled before it produced an outcome.
    #[error("mesh call was cancelled")]
    Cancelled,

    /// The caller's deadline elapsed before an outcome arrived.
    #[error("mesh call exceeded its deadline")]
    DeadlineExceeded,
}

impl MeshError {
    /// Map a wire-level capability error back onto the caller's surface.
    pub fn from_capability_error(error: CapabilityError) -> Self {
        match error.code {
            MeshErrorCode::Cancelled => Self::Cancelled,
            MeshErrorCode::DeadlineExceeded => Self::DeadlineExceeded,
            code => Self::Remote {
                capability: error.capability,
                code,
                message: error.message,
            },
        }
    }
}

/// A failure inside the executing node's canonical dispatch path.
///
/// The semantic split matters: a tool that ran and failed produces a
/// [`apeireth_protocol::canonical::ToolResult`] error (the model's business),
/// while these mean the invocation never legitimately ran (refused, unknown,
/// gone) — the caller's business.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MeshExecError {
    /// No such capability is indexed.
    #[error("capability {capability} is not known on this node")]
    UnknownCapability {
        /// The missing capability.
        capability: CapabilityId,
    },

    /// The capability exists but its module cannot dispatch right now.
    #[error("capability {capability} is not available: {reason}")]
    NotAvailable {
        /// The blocked capability.
        capability: CapabilityId,
        /// What blocks it.
        reason: String,
    },

    /// Canonical governance refused the invocation. Nothing executed.
    #[error("governance denied capability {capability}: {reason}")]
    Denied {
        /// The refused capability.
        capability: CapabilityId,
        /// Why.
        reason: String,
    },

    /// Governance wants a human; remote calls fail closed instead of minting a
    /// hidden approval.
    #[error(
        "capability {capability} requires approval which remote calls cannot request: {reason}"
    )]
    ApprovalRequired {
        /// The gated capability.
        capability: CapabilityId,
        /// Why a human would have been asked.
        reason: String,
    },

    /// The invocation was cancelled.
    #[error("capability {capability} was cancelled")]
    Cancelled {
        /// The cancelled capability.
        capability: CapabilityId,
    },

    /// The caller's deadline elapsed before execution started or finished.
    #[error("capability {capability} exceeded its deadline")]
    DeadlineExceeded {
        /// The timed-out capability.
        capability: CapabilityId,
    },

    /// Execution failed in a way not described above.
    #[error("capability {capability} failed internally: {message}")]
    Internal {
        /// The failing capability.
        capability: CapabilityId,
        /// What happened.
        message: String,
    },
}

impl MeshExecError {
    /// Map an execution-side failure onto its wire form.
    pub fn to_capability_error(&self) -> CapabilityError {
        let (capability, code, message) = match self {
            Self::UnknownCapability { capability } => (
                Some(capability.clone()),
                MeshErrorCode::UnknownCapability,
                self.to_string(),
            ),
            Self::NotAvailable { capability, .. } => (
                Some(capability.clone()),
                MeshErrorCode::NotAvailable,
                self.to_string(),
            ),
            Self::Denied { capability, .. } => (
                Some(capability.clone()),
                MeshErrorCode::Denied,
                self.to_string(),
            ),
            Self::ApprovalRequired { capability, .. } => (
                Some(capability.clone()),
                MeshErrorCode::ApprovalRequired,
                self.to_string(),
            ),
            Self::Cancelled { capability } => (
                Some(capability.clone()),
                MeshErrorCode::Cancelled,
                self.to_string(),
            ),
            Self::DeadlineExceeded { capability } => (
                Some(capability.clone()),
                MeshErrorCode::DeadlineExceeded,
                self.to_string(),
            ),
            Self::Internal { capability, .. } => (
                Some(capability.clone()),
                MeshErrorCode::Internal,
                self.to_string(),
            ),
        };
        CapabilityError {
            capability,
            code,
            message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability() -> CapabilityId {
        CapabilityId::new("tool.mesh.echo").unwrap()
    }

    #[test]
    fn execution_failures_map_to_wire_errors_and_back() {
        for exec in [
            MeshExecError::UnknownCapability {
                capability: capability(),
            },
            MeshExecError::Denied {
                capability: capability(),
                reason: "policy".into(),
            },
            MeshExecError::Cancelled {
                capability: capability(),
            },
            MeshExecError::DeadlineExceeded {
                capability: capability(),
            },
        ] {
            let wire = exec.to_capability_error();
            let back = MeshError::from_capability_error(wire);
            match back {
                MeshError::Cancelled => assert!(matches!(exec, MeshExecError::Cancelled { .. })),
                MeshError::DeadlineExceeded => {
                    assert!(matches!(exec, MeshExecError::DeadlineExceeded { .. }))
                }
                MeshError::Remote {
                    code, capability, ..
                } => {
                    assert_eq!(capability, exec.to_capability_error().capability);
                    let expected = match exec {
                        MeshExecError::UnknownCapability { .. } => MeshErrorCode::UnknownCapability,
                        MeshExecError::Denied { .. } => MeshErrorCode::Denied,
                        _ => code,
                    };
                    assert_eq!(code, expected);
                }
                other => panic!("unexpected mapping: {other}"),
            }
        }
    }
}
