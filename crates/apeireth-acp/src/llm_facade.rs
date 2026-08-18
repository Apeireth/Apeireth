//! apeireth-acp::llm_facade \u2014 LLM \u552f\u4e00\u63a5\u5165\u53e3 (per ADR-0033)
//!
//! \u300c\u672c\u8d44\u4ea7\u300d: LLM (HTTP / MCP / JSON-RPC \u63a5\u5165) \u552f\u4e00\u80fd\u8c03\u7684\u662f `LlmFacade::dispatch`.
//! \u4e0d\u80fd\u76f4\u63a5\u8c03 organ crate (consciousness/perception/cognition/...) \u907f\u514d\u8df3\u8fc7\u9274\u6743/\u9650\u6d41/\u534f\u8bae\u8f6c\u6362.
//!
//! **5 Provider \u7edf\u4e00\u63a5\u5165**: claude_code / codex / copilot / gemini_cli / opencode / minimax \u90fd\u5b9e\u73b0 `LlmFacade` trait.
//!
//! **\u4e0d\u6f02\u79fb**:
//! - 0 \u6539 Envelope (R23 LOCKED)
//! - 0 \u6539 5 Provider \u5b9e\u88c5 (R35 LOCKED)
//! - 0 \u52a8 workspace.version
//!
//! **\u72b6\u6001**: R176 (2026-08-14) \u521d\u59cb\u7248, 5 \u9879 + 6 Provider \u63a5\u5165 facade.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

/// LLM \u8bf7\u6c42\u7edf\u4e00\u5f62\u5f0f (per ADR-0033 \u00a72.2)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmRequest {
    /// \u63a5\u5165\u534f\u8bae (http / mcp / jsonrpc / cli)
    pub protocol: String,
    /// \u9009\u5b9a\u7684 provider (\u4e00\u4e2a\u4e8e 6 \u4e2a ALL_PROVIDERS \u4e4b\u4e2d)
    pub provider: String,
    /// \u9009\u5b9a\u7684 model_kind (\u7531 provider \u51b3\u5b9a\u662f\u5426\u5408\u6cd5)
    pub model: String,
    /// \u4f7f\u547d (system prompt)
    pub system: String,
    /// \u7528\u6237\u8f93\u5165 (user prompt)
    pub user: String,
    /// \u6d41\u5f0f\u54cd\u5e94 (\u9ed8\u8ba4 false)
    pub stream: bool,
    /// \u6700\u5927 token (\u9650\u6d41, \u9ed8\u8ba4 8192)
    pub max_tokens: u32,
    /// \u6e29\u5ea6 (\u9ed8\u8ba4 0.7, range [0.0, 2.0])
    pub temperature_x100: u16,
    /// \u8bf7\u6c42 ID (\u8d28\u8bc1\u7528, \u9ed8\u8ba4 empty = \u672a\u8d28\u8bc1)
    pub auth_token: String,
}

impl LlmRequest {
    /// \u6784\u9020 \u9ed8\u8ba4\u8bf7\u6c42
    pub fn new(
        provider: impl Into<String>,
        system: impl Into<String>,
        user: impl Into<String>,
    ) -> Self {
        Self {
            protocol: "http".into(),
            provider: provider.into(),
            model: String::new(),
            system: system.into(),
            user: user.into(),
            stream: false,
            max_tokens: 8192,
            temperature_x100: 70,
            auth_token: String::new(),
        }
    }

    /// \u9a8c\u8bc1\u8bf7\u6c42\u5408\u6cd5\u6027
    pub fn validate(&self) -> Result<(), LlmFacadeError> {
        if self.provider.is_empty() {
            return Err(LlmFacadeError::EmptyProvider);
        }
        if self.user.is_empty() && self.system.is_empty() {
            return Err(LlmFacadeError::EmptyPrompt);
        }
        if self.max_tokens == 0 || self.max_tokens > 200_000 {
            return Err(LlmFacadeError::InvalidMaxTokens(self.max_tokens));
        }
        if self.temperature_x100 > 200 {
            return Err(LlmFacadeError::InvalidTemperature(self.temperature_x100));
        }
        Ok(())
    }

    /// \u6e29\u5ea6\u8fd4\u56de f64 (temperature_x100 / 100)
    pub fn temperature(&self) -> f64 {
        f64::from(self.temperature_x100) / 100.0
    }
}

/// LLM \u54cd\u5e94\u7edf\u4e00\u5f62\u5f0f
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmResponse {
    /// \u8bf7\u6c42 ID (\u4e0e LlmRequest \u4e2d\u7684 trace_id \u5bf9\u5e94)
    pub request_id: String,
    /// provider \u540d
    pub provider: String,
    /// model_kind
    pub model: String,
    /// \u54cd\u5e94\u6587\u672c
    pub text: String,
    /// prompt tokens
    pub prompt_tokens: u32,
    /// completion tokens
    pub completion_tokens: u32,
    /// \u54cd\u5e94\u72b6\u6001 (ok / error)
    pub status: LlmStatus,
}

impl LlmResponse {
    /// \u6784\u9020 ok \u54cd\u5e94
    pub fn ok(
        provider: impl Into<String>,
        model: impl Into<String>,
        text: impl Into<String>,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) -> Self {
        Self {
            request_id: String::new(),
            provider: provider.into(),
            model: model.into(),
            text: text.into(),
            prompt_tokens,
            completion_tokens,
            status: LlmStatus::Ok,
        }
    }

    /// \u6784\u9020 error \u54cd\u5e94
    pub fn error(
        provider: impl Into<String>,
        model: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            request_id: String::new(),
            provider: provider.into(),
            model: model.into(),
            text: message.into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            status: LlmStatus::Error,
        }
    }

    /// \u603b token \u6570 (prompt + completion)
    pub fn total_tokens(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// LLM \u54cd\u5e94\u72b6\u6001
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmStatus {
    Ok,
    Error,
    RateLimited,
    InvalidAuth,
    Timeout,
}

impl LlmStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

/// LlmFacade \u9519\u8bef
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmFacadeError {
    EmptyProvider,
    EmptyPrompt,
    InvalidMaxTokens(u32),
    InvalidTemperature(u16),
    UnknownProvider(String),
    InvalidModel {
        provider: String,
        model: String,
    },
    /// Provider config/API key issue (e.g. http_dispatch)
    InvalidAuth,
    /// HTTP error (network/timeout/5xx)
    HttpError(String),
}

impl std::fmt::Display for LlmFacadeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProvider => write!(f, "llm_facade: provider is empty"),
            Self::EmptyPrompt => write!(f, "llm_facade: both system and user are empty"),
            Self::InvalidMaxTokens(n) => {
                write!(f, "llm_facade: invalid max_tokens {n} (must be 1-200000)")
            }
            Self::InvalidTemperature(t) => write!(
                f,
                "llm_facade: invalid temperature_x100 {t} (must be 0-200)"
            ),
            Self::UnknownProvider(p) => write!(f, "llm_facade: unknown provider '{p}'"),
            Self::InvalidModel { provider, model } => write!(
                f,
                "llm_facade: model '{model}' not supported by '{provider}'"
            ),
            Self::InvalidAuth => write!(f, "llm_facade: invalid API key or auth"),
            Self::HttpError(msg) => write!(f, "llm_facade: HTTP error: {msg}"),
        }
    }
}

impl std::error::Error for LlmFacadeError {}

/// LlmFacade trait \u2014 5 Provider \u7edf\u4e00\u63a5\u5165
///
/// **\u7ea6\u5b9a**: \u6240\u6709 Provider \u90fd\u5b9e\u73b0\u6b64 trait, \u8c03\u7528\u65b9\u4e0d\u80fd\u76f4\u63a5\u8bbf\u95ee provider \u5b9e\u73b0\u3002
/// \u8fd9\u662f ADR-0033 \u00a72.2 \u7684\u5f3a\u5236\u70b9 \u2014 \u7edf\u4e00\u9274\u6743/\u9650\u6d41/\u534f\u8bae\u8f6c\u6362\u5728 facade \u5904\u5b8c\u6210.
pub trait LlmFacade: Send + Sync {
    /// Provider \u540d (\u4e0e ALL_PROVIDERS \u5bf9\u9f50)
    fn name(&self) -> &'static str;

    /// \u652f\u6301\u7684 model_kind \u5217\u8868
    fn supported_models(&self) -> Vec<&'static str>;

    /// \u652f\u6301\u7684\u5de5\u5177\u540d\u5217\u8868
    fn supported_tools(&self) -> Vec<&'static str>;

    /// \u8c03\u5ea6\u8bf7\u6c42 \u2014 \u9ed8\u8ba4\u5b9e\u73b0\u4f1a\u9a8c\u8bc1 + \u63a5\u5165 upstream + \u8fd4\u54cd\u5e94
    ///
    /// \u7eaf\u51fd\u6570\u7c7b\u63a5\u53e3, \u4e0d\u4fee\u6539\u8bf7\u6c42, \u8fd4\u54cd\u5e94\u4e0e upstream \u4e00\u81f4\u3002
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError>;

    /// \u9ed8\u8ba4\u8c03\u5ea6 \u2014 \u8c03\u7528 dispatch \u4e0e\u9a8c\u8bc1
    fn handle(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError> {
        request.validate()?;
        if !self.supported_models().iter().any(|m| *m == request.model) {
            if !request.model.is_empty() {
                return Err(LlmFacadeError::InvalidModel {
                    provider: self.name().into(),
                    model: request.model.clone(),
                });
            }
        }
        self.dispatch(request)
    }
}

/// 6 Provider \u540d\u5b57\u5217\u8868 (\u4e0e ALL_PROVIDERS \u5bf9\u9f50)
pub const ALL_PROVIDER_NAMES: [&str; 6] = [
    "claude-code",
    "codex",
    "copilot",
    "gemini-cli",
    "opencode",
    "minimax",
];

/// \u68c0\u67e5 provider \u540d\u662f\u5426\u5408\u6cd5
pub fn is_valid_provider(name: &str) -> bool {
    ALL_PROVIDER_NAMES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_default_valid() {
        let r = LlmRequest::new("minimax", "hi", "hello");
        assert!(r.validate().is_ok());
    }

    #[test]
    fn request_empty_provider_rejected() {
        let mut r = LlmRequest::new("minimax", "hi", "hello");
        r.provider = String::new();
        assert_eq!(r.validate(), Err(LlmFacadeError::EmptyProvider));
    }

    #[test]
    fn request_empty_prompt_rejected() {
        let mut r = LlmRequest::new("minimax", "", "");
        r.system = String::new();
        r.user = String::new();
        assert_eq!(r.validate(), Err(LlmFacadeError::EmptyPrompt));
    }

    #[test]
    fn request_max_tokens_zero_rejected() {
        let mut r = LlmRequest::new("minimax", "hi", "hello");
        r.max_tokens = 0;
        assert!(matches!(
            r.validate(),
            Err(LlmFacadeError::InvalidMaxTokens(_))
        ));
    }

    #[test]
    fn request_max_tokens_too_large_rejected() {
        let mut r = LlmRequest::new("minimax", "hi", "hello");
        r.max_tokens = 200_001;
        assert!(matches!(
            r.validate(),
            Err(LlmFacadeError::InvalidMaxTokens(_))
        ));
    }

    #[test]
    fn request_invalid_temperature_rejected() {
        let mut r = LlmRequest::new("minimax", "hi", "hello");
        r.temperature_x100 = 250;
        assert!(matches!(
            r.validate(),
            Err(LlmFacadeError::InvalidTemperature(_))
        ));
    }

    #[test]
    fn request_temperature_returns_float() {
        let mut r = LlmRequest::new("minimax", "hi", "hello");
        r.temperature_x100 = 75;
        assert_eq!(r.temperature(), 0.75);
    }

    #[test]
    fn response_ok_total_tokens() {
        let r = LlmResponse::ok("minimax", "MiniMax-M3", "hi", 100, 50);
        assert_eq!(r.total_tokens(), 150);
        assert!(r.status.is_success());
    }

    #[test]
    fn response_error_not_success() {
        let r = LlmResponse::error("minimax", "MiniMax-M3", "boom");
        assert!(!r.status.is_success());
        assert_eq!(r.total_tokens(), 0);
    }

    #[test]
    fn all_provider_names_count_is_6() {
        assert_eq!(ALL_PROVIDER_NAMES.len(), 6);
    }

    #[test]
    fn is_valid_provider_recognizes_known_names() {
        for n in &ALL_PROVIDER_NAMES {
            assert!(is_valid_provider(n), "provider {} should be valid", n);
        }
    }

    #[test]
    fn is_valid_provider_rejects_unknown() {
        assert!(!is_valid_provider("unknown"));
        assert!(!is_valid_provider(""));
    }

    #[test]
    fn llm_status_is_success_for_ok() {
        assert!(LlmStatus::Ok.is_success());
        assert!(!LlmStatus::Error.is_success());
        assert!(!LlmStatus::RateLimited.is_success());
        assert!(!LlmStatus::InvalidAuth.is_success());
        assert!(!LlmStatus::Timeout.is_success());
    }

    #[test]
    fn request_serde_roundtrip() {
        let r = LlmRequest::new("minimax", "system", "user");
        let s = serde_json::to_string(&r).unwrap();
        let decoded: LlmRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(decoded, r);
    }

    #[test]
    fn response_serde_roundtrip() {
        let r = LlmResponse::ok("minimax", "MiniMax-M3", "hello", 10, 5);
        let s = serde_json::to_string(&r).unwrap();
        let decoded: LlmResponse = serde_json::from_str(&s).unwrap();
        assert_eq!(decoded, r);
    }
}
