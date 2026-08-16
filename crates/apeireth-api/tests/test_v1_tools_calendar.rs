//! Integration test for `/tools/calendar/invoke` (R20 阶段 2, **D-01 真接**)
//! Per 蓝图 §2.7 fixture 5.

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
async fn calendar_invoke_5_actions_e2e() {
    let state = make_state();
    // 1. list empty
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/calendar/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"args": {"action": "list"}})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["meta"]["tool"], "calendar", "D-01 真接");
    assert_eq!(json["ok"], true, "D-01: NOT 501 stub");
    assert_eq!(json["result"]["count"], 0);

    // 2. create
    let resp = app(state.clone()).oneshot(
        Request::builder().method("POST").uri("/tools/calendar/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&json!({
                "args": {"action": "create", "event": {"title": "standup", "start_ts": 1722931200, "end_ts": 1722934800, "attendees": ["a@x"]}}
            })).unwrap()))
            .unwrap(),
    ).await.unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], true);
    let event_id = json["result"]["event_id"]
        .as_str()
        .expect("event_id")
        .to_string();

    // 3. update
    let resp = app(state.clone()).oneshot(
        Request::builder().method("POST").uri("/tools/calendar/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&json!({"args": {"action": "update", "id": event_id, "event": {"title": "updated"}}})).unwrap()))
            .unwrap(),
    ).await.unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], true);

    // 4. list_range
    let resp = app(state.clone()).oneshot(
        Request::builder().method("POST").uri("/tools/calendar/invoke")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&json!({"args": {"action": "list_range", "range": {"from_ts": 1722931000, "to_ts": 1722932000}}})).unwrap()))
            .unwrap(),
    ).await.unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["count"], 1);

    // 5. delete
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/calendar/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"args": {"action": "delete", "id": event_id}}))
                        .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value =
        serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["deleted"], event_id);
}

#[tokio::test]
async fn calendar_invoke_error_paths() {
    let state = make_state();
    // 缺 action
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/calendar/invoke")
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
}
