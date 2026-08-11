//! # OAuthError — 5 K-1 强校验 + 8 变体
//!
//! 5 K-1 强校验 (per 借鉴 Golutra #2 spec + sister #6 模式 1:1 镜像):
//! 1. `validate_client_id` — client_id 非空
//! 2. `validate_redirect_uri` — redirect_uri 非空 + http://localhost or https://
//! 3. `validate_scope` — scope 数组非空 + 元素非空
//! 4. `validate_pkce_verifier` — code_verifier 43-128 字符 + base64url charset (RFC 7636 §4.1)
//! 5. `validate_state` — state 非空 (CSRF 防御, RFC 6749 §10.12)
//!
//! 8 变体 OAuthError: 5 K-1 强校验变体 + 3 utility (NotImplemented / TokenExchange / Other)
//!
//! ## 借用 Golutra
//!
//! 借鉴 Golutra v0.1.0 OAuth 错误模式 (InvalidClient / InvalidGrant / InvalidRequest / ...),
//! 简化到 5+3 通用 8 类, 适配 Rust 路线.

use std::fmt;

use thiserror::Error;

/// **K-1 强校验 #1-5**: 5 K-1 强校验变体 (编译期 hardcode, 跟 sister #6 1:1 镜像).
pub const K1_STRONG_VALIDATION_VARIANTS: [&str; 5] = [
    "EmptyClientId",
    "EmptyRedirectUri",
    "EmptyScope",
    "InvalidPkceVerifier",
    "EmptyState",
];

/// OAuthError 变体总数 (5 K-1 + 3 utility).
pub const OAUTH_ERROR_VARIANT_COUNT: usize = 8;

/// OAuth 错误 (5 K-1 + 3 utility, 跨模块统一包装, 错误传播支持 `?`).
#[derive(Debug, Clone, Error)]
pub enum OAuthError {
    /// K-1 #1: client_id 为空.
    #[error("[oauth empty client_id] client_id cannot be empty")]
    EmptyClientId,

    /// K-1 #2: redirect_uri 为空或非 http(s).
    #[error("[oauth empty redirect_uri] redirect_uri cannot be empty and must be http:// or https://")]
    EmptyRedirectUri,

    /// K-1 #3: scope 为空或元素为空.
    #[error("[oauth empty scope] scope must contain at least one non-empty entry")]
    EmptyScope,

    /// K-1 #4: PKCE code_verifier 长度或字符不合法.
    #[error(
        "[oauth invalid pkce_verifier] must be 43-128 chars, base64url charset (RFC 7636 §4.1)"
    )]
    InvalidPkceVerifier,

    /// K-1 #5: state 为空 (CSRF 防御).
    #[error("[oauth empty state] state cannot be empty (RFC 6749 §10.12)")]
    EmptyState,

    /// utility #1: R21+ 续真接时, token exchange 失败 (HTTP error / 4xx / 5xx).
    #[error("[oauth token exchange error] {0}")]
    TokenExchange(String),

    /// utility #2: R21+ 续真接时, OAuth flow 内部状态不合法.
    #[error("[oauth invalid state] {0}")]
    InvalidState(String),

    /// utility #3: 兜底 (catch-all).
    #[error("[oauth other] {0}")]
    Other(String),
}

impl OAuthError {
    /// K-1 强校验变体 (5 个), 用于跨层通信的"5 大类"摘要.
    #[allow(dead_code)] // R21+ 真接时跨层通信用
    pub fn kind(&self) -> OAuthErrorKind {
        match self {
            Self::EmptyClientId => OAuthErrorKind::EmptyClientId,
            Self::EmptyRedirectUri => OAuthErrorKind::EmptyRedirectUri,
            Self::EmptyScope => OAuthErrorKind::EmptyScope,
            Self::InvalidPkceVerifier => OAuthErrorKind::InvalidPkceVerifier,
            Self::EmptyState => OAuthErrorKind::EmptyState,
            Self::TokenExchange(_) => OAuthErrorKind::TokenExchange,
            Self::InvalidState(_) => OAuthErrorKind::InvalidState,
            Self::Other(_) => OAuthErrorKind::Other,
        }
    }
}

/// OAuth 错误序列化摘要 (跨层通信用, 不含具体 message 以减小体积).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OAuthErrorKind {
    /// K-1 #1
    EmptyClientId,
    /// K-1 #2
    EmptyRedirectUri,
    /// K-1 #3
    EmptyScope,
    /// K-1 #4
    InvalidPkceVerifier,
    /// K-1 #5
    EmptyState,
    /// utility #1
    TokenExchange,
    /// utility #2
    InvalidState,
    /// utility #3
    Other,
}

impl fmt::Display for OAuthErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl OAuthErrorKind {
    /// 编译期字符串表示.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EmptyClientId => "empty_client_id",
            Self::EmptyRedirectUri => "empty_redirect_uri",
            Self::EmptyScope => "empty_scope",
            Self::InvalidPkceVerifier => "invalid_pkce_verifier",
            Self::EmptyState => "empty_state",
            Self::TokenExchange => "token_exchange",
            Self::InvalidState => "invalid_state",
            Self::Other => "other",
        }
    }
}

/// OAuth 错误结果类型 (per sister #6 模式 1:1).
pub type OAuthResult<T> = Result<T, OAuthError>;

// ============================================================================
// 5 K-1 强校验函数 (per 任务 spec + sister #6 1:1 镜像)
// ============================================================================

/// K-1 #1: 校验 client_id 非空.
pub fn validate_client_id(client_id: &str) -> OAuthResult<()> {
    if client_id.trim().is_empty() {
        return Err(OAuthError::EmptyClientId);
    }
    Ok(())
}

/// K-1 #2: 校验 redirect_uri 非空 + http://localhost 或 https:// 开头.
///
/// per RFC 6749 §3.1.2 + RFC 8252 §7.3 (native apps 推荐 `http://localhost:port`).
pub fn validate_redirect_uri(redirect_uri: &str) -> OAuthResult<()> {
    if redirect_uri.trim().is_empty() {
        return Err(OAuthError::EmptyRedirectUri);
    }
    if !redirect_uri.starts_with("https://")
        && !redirect_uri.starts_with("http://localhost")
    {
        return Err(OAuthError::EmptyRedirectUri);
    }
    Ok(())
}

/// K-1 #3: 校验 scope 数组非空 + 元素非空.
///
/// per RFC 6749 §3.3 (scope 是 space-separated string, 至少 1 字符).
pub fn validate_scope(scope: &[&str]) -> OAuthResult<()> {
    if scope.is_empty() {
        return Err(OAuthError::EmptyScope);
    }
    for s in scope {
        if s.trim().is_empty() {
            return Err(OAuthError::EmptyScope);
        }
    }
    Ok(())
}

/// K-1 #4: 校验 PKCE code_verifier 长度 + base64url charset.
///
/// per RFC 7636 §4.1: code_verifier = high-entropy cryptographic random STRING
/// using the unreserved characters `[A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"`,
/// with a minimum length of 43 characters and a maximum length of 128 characters.
pub fn validate_pkce_verifier(code_verifier: &str) -> OAuthResult<()> {
    const MIN: usize = 43;
    const MAX: usize = 128;
    let len = code_verifier.len();
    if len < MIN || len > MAX {
        return Err(OAuthError::InvalidPkceVerifier);
    }
    // base64url charset (RFC 7636 §4.1 unreserved chars, NO '+' '/' '=')
    for c in code_verifier.chars() {
        if !is_unreserved(c) {
            return Err(OAuthError::InvalidPkceVerifier);
        }
    }
    Ok(())
}

/// K-1 #5: 校验 state 非空 (CSRF 防御).
///
/// per RFC 6749 §10.12: state 必传非空, 用于 cross-request 校验.
pub fn validate_state(state: &str) -> OAuthResult<()> {
    if state.trim().is_empty() {
        return Err(OAuthError::EmptyState);
    }
    Ok(())
}

/// RFC 7636 §4.1 unreserved chars: `[A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"`.
const fn is_unreserved(c: char) -> bool {
    matches!(c,
        'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~'
    )
}

// ============================================================================
// 单元测试 (8 variant + kind + 5 K-1 = 25 测试)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_variant_constructible() {
        let _ = OAuthError::EmptyClientId;
        let _ = OAuthError::EmptyRedirectUri;
        let _ = OAuthError::EmptyScope;
        let _ = OAuthError::InvalidPkceVerifier;
        let _ = OAuthError::EmptyState;
        let _ = OAuthError::TokenExchange("test".to_string());
        let _ = OAuthError::InvalidState("test".to_string());
        let _ = OAuthError::Other("test".to_string());
    }

    #[test]
    fn eight_kind_variants_constructible() {
        let _ = OAuthErrorKind::EmptyClientId;
        let _ = OAuthErrorKind::EmptyRedirectUri;
        let _ = OAuthErrorKind::EmptyScope;
        let _ = OAuthErrorKind::InvalidPkceVerifier;
        let _ = OAuthErrorKind::EmptyState;
        let _ = OAuthErrorKind::TokenExchange;
        let _ = OAuthErrorKind::InvalidState;
        let _ = OAuthErrorKind::Other;
    }

    #[test]
    fn kind_mapping_consistent() {
        let e = OAuthError::EmptyClientId;
        assert_eq!(e.kind(), OAuthErrorKind::EmptyClientId);
        let e = OAuthError::TokenExchange("x".to_string());
        assert_eq!(e.kind(), OAuthErrorKind::TokenExchange);
        let e = OAuthError::Other("x".to_string());
        assert_eq!(e.kind(), OAuthErrorKind::Other);
    }

    #[test]
    fn kind_as_str_8_distinct() {
        let s = [
            OAuthErrorKind::EmptyClientId.as_str(),
            OAuthErrorKind::EmptyRedirectUri.as_str(),
            OAuthErrorKind::EmptyScope.as_str(),
            OAuthErrorKind::InvalidPkceVerifier.as_str(),
            OAuthErrorKind::EmptyState.as_str(),
            OAuthErrorKind::TokenExchange.as_str(),
            OAuthErrorKind::InvalidState.as_str(),
            OAuthErrorKind::Other.as_str(),
        ];
        let unique: std::collections::HashSet<&str> = s.iter().copied().collect();
        assert_eq!(unique.len(), 8);
    }

    #[test]
    fn variant_count_constant_matches_8() {
        assert_eq!(OAUTH_ERROR_VARIANT_COUNT, 8);
    }

    #[test]
    fn k1_variants_constant_matches_5() {
        assert_eq!(K1_STRONG_VALIDATION_VARIANTS.len(), 5);
    }

    // K-1 #1: client_id
    #[test]
    fn validate_client_id_accepts_nonempty() {
        assert!(validate_client_id("client_abc123").is_ok());
        assert!(validate_client_id("x").is_ok());
    }

    #[test]
    fn validate_client_id_rejects_empty() {
        assert!(validate_client_id("").is_err());
        assert!(validate_client_id("   ").is_err());
        assert!(validate_client_id("\t\n").is_err());
    }

    // K-1 #2: redirect_uri
    #[test]
    fn validate_redirect_uri_accepts_https() {
        assert!(validate_redirect_uri("https://example.com/cb").is_ok());
        assert!(validate_redirect_uri("https://app.example.com/oauth/callback").is_ok());
    }

    #[test]
    fn validate_redirect_uri_accepts_localhost() {
        assert!(validate_redirect_uri("http://localhost:8080/cb").is_ok());
        assert!(validate_redirect_uri("http://localhost:53682/cb").is_ok());
    }

    #[test]
    fn validate_redirect_uri_rejects_empty() {
        assert!(validate_redirect_uri("").is_err());
        assert!(validate_redirect_uri("   ").is_err());
    }

    #[test]
    fn validate_redirect_uri_rejects_http_non_localhost() {
        assert!(validate_redirect_uri("http://example.com/cb").is_err());
        assert!(validate_redirect_uri("ftp://example.com/cb").is_err());
        assert!(validate_redirect_uri("javascript:alert(1)").is_err());
    }

    // K-1 #3: scope
    #[test]
    fn validate_scope_accepts_nonempty() {
        assert!(validate_scope(&["read"]).is_ok());
        assert!(validate_scope(&["read", "write", "admin"]).is_ok());
    }

    #[test]
    fn validate_scope_rejects_empty_array() {
        assert!(validate_scope(&[]).is_err());
    }

    #[test]
    fn validate_scope_rejects_empty_element() {
        assert!(validate_scope(&["read", ""]).is_err());
        assert!(validate_scope(&["", "write"]).is_err());
        assert!(validate_scope(&["   "]).is_err());
    }

    // K-1 #4: pkce_verifier
    #[test]
    fn validate_pkce_verifier_accepts_43_to_128() {
        // 43 chars minimum
        let v43 = "a".repeat(43);
        assert!(validate_pkce_verifier(&v43).is_ok());
        // 128 chars maximum
        let v128 = "a".repeat(128);
        assert!(validate_pkce_verifier(&v128).is_ok());
    }

    #[test]
    fn validate_pkce_verifier_rejects_too_short() {
        let v42 = "a".repeat(42);
        assert!(validate_pkce_verifier(&v42).is_err());
        let v0 = "";
        assert!(validate_pkce_verifier(v0).is_err());
    }

    #[test]
    fn validate_pkce_verifier_rejects_too_long() {
        let v129 = "a".repeat(129);
        assert!(validate_pkce_verifier(&v129).is_err());
        let v1000 = "a".repeat(1000);
        assert!(validate_pkce_verifier(&v1000).is_err());
    }

    #[test]
    fn validate_pkce_verifier_accepts_base64url_charset() {
        // RFC 7636 §4.1 unreserved chars: [A-Z] [a-z] [0-9] - . _ ~
        let valid = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnop_-.~0123456789";
        assert!(validate_pkce_verifier(valid).is_ok());
    }

    #[test]
    fn validate_pkce_verifier_rejects_non_unreserved() {
        // '+' '/' '=' 是 base64 但非 unreserved
        let bad = "a".repeat(42) + "+";
        assert!(validate_pkce_verifier(&bad).is_err());
        let bad = "a".repeat(42) + "/";
        assert!(validate_pkce_verifier(&bad).is_err());
        let bad = "a".repeat(42) + "=";
        assert!(validate_pkce_verifier(&bad).is_err());
        let bad = "a".repeat(42) + " ";
        assert!(validate_pkce_verifier(&bad).is_err());
        let bad = "a".repeat(42) + "!";
        assert!(validate_pkce_verifier(&bad).is_err());
    }

    // K-1 #5: state
    #[test]
    fn validate_state_accepts_nonempty() {
        assert!(validate_state("abc123").is_ok());
        assert!(validate_state("random_base64url_state_xyz").is_ok());
    }

    #[test]
    fn validate_state_rejects_empty() {
        assert!(validate_state("").is_err());
        assert!(validate_state("   ").is_err());
    }

    // is_unreserved helper
    #[test]
    fn is_unreserved_helper_correct() {
        assert!(is_unreserved('a'));
        assert!(is_unreserved('Z'));
        assert!(is_unreserved('5'));
        assert!(is_unreserved('-'));
        assert!(is_unreserved('.'));
        assert!(is_unreserved('_'));
        assert!(is_unreserved('~'));
        assert!(!is_unreserved('+'));
        assert!(!is_unreserved('/'));
        assert!(!is_unreserved('='));
        assert!(!is_unreserved(' '));
        assert!(!is_unreserved('!'));
    }

    // Display
    #[test]
    fn display_includes_message_for_utility_variants() {
        let e = OAuthError::TokenExchange("HTTP 500".to_string());
        let s = format!("{e}");
        assert!(s.contains("HTTP 500"), "Display should include message: {s}");
    }

    #[test]
    fn display_works_for_all_8_variants() {
        // 编译期守门: 8 variant 都能 Display
        let _ = format!("{}", OAuthError::EmptyClientId);
        let _ = format!("{}", OAuthError::EmptyRedirectUri);
        let _ = format!("{}", OAuthError::EmptyScope);
        let _ = format!("{}", OAuthError::InvalidPkceVerifier);
        let _ = format!("{}", OAuthError::EmptyState);
        let _ = format!("{}", OAuthError::TokenExchange("x".to_string()));
        let _ = format!("{}", OAuthError::InvalidState("x".to_string()));
        let _ = format!("{}", OAuthError::Other("x".to_string()));
    }
}
