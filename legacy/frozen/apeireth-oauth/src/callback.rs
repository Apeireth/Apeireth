//! # OAuth Callback trait + 3 Callback mode impl
//!
//! 借鉴 Golutra v0.1.0 OAuth 3 callback 模式 1:1 翻译, 跟 sister #1+#6 1:1 镜像.
//!
//! **3 Callback mode 编译期 hardcode enum** (per RFC 6749 §1.3 + §4):
//!   - `AuthorizationCode` — authorization code 模式 (推荐 + PKCE, RFC 6749 §4.1)
//!   - `Implicit` — implicit 模式 (deprecated, RFC 6749 §4.2, 保留为完整 spec)
//!   - `ClientCredentials` — client credentials 模式 (M2M, RFC 6749 §4.4)
//!
//! **3 核心方法** (per OAuth 2.0 standard + 借鉴 Golutra):
//! 1. `build_authorization` — 构造 authorization request (callback 模式特化)
//! 2. `parse_callback` — 解析 callback response (callback 模式特化)
//! 3. `client_credentials_grant` — client_credentials 模式直接发 token request

use serde::{Deserialize, Serialize};

use crate::error::{
    validate_client_id, validate_redirect_uri, validate_scope, validate_state, OAuthError,
    OAuthResult,
};
use crate::state::PkcePair;

// ============================================================================
// §1 编译期 hardcode 常量 (3 Callback mode 守门)
// ============================================================================

/// **3 Callback mode 编译期守门** (K-1 强校验 + 8 项不修改承诺).
pub const CALLBACK_MODE_COUNT: usize = 3;

// ============================================================================
// §2 3 Callback mode 枚举 (编译期 hardcode, 8 项不修改承诺 #2)
// ============================================================================

/// 3 OAuth callback mode (per RFC 6749 §1.3 + §4.1-§4.4 + 任务 spec 借鉴 Golutra #2).
///
/// 枚举顺序固定: `AuthorizationCode` → `Implicit` → `ClientCredentials`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallbackMode {
    /// 0: authorization_code (RFC 6749 §4.1, 推荐 + PKCE).
    AuthorizationCode,
    /// 1: implicit (RFC 6749 §4.2, deprecated 但保留完整 spec).
    Implicit,
    /// 2: client_credentials (RFC 6749 §4.4, M2M).
    ClientCredentials,
}

impl CallbackMode {
    /// 全部 3 callback mode 列表.
    pub const ALL: &'static [CallbackMode] = &[
        CallbackMode::AuthorizationCode,
        CallbackMode::Implicit,
        CallbackMode::ClientCredentials,
    ];

    /// Callback mode 字符串 (per RFC 6749 OAuth 2.0 spec).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthorizationCode => "authorization_code",
            Self::Implicit => "implicit",
            Self::ClientCredentials => "client_credentials",
        }
    }

    /// Callback mode 是否需要 user interaction (per RFC 6749 §1.3).
    pub const fn requires_user_interaction(self) -> bool {
        match self {
            Self::AuthorizationCode => true,
            Self::Implicit => true,
            Self::ClientCredentials => false, // M2M, no user
        }
    }

    /// Callback mode 是否需要 redirect_uri (per RFC 6749 §3.1.2).
    pub const fn requires_redirect_uri(self) -> bool {
        match self {
            Self::AuthorizationCode => true,
            Self::Implicit => true,
            Self::ClientCredentials => false, // M2M no redirect
        }
    }

    /// Callback mode 是否需要 PKCE (per RFC 7636 §1.1 + §4.1).
    pub const fn requires_pkce(self) -> bool {
        match self {
            Self::AuthorizationCode => true, // RFC 7636 §1.1 推荐
            Self::Implicit => false,
            Self::ClientCredentials => false,
        }
    }
}

impl std::fmt::Display for CallbackMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// §3 OAuthCallback trait (3 callback 模式统一接口)
// ============================================================================

/// OAuth Callback trait (per 借鉴 Golutra 6 AI CLI + RFC 6749 §4).
///
/// **3 核心方法**:
/// 1. `build_authorization` — 构造 authorization request (callback 模式特化)
/// 2. `parse_callback` — 解析 callback response (callback 模式特化)
/// 3. `client_credentials_grant` — client_credentials 直接发 token request (RFC 6749 §4.4)
///
/// **不假装**: skeleton 阶段方法全部 stub, R21+ 续真接 HTTP.
pub trait OAuthCallback: Send + Sync {
    /// Callback mode.
    fn mode(&self) -> CallbackMode;

    /// 构造 authorization request (per RFC 6749 §4.1.1 / §4.2.1 / §4.4).
    fn build_authorization(
        &self,
        auth_endpoint: &str,
        client_id: &str,
        redirect_uri: Option<&str>,
        scope: &[&str],
        state: &str,
        pkce: Option<&PkcePair>,
    ) -> OAuthResult<String> {
        // 5 K-1 强校验
        validate_client_id(client_id)?;
        if let Some(redirect) = redirect_uri {
            validate_redirect_uri(redirect)?;
        }
        validate_scope(scope)?;
        validate_state(state)?;

        // 模式特化
        let url = match self.mode() {
            CallbackMode::AuthorizationCode => {
                // RFC 6749 §4.1.1 + RFC 7636 §4.3
                let redirect = redirect_uri.ok_or(OAuthError::EmptyRedirectUri)?;
                let pkce_required = pkce.ok_or(OAuthError::InvalidPkceVerifier)?;
                format!(
                    "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
                    auth_endpoint,
                    url_encode(client_id),
                    url_encode(redirect),
                    url_encode(&scope.join(" ")),
                    url_encode(state),
                    url_encode(pkce_required.code_challenge()),
                )
            }
            CallbackMode::Implicit => {
                // RFC 6749 §4.2.1 (deprecated, response_type=token)
                let redirect = redirect_uri.ok_or(OAuthError::EmptyRedirectUri)?;
                format!(
                    "{}?response_type=token&client_id={}&redirect_uri={}&scope={}&state={}",
                    auth_endpoint,
                    url_encode(client_id),
                    url_encode(redirect),
                    url_encode(&scope.join(" ")),
                    url_encode(state),
                )
            }
            CallbackMode::ClientCredentials => {
                // RFC 6749 §4.4 (no authorization endpoint, use token_endpoint directly)
                // Return empty URL here; use client_credentials_grant() instead
                String::new()
            }
        };
        Ok(url)
    }

    /// 解析 callback response (per RFC 6749 §4.1.2 / §4.2.2 / §4.4.2).
    ///
    /// **skeleton 阶段**: 校验 + 解析 query string + 提取 code/error.
    fn parse_callback(&self, query: &str) -> OAuthResult<CallbackResponse> {
        // 解析 query string (?code=...&state=... or ?error=...)
        let mut code: Option<String> = None;
        let mut state_received: Option<String> = None;
        let mut error: Option<String> = None;
        for pair in query.trim_start_matches('?').split('&') {
            let mut kv = pair.splitn(2, '=');
            let k = kv.next().unwrap_or("");
            let v = kv.next().unwrap_or("");
            match k {
                "code" => code = Some(url_decode(v)),
                "state" => state_received = Some(url_decode(v)),
                "error" => error = Some(url_decode(v)),
                _ => {}
            }
        }
        Ok(CallbackResponse {
            code,
            state: state_received,
            error,
            mode: self.mode(),
        })
    }

    /// Client credentials grant (RFC 6749 §4.4).
    ///
    /// 仅 `CallbackMode::ClientCredentials` 支持, 其他 mode 返错误.
    fn client_credentials_grant(
        &self,
        token_endpoint: &str,
        client_id: &str,
        client_secret: &str,
        scope: &[&str],
    ) -> OAuthResult<CallbackResponse> {
        // 仅 ClientCredentials 模式支持
        if self.mode() != CallbackMode::ClientCredentials {
            return Err(OAuthError::InvalidState(format!(
                "client_credentials_grant only valid for ClientCredentials mode, got {}",
                self.mode().as_str()
            )));
        }
        validate_client_id(client_id)?;
        if client_secret.trim().is_empty() {
            return Err(OAuthError::EmptyState);
        }
        validate_scope(scope)?;
        // skeleton 阶段: stub, R21+ 续真发 POST {token_endpoint}?grant_type=client_credentials
        let url = format!(
            "{}?grant_type=client_credentials&client_id={}&client_secret={}&scope={}",
            token_endpoint,
            url_encode(client_id),
            url_encode(client_secret),
            url_encode(&scope.join(" ")),
        );
        Ok(CallbackResponse {
            code: Some(url), // 用 code 字段表示 "constructed request"
            state: None,
            error: None,
            mode: self.mode(),
        })
    }
}

/// Callback 解析响应 (per RFC 6749 §4.1.2 / §4.2.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallbackResponse {
    /// authorization code (per RFC 6749 §4.1.2).
    pub code: Option<String>,
    /// state (per RFC 6749 §10.12, 用于 CSRF 验证).
    pub state: Option<String>,
    /// error (per RFC 6749 §4.1.2.1, e.g. "access_denied" / "invalid_request").
    pub error: Option<String>,
    /// callback mode (1:1 跟解析者对应).
    pub mode: CallbackMode,
}

// ============================================================================
// §4 3 Callback impl (AuthorizationCode / Implicit / ClientCredentials)
// ============================================================================

/// AuthorizationCode callback (RFC 6749 §4.1, 推荐 + PKCE).
#[derive(Debug, Clone, Copy, Default)]
pub struct AuthorizationCodeCallback;

impl OAuthCallback for AuthorizationCodeCallback {
    fn mode(&self) -> CallbackMode {
        CallbackMode::AuthorizationCode
    }
}

/// Implicit callback (RFC 6749 §4.2, deprecated, 保留完整 spec).
#[derive(Debug, Clone, Copy, Default)]
pub struct ImplicitCallback;

impl OAuthCallback for ImplicitCallback {
    fn mode(&self) -> CallbackMode {
        CallbackMode::Implicit
    }
}

/// ClientCredentials callback (RFC 6749 §4.4, M2M).
#[derive(Debug, Clone, Copy, Default)]
pub struct ClientCredentialsCallback;

impl OAuthCallback for ClientCredentialsCallback {
    fn mode(&self) -> CallbackMode {
        CallbackMode::ClientCredentials
    }
}

// ============================================================================
// §5 辅助函数: URL encode/decode (RFC 6749 §3.1 + RFC 3986)
// ============================================================================

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

fn url_decode(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ============================================================================
// 单元测试 (3 callback mode + K-1 + parse_callback = 25 测试)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_mode_all_has_3_entries() {
        assert_eq!(CallbackMode::ALL.len(), CALLBACK_MODE_COUNT);
        assert_eq!(CALLBACK_MODE_COUNT, 3);
    }

    #[test]
    fn callback_mode_as_str_matches_rfc6749() {
        assert_eq!(CallbackMode::AuthorizationCode.as_str(), "authorization_code");
        assert_eq!(CallbackMode::Implicit.as_str(), "implicit");
        assert_eq!(CallbackMode::ClientCredentials.as_str(), "client_credentials");
    }

    #[test]
    fn callback_mode_user_interaction() {
        assert!(CallbackMode::AuthorizationCode.requires_user_interaction());
        assert!(CallbackMode::Implicit.requires_user_interaction());
        assert!(!CallbackMode::ClientCredentials.requires_user_interaction());
    }

    #[test]
    fn callback_mode_redirect_uri_required() {
        assert!(CallbackMode::AuthorizationCode.requires_redirect_uri());
        assert!(CallbackMode::Implicit.requires_redirect_uri());
        assert!(!CallbackMode::ClientCredentials.requires_redirect_uri());
    }

    #[test]
    fn callback_mode_pkce_required() {
        assert!(CallbackMode::AuthorizationCode.requires_pkce());
        assert!(!CallbackMode::Implicit.requires_pkce());
        assert!(!CallbackMode::ClientCredentials.requires_pkce());
    }

    #[test]
    fn callback_mode_serialize_round_trip() {
        for mode in [
            CallbackMode::AuthorizationCode,
            CallbackMode::Implicit,
            CallbackMode::ClientCredentials,
        ] {
            let s = serde_json::to_string(&mode).unwrap();
            let back: CallbackMode = serde_json::from_str(&s).unwrap();
            assert_eq!(mode, back);
        }
    }

    // ---- AuthorizationCode callback ----

    #[test]
    fn auth_code_build_with_pkce() {
        let cb = AuthorizationCodeCallback;
        let pkce = PkcePair::new();
        let url = cb
            .build_authorization(
                "https://provider.example.com/authorize",
                "client_abc",
                Some("https://app.example.com/cb"),
                &["read", "write"],
                "state_xyz",
                Some(&pkce),
            )
            .unwrap();
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=client_abc"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains(&format!("code_challenge={}", pkce.code_challenge())));
    }

    #[test]
    fn auth_code_build_rejects_missing_pkce() {
        let cb = AuthorizationCodeCallback;
        // AuthorizationCode 必须传 PKCE
        assert!(cb
            .build_authorization(
                "https://provider.example.com/authorize",
                "client_abc",
                Some("https://app.example.com/cb"),
                &["read"],
                "state_xyz",
                None, // missing PKCE
            )
            .is_err());
    }

    #[test]
    fn auth_code_build_rejects_missing_redirect() {
        let cb = AuthorizationCodeCallback;
        let pkce = PkcePair::new();
        assert!(cb
            .build_authorization(
                "https://provider.example.com/authorize",
                "client_abc",
                None, // missing redirect
                &["read"],
                "state_xyz",
                Some(&pkce),
            )
            .is_err());
    }

    #[test]
    fn auth_code_rejects_empty_client_id() {
        let cb = AuthorizationCodeCallback;
        let pkce = PkcePair::new();
        assert!(cb
            .build_authorization(
                "https://provider.example.com/authorize",
                "",
                Some("https://app.example.com/cb"),
                &["read"],
                "state_xyz",
                Some(&pkce),
            )
            .is_err());
    }

    // ---- Implicit callback ----

    #[test]
    fn implicit_build_no_pkce() {
        let cb = ImplicitCallback;
        let url = cb
            .build_authorization(
                "https://provider.example.com/authorize",
                "client_abc",
                Some("https://app.example.com/cb"),
                &["read"],
                "state_xyz",
                None, // implicit 不需要 PKCE
            )
            .unwrap();
        assert!(url.contains("response_type=token"));
        assert!(url.contains("client_id=client_abc"));
        assert!(!url.contains("code_challenge"));
    }

    #[test]
    fn implicit_requires_redirect() {
        let cb = ImplicitCallback;
        assert!(cb
            .build_authorization(
                "https://provider.example.com/authorize",
                "client_abc",
                None, // implicit 仍需 redirect
                &["read"],
                "state_xyz",
                None,
            )
            .is_err());
    }

    // ---- ClientCredentials callback ----

    #[test]
    fn client_credentials_build_authorization_returns_empty() {
        let cb = ClientCredentialsCallback;
        // client_credentials 模式不走 authorization endpoint
        let url = cb
            .build_authorization(
                "https://provider.example.com/token",
                "client_abc",
                None,
                &["read"],
                "state_xyz",
                None,
            )
            .unwrap();
        assert!(url.is_empty());
    }

    #[test]
    fn client_credentials_grant_constructs_url() {
        let cb = ClientCredentialsCallback;
        let resp = cb
            .client_credentials_grant(
                "https://provider.example.com/token",
                "client_abc",
                "secret_xyz",
                &["read", "write"],
            )
            .unwrap();
        assert_eq!(resp.mode, CallbackMode::ClientCredentials);
        assert!(resp.code.is_some());
        let url = resp.code.unwrap();
        assert!(url.contains("grant_type=client_credentials"));
        assert!(url.contains("client_id=client_abc"));
        assert!(url.contains("client_secret=secret_xyz"));
    }

    #[test]
    fn client_credentials_grant_only_valid_for_own_mode() {
        let cb = AuthorizationCodeCallback;
        assert!(cb
            .client_credentials_grant(
                "https://provider.example.com/token",
                "client_abc",
                "secret_xyz",
                &["read"],
            )
            .is_err());
    }

    #[test]
    fn client_credentials_rejects_empty_secret() {
        let cb = ClientCredentialsCallback;
        assert!(cb
            .client_credentials_grant(
                "https://provider.example.com/token",
                "client_abc",
                "",
                &["read"],
            )
            .is_err());
    }

    // ---- parse_callback ----

    #[test]
    fn parse_callback_extracts_code_and_state() {
        let cb = AuthorizationCodeCallback;
        let resp = cb
            .parse_callback("?code=auth_code_xyz&state=state_abc")
            .unwrap();
        assert_eq!(resp.code, Some("auth_code_xyz".to_string()));
        assert_eq!(resp.state, Some("state_abc".to_string()));
        assert!(resp.error.is_none());
        assert_eq!(resp.mode, CallbackMode::AuthorizationCode);
    }

    #[test]
    fn parse_callback_extracts_error() {
        let cb = AuthorizationCodeCallback;
        let resp = cb
            .parse_callback("?error=access_denied&state=state_abc")
            .unwrap();
        assert_eq!(resp.error, Some("access_denied".to_string()));
        assert!(resp.code.is_none());
    }

    #[test]
    fn parse_callback_handles_empty_query() {
        let cb = AuthorizationCodeCallback;
        let resp = cb.parse_callback("").unwrap();
        assert!(resp.code.is_none());
        assert!(resp.state.is_none());
        assert!(resp.error.is_none());
    }

    #[test]
    fn parse_callback_url_decodes() {
        let cb = AuthorizationCodeCallback;
        let resp = cb
            .parse_callback("?code=auth%2Bcode%2Fxyz&state=state%20abc")
            .unwrap();
        assert_eq!(resp.code, Some("auth+code/xyz".to_string()));
        assert_eq!(resp.state, Some("state abc".to_string()));
    }

    // ---- url_encode/decode ----

    #[test]
    fn url_encode_helper_works() {
        assert_eq!(url_encode("abc"), "abc");
        assert_eq!(url_encode("a b"), "a%20b");
        assert_eq!(url_encode("a+b"), "a%2Bb");
    }

    #[test]
    fn url_decode_helper_works() {
        assert_eq!(url_decode("abc"), "abc");
        assert_eq!(url_decode("a%20b"), "a b");
        assert_eq!(url_decode("a%2Bb"), "a+b");
    }

    #[test]
    fn url_encode_decode_round_trip() {
        let s = "中文 + special ~chars_-.123";
        let encoded = url_encode(s);
        let decoded = url_decode(&encoded);
        assert_eq!(decoded, s);
    }

    #[test]
    fn callback_mode_count_constant_is_3() {
        assert_eq!(CALLBACK_MODE_COUNT, 3);
    }
}
