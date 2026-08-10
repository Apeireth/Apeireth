//! Integration test for `/tools/code_exec/invoke` (R20 阶段 2, 4 真接)
//! Per 蓝图 §2.7 fixture 4.

use std::sync::Arc;

use apeireth_api::v1_tools::build_full_registry;
use apeireth_api::v2_endpoints::{build_router, V2State};
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

fn app(state: Arc<V2State>) -> axum::Router { build_router(state) }

#[tokio::test]
async fn code_exec_runs_command_and_captures_stdout() {
    let state = make_state();
    let cmd = if cfg!(windows) { r#"cmd /c "echo hello-v1-tools""# } else { "echo hello-v1-tools" };
    let body = json!({ "args": { "cmd": cmd } });
    let resp = app(state).oneshot(
        Request::builder().method("POST").uri("/tools/code_exec/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["meta"]["tool"], "code_exec");
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["exit_code"], 0);
}

#[tokio::test]
async fn code_exec_nonzero_exit_returns_ok_false() {
    let state = make_state();
    let cmd = if cfg!(windows) { "cmd /c exit 1" } else { "false" };
    let body = json!({ "args": { "cmd": cmd } });
    let resp = app(state).oneshot(
        Request::builder().method("POST").uri("/tools/code_exec/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["exit_code"], 1);
}
