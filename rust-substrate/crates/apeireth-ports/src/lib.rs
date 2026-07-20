//! # Apeireth Ports — Interface Definitions
//!
//! 主人 14:52 "最高深度" — 借鉴 MemoryOS-Rust hexagonal architecture
//!
//! Hexagonal architecture (Ports & Adapters):
//! - **Ports** (this crate) = abstract interfaces
//! - **Adapters** (apeireth-adapters) = concrete implementations
//!   (Qdrant / Tantivy / SQLite / LLM)
//!
//! 为什么 hexagonal:
//! - 主人 11:40 "任意域接入" → 业务逻辑不绑特定实现
//! - 容易测: port 是 trait, mock adapter 即可
//! - 容易换: 把 Qdrant adapter 换成 Pinecone, business code 不动

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod episode;
pub mod note;
pub mod identity;
pub mod vector;
pub mod fulltext;
pub mod wal;
pub mod llm;

pub use episode::EpisodeRepository;
pub use note::NoteRepository;
pub use identity::IdentityRepository;
pub use vector::{VectorIndex, VectorHit};
pub use fulltext::{FullTextIndex, FullTextHit};
pub use wal::WalSink;
pub use llm::{LlmClient, LlmRequest, LlmResponse};

/// Common port error
#[derive(Debug, Error)]
pub enum PortError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Serialization: {0}")]
    Serde(String),
    #[error("Backend error: {0}")]
    Backend(String),
    #[error("Timeout")]
    Timeout,
}

/// Health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub healthy: bool,
    pub backend: String,
    pub latency_ms: u64,
    pub message: String,
}

impl Health {
    pub fn ok(backend: impl Into<String>, latency_ms: u64) -> Self {
        Self {
            healthy: true,
            backend: backend.into(),
            latency_ms,
            message: "healthy".to_string(),
        }
    }
}