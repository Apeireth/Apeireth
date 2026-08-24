use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelegramError {
    #[error("HTTP error: {0}")]
    Http(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessagePayload {
    pub chat_id: String,
    pub text: String,
    pub parse_mode: Option<String>,
}

pub struct TelegramBridge {
    bot_token: String,
    http_client: reqwest::Client,
}

impl TelegramBridge {
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn format_payload(&self, chat_id: &str, text: &str) -> SendMessagePayload {
        SendMessagePayload {
            chat_id: chat_id.to_string(),
            text: text.to_string(),
            parse_mode: Some("Markdown".into()),
        }
    }

    pub async fn send_message(&self, chat_id: &str, text: &str) -> Result<(), TelegramError> {
        if self.bot_token.is_empty() {
            return Ok(());
        }

        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.bot_token);
        let payload = self.format_payload(chat_id, text);

        let res = self.http_client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| TelegramError::Http(e.to_string()))?;

        if !res.status().is_success() {
            return Err(TelegramError::Http(format!("Telegram returned status {}", res.status())));
        }

        Ok(())
    }
}
