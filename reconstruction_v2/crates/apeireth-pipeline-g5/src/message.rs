//! Pipeline messages.

use serde::{Deserialize, Serialize};

pub const MAX_KIND_LEN: usize = 64;
pub const MAX_PAYLOAD_LEN: usize = 256 * 1024;
pub const MAX_TRACE_ID_LEN: usize = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineMessage {
    pub kind: String,
    pub trace_id: String,
    pub payload: serde_json::Value,
}

impl PipelineMessage {
    pub fn new(kind: impl Into<String>, trace_id: impl Into<String>, payload: serde_json::Value) -> Self {
        Self { kind: kind.into(), trace_id: trace_id.into(), payload }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_construct() {
        let m = PipelineMessage::new("chat", "trace-1", serde_json::json!({"text": "hi"}));
        assert_eq!(m.kind, "chat");
        assert_eq!(m.trace_id, "trace-1");
    }

    #[test]
    fn constants() {
        assert_eq!(MAX_KIND_LEN, 64);
        assert_eq!(MAX_TRACE_ID_LEN, 128);
    }
}
