//! Deterministic three-provider canonical routing (§40).
//!
//! With Phase 3, three ordinary HTTP provider families are canonical:
//! `provider.minimax`, `provider.anthropic`, and `provider.openai-compatible`.
//! This test proves the router selects deterministically by model across all
//! three, that an unsupported model names the registered providers, and that two
//! providers supporting the same model resolve deterministically by the
//! explicit fallback order (§41 — no insertion-order dependence).
//!
//! All providers are real canonical capabilities, each pointed at its own mock
//! vendor server. No Internet, no real keys.

use std::sync::Arc;
use std::sync::Mutex;

use apeireth_core::kernel::{Clock, SessionId, Timestamp, VirtualClock};
use apeireth_governance::AllowAll;
use apeireth_plugin::{CredentialResolver, StaticCredentials};
use apeireth_provider::canonical_anthropic::AnthropicProviderPlugin;
use apeireth_provider::canonical_minimax::MinimaxProviderPlugin;
use apeireth_provider::canonical_openai_compatible::OpenAiCompatibleProviderPlugin;
use apeireth_provider::credentials::{
    ANTHROPIC_API_KEY, MINIMAX_API_KEY, OPENAI_COMPATIBLE_API_KEY,
};
use apeireth_runtime::canonical::{InMemorySessionStore, Runtime, RuntimeError, TurnRequest};

const MINIMAX_KEY: &str = "sk-minimax-3";
const ANTHROPIC_KEY: &str = "sk-ant-3";
const OPENAI_KEY: &str = "sk-openai-3";
const MINIMAX_MODEL: &str = "MiniMax-M3";
const ANTHROPIC_MODEL: &str = "claude-sonnet-4-5";
const OPENAI_MODEL: &str = "gpt-4o-mini";

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
            let (mut socket, _) = match listener.accept().await {
                Ok(s) => s,
                Err(_) => return,
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

fn openai_body(text: &str) -> String {
    serde_json::json!({
        "id": "x", "model": "m",
        "choices": [{"message": {"content": text}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
    })
    .to_string()
}

fn anthropic_body(text: &str) -> String {
    serde_json::json!({
        "id": "x", "model": "m", "stop_reason": "end_turn",
        "content": [{"type": "text", "text": text}],
        "usage": {"input_tokens": 1, "output_tokens": 1}
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

fn resolver() -> Arc<dyn CredentialResolver> {
    Arc::new(
        StaticCredentials::new()
            .with(MINIMAX_API_KEY, MINIMAX_KEY)
            .with(ANTHROPIC_API_KEY, ANTHROPIC_KEY)
            .with(OPENAI_COMPATIBLE_API_KEY, OPENAI_KEY),
    )
}

/// Build a runtime with all three canonical providers, each at its own mock.
/// `order` is the explicit fallback order used for ambiguous-model resolution.
async fn three_provider_runtime(
    minimax_url: &str,
    anthropic_url: &str,
    openai_url: &str,
    order: Vec<apeireth_core::kernel::CapabilityId>,
) -> Runtime {
    let http = reqwest::Client::builder().build().unwrap();
    let minimax = Arc::new(
        MinimaxProviderPlugin::new(minimax_url, vec![MINIMAX_MODEL.into()], http.clone(), 2_000)
            .unwrap(),
    );
    let anthropic = Arc::new(
        AnthropicProviderPlugin::new(
            anthropic_url,
            vec![ANTHROPIC_MODEL.into()],
            http.clone(),
            2_000,
        )
        .unwrap(),
    );
    let openai = Arc::new(
        OpenAiCompatibleProviderPlugin::new(openai_url, vec![OPENAI_MODEL.into()], http, 2_000)
            .unwrap(),
    );
    Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver())
        .with_plugin(minimax)
        .with_plugin(anthropic)
        .with_plugin(openai)
        .with_fallback_order(order)
        .build()
        .await
        .expect("runtime builds")
}

#[tokio::test]
async fn case_a_minimax_model_routes_to_provider_minimax() {
    let minimax = MockServer::start(openai_body("minimax")).await;
    let anthropic = MockServer::start(anthropic_body("anthropic")).await;
    let openai = MockServer::start(openai_body("openai")).await;
    let runtime = three_provider_runtime(
        minimax.base_url(),
        anthropic.base_url(),
        openai.base_url(),
        vec![
            apeireth_core::kernel::CapabilityId::new("provider.minimax").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.anthropic").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.openai-compatible").unwrap(),
        ],
    )
    .await;

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model(MINIMAX_MODEL))
        .await
        .expect("turn");
    assert_eq!(outcome.served_by.as_str(), "provider.minimax");
    assert!(minimax.served());
    assert!(
        !anthropic.served() && !openai.served(),
        "no cross-consultation"
    );
}

#[tokio::test]
async fn case_b_anthropic_model_routes_to_provider_anthropic() {
    let minimax = MockServer::start(openai_body("m")).await;
    let anthropic = MockServer::start(anthropic_body("anthropic")).await;
    let openai = MockServer::start(openai_body("o")).await;
    let runtime = three_provider_runtime(
        minimax.base_url(),
        anthropic.base_url(),
        openai.base_url(),
        vec![
            apeireth_core::kernel::CapabilityId::new("provider.minimax").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.anthropic").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.openai-compatible").unwrap(),
        ],
    )
    .await;

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model(ANTHROPIC_MODEL))
        .await
        .expect("turn");
    assert_eq!(outcome.served_by.as_str(), "provider.anthropic");
    assert!(anthropic.served());
    assert!(!minimax.served() && !openai.served());
}

#[tokio::test]
async fn case_c_openai_model_routes_to_provider_openai_compatible() {
    let minimax = MockServer::start(openai_body("m")).await;
    let anthropic = MockServer::start(anthropic_body("a")).await;
    let openai = MockServer::start(openai_body("openai")).await;
    let runtime = three_provider_runtime(
        minimax.base_url(),
        anthropic.base_url(),
        openai.base_url(),
        vec![
            apeireth_core::kernel::CapabilityId::new("provider.minimax").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.anthropic").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.openai-compatible").unwrap(),
        ],
    )
    .await;

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model(OPENAI_MODEL))
        .await
        .expect("turn");
    assert_eq!(outcome.served_by.as_str(), "provider.openai-compatible");
    assert!(openai.served());
    assert!(!minimax.served() && !anthropic.served());
}

#[tokio::test]
async fn case_d_unsupported_model_names_all_registered_providers() {
    let minimax = MockServer::start(openai_body("m")).await;
    let anthropic = MockServer::start(anthropic_body("a")).await;
    let openai = MockServer::start(openai_body("o")).await;
    let runtime = three_provider_runtime(
        minimax.base_url(),
        anthropic.base_url(),
        openai.base_url(),
        vec![
            apeireth_core::kernel::CapabilityId::new("provider.minimax").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.anthropic").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.openai-compatible").unwrap(),
        ],
    )
    .await;

    let err = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model("nonexistent-model"))
        .await
        .expect_err("unsupported model");
    match err {
        RuntimeError::NoProvider { model, available } => {
            assert_eq!(model, "nonexistent-model");
            assert!(available.contains("provider.minimax"), "{available}");
            assert!(available.contains("provider.anthropic"), "{available}");
            assert!(
                available.contains("provider.openai-compatible"),
                "{available}"
            );
        }
        other => panic!("expected NoProvider, got {other:?}"),
    }
    assert!(!minimax.served() && !anthropic.served() && !openai.served());
}

#[tokio::test]
async fn case_e_two_providers_same_model_resolves_deterministically_by_fallback_order() {
    // §41: if two providers both support a model (because each was configured
    // with the same model name), the router must resolve deterministically by
    // the explicit fallback order — not by insertion order. Configure minimax
    // and the openai-compatible provider with the SAME model.
    let shared_model = "shared-model";
    let minimax = MockServer::start(openai_body("from minimax")).await;
    let openai = MockServer::start(openai_body("from openai")).await;
    let http = reqwest::Client::builder().build().unwrap();
    let minimax_plugin = Arc::new(
        MinimaxProviderPlugin::new(
            minimax.base_url(),
            vec![shared_model.into()],
            http.clone(),
            2_000,
        )
        .unwrap(),
    );
    let openai_plugin = Arc::new(
        OpenAiCompatibleProviderPlugin::new(
            openai.base_url(),
            vec![shared_model.into()],
            http,
            2_000,
        )
        .unwrap(),
    );
    // Explicit fallback order: openai-compatible first.
    let runtime = Runtime::builder()
        .with_clock(frozen_clock())
        .with_session_store(Arc::new(InMemorySessionStore::new()))
        .with_governance(Arc::new(AllowAll))
        .with_credentials(resolver())
        .with_plugin(minimax_plugin)
        .with_plugin(openai_plugin)
        .with_fallback_order(vec![
            apeireth_core::kernel::CapabilityId::new("provider.openai-compatible").unwrap(),
            apeireth_core::kernel::CapabilityId::new("provider.minimax").unwrap(),
        ])
        .build()
        .await
        .expect("runtime builds");

    let outcome = runtime
        .execute(TurnRequest::new(SessionId::new(), "hi").with_model(shared_model))
        .await
        .expect("turn");
    // The first-listed provider in the explicit fallback order serves.
    assert_eq!(outcome.served_by.as_str(), "provider.openai-compatible");
    assert!(openai.served());
    assert!(
        !minimax.served(),
        "the lower-priority provider is not consulted on success"
    );
}
