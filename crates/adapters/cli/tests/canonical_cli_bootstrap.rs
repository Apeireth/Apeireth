//! Deterministic proof that the rewired CLI bootstrap reaches the migrated
//! canonical provider.
//!
//! `build_canonical_runtime_from_env` is the real production bootstrap used by
//! both `dispatch_canonical_chat` and `dispatch_gateway_serve`. This test points
//! it at a mock vendor HTTP server (via `APEIRETH_API_URL`) and proves the
//! assembled runtime serves a turn through `provider.minimax` — the canonical
//! capability, not the `LegacyLlmCapability` bridge.
//!
//! No Internet, no real key. Env-mutating tests are serialized because
//! `std::env` is process-global.

use std::sync::Mutex;

use apeireth_cli::{
    build_canonical_runtime_from_env, execute_canonical_cli_turn, CanonicalCliTurn,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use serde_json::Value;

/// Serializes env-mutating tests (std::env is process-global).
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Restores an env var on drop.
struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let prev = std::env::var(key).ok();
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        Self { key, prev }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.prev {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

/// A minimal one-shot mock vendor HTTP server returning a canned 200 body.
async fn mock_vendor(body: &'static str) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
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
    });
    format!("http://{addr}")
}

const SUCCESS_BODY: &str = r#"{
    "id": "chatcmpl-cli-bootstrap",
    "model": "MiniMax-M3",
    "choices": [{"index": 0, "message": {"role": "assistant", "content": "hello from canonical cli"}, "finish_reason": "stop"}],
    "usage": {"prompt_tokens": 5, "completion_tokens": 4, "total_tokens": 9}
}"#;

#[tokio::test]
async fn the_cli_bootstrap_registers_both_canonical_providers_and_serves_minimax() {
    let _lock = ENV_LOCK.lock().unwrap();
    let base_url = mock_vendor(SUCCESS_BODY).await;

    let _g_key = EnvGuard::set("APEIRETH_API_KEY", Some("sk-fake-cli-bootstrap"));
    let _g_url = EnvGuard::set("APEIRETH_API_URL", Some(&base_url));
    let _g_models = EnvGuard::set("APEIRETH_API_MODELS", Some("MiniMax-M3"));
    let _g_model = EnvGuard::set("APEIRETH_MODEL", Some("MiniMax-M3"));
    let _g_local_read = EnvGuard::set("APEIRETH_ENABLE_LOCAL_READ_TOOLS", None);
    let session_db = std::env::temp_dir().join(format!(
        "apeireth-cli-session-{}.sqlite3",
        std::process::id()
    ));
    let session_db = session_db.to_string_lossy().into_owned();
    let _g_session_db = EnvGuard::set("APEIRETH_SESSION_DB", Some(&session_db));
    // Anthropic key absent — its provider is still registered (keyless), but
    // would fail explicitly if routed to. Minimax serves this turn.
    let _g_ant_key = EnvGuard::set("APEIRETH_ANTHROPIC_KEY", None);

    let runtime = build_canonical_runtime_from_env()
        .await
        .expect("the rewired bootstrap builds a runtime");

    // Both canonical providers are registered — no compat.* bridge.
    let ids: Vec<String> = runtime
        .providers()
        .provider_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    assert!(ids.contains(&"provider.minimax".to_string()), "{ids:?}");
    assert!(ids.contains(&"provider.anthropic".to_string()), "{ids:?}");
    assert!(
        !ids.iter().any(|id| id.starts_with("compat.")),
        "no compatibility bridge provider: {ids:?}"
    );

    let shell = CapabilityId::new("tool.shell").unwrap();
    let arguments = Value::Null;
    let governance_verdict = runtime
        .governance()
        .evaluate_verbose(&GovernanceRequest::new(
            Action::CapabilityDispatch {
                capability: &shell,
                arguments: &arguments,
            },
            SessionId::new(),
            TraceId::new(),
            1,
        ))
        .await;
    assert!(matches!(governance_verdict.decision, Decision::Deny { .. }));
    assert_eq!(governance_verdict.hook, "permission_governance");

    let CanonicalCliTurn::Completed(outcome) =
        execute_canonical_cli_turn(&runtime, "hi", None, None)
            .await
            .expect("the turn completes")
    else {
        panic!("expected a completed turn");
    };
    assert_eq!(outcome.served_by.as_str(), "provider.minimax");
    assert_eq!(outcome.text, "hello from canonical cli");
    assert_eq!(outcome.rounds, 1);

    let mut names: Vec<String> = runtime
        .tool_declarations()
        .into_iter()
        .map(|tool| tool.name)
        .collect();
    names.sort();
    for required in ["filesystem", "search", "repo"] {
        let count = names
            .iter()
            .filter(|name| name.as_str() == required)
            .count();
        assert_eq!(count, 1, "{required} must be unique, got {names:?}");
    }
    for absent in ["shell", "fetch"] {
        assert!(
            !names.iter().any(|name| name.as_str() == absent),
            "{absent} must not be registered in production CLI by default: {names:?}"
        );
    }
}

#[tokio::test]
async fn the_cli_bootstrap_routes_an_anthropic_model_to_provider_anthropic() {
    let _lock = ENV_LOCK.lock().unwrap();
    // A mock speaking the Anthropic Messages API shape.
    let anthropic_body = r#"{
        "id": "msg_cli_anthropic",
        "model": "claude-sonnet-4-5",
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": "hello from anthropic cli"}],
        "usage": {"input_tokens": 3, "output_tokens": 2}
    }"#;
    let anthropic_url = mock_vendor(anthropic_body).await;

    let _g_minimax_key = EnvGuard::set("APEIRETH_API_KEY", None);
    let _g_ant_key = EnvGuard::set("APEIRETH_ANTHROPIC_KEY", Some("sk-ant-cli-bootstrap"));
    let _g_ant_url = EnvGuard::set("APEIRETH_ANTHROPIC_URL", Some(&anthropic_url));
    let _g_ant_models = EnvGuard::set("APEIRETH_ANTHROPIC_MODELS", Some("claude-sonnet-4-5"));
    let _g_model = EnvGuard::set("APEIRETH_MODEL", Some("claude-sonnet-4-5"));
    let session_db = std::env::temp_dir().join(format!(
        "apeireth-cli-session-anthropic-{}.sqlite3",
        std::process::id()
    ));
    let session_db = session_db.to_string_lossy().into_owned();
    let _g_session_db = EnvGuard::set("APEIRETH_SESSION_DB", Some(&session_db));

    let runtime = build_canonical_runtime_from_env()
        .await
        .expect("bootstrap builds a runtime with both providers");

    let CanonicalCliTurn::Completed(outcome) =
        execute_canonical_cli_turn(&runtime, "hi", None, None)
            .await
            .expect("the turn completes")
    else {
        panic!("expected a completed turn");
    };
    assert_eq!(outcome.served_by.as_str(), "provider.anthropic");
    assert_eq!(outcome.text, "hello from anthropic cli");
}

#[tokio::test]
async fn the_cli_bootstrap_routes_an_openai_model_to_provider_openai_compatible() {
    let _lock = ENV_LOCK.lock().unwrap();
    // A mock speaking the OpenAI Chat Completions shape.
    let openai_body = r#"{
        "id": "chatcmpl_cli_openai",
        "model": "gpt-4o-mini",
        "choices": [{"index": 0, "message": {"role": "assistant", "content": "hello from openai cli"}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 3, "completion_tokens": 2, "total_tokens": 5}
    }"#;
    let openai_url = mock_vendor(openai_body).await;

    let _g_minimax_key = EnvGuard::set("APEIRETH_API_KEY", None);
    let _g_ant_key = EnvGuard::set("APEIRETH_ANTHROPIC_KEY", None);
    let _g_openai_key = EnvGuard::set("OPENAI_API_KEY", Some("sk-openai-cli-bootstrap"));
    let _g_openai_url = EnvGuard::set("APEIRETH_OPENAI_URL", Some(&openai_url));
    let _g_openai_models = EnvGuard::set("APEIRETH_OPENAI_MODELS", Some("gpt-4o-mini"));
    let _g_model = EnvGuard::set("APEIRETH_MODEL", Some("gpt-4o-mini"));
    let session_db = std::env::temp_dir().join(format!(
        "apeireth-cli-session-openai-{}.sqlite3",
        std::process::id()
    ));
    let session_db = session_db.to_string_lossy().into_owned();
    let _g_session_db = EnvGuard::set("APEIRETH_SESSION_DB", Some(&session_db));

    let runtime = build_canonical_runtime_from_env()
        .await
        .expect("bootstrap builds a runtime with the openai-compatible provider");

    // The openai-compatible provider is registered only when models are
    // configured; here they are, so it participates.
    let ids: Vec<String> = runtime
        .providers()
        .provider_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    assert!(
        ids.contains(&"provider.openai-compatible".to_string()),
        "{ids:?}"
    );

    let CanonicalCliTurn::Completed(outcome) =
        execute_canonical_cli_turn(&runtime, "hi", None, None)
            .await
            .expect("the turn completes")
    else {
        panic!("expected a completed turn");
    };
    assert_eq!(outcome.served_by.as_str(), "provider.openai-compatible");
    assert_eq!(outcome.text, "hello from openai cli");
}

#[tokio::test]
async fn the_cli_bootstrap_omits_openai_compatible_when_unconfigured() {
    let _lock = ENV_LOCK.lock().unwrap();
    // No APEIRETH_OPENAI_MODELS: the generic provider must NOT be registered
    // (it has no hardcoded model default — §21/§38).
    let _g_minimax_key = EnvGuard::set("APEIRETH_API_KEY", Some("sk-fake"));
    let _g_openai_models = EnvGuard::set("APEIRETH_OPENAI_MODELS", None);
    let session_db = std::env::temp_dir().join(format!(
        "apeireth-cli-session-omit-{}.sqlite3",
        std::process::id()
    ));
    let session_db = session_db.to_string_lossy().into_owned();
    let _g_session_db = EnvGuard::set("APEIRETH_SESSION_DB", Some(&session_db));

    let runtime = build_canonical_runtime_from_env()
        .await
        .expect("bootstrap builds without the generic provider");

    let ids: Vec<String> = runtime
        .providers()
        .provider_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    assert!(
        !ids.contains(&"provider.openai-compatible".to_string()),
        "unconfigured generic provider must not register: {ids:?}"
    );
    // The minimax + anthropic providers are still present.
    assert!(ids.contains(&"provider.minimax".to_string()));
    assert!(ids.contains(&"provider.anthropic".to_string()));
}

#[tokio::test]
async fn the_cli_bootstrap_boots_keyless_and_fails_explicitly_on_execute() {
    let _lock = ENV_LOCK.lock().unwrap();
    let base_url = mock_vendor(SUCCESS_BODY).await;

    // Both keys absent: the runtime must still build (keyless boot), and
    // executing against the default minimax model must fail explicitly — never
    // a silent success or mock fallback.
    let _g_key = EnvGuard::set("APEIRETH_API_KEY", None);
    let _g_ant_key = EnvGuard::set("APEIRETH_ANTHROPIC_KEY", None);
    let _g_url = EnvGuard::set("APEIRETH_API_URL", Some(&base_url));
    let _g_models = EnvGuard::set("APEIRETH_API_MODELS", Some("MiniMax-M3"));
    let _g_model = EnvGuard::set("APEIRETH_MODEL", Some("MiniMax-M3"));
    let session_db = std::env::temp_dir().join(format!(
        "apeireth-cli-session-keyless-{}.sqlite3",
        std::process::id()
    ));
    let session_db = session_db.to_string_lossy().into_owned();
    let _g_session_db = EnvGuard::set("APEIRETH_SESSION_DB", Some(&session_db));

    let runtime = build_canonical_runtime_from_env()
        .await
        .expect("keyless boot must succeed");

    // Both providers are registered even without keys...
    let ids: Vec<String> = runtime
        .providers()
        .provider_ids()
        .into_iter()
        .map(|id| id.to_string())
        .collect();
    assert!(ids.contains(&"provider.minimax".to_string()));
    assert!(ids.contains(&"provider.anthropic".to_string()));

    // ...but executing a turn against the default model fails explicitly with
    // the credential problem.
    let err = execute_canonical_cli_turn(&runtime, "hi", None, None)
        .await
        .expect_err("a missing key must fail the turn");
    assert!(
        err.contains("api_key") || err.contains("auth"),
        "the failure must name the credential problem: {err}"
    );
}
