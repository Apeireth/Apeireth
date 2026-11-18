//! C2-2 stop 端点的 HTTP 可观察契约。
//!
//! 钉的是:
//! - `POST /v1/sessions/{session_id}/requests/{request_id}/stop` 命中在飞回合 ⇒ 200;
//! - 重复 stop ⇒ 200 且 body 逐字段等于第一次 (幂等回放);
//! - 未知回合 ⇒ 404 `session_not_found`;
//! - 非法 id ⇒ 400 `invalid_request`;
//! - 回合结束后 (guard drop) 重复 stop 仍回放同一摘要。
//!
//! 409 (待批) 分支由 `canonical_stream.rs` 内 `approval_blocks_stop` 纯函数单测
//! 覆盖; 这里不伪造一个真实 PendingApproval 端到端场景。

use std::sync::Arc;

use apeireth_core::kernel::{RequestId, SessionId};
use apeireth_gateway::{build_gateway_state, canonical_router_with_state, GatewayState};
use apeireth_runtime::canonical::turn_cancellation::register;
use apeireth_runtime::canonical::Runtime;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// 测试令牌 (强制态过门用; 形似凭据以便泄漏显形, 非真实秘密)。
const TOKEN: &str = "c2-stop-face-token-0123456789abcdef";

async fn gateway_state() -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut state: GatewayState = build_gateway_state(runtime, None);
    state.token_gate = apeireth_gateway::GatewayTokenGate::Enforced(TOKEN.to_string());
    state
}

fn stop_request(session_id: &str, request_id: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(format!(
            "/v1/sessions/{session_id}/requests/{request_id}/stop"
        ))
        .header("authorization", format!("Bearer {TOKEN}"))
        .body(Body::empty())
        .expect("request")
}

async fn json_body(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json body")
}

#[tokio::test]
async fn stopping_a_live_turn_returns_the_fixed_summary() {
    let state = gateway_state().await;
    let session = SessionId::new();
    let request = RequestId::new();
    let _guard = register(session, request);
    let router = canonical_router_with_state(state);

    let response = router
        .oneshot(stop_request(&session.to_string(), &request.to_string()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["session_id"], session.to_string());
    assert_eq!(body["request_id"], request.to_string());
    assert_eq!(body["state"], "stopped");
    assert_eq!(body["effects_may_be_incomplete"], true);
}

#[tokio::test]
async fn repeating_stop_replays_the_same_body() {
    let state = gateway_state().await;
    let session = SessionId::new();
    let request = RequestId::new();
    let _guard = register(session, request);
    let router = canonical_router_with_state(state);

    let first = router
        .clone()
        .oneshot(stop_request(&session.to_string(), &request.to_string()))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = json_body(first).await;

    let second = router
        .oneshot(stop_request(&session.to_string(), &request.to_string()))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = json_body(second).await;
    assert_eq!(first_body, second_body);
}

#[tokio::test]
async fn unknown_turn_is_a_404_session_not_found() {
    let state = gateway_state().await;
    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stop_request(
            &SessionId::new().to_string(),
            &RequestId::new().to_string(),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = json_body(response).await;
    assert_eq!(body["error"]["code"], "session_not_found");
}

#[tokio::test]
async fn malformed_session_or_request_is_a_400_invalid_request() {
    let state = gateway_state().await;
    let router = canonical_router_with_state(state);
    let valid_session = SessionId::new().to_string();
    let valid_request = RequestId::new().to_string();

    let bad_session = router
        .clone()
        .oneshot(stop_request("not-a-uuid", &valid_request))
        .await
        .unwrap();
    assert_eq!(bad_session.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(bad_session).await["error"]["code"],
        "invalid_request"
    );

    let bad_request = router
        .oneshot(stop_request(&valid_session, "not-a-uuid"))
        .await
        .unwrap();
    assert_eq!(bad_request.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        json_body(bad_request).await["error"]["code"],
        "invalid_request"
    );
}

#[tokio::test]
async fn the_summary_survives_the_end_of_the_turn() {
    let state = gateway_state().await;
    let session = SessionId::new();
    let request = RequestId::new();
    let guard = register(session, request);
    let router = canonical_router_with_state(state);

    let first = router
        .clone()
        .oneshot(stop_request(&session.to_string(), &request.to_string()))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = json_body(first).await;
    drop(guard);

    let after = router
        .oneshot(stop_request(&session.to_string(), &request.to_string()))
        .await
        .unwrap();
    assert_eq!(after.status(), StatusCode::OK);
    assert_eq!(json_body(after).await, first_body);
}
