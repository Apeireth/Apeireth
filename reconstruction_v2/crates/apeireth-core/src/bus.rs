use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub topic: Topic,
    pub payload: String,
    pub timestamp_ms: i64,
}

pub struct EventBus {
    sender: broadcast::Sender<EventEnvelope>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(128)
    }
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { sender: tx }
    }

    pub fn publish(&self, topic: impl Into<String>, payload: impl Into<String>) -> usize {
        let envelope = EventEnvelope {
            topic: Topic(topic.into()),
            payload: payload.into(),
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        };
        self.sender.send(envelope).unwrap_or(0)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new(32);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.publish("companion.emotion", "Pleasure increased");

        let msg1 = rx1.recv().await.unwrap();
        assert_eq!(msg1.topic.0, "companion.emotion");
        assert_eq!(msg1.payload, "Pleasure increased");

        let msg2 = rx2.recv().await.unwrap();
        assert_eq!(msg2.topic.0, "companion.emotion");
        assert_eq!(msg2.payload, "Pleasure increased");
    }
}

