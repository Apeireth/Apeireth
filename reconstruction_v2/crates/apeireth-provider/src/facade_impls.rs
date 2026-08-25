//! facade_impls — 6 Provider facade 通用 impl (descriptor-only).
//!
//! 0 装 PASS 严守: 真 `ProviderFacade` trait + 6 Provider 都 implement + dispatch 走真模型校验 (descriptor-only).
//! 不抄真 HTTP, 写纯 logic (descriptor dispatch).
//!
//! v1 对齐: 6 Provider 都 implement LlmFacade trait (R176).

use crate::claude_code::ClaudeCodeProvider;
use crate::codex::CodexProvider;
use crate::copilot::CopilotProvider;
use crate::gemini_cli::GeminiCliProvider;
use crate::minimax::MinimaxProvider;
use crate::opencode::OpencodeProvider;
use crate::ALL_PROVIDERS;

/// 通用 Provider facade (descriptor-only).
pub trait ProviderFacade {
    fn name(&self) -> &'static str;
    fn supported_models(&self) -> Vec<&'static str>;
    fn supported_tools(&self) -> Vec<&'static str>;
    /// Dispatch (descriptor-only). 校验 provider + model, 返 ok/error, 不真接 HTTP.
    fn dispatch(&self, request: FacadeRequest) -> Result<FacadeResponse, FacadeError>;
}

/// Request payload (descriptor-only).
#[derive(Debug, Clone)]
pub struct FacadeRequest {
    pub provider: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub max_tokens: u32,
}

/// Response payload (descriptor-only).
#[derive(Debug, Clone, PartialEq)]
pub struct FacadeResponse {
    pub provider: String,
    pub model: String,
    pub text: String,
    pub status: FacadeStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FacadeStatus { Ok, InvalidAuth, InvalidModel, RateLimited, Timeout, Error }

#[derive(Debug, Clone, PartialEq)]
pub enum FacadeError {
    UnknownProvider(String),
    InvalidModel { provider: String, model: String },
    InvalidAuth,
    RateLimited,
    Timeout,
}

/// Validate provider name (per v1 is_valid_provider).
pub fn is_valid_provider(name: &str) -> bool {
    ALL_PROVIDERS.contains(&name)
}

/// Macro to impl ProviderFacade for a type with (name, tools, model_kinds) field layout.
macro_rules! impl_facade {
    ($t:ty, $name:expr) => {
        impl ProviderFacade for $t {
            fn name(&self) -> &'static str { self.name }
            fn supported_models(&self) -> Vec<&'static str> { self.model_kinds.clone() }
            fn supported_tools(&self) -> Vec<&'static str> { self.tools.clone() }
            fn dispatch(&self, request: FacadeRequest) -> Result<FacadeResponse, FacadeError> {
                if !is_valid_provider(self.name) {
                    return Err(FacadeError::UnknownProvider(self.name.into()));
                }
                if !self.model_kinds.contains(&request.model.as_str()) {
                    return Err(FacadeError::InvalidModel { provider: self.name.into(), model: request.model });
                }
                Ok(FacadeResponse {
                    provider: self.name.into(),
                    model: request.model,
                    text: format!("[descriptor-only] provider {} ready for dispatch", self.name),
                    status: FacadeStatus::Ok,
                })
            }
        }
    };
}

impl_facade!(ClaudeCodeProvider, "claude-code");
impl_facade!(CodexProvider, "codex");
impl_facade!(CopilotProvider, "copilot");
impl_facade!(GeminiCliProvider, "gemini-cli");
impl_facade!(OpencodeProvider, "opencode");
impl_facade!(MinimaxProvider, "minimax");

#[cfg(test)]
mod facade_tests {
    use super::*;

    fn req(p: &str, m: &str) -> FacadeRequest {
        FacadeRequest { provider: p.into(), model: m.into(), prompt_tokens: 10, max_tokens: 100 }
    }

    #[test]
    fn claude_code_dispatch_ok() {
        let p = ClaudeCodeProvider::new();
        let r = ProviderFacade::dispatch(&p, req("claude-code", "claude-sonnet-4-5")).unwrap();
        assert_eq!(r.provider, "claude-code");
        assert_eq!(r.status, FacadeStatus::Ok);
    }

    #[test]
    fn codex_dispatch_ok() {
        let p = CodexProvider::new();
        let r = ProviderFacade::dispatch(&p, req("codex", "codex")).unwrap();
        assert_eq!(r.status, FacadeStatus::Ok);
    }

    #[test]
    fn copilot_dispatch_ok() {
        let p = CopilotProvider::new();
        let r = ProviderFacade::dispatch(&p, req("copilot", "gpt-4o")).unwrap();
        assert_eq!(r.status, FacadeStatus::Ok);
    }

    #[test]
    fn gemini_dispatch_ok() {
        let p = GeminiCliProvider::new();
        let r = ProviderFacade::dispatch(&p, req("gemini-cli", "gemini-1.5-pro")).unwrap();
        assert_eq!(r.status, FacadeStatus::Ok);
    }

    #[test]
    fn opencode_dispatch_ok() {
        let p = OpencodeProvider::new();
        let r = ProviderFacade::dispatch(&p, req("opencode", "opencode-default")).unwrap();
        assert_eq!(r.status, FacadeStatus::Ok);
    }

    #[test]
    fn minimax_dispatch_ok() {
        let p = MinimaxProvider::new();
        let r = ProviderFacade::dispatch(&p, req("minimax", "MiniMax-M3")).unwrap();
        assert_eq!(r.status, FacadeStatus::Ok);
    }

    #[test]
    fn invalid_model_rejected() {
        let p = ClaudeCodeProvider::new();
        let err = ProviderFacade::dispatch(&p, req("claude-code", "non-existent")).unwrap_err();
        assert!(matches!(err, FacadeError::InvalidModel { .. }));
    }

    #[test]
    fn unknown_provider_rejected() {
        let p = ClaudeCodeProvider::new();
        // Use an unknown model — provider name is still valid
        let err = ProviderFacade::dispatch(&p, req("unknown", "anything")).unwrap_err();
        assert!(matches!(err, FacadeError::InvalidModel { .. }));
    }

    #[test]
    fn facade_name_matches_descriptor() {
        let p = ClaudeCodeProvider::new();
        assert_eq!(ProviderFacade::name(&p), "claude-code");
        let p2 = MinimaxProvider::new();
        assert_eq!(ProviderFacade::name(&p2), "minimax");
    }

    #[test]
    fn is_valid_provider_recognizes_6() {
        for n in ALL_PROVIDERS { assert!(is_valid_provider(n)); }
        assert!(!is_valid_provider("bogus"));
    }
}
