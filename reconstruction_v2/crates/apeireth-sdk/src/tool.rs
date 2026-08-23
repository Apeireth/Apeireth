use reqwest::Client;
use serde::Deserialize;

pub struct ToolClient {
    base_url: String,
    http: Client,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SdkToolDefinition {
    pub name: String,
    pub description: String,
    pub risk_level: Option<String>,
}

#[derive(Deserialize)]
struct ToolListResponse {
    tools: Vec<SdkToolDefinition>,
}

impl ToolClient {
    pub(crate) fn new(base_url: String, http: Client) -> Self {
        Self { base_url, http }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub async fn list_tools(&self) -> Result<Vec<SdkToolDefinition>, crate::Error> {
        let url = format!("{}/v1/tools/list", self.base_url);
        let resp = self.http.get(&url)
            .send()
            .await
            .map_err(|e| crate::Error::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(crate::Error::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            });
        }

        let body: ToolListResponse = resp.json()
            .await
            .map_err(|e| crate::Error::Serialization(e.to_string()))?;

        Ok(body.tools)
    }
}
