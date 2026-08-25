//! StateError — apeireth-state 错误类型.
use serde::{Deserialize, Serialize};

/// 5 variant StateError (matches GO-3 design).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateError {
    NotInitialized,
    LockFailed,
    TypeMismatch,
    OrganMissing,
    AlreadyRegistered,
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
            Self::LockFailed => StateErrorKind::LockFailed,
            Self::TypeMismatch => StateErrorKind::TypeMismatch,
            Self::OrganMissing => StateErrorKind::OrganMissing,
            Self::AlreadyRegistered => StateErrorKind::AlreadyRegistered,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NotInitialized => "not_initialized",
            Self::LockFailed => "lock_failed",
            Self::TypeMismatch => "type_mismatch",
            Self::OrganMissing => "organ_missing",
            Self::AlreadyRegistered => "already_registered",
        }
    }
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for StateError {}