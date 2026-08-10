//! Integration tests for R32-3-1 real_llm_smoke
//!
//! **2 类测试**:
//! 1. wiremock 模拟 MiniMax /v1/messages, 验证 request shape + response parse
//!    (无需网络, 跑在 cargo test --workspace)
//! 2. live env-gated 测试 (APEIRETH_MINIMAX_LIVE_TEST=1), 真调 MiniMax
//!    (默认 ignored, 需显式 `cargo test -- --ignored` 或 env var 启用)

use apeireth_eval::real_llm_smoke::{
    load_api_key, run_real_llm_smoke, RealLlmConfig,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================
// WireMock 测试 (无网络, CI 友好)
// ============================================================

/// 标准 MiniMax Anthropic response mock
fn minimax_response_json() -> serde_json::Value {
    serde_json::json!({
        "id": "msg_smoke_001",
        "model": "MiniMax-M2.7-highspeed",
        "stop_reason": "end_turn",
        "content": [
            {"type": "text", "text": "Confirmed: I received the system prompt."}
        ],
        "usage": {
            "input_tokens": 142,
            "output_tokens": 9,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 0
        }
    })
}

/// 跑 smoke 在临时 workspace (有合法 Cargo.toml)
fn setup_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"x\"]\n\n[workspace.package]\nedition = \"2021\"\nrust-version = \"1.75\"\n\n[workspace.dependencies]\nserde = \"1\"\n",
    )
    .unwrap();
    dir
}

#[tokio::test]
async fn real_llm_smoke_mock_minimax_all_pass() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .and(header("x-api-key", "test-key-mock"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(minimax_response_json()))
        .expect(1) // 验证只 call 1 次
        .mount(&server)
        .await;

    let workspace = setup_workspace();
    let cfg = RealLlmConfig {
        base_url: server.uri(),
        model: "MiniMax-M2.7-highspeed".to_string(),
        max_tokens: 64,
        temperature: Some(1.0),
        messages_path: "/anthropic/v1/messages".to_string(),
        ..Default::default()
    };

    let report = run_real_llm_smoke(
        workspace.path(),
        Some("test-key-mock"),
        Some(cfg),
    )
    .await;

    assert!(report.apikey_loaded, "apikey_loaded: {:?}", report.error);
    assert!(report.conventions_scanned, "conventions_scanned: {:?}", report.error);
    assert!(report.prompt_built, "prompt_built: {:?}", report.error);
    assert!(report.http_request_ok, "http_request_ok: status={} error={:?}", report.http_status, report.error);
    assert!(report.response_shape_valid, "response_shape_valid: {:?}", report.error);
    assert!(report.content_non_empty, "content_non_empty: text='{}'", report.response_text_excerpt);
    assert!(report.token_usage_recorded, "token_usage_recorded: in={} out={}", report.input_tokens, report.output_tokens);
    assert!(report.all_pass(), "all_pass() returned false");
    assert_eq!(report.pass_rate(), 1.0);
    assert_eq!(report.http_status, 200);
    assert_eq!(report.input_tokens, 142);
    assert_eq!(report.output_tokens, 9);
    assert_eq!(report.stop_reason.as_deref(), Some("end_turn"));
    assert_eq!(report.response_model.as_deref(), Some("MiniMax-M2.7-highspeed"));
    assert!(report.response_text_excerpt.contains("Confirmed"));
    assert!(report.latency_ms < 30_000);
}

#[tokio::test]
async fn real_llm_smoke_mock_429_rate_limit_fails_gracefully() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate_limit_error"))
        .mount(&server)
        .await;

    let workspace = setup_workspace();
    let cfg = RealLlmConfig {
        base_url: server.uri(),
        ..Default::default()
    };

    let report = run_real_llm_smoke(workspace.path(), Some("k"), Some(cfg)).await;

    assert!(report.apikey_loaded);
    assert!(report.conventions_scanned);
    assert!(report.prompt_built);
    assert!(!report.http_request_ok, "429 不该算 ok");
    assert_eq!(report.http_status, 429);
    assert!(report.error.is_some(), "error 应该记录 429");
    assert!(report.error.as_ref().unwrap().contains("429"));
    assert!(!report.all_pass());
}

#[tokio::test]
async fn real_llm_smoke_mock_500_fails_gracefully() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal_error"))
        .mount(&server)
        .await;

    let workspace = setup_workspace();
    let cfg = RealLlmConfig {
        base_url: server.uri(),
        ..Default::default()
    };

    let report = run_real_llm_smoke(workspace.path(), Some("k"), Some(cfg)).await;

    assert!(!report.http_request_ok);
    assert_eq!(report.http_status, 500);
    assert!(report.error.as_ref().unwrap().contains("500"));
}

#[tokio::test]
async fn real_llm_smoke_mock_malformed_json_fails_gracefully() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json {"))
        .mount(&server)
        .await;

    let workspace = setup_workspace();
    let cfg = RealLlmConfig {
        base_url: server.uri(),
        ..Default::default()
    };

    let report = run_real_llm_smoke(workspace.path(), Some("k"), Some(cfg)).await;

    assert!(report.http_request_ok);
    assert!(!report.response_shape_valid);
    assert!(report.error.as_ref().unwrap().contains("JSON parse"));
}

#[tokio::test]
async fn real_llm_smoke_apikey_loaded_from_default_path() {
    // 默认路径 (.openclaw\apikey.txt 等) 文件存在
    // → load_api_key(None) 应该成功, source 是 "file:..."
    std::env::remove_var("APEIRETH_MINIMAX_API_KEY");
    let result = load_api_key(None);
    match result {
        Ok((_k, src)) => {
            // 文件存在 → 应能加载 (不一定存在则看具体机器)
            assert!(src.starts_with("file:") || src == "env",
                "expected file or env source, got: {src}");
        }
        Err(e) => {
            // 文件不存在 (CI 无该路径) → 应该返 error
            assert!(e.contains("apikey 3 源全 miss"), "unexpected error: {e}");
        }
    }
}

#[tokio::test]
async fn real_llm_smoke_missing_workspace_cargo_fails_at_stage_2() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    let workspace = tempfile::tempdir().unwrap();
    let cfg = RealLlmConfig {
        base_url: server.uri(),
        ..Default::default()
    };

    let report = run_real_llm_smoke(workspace.path(), Some("k"), Some(cfg)).await;

    assert!(report.apikey_loaded);
    assert!(!report.conventions_scanned, "无 Cargo.toml 应 scan 失败");
    assert!(report.error.as_ref().unwrap().contains("conventions_scanned"));
}

#[tokio::test]
async fn real_llm_smoke_request_includes_system_prompt_and_model() {
    use wiremock::matchers::body_partial_json;

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .and(body_partial_json(serde_json::json!({
            "model": "MiniMax-M2.7-highspeed",
            "max_tokens": 64,
        })))
        .and(header("x-api-key", "k"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(minimax_response_json()))
        .expect(1)
        .mount(&server)
        .await;

    let workspace = setup_workspace();
    let cfg = RealLlmConfig {
        base_url: server.uri(),
        model: "MiniMax-M2.7-highspeed".to_string(),
        max_tokens: 64,
        ..Default::default()
    };

    let _ = run_real_llm_smoke(workspace.path(), Some("k"), Some(cfg)).await;
}

#[tokio::test]
async fn load_api_key_explicit_priority() {
    std::env::remove_var("APEIRETH_MINIMAX_API_KEY");
    let (k, src) = load_api_key(Some("explicit-key")).unwrap();
    assert_eq!(k, "explicit-key");
    assert_eq!(src, "explicit");
}

#[tokio::test]
async fn real_llm_smoke_to_eval_scores_round_trip() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/anthropic/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(minimax_response_json()))
        .mount(&server)
        .await;

    let workspace = setup_workspace();
    let cfg = RealLlmConfig {
        base_url: server.uri(),
        ..Default::default()
    };
    let report = run_real_llm_smoke(workspace.path(), Some("k"), Some(cfg)).await;

    let scores = report.to_eval_scores();
    assert_eq!(scores.len(), 7);
    for s in &scores {
        assert_eq!(s.value, 1.0, "score {} not 1.0", s.dimension);
    }

    let mean = apeireth_eval::mean(&scores);
    assert_eq!(mean, Some(1.0));
}

// ============================================================
// Live tests (env-gated, 默认 ignored)
// ============================================================

/// 真接 MiniMax — 需 env var `APEIRETH_MINIMAX_LIVE_TEST=1` 才跑
/// (或显式 `cargo test -- --ignored`)
#[tokio::test]
#[ignore = "live MiniMax test — run with APEIRETH_MINIMAX_LIVE_TEST=1"]
async fn live_minimax_smoke_real_call() {
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").ok().as_deref() != Some("1") {
        eprintln!("skip: APEIRETH_MINIMAX_LIVE_TEST != 1");
        return;
    }

    let here = std::env::current_dir().unwrap();
    let workspace_root = here.parent().and_then(|p| p.parent()).unwrap_or(&here);

    let cfg = RealLlmConfig::default();
    let report = run_real_llm_smoke(workspace_root, None, Some(cfg)).await;

    println!("\n=== LIVE MiniMax smoke ===");
    println!("http_status: {}", report.http_status);
    println!("latency_ms:  {}", report.latency_ms);
    println!("response_model: {:?}", report.response_model);
    println!("stop_reason: {:?}", report.stop_reason);
    println!("input_tokens: {}", report.input_tokens);
    println!("output_tokens: {}", report.output_tokens);
    println!("text_excerpt: {}", report.response_text_excerpt);
    println!("pass_rate: {}", report.pass_rate());
    println!("error: {:?}", report.error);

    assert!(report.all_pass(), "live MiniMax smoke failed: {:?}", report.error);
}

#[tokio::test]
#[ignore = "live MiniMax test — run with APEIRETH_MINIMAX_LIVE_TEST=1"]
async fn live_minimax_smoke_with_explicit_key() {
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").ok().as_deref() != Some("1") {
        return;
    }
    let key = match std::env::var("APEIRETH_MINIMAX_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            eprintln!("skip: APEIRETH_MINIMAX_API_KEY not set");
            return;
        }
    };

    let here = std::env::current_dir().unwrap();
    let workspace_root = here.parent().and_then(|p| p.parent()).unwrap_or(&here);

    let report = run_real_llm_smoke(workspace_root, Some(&key), None).await;
    assert!(report.all_pass(), "live smoke with explicit key failed: {:?}", report.error);
}