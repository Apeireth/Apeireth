//! # StateError — 3 模式 state 错误类型
//!
//! 5 variant 设计 (per 借鉴 Golutra 9 Tauri state 错误模式 + 1 catch-all),
//! 编译期 enum exhaustive match 守门.
//!
//! ## 5 variant 守门 (编译期 hardcode)
//!
//! 1. `Poisoned { mode, organ }` — Mutex/RwLock 中毒 (panic 传播)
//! 2. `NotInitialized { mode, organ }` — OnceLockState 还未 init 时访问
//! 3. `TypeMismatch { expected, actual }` — SharedState 模式不匹配 (防御)
//! 4. `Unsupported { mode, organ, reason }` — 模式在当前 organ 不支持 (防御)
//! 5. `Other { msg }` — 兜底 (catch-all)
//!
//! ## 借用 Golutra
//!
//! Golutra 9 Tauri state 错误模式 (`tauri::Error::StateNotFound` / `tauri::Error::StateSetup` / ...),
//! 简化通用化到 5 类, 适配 ratatui 路线 (无 Tauri 框架).
//!
//! **0 抄 Golutra 业务代码**, 只借 5 类错误分类思想.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::organ::Organ;
use crate::shared_state::SharedStateMode;

/// **Error K-1 强校验 #1**: StateError 5 variant (编译期 hardcode, 跟 3 模式 + 1 防御 + 1 catch-all 对齐).
pub const STATE_ERROR_VARIANT_COUNT: usize = 5;

/// 3 模式 state 错误.
///
/// 跨 3 模式错误统一包装, 错误传播支持 `?`.
#[derive(Debug, Clone)]
pub enum StateError {
    /// Mutex / RwLock 中毒 (panic 传播).
    Poisoned {
        /// 失败的 state 模式 (Mutex / RwLock).
        mode: SharedStateMode,
        /// 受影响的 organ (per 9 器官 enum, 0-8).
        organ: Organ,
    },

    /// OnceLockState 还未 init 时访问 (per `OnceLockState::get_uninit`).
    NotInitialized {
        /// state 模式 (OnceLock).
        mode: SharedStateMode,
        /// 受影响的 organ.
        organ: Organ,
    },

    /// SharedState 模式不匹配 (防御: 调用方传了别的模式).
    TypeMismatch {
        /// 期望的模式 (e.g. `Mutex`).
        expected: SharedStateMode,
        /// 实际的模式 (e.g. `RwLock`).
        actual: SharedStateMode,
    },

    /// 模式在当前 organ 不支持 (防御: 编译期抓不到, 运行时 downcast 抓).
    Unsupported {
        /// 模式.
        mode: SharedStateMode,
        /// 受影响的 organ.
        organ: Organ,
        /// 不支持原因 (e.g. "Heart 仅 OnceLockState 支持, 当前请求 Mutex").
        reason: String,
    },

    /// 兜底 (catch-all).
    Other {
        /// 错误消息.
        msg: String,
    },
}

impl StateError {
    /// 序列化摘要 (跟 借鉴 Golutra `ErrorKind` 模式对齐, 跨层通信用).
    pub fn kind(&self) -> StateErrorKind {
        match self {
            Self::Poisoned { .. } => StateErrorKind::Poisoned,
            Self::NotInitialized { .. } => StateErrorKind::NotInitialized,
            Self::TypeMismatch { .. } => StateErrorKind::TypeMismatch,
            Self::Unsupported { .. } => StateErrorKind::Unsupported,
            Self::Other { .. } => StateErrorKind::Other,
        }
    }
}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Poisoned { mode, organ } => {
                write!(f, "[state poisoned] mode={mode:?}, organ={organ:?}")
            }
            Self::NotInitialized { mode, organ } => {
                write!(f, "[state not initialized] mode={mode:?}, organ={organ:?}")
            }
            Self::TypeMismatch { expected, actual } => {
                write!(f, "[state type mismatch] expected={expected:?}, actual={actual:?}")
            }
            Self::Unsupported { mode, organ, reason } => {
                write!(f, "[state unsupported] mode={mode:?}, organ={organ:?}, reason={reason}")
            }
            Self::Other { msg } => write!(f, "[state other] {msg}"),
        }
    }
}

impl std::error::Error for StateError {}

/// **StateError 序列化摘要** (跨层通信用, 不含具体 organ 字段以减小序列化体积).
///
/// per 借鉴 Golutra `tauri::Error` 序列化模式 (per 借鉴 #1 sister 报告 9 organ 错误风格).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateErrorKind {
    /// Mutex / RwLock 中毒.
    Poisoned,
    /// OnceLockState 还未 init.
    NotInitialized,
    /// SharedState 模式不匹配.
    TypeMismatch,
    /// 模式在当前 organ 不支持.
    Unsupported,
    /// 兜底.
    Other,
}

impl fmt::Display for StateErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl StateErrorKind {
    /// 编译期字符串表示.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Poisoned => "poisoned",
            Self::NotInitialized => "not_initialized",
            Self::TypeMismatch => "type_mismatch",
            Self::Unsupported => "unsupported",
            Self::Other => "other",
        }
    }
}

// =====================================================================
// 单元测试 (5 variant + kind 映射 + Display = 10+ 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_variant_constructible() {
        let _ = StateError::Poisoned {
            mode: SharedStateMode::Mutex,
            organ: Organ::Heart,
        };
        let _ = StateError::NotInitialized {
            mode: SharedStateMode::OnceLock,
            organ: Organ::Brain,
        };
        let _ = StateError::TypeMismatch {
            expected: SharedStateMode::Mutex,
            actual: SharedStateMode::RwLock,
        };
        let _ = StateError::Unsupported {
            mode: SharedStateMode::OnceLock,
            organ: Organ::Mind,
            reason: "test".to_string(),
        };
        let _ = StateError::Other { msg: "test".to_string() };
    }

    #[test]
    fn five_kind_variants_constructible() {
        let _ = StateErrorKind::Poisoned;
        let _ = StateErrorKind::NotInitialized;
        let _ = StateErrorKind::TypeMismatch;
        let _ = StateErrorKind::Unsupported;
        let _ = StateErrorKind::Other;
    }

    #[test]
    fn kind_mapping_consistent() {
        let poisoned = StateError::Poisoned {
            mode: SharedStateMode::Mutex,
            organ: Organ::Heart,
        };
        assert_eq!(poisoned.kind(), StateErrorKind::Poisoned);

        let not_init = StateError::NotInitialized {
            mode: SharedStateMode::OnceLock,
            organ: Organ::Brain,
        };
        assert_eq!(not_init.kind(), StateErrorKind::NotInitialized);

        let type_mismatch = StateError::TypeMismatch {
            expected: SharedStateMode::Mutex,
            actual: SharedStateMode::RwLock,
        };
        assert_eq!(type_mismatch.kind(), StateErrorKind::TypeMismatch);

        let unsupported = StateError::Unsupported {
            mode: SharedStateMode::OnceLock,
            organ: Organ::Mind,
            reason: "x".to_string(),
        };
        assert_eq!(unsupported.kind(), StateErrorKind::Unsupported);

        let other = StateError::Other { msg: "x".to_string() };
        assert_eq!(other.kind(), StateErrorKind::Other);
    }

    #[test]
    fn kind_as_str_5_distinct() {
        let s = [
            StateErrorKind::Poisoned.as_str(),
            StateErrorKind::NotInitialized.as_str(),
            StateErrorKind::TypeMismatch.as_str(),
            StateErrorKind::Unsupported.as_str(),
            StateErrorKind::Other.as_str(),
        ];
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 5, "5 variant 字符串应互不相同");
    }

    #[test]
    fn display_includes_organ_and_mode() {
        let e = StateError::Poisoned {
            mode: SharedStateMode::Mutex,
            organ: Organ::Heart,
        };
        let s = format!("{e}");
        assert!(s.contains("Heart"), "Display 应含 organ: {s}");
        assert!(s.contains("Mutex"), "Display 应含 mode: {s}");
    }

    #[test]
    fn variant_count_constant_matches_5() {
        assert_eq!(STATE_ERROR_VARIANT_COUNT, 5);
    }
}
