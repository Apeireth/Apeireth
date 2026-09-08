//! Runtime-owned provider-context projection port.
//!
//! Projection is deliberately expressed only in terms of canonical protocol
//! messages. Concrete memory/context-window implementations belong in an
//! assembly crate and are injected through this port.

use apeireth_protocol::canonical::NormalizedMessage;

/// A failure while constructing a provider-facing context projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextProjectionError {
    message: String,
}

impl ContextProjectionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ContextProjectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ContextProjectionError {}

/// Runtime-owned hook for making a provider-facing message projection.
///
/// The transcript is borrowed and must not be mutated. `model_context_tokens`
/// is optional because provider metadata may not advertise a context limit.
/// Implementations should return an owned vector; runtime callers retain the
/// original transcript and fail open to it when projection fails.
pub trait ContextProjector: Send + Sync {
    fn project(
        &self,
        transcript: &[NormalizedMessage],
        model_context_tokens: Option<u32>,
    ) -> Result<Vec<NormalizedMessage>, ContextProjectionError>;
}

/// The default projector, preserving the existing unbounded request behavior.
#[derive(Debug, Default)]
pub struct NoContextProjector;

impl ContextProjector for NoContextProjector {
    fn project(
        &self,
        transcript: &[NormalizedMessage],
        _model_context_tokens: Option<u32>,
    ) -> Result<Vec<NormalizedMessage>, ContextProjectionError> {
        Ok(transcript.to_vec())
    }
}
