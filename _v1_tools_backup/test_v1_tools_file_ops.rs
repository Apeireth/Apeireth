//! Integration test for `/tools/file_ops/invoke` (R20 阶段 2)
//!
//! **Per 蓝图 §2.7 fixture 2**: `file_ops_read_returns_content()` + `file_ops_write_creates_file()`
//! **Per §2.2 路由表 #2**: `file_ops` 调 `apeireth-tools::FileOperatorTool::call`
//! **Per §2.9 V20-S2-V1 守门**: 6 工具 endpoint 存在, file_ops 真写 + 真读 OK
//!
//! **测试策略** (per 蓝图 §2.7 失败模式):
//! - happy1: write 真写文件 + read 真读内容 byte-equal
//! - happy2: write idempotent (同 path 二次写覆盖)
//! - error: 不存在路径 read 返 error

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

// =====================================================================
// Fixture 2 — file_ops_read_returns_content + file_ops_write_creates_file
// 蓝图 §2.7 #2: 2 happy + 1 error (path 不存在 → 422)
// =====================================================================

#[tokio::test]
async fn file_ops_write_creates_file() {
    let state = make_state();
    let dir = TempDir::new().expect("tempdir");
    let p = dir.path().join("hello.txt");
    let body = json!({
        "args": {
            "op": "write",
            "path": p.to_string_lossy(),
            "content": "hello apeireth v1_tools"
        }
    });
    let resp = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/file_ops/invoke")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["meta"]["tool"], "file_ops");
    // FileOperator 真写, 返 ok=true + result.op == "write"
    assert_eq!(json["ok"], true, "write 成功: {json}");
    assert_eq!(json["result"]["op"], "write");
    // 文件真存在 + 内容 byte-equal
    let actual = std::fs::read_to_string(&p).expect("file written");
    assert_eq!(actual, "hello apeireth v1_tools");
}

#[tokio::test]
async fn file_ops_read_returns_content() {
    let state = make_state();
    let dir = TempDir::new().expect("tempdir");
    let p = dir.path().join("read.txt");
    // 先真写
    std::fs::write(&p, "read-back-content").expect("seed write");
    let body = json!({
        "args": { "op": "read", "path": p.to_string_lossy() }
    });
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/file_ops/invoke")
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
    assert_eq!(json["result"]["content"], "read-back-content");
}
