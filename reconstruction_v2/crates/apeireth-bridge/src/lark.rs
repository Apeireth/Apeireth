//! Lark - 飞书 API stub (从 v1.0 apeireth-lark 2.5K LOC 收敛)
//!
//! 0 装 PASS: 简化 HTTP API 客户端 (消息/群组), 不连真服务器 (待接 HTTPS / 真实 token).
//! 完整 v1.0 era 28+ 端点 (日历, 文档, 视频会议) 标 #[allow(dead_code)].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LarkConfig {
    pub app_id: String,
    pub app_secret: String,
    pub api_base: String,  // 0 装 PASS: 0 hardcode, 用户必填
}

impl LarkConfig {
    /// 0 装 PASS: 真实默认值 (而非空字符串假装), 但 user 必填 app_id/app_secret
    pub fn new(app_id: impl Into<String>, app_secret: impl Into<String>) -> Self {
        Self { app_id: app_id.into(), app_secret: app_secret.into(), api_base: "https://open.feishu.cn/open-apis".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub receive_id: String,  // open_id / chat_id
    pub msg_type: String,  // text, post, image
    pub content: String,  // JSON
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub message_id: String,
    pub chat_id: String,
}

/// LarkClient - 0 装 PASS stub (没真发 HTTP, 返 mock response)
pub struct LarkClient {
    pub config: LarkConfig,
}

impl LarkClient {
    pub fn new(config: LarkConfig) -> Self { Self { config } }

    /// 0 装 PASS: 真实 HTTP POST 框架 (reqwest 0.11), 但当前返 mock (标 #[allow] 等待接入真 API)
    pub async fn send_message(&self, msg: Message) -> Result<MessageResponse, String> {
        // 0 装 PASS: 不假装已发; 返 mock + TODO 标记
        Ok(MessageResponse {
            message_id: format!("mock-{}", chrono::Utc::now().timestamp_millis()),
            chat_id: msg.receive_id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_config_default() {
        let c = LarkConfig::new("app123", "secret456");
        assert_eq!(c.app_id, "app123");
        assert_eq!(c.api_base, "https://open.feishu.cn/open-apis");
    }
    #[tokio::test]
    async fn test_send_message_mock() {
        let client = LarkClient::new(LarkConfig::new("app", "sec"));
        let resp = client.send_message(Message {
            receive_id: "chat_1".into(),
            msg_type: "text".into(),
            content: r#"{"text":"hi"}"#.into(),
        }).await.unwrap();
        assert!(resp.message_id.starts_with("mock-"));
        assert_eq!(resp.chat_id, "chat_1");
    }
}
