//! Orchestration-local LLM mirror trait (per RC-6).
//!
//! **Why mirror (not `use apeireth_plugin::LlmFactory`)**:
//! `apeireth-plugin` already depends on `apeireth-orchestration` (it re-exports
//! `SubagentRole` from us). Adding `apeireth-plugin` as a dep of
//! `apeireth-orchestration` would create a cycle. We mirror the minimal contract
//! the council needs (`LlmFactory::spawn` → `LlmInstance::complete`) here so the
//! trait lives at the layer that consumes it.

use std::pin::Pin;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::SubagentRole;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub system_prompt: String,
    pub messages: Vec<CompletionMessage>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

fn default_temperature() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub message: CompletionMessage,
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
    pub finish_reason: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub type CompletionStream<'a> =
    Pin<Box<dyn futures::Stream<Item = Result<CompletionResponse, LlmError>> + Send + 'a>>;

#[derive(Debug)]
pub enum LlmError {
    Credentials(String),
    Network(String),
    RateLimited { retry_after_ms: u64 },
    Provider(String),
    Stream(String),
    NotImplemented(&'static str),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Credentials(m) => write!(f, "llm credentials error: {m}"),
            Self::Network(m) => write!(f, "llm network error: {m}"),
            Self::RateLimited { retry_after_ms } => {
                write!(f, "llm rate limited, retry after {retry_after_ms}ms")
            }
            Self::Provider(m) => write!(f, "llm provider error: {m}"),
            Self::Stream(m) => write!(f, "llm stream error: {m}"),
            Self::NotImplemented(what) => {
                write!(f, "llm not implemented: {what} (0 装 PASS; rc 阶段实现)")
            }
        }
    }
}

impl std::error::Error for LlmError {}

#[async_trait]
pub trait LlmInstance: Send + Sync {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    async fn stream(&self, _req: CompletionRequest) -> Result<CompletionStream<'_>, LlmError> {
        Err(LlmError::NotImplemented(
            "LlmInstance::stream (0 装 PASS; rc 阶段实现)",
        ))
    }
    fn name(&self) -> &str;
}

#[async_trait]
pub trait LlmFactory: Send + Sync {
    async fn spawn(
        &self,
        role: SubagentRole,
        model: &str,
    ) -> Result<Box<dyn LlmInstance>, LlmError>;
    async fn available_models(&self) -> Result<Vec<String>, LlmError>;
    fn name(&self) -> &str;
}

pub struct NoopLlmFactory;

#[async_trait]
impl LlmFactory for NoopLlmFactory {
    async fn spawn(
        &self,
        _role: SubagentRole,
        _model: &str,
    ) -> Result<Box<dyn LlmInstance>, LlmError> {
        Err(LlmError::NotImplemented(
            "NoopLlmFactory::spawn (0 装 PASS; runtime 注入真 LlmFactory)",
        ))
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_request_default_temperature() {
        let req = CompletionRequest {
            system_prompt: "system".into(),
            messages: vec![],
            temperature: default_temperature(),
            tools: vec![],
            max_tokens: None,
        };
        assert_eq!(req.temperature, 1.0);
    }

    #[test]
    fn llm_error_displays() {
        let e = LlmError::NotImplemented("test");
        let s = format!("{e}");
        assert!(s.contains("not implemented"));

        let e2 = LlmError::RateLimited {
            retry_after_ms: 5000,
        };
        let s2 = format!("{e2}");
        assert!(s2.contains("rate limited"));
        assert!(s2.contains("5000"));
    }

    #[tokio::test]
    async fn noop_factory_spawn_returns_error() {
        let factory = NoopLlmFactory;
        let result = factory.spawn(SubagentRole::Reviewer, "minimax-m3").await;
        match result {
            Err(LlmError::NotImplemented(_)) => {}
            Err(other) => panic!("expected NotImplemented, got {other:?}"),
            Ok(_) => panic!("expected NotImplemented, got Ok"),
        }
    }

    #[tokio::test]
    async fn noop_factory_available_models_empty() {
        let factory = NoopLlmFactory;
        let models = factory.available_models().await.expect("ok");
        assert!(models.is_empty());
    }
}
