//! LLM port — abstract LLM client (OpenAI-compatible / Anthropic / local Ollama)

use async_trait::async_trait;
use super::PortError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    pub prompt: String,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub temperature: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: u32,
    pub latency_ms: u64,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse, PortError>;
    async fn stream(&self, req: &LlmRequest) -> Result<tokio::sync::mpsc::Receiver<String>, PortError>;
}