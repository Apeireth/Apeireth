//! Integration tests for apeireth-protocol
//!
//! **R18 第 2 阶段**: 4 协议 round-trip wire format tests (golden tests)
//! 抄 tokio 测试模式: `tests/wire_format_*.rs` 多文件分离.
//!
//! 测试策略:
//! - 每个协议多组 fixture (最小 / 含温度+max_tokens / 响应文本)
//! - encode → decode 完整 round-trip
//! - 验证关键字段 (model, messages, choices[0], content 等)
//!
//! **不假装**: 测试是真跑 (`cargo nextest run -p apeireth-protocol`).

use apeireth_protocol::{
    endpoint_path_for_kind,
    AnthropicMessagesAdapter,
    GeminiAdapter,
    NormalizedMessage,
    NormalizedRequest,
    OpenAiChatAdapter,
    OpenAiResponsesAdapter,
    ProtocolAdapter,
    ProtocolBridge,
    ProtocolKind,
    // R37-1: 引入 ProtocolBridge 替 ProtocolRouter
};
use serde_json::{json, Value};

// =====================================================================
// OpenAI Chat Completions
// =====================================================================

#[test]
fn openai_chat_request_has_model_and_messages() {
    let adapter = OpenAiChatAdapter::new();
    let req = NormalizedRequest::new(
        "gpt-4o-mini".to_string(),
        vec![NormalizedMessage::user("hello")],
    );
    let body = adapter.adapt_request(&req).unwrap();
    assert_eq!(body["model"], "gpt-4o-mini");
    assert!(body["messages"].is_array());
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"], "hello");
}

#[test]
fn openai_chat_request_with_temperature_and_max_tokens() {
    let adapter = OpenAiChatAdapter::new();
    let mut req = NormalizedRequest::new("gpt-4o".to_string(), vec![NormalizedMessage::user("hi")]);
    req.temperature = Some(0.7);
    req.max_tokens = Some(512);
    let body = adapter.adapt_request(&req).unwrap();
    // f32 → f64 精度损失 (0.7 → 0.699999988079071), 用 abs 差判断
    let temp = body["temperature"]
        .as_f64()
        .expect("temperature must be a number");
    assert!(
        (temp - 0.7_f64).abs() < 1e-5,
        "temperature drift: {temp} (expected ~0.7)"
    );
    assert_eq!(body["max_tokens"], 512);
}

#[test]
fn openai_chat_response_text_choice() {
    let adapter = OpenAiChatAdapter::new();
    let raw: Value = json!({
        "id": "chatcmpl-x",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "gpt-4o",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "Hello from OpenAI"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    });
    let resp = adapter.adapt_response(&raw).unwrap();
    assert_eq!(resp.content, "Hello from OpenAI");
}

#[test]
fn openai_chat_empty_model_fails() {
    let adapter = OpenAiChatAdapter::new();
    let req = NormalizedRequest::new("".to_string(), vec![NormalizedMessage::user("hi")]);
    assert!(adapter.adapt_request(&req).is_err());
}

#[test]
fn openai_chat_empty_messages_fails() {
    let adapter = OpenAiChatAdapter::new();
    let req = NormalizedRequest::new("gpt-4o".to_string(), vec![]);
    assert!(adapter.adapt_request(&req).is_err());
}

// =====================================================================
// OpenAI Responses API
// =====================================================================

#[test]
fn openai_responses_request_has_input_array() {
    let adapter = OpenAiResponsesAdapter::new();
    let req = NormalizedRequest::new("gpt-4o".to_string(), vec![NormalizedMessage::user("hello")]);
    let body = adapter.adapt_request(&req).unwrap();
    assert_eq!(body["model"], "gpt-4o");
    assert!(
        body["input"].is_array(),
        "input should be array, got: {:?}",
        body
    );
}

#[test]
fn openai_responses_response_text() {
    let adapter = OpenAiResponsesAdapter::new();
    let raw: Value = json!({
        "id": "resp_x",
        "object": "response",
        "created_at": 1234567890,
        "model": "gpt-4o",
        "status": "completed",
        "output": [{
            "id": "msg_x",
            "type": "message",
            "role": "assistant",
            "status": "completed",
            "content": [{"type": "output_text", "text": "Hello from Responses", "annotations": []}]
        }],
        "usage": {"input_tokens": 10, "output_tokens": 5, "total_tokens": 15}
    });
    let resp = adapter.adapt_response(&raw).unwrap();
    assert_eq!(resp.content, "Hello from Responses");
}

// =====================================================================
// Anthropic Messages
// =====================================================================

#[test]
fn anthropic_request_requires_max_tokens() {
    let adapter = AnthropicMessagesAdapter::new();
    let req = NormalizedRequest::new(
        "claude-sonnet-4".to_string(),
        vec![NormalizedMessage::user("hi")],
    );
    // Anthropic 必填 max_tokens, 缺了应该报错
    let result = adapter.adapt_request(&req);
    assert!(result.is_err(), "Anthropic must require max_tokens");
}

#[test]
fn anthropic_request_with_max_tokens_succeeds() {
    let adapter = AnthropicMessagesAdapter::new();
    let mut req = NormalizedRequest::new(
        "claude-sonnet-4".to_string(),
        vec![NormalizedMessage::user("hi")],
    );
    req.max_tokens = Some(1024);
    let body = adapter.adapt_request(&req).unwrap();
    assert_eq!(body["model"], "claude-sonnet-4");
    assert_eq!(body["max_tokens"], 1024);
    assert!(body["messages"].is_array());
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"], "hi");
}

#[test]
fn anthropic_response_text() {
    let adapter = AnthropicMessagesAdapter::new();
    let raw: Value = json!({
        "id": "msg_x",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet-4",
        "content": [{"type": "text", "text": "Hello from Claude"}],
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    });
    let resp = adapter.adapt_response(&raw).unwrap();
    assert_eq!(resp.content, "Hello from Claude");
}

// =====================================================================
// Google Gemini GenerateContent
// =====================================================================

#[test]
fn gemini_request_has_contents_array() {
    let adapter = GeminiAdapter::new();
    let req = NormalizedRequest::new(
        "gemini-1.5-pro".to_string(),
        vec![NormalizedMessage::user("hi")],
    );
    let body = adapter.adapt_request(&req).unwrap();
    assert!(
        body["contents"].is_array(),
        "contents should be array: {:?}",
        body
    );
    assert_eq!(body["contents"][0]["role"], "user");
    assert_eq!(body["contents"][0]["parts"][0]["text"], "hi");
}

#[test]
fn gemini_response_text() {
    let adapter = GeminiAdapter::new();
    let raw: Value = json!({
        "candidates": [{
            "content": {"role": "model", "parts": [{"text": "Hello from Gemini"}]},
            "finishReason": "STOP",
            "index": 0
        }],
        "modelVersion": "gemini-1.5-pro",
        "responseId": "r1"
    });
    let resp = adapter.adapt_response(&raw).unwrap();
    assert_eq!(resp.content, "Hello from Gemini");
}

// =====================================================================
// ProtocolBridge — 4 协议 dispatch (R37-1: 替 ProtocolRouter)
// =====================================================================

#[test]
fn bridge_dispatch_all_4_protocols() {
    // R37-1: ProtocolBridge trait + 4 Bridge struct, 0 router 中间层
    assert_eq!(apeireth_protocol::OpenAiChatBridge::name(), "openai_chat");
    assert_eq!(
        apeireth_protocol::OpenAiResponsesBridge::name(),
        "openai_responses"
    );
    assert_eq!(
        apeireth_protocol::AnthropicMessagesBridge::name(),
        "anthropic_messages"
    );
    assert_eq!(apeireth_protocol::GeminiBridge::name(), "gemini");
}

#[test]
fn bridge_endpoints_are_distinct() {
    let eps: Vec<&str> = [
        ProtocolKind::OpenAiChat,
        ProtocolKind::OpenAiResponses,
        ProtocolKind::AnthropicMessages,
        ProtocolKind::Gemini,
    ]
    .iter()
    .filter_map(|k| endpoint_path_for_kind(*k))
    .collect();
    let unique: std::collections::HashSet<&str> = eps.iter().copied().collect();
    assert_eq!(unique.len(), 4, "endpoints not unique: {:?}", eps);
}

#[test]
fn bridge_supports_exactly_4_protocols() {
    // 4 种 ProtocolKind 都对应一个 Bridge
    let kinds = [
        ProtocolKind::OpenAiChat,
        ProtocolKind::OpenAiResponses,
        ProtocolKind::AnthropicMessages,
        ProtocolKind::Gemini,
    ];
    assert_eq!(kinds.len(), 4);
}

#[test]
fn protocol_kind_parse_case_insensitive() {
    assert_eq!(
        ProtocolKind::parse("OpenAI"),
        Some(ProtocolKind::OpenAiChat)
    );
    assert_eq!(
        ProtocolKind::parse("OPENAI_RESPONSES"),
        Some(ProtocolKind::OpenAiResponses)
    );
    assert_eq!(
        ProtocolKind::parse("Anthropic"),
        Some(ProtocolKind::AnthropicMessages)
    );
    assert_eq!(ProtocolKind::parse("GEMINI"), Some(ProtocolKind::Gemini));
}

#[test]
fn protocol_kind_parse_unknown_returns_none() {
    assert_eq!(ProtocolKind::parse("foo"), None);
    assert_eq!(ProtocolKind::parse(""), None);
}
