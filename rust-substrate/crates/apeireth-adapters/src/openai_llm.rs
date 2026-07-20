//! OpenAI-compatible LLM adapter (DeepSeek / OpenAI / MiniMax / Ollama 通用)

use async_trait::async_trait;
use apeireth_ports::{LlmClient, LlmRequest, LlmResponse, PortError};
use std::time::Instant;

pub struct OpenAICompatibleLlmClient {
    api_key: String,
    base_url: String,
    http: reqwest::Client,
}

impl OpenAICompatibleLlmClient {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmClient for OpenAICompatibleLlmClient {
    async fn complete(&self, req: &LlmRequest) -> Result<LlmResponse, PortError> {
        let start = Instant::now();
        let body = serde_json::json!({
            "model": req.model,
            "messages": [
                if let Some(sys) = &req.system {
                    serde_json::json!({"role": "system", "content": sys})
                } else {
                    serde_json::json!({"role": "system", "content": "You are a helpful assistant."})
                },
                serde_json::json!({"role": "user", "content": req.prompt}),
            ],
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
        });

        let res = self.http.post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| PortError::Backend(e.to_string()))?;

        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(PortError::Backend(format!("HTTP {}: {}", status, text)));
        }

        let json: serde_json::Value = res.json().await.map_err(|e| PortError::Backend(e.to_string()))?;
        let content = json["choices"][0]["message"]["content"].as_str()
            .ok_or_else(|| PortError::Backend("no content in response".to_string()))?
            .to_string();
        let tokens = json["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32;
        let latency = start.elapsed().as_millis() as u64;

        Ok(LlmResponse {
            content,
            model: req.model.clone(),
            tokens_used: tokens,
            latency_ms: latency,
        })
    }

    async fn stream(&self, _req: &LlmRequest) -> Result<tokio::sync::mpsc::Receiver<String>, PortError> {
        let (_tx, rx) = tokio::sync::mpsc::channel(32);
        Ok(rx)
    }
}