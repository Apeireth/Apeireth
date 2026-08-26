//! **TP3/N21 / 错误类型 — 错误消息脱敏红线**
//!
//! 所有变体的 `Display` **只含服务名 / 路径等元信息**, 绝不携带凭据明文。
//! `SecretString` 自身覆写了 `Debug`/`Display`, 即便误入错误链也只出脱敏串。

use thiserror::Error;

/// 凭据存取错误。
///
/// **红线**: 任一 `Display` 输出不得包含明文 (只含服务名/路径/长度元信息)。
#[derive(Debug, Error)]
pub enum CredentialsError {
    /// 未知名 (服务不存在) — 验收项"未知名报错"。
    #[error("credentials: unknown service `{0}`")]
    UnknownService(String),

    /// 服务名非法 (含路径分隔符/空名等, 防路径穿越)。
    #[error("credentials: invalid service name `{0}`")]
    InvalidServiceName(String),

    /// 高危凭据未获审批 (权限洋葱衔接, 见 crate::gate)。
    #[error("credentials: high-risk service `{service}` requires approval (operation denied)")]
    ApprovalRequired {
        /// 目标服务名 (元信息, 非明文)。
        service: String,
    },

    /// 底层 IO 错误 (路径为元信息; 明文不落错误消息)。
    #[error("credentials: io error for service `{service}`: {source}")]
    Io {
        /// 目标服务名。
        service: String,
        #[source]
        source: std::io::Error,
    },

    /// 序列化/反序列化错误 (存储格式解析)。
    #[error("credentials: storage format error for service `{service}`: {message}")]
    Format {
        /// 目标服务名。
        service: String,
        /// 解析错误描述 (serde 消息, 不含明文)。
        message: String,
    },
}

/// 便捷 Result 别名。
pub type Result<T> = std::result::Result<T, CredentialsError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_service_message_has_no_secret() {
        let e = CredentialsError::UnknownService("openai".into());
        let msg = format!("{e}");
        assert!(msg.contains("openai"));
        assert!(!msg.contains("sk-"), "错误消息不得含明文样例");
    }

    #[test]
    fn approval_required_message_only_service_name() {
        let e = CredentialsError::ApprovalRequired {
            service: "master".into(),
        };
        let msg = format!("{e}");
        assert!(msg.contains("master"));
        assert!(msg.contains("approval"));
    }

    #[test]
    fn io_error_carries_service_not_secret() {
        let e = CredentialsError::Io {
            service: "github".into(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file gone"),
        };
        let msg = format!("{e}");
        assert!(msg.contains("github"));
    }
}
