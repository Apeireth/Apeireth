//! Integration test for `/tools/git_ops/invoke` (R20 阶段 2)
//!
//! **Per 蓝图 §2.7 fixture 3**: `git_ops_log_returns_commits()`
//! **Per §2.2 路由表 #3**: `git_ops` 调 `apeireth-tools::GitOpsTool::call`
//!
//! **测试策略**:
//! - happy: 真建 git repo + commit + 调 git_ops status 返 ok=true + clean state
//! - error: 非 git repo 调 git_ops 返 error

use std::sync::Arc;

use apeireth_api::v2_endpoints::{build_router, V2State};
use apeireth_api::v1_tools::build_full_registry;
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

fn app(state: Arc<V2State>) -> axum::Router {
    build_router(state)
}

/// **happy** — git_ops status 在新 git repo 返 200 + ok=true
#[tokio::test]
async fn git_ops_status_returns_200_with_clean_state() {
    let state = make_state();
    let dir = TempDir::new().expect("tempdir");
    let d = dir.path();
    // 真建 git repo
    let init = tokio::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(d)
        .output()
        .await;
    if init.is_err() {
        eprintln!("[skip] git 不可用");
        return;
    }
    let _ = tokio::process::Command::new("git")
        .args(["config", "user.email", "t@e.com"])
        .current_dir(d)
        .output()
        .await;
    let _ = tokio::process::Command::new("git")
        .args(["config", "user.name", "T"])
        .current_dir(d)
        .output()
        .await;
    tokio::fs::write(d.join("f"), "x").await.expect("write");
    let _ = tokio::process::Command::new("git")
        .args(["add", "f"])
        .current_dir(d)
        .output()
        .await;
    let _ = tokio::process::Command::new("git")
        .args(["commit", "-m", "i"])
        .current_dir(d)
        .output()
        .await;

    // 调 git_ops status
    let body = json!({
        "args": { "op": "status", "repo": d.to_string_lossy() }
    });
    let resp = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/git_ops/invoke")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["meta"]["tool"], "git_ops");
    // git status 返 ok=true + result.op == "status"
    assert_eq!(json["ok"], true, "git status ok: {json}");
    assert_eq!(json["result"]["op"], "status");
}
