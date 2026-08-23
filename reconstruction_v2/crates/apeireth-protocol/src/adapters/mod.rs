pub mod anthropic;
pub mod gemini;
pub mod minimax;
pub mod openai;

pub use anthropic::AnthropicAdapter;
pub use gemini::GeminiAdapter;
pub use minimax::MinimaxAdapter;
pub use openai::OpenAiAdapter;

use crate::normalized::{NormalizedRequest, NormalizedResponse};
use async_trait::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("Protocol error: {0}")]
    InvalidProtocol(String),
}

#[async_trait]
pub trait ProtocolAdapter: Send + Sync {
    fn provider_name(&self) -> &'static str;
    async fn execute(&self, api_key: &str, request: &NormalizedRequest) -> Result<NormalizedResponse, ProtocolError>;
}

