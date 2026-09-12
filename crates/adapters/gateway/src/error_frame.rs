//! Unified JSON error frame for every gateway error response.
//!
//! Every HTTP error the gateway produces uses one envelope:
//!
//! ```json
//! {"error": {"message": "<human>", "code": "<error_code>", "solution": "<action>"}}
//! ```
//!
//! The `message` field is the forward-compatible continuation of the legacy
//! flat `{"error": "..."}` string: clients that only read the human message can
//! switch from `error` to `error.message` without losing information. `code`
//! is the machine-readable contract and `solution` is an actionable Chinese
//! hint shown by the desktop companion.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// Stable machine-readable error codes.
///
/// This catalog is closed: the frontend and the desktop settings surface key
/// their recovery UX off these exact strings, so new codes are a contract
/// change, not a local convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// No API key has been configured.
    AuthMissingKey,
    /// The configured API key was rejected by the provider.
    AuthInvalidKey,
    /// The upstream provider could not be reached.
    ProviderUnreachable,
    /// The upstream provider answered with an error.
    ProviderError,
    /// The request parameters were invalid.
    InvalidRequest,
    /// The referenced session does not exist.
    SessionNotFound,
    /// The request was rate limited.
    RateLimited,
    /// An unexpected internal error.
    Internal,
}

impl ErrorCode {
    /// The wire representation used in the `code` field.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthMissingKey => "auth_missing_key",
            Self::AuthInvalidKey => "auth_invalid_key",
            Self::ProviderUnreachable => "provider_unreachable",
            Self::ProviderError => "provider_error",
            Self::InvalidRequest => "invalid_request",
            Self::SessionNotFound => "session_not_found",
            Self::RateLimited => "rate_limited",
            Self::Internal => "internal",
        }
    }

    /// The actionable Chinese hint attached to this code.
    pub fn solution(self) -> &'static str {
        match self {
            Self::AuthMissingKey => "在「设置 → 模型」中保存 API 密钥后重试",
            Self::AuthInvalidKey => "检查密钥是否过期或被撤销, 在设置中重新保存",
            Self::ProviderUnreachable => "检查 base_url 与网络连通性后重试",
            Self::ProviderError => "查看 message 中的上游信息, 必要时联系服务商",
            Self::InvalidRequest => "检查参数后重试",
            Self::SessionNotFound => "回到会话列表重新选择",
            Self::RateLimited => "稍后重试或降低请求频率",
            Self::Internal => "重启 Companion 后重试, 若复现请提交日志",
        }
    }
}

/// One error frame: human message + machine code + actionable hint.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorFrame {
    pub message: String,
    pub code: ErrorCode,
    pub solution: String,
}

impl ErrorFrame {
    /// Build a frame for `code`; `solution` is derived from the code.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code,
            solution: code.solution().to_string(),
        }
    }

    /// Build a complete HTTP error response (`{"error": {...}}`).
    pub fn response(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Response {
        (status, Json(ErrorEnvelope { error: Self::new(code, message) })).into_response()
    }
}

/// The HTTP body: `{"error": {...}}`.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub error: ErrorFrame,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_serializes_message_code_and_solution_in_contract_order() {
        let body = serde_json::to_value(ErrorEnvelope {
            error: ErrorFrame::new(ErrorCode::AuthMissingKey, "no key configured"),
        })
        .unwrap();

        assert_eq!(body["error"]["message"], "no key configured");
        assert_eq!(body["error"]["code"], "auth_missing_key");
        assert_eq!(
            body["error"]["solution"],
            "在「设置 → 模型」中保存 API 密钥后重试"
        );
        // The contract is a stable wire shape; assert the exact JSON.
        assert_eq!(
            serde_json::to_string(&ErrorEnvelope {
                error: ErrorFrame::new(ErrorCode::InvalidRequest, "bad input")
            })
            .unwrap(),
            r#"{"error":{"message":"bad input","code":"invalid_request","solution":"检查参数后重试"}}"#
        );
    }

    #[test]
    fn every_catalog_code_has_a_nonempty_solution() {
        let codes = [
            ErrorCode::AuthMissingKey,
            ErrorCode::AuthInvalidKey,
            ErrorCode::ProviderUnreachable,
            ErrorCode::ProviderError,
            ErrorCode::InvalidRequest,
            ErrorCode::SessionNotFound,
            ErrorCode::RateLimited,
            ErrorCode::Internal,
        ];
        for code in codes {
            assert!(!code.solution().is_empty(), "{code:?}");
            assert!(!code.as_str().is_empty(), "{code:?}");
        }
    }
}
