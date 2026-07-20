//! Vector port — abstract vector index (Qdrant / Pinecone / LanceDB / sqlite-vec)

use async_trait::async_trait;
use super::PortError;

#[derive(Debug, Clone)]
pub struct VectorHit {
    pub id: String,
    pub score: f64,
    pub payload: serde_json::Value,
}

#[async_trait]
pub trait VectorIndex: Send + Sync {
    /// Insert or update a vector with payload
    async fn upsert(&self, id: &str, vector: &[f32], payload: serde_json::Value) -> Result<(), PortError>;

    /// Search top-k similar vectors
    async fn search(&self, vector: &[f32], top_k: usize) -> Result<Vec<VectorHit>, PortError>;

    /// Delete by ID
    async fn delete(&self, id: &str) -> Result<(), PortError>;

    /// Number of indexed vectors
    async fn count(&self) -> Result<u64, PortError>;
}