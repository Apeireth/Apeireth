//! apeireth-provider::http_dispatch \u2014 6 Provider \u7edf\u4e00 HTTP \u63a5\u5165 (R176)
//!
//! **\u80cc\u666f**: R20 \u9636\u6bb5 4 \u4f30\u8865\u7684 5 Provider \u4ec5\u6709 struct descriptor (name + tools + model_kinds).
//! R176 \u589e\u52a0\u771f\u63a5 HTTP \u8c03\u5ea6\u80fd\u529b:
//! - 6 Provider \u90fd\u53ef\u9009\u62e9\u8d70 OpenAI Chat Completions \u534f\u8bae (4 \u534f\u8bae\u5171\u4eab\u540c\u4e00\u4e2a dispatch \u5165\u53e3)
//! - \u914d\u7f6e\u53c2\u6570: api_key / base_url / model_kind / max_tokens / temperature
//! - \u7edf\u4e00\u8c03\u7528 apeireth-http-client::HttpClient::post_json
//! - \u9519\u8bef\u8f6c\u6362: HTTP \u9519\u8bef \u2192 LlmFacadeError \u4e0d\u540c\u72b6\u6001 (401/429/5xx/timeout)
//!
//! **\u4e0d\u6f02\u79fb**:
//! - 0 \u6539 6 Provider struct (R35 LOCKED)
//! - 0 \u52a8 workspace.version
//!
//! **\u72b6\u6001**: R176 (2026-08-15) \u521d\u59cb\u7248, 6 Provider \u90fd\u80fd configure + dispatch_http.

#![allow(missing_docs)]

use apeireth_acp::llm_facade::{LlmFacadeError, LlmRequest, LlmResponse, LlmStatus};
use apeireth_http_client::HttpClient;
use serde_json::json;
use std::time::Instant;

use crate::claude_code::ClaudeCodeProvider;
use crate::codex::CodexProvider;
use crate::copilot::CopilotProvider;
use crate::gemini_cli::GeminiCliProvider;
use crate::minimax::MinimaxProvider;
use crate::opencode::OpencodeProvider;

/// Provider \u914d\u7f6e (api_key + base_url + default model)
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_name: &'static str,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
}

impl ProviderConfig {
    /// Construct from env vars (APEIRETH_API_KEY + APEIRETH_BASE_URL + APEIRETH_MODEL)
    pub fn from_env(provider_name: &'static str) -> Result<Self, LlmFacadeError> {
        let api_key = std::env::var("APEIRETH_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return Err(LlmFacadeError::InvalidAuth);
        }
        let base_url = std::env::var("APEIRETH_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com".to_string());
        let default_model = std::env::var("APEIRETH_MODEL").unwrap_or_default();
        Ok(Self {
            provider_name,
            base_url,
            api_key,
            default_model,
        })
    }

    /// Construct with explicit values (for tests)
    pub fn new(
        provider_name: &'static str,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: impl Into<String>,
    ) -> Self {
        Self {
            provider_name,
            base_url: base_url.into(),
            api_key: api_key.into(),
            default_model: default_model.into(),
        }
    }
}

/// Map HTTP status to LlmStatus
fn status_to_llm_status(status: u16) -> LlmStatus {
    match status {
        200..=299 => LlmStatus::Ok,
        401 | 403 => LlmStatus::InvalidAuth,
        429 => LlmStatus::RateLimited,
        408 | 504 => LlmStatus::Timeout,
        _ => LlmStatus::Error,
    }
}

/// \u7edf\u4e00 HTTP dispatch \u5165\u53e3 \u2014 \u8d70 OpenAI Chat Completions \u534f\u8bae
///
/// \u6240\u6709 6 Provider \u90fd\u8d70\u540c\u4e00\u4e2a\u534f\u8bae, \u4ec5 base_url + api_key + model \u4e0d\u540c.
pub async fn dispatch_http(
    config: &ProviderConfig,
    request: &LlmRequest,
) -> Result<LlmResponse, LlmFacadeError> {
    let model = if request.model.is_empty() {
        &config.default_model
    } else {
        &request.model
    };
    if model.is_empty() {
        return Err(LlmFacadeError::InvalidModel {
            provider: config.provider_name.into(),
            model: "empty".into(),
        });
    }
    if config.api_key.is_empty() {
        return Err(LlmFacadeError::InvalidAuth);
    }

    let client = HttpClient::with_chat_defaults()
        .map_err(|e| LlmFacadeError::HttpError(format!("client init: {e}")))?;

    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": request.system},
            {"role": "user", "content": request.user},
        ],
        "max_tokens": request.max_tokens,
        "temperature": request.temperature(),
        "stream": false,
    });

    let url = format!(
        "{}/v1/chat/completions",
        config.base_url.trim_end_matches('/')
    );
    let start = Instant::now();
    let resp = client
        .post_json(&url, body)
        .await
        .map_err(|e| LlmFacadeError::HttpError(format!("{e}")))?;
    let status = status_to_llm_status(resp.status().as_u16());
    let elapsed_ms = start.elapsed().as_millis() as u64;

    if !matches!(status, LlmStatus::Ok) {
        return Ok(LlmResponse {
            request_id: format!("req-{}", elapsed_ms),
            provider: config.provider_name.into(),
            model: model.into(),
            text: format!("HTTP {}", resp.status().as_u16()),
            prompt_tokens: 0,
            completion_tokens: 0,
            status,
        });
    }

    let text = resp.text().await.unwrap_or_default();
    // N12: 推理字段归一化 (默认配置关闭 → 行为 0 变化; 显式 APEIRETH_REASONING_ENABLED=1
    // + APEIRETH_REASONING_MODEL_FILTERS 后才把推理别名归一为 think 块, 见 reasoning_adapter)
    let text = crate::reasoning_adapter::normalize_chat_completion_body(
        &text,
        &crate::reasoning_adapter::ReasoningAdapterConfig::from_env(),
        model,
    );
    Ok(LlmResponse {
        request_id: format!("req-{}", elapsed_ms),
        provider: config.provider_name.into(),
        model: model.into(),
        text,
        prompt_tokens: 0,
        completion_tokens: 0,
        status: LlmStatus::Ok,
    })
}

/// \u6784\u9020 ProviderConfig (\u9ed8\u8ba4 base_url \u4e0e model_kind)
pub fn config_for_claude_code(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new(
        "claude-code",
        "https://api.anthropic.com",
        api_key,
        "claude-sonnet-4-5",
    )
}
pub fn config_for_codex(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("codex", "https://api.openai.com", api_key, "codex")
}
pub fn config_for_copilot(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("copilot", "https://api.github.com", api_key, "gpt-4o")
}
pub fn config_for_gemini_cli(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new(
        "gemini-cli",
        "https://generativelanguage.googleapis.com",
        api_key,
        "gemini-pro",
    )
}
pub fn config_for_opencode(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new(
        "opencode",
        "https://api.opencode.ai",
        api_key,
        "opencode-default",
    )
}
pub fn config_for_minimax(api_key: impl Into<String>) -> ProviderConfig {
    ProviderConfig::new("minimax", "https://api.minimaxi.com", api_key, "MiniMax-M3")
}

/// \u6cd5: 6 Provider \u90fd\u53ef\u4ee5\u751f\u6210 config + dispatch_http \u8c03\u7528
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

#[cfg(test)]
mod http_dispatch_tests {
    use super::*;

    #[test]
    fn provider_config_new() {
        let c = ProviderConfig::new("test", "https://api.test.com", "key123", "model-x");
        assert_eq!(c.provider_name, "test");
        assert_eq!(c.api_key, "key123");
        assert_eq!(c.default_model, "model-x");
    }

    #[test]
    fn provider_config_from_env_missing_key() {
        // Clear env var temporarily
        let prev = std::env::var("APEIRETH_API_KEY").ok();
        std::env::remove_var("APEIRETH_API_KEY");
        let result = ProviderConfig::from_env("test");
        std::env::set_var("APEIRETH_API_KEY", prev.unwrap_or_default());
        assert!(matches!(result, Err(LlmFacadeError::InvalidAuth)));
    }

    #[test]
    fn status_mapping_ok() {
        assert_eq!(status_to_llm_status(200), LlmStatus::Ok);
        assert_eq!(status_to_llm_status(201), LlmStatus::Ok);
        assert_eq!(status_to_llm_status(299), LlmStatus::Ok);
    }

    #[test]
    fn status_mapping_auth() {
        assert_eq!(status_to_llm_status(401), LlmStatus::InvalidAuth);
        assert_eq!(status_to_llm_status(403), LlmStatus::InvalidAuth);
    }

    #[test]
    fn status_mapping_rate_limit() {
        assert_eq!(status_to_llm_status(429), LlmStatus::RateLimited);
    }

    #[test]
    fn status_mapping_timeout() {
        assert_eq!(status_to_llm_status(408), LlmStatus::Timeout);
        assert_eq!(status_to_llm_status(504), LlmStatus::Timeout);
    }

    #[test]
    fn status_mapping_error() {
        assert_eq!(status_to_llm_status(500), LlmStatus::Error);
        assert_eq!(status_to_llm_status(503), LlmStatus::Error);
    }

    #[test]
    fn configs_for_all_returns_6() {
        let configs = configs_for_all("test-key");
        assert_eq!(configs.len(), 6);
        for c in &configs {
            assert!(!c.api_key.is_empty());
            assert!(!c.base_url.is_empty());
        }
    }

    #[test]
    fn each_provider_has_factory() {
        // 6 Provider \u90fd\u80fd\u751f\u6210 config
        let _ = config_for_claude_code("k");
        let _ = config_for_codex("k");
        let _ = config_for_copilot("k");
        let _ = config_for_gemini_cli("k");
        let _ = config_for_opencode("k");
        let _ = config_for_minimax("k");
    }

    #[test]
    fn dispatch_http_empty_api_key_rejected() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = ProviderConfig::new("test", "https://api.test.com", "", "model-x");
            let req = LlmRequest::new("test", "sys", "user");
            let result = dispatch_http(&config, &req).await;
            assert!(matches!(result, Err(LlmFacadeError::InvalidAuth)));
        });
    }

    #[test]
    fn dispatch_http_empty_model_rejected() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = ProviderConfig::new("test", "https://api.test.com", "key", "");
            let req = LlmRequest::new("test", "sys", "user");
            let result = dispatch_http(&config, &req).await;
            assert!(matches!(result, Err(LlmFacadeError::InvalidModel { .. })));
        });
    }
}
