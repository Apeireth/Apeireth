//! apeireth-acp::llm_facade \u{2014} LLM \u{552f}\u{4e00}\u{63a5}\u{5165}\u{53e3} (per ADR-0033)
//!
//! \u{300c}\u{672c}\u{8d44}\u{4ea7}\u{300d}: LLM (HTTP / MCP / JSON-RPC \u{63a5}\u{5165}) \u{552f}\u{4e00}\u{80fd}\u{8c03}\u{7684}\u{662f} `LlmFacade::dispatch`.
//! \u{4e0d}\u{80fd}\u{76f4}\u{63a5}\u{8c03} organ crate (consciousness/perception/cognition/...) \u{907f}\u{514d}\u{8df3}\u{8fc7}\u{9274}\u{6743}/\u{9650}\u{6d41}/\u{534f}\u{8bae}\u{8f6c}\u{6362}.
//!
//! **5 Provider \u{7edf}\u{4e00}\u{63a5}\u{5165}**: claude_code / codex / copilot / gemini_cli / opencode / minimax \u{90fd}\u{5b9e}\u{73b0} `LlmFacade` trait.
//!
//! **\u{4e0d}\u{6f02}\u{79fb}**:
//! - 0 \u{6539} Envelope (R23 LOCKED)
//! - 0 \u{6539} 5 Provider \u{5b9e}\u{88c5} (R35 LOCKED)
//! - 0 \u{52a8} workspace.version
//!
//! **\u{72b6}\u{6001}**: R176 (2026-08-14) \u{521d}\u{59cb}\u{7248}, 5 \u{9879} + 6 Provider \u{63a5}\u{5165} facade.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

/// LLM \u{8bf7}\u{6c42}\u{7edf}\u{4e00}\u{5f62}\u{5f0f} (per ADR-0033 \u00a72.2)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmRequest {
    /// \u{63a5}\u{5165}\u{534f}\u{8bae} (http / mcp / jsonrpc / cli)
    pub protocol: String,
    /// \u{9009}\u{5b9a}\u{7684} provider (\u{4e00}\u{4e2a}\u{4e8e} 6 \u{4e2a} ALL_PROVIDERS \u{4e4b}\u{4e2d})
    pub provider: String,
    /// \u{9009}\u{5b9a}\u{7684} model_kind (\u{7531} provider \u{51b3}\u{5b9a}\u{662f}\u{5426}\u{5408}\u{6cd5})
    pub model: String,
    /// \u{4f7f}\u{547d} (system prompt)
    pub system: String,
    /// \u{7528}\u{6237}\u{8f93}\u{5165} (user prompt)
    pub user: String,
    /// \u{6d41}\u{5f0f}\u{54cd}\u{5e94} (\u{9ed8}\u{8ba4} false)
    pub stream: bool,
    /// \u{6700}\u{5927} token (\u{9650}\u{6d41}, \u{9ed8}\u{8ba4} 8192)
    pub max_tokens: u32,
    /// \u{6e29}\u{5ea6} (\u{9ed8}\u{8ba4} 0.7, range [0.0, 2.0])
    pub temperature_x100: u16,
    /// \u{8bf7}\u{6c42} ID (\u{8d28}\u{8bc1}\u{7528}, \u{9ed8}\u{8ba4} empty = \u{672a}\u{8d28}\u{8bc1})
    pub auth_token: String,
}

impl LlmRequest {
    /// \u{6784}\u{9020} \u{9ed8}\u{8ba4}\u{8bf7}\u{6c42}
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

    /// \u{9a8c}\u{8bc1}\u{8bf7}\u{6c42}\u{5408}\u{6cd5}\u{6027}
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

    /// \u{6e29}\u{5ea6}\u{8fd4}\u{56de} f64 (temperature_x100 / 100)
    pub fn temperature(&self) -> f64 {
        f64::from(self.temperature_x100) / 100.0
    }
}

/// LLM \u{54cd}\u{5e94}\u{7edf}\u{4e00}\u{5f62}\u{5f0f}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmResponse {
    /// \u{8bf7}\u{6c42} ID (\u{4e0e} LlmRequest \u{4e2d}\u{7684} trace_id \u{5bf9}\u{5e94})
    pub request_id: String,
    /// provider \u{540d}
    pub provider: String,
    /// model_kind
    pub model: String,
    /// \u{54cd}\u{5e94}\u{6587}\u{672c}
    pub text: String,
    /// prompt tokens
    pub prompt_tokens: u32,
    /// completion tokens
    pub completion_tokens: u32,
    /// \u{54cd}\u{5e94}\u{72b6}\u{6001} (ok / error)
    pub status: LlmStatus,
}

impl LlmResponse {
    /// \u{6784}\u{9020} ok \u{54cd}\u{5e94}
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

    /// \u{6784}\u{9020} error \u{54cd}\u{5e94}
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

    /// \u{603b} token \u{6570} (prompt + completion)
    pub fn total_tokens(&self) -> u32 {
        self.prompt_tokens + self.completion_tokens
    }
}

/// LLM \u{54cd}\u{5e94}\u{72b6}\u{6001}
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

/// LlmFacade \u{9519}\u{8bef}
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

/// LlmFacade trait \u{2014} 5 Provider \u{7edf}\u{4e00}\u{63a5}\u{5165}
///
/// **\u{7ea6}\u{5b9a}**: \u{6240}\u{6709} Provider \u{90fd}\u{5b9e}\u{73b0}\u{6b64} trait, \u{8c03}\u{7528}\u{65b9}\u{4e0d}\u{80fd}\u{76f4}\u{63a5}\u{8bbf}\u{95ee} provider \u{5b9e}\u{73b0}\u{3002}
/// \u{8fd9}\u{662f} ADR-0033 \u00a72.2 \u{7684}\u{5f3a}\u{5236}\u{70b9} \u{2014} \u{7edf}\u{4e00}\u{9274}\u{6743}/\u{9650}\u{6d41}/\u{534f}\u{8bae}\u{8f6c}\u{6362}\u{5728} facade \u{5904}\u{5b8c}\u{6210}.
pub trait LlmFacade: Send + Sync {
    /// Provider \u{540d} (\u{4e0e} ALL_PROVIDERS \u{5bf9}\u{9f50})
    fn name(&self) -> &'static str;

    /// \u{652f}\u{6301}\u{7684} model_kind \u{5217}\u{8868}
    fn supported_models(&self) -> Vec<&'static str>;

    /// \u{652f}\u{6301}\u{7684}\u{5de5}\u{5177}\u{540d}\u{5217}\u{8868}
    fn supported_tools(&self) -> Vec<&'static str>;

    /// \u{8c03}\u{5ea6}\u{8bf7}\u{6c42} \u{2014} \u{9ed8}\u{8ba4}\u{5b9e}\u{73b0}\u{4f1a}\u{9a8c}\u{8bc1} + \u{63a5}\u{5165} upstream + \u{8fd4}\u{54cd}\u{5e94}
    ///
    /// \u{7eaf}\u{51fd}\u{6570}\u{7c7b}\u{63a5}\u{53e3}, \u{4e0d}\u{4fee}\u{6539}\u{8bf7}\u{6c42}, \u{8fd4}\u{54cd}\u{5e94}\u{4e0e} upstream \u{4e00}\u{81f4}\u{3002}
    fn dispatch(&self, request: LlmRequest) -> Result<LlmResponse, LlmFacadeError>;

    /// \u{9ed8}\u{8ba4}\u{8c03}\u{5ea6} \u{2014} \u{8c03}\u{7528} dispatch \u{4e0e}\u{9a8c}\u{8bc1}
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

/// 6 Provider \u{540d}\u{5b57}\u{5217}\u{8868} (\u{4e0e} ALL_PROVIDERS \u{5bf9}\u{9f50})
pub const ALL_PROVIDER_NAMES: [&str; 6] = [
    "claude-code",
    "codex",
    "copilot",
    "gemini-cli",
    "opencode",
    "minimax",
];

/// \u{68c0}\u{67e5} provider \u{540d}\u{662f}\u{5426}\u{5408}\u{6cd5}
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