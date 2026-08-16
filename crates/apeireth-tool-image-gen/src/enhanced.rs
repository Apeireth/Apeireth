//! EnhancedImageGen — composed entry point.
// R156 O-5: allow(missing_docs)
#![allow(missing_docs)]

use crate::generators::MockProvider;
use crate::mcp::{ImageGenMcp, McpRequest, McpResponse};
use crate::params::ImageGenParams;
use crate::provider::{ImageGenProvider, ProviderError, ProviderRegistry};
use crate::result::ImageGenResult;

pub struct EnhancedImageGen {
    mcp: ImageGenMcp,
}

impl EnhancedImageGen {
    pub fn new() -> Self {
        Self {
            mcp: ImageGenMcp::new(),
        }
    }
    pub fn registry(&self) -> &ProviderRegistry {
        self.mcp.registry()
    }
    pub fn dispatch_mcp(&self, req: McpRequest) -> McpResponse {
        self.mcp.handle(req)
    }
    /// Quick generate via mock (no API key required).
    pub async fn generate_mock(
        &self,
        params: &ImageGenParams,
    ) -> Result<ImageGenResult, ProviderError> {
        MockProvider::new().generate(params).await
    }
}

impl Default for EnhancedImageGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn generate_mock_works() {
        let e = EnhancedImageGen::new();
        let r = e
            .generate_mock(&ImageGenParams::new("a dog"))
            .await
            .unwrap();
        assert_eq!(r.provider, "mock");
    }

    #[test]
    fn registry_has_4() {
        let e = EnhancedImageGen::new();
        assert_eq!(e.registry().count(), 4);
    }

    #[test]
    fn default_registry_helper() {
        assert_eq!(crate::generators::default_registry().count(), 4);
    }
}
