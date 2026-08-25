//! apeireth-llm-iface - LLM interface trait (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 LlmProvider trait + 真 send + 真 async

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage { pub role: String, pub content: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub usage: std::collections::HashMap<String, u32>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, String>;
}

pub struct MockLlm { pub model: String }

impl MockLlm {
    pub fn new(model: impl Into<String>) -> Self { Self { model: model.into() } }
}

#[async_trait]
impl LlmProvider for MockLlm {
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse, String> {
        Ok(ChatResponse { content: format!("[{} mock] last: {}", self.model, req.messages.last().map(|m| m.content.clone()).unwrap_or_default()), usage: [("tokens".into(), req.max_tokens as u32)].iter().cloned().collect() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_mock_chat() {
        let llm = MockLlm::new("test");
        let req = ChatRequest { model: "test".into(), messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }], max_tokens: 10, temperature: 0.5 };
        let r = llm.chat(req).await.unwrap();
        assert!(r.content.contains("hi"));
    }
    #[test]
    fn test_msg() {
        let m = ChatMessage { role: "user".into(), content: "x".into() };
        assert_eq!(m.role, "user");
    }
}
