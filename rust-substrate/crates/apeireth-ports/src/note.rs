//! Note port — abstract knowledge store

use async_trait::async_trait;
use super::PortError;

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn upsert(&self, note: &apeireth_core::Note) -> Result<bool, PortError>;
    async fn get(&self, nid: &str) -> Result<Option<apeireth_core::Note>, PortError>;
    async fn list(&self, limit: usize) -> Result<Vec<apeireth_core::Note>, PortError>;
    async fn forget(&self, nid: &str) -> Result<(), PortError>;
}