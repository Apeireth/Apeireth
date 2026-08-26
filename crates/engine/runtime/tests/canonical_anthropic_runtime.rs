//! Deterministic proof that the canonical runtime executes the migrated
//! anthropic provider **without** the LegacyLlmCapability bridge and **without**
//! any OpenAI conversion.
//!
//! The chain being proved:
//!
//! ```text
//!   Runtime::execute
//!     -> ProviderRouter
//!     -> AnthropicProviderCapability (canonical, not the bridge)
//!     -> CredentialResolver (StaticCredentials, fake key)
//!     -> mock vendor HTTP server (Anthropic Messages API)
//!     -> canonical response -> trace
//! ```
//!
//! Everything below the capability edge is real: the runtime, plugin manager,
//! capability registry, router, session store, and agent loop are the canonical
//! implementations. Only the vendor HTTP edge is a local mock, and the
//! credential is a fake. No Internet, no real key, no `LegacyLlmCapability`.

use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::{Clock, SessionId, Timestamp, VirtualClock};
use apeireth_governance::AllowAll;
use apeireth_plugin::{CredentialResolver, ProviderError, StaticCredentials};
use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
use apeireth_provider::credentials::ANTHROPIC_API_KEY;
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, RuntimeError, TurnRequest};

const FAKE_KEY: &str = "sk-ant-runtime-key";
const MODEL: &str = "MiniMax-M3";

#[derive(Clone)]
struct CannedResponse {
    status: u16,
    body: String,
}

struct MockServer {
    base_url: String,
    served: Arc<Mutex<bool>>,
}

impl MockServer {
    async fn start(canned: CannedResponse) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let served = Arc::new(Mutex::new(false));
        let served_clone = Arc::clone(&served);
        tokio::spawn(async move {
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
            };
            use tokio::io::AsyncReadExt;
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            use tokio::io::AsyncWriteExt;
            let reason = match canned.status {
                200 => "OK",
                401 => "Unauthorized",
                _ => "OK",
            };
            let mut out = format!("HTTP/1.1 {} {}\r\n", canned.status, reason);
            out.push_str("content-type: application/json\r\n");
            out.push_str(&format!("content-length: {}\r\n", canned.body.len()));
            out.push_str("connection: close\r\n\r\n");
            out.push_str(&canned.body);
            let _ = socket.write_all(out.as_bytes()).await;
            let _ = socket.flush().await;
            *served_clone.lock().unwrap() = true;
        });
        Self {
            base_url: format!("http://{addr}"),
            served,
        }
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn served(&self) -> bool {
        *self.served.lock().unwrap()
    }
}

fn anthropic_success_body() -> String {
    serde_json::json!({
        "id": "msg_runtime",
        "model": MODEL,
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": "hello from anthropic"}],
        "usage": {"input_tokens": 8, "output_tokens": 4}
    })
    .to_string()
}

fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .unwrap()
            .as_datetime(),
    ))
}

async fn runtime_at(base_url: &str, resolver: Arc<dyn CredentialResolver>) -> Runtime {
    let http = reqwest::Client::builder().build().unwrap();
    let plugin =
        Arc::new(AnthropicProviderPlugin::new(base_url, vec![MODEL.into()], http, 2_000).unwrap());
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver)
        .with_plugin(plugin)
        .with_default_model(MODEL)
        .build()
        .await
        .expect("runtime builds")
}

#[tokio::test]
async fn the_runtime_serves_a_turn_through_the_anthropic_provider() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        body: anthropic_success_body(),
    })
    .await;
    let resolver: Arc<dyn CredentialResolver> =
        Arc::new(StaticCredentials::new().with(ANTHROPIC_API_KEY, FAKE_KEY));
    let runtime = runtime_at(server.base_url(), resolver).await;

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "say hello"))
        .await
        .expect("the turn completes");

    // The migrated canonical anthropic capability served the turn.
    assert_eq!(outcome.served_by.as_str(), "provider.anthropic");
    assert_eq!(outcome.text, "hello from anthropic");
    assert_eq!(outcome.rounds, 1, "no tools, so one provider round");
    assert_eq!(outcome.usage.prompt_tokens, 8);
    assert_eq!(outcome.usage.completion_tokens, 4);
    assert_eq!(outcome.usage.total_tokens, 12);
    assert_eq!(outcome.trace.provider_invocations(), 1);
    assert!(server.served(), "the mock vendor endpoint was actually hit");
}

#[tokio::test]
async fn a_missing_credential_fails_permanently_without_falling_back() {
    let server = MockServer::start(CannedResponse {
        status: 200,
        body: anthropic_success_body(),
    })
    .await;
    let resolver: Arc<dyn CredentialResolver> = Arc::new(apeireth_plugin::NoCredentials);
    let runtime = runtime_at(server.base_url(), resolver).await;

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "say hello"))
        .await
        .expect_err("missing credential must fail the turn");

    // AuthFailed is permanent → RuntimeError::Provider, not ProvidersExhausted.
    match err {
        RuntimeError::Provider(ProviderError::AuthFailed { .. }) => {}
        other => panic!("expected Provider(AuthFailed), got {other:?}"),
    }
    assert!(
        !server.served(),
        "no HTTP must be attempted when the key is missing"
    );
}

#[tokio::test]
async fn the_runtime_never_routes_anthropic_through_openai_or_the_bridge() {
    // Structural guard: the runtime's registered provider is the canonical
    // anthropic capability. served_by proves the routed path did not pass
    // through LegacyLlmCapability (compat.*) or any OpenAI provider.
    let server = MockServer::start(CannedResponse {
        status: 200,
        body: anthropic_success_body(),
    })
    .await;
    let resolver: Arc<dyn CredentialResolver> =
        Arc::new(StaticCredentials::new().with(ANTHROPIC_API_KEY, FAKE_KEY));
    let runtime = runtime_at(server.base_url(), resolver).await;

    let ids: Vec<_> = runtime
        .providers()
        .provider_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    assert_eq!(ids, vec!["provider.anthropic".to_string()]);
    assert!(
        !ids.iter().any(|id| id.starts_with("compat.")),
        "no compatibility bridge provider may be registered: {ids:?}"
    );

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi"))
        .await
        .expect("turn completes");
    assert_eq!(outcome.served_by.as_str(), "provider.anthropic");
}
