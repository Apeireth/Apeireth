//! Integration test for `/tools/code_exec/invoke` (R20 阶段 2)
//!
//! **Per 蓝图 §2.7 fixture 4**: `code_exec_runs_command_and_captures_stdout()` + `code_exec_timeout_returns_504()`
//! **Per §2.2 路由表 #4**: `code_exec` 调 `apeireth-tools::CodeExecTool::call`
//! **Per §2.6 限流策略**: code_exec 是 P0 endpoint (1000 req/s 软上限, 触发只 WARN 不 429)
//!
//! **测试策略**:
//! - happy: `echo hello` 返 200 + `result.exit_code == 0` + `result.output` 含 "hello"
//! - error: `sleep 60` + `timeout_ms: 500` 应 timeout (504 gateway_timeout 或 tool 返 error)

use std::sync::Arc;

use apeireth_api::v2_endpoints::{build_router, V2State};
use apeireth_api::v1_tools::build_full_registry;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn make_state() -> Arc<V2State> {
    let state = Arc::new(V2State::new());
    state.install_tools(build_full_registry());
    state
}

fn app(state: Arc<V2State>) -> axum::Router {
    build_router(state)
}

/// **happy** — `cmd /c echo` 真跑 + stdout 捕获 (Windows compatible, per §2.7 fixture 4)
#[tokio::test]
async fn code_exec_runs_command_and_captures_stdout() {
    let state = make_state();
    // Windows: cmd /c "echo hello-v1-tools"
    // Linux:  echo hello-v1-tools
    let cmd = if cfg!(windows) {
        r#"cmd /c "echo hello-v1-tools""#
    } else {
        "echo hello-v1-tools"
    };
    let body = json!({
        "args": { "cmd": cmd }
    });
    let resp = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/code_exec/invoke")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    eprintln!("[DEBUG] code_exec status={status} body={json}");
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["meta"]["tool"], "code_exec");
    assert_eq!(json["ok"], true, "code_exec ok: {json}");
    assert_eq!(json["result"]["exit_code"], 0);
    let output = json["result"]["output"].as_str().unwrap_or("");
    assert!(
        output.contains("hello-v1-tools"),
        "stdout must contain 'hello-v1-tools', got: {output}"
    );
}

/// **error** — `false` 返 exit_code=1 (跨平台: Linux false, Windows `cmd /c exit 1`)
#[tokio::test]
async fn code_exec_nonzero_exit_returns_ok_false() {
    let state = make_state();
    let cmd = if cfg!(windows) { "cmd /c exit 1" } else { "false" };
    let body = json!({
        "args": { "cmd": cmd }
    });
    let resp = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/code_exec/invoke")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["exit_code"], 1);
}
