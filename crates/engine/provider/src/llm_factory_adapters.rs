//! Shared adapter logic for the two `LlmFactory` implementations
//! (`minimax_llm_factory` / `openai_compatible_llm_factory`).
//!
//! O-6 三阶审查: 两个 factory 的 CompletionRequest ↔ NormalizedRequest 转换与
//! ProviderError → LlmError 映射是完全相同的协议层逻辑。复制粘贴两份 = 未来
//! 修一处漏一处；集中在此模块，两个 factory 共用（0 复制，0 行为变化）。

use apeireth_plugin::llm_factory::{
    CompletionMessage, CompletionRequest, CompletionResponse, LlmError, TokenUsage,
};
use apeireth_plugin::ProviderError;
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse,
};

/// `CompletionRequest` (factory 边界) → `NormalizedRequest` (provider 边界)。
///
/// **字段映射** (与 minimax factory 原实现一致):
/// - `system_prompt` → 头部 `NormalizedMessage::system(...)`
/// - `messages` (factory 风格 role/content 字符串) → `NormalizedMessage` (枚举 role + 多模 content)
/// - `temperature` (f64) → `temperature` (Option<f32>)
/// - `max_tokens` → `max_tokens`
/// - `tools` → **0 装**: 两个 factory 当前的 capability 都不支持 tool calls,
///   传 tools 由 capability 层拒绝 (`adapt_request` / feature truthfulness),
///   这里保持空 tools 透传。
pub(crate) fn to_normalized(model: &str, req: &CompletionRequest) -> NormalizedRequest {
    let mut messages = Vec::with_capacity(req.messages.len() + 1);
    if !req.system_prompt.is_empty() {
        messages.push(NormalizedMessage::system(&req.system_prompt));
    }
    for m in &req.messages {
        messages.push(NormalizedMessage {
            role: MessageRole::from_legacy_value(&m.role),
            content: vec![ContentPart::Text {
                text: m.content.clone(),
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        });
    }
    NormalizedRequest {
        model: model.to_string(),
        messages,
        temperature: Some(req.temperature as f32),
        max_tokens: req.max_tokens,
        stream: false,
        stop: Vec::new(),
        tools: Vec::new(),
        tool_choice: None,
        metadata: Default::default(),
    }
}

/// `NormalizedResponse` (provider 边界) → `CompletionResponse` (factory 边界)。
pub(crate) fn from_normalized(resp: NormalizedResponse) -> CompletionResponse {
    let finish_reason = match resp.finish_reason {
        Some(NormalizedFinishReason::Stop) => "stop",
        Some(NormalizedFinishReason::Length) => "length",
        Some(NormalizedFinishReason::ToolCalls) => "tool_calls",
        Some(NormalizedFinishReason::ContentFilter) => "content_filter",
        Some(NormalizedFinishReason::StopSequence) => "stop_sequence",
        Some(NormalizedFinishReason::Other) | None => "other",
    }
    .to_string();

    let tool_calls: Vec<serde_json::Value> = resp
        .tool_calls
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "type": "function",
                "function": {
                    "name": c.name,
                    "arguments": c.arguments,
                }
            })
        })
        .collect();

    CompletionResponse {
        message: CompletionMessage {
            role: "assistant".into(),
            content: resp.content,
        },
        tool_calls,
        finish_reason,
        usage: TokenUsage {
            prompt_tokens: resp.usage.prompt_tokens,
            completion_tokens: resp.usage.completion_tokens,
            total_tokens: resp.usage.total_tokens,
        },
    }
}

/// `ProviderError` → `LlmError` 一对一映射（保持 provider 已分类的 transient vs
/// permanent 语义; 上层按 `is_retryable` 派生 fallback / 重试）。
pub(crate) fn map_provider_error(err: ProviderError) -> LlmError {
    match err {
        ProviderError::AuthFailed { detail, .. } => LlmError::Credentials(detail),
        ProviderError::RateLimited { retry_after_ms, .. } => {
            LlmError::RateLimited { retry_after_ms }
        }
        ProviderError::Timeout { timeout_ms, .. } => {
            LlmError::Stream(format!("timeout after {timeout_ms}ms"))
        }
        ProviderError::Network { detail, .. } => LlmError::Network(detail),
        ProviderError::BadResponse { detail, .. } => LlmError::Provider(detail),
        ProviderError::Refused { detail, .. } => LlmError::Provider(detail),
        // 0 装诚实: `ProviderError` 标 `#[non_exhaustive]`, 未来加 variant 时
        // 兜底成 LlmError::Provider, 0 panic。
        other => LlmError::Provider(format!("unclassified provider error: {other}")),
    }
}
