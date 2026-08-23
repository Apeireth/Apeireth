use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "frame_type", rename_all = "snake_case")]
pub enum WsFrame {
    // 1. Handshake & Auth
    Handshake { client_id: String, token: String },
    // 2. Ping
    Ping { timestamp: i64 },
    // 3. Pong
    Pong { timestamp: i64 },
    // 4. Text Stream Delta
    TextDelta { session_id: String, text: String },
    // 5. CoT Reasoning Delta
    CoTDelta { session_id: String, reasoning: String },
    // 6. Tool Call Request
    ToolCall { call_id: String, tool_name: String, args_json: String },
    // 7. Tool Result Response
    ToolResult { call_id: String, output: String, success: bool },
    // 8. Error Frame
    Error { code: u16, message: String },
}

impl WsFrame {
    pub fn encode(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn decode(payload: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_8_frame_roundtrip() {
        let frames = vec![
            WsFrame::Handshake { client_id: "c1".into(), token: "tok1".into() },
            WsFrame::Ping { timestamp: 123456 },
            WsFrame::Pong { timestamp: 123456 },
            WsFrame::TextDelta { session_id: "s1".into(), text: "hello".into() },
            WsFrame::CoTDelta { session_id: "s1".into(), reasoning: "thinking".into() },
            WsFrame::ToolCall { call_id: "tc1".into(), tool_name: "shell".into(), args_json: "{}".into() },
            WsFrame::ToolResult { call_id: "tc1".into(), output: "ok".into(), success: true },
            WsFrame::Error { code: 500, message: "server crash".into() },
        ];

        for frame in frames {
            let enc = frame.encode().unwrap();
            let dec = WsFrame::decode(&enc).unwrap();
            assert_eq!(frame, dec);
        }
    }
}

