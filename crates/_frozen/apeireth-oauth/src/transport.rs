//! OAuth HTTP transport — R23 P1 真接入口
//!
//! **目的**: 把 14 处 example.com 的 "fake URL" 变 "R23 未开启，待 R24+ 打开 real-http feature 真接".
//!
//! 不弄以重的 (per 8 项承诺 #2 不偷不漏, S-1 北极星不骗人):
//! - R21+R22+R23 在 1.0 release 中以 skeleton 面貌出货 (仅生成 authorize URL 字符串, 0 走 HTTP)
//! - R23 本次 commit 添加了 transport.rs skeleton — 打包了 reqwest::Client 加载点 + 3 Provider 端点选择口子
//!   但 0 被 default feature 启动 (避免 1.0 release 运行本地隐式发 HTTP 请求去 example.com)
//! - 3 Provider 端点 (在 provider.rs:CLAUDE_CODE_AUTHORIZATION_ENDPOINT 等 9 个常量) 都是 真走产品 Endpoint:
//!   - api.anthropic.com / oauth/{authorize,token,revoke}
//!   - api.opencode.ai / oauth/{authorize,token,revoke}
//!   - github.com/login/oauth/{authorize,access_token}
//! - 以 https://provider.example.com/... 样子出现的 "example.com" 是 OAuth RFC 6749 §3.1 + §4.1.3 范例中的 "reference URI"
//!   — 处于 OAuth library 的 docstring + test fixture 中, 0 进入生产调用
//!
//! R24+ 启用 "real-http" feature 后:
//! ```ignore
//! use apeireth_oauth::transport::HttpTransport;
//! let transport = HttpTransport::new()?;
//! let token = transport.post_token_form(
//!     ProviderKind::ClaudeCode.token_endpoint(),
//!     &[("code".into(), "my_code".into()), ...],
//! ).await?;
//! ```

#![cfg(feature = "real-http")]

use std::time::Duration;

use reqwest::Client;

use crate::error::{OAuthError, OAuthResult};
use crate::provider::ProviderKind;

/// 默认 HTTP client 超时 (per RFC 6749 §10.16 / BCP §6.1.7 推荐 ≤10s, 安全防 DoS).
pub const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// HTTP 传输层 — reqwest::Client wrapper.
#[derive(Debug, Clone)]
pub struct HttpTransport { client: Client, timeout: Duration }

impl HttpTransport {
    /// Construct a new transport. 需 `real-http` feature 启动.
    pub fn new() -> OAuthResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .user_agent(concat!("apeireth-oauth/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| OAuthError::Other(format!("reqwest::Client build: {e}")))?;
        Ok(Self { client, timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS) })
    }
    /// 查询超时 (给测试用).
    pub fn timeout(&self) -> Duration { self.timeout }

    /// Token endpoint POST application/x-www-form-urlencoded.
    /// 返 access_token JSON (RFC 6749 §5.1).
    pub async fn post_token_form(
        &self,
        token_endpoint: &str,
        form: &[(String, String)],
    ) -> OAuthResult<serde_json::Value> {
        let resp = self
            .client
            .post(token_endpoint)
            .form(form)
            .send()
            .await
            .map_err(|e| OAuthError::TokenExchange(format!("POST {token_endpoint}: {e}")))?;
        let status = resp.status();
        let body = resp.text().await
            .map_err(|e| OAuthError::TokenExchange(format!("read body: {e}")))?;
        if !status.is_success() {
            return Err(OAuthError::TokenExchange(format!(
                "POST {token_endpoint} 返 HTTP {status}: {body}"
            )));
        }
        serde_json::from_str(&body)
            .map_err(|e| OAuthError::TokenExchange(format!("JSON decode: {e}; body: {body}")))
    }
}

/// 选 Provider 的 token_endpoint — 真 走产品 OAuth endpoint
/// (api.anthropic.com / api.opencode.ai / github.com), 不是 fake.example.com.
pub fn token_endpoint_for(kind: ProviderKind) -> &'static str { kind.token_endpoint() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_endpoint_for_claude_code_is_anthropic() {
        assert!(token_endpoint_for(ProviderKind::ClaudeCode).starts_with("https://api.anthropic.com/"));
    }
    #[test]
    fn token_endpoint_for_opencode_is_opencode_ai() {
        assert!(token_endpoint_for(ProviderKind::Opencode).starts_with("https://api.opencode.ai/"));
    }
    #[test]
    fn token_endpoint_for_copilot_is_github() {
        assert!(token_endpoint_for(ProviderKind::Copilot).starts_with("https://github.com/"));
    }
    #[test]
    fn endpoint_constants_are_https_only() {
        for kind in ProviderKind::ALL {
            assert!(kind.authorization_endpoint().starts_with("https://"));
            assert!(kind.token_endpoint().starts_with("https://"));
            assert!(kind.revocation_endpoint().starts_with("https://"));
        }
    }
    #[test]
    fn endpoint_constants_no_example_com() {
        // 9 个 endpoint 都不能指向 example.com (那是 RFC 范例里的 reference URI, 不是产品)
        for kind in ProviderKind::ALL {
            for ep in [kind.authorization_endpoint(), kind.token_endpoint(), kind.revocation_endpoint()] {
                assert!(!ep.contains("example.com"), "endpoint {ep} 仍指 example.com");
            }
        }
    }
}

// ============================================================================
// R44: device_code HTTP polling 真接 (RFC 8628 §3.1 + §3.5)
// ============================================================================

/// RFC 8628 §3.1 device authorization request body (application/x-www-form-urlencoded).
#[derive(Debug, serde::Serialize)]
pub struct DeviceAuthForm {
    /// OAuth client_id (RFC 8628 §3.1 必填).
    pub client_id: String,
    /// space-delimited scope (RFC 8628 §3.1).
    pub scope: String,
}

/// RFC 8628 §3.2 device authorization response.
#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}

/// RFC 8628 §3.5 token polling request body.
#[derive(Debug, serde::Serialize)]
pub struct TokenPollForm {
    pub grant_type: String,
    pub device_code: String,
    pub client_id: String,
}

/// RFC 8628 §3.5 token polling response (含 error 字段, RFC 8628 §3.5.1 / §3.5.2).
#[derive(Debug, serde::Deserialize, Clone, PartialEq)]
pub struct TokenPollResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

impl HttpTransport {
    /// POST device_authorization_endpoint (RFC 8628 §3.1).
    /// 返 DeviceAuthResponse (含 device_code + user_code + verification_uri + interval).
    pub async fn post_device_authorization(
        &self,
        device_authorization_endpoint: &str,
        client_id: &str,
        scope: &str,
    ) -> OAuthResult<DeviceAuthResponse> {
        let body = DeviceAuthForm {
            client_id: client_id.to_string(),
            scope: scope.to_string(),
        };
        let resp = self
            .client
            .post(device_authorization_endpoint)
            .form(&body)
            .send()
            .await
            .map_err(|e| OAuthError::Other(format!("POST {device_authorization_endpoint}: {e}")))?;
        let status = resp.status();
        let text = resp.text().await
            .map_err(|e| OAuthError::Other(format!("read body: {e}")))?;
        if !status.is_success() {
            return Err(OAuthError::Other(format!(
                "POST {device_authorization_endpoint} 返 HTTP {status}: {text}"
            )));
        }
        serde_json::from_str(&text).map_err(|e| {
            OAuthError::Other(format!("JSON decode: {e}; body: {text}"))
        })
    }

    /// POST token_endpoint with device_code grant (RFC 8628 §3.5 polling).
    /// 返 TokenPollResponse (含 authorization_pending / access_token 等).
    pub async fn post_token_poll(
        &self,
        token_endpoint: &str,
        client_id: &str,
        device_code: &str,
    ) -> OAuthResult<TokenPollResponse> {
        let body = TokenPollForm {
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
            device_code: device_code.to_string(),
            client_id: client_id.to_string(),
        };
        let resp = self
            .client
            .post(token_endpoint)
            .form(&body)
            .send()
            .await
            .map_err(|e| OAuthError::Other(format!("POST {token_endpoint}: {e}")))?;
        let status = resp.status();
        let text = resp.text().await
            .map_err(|e| OAuthError::Other(format!("read body: {e}")))?;
        // RFC 8628 §3.5: error responses (authorization_pending, slow_down, expired_token, access_denied)
        // 都返 HTTP 400, 但 body 含 error 字段, 我们 JSON 解析后让 caller 处理.
        if !status.is_success() && status.as_u16() != 400 {
            return Err(OAuthError::Other(format!(
                "POST {token_endpoint} 返 HTTP {status}: {text}"
            )));
        }
        serde_json::from_str(&text).map_err(|e| {
            OAuthError::Other(format!("JSON decode: {e}; body: {text}"))
        })
    }
}

// ============================================================================
// R44: device_code HTTP polling 端到端 wiremock 测试 (real-http feature)
// ============================================================================

#[cfg(all(test, feature = "real-http"))]
mod device_code_http_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const DEVICE_AUTH_BODY: &str = r#"{
        "device_code": "GmRhmhcxhwAzkoEqiMEg_DnyEysNkuNhszIySk9eS",
        "user_code": "WDJB-MJHT",
        "verification_uri": "https://example.com/device",
        "verification_uri_complete": "https://example.com/device?user_code=WDJB-MJHT",
        "expires_in": 1800,
        "interval": 5
    }"#;

    const PENDING_BODY: &str = r#"{
        "error": "authorization_pending",
        "error_description": "The authorization request is still pending."
    }"#;

    const TOKEN_BODY: &str = r#"{
        "access_token": "eyJhbGciOiJIUzI1NiJ9.payload.signature",
        "token_type": "Bearer",
        "expires_in": 3600,
        "refresh_token": "r1f2r3e4s5h",
        "scope": "read write"
    }"#;

    #[tokio::test]
    async fn device_authorization_full_round_trip() {
        let mock = MockServer::start().await;
        Mock::given(method("POST")).and(path("/device_authorization"))
            .respond_with(ResponseTemplate::new(200).set_body_string(DEVICE_AUTH_BODY))
            .mount(&mock).await;
        let transport = HttpTransport::new().expect("HttpTransport::new");
        let resp = transport
            .post_device_authorization(
                &format!("{}/device_authorization", mock.uri()),
                "test_client",
                "read write",
            )
            .await
            .expect("post_device_authorization");
        assert_eq!(resp.device_code, "GmRhmhcxhwAzkoEqiMEg_DnyEysNkuNhszIySk9eS");
        assert_eq!(resp.user_code, "WDJB-MJHT");
        assert_eq!(resp.verification_uri, "https://example.com/device");
        assert_eq!(resp.expires_in, 1800);
        assert_eq!(resp.interval, 5);
    }

    #[tokio::test]
    async fn device_authorization_5xx_returns_error() {
        let mock = MockServer::start().await;
        Mock::given(method("POST")).and(path("/device_authorization"))
            .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
            .mount(&mock).await;
        let transport = HttpTransport::new().expect("HttpTransport::new");
        let result = transport
            .post_device_authorization(
                &format!("{}/device_authorization", mock.uri()),
                "test_client",
                "read",
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 500"));
    }

    #[tokio::test]
    async fn token_poll_authorization_pending_then_success() {
        let mock = MockServer::start().await;
        Mock::given(method("POST")).and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string(PENDING_BODY))
            .up_to_n_times(2)
            .mount(&mock).await;
        Mock::given(method("POST")).and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(TOKEN_BODY))
            .mount(&mock).await;
        let transport = HttpTransport::new().expect("HttpTransport::new");
        for _ in 0..2 {
            let resp = transport
                .post_token_poll(&format!("{}/token", mock.uri()), "test_client", "dev_code")
                .await
                .expect("post_token_poll");
            assert_eq!(resp.error.as_deref(), Some("authorization_pending"));
            assert!(resp.access_token.is_none());
        }
        let resp = transport
            .post_token_poll(&format!("{}/token", mock.uri()), "test_client", "dev_code")
            .await
            .expect("post_token_poll");
        assert!(resp.error.is_none());
        assert_eq!(
            resp.access_token.as_deref(),
            Some("eyJhbGciOiJIUzI1NiJ9.payload.signature")
        );
        assert_eq!(resp.token_type.as_deref(), Some("Bearer"));
        assert_eq!(resp.refresh_token.as_deref(), Some("r1f2r3e4s5h"));
    }

    #[tokio::test]
    async fn token_poll_access_denied_error() {
        let mock = MockServer::start().await;
        Mock::given(method("POST")).and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string(
                r#"{"error":"access_denied","error_description":"user denied"}"#,
            ))
            .mount(&mock).await;
        let transport = HttpTransport::new().expect("HttpTransport::new");
        let resp = transport
            .post_token_poll(&format!("{}/token", mock.uri()), "test_client", "dev_code")
            .await
            .expect("post_token_poll");
        assert_eq!(resp.error.as_deref(), Some("access_denied"));
        assert_eq!(resp.error_description.as_deref(), Some("user denied"));
    }
}

