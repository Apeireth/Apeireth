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
pub const SESSION_NOT_FOUND: &str = "session_not_found";
pub const RATE_LIMITED: &str = "rate_limited";
pub const INTERNAL: &str = "internal";

/// Resolve a wire code string to its typed [`ErrorCode`].
pub fn parse_code(code: &str) -> Option<ErrorCode> {
    match code {
        AUTH_MISSING_KEY => Some(ErrorCode::AuthMissingKey),
        AUTH_INVALID_KEY => Some(ErrorCode::AuthInvalidKey),
        PROVIDER_UNREACHABLE => Some(ErrorCode::ProviderUnreachable),
        PROVIDER_ERROR => Some(ErrorCode::ProviderError),
        INVALID_REQUEST => Some(ErrorCode::InvalidRequest),
        SESSION_NOT_FOUND => Some(ErrorCode::SessionNotFound),
        RATE_LIMITED => Some(ErrorCode::RateLimited),
        INTERNAL => Some(ErrorCode::Internal),
        _ => None,
    }
}

/// Build an [`ErrorFrame`] for a typed code and a human message.
pub fn frame(code: ErrorCode, message: impl Into<String>) -> ErrorFrame {
    ErrorFrame::new(code, message)
}

/// Build a complete HTTP error response (`{"error": {...}}`).
pub fn error_response(
    status: StatusCode,
    code: ErrorCode,
    message: impl Into<String>,
) -> Response {
    (status, Json(ErrorEnvelope { error: frame(code, message) })).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_catalog_constant_round_trips_through_parse_code() {
        let codes = [
            AUTH_MISSING_KEY,
            AUTH_INVALID_KEY,
            PROVIDER_UNREACHABLE,
            PROVIDER_ERROR,
            INVALID_REQUEST,
            SESSION_NOT_FOUND,
            RATE_LIMITED,
            INTERNAL,
        ];
        for code in codes {
            assert_eq!(parse_code(code).unwrap().as_str(), code);
        }
        assert!(parse_code("not_a_code").is_none());
    }
}
