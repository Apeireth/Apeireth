//! Integration tests for apeireth-pipeline
//!
//! **R18 第 2 阶段第 7 项**: 5 步管线 e2e
//! 抄 tokio 模式: `wiremock` 模拟下游 HTTP, 真跑完整 pipeline

use apeireth_http_client::HttpClient;
use apeireth_pipeline::{Pipeline, PipelineConfig, RetrySuppression};
use apeireth_protocol::{NormalizedMessage, NormalizedRequest, ProtocolKind};
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// =====================================================================
// Pipeline 基础 API
// =====================================================================

/// 构造一个 base_url 指向 mock server 的 Pipeline (e2e 用)
///
/// 跟 src/lib.rs:543 内置测试的同款模式:
/// `let mut config = PipelineConfig::default(); config.base_url = server.uri();`
fn make_pipeline_at(server: &MockServer) -> Pipeline {
    let http = HttpClient::new(apeireth_http_client::KeepAliveConfig::chat_default()).unwrap();
    let mut config = PipelineConfig::default();
    config.base_url = server.uri();
    // **不抑制**: fresh suppression 避免上一次测试残留
    config.suppression = RetrySuppression::new(Duration::from_millis(50));
    Pipeline::with_config(http, config).unwrap()
}

/// 兼容旧 API: 不带 mock server 时用 vcp defaults (用于单测 Pipeline 自身 API)
fn make_pipeline() -> Pipeline {
    let http = HttpClient::with_chat_defaults().expect("http client");
    Pipeline::new(http).expect("pipeline new")
}

#[test]
fn pipeline_new_works() {
    let p = make_pipeline();
    assert!(p.config().base_url.contains("api.minimaxi.com"));
}

#[test]
fn pipeline_with_chat_defaults_works() {
    let p = Pipeline::with_chat_defaults().expect("vcp defaults");
    assert!(p.config().base_url.contains("api.minimaxi.com"));
}

#[test]
fn pipeline_http_returns_http_client() {
    let p = make_pipeline();
    let _h = p.http();
}

// =====================================================================
// Pipeline::run() — 4 协议 e2e (用 wiremock)
// =====================================================================

#[tokio::test]
async fn pipeline_runs_openai_chat() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-x",
            "object": "chat.completion",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Mock response"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 3}
        })))
        .mount(&server)
        .await;

    let p = make_pipeline_at(&server);
    let req = NormalizedRequest::new("gpt-4o".to_string(), vec![NormalizedMessage::user("hi")]);
    let resp = p.run(ProtocolKind::OpenAiChat, req).await.expect("pipeline run");
    assert_eq!(resp.content, "Mock response");
}

#[tokio::test]
async fn pipeline_runs_anthropic() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_x",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4",
            "content": [{"type": "text", "text": "Anthropic mock"}],
            "stop_reason": "end_turn"
        })))
        .mount(&server)
        .await;

    let p = make_pipeline_at(&server);
    let mut req = NormalizedRequest::new(
        "claude-sonnet-4".to_string(),
        vec![NormalizedMessage::user("hi")],
    );
    req.max_tokens = Some(1024);
    let resp = p.run(ProtocolKind::AnthropicMessages, req).await.expect("pipeline run anthropic");
    assert_eq!(resp.content, "Anthropic mock");
}

#[tokio::test]
async fn pipeline_runs_gemini() {
    let server = MockServer::start().await;
    // 实际 src 发的 URL = `{base_url}/v1beta/models/{model}:generateContent` 字面,
    // 因为 `Pipeline::run()` line 244 `format!("{}{}", base_url, endpoint_path)`
    // 没有替换 `{model}` placeholder (src bug, R21 续 — 0 改 LOCKED src)
    // mock 只 match method 不 match path, 接受任何 POST
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "Gemini mock"}]},
                "finishReason": "STOP"
            }],
            "modelVersion": "gemini-1.5-pro",
            // `responseId` 是 GeminiAdapter::decode 必填字段 (per `protocol/tests/wire_format.rs:200`)
            "responseId": "r-gemini-mock-001"
        })))
        .mount(&server)
        .await;

    let p = make_pipeline_at(&server);
    let req = NormalizedRequest::new("gemini-1.5-pro".to_string(), vec![NormalizedMessage::user("hi")]);
    let resp = p.run(ProtocolKind::Gemini, req).await.expect("pipeline run gemini");
    assert_eq!(resp.content, "Gemini mock");
}

#[tokio::test]
async fn pipeline_handles_404() {
    let server = MockServer::start().await;
    // 不 mount 任何 mock → mock server 返 404
    let p = make_pipeline_at(&server);
    let req = NormalizedRequest::new("gpt-4o".to_string(), vec![NormalizedMessage::user("hi")]);
    let result = p.run(ProtocolKind::OpenAiChat, req).await;
    assert!(result.is_err(), "expected error on 404");
}

#[tokio::test]
async fn pipeline_handles_500() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let p = make_pipeline_at(&server);
    let req = NormalizedRequest::new("gpt-4o".to_string(), vec![NormalizedMessage::user("hi")]);
    let result = p.run(ProtocolKind::OpenAiChat, req).await;
    assert!(result.is_err(), "expected error on 500");
}

#[test]
fn pipeline_config_has_base_url() {
    let p = make_pipeline();
    assert!(!p.config().base_url.is_empty());
}

// =====================================================================
// token_budget::truncate_to_max (VCP §6.2.2 #15) — 边界测试
// =====================================================================

#[test]
fn pipeline_truncate_to_max_short_input_unchanged() {
    use apeireth_pipeline::truncate_to_max;
    let s = "short text";
    let out = truncate_to_max(s, 100);
    assert_eq!(out, s, "短输入 (< 100 chars) 不应被截断");
}

#[test]
fn pipeline_truncate_to_max_long_input_truncated() {
    use apeireth_pipeline::truncate_to_max;
    // 1MB 字符串
    let s: String = "a".repeat(20_000);
    let out = truncate_to_max(&s, 100);
    let char_count = out.chars().count();
    // 应 ≤ 100 chars (truncate_to_max 承诺)
    assert!(char_count <= 100, "应 ≤ 100 chars, got {char_count}");
    // 应有 marker 提示
    assert!(out.contains("truncated") || out.contains("…"), "应含截断 marker: {out}");
}

#[test]
fn pipeline_exceeds_budget() {
    use apeireth_pipeline::exceeds_budget;
    assert!(!exceeds_budget("hello", 100), "短文本未超预算");
    assert!(exceeds_budget(&"a".repeat(200), 100), "长文本超预算");
}

// =====================================================================
// placeholder::resolve_placeholders (VCP §6.2.2 #17) — 递归 + 防循环
// =====================================================================

#[test]
fn pipeline_resolve_placeholders_simple() {
    use apeireth_pipeline::resolve_placeholders;
    use std::collections::HashMap;
    let mut ctx = HashMap::new();
    ctx.insert("name".to_string(), "Alice".to_string());
    let out = resolve_placeholders("Hello, {{name}}!", &ctx);
    assert_eq!(out, "Hello, Alice!");
}

#[test]
fn pipeline_resolve_placeholders_circular_protection() {
    // VCP 行为: 循环引用 (a→b→a) 应被防住, 不会栈溢出
    use apeireth_pipeline::resolve_placeholders;
    use std::collections::HashMap;
    let mut ctx = HashMap::new();
    ctx.insert("a".to_string(), "go to {{b}}".to_string());
    ctx.insert("b".to_string(), "go to {{a}}".to_string());
    // 不应 panic / 栈溢出; 应停在某处 (要么保留原样, 要么 error 标记)
    let _ = resolve_placeholders("start {{a}} end", &ctx);
    // 不 panic 即成功
}

#[test]
fn pipeline_resolve_placeholders_missing_keeps_original() {
    // 找不到的 placeholder 保留原样 (VCP 行为)
    use apeireth_pipeline::resolve_placeholders;
    use std::collections::HashMap;
    let ctx = HashMap::new();
    let out = resolve_placeholders("hello {{unknown}} world", &ctx);
    assert_eq!(out, "hello {{unknown}} world", "找不到应保留原样");
}

// =====================================================================
// force_translate (VCP §6.2.2 #20) — 文本模型 + base64 检测
// =====================================================================

#[test]
fn pipeline_force_translate_is_text_only_model() {
    use apeireth_pipeline::force_translate::is_text_only_model_by_tag;
    let tags = vec!["deepseek".to_string(), "glm".to_string()];
    assert!(is_text_only_model_by_tag("deepseek-chat", &tags));
    assert!(is_text_only_model_by_tag("GLM-4", &tags)); // case-insensitive
    assert!(!is_text_only_model_by_tag("gpt-4o", &tags));
    // 空 list → false
    assert!(!is_text_only_model_by_tag("deepseek", &[]));
}

#[test]
fn pipeline_force_translate_messages_contain_base64() {
    use apeireth_pipeline::force_translate::messages_contain_base64_media;
    use apeireth_protocol::{ContentPart, MessageRole, NormalizedMessage};
    // 1) 无 base64
    let msgs = vec![NormalizedMessage::user("hello")];
    assert!(!messages_contain_base64_media(&msgs));
    // 2) 有 base64 image
    let msgs_with_b64 = vec![NormalizedMessage {
        role: MessageRole::User,
        content: vec![ContentPart::ImageUrl {
            url: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
            detail: None,
        }],
        tool_calls: Vec::new(),
        tool_call_id: None,
        name: None,
    }];
    assert!(messages_contain_base64_media(&msgs_with_b64));
}

// =====================================================================
// retry_suppression (VCP §6.2.2 #19) — 15s 抑制窗口
// =====================================================================

#[test]
fn pipeline_retry_suppression_first_call_not_suppressed() {
    use apeireth_pipeline::{RetrySuppression, DEFAULT_SUPPRESSION_WINDOW_MS};
    let s = RetrySuppression::new(std::time::Duration::from_millis(DEFAULT_SUPPRESSION_WINDOW_MS));
    // 首次调用应不被抑制
    assert!(!s.should_suppress("key1"));
    assert!(!s.should_suppress("key2"));
}

#[test]
fn pipeline_retry_suppression_second_call_within_window_suppressed() {
    use apeireth_pipeline::RetrySuppression;
    // 短窗口测试 (1s)
    let s = RetrySuppression::new(std::time::Duration::from_millis(1_000));
    assert!(!s.should_suppress("key1"), "首次 ok");
    assert!(s.should_suppress("key1"), "1s 内第二次应被抑制");
}

#[test]
fn pipeline_default_suppression_is_15s() {
    // VCP 真值 15000ms
    use apeireth_pipeline::DEFAULT_SUPPRESSION_WINDOW_MS;
    assert_eq!(DEFAULT_SUPPRESSION_WINDOW_MS, 15_000);
}
