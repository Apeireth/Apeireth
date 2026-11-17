//! C1 规范流的**后端原生流面** (后端合同 C 批 · C1 段 · 裁决 15/17)。
//!
//! 钉的是 HTTP/SSE 面的可观察契约:
//! - 路径 `GET /v1/sessions/{session_id}/requests/{request_id}/stream`;
//! - 帧带 `id: <sequence>` (客户端可直接把它当下一次重连的 `Last-Event-ID`);
//! - 续发锚点: `Last-Event-ID` 头**优先**于 `?after_sequence=`;
//! - 落后于环形窗口 ⇒ 先发一条 `resync_required: true` 的**事实帧**, 再关闭通道;
//! - 流不存在 ⇒ 404 `session_not_found`; 锚点非法 ⇒ 400 `invalid_request`。

use std::sync::Arc;

use apeireth_gateway::{build_gateway_state, canonical_router_with_state, GatewayState};
use apeireth_protocol::canonical_stream::{C1FinalFrame, C1StreamState};
use apeireth_runtime::canonical::canonical_stream::{global_registry, STREAM_BUFFER_CAPACITY};
use apeireth_runtime::canonical::Runtime;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// 测试令牌 (强制态过门用; 形似凭据以便泄漏显形, 非真实秘密)。
const TOKEN: &str = "c1-stream-face-token-0123456789abcdef";

/// 一帧 SSE (按 `event:` / `data:` / `id:` 三行切分)。
#[derive(Debug, Clone, PartialEq, Eq)]
struct SseFrame {
    event: String,
    data: String,
    id: Option<String>,
}

/// 造一个带令牌门的网关状态。
async fn gateway_state() -> GatewayState {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let mut state: GatewayState = build_gateway_state(runtime, None);
    state.token_gate = apeireth_gateway::GatewayTokenGate::Enforced(TOKEN.to_string());
    state
}

/// 造一条流面请求 (可带查询串与 `Last-Event-ID` 头)。
fn stream_request(
    session_id: &str,
    request_id: &str,
    query: &str,
    last_event_id: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .uri(format!(
            "/v1/sessions/{session_id}/requests/{request_id}/stream{query}"
        ))
        .header("authorization", format!("Bearer {TOKEN}"));
    if let Some(anchor) = last_event_id {
        builder = builder.header("last-event-id", anchor);
    }
    builder.body(Body::empty()).expect("request")
}

/// 读 SSE 帧直到累计文本里出现 `marker`。
async fn collect_sse_until(response: axum::response::Response, marker: &str) -> String {
    let mut stream = http_body_util::BodyStream::new(response.into_body());
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let next =
            tokio::time::timeout_at(deadline, tokio_stream::StreamExt::next(&mut stream)).await;
        match next {
            Ok(Some(Ok(frame))) => {
                let bytes = frame.into_data().unwrap_or_default();
                collected.push_str(&String::from_utf8_lossy(&bytes));
                if collected.contains(marker) {
                    break;
                }
            }
            Ok(Some(Err(_))) => {}
            Ok(None) => break,
            Err(_) => {
                panic!("timed out waiting for SSE marker {marker:?}; collected={collected:?}")
            }
        }
    }
    collected
}

/// 读到 SSE 通道自然关闭 (超时即 panic ⇒ 通道没关就会红)。
async fn collect_sse_until_closed(response: axum::response::Response) -> String {
    let mut stream = http_body_util::BodyStream::new(response.into_body());
    let mut collected = String::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let next =
            tokio::time::timeout_at(deadline, tokio_stream::StreamExt::next(&mut stream)).await;
        match next {
            Ok(Some(Ok(frame))) => {
                let bytes = frame.into_data().unwrap_or_default();
                collected.push_str(&String::from_utf8_lossy(&bytes));
            }
            Ok(Some(Err(_))) => {}
            Ok(None) => break,
            Err(_) => panic!("SSE channel did not close in time; collected={collected:?}"),
        }
    }
    collected
}

/// 切分 SSE 文本为帧 (keep-alive 注释行忽略)。
fn parse_sse(text: &str) -> Vec<SseFrame> {
    let mut frames = Vec::new();
    for block in text.split("\n\n") {
        let mut event = String::new();
        let mut data = Vec::new();
        let mut id = None;
        let mut has_event = false;
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("event: ") {
                event = rest.to_string();
                has_event = true;
            } else if let Some(rest) = line.strip_prefix("data: ") {
                data.push(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("id: ") {
                id = Some(rest.to_string());
            }
        }
        if has_event {
            frames.push(SseFrame {
                event,
                data: data.join("\n"),
                id,
            });
        }
    }
    frames
}

#[tokio::test]
async fn the_stream_face_replays_the_delta_frames_then_the_final_frame_with_id_equal_to_sequence() {
    let state = gateway_state().await;
    let registry = global_registry();
    let handle = registry
        .register("c1-face-a", "c1-face-a-req")
        .expect("register");
    registry
        .publish_delta(&handle, None, "hello ")
        .expect("delta 1");
    registry
        .publish_delta(&handle, None, "world")
        .expect("delta 2");
    let mut final_frame =
        C1FinalFrame::new("c1-face-a", "c1-face-a-req", None, C1StreamState::Completed);
    final_frame.text = Some("hello world".to_string());
    registry
        .publish_final(&handle, &mut final_frame)
        .expect("final");

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stream_request("c1-face-a", "c1-face-a-req", "", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let collected = collect_sse_until(response, "apeireth.c1.final.v1").await;
    let frames = parse_sse(&collected);
    assert_eq!(frames.len(), 3, "{collected}");
    assert_eq!(frames[0].event, "apeireth.c1.delta.v1");
    assert_eq!(frames[0].id.as_deref(), Some("1"));
    assert_eq!(frames[1].event, "apeireth.c1.delta.v1");
    assert_eq!(frames[1].id.as_deref(), Some("2"));
    assert_eq!(frames[2].event, "apeireth.c1.final.v1");
    assert_eq!(frames[2].id.as_deref(), Some("3"));

    let data: serde_json::Value = serde_json::from_str(&frames[2].data).expect("data is json");
    assert_eq!(data["state"], "completed");
    assert_eq!(data["text"], "hello world");
    assert_eq!(data["session_id"], "c1-face-a");
    assert_eq!(data["request_id"], "c1-face-a-req");
}

#[tokio::test]
async fn the_after_sequence_query_skips_the_frames_the_client_already_has() {
    let state = gateway_state().await;
    let registry = global_registry();
    let handle = registry
        .register("c1-face-b", "c1-face-b-req")
        .expect("register");
    for text in ["a", "b", "c"] {
        registry.publish_delta(&handle, None, text).expect("delta");
    }
    let mut final_frame =
        C1FinalFrame::new("c1-face-b", "c1-face-b-req", None, C1StreamState::Completed);
    registry
        .publish_final(&handle, &mut final_frame)
        .expect("final");

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stream_request(
            "c1-face-b",
            "c1-face-b-req",
            "?after_sequence=1",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let collected = collect_sse_until(response, "apeireth.c1.final.v1").await;
    let frames = parse_sse(&collected);
    assert_eq!(frames.len(), 3, "{collected}");
    assert_eq!(frames[0].id.as_deref(), Some("2"));
    assert_eq!(frames[1].id.as_deref(), Some("3"));
    assert_eq!(frames[2].id.as_deref(), Some("4"));
    assert_eq!(frames[2].event, "apeireth.c1.final.v1");
}

#[tokio::test]
async fn the_last_event_id_header_wins_over_the_after_sequence_query() {
    let state = gateway_state().await;
    let registry = global_registry();
    let handle = registry
        .register("c1-face-c", "c1-face-c-req")
        .expect("register");
    for text in ["a", "b", "c"] {
        registry.publish_delta(&handle, None, text).expect("delta");
    }
    let mut final_frame =
        C1FinalFrame::new("c1-face-c", "c1-face-c-req", None, C1StreamState::Completed);
    registry
        .publish_final(&handle, &mut final_frame)
        .expect("final");

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stream_request(
            "c1-face-c",
            "c1-face-c-req",
            "?after_sequence=1",
            Some("2"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let collected = collect_sse_until(response, "apeireth.c1.final.v1").await;
    let frames = parse_sse(&collected);
    assert_eq!(frames.len(), 2, "{collected}");
    assert_eq!(
        frames[0].id.as_deref(),
        Some("3"),
        "Last-Event-ID 头优先于查询参数: {collected}"
    );
    assert_eq!(frames[1].id.as_deref(), Some("4"));
}

#[tokio::test]
async fn a_client_behind_the_window_gets_an_explicit_resync_fact_frame_and_the_channel_closes() {
    let state = gateway_state().await;
    let registry = global_registry();
    let handle = registry
        .register("c1-face-d", "c1-face-d-req")
        .expect("register");
    for index in 0..(STREAM_BUFFER_CAPACITY + 10) {
        registry
            .publish_delta(&handle, None, &index.to_string())
            .expect("delta");
    }

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stream_request(
            "c1-face-d",
            "c1-face-d-req",
            "?after_sequence=0",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 读到通道自然关闭: 事实帧之后必须关通道 (否则这里会超时红)。
    let collected = collect_sse_until_closed(response).await;
    let frames = parse_sse(&collected);
    assert_eq!(frames.len(), 1, "{collected}");
    assert_eq!(frames[0].event, "apeireth.c1.final.v1");
    let data: serde_json::Value = serde_json::from_str(&frames[0].data).expect("data is json");
    assert_eq!(data["resync_required"], true, "{collected}");
    assert_eq!(data["state"], "unknown");
    assert_eq!(
        frames[0].id.as_deref(),
        Some("11"),
        "事实帧的 id 是窗口内最早可用序号: {collected}"
    );
}

#[tokio::test]
async fn a_frame_published_after_the_connection_reaches_the_client_live() {
    let state = gateway_state().await;
    let registry = global_registry();
    let handle = registry
        .register("c1-face-e", "c1-face-e-req")
        .expect("register");
    registry
        .publish_delta(&handle, None, "first")
        .expect("delta");

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stream_request("c1-face-e", "c1-face-e-req", "", None))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 连接建立之后发布的帧必须实时投到。
    registry
        .publish_delta(&handle, None, "live-delta")
        .expect("live delta");

    let collected = collect_sse_until(response, "live-delta").await;
    let frames = parse_sse(&collected);
    assert_eq!(frames.len(), 2, "{collected}");
    assert_eq!(frames[0].id.as_deref(), Some("1"));
    assert_eq!(frames[1].id.as_deref(), Some("2"));
    let data: serde_json::Value = serde_json::from_str(&frames[1].data).expect("data is json");
    assert_eq!(data["delta"], "live-delta");
}

#[tokio::test]
async fn an_unknown_stream_answers_not_found() {
    let state = gateway_state().await;
    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stream_request(
            "c1-face-missing",
            "c1-face-missing-req",
            "",
            None,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("error body");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("error body is json");
    assert_eq!(value["error"]["code"], "session_not_found");
}

#[tokio::test]
async fn an_invalid_after_sequence_answers_bad_request() {
    let state = gateway_state().await;
    let registry = global_registry();
    registry
        .register("c1-face-f", "c1-face-f-req")
        .expect("register");

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(stream_request(
            "c1-face-f",
            "c1-face-f-req",
            "?after_sequence=not-a-number",
            None,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("error body");
    let value: serde_json::Value = serde_json::from_slice(&body).expect("error body is json");
    assert_eq!(value["error"]["code"], "invalid_request");
}
