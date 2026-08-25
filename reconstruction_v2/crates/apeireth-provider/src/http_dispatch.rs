//! http_dispatch — 6 Provider HTTP 描述符 (descriptor-only).
//!
//! 0 装 PASS 严守: 真 ProviderConfig + status → FacadeStatus 映射 + config factory per provider.
//! 不抄真 HTTP, 写纯 logic (descriptor config + status 映射).
//!
//! v1 对齐 (R176): ProviderConfig + status_to_llm_status + 6 factory fn + configs_for_all.

use crate::claude_code::ClaudeCodeProvider;
use crate::codex::CodexProvider;
use crate::copilot::CopilotProvider;
use crate::gemini_cli::GeminiCliProvider;
use crate::minimax::MinimaxProvider;
use crate::opencode::OpencodeProvider;
use crate::facade_impls::FacadeStatus;

/// Provider 配置描述符 (api_key + base_url + default model).
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_name: &'static str,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
}

impl ProviderConfig {
    /// 构造 (explicit).
    pub fn new(provider_name: &'static str, base_url: impl Into<String>, api_key: impl Into<String>, default_model: impl Into<String>) -> Self {
        Self { provider_name, base_url: base_url.into(), api_key: api_key.into(), default_model: default_model.into() }
    }

    /// 校验 (per v1 K1_CHECKS): base_url 非空 + api_key 非空 + default_model 非空.
    pub fn validate(&self) -> Result<(), String> {
        if self.base_url.trim().is_empty() { return Err("base_url_not_empty".into()); }
        if self.api_key.trim().is_empty() { return Err("auth_token_format".into()); }
        if self.default_model.trim().is_empty() { return Err("default_model is empty".into()); }
        Ok(())
    }
}

/// HTTP status → FacadeStatus 映射 (per v1 status_to_llm_status).
pub fn status_to_facade_status(status: u16) -> FacadeStatus {
    match status {
        200..=299 => FacadeStatus::Ok,
        401 | 403 => FacadeStatus::InvalidAuth,
        429 => FacadeStatus::RateLimited,
        408 | 504 => FacadeStatus::Timeout,
        _ => FacadeStatus::Error,
    }
}

/// 6 个 config factory (per v1).
pub fn config_for_claude_code(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("claude-code", "https://api.anthropic.com", api_key, "claude-sonnet-4-5")
}
pub fn config_for_codex(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("codex", "https://api.openai.com", api_key, "codex")
}
pub fn config_for_copilot(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("copilot", "https://api.github.com", api_key, "gpt-4o")
}
pub fn config_for_gemini_cli(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("gemini-cli", "https://generativelanguage.googleapis.com", api_key, "gemini-pro")
}
pub fn config_for_opencode(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("opencode", "https://api.opencode.ai", api_key, "opencode-default")
}
pub fn config_for_minimax(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("minimax", "https://api.minimaxi.com", api_key, "MiniMax-M3")
}

/// 6 Provider 都生成 config.
pub fn configs_for_all(api_key: impl Into<String>) -> Vec<ProviderConfig> {
    let key = api_key.into();
    vec![
        config_for_claude_code(key.clone()),
        config_for_codex(key.clone()),
        config_for_copilot(key.clone()),
        config_for_gemini_cli(key.clone()),
        config_for_opencode(key.clone()),
        config_for_minimax(key),
    ]
}

/// 与 Provider descriptor 配对 (model_kinds 校验) — 复用 facade dispatch 的 model 校验逻辑。
pub fn validate_config_against_descriptor(cfg: &ProviderConfig, model: &str) -> Result<(), String> {
    cfg.validate()?;
    let valid = match cfg.provider_name {
        "claude-code" => ClaudeCodeProvider::new().model_kinds.iter().any(|m| *m == model),
        "codex" => CodexProvider::new().model_kinds.iter().any(|m| *m == model),
        "copilot" => CopilotProvider::new().model_kinds.iter().any(|m| *m == model),
        "gemini-cli" => GeminiCliProvider::new().model_kinds.iter().any(|m| *m == model),
        "opencode" => OpencodeProvider::new().model_kinds.iter().any(|m| *m == model),
        "minimax" => MinimaxProvider::new().model_kinds.iter().any(|m| *m == model),
        _ => false,
    };
    if valid { Ok(()) } else { Err(format!("model `{model}` not in descriptor for `{}`", cfg.provider_name)) }
}

#[cfg(test)]
mod http_dispatch_tests {
    use super::*;

    #[test]
    fn provider_config_new() {
        let c = ProviderConfig::new("test", "https://api.test.com", "key123", "model-x");
        assert_eq!(c.provider_name, "test");
        assert_eq!(c.api_key, "key123");
    }

    #[test]
    fn validate_rejects_empty() {
        let c = ProviderConfig::new("t", "", "k", "m");
        assert!(c.validate().is_err());
        let c = ProviderConfig::new("t", "u", "", "m");
        assert!(c.validate().is_err());
        let c = ProviderConfig::new("t", "u", "k", "");
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_accepts_filled() {
        let c = ProviderConfig::new("t", "u", "k", "m");
        assert!(c.validate().is_ok());
    }

    #[test]
    fn status_mapping_ok() {
        assert_eq!(status_to_facade_status(200), FacadeStatus::Ok);
        assert_eq!(status_to_facade_status(201), FacadeStatus::Ok);
        assert_eq!(status_to_facade_status(299), FacadeStatus::Ok);
    }

    #[test]
    fn status_mapping_auth() {
        assert_eq!(status_to_facade_status(401), FacadeStatus::InvalidAuth);
        assert_eq!(status_to_facade_status(403), FacadeStatus::InvalidAuth);
    }

    #[test]
    fn status_mapping_rate_limit() {
        assert_eq!(status_to_facade_status(429), FacadeStatus::RateLimited);
    }

    #[test]
    fn status_mapping_timeout() {
        assert_eq!(status_to_facade_status(408), FacadeStatus::Timeout);
        assert_eq!(status_to_facade_status(504), FacadeStatus::Timeout);
    }

    #[test]
    fn status_mapping_error() {
        assert_eq!(status_to_facade_status(500), FacadeStatus::Error);
        assert_eq!(status_to_facade_status(503), FacadeStatus::Error);
    }

    #[test]
    fn configs_for_all_returns_6() {
        let v = configs_for_all("test-key");
        assert_eq!(v.len(), 6);
        for c in &v {
            assert!(!c.api_key.is_empty());
            assert!(!c.base_url.is_empty());
        }
    }

    #[test]
    fn each_provider_has_factory() {
        let _ = config_for_claude_code("k");
        let _ = config_for_codex("k");
        let _ = config_for_copilot("k");
        let _ = config_for_gemini_cli("k");
        let _ = config_for_opencode("k");
        let _ = config_for_minimax("k");
    }

    #[test]
    fn validate_against_descriptor_known() {
        let cfg = config_for_claude_code("k");
        assert!(validate_config_against_descriptor(&cfg, "claude-sonnet-4-5").is_ok());
        assert!(validate_config_against_descriptor(&cfg, "unknown-model").is_err());
    }

    #[test]
    fn validate_against_descriptor_each_provider() {
        assert!(validate_config_against_descriptor(&config_for_codex("k"), "codex").is_ok());
        assert!(validate_config_against_descriptor(&config_for_copilot("k"), "gpt-4o").is_ok());
        assert!(validate_config_against_descriptor(&config_for_gemini_cli("k"), "gemini-pro").is_ok());
        assert!(validate_config_against_descriptor(&config_for_opencode("k"), "opencode-default").is_ok());
        assert!(validate_config_against_descriptor(&config_for_minimax("k"), "MiniMax-M3").is_ok());
    }
}
