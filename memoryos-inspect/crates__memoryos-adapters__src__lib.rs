**Source**: https://raw.githubusercontent.com/TelivANT/memoryos-rust/main/crates/memoryos-adapters/src/lib.rs

---

pub mod eventbus;
pub mod history;
pub mod llm;
pub mod memory;
pub mod multimodal;
pub mod wiki;

use async_trait::async_trait;
use memoryos_core::{AppError, DependencyState, HealthMode, HealthStatus};
use memoryos_ports::HealthProbe;

pub use eventbus::RedisStreamEventBus;
pub use history::QdrantHistoryStorage;
pub use llm::{
    AzureOpenAiAdapter, ClaudeAdapter, CohereAdapter, DeepSeekAdapter, GeminiAdapter, GroqAdapter,
    MistralAdapter, OllamaAdapter, OpenAiAdapter, OpenRouterAdapter,
};
pub use memory::{
    ChromaStorage, DefaultMemoryManager, DegradedMemoryManager, NoopMemoryManager, PineconeStorage,
    QdrantStorage, RedisStorage,
};
pub use wiki::OpenDALAdapter;

#[derive(Debug, Clone)]
pub struct StaticHealthProbe {
    pub status: HealthStatus,
}

impl Default for StaticHealthProbe {
    fn default() -> Self {
        Self {
            status: HealthStatus {
                mode: HealthMode::Ready,
                redis: DependencyState::Up,
                qdrant: DependencyState::Up,
                upstream: DependencyState::Up,
                auth_cache: DependencyState::Up,
            },
        }
    }
}

#[async_trait]
impl HealthProbe for StaticHealthProbe {
    async fn current_status(&self) -> Result<HealthStatus, AppError> {
        Ok(self.status.clone())
    }
}