//! SSE event bus: in-process delivery plus the HTTP endpoint.

use std::sync::Arc;

use apeireth_gateway::{
    build_gateway_state, canonical_router_with_state, EventBus, GatewayEvent, GatewayState,
};
use apeireth_runtime::canonical::Runtime;
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
        let next = tokio::time::timeout_at(deadline, tokio_stream::StreamExt::next(&mut stream)).await;
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
