//! StateError — apeireth-state 错误类型 (thiserror derive).
//!
//! - 5 variant StateError (GO-3 设计)
//! - thiserror derive 自动实现 Display + std::error::Error
//! - StateErrorKind 独立枚举 (轻量、Hash、序列化摘要)
//! - From impls: lock-poisoning, &str, StateErrorKind
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
pub enum StateError {
    #[error("state not initialized")]
    NotInitialized,

    #[error("internal lock poisoned: {0}")]
    LockFailed(String),

    #[error("type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("organ not registered: {0}")]
    OrganMissing(String),

    #[error("organ already registered: {0}")]
    AlreadyRegistered(String),
}

/// 序列化摘要 (per R22 documentation policy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateErrorKind {
    NotInitialized,
    LockFailed,
    TypeMismatch,
    OrganMissing,
    AlreadyRegistered,
}

/// Total variant count — used by `lib.rs` compile-time gate.
pub const STATE_ERROR_VARIANT_COUNT: usize = 5;

impl StateError {
    pub fn kind(&self) -> StateErrorKind {
        match self {
            Self::NotInitialized => StateErrorKind::NotInitialized,
            Self::LockFailed(_) => StateErrorKind::LockFailed,
            Self::TypeMismatch { .. } => StateErrorKind::TypeMismatch,
            Self::OrganMissing(_) => StateErrorKind::OrganMissing,
            Self::AlreadyRegistered(_) => StateErrorKind::AlreadyRegistered,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotInitialized => "not_initialized",
            Self::LockFailed(_) => "lock_failed",
            Self::TypeMismatch { .. } => "type_mismatch",
            Self::OrganMissing(_) => "organ_missing",
            Self::AlreadyRegistered(_) => "already_registered",
        }
    }
}

impl From<StateErrorKind> for StateError {
    fn from(k: StateErrorKind) -> Self {
        match k {
            StateErrorKind::NotInitialized => Self::NotInitialized,
            StateErrorKind::LockFailed => Self::LockFailed("poisoned".into()),
            StateErrorKind::TypeMismatch => Self::TypeMismatch {
                expected: "?".into(),
                actual: "?".into(),
            },
            StateErrorKind::OrganMissing => Self::OrganMissing("?".into()),
            StateErrorKind::AlreadyRegistered => Self::AlreadyRegistered("?".into()),
        }
    }
}

impl From<&str> for StateError {
    fn from(s: &str) -> Self {
        Self::LockFailed(s.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for StateError {
    fn from(e: std::sync::PoisonError<T>) -> Self {
        Self::LockFailed(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_and_error_trait() {
        let e = StateError::NotInitialized;
        assert_eq!(e.to_string(), "state not initialized");
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn kind_round_trip() {
        for k in [
            StateErrorKind::NotInitialized,
            StateErrorKind::LockFailed,
            StateErrorKind::TypeMismatch,
            StateErrorKind::OrganMissing,
            StateErrorKind::AlreadyRegistered,
        ] {
            let e: StateError = k.into();
            assert_eq!(e.kind(), k);
        }
    }

    #[test]
    fn as_str_matches_kind() {
        assert_eq!(StateError::NotInitialized.as_str(), "not_initialized");
        assert_eq!(StateError::LockFailed("x".into()).as_str(), "lock_failed");
    }

    #[test]
    fn from_str_converts_to_lock_failed() {
        let e: StateError = "mutex poisoned".into();
        assert!(matches!(e, StateError::LockFailed(_)));
    }

    #[test]
    fn variant_count_constant() {
        assert_eq!(STATE_ERROR_VARIANT_COUNT, 5);
    }
}