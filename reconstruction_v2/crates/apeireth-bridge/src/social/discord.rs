use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiscordError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMessage {
    pub content: String,
    pub username: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct DiscordBridge {
    webhook_url: String,
    bot_name: String,
    http_client: reqwest::Client,
}

impl DiscordBridge {
    pub fn new(webhook_url: impl Into<String>, bot_name: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            bot_name: bot_name.into(),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn format_payload(&self, content: &str) -> DiscordMessage {
        DiscordMessage {
            content: content.to_string(),
            username: Some(self.bot_name.clone()),
            avatar_url: None,
        }
    }

    pub async fn send_message(&self, content: &str) -> Result<(), DiscordError> {
        let payload = self.format_payload(content);
        if self.webhook_url.is_empty() {
            return Ok(());
        }

        let res = self.http_client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| DiscordError::Http(e.to_string()))?;

        if !res.status().is_success() {
            return Err(DiscordError::Http(format!("Discord returned status {}", res.status())));
        }

        Ok(())
    }
}
