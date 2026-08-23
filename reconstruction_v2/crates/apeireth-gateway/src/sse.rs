use tokio::sync::broadcast::{self, Receiver, Sender};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: i64,
}

#[derive(Clone)]
pub struct SseBroadcaster {
    sender: Sender<SseEvent>,
}

impl Default for SseBroadcaster {
    fn default() -> Self {
        Self::new(256)
    }
}

impl SseBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn broadcast(&self, event_type: impl Into<String>, payload: serde_json::Value) {
        let event = SseEvent {
            event_type: event_type.into(),
            payload,
            timestamp: chrono::Utc::now().timestamp(),
        };
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> Receiver<SseEvent> {
        self.sender.subscribe()
    }
}
