//! Integration test for `/tools/git_ops/invoke` (R20 阶段 2, 4 真接)
//! Per 蓝图 §2.7 fixture 3.

use std::sync::Arc;

use apeireth_api::v1_tools::build_full_registry;
use apeireth_api::v2_endpoints::{build_router, V2State};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tempfile::TempDir;
use tower::ServiceExt;

fn make_state() -> Arc<V2State> {
    let state = Arc::new(V2State::new());
    state.install_tools(build_full_registry());
    state
}

fn app(state: Arc<V2State>) -> axum::Router { build_router(state) }

#[tokio::test]
async fn git_ops_status_returns_200_with_clean_state() {
    let state = make_state();
    let dir = TempDir::new().expect("tempdir");
    let d = dir.path();
    let init = tokio::process::Command::new("git")
        .args(["init", "--initial-branch=main"]).current_dir(d).output().await;
    if init.is_err() { eprintln!("[skip] git unavailable"); return; }
    let _ = tokio::process::Command::new("git")
        .args(["config", "user.email", "t@e.com"]).current_dir(d).output().await;
    let _ = tokio::process::Command::new("git")
        .args(["config", "user.name", "T"]).current_dir(d).output().await;
    tokio::fs::write(d.join("f"), "x").await.expect("write");
    let _ = tokio::process::Command::new("git").args(["add", "f"]).current_dir(d).output().await;
    let _ = tokio::process::Command::new("git").args(["commit", "-m", "i"]).current_dir(d).output().await;

    let body = json!({ "args": { "op": "status", "repo": d.to_string_lossy() } });
    let resp = app(state).oneshot(
        Request::builder().method("POST").uri("/tools/git_ops/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["meta"]["tool"], "git_ops");
    assert_eq!(json["ok"], true);
}
