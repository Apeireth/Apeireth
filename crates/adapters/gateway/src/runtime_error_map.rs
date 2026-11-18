//! Runtime failure → HTTP status + stable error code.
//!
//! The mapping is a catalog of its own: every runtime failure shape the
//! gateway can surface has exactly one status/code pair, so a caller can act
//! on the code without reading the message.

use axum::http::StatusCode;

use apeireth_runtime::canonical::RuntimeError;

use crate::error_frame::ErrorCode;

/// Classify one runtime failure into a status and a stable error code.
pub(crate) fn classify_runtime_error(error: &RuntimeError) -> (StatusCode, ErrorCode) {
    match error {
        RuntimeError::Provider(provider) => (
            StatusCode::BAD_GATEWAY,
            provider_code(&provider.to_string()),
        ),
        RuntimeError::ProvidersExhausted { source, .. } => {
            (StatusCode::BAD_GATEWAY, provider_code(&source.to_string()))
        }
        RuntimeError::NoProvider { .. } | RuntimeError::NoHealthyProvider { .. } => (
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::ProviderUnreachable,
        ),
        RuntimeError::Misconfigured(_) => (StatusCode::SERVICE_UNAVAILABLE, ErrorCode::Internal),
        RuntimeError::Denied { .. } => (StatusCode::FORBIDDEN, ErrorCode::InvalidRequest),
        RuntimeError::ModuleStopped { .. } => {
            (StatusCode::UNPROCESSABLE_ENTITY, ErrorCode::ReviewRejected)
        }
        RuntimeError::RoundLimitExceeded { .. } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorCode::TurnNotConverged,
        ),
        RuntimeError::ApprovalRequired { .. } | RuntimeError::SessionApprovalPending { .. } => {
            (StatusCode::CONFLICT, ErrorCode::InvalidRequest)
        }
        // 回合闸拒绝了这一回合: 会话的 agent 绑定不再成立 ⇒ 409 + 专用码。
        RuntimeError::AgentBindingRejected { .. } => {
            (StatusCode::CONFLICT, ErrorCode::AgentBindingRejected)
        }
        // C2-1 操作幂等: 同键异载荷 / 上次执行中断 ⇒ 409 conflict; 同键在飞 ⇒ 409 in_progress。
        RuntimeError::OperationConflict { .. } => {
            (StatusCode::CONFLICT, ErrorCode::OperationConflict)
        }
        RuntimeError::OperationInProgress { .. } => {
            (StatusCode::CONFLICT, ErrorCode::OperationInProgress)
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::Internal),
    }
}

/// Classify a provider failure from its human-readable rendering.
///
/// The gateway crate has no regular dependency on `apeireth_plugin` (where
/// `ProviderError` is declared), so it cannot destructure the error type. It
/// reads the stable `Display` wording instead and preserves the full text in
/// the frame `message`.
fn provider_code(text: &str) -> ErrorCode {
    let lower = text.to_ascii_lowercase();
    if lower.contains("rate limited") {
        ErrorCode::RateLimited
    } else if lower.contains("timed out") || lower.contains("network error") {
        ErrorCode::ProviderUnreachable
    } else if lower.contains("authentication failed") {
        if lower.contains("missing") {
            ErrorCode::AuthMissingKey
        } else {
            ErrorCode::AuthInvalidKey
        }
    } else {
        ErrorCode::ProviderError
    }
}
