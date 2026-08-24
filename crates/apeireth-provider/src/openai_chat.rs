//! Shared OpenAI Chat Completions protocol primitives.
//!
//! Two canonical providers — `provider.minimax` and the generic
//! `provider.openai-compatible` — both speak the OpenAI Chat Completions wire
//! protocol (request envelope, response shape, Bearer auth, status mapping).
//! Duplicating that translation across providers is the kind of repetition that
//! drifts; this module is the small internal helper that both call.
//!
//! This is a **provider-internal protocol helper**, not a runtime abstraction:
//! it owns no client, no credentials, no routing, no lifecycle, and no vendor
//! identity. Each provider still owns its own `ProviderCapability`, plugin
//! manifest, credential key, model mapping, and configuration (§13-16). The
//! helper is parameterized by the provider's own id string so errors carry the
//! right attribution.

use apeireth_plugin::ProviderError;
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};

/// Build the OpenAI Chat Completions request body from a canonical request.
///
/// `wire_model` is the vendor wire spelling the provider resolved from the
/// canonical requested model — callers map canonical→wire before this. Text
/// content of each message is joined; tools, tool calls/results, and image
/// parts are not transported and surface as a permanent `BadResponse` rather
/// than being silently dropped (§10/§26). `stream:false` is set explicitly.
pub fn build_request_body(
    request: &NormalizedRequest,
    wire_model: &str,
    provider: &str,
) -> Result<serde_json::Value, ProviderError> {
    if !request.tools.is_empty() {
        return Err(ProviderError::BadResponse {
            provider: provider.to_string(),
            detail: "provider does not transport tool declarations".into(),
        });
    }

    let mut messages = Vec::with_capacity(request.messages.len());
    for message in &request.messages {
        if !message.tool_calls.is_empty() || message.role == MessageRole::Tool {
            return Err(ProviderError::BadResponse {
                provider: provider.to_string(),
                detail: "provider does not transport tool calls/results".into(),
            });
        }
        if message
            .content
            .iter()
            .any(|part| matches!(part, ContentPart::ImageUrl { .. }))
        {
            return Err(ProviderError::BadResponse {
                provider: provider.to_string(),
                detail: "provider only supports text content".into(),
            });
        }

        let role = match message.role {
            MessageRole::System => "system",
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => unreachable!("tool messages rejected above"),
        };
        messages.push(serde_json::json!({
            "role": role,
            "content": ContentPart::join_text(&message.content),
        }));
    }

    let mut body = serde_json::json!({
        "model": wire_model,
        "messages": messages,
        "stream": false,
    });
    if let Some(temperature) = request.temperature {
        body["temperature"] = serde_json::json!(temperature.clamp(0.0, 2.0));
    }
    if let Some(max_tokens) = request.max_tokens {
        body["max_tokens"] = serde_json::json!(max_tokens.min(32_768));
    }
    if !request.stop.is_empty() {
        body["stop"] = serde_json::json!(request.stop);
    }
    Ok(body)
}

/// Parse an OpenAI Chat Completions response into a canonical response.
///
/// Reads `choices[0].message.content`, `finish_reason` (via
/// [`NormalizedFinishReason::from_openai`]), and `usage`
/// (`prompt_tokens`/`completion_tokens`/`total_tokens`). Usage is omitted
/// (defaulted) when absent — never fabricated. The `id` falls back to a
/// provider-tagged synthetic value.
pub fn parse_response(
    body: serde_json::Value,
    request_model: &str,
    provider: &str,
) -> Result<NormalizedResponse, ProviderError> {
    let provider_owned = provider.to_string();

    let choices = body
        .get("choices")
        .and_then(|c| c.as_array())
        .ok_or_else(|| ProviderError::BadResponse {
            provider: provider_owned.clone(),
            detail: "response has no choices array".into(),
        })?;
    let choice = choices.first().ok_or_else(|| ProviderError::BadResponse {
        provider: provider_owned.clone(),
        detail: "response choices array is empty".into(),
    })?;

    let content = choice
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string();
    let finish_reason = choice
        .get("finish_reason")
        .and_then(|f| f.as_str())
        .unwrap_or("stop");

    let usage = body
        .get("usage")
        .map(|u| NormalizedUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u
                .get("completion_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        })
        .unwrap_or_default();

    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(request_model)
        .to_string();

    Ok(NormalizedResponse {
        id: body
            .get("id")
            .and_then(|i| i.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("openai-{provider}")),
        model,
        content,
        finish_reason: Some(NormalizedFinishReason::from_openai(finish_reason)),
        usage,
        tool_calls: Vec::new(),
        raw_metadata: serde_json::Map::new(),
    })
}

/// Classify a vendor HTTP outcome into a canonical [`ProviderError`].
///
/// Transport failures (429/408/504/timeout) are transient; auth (401/403),
/// policy (5xx), and bad-request are permanent. `retry_after_body` is the
/// response body text; the `Retry-After` hint is parsed from it when present
/// (the 429 path receives the body for unit-testability).
pub fn classify_status(
    status: reqwest::StatusCode,
    body_text: String,
    provider: &str,
    timeout_ms: u64,
) -> ProviderError {
    let provider = provider.to_string();
    match status.as_u16() {
        401 | 403 => ProviderError::AuthFailed {
            provider,
            detail: format!("vendor returned {status}: {body_text}"),
        },
        429 => {
            let retry_after_ms = parse_retry_after_ms(&body_text).unwrap_or(1_000);
            ProviderError::RateLimited {
                provider,
                retry_after_ms,
            }
        }
        408 | 504 => ProviderError::Timeout {
            provider,
            timeout_ms,
        },
        _ if status.is_server_error() => ProviderError::Refused {
            provider,
            detail: format!("vendor returned {status}: {body_text}"),
        },
        _ => ProviderError::BadResponse {
            provider,
            detail: format!("vendor returned {status}: {body_text}"),
        },
    }
}

/// Parse a `Retry-After` hint (seconds) into milliseconds.
///
/// `retry-after` arrives as a header in real responses; for unit-testability of
/// the classification (which receives the body text) this helper accepts a plain
/// seconds integer. Returns `None` when no hint is present.
pub fn parse_retry_after_ms(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    trimmed.parse::<u64>().ok().map(|secs| secs * 1_000)
}

/// Normalize a base URL + path join so `/v1`, `/v1/`, and bare hosts all
/// produce a clean endpoint, never `/v1//chat/completions` or a doubled
/// `/v1/v1/...` (§22).
pub fn join_endpoint(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::NormalizedMessage;

    fn request() -> NormalizedRequest {
        NormalizedRequest::new(
            "m",
            vec![
                NormalizedMessage::system("be brief"),
                NormalizedMessage::user("hi"),
            ],
        )
    }

    #[test]
    fn build_request_body_maps_roles_and_content() {
        let body = build_request_body(&request(), "wire-model", "provider.test").unwrap();
        assert_eq!(body["model"], "wire-model");
        assert_eq!(body["stream"], false);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "be brief");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[1]["content"], "hi");
    }

    #[test]
    fn build_request_body_rejects_tools_and_images() {
        let mut req = request();
        req.tools
            .push(apeireth_protocol::canonical::NormalizedTool::new("t"));
        let err = build_request_body(&req, "m", "provider.test").unwrap_err();
        assert!(matches!(err, ProviderError::BadResponse { .. }));
    }

    #[test]
    fn parse_response_maps_content_usage_and_finish_reason() {
        let body = serde_json::json!({
            "id": "chatcmpl-x",
            "model": "wire-model",
            "choices": [{"message": {"content": "hello back"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        });
        let resp = parse_response(body, "m", "provider.test").unwrap();
        assert_eq!(resp.content, "hello back");
        assert_eq!(resp.id, "chatcmpl-x");
        assert_eq!(resp.model, "wire-model");
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(resp.usage.total_tokens, 15);
    }

    #[test]
    fn parse_response_omits_usage_when_absent() {
        let body = serde_json::json!({
            "choices": [{"message": {"content": "ok"}, "finish_reason": "length"}]
        });
        let resp = parse_response(body, "m", "provider.test").unwrap();
        assert_eq!(resp.usage, NormalizedUsage::default());
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Length));
    }

    #[test]
    fn classify_status_maps_each_category() {
        let auth = classify_status(reqwest::StatusCode::UNAUTHORIZED, "bad".into(), "p", 1000);
        assert!(matches!(auth, ProviderError::AuthFailed { .. }) && !auth.is_retryable());

        let rate = classify_status(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            "2".into(),
            "p",
            1000,
        );
        assert!(matches!(rate, ProviderError::RateLimited { .. }) && rate.is_retryable());

        let timeout = classify_status(reqwest::StatusCode::GATEWAY_TIMEOUT, "".into(), "p", 1000);
        assert!(matches!(timeout, ProviderError::Timeout { .. }) && timeout.is_retryable());

        let server = classify_status(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            "boom".into(),
            "p",
            1000,
        );
        assert!(matches!(server, ProviderError::Refused { .. }) && !server.is_retryable());

        let bad = classify_status(reqwest::StatusCode::BAD_REQUEST, "nope".into(), "p", 1000);
        assert!(matches!(bad, ProviderError::BadResponse { .. }) && !bad.is_retryable());
    }

    #[test]
    fn parse_retry_after_accepts_seconds() {
        assert_eq!(parse_retry_after_ms("2"), Some(2_000));
        assert_eq!(parse_retry_after_ms("not-a-number"), None);
    }

    #[test]
    fn join_endpoint_normalizes_trailing_and_leading_slashes() {
        assert_eq!(
            join_endpoint("https://h/v1", "/chat/completions"),
            "https://h/v1/chat/completions"
        );
        assert_eq!(
            join_endpoint("https://h/v1/", "/chat/completions"),
            "https://h/v1/chat/completions"
        );
        assert_eq!(
            join_endpoint("https://h", "chat/completions"),
            "https://h/chat/completions"
        );
        assert_eq!(
            join_endpoint("https://h/v1", "chat/completions"),
            "https://h/v1/chat/completions"
        );
    }
}
