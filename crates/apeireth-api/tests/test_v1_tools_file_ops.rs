//! Integration test for `/tools/file_ops/invoke` (R20 阶段 2, 4 真接)
//! Per 蓝图 §2.7 fixture 2.

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
async fn file_ops_write_creates_file() {
    let state = make_state();
    let dir = TempDir::new().expect("tempdir");
    let p = dir.path().join("hello.txt");
    let body = json!({
        "args": { "op": "write", "path": p.to_string_lossy(), "content": "hello v1" }
    });
    let resp = app(state).oneshot(
        Request::builder().method("POST").uri("/tools/file_ops/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["meta"]["tool"], "file_ops");
    assert_eq!(json["ok"], true);
    let actual = std::fs::read_to_string(&p).expect("file written");
    assert_eq!(actual, "hello v1");
}

#[tokio::test]
async fn file_ops_read_returns_content() {
    let state = make_state();
    let dir = TempDir::new().expect("tempdir");
    let p = dir.path().join("r.txt");
    std::fs::write(&p, "read-back").expect("seed");
    let body = json!({ "args": { "op": "read", "path": p.to_string_lossy() } });
    let resp = app(state).oneshot(
        Request::builder().method("POST").uri("/tools/file_ops/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["content"], "read-back");
}
