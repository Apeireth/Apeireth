use crate::{SessionHandle, MemoryClient, ToolClient};
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct ApeirethClient {
    pub(crate) base_url: String,
    pub(crate) http: Client,
}

impl ApeirethClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            http,
        }
    }

    pub async fn check_health(&self) -> Result<serde_json::Value, crate::Error> {
        let url = format!("{}/health", self.base_url);
        let resp = self.http.get(&url)
            .send()
            .await
            .map_err(|e| crate::Error::Network(e.to_string()))?;
        resp.json().await.map_err(|e| crate::Error::Serialization(e.to_string()))
    }

    pub async fn create_session(&self) -> Result<SessionHandle, crate::Error> {
        let session_id = format!("sess_{}", uuid::Uuid::new_v4());
        Ok(SessionHandle::new(self.base_url.clone(), session_id, self.http.clone()))
    }

    pub fn session(&self, session_id: impl Into<String>) -> SessionHandle {
        SessionHandle::new(self.base_url.clone(), session_id.into(), self.http.clone())
    }

    pub fn memory(&self) -> MemoryClient {
        MemoryClient::new(self.base_url.clone(), self.http.clone())
    }

    pub fn tools(&self) -> ToolClient {
        ToolClient::new(self.base_url.clone(), self.http.clone())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(String),
    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sdk_client_initialization() {
        let client = ApeirethClient::new("http://localhost:8080");
        let session = client.create_session().await.unwrap();
        assert!(!session.session_id().is_empty());
        let _mem = client.memory();
        let _tools = client.tools();
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
