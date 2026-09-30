//! IM 快捷接入: 错误面 (闭合分类词表, 事实错误 + 可重试性)。
//!
//! 口径与 [`crate::lark::error`] 一致: 错误只报事实类别, 0 回显共享秘密 /
//! 0 静默吞错。分类闭合 (6 variant), [`ImError::class`] 一处穷举。

use serde::{Deserialize, Serialize};

/// IM 快捷接入的事实错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImError {
    /// 渠道配置结构/校验不过 (加载期 fail-closed)。
    #[error("im channel configuration is invalid: {reason}")]
    InvalidConfig {
        /// 具体原因。
        reason: String,
    },
    /// 该协议面显式不支持 (0 占位桩, 0 静默放行)。
    #[error("im adapter aspect not supported: {aspect}")]
    Unsupported {
        /// 不支持的协议面标识。
        aspect: &'static str,
    },
    /// 传输层失败 (可重试: 连接/超时/限流)。
    #[error("im transport failure: {reason}")]
    Transport {
        /// 具体原因 (不含秘密)。
        reason: String,
    },
    /// 对端明确拒绝 (永久错误, 重试无意义)。
    #[error("im peer rejected the request: {reason}")]
    Rejected {
        /// 具体原因 (不含秘密)。
        reason: String,
    },
    /// 入站信封无法解码 (形状/字段不符)。
    #[error("im inbound payload could not be decoded: {reason}")]
    Decode {
        /// 具体原因。
        reason: String,
    },
    /// 入站签名校验失败 (0 回显共享秘密)。
    #[error("im inbound payload failed verification: {reason}")]
    Verify {
        /// 只报事实类别。
        reason: String,
    },
}

/// 错误分类 (闭合枚举)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImErrorClass {
    /// 配置缺陷。
    Config,
    /// 显式不支持的协议面。
    Unsupported,
    /// 可重试的传输失败。
    Transport,
    /// 永久拒绝。
    Rejected,
    /// 入站解码失败。
    Decode,
    /// 校验失败。
    Verify,
}

impl ImErrorClass {
    /// 分类变体数守门 (6)。
    pub const COUNT: usize = 6;
}

impl ImError {
    /// 闭合分类: 一 variant 一 arm, 新错误必须先分类。
    pub const fn class(&self) -> ImErrorClass {
        match self {
            Self::InvalidConfig { .. } => ImErrorClass::Config,
            Self::Unsupported { .. } => ImErrorClass::Unsupported,
            Self::Transport { .. } => ImErrorClass::Transport,
            Self::Rejected { .. } => ImErrorClass::Rejected,
            Self::Decode { .. } => ImErrorClass::Decode,
            Self::Verify { .. } => ImErrorClass::Verify,
        }
    }

    /// 只有传输失败值得重试; 其余 (配置/拒绝/解码/校验) 重试无意义。
    pub const fn is_retryable(&self) -> bool {
        matches!(self.class(), ImErrorClass::Transport)
    }
}

/// IM 快捷接入结果别名。
pub type ImResult<T> = Result<T, ImError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_classes_are_closed_and_retryable_only_for_transport() {
        let cases = [
            (
                ImError::InvalidConfig { reason: "x".into() },
                ImErrorClass::Config,
            ),
            (
                ImError::Unsupported { aspect: "x" },
                ImErrorClass::Unsupported,
            ),
            (
                ImError::Transport { reason: "x".into() },
                ImErrorClass::Transport,
            ),
            (
                ImError::Rejected { reason: "x".into() },
                ImErrorClass::Rejected,
            ),
            (ImError::Decode { reason: "x".into() }, ImErrorClass::Decode),
            (
                ImError::Verify {
                    reason: "mismatch".into(),
                },
                ImErrorClass::Verify,
            ),
        ];
        for (error, class) in cases {
            assert_eq!(error.class(), class);
            assert_eq!(error.is_retryable(), class == ImErrorClass::Transport);
        }
        assert_eq!(ImErrorClass::COUNT, 6);
    }

    #[test]
    fn verify_error_never_echoes_the_secret() {
        let error = ImError::Verify {
            reason: "signature mismatch".to_string(),
        };
        let rendered = error.to_string();
        assert!(rendered.contains("mismatch"), "{rendered}");
        assert!(!rendered.contains("secret-value"), "{rendered}");
    }
}
