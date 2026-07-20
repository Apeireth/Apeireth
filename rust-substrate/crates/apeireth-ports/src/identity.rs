//! Identity port

use async_trait::async_trait;
use super::PortError;

#[async_trait]
pub trait IdentityRepository: Send + Sync {
    async fn load(&self) -> Result<Option<apeireth_core::IdentityCard>, PortError>;
    async fn save(&self, card: &apeireth_core::IdentityCard) -> Result<(), PortError>;
    async fn hash(&self) -> Result<String, PortError>;
}