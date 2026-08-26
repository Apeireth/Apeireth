//! Integration test for `/tools/web_search/invoke` (R20 阶段 2, 4 真接)
//! Per 蓝图 §2.7 fixture 1.

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

fn app(state: Arc<V2State>) -> axum::Router {
    build_router(state)
}

#[tokio::test]
async fn web_search_invoke_returns_results_with_meta() {
    let state = make_state();
    let body = json!({ "args": { "query": "rust async trait" } });
    let resp = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/web_search/invoke")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "web_search happy 200");
    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(json.get("ok").is_some());
    assert!(json.get("meta").is_some());
    assert_eq!(json["meta"]["tool"], "web_search");
    assert!(json["meta"]["duration_ms"].is_number());
    assert!(json["meta"]["trace_id"].is_string());
}

#[tokio::test]
async fn web_search_unknown_tool_returns_404() {
    let state = make_state();
    let body = json!({ "args": {} });
    let resp = app(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/no_such_tool/invoke")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
