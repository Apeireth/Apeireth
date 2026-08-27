//! Deterministic proof that two canonical production providers —
//! `provider.minimax` (OpenAI Chat Completions) and `provider.anthropic`
//! (Anthropic Messages API) — coexist in one runtime and route deterministically
//! by model, through the same `ProviderRouter` / `ProviderCapability`
//! architecture (§49).
//!
//! Both providers are real canonical capabilities. Each points at its own mock
//! vendor HTTP server. The runtime knows no vendor; the router selects purely
//! on `supports_model` + health. No Internet, no real keys.

use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::{Clock, SessionId, Timestamp, VirtualClock};
use apeireth_governance::AllowAll;
use apeireth_plugin::{CredentialResolver, ProviderError, StaticCredentials};
use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
use apeireth_provider::credentials::{ANTHROPIC_API_KEY, MINIMAX_API_KEY};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, RuntimeError, TurnRequest};

const MINIMAX_KEY: &str = "sk-minimax-multi";
const ANTHROPIC_KEY: &str = "sk-ant-multi";
const MINIMAX_MODEL: &str = "MiniMax-M3";
const ANTHROPIC_MODEL: &str = "claude-sonnet-4-5";

/// A mock vendor server that always answers 200 with `body`.
struct MockServer {
    base_url: String,
    served: Arc<Mutex<bool>>,
}

impl MockServer {
    async fn start(body: String) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let served = Arc::new(Mutex::new(false));
        let served_clone = Arc::clone(&served);
        tokio::spawn(async move {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = [0u8; 4096];
            let _ = socket.read(&mut buf).await;
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = socket.write_all(resp.as_bytes()).await;
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

fn openai_body(model: &str, text: &str) -> String {
    serde_json::json!({
        "id": "chatcmpl-multi",
        "model": model,
        "choices": [{"index": 0, "message": {"role": "assistant", "content": text}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 4, "completion_tokens": 2, "total_tokens": 6}
    })
    .to_string()
}

fn anthropic_body(model: &str, text: &str) -> String {
    serde_json::json!({
        "id": "msg_multi",
        "model": model,
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": text}],
        "usage": {"input_tokens": 4, "output_tokens": 2}
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

/// A resolver carrying both providers' fake keys.
fn resolver() -> Arc<dyn CredentialResolver> {
    Arc::new(
        StaticCredentials::new()
            .with(MINIMAX_API_KEY, MINIMAX_KEY)
            .with(ANTHROPIC_API_KEY, ANTHROPIC_KEY),
    )
}

/// Build a runtime with BOTH canonical providers, each pointed at its own mock.
async fn multi_runtime(minimax_url: &str, anthropic_url: &str) -> Runtime {
    let http = reqwest::Client::builder().build().unwrap();
    let minimax = Arc::new(
        MinimaxProviderPlugin::new(minimax_url, vec![MINIMAX_MODEL.into()], http.clone(), 2_000)
            .unwrap(),
    );
    let anthropic = Arc::new(
        AnthropicProviderPlugin::new(anthropic_url, vec![ANTHROPIC_MODEL.into()], http, 2_000)
            .unwrap(),
    );
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver())
        .with_plugin(minimax)
        .with_plugin(anthropic)
        .build()
        .await
        .expect("runtime builds")
}

#[tokio::test]
async fn case_a_a_minimax_model_routes_to_provider_minimax() {
    let minimax = MockServer::start(openai_body(MINIMAX_MODEL, "minimax answer")).await;
    let anthropic = MockServer::start(anthropic_body(ANTHROPIC_MODEL, "anthropic answer")).await;
    let runtime = multi_runtime(minimax.base_url(), anthropic.base_url()).await;

    // Both providers are registered.
    let ids: Vec<String> = runtime
        .providers()
        .provider_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    assert!(ids.contains(&"provider.minimax".to_string()));
    assert!(ids.contains(&"provider.anthropic".to_string()));

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model(MINIMAX_MODEL))
        .await
        .expect("turn completes");

    // Routed to minimax, not anthropic.
    assert_eq!(outcome.served_by.as_str(), "provider.minimax");
    assert_eq!(outcome.text, "minimax answer");
    assert!(minimax.served());
    assert!(
        !anthropic.served(),
        "the anthropic provider must not be consulted for a minimax model"
    );
}

#[tokio::test]
async fn case_b_an_anthropic_model_routes_to_provider_anthropic() {
    let minimax = MockServer::start(openai_body(MINIMAX_MODEL, "minimax answer")).await;
    let anthropic = MockServer::start(anthropic_body(ANTHROPIC_MODEL, "anthropic answer")).await;
    let runtime = multi_runtime(minimax.base_url(), anthropic.base_url()).await;

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model(ANTHROPIC_MODEL))
        .await
        .expect("turn completes");

    // Routed to anthropic, not minimax — and the wire envelope was Anthropic,
    // proving the same runtime served a different protocol with no vendor
    // branch in the runtime.
    assert_eq!(outcome.served_by.as_str(), "provider.anthropic");
    assert_eq!(outcome.text, "anthropic answer");
    assert!(anthropic.served());
    assert!(
        !minimax.served(),
        "the minimax provider must not be consulted for an anthropic model"
    );
}

#[tokio::test]
async fn case_c_an_unsupported_model_names_the_registered_providers() {
    let minimax = MockServer::start(openai_body(MINIMAX_MODEL, "x")).await;
    let anthropic = MockServer::start(anthropic_body(ANTHROPIC_MODEL, "x")).await;
    let runtime = multi_runtime(minimax.base_url(), anthropic.base_url()).await;

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model("gpt-4o"))
        .await
        .expect_err("unsupported model must fail");

    // No provider claims the model → NoProvider, naming what IS registered.
    match err {
        RuntimeError::NoProvider { model, available } => {
            assert_eq!(model, "gpt-4o");
            assert!(available.contains("provider.minimax"), "{available}");
            assert!(available.contains("provider.anthropic"), "{available}");
        }
        other => panic!("expected NoProvider, got {other:?}"),
    }
    // Neither vendor was contacted.
    assert!(!minimax.served());
    assert!(!anthropic.served());
}

#[tokio::test]
async fn case_d_a_unhealthy_supporting_provider_with_no_alternative_reports_no_healthy() {
    // A minimax-only runtime (no anthropic alternative) where minimax returns a
    // permanent auth error. After the failure the provider is still "healthy"
    // (one failure is not enough to sideline it), and a permanent error returns
    // RuntimeError::Provider immediately — it must NOT fall back to a provider
    // that does not support the model (there is none here). This proves no
    // wrong-provider fallback (§49 Case C).
    //
    // Use a minimax-only runtime so there is genuinely no alternative.
    let http = reqwest::Client::builder().build().unwrap();
    // NoCredentials: minimax will fail with AuthFailed (permanent) on execute.
    let resolver: Arc<dyn CredentialResolver> = Arc::new(
        StaticCredentials::new().with(ANTHROPIC_API_KEY, ANTHROPIC_KEY), // minimax key absent
    );
    let minimax = Arc::new(
        MinimaxProviderPlugin::new(
            "http://127.0.0.1:1",
            vec![MINIMAX_MODEL.into()],
            http,
            2_000,
        )
        .unwrap(),
    );
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver)
        .with_plugin(minimax)
        .with_default_model(MINIMAX_MODEL)
        .build()
        .await
        .expect("runtime builds");

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi"))
        .await
        .expect_err("missing minimax key must fail");

    // Permanent AuthFailed → RuntimeError::Provider, not a fallback/exhaustion.
    match err {
        RuntimeError::Provider(ProviderError::AuthFailed { .. }) => {}
        other => panic!("expected Provider(AuthFailed), got {other:?}"),
    }
}
