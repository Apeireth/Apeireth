//! Gateway-level SSE event bus (`GET /v1/apeireth/events`).
//!
//! Emits product-facing lifecycle events:
//! `backend_ready` / `turn_started` / `turn_delta` / `turn_completed` /
//! `approval_required` / `approval_resolved`.
//!
//! v1 honesty notes (contract §8):
//! - `turn_delta` carries the final assistant text as ONE delta: the canonical
//!   runtime completes a turn before the gateway encodes it, so token-level
//!   deltas are not observable at this boundary.
//! - The bus is in-process broadcast; a subscriber that lags behind is
//!   disconnected by tokio broadcast semantics (no unbounded buffering).

use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::extract::State;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::panels::GatewayState;

/// Bus capacity: bounded, newest-first under pressure.
const BUS_CAPACITY: usize = 256;

/// One product-facing event frame.
#[derive(Debug, Clone, Serialize)]
pub struct GatewayEvent {
    /// Stable event name (see module docs).
    pub event: String,
    /// Event payload (JSON object).
    pub data: serde_json::Value,
}

impl GatewayEvent {
    pub fn new(event: &str, data: serde_json::Value) -> Self {
        Self {
            event: event.to_string(),
            data,
        }
    }
}

/// In-process event bus shared by handlers and the SSE endpoint.
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<GatewayEvent>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(BUS_CAPACITY)
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish one event; no subscribers is a silent success, and a lagging
    /// subscriber never blocks the publisher.
    pub fn publish(&self, event: GatewayEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe for events emitted after this call.
    pub fn subscribe(&self) -> broadcast::Receiver<GatewayEvent> {
        self.tx.subscribe()
    }
}

/// `GET /v1/apeireth/events` — SSE stream of gateway lifecycle events.
pub async fn events_handler(State(state): State<GatewayState>) -> Response {
    let receiver = state.events.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|item| match item {
        Ok(event) => match serde_json::to_string(&event.data) {
            Ok(payload) => Some(Ok::<_, Infallible>(
                SseEvent::default().event(&event.event).data(payload),
            )),
            Err(_) => Some(Ok::<_, Infallible>(
                SseEvent::default().event(&event.event).data("{}"),
            )),
        },
        Err(_lagged) => None,
    });
    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}
