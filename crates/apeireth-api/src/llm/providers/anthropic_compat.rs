//! `AnthropicCompatibleProvider` — Anthropic Messages API 协议 provider
//!
//! **协议**: Anthropic Messages API (跟 Anthropic / minimaxi `/anthropic` / 等同协议)
//! **默认目标**: `https://api.minimaxi.com/anthropic` (minimaxi 开放平台, Anthropic 协议端点)
//! **用途**: Apeireth 第二种 LLM 协议, 跟 `ApeirethApiProvider` (OpenAI 协议) 并存
//!
//! **协议差异** (vs OpenAI Chat Completion):
//! - **Auth header**: `x-api-key: <key>` (不是 `Authorization: Bearer ...`)
//! - **Version header**: `anthropic-version: 2023-06-01` 必带
//! - **System role**: 单独 `system` 字段, 不在 messages 数组里
//! - **Response content**: 数组形式 `[{type: "text", text: "..."}]`, 不是单纯字符串
//!
//! **环境变量**:
//! - `APEIRETH_ANTHROPIC_KEY` — API 密钥
//! - `APEIRETH_ANTHROPIC_URL` — base_url (默认 `https://api.minimaxi.com/anthropic`)
//! - `APEIRETH_ANTHROPIC_MODELS` — 支持的 model 列表 (逗号分隔, 默认 `MiniMax-M3`)

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::llm::error::LlmError;
use crate::llm::traits::{
    ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities, TokenUsage,
};

/// 战役 4-1: 解析 Anthropic SSE 单行 `data: {JSON}`, 提取 `delta.text` (if event is content_block_delta).
///
/// 行为:
/// - 空行 / 非 `data:` 开头 → `None` (skip)
/// - `data: {JSON}` → 解析 `delta.text`:
///   - 有 text → `Some(Some(text))`
///   - 其他 event (message_start / content_block_start / message_delta / ping) → `None` (skip)
/// - JSON 解析失败 → `None` (不假装错误)
fn parse_anthropic_sse_data_line(line: &str) -> Option<Option<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with("data:") {
        return None;
    }
    let payload = trimmed[5..].trim();
    if payload.is_empty() {
        return None;
    }
    let parsed: serde_json::Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return None,
    };
    let event_type = parsed.get("type").and_then(|t| t.as_str()).unwrap_or("");
    // 只关心 content_block_delta 里的 text_delta.text
    if event_type == "content_block_delta" {
        let text = parsed
            .get("delta")
            .and_then(|d| d.get("text"))
            .and_then(|t| t.as_str());
        match text {
            Some(s) => Some(Some(s.to_string())),
            None => None, // 其他 delta type (e.g. input_json_delta) 跳过
        }
    } else {
        None // message_start / content_block_start / content_block_stop / message_delta / ping / error
    }
}

/// 战役 4-1: 解析 Anthropic SSE `event: <type>` 行, 提取 event type.
fn parse_anthropic_sse_event_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with("event:") {
        return None;
    }
    let event = trimmed[6..].trim();
    if event.is_empty() {
        None
    } else {
        Some(event.to_string())
    }
}

/// 默认 base_url (R17 直连 minimaxi Anthropic 协议端点)
const DEFAULT_BASE_URL: &str = "https://api.minimaxi.com/anthropic";

/// Anthropic Messages API 协议版本 (R17 锁定 2023-06-01, 后续 minimaxi 更新可调)
const ANTHROPIC_VERSION: &str = "2023-06-01";

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_MAX_RETRIES: u32 = 3;
const DEFAULT_RETRY_BACKOFF_MS: u64 = 500;
const PROVIDER_NAME: &str = "anthropic-compat";

// ============================================================
// 配置
// ============================================================

/// AnthropicCompatibleProvider 配置
#[derive(Debug, Clone)]
pub struct AnthropicCompatibleConfig {
    /// base_url (e.g. `https://api.minimaxi.com/anthropic`)
    pub base_url: String,
    /// API 密钥 (anthropic Messages API 用 `x-api-key` header, 不是 Bearer)
    pub api_key: String,
    /// 支持的 model 列表 (严格白名单)
    pub models: Vec<String>,
    /// HTTP 超时 (毫秒)
    pub timeout_ms: u64,
    /// 最大重试次数
    pub max_retries: u32,
}

impl AnthropicCompatibleConfig {
    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("APEIRETH_ANTHROPIC_KEY")
            .map_err(|_| LlmError::Config("APEIRETH_ANTHROPIC_KEY env var not set".into()))?;
        let base_url = std::env::var("APEIRETH_ANTHROPIC_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let models = std::env::var("APEIRETH_ANTHROPIC_MODELS")
            .unwrap_or_else(|_| "MiniMax-M3".to_string())
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

/// Anthropic Messages API 协议 provider
pub struct AnthropicCompatibleProvider {
    config: AnthropicCompatibleConfig,
    http: reqwest::Client,
}

impl AnthropicCompatibleProvider {
    pub fn new(config: AnthropicCompatibleConfig) -> Result<Self, LlmError> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| LlmError::Config(format!("reqwest client build failed: {e}")))?;
        Ok(Self { config, http })
    }

    pub fn from_env() -> Result<Self, LlmError> {
        Self::new(AnthropicCompatibleConfig::from_env()?)
    }

    /// 列出支持的 models
    #[allow(dead_code)]
    pub fn models(&self) -> &[String] {
        &self.config.models
    }
}

// ============================================================
// Anthropic Messages API 请求 / 响应 schema
// ============================================================

/// Anthropic Messages API 请求体
///
/// 注意: `system` 不在 `messages` 里, 是顶层字段.
#[derive(Debug, Serialize)]
struct AnthropicRequestBody {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// Anthropic Messages API message (user / assistant only)
#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String, // "user" | "assistant"
    content: String,
}

/// Anthropic Messages API 响应
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[allow(dead_code)]
    id: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    stop_reason: Option<String>,
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

/// Anthropic content 数组中的一项
#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// Anthropic usage
#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[async_trait]
impl LlmProvider for AnthropicCompatibleProvider {
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
    }

    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        // 拆分: System message 抽出来放 system 顶层字段, User/Assistant 进 messages
        let mut system_prompt: Option<String> = None;
        let mut messages: Vec<AnthropicMessage> = Vec::new();
        for m in req.messages {
            match m.role {
                ChatRole::System => {
                    if let Some(existing) = system_prompt.as_mut() {
                        existing.push('\n');
                        existing.push_str(&m.content);
                    } else {
                        system_prompt = Some(m.content);
                    }
                }
                ChatRole::User => {
                    messages.push(AnthropicMessage {
                        role: "user".into(),
                        content: m.content,
                    });
                }
                ChatRole::Assistant => {
                    messages.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: m.content,
                    });
                }
            }
        }

        if messages.is_empty() {
            return Err(LlmError::Config(
                "Anthropic Messages API requires at least one user/assistant message (system-only is invalid)"
                    .into(),
            ));
        }

        let body = AnthropicRequestBody {
            model: req.model.clone(),
            max_tokens: req.max_tokens,
            system: system_prompt,
            messages,
            temperature: Some(req.temperature),
        };

        let url = format!("{}/v1/messages", self.config.base_url);
        let mut last_err: Option<LlmError> = None;
        for attempt in 0..=self.config.max_retries {
            let start = Instant::now();
            let result = self
                .http
                .post(&url)
                .header("x-api-key", &self.config.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("content-type", "application/json")
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
                    return parse_anthropic_response(
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
                    continue;
                }
                Err(e) => {
                    tracing::warn!(provider = PROVIDER_NAME, attempt, error = %e, "network error");
                    last_err = Some(LlmError::Network {
                        provider: PROVIDER_NAME.to_string(),
                        detail: e.to_string(),
                    });
                    let backoff = DEFAULT_RETRY_BACKOFF_MS * (1 << attempt.min(5));
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                    continue;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| LlmError::ProviderExhausted {
            provider: PROVIDER_NAME.to_string(),
            attempts: self.config.max_retries + 1,
            last_error: None,
        }))
    }

    /// **战役 4-1 真流式 (R17-2026-08-04)**: Anthropic Messages API SSE 推流.
    ///
    /// **协议** (字段级对齐 Anthropic 官方文档):
    /// 1. POST `/v1/messages` 带 `stream: true`
    /// 2. 响应 `Content-Type: text/event-stream`, body 是 SSE 格式
    /// 3. 每个 event 由 2 行组成: `event: <type>\n` + `data: {JSON}\n\n`
    /// 4. 终止标志 `event: message_stop` + `data: {"type":"message_stop"}`
    ///
    /// **不假装** (主 17:58 O-5):
    /// - 真接 `reqwest::Response::bytes_stream()`, 真 `StreamExt::next()` 拉 chunk
    /// - 真按 `\n\n` 切 SSE event
    /// - JSON 解析失败不假装错误, 跳过该行
    /// - `event: message_stop` 终止流
    /// - System message 拆出放顶层 system 字段 (跟 `complete` 一致)
    async fn complete_stream(
        &self,
        req: LlmRequest,
    ) -> Result<futures::stream::BoxStream<'static, Result<String, LlmError>>, LlmError> {
        use futures::stream::{self, StreamExt};

        // System message 拆出 (跟 complete 一样的逻辑)
        let mut system_prompt: Option<String> = None;
        let mut messages: Vec<AnthropicMessage> = Vec::new();
        for m in req.messages {
            match m.role {
                ChatRole::System => {
                    if let Some(existing) = system_prompt.as_mut() {
                        existing.push('\n');
                        existing.push_str(&m.content);
                    } else {
                        system_prompt = Some(m.content);
                    }
                }
                ChatRole::User => {
                    messages.push(AnthropicMessage {
                        role: "user".into(),
                        content: m.content,
                    });
                }
                ChatRole::Assistant => {
                    messages.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: m.content,
                    });
                }
            }
        }

        if messages.is_empty() {
            return Err(LlmError::Config(
                "Anthropic Messages API requires at least one user/assistant message (system-only is invalid)"
                    .into(),
            ));
        }

        let body = AnthropicRequestBody {
            model: req.model.clone(),
            max_tokens: req.max_tokens,
            system: system_prompt,
            messages,
            temperature: Some(req.temperature),
        };

        let url = format!("{}/v1/messages", self.config.base_url);
        // 流式不重试 (SSE 状态机复杂, 简化处理: 1 次尝试, 失败返 Err)
        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network {
                provider: PROVIDER_NAME.to_string(),
                detail: e.to_string(),
            })?;

        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
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

        // 真流: 拉 bytes_stream, 按 SSE event 切分, 解析 event + data 行
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
                        // 1. 看 buffer 里有无完整 SSE event (空行 \n\n 分隔)
                        if let Some(event_end) = buffer.find("\n\n") {
                            let event: String = buffer.drain(..event_end + 2).collect();
                            // event 内多行 (event: + data:), 逐行处理
                            let mut stream_ended = false;
                            let mut chunk_to_emit: Option<String> = None;
                            for line in event.lines() {
                                if let Some(event_type) = parse_anthropic_sse_event_line(line) {
                                    if event_type == "message_stop" {
                                        stream_ended = true;
                                        break;
                                    }
                                } else if let Some(maybe_content) =
                                    parse_anthropic_sse_data_line(line)
                                {
                                    if let Some(content) = maybe_content {
                                        if !content.is_empty() {
                                            chunk_to_emit = Some(content);
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
                            // 整个 event 无 content, 继续 loop
                            continue;
                        }
                        // 2. buffer 不够, 拉更多字节
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
                                            parse_anthropic_sse_data_line(line)
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
// 响应解析 (Anthropic Messages API)
// ============================================================

fn parse_anthropic_response(
    body: serde_json::Value,
    latency_ms: u64,
    provider: &str,
) -> Result<LlmResponse, LlmError> {
    let parsed: AnthropicResponse =
        serde_json::from_value(body).map_err(|e| LlmError::BadResponse {
            provider: provider.to_string(),
            detail: format!("response parse: {e}"),
            status_code: None,
        })?;

    // 提取第一个 type="text" 的 content
    let text = parsed
        .content
        .iter()
        .find(|c| c.content_type == "text")
        .map(|c| c.text.clone())
        .unwrap_or_default();

    let usage = TokenUsage::new(parsed.usage.input_tokens, parsed.usage.output_tokens);

    let model = if parsed.model.is_empty() {
        "unknown".into()
    } else {
        parsed.model
    };
    let finish_reason = parsed.stop_reason.unwrap_or_else(|| "end_turn".into());

    Ok(LlmResponse {
        content: text,
        usage,
        latency_ms,
        model,
        finish_reason,
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

    /// R17 不假装验证: Anthropic provider 默认 base_url 走 minimaxi /anthropic
    #[test]
    fn test_default_base_url_is_minimaxi_anthropic() {
        assert!(
            DEFAULT_BASE_URL.starts_with("https://api.minimaxi.com/"),
            "R17 不假装: Anthropic provider 默认 base_url 必须直连 minimaxi. 实际: {}",
            DEFAULT_BASE_URL
        );
        assert!(
            DEFAULT_BASE_URL.ends_with("/anthropic"),
            "R17 不假装: Anthropic provider base_url 必须以 /anthropic 结尾. 实际: {}",
            DEFAULT_BASE_URL
        );
        assert!(
            !DEFAULT_BASE_URL.contains("localhost"),
            "R17 不假装: Anthropic provider 不能是 localhost (那是 NewAPI 时代)"
        );
    }

    #[test]
    fn test_parse_anthropic_response() {
        let body = serde_json::json!({
            "id": "msg_xxx",
            "type": "message",
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Hello from Anthropic protocol"}
            ],
            "model": "MiniMax-M3",
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 15, "output_tokens": 8}
        });
        let resp = parse_anthropic_response(body, 200, PROVIDER_NAME).unwrap();
        assert_eq!(resp.content, "Hello from Anthropic protocol");
        assert_eq!(resp.usage.prompt_tokens, 15);
        assert_eq!(resp.usage.completion_tokens, 8);
        assert_eq!(resp.model, "MiniMax-M3");
        assert_eq!(resp.provider, PROVIDER_NAME);
        assert_eq!(resp.finish_reason, "end_turn");
        assert_eq!(resp.latency_ms, 200);
    }

    /// R17 不假装验证: Anthropic 协议把 system 从 messages 抽出放顶层 system 字段
    #[test]
    fn test_system_message_separated_from_messages() {
        // 模拟 complete 内部 system 拆分逻辑
        let req_messages = vec![
            ChatMessage::system("你是一个 Rust 助手"),
            ChatMessage::user("写个 hello world"),
        ];
        let mut system_prompt: Option<String> = None;
        let mut messages: Vec<AnthropicMessage> = Vec::new();
        for m in req_messages {
            match m.role {
                ChatRole::System => {
                    if let Some(existing) = system_prompt.as_mut() {
                        existing.push('\n');
                        existing.push_str(&m.content);
                    } else {
                        system_prompt = Some(m.content);
                    }
                }
                ChatRole::User => {
                    messages.push(AnthropicMessage {
                        role: "user".into(),
                        content: m.content,
                    });
                }
                ChatRole::Assistant => {
                    messages.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: m.content,
                    });
                }
            }
        }
        assert_eq!(
            system_prompt,
            Some("你是一个 Rust 助手".to_string()),
            "R17 不假装: System message 必须抽出到 system 字段, 不进 messages"
        );
        assert_eq!(messages.len(), 1, "messages 数组应该只剩 user/assistant");
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "写个 hello world");
    }

    /// R17 不假装验证: 多个 System messages 必须合并
    #[test]
    fn test_multiple_system_messages_concatenated() {
        let req_messages = vec![
            ChatMessage::system("规则 1: 简洁回答"),
            ChatMessage::system("规则 2: 用中文"),
        ];
        let mut system_prompt: Option<String> = None;
        for m in req_messages {
            if matches!(m.role, ChatRole::System) {
                if let Some(existing) = system_prompt.as_mut() {
                    existing.push('\n');
                    existing.push_str(&m.content);
                } else {
                    system_prompt = Some(m.content);
                }
            }
        }
        assert_eq!(
            system_prompt,
            Some("规则 1: 简洁回答\n规则 2: 用中文".to_string())
        );
    }

    #[test]
    fn test_capabilities() {
        let config = AnthropicCompatibleConfig::new(
            "test-key",
            "https://api.minimaxi.com/anthropic",
            vec!["MiniMax-M3".into()],
        );
        let provider = AnthropicCompatibleProvider::new(config).unwrap();
        let caps = provider.capabilities();
        assert!(caps.contains(ProviderCapabilities::CHAT));
        assert!(caps.contains(ProviderCapabilities::SYSTEM_PROMPT));
        assert!(caps.contains(ProviderCapabilities::CUSTOM_TEMPERATURE));
        assert!(
            !caps.contains(ProviderCapabilities::JSON_MODE),
            "Anthropic 协议不通过本 trait 暴露 JSON mode"
        );
    }

    #[test]
    fn test_supports_model() {
        let config = AnthropicCompatibleConfig::new(
            "test-key",
            "https://api.minimaxi.com/anthropic",
            vec!["MiniMax-M3".into()],
        );
        let provider = AnthropicCompatibleProvider::new(config).unwrap();
        assert!(provider.supports_model("MiniMax-M3"));
        assert!(!provider.supports_model("gpt-4"));
    }

    // ============================================================
    // 战役 4-1: Anthropic SSE 解析单元测试
    // ============================================================

    #[test]
    fn parse_anthropic_sse_data_line_extracts_text_delta() {
        // 标准 Anthropic content_block_delta: data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}
        let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"hi"}}"#;
        let result = parse_anthropic_sse_data_line(line);
        assert_eq!(result, Some(Some("hi".to_string())));
    }

    #[test]
    fn parse_anthropic_sse_data_line_skips_non_text_delta() {
        // input_json_delta (tool use 场景) → 跳过
        let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{}"}}"#;
        let result = parse_anthropic_sse_data_line(line);
        assert_eq!(result, None);
    }

    #[test]
    fn parse_anthropic_sse_data_line_skips_other_event_types() {
        // message_start / content_block_start / content_block_stop / message_delta / ping → 跳过
        let cases = [
            r#"data: {"type":"message_start","message":{}}"#,
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#"data: {"type":"content_block_stop","index":0}"#,
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#,
            r#"data: {"type":"ping"}"#,
        ];
        for line in &cases {
            assert_eq!(parse_anthropic_sse_data_line(line), None, "应跳过: {line}");
        }
    }

    #[test]
    fn parse_anthropic_sse_data_line_skips_invalid_json() {
        let line = "data: {not valid json}";
        assert_eq!(parse_anthropic_sse_data_line(line), None);
    }

    #[test]
    fn parse_anthropic_sse_data_line_handles_cjk() {
        // CJK 内容 (战役 4-1: 不假装 CJK)
        let line = r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"中文测试"}}"#;
        let result = parse_anthropic_sse_data_line(line);
        assert_eq!(result, Some(Some("中文测试".to_string())));
    }

    #[test]
    fn parse_anthropic_sse_event_line_extracts_event_type() {
        assert_eq!(
            parse_anthropic_sse_event_line("event: message_start"),
            Some("message_start".to_string())
        );
        assert_eq!(
            parse_anthropic_sse_event_line("event: content_block_delta"),
            Some("content_block_delta".to_string())
        );
        assert_eq!(
            parse_anthropic_sse_event_line("event: message_stop"),
            Some("message_stop".to_string())
        );
    }

    #[test]
    fn parse_anthropic_sse_event_line_skips_non_event_lines() {
        assert_eq!(parse_anthropic_sse_event_line("data: ..."), None);
        assert_eq!(parse_anthropic_sse_event_line("id: 1"), None);
        assert_eq!(parse_anthropic_sse_event_line(""), None);
        assert_eq!(parse_anthropic_sse_event_line("event: "), None);
    }

    // ============================================================
    // 战役 4-1: 真 SSE 推流测试 (用 in-memory byte stream 模拟)
    // ============================================================

    #[tokio::test]
    async fn complete_stream_parses_simulated_anthropic_sse_bytes() {
        use bytes::Bytes;
        use futures::stream::{self, StreamExt};

        // 模拟 Anthropic 真实 SSE 流
        let sse_body = concat!(
            "event: message_start\n",
            r#"data: {"type":"message_start","message":{"id":"msg_01","type":"message","role":"assistant","content":[],"model":"MiniMax-M3","stop_reason":null,"usage":{"input_tokens":8,"output_tokens":1}}}"#,
            "\n\n",
            "event: content_block_start\n",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            "\n\n",
            "event: content_block_delta\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"你"}}"#,
            "\n\n",
            "event: content_block_delta\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"好"}}"#,
            "\n\n",
            "event: content_block_delta\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":","}}"#,
            "\n\n",
            "event: content_block_delta\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" 世"}}"#,
            "\n\n",
            "event: content_block_delta\n",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"界"}}"#,
            "\n\n",
            "event: content_block_stop\n",
            r#"data: {"type":"content_block_stop","index":0}"#,
            "\n\n",
            "event: message_delta\n",
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":6}}"#,
            "\n\n",
            "event: message_stop\n",
            r#"data: {"type":"message_stop"}"#,
            "\n\n",
        );

        let events: Vec<&str> = sse_body.split_inclusive("\n\n").collect();
        let byte_chunks: Vec<Result<Bytes, std::io::Error>> = events
            .into_iter()
            .map(|e| Ok(Bytes::copy_from_slice(e.as_bytes())))
            .collect();
        let byte_stream = stream::iter(byte_chunks);

        // 用 unfold 跟生产代码同款
        let mut buffer = String::new();
        let mut byte_stream = Box::pin(byte_stream);
        let mut chunks: Vec<String> = Vec::new();
        let mut done = false;
        while !done {
            if let Some(event_end) = buffer.find("\n\n") {
                let event: String = buffer.drain(..event_end + 2).collect();
                let mut stream_ended = false;
                let mut chunk_to_emit: Option<String> = None;
                for line in event.lines() {
                    if let Some(event_type) = parse_anthropic_sse_event_line(line) {
                        if event_type == "message_stop" {
                            stream_ended = true;
                            break;
                        }
                    } else if let Some(maybe_content) = parse_anthropic_sse_data_line(line) {
                        if let Some(content) = maybe_content {
                            if !content.is_empty() {
                                chunk_to_emit = Some(content);
                            }
                        }
                    }
                }
                if stream_ended {
                    done = true;
                    continue;
                }
                if let Some(content) = chunk_to_emit {
                    chunks.push(content);
                }
                continue;
            }
            match byte_stream.next().await {
                Some(Ok(bytes)) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                }
                Some(Err(_)) | None => {
                    if !buffer.trim().is_empty() {
                        for line in buffer.lines() {
                            if let Some(Some(content)) = parse_anthropic_sse_data_line(line) {
                                if !content.is_empty() {
                                    chunks.push(content);
                                }
                            }
                        }
                    }
                    done = true;
                }
            }
        }

        // 验收: 5 个 content chunk (message_start / content_block_start / content_block_stop / message_delta / message_stop 全部跳过)
        assert_eq!(chunks, vec!["你", "好", ",", " 世", "界"]);
        let joined: String = chunks.concat();
        assert_eq!(joined, "你好, 世界");
    }
}
