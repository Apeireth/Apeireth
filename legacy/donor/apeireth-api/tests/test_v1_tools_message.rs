//! Integration test for `/tools/message/invoke` (R20 阶段 2, **D-01 真接**)
//! Per 蓝图 §2.7 fixture 6.

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
async fn message_invoke_3_actions_e2e() {
    let state = make_state();

    // 1. send to alice
    let resp = app(state.clone()).oneshot(
        Request::builder().method("POST").uri("/tools/message/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&json!({
                "args": {"action": "send", "target": "alice", "sender": "bob", "payload": {"text": "hi alice"}}
            })).unwrap()))
            .unwrap(),
    ).await.unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["meta"]["tool"], "message", "D-01 真接");
    assert_eq!(json["ok"], true, "D-01: NOT 501 stub");
    assert!(json["result"]["message_id"].is_string());

    // 2. send to bob
    let resp = app(state.clone()).oneshot(
        Request::builder().method("POST").uri("/tools/message/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&json!({
                "args": {"action": "send", "target": "bob", "sender": "alice", "payload": {"text": "hi bob"}}
            })).unwrap()))
            .unwrap(),
    ).await.unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], true);

    // 3. list
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/message/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"args": {"action": "list"}})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["count"], 2);

    // 4. subscribe alice (drain)
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/message/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(
                        &json!({"args": {"action": "subscribe", "target": "alice"}}),
                    )
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["count"], 1, "subscribe alice 应拉 1 条");
    assert_eq!(json["result"]["messages"][0]["target"], "alice");
    assert_eq!(json["result"]["messages"][0]["payload"]["text"], "hi alice");

    // 5. list again (剩 1 条 = bob)
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/message/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"args": {"action": "list"}})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["result"]["count"], 1, "alice drain 后剩 1 条 (bob)");
    assert_eq!(json["result"]["messages"][0]["target"], "bob");
}

#[tokio::test]
async fn message_invoke_error_paths() {
    let state = make_state();
    // 缺 action
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/message/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"args": {}})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], false);
    assert!(json["error"].as_str().unwrap().contains("action"));

    // send 缺 target
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/message/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"args": {"action": "send", "payload": "x"}}))
                        .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], false);
    assert!(json["error"].as_str().unwrap().contains("target"));
}
