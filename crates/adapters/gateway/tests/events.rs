//! SSE event bus: in-process delivery plus the HTTP endpoint.

use std::sync::Arc;

use apeireth_gateway::{
    build_gateway_state, canonical_router_with_state, EventBus, GatewayEvent, GatewayState,
};
use apeireth_runtime::canonical::{Runtime, RuntimeEvent, RuntimeEventSink};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn event_bus_delivers_published_events_in_order() {
    let bus = EventBus::new(16);
    let mut receiver = bus.subscribe();

    bus.publish(GatewayEvent::new(
        "turn_started",
        serde_json::json!({ "session": "s1" }),
    ));
    bus.publish(GatewayEvent::new(
        "turn_completed",
        serde_json::json!({ "rounds": 2 }),
    ));

    let first = receiver.recv().await.unwrap();
    assert_eq!(first.event, "turn_started");
    assert_eq!(first.data["session"], "s1");

    let second = receiver.recv().await.unwrap();
    assert_eq!(second.event, "turn_completed");
    assert_eq!(second.data["rounds"], 2);
}

#[tokio::test]
async fn one_runtime_event_becomes_one_sse_event() {
    let bus = EventBus::new(16);
    let mut receiver = bus.subscribe();
    let sink: &dyn RuntimeEventSink = &bus;

    sink.emit(RuntimeEvent::TurnStarted {
        session: "00000000-0000-0000-0000-000000000001".parse().unwrap(),
        request: apeireth_core::kernel::RequestId::new(),
        trace: apeireth_core::kernel::TraceId::new(),
    });

    assert_eq!(receiver.recv().await.unwrap().event, "turn_started");
    assert!(matches!(
        receiver.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
}

#[tokio::test]
async fn sse_endpoint_streams_events_to_subscribers() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);
    let bus = state.events.clone();
    let router = canonical_router_with_state(state);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Publish only after the handler has subscribed.
    bus.publish(GatewayEvent::new(
        "backend_ready",
        serde_json::json!({ "endpoint": "t" }),
    ));
    bus.publish(GatewayEvent::new(
        "turn_started",
        serde_json::json!({ "session": "s9" }),
    ));

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
                if collected.contains("turn_started") {
                    break;
                }
            }
            Ok(Some(Err(_))) => {}
            Ok(None) => break,
            Err(_) => panic!("timed out waiting for SSE frames; collected={collected:?}"),
        }
    }

    assert!(collected.contains("event: backend_ready"), "{collected}");
    assert!(collected.contains("event: turn_started"), "{collected}");
    assert!(collected.contains("s9"), "{collected}");
}

#[tokio::test]
async fn gateway_state_wires_presence_service_onto_the_bus() {
    // 装配接线证明: build_gateway_state 注入的 presence 服务与总线同源,
    // 心跳帧能到达订阅者 (契约 §8a: presence_state 与 turn_* 同管道)。
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);
    let mut receiver = state.events.subscribe();

    state.presence.emit_heartbeat();

    let frame = receiver.recv().await.unwrap();
    assert_eq!(frame.event, "presence_state");
    assert_eq!(frame.data["type"], "presence_state");
    assert_eq!(frame.data["significance"], "heartbeat");
    assert_eq!(frame.data["source"]["kind"], "heuristic_v0");
}

// ---------------------------------------------------------------------------
// SSE 交付面 (逐连接租约 + 连接即送快照帧 + lag 显式省略) 与 presence 快照口
// ---------------------------------------------------------------------------

/// Read SSE frames until `marker` shows up in the accumulated text.
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

#[tokio::test]
async fn sse_connection_receives_the_snapshot_frame_on_connect_without_replay() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);
    let bus = state.events.clone();

    // 连接前已有历史帧: 只有最新帧应当以快照帧送出, 历史不回放。
    bus.publish(GatewayEvent::new(
        "turn_started",
        serde_json::json!({ "marker": "stale-first-frame" }),
    ));
    bus.publish(GatewayEvent::new(
        "turn_completed",
        serde_json::json!({ "marker": "latest-frame" }),
    ));

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let collected = collect_sse_until(response, "stream_snapshot").await;
    assert!(collected.contains("event: stream_snapshot"), "{collected}");
    assert!(collected.contains("latest-frame"), "连接即送当前帧");
    assert!(
        !collected.contains("stale-first-frame"),
        "快照即读不回放历史帧: {collected}"
    );
}

#[tokio::test]
async fn sse_connection_holds_one_delivery_lease_for_the_body_lifetime() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);
    let bus = state.events.clone();
    assert_eq!(bus.connections(), 0, "无连接时无租约");

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .unwrap();
    assert_eq!(bus.connections(), 1, "一条 SSE 连接持一枚租约");

    drop(response); // 客户端断开 = 租约释放
    assert_eq!(bus.connections(), 0, "断开即释放租约");
}

#[tokio::test]
async fn sse_lag_reports_an_explicit_frames_omitted_fact_instead_of_a_silent_gap() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);
    // 用小容量总线逼出 broadcast 落后语义。
    let bus = EventBus::new(2);
    let events_stream = Arc::new(bus.keepalive().expect("pin"));
    let state = GatewayState {
        events: bus.clone(),
        events_stream,
        ..state
    };

    // 保留一份 state 使装配 pin 在请求之后仍然在场 (帧面保持开启)。
    let router = canonical_router_with_state(state.clone());
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .unwrap();

    // 连接方尚未读取, 连发 6 帧越过容量 2 → 落后跳帧。
    for n in 0..6 {
        bus.publish(GatewayEvent::new(
            "turn_started",
            serde_json::json!({ "n": n }),
        ));
    }

    let collected = collect_sse_until(response, "frames_omitted").await;
    assert!(collected.contains("event: frames_omitted"), "{collected}");
    assert!(collected.contains("\"omitted\""), "{collected}");

    // 失败即帧: 丢帧作为显式失败帧挂在帧面上, 下一成功帧自清。
    let failure = bus.frame_snapshot().failure;
    assert!(
        failure.as_deref().unwrap_or_default().contains("omitted"),
        "丢帧必须成为显式失败帧, got {failure:?}"
    );
    bus.publish(GatewayEvent::new("turn_completed", serde_json::json!({})));
    assert!(!bus.frame_snapshot().has_failure(), "下一成功帧自清失败帧");
}

#[tokio::test]
async fn sse_delivery_shutdown_answers_an_explicit_error_frame() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);
    state.events.shutdown_delivery();

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/events")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["error"]["code"], "internal");
    assert!(
        body["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("unavailable"),
        "{body}"
    );
}

#[tokio::test]
async fn presence_snapshot_route_returns_the_current_frame_on_read() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);
    state.presence.emit_heartbeat();

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/presence")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["type"], "presence_snapshot");
    assert_eq!(body["frame"]["type"], "presence_state");
    assert_eq!(body["frame"]["significance"], "heartbeat");
    assert!(body["failure"].is_null(), "无故障时不伪造失败帧");
}

#[tokio::test]
async fn presence_snapshot_route_shapes_the_snapshot_envelope() {
    let runtime = Arc::new(Runtime::builder().build().await.unwrap());
    let state: GatewayState = build_gateway_state(runtime, None);

    let router = canonical_router_with_state(state);
    let response = router
        .oneshot(
            Request::builder()
                .uri("/v1/apeireth/presence")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    // 快照即读不回放: 从未出帧时 frame 为空, 不编造。
    let object = body.as_object().unwrap();
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["failure", "frame", "type"]);
    assert!(object["frame"].is_null(), "无当前帧时如实为空");
}
