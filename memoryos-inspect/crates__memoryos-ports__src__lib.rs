**Source**: https://raw.githubusercontent.com/TelivANT/memoryos-rust/main/crates/memoryos-ports/src/lib.rs

---

pub mod history;
pub mod llm;
pub mod memory;
pub mod multimodal;
pub mod wiki;

use async_trait::async_trait;
use memoryos_core::{AppError, HealthStatus};

pub use history::HistoryStorage;
pub use llm::{
    ChatDelta, ChatMessage, ChatRequest, ChatResponse, ChatStreamChoice, ChatStreamChunk,
    LlmAdapter,
};
pub use memory::{ConcurrencyControl, MemoryManager, ShortTermStorage, VectorStorage};
pub use multimodal::MultiModalStorage;
pub use wiki::{WikiAdapter, WikiDocument};

#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish_chat_log(
        &self,
        event_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), AppError>;
}

#[async_trait]
pub trait HealthProbe: Send + Sync {
    async fn current_status(&self) -> Result<HealthStatus, AppError>;
}