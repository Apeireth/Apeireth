//! `ApeirethApiProvider` — minimaxi 专有 LLM provider (走 OpenAI Chat Completion 协议)
//!
//! **协议**: OpenAI Chat Completion API (跟 OpenAI / Together / vLLM / 等同协议)
//! **默认目标**: `https://api.minimaxi.com/v1` (minimaxi 开放平台, OpenAI 协议端点)
//! **用途**: Apeireth 默认 LLM provider, minimaxi 已验证 (R16-13 真接通, R17 持续维护)
//!
//! **跟 OpenAiCompatibleProvider 的区别**:
//! - `ApeirethApiProvider`: minimaxi 专有 (默认 base_url + minimaxi model 列表 + 严格 model 白名单)
//! - `OpenAiCompatibleProvider`: 通用 (任意 base_url, 任意 model 任意通过)
//!
//! **R17 改造**:
//! - ❌ 删 NewAPI `localhost:3000/v1` 默认 (R16 强依赖)
//! - ❌ 删 `api_user_id` 字段 (NewAPI `New-Api-User` header 专有)
//! - ❌ 删 `with_api_user_id` 构造方法
//! - ✅ 默认 base_url = `https://api.minimaxi.com/v1`
//! - ✅ 默认 models = `["MiniMax-M3", "MiniMax-M3-thinking"]`
//!
//! **环境变量**:
//! - `APEIRETH_API_KEY` — API 密钥 (主人给)
//! - `APEIRETH_API_URL` — base_url (默认 `https://api.minimaxi.com/v1`)
//! - `APEIRETH_API_MODELS` — 支持的 model 列表 (逗号分隔, 默认 `MiniMax-M3,MiniMax-M3-thinking`)

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::Deserialize;

use crate::llm::error::LlmError;
use crate::llm::traits::{
    ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, TokenUsage,
};

/// 默认 base_url (R17 直连 minimaxi OpenAI 协议端点)
const DEFAULT_BASE_URL: &str = "https://api.minimaxi.com/v1";
const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_BACKOFF_MS: u64 = 500;
const PROVIDER_NAME: &str = "apeireth-api";

// ============================================================
// 配置
// ============================================================

/// ApeirethApiProvider 配置
#[derive(Debug, Clone)]
pub struct ApeirethApiConfig {
    /// base_url (e.g. `https://api.minimaxi.com/v1`)
    pub base_url: String,
    /// API 密钥 (bearer auth)
    pub api_key: String,
    /// 支持的 model 列表 (严格白名单)
    pub models: Vec<String>,
    /// HTTP 超时 (毫秒)
    pub timeout_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

impl ApeirethApiConfig {
    /// 从环境变量构造 (APEIRETH_API_KEY / APEIRETH_API_URL / APEIRETH_API_MODELS)
    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("APEIRETH_API_KEY")
            .map_err(|_| LlmError::Config("APEIRETH_API_KEY env var not set".into()))?;
        let base_url =
            std::env::var("APEIRETH_API_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let models = std::env::var("APEIRETH_API_MODELS")
            .unwrap_or_else(|_| "MiniMax-M3,MiniMax-M3-thinking".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Ok(Self {
            base_url,
            api_key,
            models,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_retries: DEFAULT_MAX_RETRIES,
        })
    }

    /// 直接构造 (用于测试 / 编程方式配置)
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        models: Vec<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            models,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_retries: DEFAULT_MAX_RETRIES,
        }
    }
}

// ============================================================
// Provider 实现
// ============================================================

/// minimaxi 专有 LLM provider
pub struct ApeirethApiProvider {
    config: ApeirethApiConfig,
    http: reqwest::Client,
}

impl ApeirethApiProvider {
    pub fn new(config: ApeirethApiConfig) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| LlmError::Config(format!("reqwest client build failed: {e}")))?;
        Ok(Self { config, http })
    }

    pub fn from_env() -> Result<Self, LlmError> {
        Self::new(ApeirethApiConfig::from_env()?)
    }

    /// 列出支持的 models
    #[allow(dead_code)]
    pub fn models(&self) -> &[String] {
        &self.config.models
    }
}

#[async_trait]
impl LlmProvider for ApeirethApiProvider {
    fn name(&self) -> &str {
        PROVIDER_NAME
    }

    fn supports_model(&self, model: &str) -> bool {
        self.config.models.iter().any(|m| m == model)
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::CHAT
            | ProviderCapabilities::SYSTEM_PROMPT
            | ProviderCapabilities::CUSTOM_TEMPERATURE
            | ProviderCapabilities::JSON_MODE
    }

    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/chat/completions", self.config.base_url);
        let body = serde_json::json!({
            "model": req.model,
            "messages": req.messages.iter().map(|m| match m {
                ChatMessage { role: ChatRole::System, content } => serde_json::json!({"role": "system", "content": content}),
                ChatMessage { role: ChatRole::User, content } => serde_json::json!({"role": "user", "content": content}),
                ChatMessage { role: ChatRole::Assistant, content } => serde_json::json!({"role": "assistant", "content": content}),
            }).collect::<Vec<_>>(),
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        });

        let mut last_err: Option<LlmError> = None;
        for attempt in 0..=self.config.max_retries {
            let start = Instant::now();
            let result = self
                .http
                .post(&url)
                .bearer_auth(&self.config.api_key)
                .json(&body)
                .send()
                .await;

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        let retry_after_ms = resp
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(1000);
                        tracing::warn!(provider = PROVIDER_NAME, retry_after_ms, "rate limited");
                        last_err = Some(LlmError::RateLimited {
                            retry_after_ms,
                            provider: PROVIDER_NAME.to_string(),
                        });
                        tokio::time::sleep(Duration::from_millis(retry_after_ms)).await;
                        continue;
                    }
                    if status == reqwest::StatusCode::UNAUTHORIZED
                        || status == reqwest::StatusCode::FORBIDDEN
                    {
                        let text = resp.text().await.unwrap_or_default();
                        return Err(LlmError::AuthFailed(format!(
                            "{} returned {}: {}",
                            PROVIDER_NAME, status, text
                        )));
                    }
                    if !status.is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        return Err(LlmError::BadResponse {
                            provider: PROVIDER_NAME.to_string(),
                            detail: format!("status {}: {}", status, text),
                            status_code: Some(status.as_u16()),
                        });
                    }
                    let body: serde_json::Value =
                        resp.json().await.map_err(|e| LlmError::BadResponse {
                            provider: PROVIDER_NAME.to_string(),
                            detail: format!("json parse: {e}"),
                            status_code: None,
                        })?;
                    return parse_openai_response(
                        body,
                        start.elapsed().as_millis() as u64,
                        PROVIDER_NAME,
                    );
                }
                Err(e) if e.is_timeout() => {
                    tracing::warn!(provider = PROVIDER_NAME, attempt, "timeout");
                    last_err = Some(LlmError::Timeout {
                        timeout_ms: self.config.timeout_ms,
                        provider: PROVIDER_NAME.to_string(),
                    });
                    let backoff = DEFAULT_RETRY_BACKOFF_MS * (1 << attempt.min(5));
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
                Err(e) => {
                    tracing::warn!(provider = PROVIDER_NAME, attempt, error = %e, "network error");
                    last_err = Some(LlmError::Network {
                        provider: PROVIDER_NAME.to_string(),
                        detail: e.to_string(),
                    });
                    let backoff = DEFAULT_RETRY_BACKOFF_MS * (1 << attempt.min(5));
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| LlmError::ProviderExhausted {
            provider: PROVIDER_NAME.to_string(),
            attempts: self.config.max_retries + 1,
            last_error: None,
        }))
    }

    /// **战役 4-1 真流式 (R17-2026-08-04)**: minimaxi OpenAI 协议 SSE 推流.
    ///
    /// 跟 `OpenAiCompatibleProvider::complete_stream` 同款 (都是 OpenAI Chat Completion 协议),
    /// 唯一区别: base_url 用 `self.config.base_url` (minimaxi 专有), auth header 同样 Bearer.
    async fn complete_stream(
        &self,
        req: LlmRequest,
    ) -> Result<futures::stream::BoxStream<'static, Result<String, LlmError>>, LlmError> {
        use futures::stream::{self, StreamExt};

        let url = format!("{}/chat/completions", self.config.base_url);
        let body = serde_json::json!({
            "model": req.model,
            "stream": true,
            "messages": req.messages.iter().map(|m| match m {
                ChatMessage { role: ChatRole::System, content } => serde_json::json!({"role": "system", "content": content}),
                ChatMessage { role: ChatRole::User, content } => serde_json::json!({"role": "user", "content": content}),
                ChatMessage { role: ChatRole::Assistant, content } => serde_json::json!({"role": "assistant", "content": content}),
            }).collect::<Vec<_>>(),
            "temperature": req.temperature,
            "max_tokens": req.max_tokens,
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                provider: PROVIDER_NAME.to_string(),
                detail: e.to_string(),
            })?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::AuthFailed(text));
        }
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(LlmError::BadResponse {
                provider: PROVIDER_NAME.to_string(),
                detail: format!("status {}: {}", status, text),
                status_code: Some(status.as_u16()),
            });
        }

        // 复用 openai_compat 的 parse 函数 (SSE 协议同款, 不重复造轮子)
        let provider_name = PROVIDER_NAME.to_string();
        let byte_stream = resp.bytes_stream();
        let s = stream::unfold(
            (byte_stream, false, String::new()),
            move |(mut byte_stream, mut done, mut buffer)| {
                let provider_name = provider_name.clone();
                async move {
                    if done {
                        return None;
                    }
                    loop {
                        if let Some(event_end) = buffer.find("\n\n") {
                            let event: String = buffer.drain(..event_end + 2).collect();
                            let mut stream_ended = false;
                            let mut chunk_to_emit: Option<String> = None;
                            for line in event.lines() {
                                if let Some(maybe_content) =
                                    super::openai_compat::parse_openai_sse_data_line(line)
                                {
                                    match maybe_content {
                                        Some(content) => {
                                            if !content.is_empty() {
                                                chunk_to_emit = Some(content);
                                            }
                                        }
                                        None => {
                                            stream_ended = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            if stream_ended {
                                return Some((Ok(String::new()), (byte_stream, true, buffer)));
                            }
                            if let Some(content) = chunk_to_emit {
                                return Some((Ok(content), (byte_stream, false, buffer)));
                            }
                            continue;
                        }
                        match byte_stream.next().await {
                            Some(Ok(bytes)) => {
                                buffer.push_str(&String::from_utf8_lossy(&bytes));
                            }
                            Some(Err(e)) => {
                                let err = LlmError::Network {
                                    provider: provider_name,
                                    detail: e.to_string(),
                                };
                                return Some((Err(err), (byte_stream, true, buffer)));
                            }
                            None => {
                                if !buffer.trim().is_empty() {
                                    let last_event = std::mem::take(&mut buffer);
                                    for line in last_event.lines() {
                                        if let Some(Some(content)) =
                                            super::openai_compat::parse_openai_sse_data_line(line)
                                        {
                                            if !content.is_empty() {
                                                return Some((
                                                    Ok(content),
                                                    (byte_stream, true, buffer),
                                                ));
                                            }
                                        }
                                    }
                                }
                                return None;
                            }
                        }
                    }
                }
            },
        );

        Ok(Box::pin(s))
    }
}

// ============================================================
// 响应解析 (OpenAI Chat Completion 协议)
// ============================================================

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    #[serde(default)]
    finish_reason: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    #[serde(default)]
    choices: Vec<OpenAiChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
    #[serde(default)]
    model: String,
}

fn parse_openai_response(
    body: serde_json::Value,
    latency_ms: u64,
    provider: &str,
) -> Result<LlmResponse, LlmError> {
    let parsed: OpenAiResponse =
        serde_json::from_value(body).map_err(|e| LlmError::BadResponse {
            provider: provider.to_string(),
            detail: format!("response parse: {e}"),
            status_code: None,
        })?;

    let choice = parsed
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| LlmError::BadResponse {
            provider: provider.to_string(),
            detail: "empty choices".into(),
            status_code: None,
        })?;

    let usage = parsed
        .usage
        .map(|u| TokenUsage::new(u.prompt_tokens, u.completion_tokens))
        .unwrap_or_default();

    let model = if parsed.model.is_empty() {
        "unknown".into()
    } else {
        parsed.model
    };

    Ok(LlmResponse {
        content: choice.message.content,
        usage,
        latency_ms,
        model,
        finish_reason: if choice.finish_reason.is_empty() {
            "stop".into()
        } else {
            choice.finish_reason
        },
        provider: provider.to_string(),
    })
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::traits::ChatMessage;

    #[test]
    fn test_config_from_env_missing() {
        // 没设 APEIRETH_API_KEY 应该报 Config 错
        std::env::remove_var("APEIRETH_API_KEY");
        let result = ApeirethApiConfig::from_env();
        assert!(matches!(result, Err(LlmError::Config(_))));
    }

    /// R17 不假装验证: 默认 base_url 必须直连 minimaxi,
    /// 不能回退到 NewAPI localhost:3000。
    #[test]
    fn test_default_base_url_is_minimaxi() {
        assert!(
            DEFAULT_BASE_URL.starts_with("https://api.minimaxi.com/"),
            "R17 不假装: 默认 base_url 必须直连 minimaxi, 不依赖 NewAPI. 实际: {}",
            DEFAULT_BASE_URL
        );
        assert!(
            !DEFAULT_BASE_URL.contains("localhost"),
            "R17 不假装: 默认 base_url 不能是 localhost (那是 NewAPI 时代)"
        );
    }

    #[test]
    fn test_parse_openai_response() {
        let body = serde_json::json!({
            "id": "chatcmpl-xxx",
            "object": "chat.completion",
            "created": 1234567890,
            "model": "MiniMax-M3",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello back"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        });
        let resp = parse_openai_response(body, 100, PROVIDER_NAME).unwrap();
        assert_eq!(resp.content, "Hello back");
        assert_eq!(resp.usage.prompt_tokens, 10);
        assert_eq!(resp.usage.completion_tokens, 5);
        assert_eq!(resp.usage.total_tokens, 15);
        assert_eq!(resp.model, "MiniMax-M3");
        assert_eq!(resp.provider, PROVIDER_NAME);
        assert_eq!(resp.finish_reason, "stop");
        assert_eq!(resp.latency_ms, 100);
    }

    #[test]
    fn test_capabilities() {
        let config =
            ApeirethApiConfig::new("test-key", "https://api.minimaxi.com/v1", vec!["m".into()]);
        let provider = ApeirethApiProvider::new(config).unwrap();
        let caps = provider.capabilities();
        assert!(caps.contains(ProviderCapabilities::CHAT));
        assert!(caps.contains(ProviderCapabilities::SYSTEM_PROMPT));
        assert!(caps.contains(ProviderCapabilities::JSON_MODE));
        assert!(!caps.contains(ProviderCapabilities::VISION));
    }

    #[test]
    fn test_supports_model() {
        let config = ApeirethApiConfig::new(
            "test-key",
            "https://api.minimaxi.com/v1",
            vec!["MiniMax-M3".into(), "MiniMax-M3-thinking".into()],
        );
        let provider = ApeirethApiProvider::new(config).unwrap();
        assert!(provider.supports_model("MiniMax-M3"));
        assert!(provider.supports_model("MiniMax-M3-thinking"));
        assert!(!provider.supports_model("gpt-4"));
    }
}
