//! Qdrant vector adapter (借鉴 qdrant/qdrant 架构)
//!
//! 主人 13:47 "最高深度" — vector search 是 L3 核心

use async_trait::async_trait;
use apeireth_ports::{VectorIndex, VectorHit, PortError};

pub struct QdrantVectorIndex {
    client: qdrant_client::Qdrant,
    collection: String,
}

impl QdrantVectorIndex {
    pub async fn connect(url: &str, collection: impl Into<String>) -> Result<Self, PortError> {
        let client = qdrant_client::Qdrant::from_url(url)
            .build()
            .map_err(|e| PortError::Backend(e.to_string()))?;
        Ok(Self {
            client,
            collection: collection.into(),
        })
    }
}

#[async_trait]
impl VectorIndex for QdrantVectorIndex {
    async fn upsert(&self, _id: &str, _vector: &[f32], _payload: serde_json::Value) -> Result<(), PortError> {
        // TODO: implement qdrant upsert
        Err(PortError::Backend("not yet implemented".to_string()))
    }

    async fn search(&self, _vector: &[f32], _top_k: usize) -> Result<Vec<VectorHit>, PortError> {
        Err(PortError::Backend("not yet implemented".to_string()))
    }

    async fn delete(&self, _id: &str) -> Result<(), PortError> {
        Err(PortError::Backend("not yet implemented".to_string()))
    }

    async fn count(&self) -> Result<u64, PortError> {
        Err(PortError::Backend("not yet implemented".to_string()))
    }
}