//! Deterministic transport tests for the migrated openai-compatible provider.
//!
//! These prove the canonical [`OpenAiCompatibleProviderCapability`] against a
//! local mock HTTP server — no Internet, no real API key — covering the §42
//! matrix. The mock is built on `tokio::net::TcpListener` + `tokio::io`, so no
//! new test dependency is introduced.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use apeireth_plugin::{CredentialResolver, ProviderCapability, ProviderError, StaticCredentials};
use apeireth_protocol::canonical::{NormalizedFinishReason, NormalizedMessage, NormalizedRequest};
use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
use apeireth_provider::credentials::OPENAI_COMPATIBLE_API_KEY;

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
    authorization: Option<String>,
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
    let authorization = text.lines().find_map(|l| {
        if l.to_ascii_lowercase().starts_with("authorization:") {
            Some(l.splitn(2, ':').nth(1).unwrap_or("").trim().to_string())
        } else {
            None
        }
    });
    let body_text = text.split("\r\n\r\n").nth(1).unwrap_or("");
    let body = serde_json::from_str(body_text).unwrap_or(serde_json::Value::Null);
    RecordedRequest {
        request_line,
        authorization,
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

const FAKE_KEY: &str = "sk-openai-fake-test-key";
const MODEL: &str = "gpt-4o-mini";

fn fake_resolver() -> Arc<dyn CredentialResolver> {
    Arc::new(StaticCredentials::new().with(OPENAI_COMPATIBLE_API_KEY, FAKE_KEY))
}

fn plugin_at(base_url: &str) -> Arc<OpenAiCompatibleProviderPlugin> {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(base_url, vec![MODEL.into()], http, 500).unwrap(),
    );
    plugin.attach_resolver_for_test(fake_resolver());
    plugin
}

fn request() -> NormalizedRequest {
    NormalizedRequest::new(MODEL, vec![NormalizedMessage::user("say hello")])
}

fn openai_success_body() -> String {
    serde_json::json!({
        "id": "chatcmpl-test",
        "model": MODEL,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "hello world"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 12, "completion_tokens": 3, "total_tokens": 15}
    })
    .to_string()
}

#[tokio::test]
async fn success_maps_content_usage_and_finish_reason() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: openai_success_body(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let resp = cap.complete(&request()).await.expect("success");
    assert_eq!(resp.content, "hello world");
    assert_eq!(resp.id, "chatcmpl-test");
    assert_eq!(resp.model, MODEL);
    assert_eq!(resp.finish_reason, Some(NormalizedFinishReason::Stop));
    assert_eq!(resp.usage.prompt_tokens, 12);
    assert_eq!(resp.usage.completion_tokens, 3);
    assert_eq!(resp.usage.total_tokens, 15);
}

#[tokio::test]
async fn request_conversion_sends_openai_chat_body_and_bearer_auth() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: openai_success_body(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    cap.complete(&request()).await.unwrap();
    let recorded = server.received();
    assert!(
        recorded.request_line.starts_with("POST /chat/completions"),
        "path: {}",
        recorded.request_line
    );
    assert_eq!(
        recorded.authorization.as_deref(),
        Some(format!("Bearer {FAKE_KEY}").as_str()),
        "Bearer auth carries the key"
    );
    assert_eq!(recorded.body["model"], MODEL);
    assert_eq!(recorded.body["stream"], false);
    let messages = recorded.body["messages"].as_array().expect("array");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "say hello");
}

#[tokio::test]
async fn base_url_normalization_handles_trailing_slash() {
    // §22: a trailing slash on base_url must not produce //chat/completions.
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: openai_success_body(),
    })
    .await;
    // Point the provider at the server WITH a trailing slash.
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(
            format!("{}/", server.base_url()),
            vec![MODEL.into()],
            http,
            500,
        )
        .unwrap(),
    );
    plugin.attach_resolver_for_test(fake_resolver());
    let cap = plugin.provider_for_test();

    let resp = cap.complete(&request()).await.expect("success");
    assert_eq!(resp.content, "hello world");
    let recorded = server.received();
    assert!(
        recorded.request_line.starts_with("POST /chat/completions"),
        "no double slash: {}",
        recorded.request_line
    );
}

#[tokio::test]
async fn model_mapping_sends_vendor_wire_name_for_canonical_request() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: serde_json::json!({
            "id": "x", "model": "Qwen/Qwen3-32B",
            "choices": [{"message": {"content": "ok"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })
        .to_string(),
    })
    .await;
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(
            server.base_url(),
            vec!["Qwen/Qwen3-32B".into()],
            http,
            500,
        )
        .unwrap(),
    );
    plugin.attach_resolver_for_test(fake_resolver());
    let cap = plugin.provider_for_test();

    // Canonical id (slash folded to -) maps to the vendor wire name.
    let req = NormalizedRequest::new("qwen-qwen3-32b", vec![NormalizedMessage::user("hi")]);
    cap.complete(&req).await.unwrap();
    let recorded = server.received();
    assert_eq!(
        recorded.body["model"], "Qwen/Qwen3-32B",
        "wire name, not canonical id"
    );
}

#[tokio::test]
async fn missing_credential_fails_permanently_without_network() {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new("http://127.0.0.1:1", vec![MODEL.into()], http, 500)
            .unwrap(),
    );
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::AuthFailed { .. }), "{err}");
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn http_401_maps_to_auth_failed_permanent() {
    let server = MockServer::start(CannedResponse {
        status: 401,
        headers: vec![],
        body: r#"{"error":"invalid api key"}"#.to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::AuthFailed { .. }));
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn http_429_maps_to_rate_limited_retryable() {
    let server = MockServer::start(CannedResponse {
        status: 429,
        headers: vec![],
        body: "2".to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::RateLimited { .. }));
    assert!(err.is_retryable());
}

#[tokio::test]
async fn http_500_maps_to_refused_permanent() {
    let server = MockServer::start(CannedResponse {
        status: 500,
        headers: vec![],
        body: r#"{"error":"internal"}"#.to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Refused { .. }));
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn transport_failure_maps_to_network_retryable() {
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
async fn malformed_response_maps_to_bad_response() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: "not json at all".to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::BadResponse { .. }));
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn empty_choices_maps_to_bad_response() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: serde_json::json!({"id": "x", "choices": []}).to_string(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::BadResponse { .. }));
}

#[tokio::test]
async fn unsupported_model_is_rejected_not_guessed() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: openai_success_body(),
    })
    .await;
    let cap = plugin_at(server.base_url()).provider_for_test();

    let req = NormalizedRequest::new("minimax-m3", vec![NormalizedMessage::user("hi")]);
    let err = cap.complete(&req).await.unwrap_err();
    assert!(matches!(err, ProviderError::BadResponse { .. }), "{err}");
    // No HTTP should have been attempted: the model is rejected before the call.
    assert!(
        server.received.lock().unwrap().is_none(),
        "no HTTP for an unsupported model"
    );
}

#[tokio::test]
async fn does_not_go_through_the_legacy_bridge() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: openai_success_body(),
    })
    .await;
    let cap: Arc<dyn ProviderCapability> =
        plugin_at(server.base_url()).provider_for_test() as Arc<dyn ProviderCapability>;

    assert_eq!(cap.id().as_str(), "provider.openai-compatible");
    let resp = cap.complete(&request()).await.expect("completes");
    assert_eq!(resp.content, "hello world");
}

#[test]
fn canonical_openai_compatible_type_is_not_a_legacy_bridge() {
    fn assert_provider_capability<T: apeireth_plugin::ProviderCapability>() {}
    assert_provider_capability::<
        apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderCapability,
    >();
}
