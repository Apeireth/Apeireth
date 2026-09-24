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
    NormalizedToolChoice, NormalizedUsage, ToolCall,
};

/// Build the OpenAI Chat Completions request body from a canonical request.
///
/// `wire_model` is the vendor wire spelling the provider resolved from the
/// canonical requested model — callers map canonical→wire before this. Text
/// content of each message is joined. 2026-09-08 (tool transport): tool
/// declarations, assistant `tool_calls`, and tool-result messages are
/// transported in the native OpenAI wire shape; image parts remain rejected
/// (providers do not claim Vision). `stream:false` is set explicitly.
pub fn build_request_body(
    request: &NormalizedRequest,
    wire_model: &str,
    provider: &str,
) -> Result<serde_json::Value, ProviderError> {
    let mut messages = Vec::with_capacity(request.messages.len());
    for message in &request.messages {
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

        match message.role {
            // 2026-09-08 tool transport: tool 结果消息 (原生 OpenAI wire 形状).
            MessageRole::Tool => {
                let tool_call_id =
                    message
                        .tool_call_id
                        .as_ref()
                        .ok_or_else(|| ProviderError::BadResponse {
                            provider: provider.to_string(),
                            detail: "tool message requires a tool_call_id".into(),
                        })?;
                messages.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": ContentPart::join_text(&message.content),
                }));
            }
            // 2026-09-08 tool transport: assistant tool_calls (原生 wire 形状).
            MessageRole::Assistant if !message.tool_calls.is_empty() => {
                let tool_calls: Vec<serde_json::Value> = message
                    .tool_calls
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "type": "function",
                            "function": {
                                "name": c.name,
                                "arguments": c.arguments.to_string(),
                            }
                        })
                    })
                    .collect();
                let text = ContentPart::join_text(&message.content);
                messages.push(serde_json::json!({
                    "role": "assistant",
                    "content": if text.is_empty() { serde_json::Value::Null } else { serde_json::json!(text) },
                    "tool_calls": tool_calls,
                }));
            }
            _ => {
                let role = match message.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => unreachable!("tool messages handled above"),
                };
                messages.push(serde_json::json!({
                    "role": role,
                    "content": ContentPart::join_text(&message.content),
                }));
            }
        }
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
    // 2026-09-08 tool transport: 工具声明 (原生 function 形状).
    if !request.tools.is_empty() {
        let tools: Vec<serde_json::Value> = request
            .tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description.clone().unwrap_or_default(),
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        body["tools"] = serde_json::json!(tools);
    }
    if let Some(choice) = &request.tool_choice {
        body["tool_choice"] = match choice {
            NormalizedToolChoice::Auto => serde_json::json!("auto"),
            NormalizedToolChoice::None => serde_json::json!("none"),
            NormalizedToolChoice::Required => serde_json::json!("required"),
            NormalizedToolChoice::Specific { name } => serde_json::json!({
                "type": "function",
                "function": { "name": name }
            }),
        };
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

    // 0 装 fail-loud (2026-10-10 真缺陷 #4 加固): 思考型模型把 max_tokens 全喂给
    // reasoning_content 时 content 落空 (教训见 canonical_openai_compatible.rs
    // adapt_request 注释: 500 必截断 / 2048 仍空 1/3 / 4096 稳)。空 content +
    // length 截断 = 预算耗尽的空壳响应 —— 显式报错并给行动指引, 不发假响应。
    // 非截断的空 content (finish=stop) 不拦: 那是模型的真实输出。
    if content.is_empty() && finish_reason == "length" {
        return Err(ProviderError::BadResponse {
            provider: provider_owned.clone(),
            detail: "output budget exhausted: content empty while finish_reason=length \
                     (reasoning models burn max_tokens on reasoning_content). Raise \
                     max_tokens (4096 known-good) or omit it (canonical default fills 4096)."
                .into(),
        });
    }

    // M11: usage 读数按 u32 封顶。`as u32` 对 > u32::MAX 的 vendor 值在
    // debug 构建 panic、release 静默截断 —— 两者都不是可接受的记账语义。
    // 缺失/非数字字段仍是 0 (usage 从不臆造)。
    let usage = body
        .get("usage")
        .map(|u| NormalizedUsage {
            prompt_tokens: token_count(u, "prompt_tokens"),
            completion_tokens: token_count(u, "completion_tokens"),
            // total 直接读 vendor 自报值 (封顶), 保持与旧实现相同的语义,
            // 不回绕也不臆造。
            total_tokens: token_count(u, "total_tokens"),
        })
        .unwrap_or_default();

    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(request_model)
        .to_string();

    // 2026-09-08 tool transport: 解析 choices[0].message.tool_calls
    // (原生 OpenAI 形状: id + function.name + function.arguments[字符串]).
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    if let Some(tcs) = choice
        .get("message")
        .and_then(|m| m.get("tool_calls"))
        .and_then(|t| t.as_array())
    {
        for tc in tcs {
            let id = tc
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let function = tc.get("function").cloned().unwrap_or_default();
            let name = function
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            // M20 (provider 侧): 模型输出畸形 arguments JSON 时, 旧实现
            // `.ok()` 静默吞为 `Value::Null` —— 排障困难, 且工具侧会拿到
            // 一个看起来像"无参数"的调用。这里 fail-loud: 带说明的
            // BadResponse, 让调用方看到是哪一次 tool call 的参数坏掉。
            let arguments = function
                .get("arguments")
                .and_then(|v| v.as_str())
                .map(|raw| {
                    serde_json::from_str(raw).map_err(|error| {
                        ProviderError::BadResponse {
                            provider: provider_owned.clone(),
                            detail: format!(
                                "tool call {id:?} ({name}) has malformed arguments JSON: {error}"
                            ),
                        }
                    })
                })
                .transpose()?
                .unwrap_or(serde_json::Value::Null);
            tool_calls.push(ToolCall {
                id,
                name,
                arguments,
            });
        }
    }

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
        tool_calls,
        raw_metadata: serde_json::Map::new(),
    })
}

/// Read one usage counter and clamp it into `u32` (M11).
///
/// `as u32` on a vendor-reported count panics in debug builds and silently
/// truncates in release builds when the value exceeds `u32::MAX`; both are
/// unacceptable accounting semantics. Clamping to `u32::MAX` keeps an
/// obviously-overflowing value instead of pretending it is precise. A missing
/// or non-numeric field stays `0` (usage is never fabricated).
fn token_count(usage: &serde_json::Value, key: &str) -> u32 {
    usage
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|value| u32::try_from(value).unwrap_or(u32::MAX))
        .unwrap_or(0)
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
    fn build_request_body_transports_tools_and_rejects_images() {
        // 2026-09-08 tool transport: 工具声明进入原生 function 形状.
        let mut req = request();
        req.tools.push(
            apeireth_protocol::canonical::NormalizedTool::new("t")
                .with_description("a tool")
                .with_parameters(
                    [("x".to_string(), serde_json::json!({"type": "string"}))]
                        .into_iter()
                        .collect(),
                ),
        );
        req.tool_choice = Some(NormalizedToolChoice::Required);
        let body = build_request_body(&req, "m", "provider.test").unwrap();
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "t");
        assert_eq!(body["tool_choice"], "required");

        // 图像仍拒绝 (未声明 Vision).
        let mut img = request();
        img.messages.push(NormalizedMessage {
            role: MessageRole::User,
            content: vec![ContentPart::ImageUrl {
                url: "https://example.invalid/i.png".into(),
                detail: None,
            }],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        });
        let err = build_request_body(&img, "m", "provider.test").unwrap_err();
        assert!(matches!(err, ProviderError::BadResponse { .. }));
    }

    #[test]
    fn build_request_body_transports_tool_calls_and_results() {
        // assistant tool_calls → 原生 wire 形状; tool 结果消息 → role=tool.
        let mut req = request();
        req.messages.push(NormalizedMessage {
            role: MessageRole::Assistant,
            content: Vec::new(),
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                name: "tool.shell".into(),
                arguments: serde_json::json!({"command": "echo hi"}),
            }],
            tool_call_id: None,
            name: None,
        });
        req.messages.push(NormalizedMessage::tool_result(
            "call_1",
            Some("tool.shell".into()),
            "hello-from-tool",
        ));
        let body = build_request_body(&req, "m", "provider.test").unwrap();
        let messages = body["messages"].as_array().unwrap();
        let assistant = &messages[2];
        assert_eq!(assistant["role"], "assistant");
        assert_eq!(assistant["content"], serde_json::Value::Null);
        assert_eq!(assistant["tool_calls"][0]["id"], "call_1");
        assert_eq!(assistant["tool_calls"][0]["function"]["name"], "tool.shell");
        assert_eq!(
            assistant["tool_calls"][0]["function"]["arguments"],
            "{\"command\":\"echo hi\"}"
        );
        let tool_msg = &messages[3];
        assert_eq!(tool_msg["role"], "tool");
        assert_eq!(tool_msg["tool_call_id"], "call_1");
        assert_eq!(tool_msg["content"], "hello-from-tool");
    }

    #[test]
    fn parse_response_extracts_tool_calls() {
        let body = serde_json::json!({
            "id": "chatcmpl-y",
            "model": "wire-model",
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_9",
                        "type": "function",
                        "function": {"name": "tool.shell", "arguments": "{\"command\":\"echo hi\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
        });
        let resp = parse_response(body, "m", "provider.test").unwrap();
        assert_eq!(resp.content, "");
        assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::ToolCalls));
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].id, "call_9");
        assert_eq!(resp.tool_calls[0].name, "tool.shell");
        assert_eq!(
            resp.tool_calls[0].arguments,
            serde_json::json!({"command": "echo hi"})
        );
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

    /// M11 回归: > u32::MAX 的 usage 计数不得 `as u32` (debug panic /
    /// release 静默截断), 封顶为 u32::MAX。
    #[test]
    fn parse_response_clamps_overflowing_usage_counters() {
        let body = serde_json::json!({
            "choices": [{"message": {"content": "x"}, "finish_reason": "stop"}],
            "usage": {
                "prompt_tokens": u64::from(u32::MAX) + 3,
                "completion_tokens": u64::from(u32::MAX) + 5,
                "total_tokens": u64::from(u32::MAX) + 11,
            }
        });
        let resp = parse_response(body, "m", "provider.test").unwrap();
        assert_eq!(resp.usage.prompt_tokens, u32::MAX);
        assert_eq!(resp.usage.completion_tokens, u32::MAX);
        assert_eq!(resp.usage.total_tokens, u32::MAX);
    }

    /// M20 (provider 侧) 回归: 畸形 arguments JSON 必须 fail-loud, 不再静默
    /// 吞为 `Value::Null` (那会让工具侧看到一个"无参数"的假调用)。
    #[test]
    fn parse_response_rejects_malformed_tool_arguments() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_bad",
                        "type": "function",
                        "function": {"name": "tool.shell", "arguments": "{\"command\": "}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let err = parse_response(body, "m", "provider.test").expect_err("must fail");
        assert!(matches!(err, ProviderError::BadResponse { .. }), "{err:?}");
        let ProviderError::BadResponse { detail, .. } = err else {
            unreachable!("asserted above");
        };
        assert!(detail.contains("call_bad"), "{detail}");
        assert!(detail.contains("malformed arguments"), "{detail}");
    }

    /// 合法 arguments 与非字符串 (缺失) 形态保持原语义: 前者解析, 后者 Null。
    #[test]
    fn parse_response_absent_tool_arguments_stay_null() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "tool_calls": [{"id": "call_ok", "function": {"name": "t"}}]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let resp = parse_response(body, "m", "provider.test").unwrap();
        assert_eq!(resp.tool_calls[0].arguments, serde_json::Value::Null);
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
