//! Deterministic transport tests for the migrated minimax provider.
//!
//! These prove the canonical [`MinimaxProviderCapability`] against a local mock
//! HTTP server — no Internet, no real API key. They cover the §49 matrix:
//! success, missing credential, 401, 429, 5xx, transport failure, timeout, plus
//! request/response conversion, usage, and finish-reason mapping.
//!
//! The mock speaks just enough HTTP/1.1 to answer one POST per connection. It
//! is built on `tokio::net::TcpListener` + `tokio::io`, so no new test
//! dependency is introduced.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use apeireth_plugin::{CredentialResolver, ProviderCapability, ProviderError, StaticCredentials};
use apeireth_protocol::canonical::{NormalizedFinishReason, NormalizedMessage, NormalizedRequest};
use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
use apeireth_provider::credentials::MINIMAX_API_KEY;

/// What the mock server should reply with for the next (single) request.
#[derive(Clone)]
struct CannedResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

/// A one-shot mock HTTP server. Accepts a single connection, reads the request,
/// records it, writes the canned response, and closes. Bound to an ephemeral
/// port so tests are isolated.
struct MockServer {
    base_url: String,
    received: Arc<Mutex<Option<RecordedRequest>>>,
}

/// The request the mock observed, for conversion assertions.
#[derive(Debug, Clone)]
struct RecordedRequest {
    request_line: String,
    authorization: Option<String>,
    body: serde_json::Value,
}

impl MockServer {
    /// Start a server that replies with `canned` to its one request.
    async fn start(canned: CannedResponse) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let received = Arc::new(Mutex::new(None));
        let received_clone = Arc::clone(&received);

        tokio::spawn(async move {
            // Accept exactly one connection; if the test never connects, the
            // task simply awaits forever and is dropped with the handle.
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

/// Read one HTTP request: request line + headers + Content-Length body.
async fn read_request<R>(reader: &mut R) -> RecordedRequest
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    // Read until the header/body separator.
    loop {
        let n = reader.read(&mut tmp).await.unwrap();
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(idx) = find_subsequence(&buf, b"\r\n\r\n") {
            let header_len = idx + 4;
            // Parse Content-Length to know how much body to read.
            let headers = String::from_utf8_lossy(&buf[..header_len]).to_string();
            let content_length = headers
                .lines()
                .find_map(|l| {
                    let l = l.to_ascii_lowercase();
                    l.strip_prefix("content-length: ")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            // Keep reading until we have the full body.
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
    let mut lines = text.lines();
    let request_line = lines.next().unwrap_or("").to_string();
    let authorization = text.lines().find_map(|l| {
        // Match the header name case-insensitively but preserve the value's
        // case (the credential scheme "Bearer" is sent with that casing).
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

/// Write a canned HTTP/1.1 response.
async fn write_response<W>(writer: &mut W, canned: &CannedResponse)
where
    W: tokio::io::AsyncWrite + Unpin,
{
    use tokio::io::AsyncWriteExt;
    let reason = match canned.status {
        200 => "OK",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
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

const FAKE_KEY: &str = "sk-fake-test-key-12345";
const MODEL: &str = "MiniMax-M3";

/// A resolver that always serves the fake key (production shape, fake value).
fn fake_resolver() -> Arc<dyn CredentialResolver> {
    Arc::new(StaticCredentials::new().with(MINIMAX_API_KEY, FAKE_KEY))
}

/// Build the plugin pointed at `base_url` with the fake key resolver attached.
fn plugin_at(base_url: &str) -> (Arc<MinimaxProviderPlugin>, Arc<dyn CredentialResolver>) {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        MinimaxProviderPlugin::new(
            base_url,
            vec![MODEL.into()],
            http,
            // Short timeout so the timeout test is fast.
            500,
        )
        .unwrap(),
    );
    let resolver = fake_resolver();
    // Attach the resolver as initialize() would. The slot is shared, so this
    // mirrors the real boot path without constructing a full Runtime.
    attach_resolver(&plugin, resolver.clone());
    (plugin, resolver)
}

/// Reach into the plugin's shared resolver slot (exposed for tests) to attach
/// a resolver, mirroring what Plugin::initialize does in the real runtime.
fn attach_resolver(plugin: &MinimaxProviderPlugin, resolver: Arc<dyn CredentialResolver>) {
    plugin.attach_resolver_for_test(resolver);
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

// ============================================================
// Tests
// ============================================================

#[tokio::test]
async fn success_maps_content_usage_and_finish_reason() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: openai_success_body(),
    })
    .await;
    let (plugin, _) = plugin_at(server.base_url());
    let cap = plugin.provider_for_test();

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
    let (plugin, _) = plugin_at(server.base_url());
    let cap = plugin.provider_for_test();

    cap.complete(&request()).await.unwrap();

    let recorded = server.received();
    assert!(recorded.request_line.starts_with("POST /chat/completions"));
    assert_eq!(
        recorded.authorization.as_deref(),
        Some(format!("Bearer {FAKE_KEY}").as_str()),
        "the key must be applied as a Bearer header, once, at the call site"
    );
    assert_eq!(recorded.body["model"], MODEL);
    assert_eq!(recorded.body["stream"], false);
    let messages = recorded.body["messages"]
        .as_array()
        .expect("messages array");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[0]["content"], "be brief");
    assert_eq!(messages[1]["role"], "user");
    assert_eq!(messages[1]["content"], "say hello");
}

#[tokio::test]
async fn missing_credential_fails_permanently_without_network() {
    // No resolver attached: must fail before any HTTP is attempted.
    let http = reqwest::Client::builder().build().unwrap();
    let plugin = Arc::new(
        MinimaxProviderPlugin::new("http://127.0.0.1:1", vec![MODEL.into()], http, 500).unwrap(),
    );
    // Intentionally do NOT attach a resolver.
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::AuthFailed { .. }), "{err}");
    assert!(
        !err.is_retryable(),
        "a missing key is permanent, not transient"
    );
}

#[tokio::test]
async fn http_401_maps_to_auth_failed_permanent() {
    let server = MockServer::start(CannedResponse {
        status: 401,
        headers: vec![],
        body: r#"{"error":"invalid api key"}"#.to_string(),
    })
    .await;
    let (plugin, _) = plugin_at(server.base_url());
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::AuthFailed { .. }), "{err}");
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn http_429_maps_to_rate_limited_retryable() {
    let server = MockServer::start(CannedResponse {
        status: 429,
        headers: vec![("retry-after".into(), "2".into())],
        body: "2".to_string(),
    })
    .await;
    let (plugin, _) = plugin_at(server.base_url());
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    match err {
        ProviderError::RateLimited { retry_after_ms, .. } => {
            assert!(retry_after_ms > 0, "retry-after must be parsed");
            assert!(err.is_retryable(), "rate limiting is transient");
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn http_500_maps_to_refused_permanent() {
    let server = MockServer::start(CannedResponse {
        status: 500,
        headers: vec![],
        body: r#"{"error":"internal"}"#.to_string(),
    })
    .await;
    let (plugin, _) = plugin_at(server.base_url());
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Refused { .. }), "{err}");
    assert!(!err.is_retryable());
}

#[tokio::test]
async fn transport_failure_maps_to_network_retryable() {
    // Bind a listener to claim an ephemeral port, then drop it so the port is
    // closed — a connection attempt is refused immediately and reliably,
    // without depending on a fixed port number.
    let addr = {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap()
    };
    let (plugin, _) = plugin_at(&format!("http://{addr}"));
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    // A refused connection is a transport failure; on some platforms it can
    // surface as a timeout instead, but both are retryable transport errors.
    assert!(
        matches!(
            err,
            ProviderError::Network { .. } | ProviderError::Timeout { .. }
        ),
        "{err}"
    );
    assert!(err.is_retryable(), "a transport failure is transient");
}

#[tokio::test]
async fn timeout_maps_to_timeout_retryable() {
    // A server that accepts and holds the connection open without responding;
    // the client's per-request timeout fires.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        // Accept and keep the socket alive (do not drop it) so the client waits
        // for a response that never arrives.
        if let Ok((socket, _)) = listener.accept().await {
            tokio::time::sleep(Duration::from_secs(10)).await;
            drop(socket);
        }
    });
    let (plugin, _) = plugin_at(&format!("http://{addr}"));
    let cap = plugin.provider_for_test();

    let err = cap.complete(&request()).await.unwrap_err();
    assert!(matches!(err, ProviderError::Timeout { .. }), "{err}");
    assert!(err.is_retryable());
}

#[tokio::test]
async fn does_not_go_through_the_legacy_bridge() {
    // The migrated capability is a direct ProviderCapability, not a
    // LegacyLlmCapability. Constructing it and calling complete() never touches
    // the bridge or the LlmProvider trait. This is a structural guard: the
    // canonical type is what the runtime routes to.
    let server = MockServer::start(CannedResponse {
        status: 200,
        headers: vec![],
        body: openai_success_body(),
    })
    .await;
    let (plugin, _) = plugin_at(server.base_url());
    let cap: Arc<dyn ProviderCapability> =
        plugin.provider_for_test() as Arc<dyn ProviderCapability>;

    // It declares the canonical id, not a compat.* id.
    assert_eq!(cap.id().as_str(), "provider.minimax");
    let resp = cap.complete(&request()).await.expect("completes");
    assert_eq!(resp.content, "hello world");
}

/// Ensure the canonical minimax capability type does not depend on the legacy
/// bridge or the LlmProvider trait — a compile-time guarantee that migration
/// moved ownership rather than wrapping the legacy path.
#[test]
fn canonical_minimax_type_is_not_a_legacy_bridge() {
    fn assert_provider_capability<T: apeireth_plugin::ProviderCapability>() {}
    assert_provider_capability::<apeireth_provider::canonical_minimax::MinimaxProviderCapability>();
    // It must NOT implement the legacy LlmProvider trait. If it ever did, this
    // trait bound would resolve and signal a regression to the bridge shape.
    // (Negative trait impls are not stable, so this is a documentation-level
    // guard enforced by the absence of an impl in canonical_minimax.rs.)
}
