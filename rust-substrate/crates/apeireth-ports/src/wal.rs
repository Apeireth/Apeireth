//! WAL port — Write-Ahead Log sink

use async_trait::async_trait;
use super::PortError;

#[async_trait]
pub trait WalSink: Send + Sync {
    async fn append(&self, payload: &[u8]) -> Result<u64, PortError>;
    async fn replay<F>(&self, handler: F) -> Result<u64, PortError>
    where
        F: FnMut(&[u8]) -> Result<(), PortError> + Send;
}