use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub channel: String, // "discord", "telegram", "onebot", "cli"
    pub sender_id: String,
    pub sender_name: String,
    pub content: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub channel: String,
    pub recipient_id: String,
    pub content: String,
}

pub struct BridgeDispatcher {
    pub total_dispatched: u64,
}

impl Default for BridgeDispatcher {
    fn default() -> Self {
        Self { total_dispatched: 0 }
    }
}

impl BridgeDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn route_inbound(&mut self, msg: InboundMessage) -> String {
        self.total_dispatched += 1;
        format!("[{}:{}] {}", msg.channel, msg.sender_name, msg.content)
    }
}
