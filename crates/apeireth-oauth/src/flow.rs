//! # OAuthFlow — 顶层 4 步 OAuth flow
//!
//! 借鉴 Golutra v0.1.0 OAuth flow 模式 1:1 翻译, 跟 sister #1+#6 1:1 镜像.
//!
//! **4 步 OAuth flow** (per RFC 6749 + 借鉴 Golutra):
//! 1. `prepare` — 准备 state + PKCE + callback handler
//! 2. `build_authorization` — 构造 authorization request URL (state + PKCE 嵌入)
//! 3. `exchange_code_for_token` — 用 callback 收到的 code 换 access_token
//! 4. `refresh_access_token` — 用 refresh_token 换新 access_token (可选)
//!
//! **不假装**: skeleton 阶段方法全部 stub, R21+ 续真接 HTTP.

use serde::{Deserialize, Serialize};

use crate::error::{validate_client_id, OAuthError, OAuthResult};
use crate::provider::AccessToken;
use crate::state::{OAuthState, PkcePair};

/// **K-1 强校验**: OAuthFlow 4 步编译期守门.
pub const FLOW_STEP_COUNT: usize = 4;

/// OAuthFlow 4 步 enum (编译期 hardcode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowStep {
    /// 0: prepare — 准备 state + PKCE.
    Prepare,
    /// 1: build_authorization — 构造 authorization request URL.
    BuildAuthorization,
    /// 2: exchange_code_for_token — code → access_token.
    ExchangeCodeForToken,
    /// 3: refresh_access_token — refresh_token → 新 access_token.
    RefreshAccessToken,
}

impl FlowStep {
    /// 全部 4 步列表.
    pub const ALL: &'static [FlowStep] = &[
        FlowStep::Prepare,
        FlowStep::BuildAuthorization,
        FlowStep::ExchangeCodeForToken,
        FlowStep::RefreshAccessToken,
    ];

    /// 编译期字符串表示 (per OAuth 2.0 spec).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::BuildAuthorization => "build_authorization",
            Self::ExchangeCodeForToken => "exchange_code_for_token",
            Self::RefreshAccessToken => "refresh_access_token",
        }
    }
}

impl std::fmt::Display for FlowStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Flow 准备结果 (state + PKCE pair + created_at).
#[derive(Debug, Clone)]
pub struct FlowHandle {
    /// CSRF state (per RFC 6749 §10.12).
    pub state: OAuthState,
    /// PKCE pair (per RFC 7636 §4.2).
    pub pkce: PkcePair,
}

/// OAuthFlow trait (顶层 4 步 OAuth flow).
///
/// **不假装**: skeleton 阶段方法全部 stub, R21+ 续真接 HTTP.
pub trait OAuthFlow: Send + Sync {
    /// 准备 flow (生成 state + PKCE pair, 1:1 镜像 sister #1 organ command 模式).
    ///
    /// 5 K-1 强校验: 必传 client_id 非空.
    fn prepare(&self, client_id: &str) -> OAuthResult<FlowHandle> {
        validate_client_id(client_id)?;
        Ok(FlowHandle {
            state: OAuthState::new(),
            pkce: PkcePair::new(),
        })
    }

    /// 构造 authorization request URL (per RFC 6749 §4.1.1 + RFC 7636 §4.3).
    ///
    /// **skeleton 阶段**: 委托给 provider trait method, R21+ 续真发 GET.
    fn build_authorization(
        &self,
        provider: &dyn crate::provider::OAuthProvider,
        redirect_uri: &str,
        scope: &[&str],
        handle: &FlowHandle,
    ) -> OAuthResult<String> {
        provider.build_authorization_url(redirect_uri, scope, &handle.state, &handle.pkce)
    }

    /// 用 callback code 换 access_token (per RFC 6749 §4.1.3).
    ///
    /// **skeleton 阶段**: 委托给 provider trait method, R21+ 续真发 POST.
    fn exchange_code_for_token(
        &self,
        provider: &dyn crate::provider::OAuthProvider,
        code: &str,
        redirect_uri: &str,
        handle: &FlowHandle,
    ) -> OAuthResult<AccessToken> {
        provider.exchange_code_for_token(code, redirect_uri, &handle.state, &handle.pkce)
    }

    /// 用 refresh_token 换新 access_token (per RFC 6749 §6).
    fn refresh_access_token(
        &self,
        provider: &dyn crate::provider::OAuthProvider,
        refresh_token: &str,
    ) -> OAuthResult<AccessToken> {
        provider.refresh_access_token(refresh_token)
    }
}

/// Default OAuthFlow impl (空 stub, 4 步走 default trait method).
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultOAuthFlow;

impl OAuthFlow for DefaultOAuthFlow {}

// ============================================================================
// 单元测试 (4 步 + FlowHandle + DefaultOAuthFlow = 15 测试)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callback::{AuthorizationCodeCallback, CallbackMode, OAuthCallback};
    use crate::provider::{ClaudeCodeProvider, OAuthProvider, ProviderKind};

    #[test]
    fn flow_step_all_has_4_entries() {
        assert_eq!(FlowStep::ALL.len(), FLOW_STEP_COUNT);
        assert_eq!(FLOW_STEP_COUNT, 4);
    }

    #[test]
    fn flow_step_as_str() {
        assert_eq!(FlowStep::Prepare.as_str(), "prepare");
        assert_eq!(FlowStep::BuildAuthorization.as_str(), "build_authorization");
        assert_eq!(
            FlowStep::ExchangeCodeForToken.as_str(),
            "exchange_code_for_token"
        );
        assert_eq!(
            FlowStep::RefreshAccessToken.as_str(),
            "refresh_access_token"
        );
    }

    #[test]
    fn flow_step_serialize_round_trip() {
        for step in [
            FlowStep::Prepare,
            FlowStep::BuildAuthorization,
            FlowStep::ExchangeCodeForToken,
            FlowStep::RefreshAccessToken,
        ] {
            let s = serde_json::to_string(&step).unwrap();
            let back: FlowStep = serde_json::from_str(&s).unwrap();
            assert_eq!(step, back);
        }
    }

    #[test]
    fn flow_step_display() {
        assert_eq!(format!("{}", FlowStep::Prepare), "prepare");
        assert_eq!(format!("{}", FlowStep::BuildAuthorization), "build_authorization");
    }

    #[test]
    fn flow_handle_contains_state_and_pkce() {
        let handle = FlowHandle {
            state: OAuthState::new(),
            pkce: PkcePair::new(),
        };
        assert_eq!(handle.state.as_str().len(), 43);
        assert_eq!(handle.pkce.code_verifier().len(), 86);
        assert_eq!(handle.pkce.code_challenge().len(), 43);
    }

    #[test]
    fn flow_handle_clone_preserves_data() {
        let h1 = FlowHandle {
            state: OAuthState::new(),
            pkce: PkcePair::new(),
        };
        let h2 = h1.clone();
        assert_eq!(h1.state, h2.state);
        assert_eq!(h1.pkce, h2.pkce);
    }

    #[test]
    fn default_oauth_flow_prepare_creates_handle() {
        let flow = DefaultOAuthFlow;
        let handle = flow.prepare("client_abc").unwrap();
        assert_eq!(handle.state.as_str().len(), 43);
        assert_eq!(handle.pkce.code_verifier().len(), 86);
    }

    #[test]
    fn default_oauth_flow_prepare_rejects_empty_client_id() {
        let flow = DefaultOAuthFlow;
        assert!(flow.prepare("").is_err());
        assert!(flow.prepare("   ").is_err());
    }

    #[test]
    fn default_oauth_flow_build_authorization_4_steps() {
        let flow = DefaultOAuthFlow;
        let provider = ClaudeCodeProvider::new("c", "s").unwrap();
        let handle = flow.prepare("client_abc").unwrap();
        let url = flow
            .build_authorization(
                &provider,
                "https://app.example.com/cb",
                &["read"],
                &handle,
            )
            .unwrap();
        assert!(url.contains("response_type=code"));
        assert!(url.contains(&format!("state={}", handle.state.as_str())));
    }

    #[test]
    fn default_oauth_flow_exchange_code_for_token() {
        let flow = DefaultOAuthFlow;
        let provider = ClaudeCodeProvider::new("c", "s").unwrap();
        let handle = flow.prepare("client_abc").unwrap();
        let token = flow
            .exchange_code_for_token(&provider, "auth_code_xyz", "https://app.example.com/cb", &handle)
            .unwrap();
        assert_eq!(token.token_type, "bearer");
    }

    #[test]
    fn default_oauth_flow_refresh_access_token() {
        let flow = DefaultOAuthFlow;
        let provider = ClaudeCodeProvider::new("c", "s").unwrap();
        let token = flow
            .refresh_access_token(&provider, "refresh_xyz")
            .unwrap();
        assert_eq!(token.token_type, "bearer");
    }

    #[test]
    fn flow_step_count_constant_is_4() {
        assert_eq!(FLOW_STEP_COUNT, 4);
    }

    #[test]
    fn flow_callback_integration_3_modes() {
        // 跨模块集成: 3 Callback mode + Flow
        let modes = [
            CallbackMode::AuthorizationCode,
            CallbackMode::Implicit,
            CallbackMode::ClientCredentials,
        ];
        for mode in modes {
            // 仅 smoke test: CallbackMode 存在 + FlowStep 4 步
            assert!(mode.as_str().len() > 0);
        }
        // 集成: 构造 callback 走 parse
        let cb = AuthorizationCodeCallback;
        let resp = cb
            .parse_callback("?code=xyz&state=abc")
            .unwrap();
        assert_eq!(resp.code, Some("xyz".to_string()));
    }

    #[test]
    fn flow_provider_integration_3_providers() {
        // 跨模块集成: 3 Provider + Flow
        let flow = DefaultOAuthFlow;
        let providers: Vec<Box<dyn OAuthProvider>> = vec![
            Box::new(ClaudeCodeProvider::new("c", "s").unwrap()),
            Box::new(crate::provider::OpencodeProvider::new("c", "s").unwrap()),
            Box::new(crate::provider::CopilotProvider::new("c", "s").unwrap()),
        ];
        for p in providers {
            let handle = flow.prepare("c").unwrap();
            let url = flow
                .build_authorization(&*p, "https://app.example.com/cb", &["read"], &handle)
                .unwrap();
            assert!(url.contains("response_type=code"));
            // 3 Provider 端点不同
            match p.kind() {
                ProviderKind::ClaudeCode => {
                    assert!(url.starts_with(crate::provider::CLAUDE_CODE_AUTHORIZATION_ENDPOINT));
                }
                ProviderKind::Opencode => {
                    assert!(url.starts_with(crate::provider::OPENCODE_AUTHORIZATION_ENDPOINT));
                }
                ProviderKind::Copilot => {
                    assert!(url.starts_with(crate::provider::COPILOT_AUTHORIZATION_ENDPOINT));
                }
            }
        }
    }
}
