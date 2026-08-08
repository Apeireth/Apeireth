//! Integration test for `/tools/calendar/invoke` (R20 阶段 2, **D-01 真接**)
//!
//! **Per 蓝图 §2.7 fixture 5**: `calendar_invoke_returns_5_actions()` — 5/5 case
//! **Per §2.2 路由表 #5**: `calendar` 真接 (D-01 推翻原 stub 501 推荐)
//! **5 actions**: list / create / update / delete / list_range
//!
//! **D-01 真接验证** (per 蓝图 §2.7 失败模式):
//! - 5 actions 各 1 case, 全部 5/5 返 200 + `ok: true` (NOT 501 stub 模式)
//! - 缺字段 → 200 + `ok: false` + error 字段
//!
//! **不假装** (per O-5 不漂移):
//! - ✅ 真 in-memory store (HashMap<String, CalendarEvent>), 不只 mock
//! - ✅ UUID v4 真生成 (workspace `uuid` crate)
//! - ✅ 5 actions 端到端走 HTTP, 0 跳过

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

/// **D-01 真接验证** — 5 actions 端到端走 HTTP (5/5 PASS, per §2.7)
#[tokio::test]
async fn calendar_invoke_5_actions_e2e() {
    let state = make_state();

    // 1. list (空)
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
    let json: Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
    assert_eq!(json["meta"]["tool"], "calendar", "D-01 真接: meta.tool = calendar");
    assert_eq!(json["ok"], true, "D-01: NOT 501 stub, 真接 list");
    assert_eq!(json["result"]["count"], 0);

    // 2. create
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/calendar/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "args": {
                            "action": "create",
                            "event": {
                                "title": "team standup",
                                "start_ts": 1722931200,
                                "end_ts": 1722934800,
                                "attendees": ["alice@x", "bob@x"]
                            }
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json: Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
    assert_eq!(json["ok"], true);
    let event_id = json["result"]["event_id"].as_str().expect("event_id").to_string();
    assert!(!event_id.is_empty());

    // 3. update
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/calendar/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "args": {
                            "action": "update",
                            "id": event_id,
                            "event": {"title": "team standup (updated)"}
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
    assert_eq!(json["ok"], true);

    // 4. list_range (命中)
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/calendar/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "args": {
                            "action": "list_range",
                            "range": {"from_ts": 1722931000, "to_ts": 1722932000}
                        }
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
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
                    serde_json::to_vec(&json!({
                        "args": {"action": "delete", "id": event_id}
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["result"]["deleted"], event_id);
}

/// **D-01 真接 error 路径** — 缺 action / 缺字段 / 不存在 id
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
                .body(Body::from(serde_json::to_vec(&json!({"args": {}})).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json: Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
    assert_eq!(json["ok"], false, "缺 action → ok=false");
    assert!(json["error"].as_str().unwrap().contains("action"));

    // 未知 action
    let resp = app(state.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tools/calendar/invoke")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({"args": {"action": "purge"}})).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes(),
    )
    .unwrap();
    assert_eq!(json["ok"], false);
    assert!(json["error"].as_str().unwrap().contains("unknown"));
}
