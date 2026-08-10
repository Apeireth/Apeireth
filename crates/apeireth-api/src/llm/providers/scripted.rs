//! ScriptedLlmProvider — 测试用 mock provider (脚本化响应)
//!
//! 用途: 单元测试 / 离线测试 / CI 环境 (无真实 LLM 时)
//! Week 1 实装
//!
//! **设计**: 维护 (关键词 → 响应) 列表, 按 prompt 中是否含关键词匹配第一个命中
//! **不调外部**: 纯 Rust, 无 IO, 无 HTTP

use std::collections::HashMap;

use async_trait::async_trait;

use crate::llm::error::LlmError;
use crate::llm::traits::{ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse, TokenUsage};

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptedResponse {
    pub content: String,
    pub usage: TokenUsage,
    pub latency_ms: u64,
    pub finish_reason: String,
}

impl ScriptedResponse {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            usage: TokenUsage::new(10, 20),
            latency_ms: 100,
            finish_reason: "stop".to_string(),
        }
    }
}

pub struct ScriptedLlmProvider {
    name: String,
    scripts: Vec<(String, ScriptedResponse)>,
    default: ScriptedResponse,
}

impl ScriptedLlmProvider {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            scripts: Vec::new(),
            default: ScriptedResponse::new("[ScriptedLlmProvider] 默认响应 — 无关键词命中"),
        }
    }

    pub fn with_script(mut self, keyword: impl Into<String>, response: ScriptedResponse) -> Self {
        self.scripts.push((keyword.into().to_lowercase(), response));
        self
    }

    pub fn with_default(mut self, response: ScriptedResponse) -> Self {
        self.default = response;
        self
    }

    pub fn scripts_count(&self) -> usize {
        self.scripts.len()
    }

    /// 拼接所有消息到一个字符串用于关键词搜索
    fn prompt_text(&self, req: &LlmRequest) -> String {
        req.messages
            .iter()
            .map(|m| match m.role {
                ChatRole::System => format!("[SYSTEM] {}\n", m.content),
                ChatRole::User => format!("[USER] {}\n", m.content),
                ChatRole::Assistant => format!("[ASSISTANT] {}\n", m.content),
            })
            .collect::<Vec<_>>()
            .join("")
            .to_lowercase()
    }
}

impl Default for ScriptedLlmProvider {
    fn default() -> Self {
        Self::new("scripted")
    }
}

#[async_trait]
impl LlmProvider for ScriptedLlmProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn supports_model(&self, _model: &str) -> bool {
        // Scripted 接受任何 model (mock)
        true
    }

    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        let prompt = self.prompt_text(&req);

        for (keyword, response) in &self.scripts {
            if prompt.contains(keyword) {
                return Ok(LlmResponse {
                    content: response.content.clone(),
                    usage: response.usage.clone(),
                    latency_ms: response.latency_ms,
                    model: req.model,
                    finish_reason: response.finish_reason.clone(),
                    provider: self.name.clone(),
                });
            }
        }

        Ok(LlmResponse {
            content: self.default.content.clone(),
            usage: self.default.usage.clone(),
            latency_ms: self.default.latency_ms,
            model: req.model,
            finish_reason: self.default.finish_reason.clone(),
            provider: self.name.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_match() {
        let provider =
            ScriptedLlmProvider::new("test").with_script("hello", ScriptedResponse::new("hi back"));
        let req = LlmRequest::new("m", vec![ChatMessage::user("hello world")]);
        let resp = provider.complete(req).await.unwrap();
        assert_eq!(resp.content, "hi back");
    }

    #[tokio::test]
    async fn test_default_response() {
        let provider = ScriptedLlmProvider::new("test");
        let req = LlmRequest::new("m", vec![ChatMessage::user("unmatched")]);
        let resp = provider.complete(req).await.unwrap();
        assert!(resp.content.contains("默认响应"));
    }

    #[tokio::test]
    async fn test_case_insensitive() {
        let provider =
            ScriptedLlmProvider::new("test").with_script("HELLO", ScriptedResponse::new("hi"));
        let req = LlmRequest::new("m", vec![ChatMessage::user("hello")]);
        let resp = provider.complete(req).await.unwrap();
        assert_eq!(resp.content, "hi");
    }

    #[tokio::test]
    async fn test_multiple_scripts_first_match() {
        let provider = ScriptedLlmProvider::new("test")
            .with_script("first", ScriptedResponse::new("matched first"))
            .with_script("hello", ScriptedResponse::new("hi"));
        let req = LlmRequest::new("m", vec![ChatMessage::user("hello first")]);
        let resp = provider.complete(req).await.unwrap();
        // "first" 在 "hello first" 中先注册, 先匹配
        assert_eq!(resp.content, "matched first");
    }
}
