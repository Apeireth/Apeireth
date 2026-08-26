//! Transport adapter registry.
//!
//! Borrowed from OpenClaw: each inbound channel (HTTP, WebSocket, Telegram,
//! Discord, iOS-push, etc.) is a `Transport` impl. The gateway process spawns
//! one transport per channel and forwards inbound frames into a single
//! `ChanneledBus` for unified routing.
//!
//! We provide two reference stubs (Http + Ws) and an `InMemory` transport for
//! tests. Real Telegram/Discord/iOS clients are out of scope for this crate
//! — they live in their own integration crate (e.g. `apeireth-telegram`).

use crate::node::NodeId;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// One inbound frame: who sent it, where it came from, what payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InFrame {
    pub node_id: NodeId,
    pub channel: String,
    pub payload: serde_json::Value,
}

impl InFrame {
    pub fn new(node_id: NodeId, channel: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            node_id,
            channel: channel.into(),
            payload,
        }
    }
}

/// One outbound frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutFrame {
    pub channel: String,
    pub payload: serde_json::Value,
}

impl OutFrame {
    pub fn new(channel: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            channel: channel.into(),
            payload,
        }
    }
}

/// Transport adapter trait: 5 methods map to OpenClaw start/stop + send/recv.
#[async_trait]
pub trait Transport: Send + Sync {
    fn channel(&self) -> &str;
    async fn start(&self) -> Result<(), TransportError>;
    async fn stop(&self) -> Result<(), TransportError>;
    async fn send(&self, frame: OutFrame) -> Result<(), TransportError>;
    async fn recv(&self) -> Option<InFrame>;
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("transport: not started")]
    NotStarted,
    #[error("transport: already started")]
    AlreadyStarted,
    #[error("transport: send failed: {0}")]
    SendFailed(String),
    #[error("transport: io error: {0}")]
    Io(String),
}

#[derive(Default, Clone)]
pub struct TransportRegistry {
    inner: Arc<Mutex<HashMap<String, Arc<dyn Transport>>>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, t: Arc<dyn Transport>) -> Option<Arc<dyn Transport>> {
        let key = t.channel().to_string();
        self.inner.lock().insert(key, t)
    }

    pub fn get(&self, channel: &str) -> Option<Arc<dyn Transport>> {
        self.inner.lock().get(channel).cloned()
    }

    pub fn unregister(&self, channel: &str) -> Option<Arc<dyn Transport>> {
        self.inner.lock().remove(channel)
    }

    pub fn channels(&self) -> Vec<String> {
        self.inner.lock().keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.inner.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().is_empty()
    }
}

/// In-memory transport for tests + the local CasePipe-style loopback.
pub struct InMemoryTransport {
    channel: String,
    started: Arc<Mutex<bool>>,
    inbox: Arc<Mutex<Vec<InFrame>>>,
    outbox: Arc<Mutex<Vec<OutFrame>>>,
}

impl InMemoryTransport {
    pub fn new(channel: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            started: Arc::new(Mutex::new(false)),
            inbox: Arc::new(Mutex::new(Vec::new())),
            outbox: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push_inbound(&self, frame: InFrame) {
        self.inbox.lock().push(frame);
    }

    pub fn drain_outbound(&self) -> Vec<OutFrame> {
        std::mem::take(&mut *self.outbox.lock())
    }

    pub fn outbound_len(&self) -> usize {
        self.outbox.lock().len()
    }
}

#[async_trait]
impl Transport for InMemoryTransport {
    fn channel(&self) -> &str {
        &self.channel
    }

    async fn start(&self) -> Result<(), TransportError> {
        let mut g = self.started.lock();
        if *g {
            return Err(TransportError::AlreadyStarted);
        }
        *g = true;
        Ok(())
    }

    async fn stop(&self) -> Result<(), TransportError> {
        let mut g = self.started.lock();
        if !*g {
            return Err(TransportError::NotStarted);
        }
        *g = false;
        Ok(())
    }

    async fn send(&self, frame: OutFrame) -> Result<(), TransportError> {
        if !*self.started.lock() {
            return Err(TransportError::NotStarted);
        }
        self.outbox.lock().push(frame);
        Ok(())
    }

    async fn recv(&self) -> Option<InFrame> {
        self.inbox.lock().pop()
    }
}

/// Stub HTTP transport. Real implementation lives in `apeireth-api`; this
/// stub exists so the gateway can be wired without pulling in axum/hyper.
pub struct HttpTransport {
    started: Arc<Mutex<bool>>,
    pub port: u16,
    pub host: String,
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new("127.0.0.1", 8080)
    }
}

impl HttpTransport {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            started: Arc::new(Mutex::new(false)),
            host: host.into(),
            port,
        }
    }
}

#[async_trait]
impl Transport for HttpTransport {
    fn channel(&self) -> &str {
        "http"
    }

    async fn start(&self) -> Result<(), TransportError> {
        let mut g = self.started.lock();
        if *g {
            return Err(TransportError::AlreadyStarted);
        }
        *g = true;
        Ok(())
    }

    async fn stop(&self) -> Result<(), TransportError> {
        let mut g = self.started.lock();
        if !*g {
            return Err(TransportError::NotStarted);
        }
        *g = false;
        Ok(())
    }

    async fn send(&self, _frame: OutFrame) -> Result<(), TransportError> {
        if !*self.started.lock() {
            return Err(TransportError::NotStarted);
        }
        // Real impl: forward into axum/hyper server's outbound channel.
        Err(TransportError::SendFailed(
            "HttpTransport stub is a no-op".into(),
        ))
    }

    async fn recv(&self) -> Option<InFrame> {
        // Real impl: poll an MPSC channel fed by axum handlers.
        None
    }
}

/// Stub WebSocket transport.
pub struct WsTransport {
    started: Arc<Mutex<bool>>,
    pub path: String,
}

impl Default for WsTransport {
    fn default() -> Self {
        Self::new("/ws")
    }
}

impl WsTransport {
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            started: Arc::new(Mutex::new(false)),
            path: path.into(),
        }
    }
}

#[async_trait]
impl Transport for WsTransport {
    fn channel(&self) -> &str {
        "ws"
    }

    async fn start(&self) -> Result<(), TransportError> {
        let mut g = self.started.lock();
        if *g {
            return Err(TransportError::AlreadyStarted);
        }
        *g = true;
        Ok(())
    }

    async fn stop(&self) -> Result<(), TransportError> {
        let mut g = self.started.lock();
        if !*g {
            return Err(TransportError::NotStarted);
        }
        *g = false;
        Ok(())
    }

    async fn send(&self, _frame: OutFrame) -> Result<(), TransportError> {
        if !*self.started.lock() {
            return Err(TransportError::NotStarted);
        }
        Err(TransportError::SendFailed(
            "WsTransport stub is a no-op".into(),
        ))
    }

    async fn recv(&self) -> Option<InFrame> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn registry_register_and_get() {
        let r = TransportRegistry::new();
        let t = Arc::new(InMemoryTransport::new("mem"));
        assert!(r.register(t).is_none());
        assert!(r.get("mem").is_some());
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn registry_unregister_drops() {
        let r = TransportRegistry::new();
        let t = Arc::new(InMemoryTransport::new("mem"));
        r.register(t);
        assert!(r.unregister("mem").is_some());
        assert!(r.is_empty());
    }

    #[test]
    fn registry_channels_lists() {
        let r = TransportRegistry::new();
        r.register(Arc::new(InMemoryTransport::new("a")));
        r.register(Arc::new(InMemoryTransport::new("b")));
        let mut channels = r.channels();
        channels.sort();
        assert_eq!(channels, vec!["a", "b"]);
    }

    #[tokio::test]
    async fn in_memory_transport_lifecycle() {
        let t = InMemoryTransport::new("mem");
        assert!(t.start().await.is_ok());
        assert!(matches!(
            t.start().await,
            Err(TransportError::AlreadyStarted)
        ));
        assert!(t.send(OutFrame::new("mem", json!({"k": 1}))).await.is_ok());
        assert_eq!(t.outbound_len(), 1);
        let _ = t.drain_outbound();
        assert_eq!(t.outbound_len(), 0);
        assert!(t.stop().await.is_ok());
        assert!(matches!(t.stop().await, Err(TransportError::NotStarted)));
    }

    #[tokio::test]
    async fn in_memory_transport_recv_yields_inbound() {
        let t = InMemoryTransport::new("mem");
        t.start().await.unwrap();
        t.push_inbound(InFrame::new(Uuid::new_v4(), "mem", json!({"hello": 1})));
        let frame = t.recv().await.unwrap();
        assert_eq!(frame.payload, json!({"hello": 1}));
    }

    #[tokio::test]
    async fn http_transport_starts_and_stops() {
        let t = HttpTransport::default();
        assert!(t.start().await.is_ok());
        assert_eq!(t.channel(), "http");
        assert!(t.stop().await.is_ok());
    }

    #[tokio::test]
    async fn http_transport_send_stub_returns_err() {
        let t = HttpTransport::default();
        t.start().await.unwrap();
        assert!(matches!(
            t.send(OutFrame::new("http", json!({}))).await,
            Err(TransportError::SendFailed(_))
        ));
    }

    #[tokio::test]
    async fn ws_transport_starts_and_stops() {
        let t = WsTransport::default();
        assert!(t.start().await.is_ok());
        assert_eq!(t.channel(), "ws");
        assert!(t.stop().await.is_ok());
    }

    #[tokio::test]
    async fn transport_send_before_start_errors() {
        let t = InMemoryTransport::new("x");
        assert!(matches!(
            t.send(OutFrame::new("x", json!({}))).await,
            Err(TransportError::NotStarted)
        ));
    }

    #[tokio::test]
    async fn http_transport_recv_returns_none() {
        let t = HttpTransport::default();
        assert!(t.recv().await.is_none());
    }
}
