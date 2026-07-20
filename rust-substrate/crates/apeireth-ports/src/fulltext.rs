//! Full-text port — BM25 / FTS5 / Tantivy abstract

use async_trait::async_trait;
use super::PortError;

#[derive(Debug, Clone)]
pub struct FullTextHit {
    pub id: String,
    pub score: f64,
    pub snippet: String,
}

#[async_trait]
pub trait FullTextIndex: Send + Sync {
    async fn index(&self, id: &str, text: &str, metadata: serde_json::Value) -> Result<(), PortError>;
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<FullTextHit>, PortError>;
    async fn delete(&self, id: &str) -> Result<(), PortError>;
}