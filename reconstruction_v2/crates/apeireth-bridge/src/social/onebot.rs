use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotMessage {
    pub post_type: String, // "message"
    pub message_type: String, // "private" or "group"
    pub sub_type: String,
    pub message_id: i32,
    pub user_id: i64,
    pub group_id: Option<i64>,
    pub message: String,
    pub raw_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotSendAction {
    pub action: String, // "send_private_msg" or "send_group_msg"
    pub params: serde_json::Value,
}

pub struct OneBotBridge {
    endpoint: String,
}

impl OneBotBridge {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub fn format_private_msg(user_id: i64, message: &str) -> OneBotSendAction {
        OneBotSendAction {
            action: "send_private_msg".into(),
            params: serde_json::json!({
                "user_id": user_id,
                "message": message,
            }),
        }
    }

    pub fn format_group_msg(group_id: i64, message: &str) -> OneBotSendAction {
        OneBotSendAction {
            action: "send_group_msg".into(),
            params: serde_json::json!({
                "group_id": group_id,
                "message": message,
            }),
        }
    }
}
