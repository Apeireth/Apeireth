//! Pipeline errors.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PIPELINE_ERROR_VARIANT_COUNT: usize = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineErrorKind {
    DispatchFailed,
    NormalizeFailed,
    PolicyViolation,
    ReliabilityExhausted,
    ThrottleRejection,
    Internal,
}

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("dispatch failed: {0}")]
    DispatchFailed(String),
    #[error("normalize failed: {0}")]
    NormalizeFailed(String),
    #[error("policy violation: {0}")]
    PolicyViolation(String),
    #[error("reliability exhausted after {0} attempts")]
    ReliabilityExhausted(u32),
    #[error("throttle rejection: {0}")]
    ThrottleRejection(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl PipelineError {
    pub fn kind(&self) -> PipelineErrorKind {
        match self {
            PipelineError::DispatchFailed(_) => PipelineErrorKind::DispatchFailed,
            PipelineError::NormalizeFailed(_) => PipelineErrorKind::NormalizeFailed,
            PipelineError::PolicyViolation(_) => PipelineErrorKind::PolicyViolation,
            PipelineError::ReliabilityExhausted(_) => PipelineErrorKind::ReliabilityExhausted,
            PipelineError::ThrottleRejection(_) => PipelineErrorKind::ThrottleRejection,
            PipelineError::Internal(_) => PipelineErrorKind::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_count() {
        assert_eq!(PIPELINE_ERROR_VARIANT_COUNT, 6);
    }

    #[test]
    fn kind_mapping() {
        let e = PipelineError::DispatchFailed("x".into());
        assert_eq!(e.kind(), PipelineErrorKind::DispatchFailed);
        let e = PipelineError::ReliabilityExhausted(3);
        assert_eq!(e.kind(), PipelineErrorKind::ReliabilityExhausted);
    }
}
