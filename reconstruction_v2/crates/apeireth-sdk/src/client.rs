use crate::{SessionHandle, MemoryClient, ToolClient};

#[derive(Clone)]
pub struct ApeirethClient {
    base_url: String,
}

impl ApeirethClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
        }
    }

    pub async fn create_session(&self) -> Result<SessionHandle, crate::Error> {
        Ok(SessionHandle::new(self.base_url.clone()))
    }

    pub fn memory(&self) -> MemoryClient {
        MemoryClient::new(self.base_url.clone())
    }

    pub fn tools(&self) -> ToolClient {
        ToolClient::new(self.base_url.clone())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sdk_client_initialization() {
        let client = ApeirethClient::new("http://localhost:8080");
        let _session = client.create_session().await.unwrap();
        let _mem = client.memory();
        let _tools = client.tools();
        assert!(!client.base_url.is_empty());
    }

}

