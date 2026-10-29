//! Deterministic proof that the rewired CLI bootstrap reaches the
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
    build_canonical_runtime_from_env, build_canonical_runtime_from_env_with_observability,
    execute_canonical_cli_turn, CanonicalCliTurn,
};
use apeireth_core::kernel::{CapabilityId, SessionId, TraceId};
use apeireth_governance::{Action, Decision, GovernanceHook, GovernanceRequest};
use serde_json::Value;
use tempfile::tempdir;

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
async fn the_cli_bootstrap_fans_out_guard_dataset_and_trace_audit_observation() {
    let _lock = ENV_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let base_url = mock_vendor(SUCCESS_BODY).await;
    let dataset_path = dir.path().join("guard-dataset.jsonl");
    let cognitive_db = dir.path().join("cognitive.sqlite3");
    let session_db = dir.path().join("sessions.sqlite3");
    let data_dir = dir.path().join("data");

    let _g_key = EnvGuard::set("APEIRETH_API_KEY", Some("sk-fake-cli-fanout"));
    let _g_url = EnvGuard::set("APEIRETH_API_URL", Some(&base_url));
    let _g_models = EnvGuard::set("APEIRETH_API_MODELS", Some("MiniMax-M3"));
    let _g_model = EnvGuard::set("APEIRETH_MODEL", Some("MiniMax-M3"));
    let _g_ant_key = EnvGuard::set("APEIRETH_ANTHROPIC_KEY", None);
    let _g_openai_models = EnvGuard::set("APEIRETH_OPENAI_MODELS", None);
    let _g_local_read = EnvGuard::set("APEIRETH_ENABLE_LOCAL_READ_TOOLS", None);
    let _g_dataset_enabled = EnvGuard::set("APEIRETH_GUARD_DATASET_ENABLED", Some("1"));
    let dataset_path = dataset_path.to_string_lossy().into_owned();
    let _g_dataset_path = EnvGuard::set("APEIRETH_GUARD_DATASET_PATH", Some(&dataset_path));
    let cognitive_db = cognitive_db.to_string_lossy().into_owned();
    let _g_cognitive_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&cognitive_db));
    let session_db = session_db.to_string_lossy().into_owned();
    let _g_session_db = EnvGuard::set("APEIRETH_SESSION_DB", Some(&session_db));
    let data_dir = data_dir.to_string_lossy().into_owned();
    let _g_data_dir = EnvGuard::set("APEIRETH_DATA_DIR", Some(&data_dir));

    let (runtime, observer, _cost_router, _bio_approval) =
        build_canonical_runtime_from_env_with_observability()
            .await
            .expect("the production CLI bootstrap builds with all observers");
    let CanonicalCliTurn::Completed(outcome) =
        execute_canonical_cli_turn(&runtime, "hello", None, None)
            .await
            .expect("the observed CLI turn completes")
    else {
        panic!("expected a completed turn");
    };
    assert_eq!(outcome.served_by.as_str(), "provider.minimax");
    observer.flush().await;

    let dataset = std::fs::read_to_string(&dataset_path).expect("dataset JSONL exists");
    assert!(dataset.contains(r#""record_type":"outcome""#));
    assert!(dataset.contains(r#""execution_outcome":"success""#));
    assert!(
        std::fs::read_to_string(dir.path().join("data/traces.jsonl"))
            .expect("trace archive exists")
            .contains("provider")
    );
    assert!(
        std::fs::read_to_string(dir.path().join("data/daemon-audit.jsonl"))
            .expect("audit archive exists")
            .contains("chat.turn.completed")
    );
}

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
    let cognitive_db = std::env::temp_dir().join(format!(
        "apeireth-cli-cognitive-{}.sqlite3",
        std::process::id()
    ));
    let cognitive_db = cognitive_db.to_string_lossy().into_owned();
    let _g_cognitive_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&cognitive_db));
    // R8-B3: 治理审计账本随数据根走 (temp 根), 测试不写真实 ~/.apeireth。
    let data_dir = std::env::temp_dir().join(format!("apeireth-cli-data-{}", std::process::id()));
    let data_dir = data_dir.to_string_lossy().into_owned();
    let _g_data_dir = EnvGuard::set("APEIRETH_DATA_DIR", Some(&data_dir));
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
    // R8-B2 后治理裁决统一经 GovernanceChain 单入口, verdict 元数据如实标注
    // `governance_chain` (R8-B1 适配器交付映射, 见
    // docs/architecture/r8-b-entry-convergence-design.md §7); 裁决行为不变
    // (tool.shell 仍拒), 四态判定种类与旧入口等价真账见
    // tests/governance_chain_equivalence.rs。
    assert_eq!(governance_verdict.hook, "governance_chain");

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
    let cognitive_db = std::env::temp_dir().join(format!(
        "apeireth-cli-cognitive-anthropic-{}.sqlite3",
        std::process::id()
    ));
    let cognitive_db = cognitive_db.to_string_lossy().into_owned();
    let _g_cognitive_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&cognitive_db));
    // R8-B3: 治理审计账本随数据根走 (temp 根), 测试不写真实 ~/.apeireth。
    let data_dir = std::env::temp_dir().join(format!(
        "apeireth-cli-data-anthropic-{}",
        std::process::id()
    ));
    let data_dir = data_dir.to_string_lossy().into_owned();
    let _g_data_dir = EnvGuard::set("APEIRETH_DATA_DIR", Some(&data_dir));

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
    let cognitive_db = std::env::temp_dir().join(format!(
        "apeireth-cli-cognitive-openai-{}.sqlite3",
        std::process::id()
    ));
    let cognitive_db = cognitive_db.to_string_lossy().into_owned();
    let _g_cognitive_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&cognitive_db));
    // R8-B3: 治理审计账本随数据根走 (temp 根), 测试不写真实 ~/.apeireth。
    let data_dir =
        std::env::temp_dir().join(format!("apeireth-cli-data-openai-{}", std::process::id()));
    let data_dir = data_dir.to_string_lossy().into_owned();
    let _g_data_dir = EnvGuard::set("APEIRETH_DATA_DIR", Some(&data_dir));

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
    let cognitive_db = std::env::temp_dir().join(format!(
        "apeireth-cli-cognitive-omit-{}.sqlite3",
        std::process::id()
    ));
    let cognitive_db = cognitive_db.to_string_lossy().into_owned();
    let _g_cognitive_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&cognitive_db));
    // R8-B3: 治理审计账本随数据根走 (temp 根), 测试不写真实 ~/.apeireth。
    let data_dir =
        std::env::temp_dir().join(format!("apeireth-cli-data-omit-{}", std::process::id()));
    let data_dir = data_dir.to_string_lossy().into_owned();
    let _g_data_dir = EnvGuard::set("APEIRETH_DATA_DIR", Some(&data_dir));

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
    let cognitive_db = std::env::temp_dir().join(format!(
        "apeireth-cli-cognitive-keyless-{}.sqlite3",
        std::process::id()
    ));
    let cognitive_db = cognitive_db.to_string_lossy().into_owned();
    let _g_cognitive_db = EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&cognitive_db));
    // R8-B3: 治理审计账本随数据根走 (temp 根), 测试不写真实 ~/.apeireth。
    let data_dir =
        std::env::temp_dir().join(format!("apeireth-cli-data-keyless-{}", std::process::id()));
    let data_dir = data_dir.to_string_lossy().into_owned();
    let _g_data_dir = EnvGuard::set("APEIRETH_DATA_DIR", Some(&data_dir));

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

/// Real bootstrap, both transports, a frozen approval and the producer's turn
/// seam all retain one managed identity and the caller's durable session.
#[tokio::test]
async fn desktop_entries_share_managed_main_sessions_approval_stream_and_heartbeat() {
    use apeireth_gateway::canonical_router;
    use apeireth_runtime::{ApprovalDecision, ApprovalResolution};
    use axum::{body::Body, http::Request, response::IntoResponse, routing::post, Json, Router};
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tower::ServiceExt;

    let _lock = ENV_LOCK.lock().unwrap();
    let directory = tempdir().unwrap();
    let seen = Arc::new(Mutex::new(Vec::<Value>::new()));
    let capture = seen.clone();
    let service = Router::new().route("/v1/chat/completions", post(move |Json(request): Json<Value>| {
        let capture = capture.clone();
        async move {
            capture.lock().unwrap().push(request.clone());
            if request["stream"] == true {
                let frames = ["first ", "second"].into_iter().map(|part| {
                    let body = serde_json::json!({"id":"managed-stream","model":"managed-fixture",
                        "choices":[{"index":0,"delta":{"content":part},"finish_reason":null}]});
                    format!("data: {body}\n\n")
                }).collect::<String>();
                let end = serde_json::json!({"id":"managed-stream","model":"managed-fixture",
                    "choices":[{"index":0,"delta":{},"finish_reason":"stop"}],
                    "usage":{"prompt_tokens":5,"completion_tokens":4,"total_tokens":9}});
                return ([("content-type", "text/event-stream")], format!("{frames}data: {end}\n\ndata: [DONE]\n\n")).into_response();
            }
            let messages = request["messages"].as_array().unwrap();
            let last_user = messages.iter().rfind(|m| m["role"] == "user").unwrap();
            let needs_tool = last_user["content"] == "run pwd with approval" &&
                !messages.iter().any(|m| m["role"] == "tool" && m["tool_call_id"] == "managed-approval-call");
            let message = if needs_tool {
                serde_json::json!({"role":"assistant","content":null,"tool_calls":[{
                    "id":"managed-approval-call","type":"function","function":{"name":"shell",
                    "arguments":"{\"command\":\"pwd\"}"}}]})
            } else { serde_json::json!({"role":"assistant","content":"managed reply"}) };
            Json(serde_json::json!({"id":"managed-response","model":"managed-fixture",
                "choices":[{"index":0,"message":message,"finish_reason":if needs_tool {"tool_calls"} else {"stop"}}],
                "usage":{"prompt_tokens":5,"completion_tokens":4,"total_tokens":9}})).into_response()
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, service).await.unwrap() });
    let cognitive = directory
        .path()
        .join("cognitive.sqlite3")
        .to_string_lossy()
        .into_owned();
    let sessions = directory
        .path()
        .join("sessions.sqlite3")
        .to_string_lossy()
        .into_owned();
    let data = directory.path().join("data").to_string_lossy().into_owned();
    let _guards = [
        EnvGuard::set("APEIRETH_API_KEY", None),
        EnvGuard::set("APEIRETH_OPENAI_KEY", Some("managed-fixture-key")),
        EnvGuard::set("OPENAI_API_KEY", Some("managed-fixture-key")),
        EnvGuard::set("APEIRETH_OPENAI_URL", Some(&url)),
        EnvGuard::set("APEIRETH_OPENAI_MODELS", Some("managed-fixture")),
        EnvGuard::set("APEIRETH_MODEL", Some("managed-fixture")),
        EnvGuard::set("APEIRETH_ANTHROPIC_KEY", None),
        EnvGuard::set("APEIRETH_COGNITIVE_DB", Some(&cognitive)),
        EnvGuard::set("APEIRETH_SESSION_DB", Some(&sessions)),
        EnvGuard::set("APEIRETH_DATA_DIR", Some(&data)),
        EnvGuard::set("APEIRETH_ENABLE_SHELL", Some("1")),
        EnvGuard::set("APEIRETH_ENABLE_ORGANS", None),
    ];
    let runtime = Arc::new(build_canonical_runtime_from_env().await.unwrap());
    let identity = runtime
        .managed_main()
        .expect("desktop main must be managed");
    assert_eq!(
        identity.owner,
        apeireth_plugin::agents::AgentOwner::Host("main".into())
    );
    assert_eq!(runtime.agent_runtime().unwrap().instances().len(), 1);
    let session = SessionId::new();
    execute_canonical_cli_turn(&runtime, "cli continuity sentinel", None, Some(session))
        .await
        .unwrap();
    let app = canonical_router(runtime.clone());
    let response = app
        .clone()
        .oneshot(
            Request::post("/v1/chat")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"session":session,"input":"gateway continuation"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let stream_session = SessionId::new();
    let response = app.clone().oneshot(Request::post("/v1/chat/completions").header("content-type", "application/json")
        .body(Body::from(serde_json::json!({"session_id":stream_session,"stream":true,"messages":[{"role":"system","content":"static host sentinel"},{"role":"user","content":"stream input"}]}).to_string())).unwrap()).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    assert!(
        body.contains("first ") && body.contains("second") && body.contains("[DONE]"),
        "{body}"
    );
    let deltas = body
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|frame| {
            frame["choices"][0]["delta"]["content"]
                .as_str()
                .map(str::to_owned)
        })
        .collect::<Vec<_>>();
    assert_eq!(deltas, ["first ", "second"]);
    let CanonicalCliTurn::PendingApproval(pending) =
        execute_canonical_cli_turn(&runtime, "run pwd with approval", None, Some(session))
            .await
            .unwrap()
    else {
        panic!("approval must remain resumable")
    };
    assert_eq!(
        runtime.managed_main().unwrap().status,
        apeireth_plugin::agents::AgentStatus::WaitingApproval
    );
    let inbox = runtime.pending_approvals(session).await.unwrap();
    assert_eq!(inbox[0].approval_id, pending.approval_id);
    let response = app
        .oneshot(
            Request::post("/v1/approvals/resolve")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"session":session, "approval":pending.approval_id,
            "decision":"approve"})
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let resolved = apeireth_cli::resolve_canonical_cli_approval(
        &runtime,
        session,
        pending.approval_id,
        ApprovalDecision::Approve,
    )
    .await
    .unwrap();
    assert!(matches!(
        resolved,
        ApprovalResolution::AlreadyResolved { .. }
    ));
    apeireth_cli::care_heartbeat::dispatch_heartbeat_turn(
        &runtime,
        session,
        "heartbeat continuation",
    )
    .await
    .unwrap();
    for id in [session, stream_session] {
        assert_eq!(runtime.tool_dispatcher().agent(id), identity.id);
        let stored = runtime.sessions().load(&id).await.unwrap().unwrap();
        assert!(
            stored.events.iter().any(|entry| matches!(
                entry.event,
                apeireth_runtime::SessionEventKind::ToolEventRecorded { .. }
            )) || id == stream_session
        );
    }
    let history = runtime.sessions().load(&session).await.unwrap().unwrap();
    let user_text = history
        .messages
        .iter()
        .filter(|m| m.role == apeireth_protocol::canonical::MessageRole::User)
        .map(|m| apeireth_protocol::canonical::ContentPart::join_text(&m.content))
        .collect::<Vec<_>>();
    assert_eq!(
        user_text,
        [
            "cli continuity sentinel",
            "gateway continuation",
            "run pwd with approval",
            "heartbeat continuation"
        ]
    );
    let streaming = runtime
        .sessions()
        .load(&stream_session)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        streaming
            .messages
            .iter()
            .filter(|m| m.role == apeireth_protocol::canonical::MessageRole::System)
            .count(),
        1
    );
    assert!(seen
        .lock()
        .unwrap()
        .iter()
        .any(|request| request["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["content"] == "cli continuity sentinel")
            && request["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["content"] == "gateway continuation")));
    let facts = runtime
        .agent_runtime()
        .unwrap()
        .requests(identity.id)
        .unwrap();
    assert_eq!(
        facts.len() as u64,
        runtime.managed_main().unwrap().usage.requests
    );
    assert!(facts
        .iter()
        .all(|fact| fact.agent == identity.id && fact.owner == identity.owner));
    assert_eq!(runtime.managed_main().unwrap().id, identity.id);
    runtime.agent_runtime().unwrap().shutdown().await.unwrap();
    assert!(apeireth_cli::care_heartbeat::dispatch_heartbeat_turn(
        &runtime,
        session,
        "after shutdown"
    )
    .await
    .is_err());
    server.abort();
}
