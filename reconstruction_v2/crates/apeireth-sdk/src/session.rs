use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct SessionHandle {
    base_url: String,
    session_id: String,
    http: Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    user: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

impl SessionHandle {
    pub(crate) fn new(base_url: String, session_id: String, http: Client) -> Self {
        Self { base_url, session_id, http }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub async fn send_message(&self, msg: &str) -> Result<String, crate::Error> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let req_body = ChatRequest {
            model: "MiniMax-Text-01".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: msg.into(),
            }],
            user: Some(self.session_id.clone()),
        };

        let resp = self.http.post(&url)
            .json(&req_body)
            .send()
            .await
            .map_err(|e| crate::Error::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let err_text = resp.text().await.unwrap_or_default();
            return Err(crate::Error::Api {
                status: status.as_u16(),
                message: err_text,
            });
        }

        let chat_resp: ChatResponse = resp.json()
            .await
            .map_err(|e| crate::Error::Serialization(e.to_string()))?;

        let content = chat_resp.choices.into_iter()
            .next()
            .map(|c| c.message.content)
            .unwrap_or_default();

        Ok(content)
    }
}
