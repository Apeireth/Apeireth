//! apeireth-pybridge 错误类型
//!
//! 主 11:51 不要二极管：4 路径都允许 (成功/重试/降级/失败)，不强制非黑即白。

use thiserror::Error;

/// apeireth-pybridge 错误
#[derive(Debug, Error)]
pub enum BridgeError {
    /// Python 模块未找到
    #[error("Python module not found: {0}")]
    ModuleNotFound(String),
    /// Python 函数调用失败
    #[error("Python call failed: {0}")]
    CallFailed(String),
    /// GIL 获取失败
    #[error("Python GIL error: {0}")]
    GilError(String),
    /// 无效参数
    #[error("Invalid argument: {0}")]
    InvalidArg(String),
}

impl BridgeError {
    /// 是否可恢复 (可重试或降级)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::CallFailed(_) | Self::GilError(_))
    }

    /// 推荐处置路径 (主 11:51 不要二极管)
    pub fn suggested_action(&self) -> SuggestedAction {
        match self {
            Self::ModuleNotFound(_) => SuggestedAction::Degrade,
            Self::CallFailed(_) => SuggestedAction::Retry,
            Self::GilError(_) => SuggestedAction::Retry,
            Self::InvalidArg(_) => SuggestedAction::Fail,
        }
    }
}

/// 推荐处置路径
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestedAction {
    /// 重试 (临时错误)
    Retry,
    /// 降级到 Rust 实现
    Degrade,
    /// 失败 (硬错误)
    Fail,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoverable_classification() {
        assert!(BridgeError::GilError("x".into()).is_recoverable());
        assert!(BridgeError::CallFailed("x".into()).is_recoverable());
        assert!(!BridgeError::ModuleNotFound("x".into()).is_recoverable());
        assert!(!BridgeError::InvalidArg("x".into()).is_recoverable());
    }

    #[test]
    fn suggested_action_paths() {
        assert_eq!(
            BridgeError::ModuleNotFound("x".into()).suggested_action(),
            SuggestedAction::Degrade
        );
        assert_eq!(
            BridgeError::CallFailed("x".into()).suggested_action(),
            SuggestedAction::Retry
        );
        assert_eq!(
            BridgeError::InvalidArg("x".into()).suggested_action(),
            SuggestedAction::Fail
        );
    }

    #[test]
    fn display_contains_module_name() {
        let e = BridgeError::ModuleNotFound("apeireth.memory.store".into());
        let msg = format!("{e}");
        assert!(msg.contains("apeireth.memory.store"));
    }
}
