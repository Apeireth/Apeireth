//! Deterministic transport tests for the migrated anthropic provider.
//!
//! These prove the canonical [`AnthropicProviderCapability`] against a local
//! mock HTTP server — no Internet, no real API key — and, critically, assert
//! the wire envelope is Anthropic Messages API, **not** OpenAI Chat Completions
//! (§41). The mock is built on `tokio::net::TcpListener` + `tokio::io`, so no
//! new test dependency is introduced.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use apeireth_plugin::{CredentialResolver, ProviderCapability, ProviderError, StaticCredentials};
use apeireth_protocol::canonical::{NormalizedFinishReason, NormalizedMessage, NormalizedRequest};
use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
use apeireth_provider::credentials::ANTHROPIC_API_KEY;

#[derive(Clone)]
struct CannedResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

struct MockServer {
    base_url: String,
    received: Arc<Mutex<Option<RecordedRequest>>>,
}

#[derive(Debug, Clone)]
struct RecordedRequest {
    request_line: String,
    x_api_key: Option<String>,
    anthropic_version: Option<String>,
    body: serde_json::Value,
}

impl MockServer {
    async fn start(canned: CannedResponse) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let received = Arc::new(Mutex::new(None));
        let received_clone = Arc::clone(&received);
        tokio::spawn(async move {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
            };
            let req = read_request(&mut socket).await;
            *received_clone.lock().unwrap() = Some(req);
            write_response(&mut socket, &canned).await;
        });
        Self {
            base_url: format!("http://{addr}"),
            received,
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn received(&self) -> RecordedRequest {
        self.received
            .lock()
            .unwrap()
            .clone()
            .expect("server saw no request")
    }
}

async fn read_request<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) -> RecordedRequest {
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = reader.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(idx) = find_subsequence(&buf, b"\r\n\r\n") {
            let header_len = idx + 4;
            let headers = String::from_utf8_lossy(&buf[..header_len]).to_string();
            let content_length = headers
                .lines()
                .find_map(|l| {
                    let l = l.to_ascii_lowercase();
                    l.strip_prefix("content-length: ")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            while buf.len() < header_len + content_length {
                let n = reader.read(&mut tmp).await.unwrap();
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
            }
            break;
        }
    }

    let text = String::from_utf8_lossy(&buf).to_string();
    let request_line = text.lines().next().unwrap_or("").to_string();
    // Match header names case-insensitively, preserve values.
    let x_api_key = text.lines().find_map(|l| {
        if l.to_ascii_lowercase().starts_with("x-api-key:") {
            Some(l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string())
        } else {
            None
        }
    });
    let anthropic_version = text.lines().find_map(|l| {
        if l.to_ascii_lowercase().starts_with("anthropic-version:") {
            Some(l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string())
        } else {
            None
        }
    });
    let body_text = text.split("\r\n\r\n").nth(1).unwrap_or("");
    let body = serde_json::from_str(body_text).unwrap_or(serde_json::Value::Null);
    RecordedRequest {
        request_line,
        x_api_key,
        anthropic_version,
        body,
    }
}

async fn write_response<W: tokio::io::AsyncWrite + Unpin>(writer: &mut W, canned: &CannedResponse) {
    use tokio::io::AsyncWriteExt;
    let reason = match canned.status {
        200 => "OK",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let mut out = format!("HTTP/1.1 {} {}\r\n", canned.status, reason);
    out.push_str("content-type: application/json\r\n");
    out.push_str(&format!("content-length: {}\r\n", canned.body.len()));
    out.push_str("connection: close\r\n");
    for (k, v) in &canned.headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    out.push_str("\r\n");
    out.push_str(&canned.body);
    writer.write_all(out.as_bytes()).await.unwrap();
    writer.flush().await.unwrap();
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

// ============================================================
// Fixtures
// ============================================================

const FAKE_KEY: &str = "sk-ant-fake-test-key-67890";
const MODEL: &str = "MiniMax-M3";

fn fake_resolver() -> Arc<dyn CredentialResolver> {
    Arc::new(StaticCredentials::new().with(ANTHROPIC_API_KEY, FAKE_KEY))
}

fn plugin_at(base_url: &str) -> Arc<AnthropicProviderPlugin> {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin =
        Arc::new(AnthropicProviderPlugin::new(base_url, vec![MODEL.into()], http, 500).unwrap());
    plugin.attach_resolver_for_test(fake_resolver());
    plugin
}

fn request() -> NormalizedRequest {
    NormalizedRequest::new(
        MODEL,
        vec![
            NormalizedMessage::system("be brief"),
            NormalizedMessage::user("say hello"),
        ],
    )
}

fn anthropic_success_body() -> String {
    serde_json::json!({
        "id": "msg_test",
        "model": MODEL,
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": "hello world"}],
        "usage": {"input_tokens": 12, "output_tokens": 3}
    })
    .to_string()
}

// ============================================================
// Tests
// ============================================================

#[tokio::test]
async fn success_maps_content_usage_and_finish_reason() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: anthropic_success_body(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let resp = cap.complete(&request()).await.expect("success");
    assert_eq!(resp.content, "hello world");
    assert_eq!(resp.id, "msg_test");
    assert_eq!(resp.model, MODEL);
    assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Stop));
    assert_eq!(resp.usage.prompt_tokens, 12);
    assert_eq!(resp.usage.completion_tokens, 3);
    assert_eq!(resp.usage.total_tokens, 15);
}

#[tokio::test]
async fn request_conversion_sends_anthropic_envelope_not_openai() {
    // §41: the wire envelope must be Anthropic Messages API, not OpenAI Chat
    // Completions. Assert the path, headers, system field, and the absence of
    // OpenAI-only fields.
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: anthropic_success_body(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    cap.complete(&request()).await.unwrap();
    let recorded = server.received();

    // Path: /v1/messages, NOT /chat/completions.
    assert!(
        recorded.request_line.starts_with("POST /v1/messages"),
        "anthropic path: {}",
        recorded.request_line
    );
    // Auth: x-api-key header (not Authorization: Bearer).
    assert_eq!(
        recorded.x_api_key.as_deref(),
        Some(FAKE_KEY),
        "x-api-key header carries the key"
    );
    // API version header.
    assert_eq!(recorded.anthropic_version.as_deref(), Some("2023-06-01"));
    // System is a top-level field, not a messages entry.
    assert_eq!(recorded.body["system"], "be brief");
    let messages = recorded.body["messages"]
        .as_array()
        .expect("messages array");
    assert_eq!(messages.len(), 1, "system was extracted out of messages");
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "say hello");
    // max_tokens is present (required by Anthropic).
    assert_eq!(recorded.body["max_tokens"], 1024);
    // No OpenAI-only fields leak onto the wire.
    assert!(recorded.body.get("choices").is_none(), "no OpenAI choices");
    assert!(recorded.body.get("stream").is_none(), "no stream field");
}

#[tokio::test]
async fn missing_credential_fails_permanently_without_network() {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        AnthropicProviderPlugin::new("http://127.0.0.1:1", vec![MODEL.into()], http, 500).unwrap(),
    );
    // Intentionally do NOT attach a resolver.
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::AuthFailed { .. }), "{err}");
    assert!(!err.is_retryable(), "missing key is permanent");
}

#[tokio::test]
async fn http_401_maps_to_auth_failed_permanent() {
    let server = MockServer::start(CannedResponse {
        status: 401,
        headers: vec![],
        body: r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#
            .to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::AuthFailed { .. }), "{err}");
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn http_429_maps_to_rate_limited_retryable() {
    let server = MockServer::start(CannedResponse {
        status: 429,
        headers: vec![],
        body: r#"{"type":"error","error":{"type":"rate_limit_error"}}"#.to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::RateLimited { .. }), "{err}");
    assert!(err.is_retryable());
}

#[tokio::test]
async fn http_500_maps_to_refused_permanent() {
    let server = MockServer::start(CannedResponse {
        status: 500,
        headers: vec![],
        body: r#"{"type":"error","error":{"type":"api_error"}}"#.to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Refused { .. }), "{err}");
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn transport_failure_maps_to_network_retryable() {
    // Bind then drop a listener to get a reliably-closed port.
    let addr = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap()
    };
    let cap = plugin_at(&format!("http://{addr}")).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(
        matches!(
            err,
            ProviderError::Network { .. } | ProviderError::Timeout { .. }
        ),
        "{err}"
    );
    assert!(err.is_retryable());
}

#[tokio::test]
async fn timeout_maps_to_timeout_retryable() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((socket, _)) = listener.accept().await {
            tokio::time::sleep(Duration::from_secs(10)).await;
            drop(socket);
        }
    });
    let cap = plugin_at(&format!("http://{addr}")).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Timeout { .. }), "{err}");
    assert!(err.is_retryable());
}

#[tokio::test]
async fn response_conversion_maps_non_text_blocks_and_stop_reasons() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: serde_json::json!({
            "id": "msg_mixed",
            "model": MODEL,
            "stop_reason": "max_tokens",
            "content": [
                {"type": "tool_use", "id": "t1", "name": "n", "input": {}},
                {"type": "text", "text": "partial answer"}
            ],
            "usage": {"input_tokens": 7, "output_tokens": 2}
        })
        .to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let resp = cap.complete(&request()).await.expect("success");
    // First text block is taken; non-text blocks are skipped.
    assert_eq!(resp.content, "partial answer");
    assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Length));
    assert_eq!(resp.usage.total_tokens, 9);
}

#[tokio::test]
async fn model_mapping_sends_vendor_wire_name_for_canonical_request() {
    // §35: a request naming the canonical id (minimax-m3) must send the vendor
    // wire spelling (MiniMax-M3) in the Anthropic body's model field.
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: anthropic_success_body(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let canonical_req = NormalizedRequest::new("minimax-m3", vec![NormalizedMessage::user("hi")]);
    cap.complete(&canonical_req).await.unwrap();
    let recorded = server.received();
    assert_eq!(
        recorded.body["model"], "MiniMax-M3",
        "wire name, not canonical id"
    );
}

#[tokio::test]
async fn does_not_go_through_the_legacy_bridge() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: anthropic_success_body(),
    })
    .await;
    let cap: Arc<dyn ProviderCapability> =
        plugin_at(server.base_url()).provider_for_test() as Arc<dyn ProviderCapability>;

    assert_eq!(cap.id().as_str(), "provider.anthropic");
    let resp = cap.complete(&request()).await.expect("completes");
    assert_eq!(resp.content, "hello world");
}

#[test]
fn canonical_anthropic_type_is_not_a_legacy_bridge() {
    fn assert_provider_capability<T: apeireth_plugin::ProviderCapability>() {}
    assert_provider_capability::<apeireth_provider::canonical_anthropic::AnthropicProviderCapability>(
    );
}
