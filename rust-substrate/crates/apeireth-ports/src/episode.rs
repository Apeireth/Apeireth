//! Episode port — append-only event store

use async_trait::async_trait;
use super::PortError;

#[async_trait]
pub trait EpisodeRepository: Send + Sync {
    /// Append an episode (de-dup by fingerprint)
    async fn append(&self, episode: &apeireth_core::Episode) -> Result<bool, PortError>;

    /// Get by ID
    async fn get(&self, eid: &str) -> Result<Option<apeireth_core::Episode>, PortError>;

    /// List by tier, latest first
    async fn list_by_tier(&self, tier: &str, limit: usize) -> Result<Vec<apeireth_core::Episode>, PortError>;

    /// Count
    async fn count(&self) -> Result<u64, PortError>;
}