//! Error-code string catalog and frame-construction helpers.
//!
//! These wire-level strings are the closed contract the frontend keys its
//! recovery UX off. [`crate::error_frame::ErrorCode`] is the typed
//! representation; this module keeps the strings and the small builders used
//! across handlers.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::error_frame::{ErrorCode, ErrorEnvelope, ErrorFrame};

pub const AUTH_MISSING_KEY: &str = "auth_missing_key";
pub const AUTH_INVALID_KEY: &str = "auth_invalid_key";
pub const PROVIDER_UNREACHABLE: &str = "provider_unreachable";
pub const PROVIDER_ERROR: &str = "provider_error";
pub const INVALID_REQUEST: &str = "invalid_request";
pub const PERMISSION_DENIED: &str = "permission_denied";
pub const DOMAIN_FORBIDDEN: &str = "domain_forbidden";
pub const DOMAIN_REFUSED: &str = "domain_refused";
pub const SESSION_NOT_FOUND: &str = "session_not_found";
pub const RATE_LIMITED: &str = "rate_limited";
pub const REVIEW_REJECTED: &str = "review_rejected";
pub const TURN_NOT_CONVERGED: &str = "turn_not_converged";
// ↓ 以下 11 个常量是 C2-0 批次补齐的历史欠账 + 本批新登记的码。
// 补齐的 9 个（A2 工作区面 3 个 / B3a 联系人面 4 个 / B3b 会话绑定面 2 个）此前只存在于
// `error_frame.rs` 的枚举里 ⇒ 本目录与 `parse_code()` 对它们一律返回「不认识」。
// 新登记的 2 个（C2-0 操作面）本批只进目录与文案, 尚无端点产生 ⇒ 见 error_frame.rs 同名变体的说明。
/// 工作区不存在或对调用方不可见 (A2 合同面)。
pub const WORKSPACE_NOT_FOUND: &str = "workspace_not_found";
/// 工作区修订号冲突 (A2 合同面)。
pub const WORKSPACE_CONFLICT: &str = "workspace_conflict";
/// 工作区已吊销 (A2 合同面)。
pub const WORKSPACE_REVOKED: &str = "workspace_revoked";
/// 联系人不存在或对调用方不可见 (B3a 合同面)。
pub const CONTACT_NOT_FOUND: &str = "contact_not_found";
/// 联系人修订号冲突 (B3a 合同面)。
pub const CONTACT_CONFLICT: &str = "contact_conflict";
/// 联系人已归档 (B3a 合同面)。
pub const CONTACT_ARCHIVED: &str = "contact_archived";
/// 工作区绑定冲突 (B3a 合同面)。
pub const CONTACT_BINDING_CONFLICT: &str = "contact_binding_conflict";
/// 会话绑定被拒 (B3b 合同面 / 回合闸)。
pub const AGENT_BINDING_REJECTED: &str = "agent_binding_rejected";
/// 会话版本冲突 (B3b 合同面)。
pub const SESSION_BINDING_CONFLICT: &str = "session_binding_conflict";
/// 操作幂等键载荷冲突 (C2-0 目录登记; 业务未接线)。
pub const OPERATION_CONFLICT: &str = "operation_conflict";
/// 操作幂等键在飞 (C2-0 目录登记; 业务未接线)。
pub const OPERATION_IN_PROGRESS: &str = "operation_in_progress";
pub const INTERNAL: &str = "internal";

/// Resolve a wire code string to its typed [`ErrorCode`].
pub fn parse_code(code: &str) -> Option<ErrorCode> {
    match code {
        AUTH_MISSING_KEY => Some(ErrorCode::AuthMissingKey),
        AUTH_INVALID_KEY => Some(ErrorCode::AuthInvalidKey),
        PROVIDER_UNREACHABLE => Some(ErrorCode::ProviderUnreachable),
        PROVIDER_ERROR => Some(ErrorCode::ProviderError),
        INVALID_REQUEST => Some(ErrorCode::InvalidRequest),
        PERMISSION_DENIED => Some(ErrorCode::PermissionDenied),
        DOMAIN_FORBIDDEN => Some(ErrorCode::DomainForbidden),
        DOMAIN_REFUSED => Some(ErrorCode::DomainRefused),
        SESSION_NOT_FOUND => Some(ErrorCode::SessionNotFound),
        RATE_LIMITED => Some(ErrorCode::RateLimited),
        REVIEW_REJECTED => Some(ErrorCode::ReviewRejected),
        TURN_NOT_CONVERGED => Some(ErrorCode::TurnNotConverged),
        WORKSPACE_NOT_FOUND => Some(ErrorCode::WorkspaceNotFound),
        WORKSPACE_CONFLICT => Some(ErrorCode::WorkspaceConflict),
        WORKSPACE_REVOKED => Some(ErrorCode::WorkspaceRevoked),
        CONTACT_NOT_FOUND => Some(ErrorCode::ContactNotFound),
        CONTACT_CONFLICT => Some(ErrorCode::ContactConflict),
        CONTACT_ARCHIVED => Some(ErrorCode::ContactArchived),
        CONTACT_BINDING_CONFLICT => Some(ErrorCode::ContactBindingConflict),
        AGENT_BINDING_REJECTED => Some(ErrorCode::AgentBindingRejected),
        SESSION_BINDING_CONFLICT => Some(ErrorCode::SessionBindingConflict),
        OPERATION_CONFLICT => Some(ErrorCode::OperationConflict),
        OPERATION_IN_PROGRESS => Some(ErrorCode::OperationInProgress),
        INTERNAL => Some(ErrorCode::Internal),
        _ => None,
    }
}

/// Build an [`ErrorFrame`] for a typed code and a human message.
pub fn frame(code: ErrorCode, message: impl Into<String>) -> ErrorFrame {
    ErrorFrame::new(code, message)
}

/// Build a complete HTTP error response (`{"error": {...}}`).
pub fn error_response(status: StatusCode, code: ErrorCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(ErrorEnvelope {
            error: frame(code, message),
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 常量目录（顺序 = `error_frame::ErrorCode` 枚举声明顺序）必须逐条能反解回
    /// 同一变体 ⇒ 目录漏项 / 错位 / `parse_code` 漏臂都会被这条测试抓住。
    #[test]
    fn every_catalog_constant_round_trips_through_parse_code() {
        let codes = [
            AUTH_MISSING_KEY,
            AUTH_INVALID_KEY,
            PROVIDER_UNREACHABLE,
            PROVIDER_ERROR,
            INVALID_REQUEST,
            PERMISSION_DENIED,
            DOMAIN_FORBIDDEN,
            DOMAIN_REFUSED,
            SESSION_NOT_FOUND,
            RATE_LIMITED,
            REVIEW_REJECTED,
            TURN_NOT_CONVERGED,
            WORKSPACE_NOT_FOUND,
            WORKSPACE_CONFLICT,
            WORKSPACE_REVOKED,
            CONTACT_NOT_FOUND,
            CONTACT_CONFLICT,
            CONTACT_ARCHIVED,
            CONTACT_BINDING_CONFLICT,
            AGENT_BINDING_REJECTED,
            SESSION_BINDING_CONFLICT,
            OPERATION_CONFLICT,
            OPERATION_IN_PROGRESS,
            INTERNAL,
        ];
        assert_eq!(codes.len(), 24, "常量目录必须覆盖 24 个错误码");
        for code in codes {
            assert_eq!(parse_code(code).unwrap().as_str(), code);
        }
        assert!(parse_code("not_a_code").is_none());
    }
}
