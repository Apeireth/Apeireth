use super::{ProtocolAdapter, ProtocolError};
use crate::normalized::{ContentPart, NormalizedMessage, NormalizedRequest, NormalizedResponse, Role, Usage};
use async_trait::async_trait;
use serde_json::Value;

pub struct MinimaxAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl Default for MinimaxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MinimaxAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: "https://api.minimax.chat/v1/chat/completions".to_string(),
        }
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
        }
    }

    pub fn serialize_request(req: &NormalizedRequest) -> Value {
        let mut messages = Vec::new();
        for msg in &req.messages {
            let role_str = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
                Role::Tool => "tool",
            };
            messages.push(serde_json::json!({
                "role": role_str,
                "content": msg.extract_text(),
            }));
        }

        let mut payload = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "stream": req.stream,
        });

        if let Some(temp) = req.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }
        if let Some(tokens) = req.max_tokens {
            payload["max_tokens"] = serde_json::json!(tokens);
        }

        payload
    }

    pub fn parse_response(body: &str) -> Result<NormalizedResponse, ProtocolError> {
        let json: Value = serde_json::from_str(body)?;

        // Check for MiniMax error structure
        if let Some(err) = json.get("error") {
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown MiniMax API error");
            let code = err.get("http_code").and_then(|c| c.as_str()).and_then(|c| c.parse::<u16>().ok()).unwrap_or(400);
            return Err(ProtocolError::Api {
                status: code,
                message: msg.to_string(),
            });
        }

        let id = json.get("id").and_then(|v| v.as_str()).unwrap_or("unknown_id").to_string();
        let model = json.get("model").and_then(|v| v.as_str()).unwrap_or("MiniMax-Text-01").to_string();

        let choice = json.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ProtocolError::InvalidProtocol("Missing 'choices' in response".into()))?;

        let raw_content = choice.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        // Separate embedded XML comment style <!-- ... --> or <think> CoT if present
        let mut parts = Vec::new();
        if raw_content.contains("<!--") && raw_content.contains("-->") {
            let pieces: Vec<&str> = raw_content.split("-->").collect();
            let cot = pieces[0].replace("<!--", "").trim().to_string();
            parts.push(ContentPart::Reasoning { reasoning: cot });
            if pieces.len() > 1 && !pieces[1].trim().is_empty() {
                parts.push(ContentPart::Text { text: pieces[1].trim().to_string() });
            }
        } else if raw_content.contains("<think>") && raw_content.contains("</think>") {
            let pieces: Vec<&str> = raw_content.split("</think>").collect();
            let cot = pieces[0].replace("<think>", "").trim().to_string();
            parts.push(ContentPart::Reasoning { reasoning: cot });
            if pieces.len() > 1 && !pieces[1].trim().is_empty() {
                parts.push(ContentPart::Text { text: pieces[1].trim().to_string() });
            }
        } else {
            parts.push(ContentPart::Text { text: raw_content });
        }

        let prompt_tokens = json.get("usage").and_then(|u| u.get("prompt_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let completion_tokens = json.get("usage").and_then(|u| u.get("completion_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let total_tokens = json.get("usage").and_then(|u| u.get("total_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        Ok(NormalizedResponse {
            id,
            model,
            message: NormalizedMessage {
                role: Role::Assistant,
                parts,
            },
            usage: Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens,
            },
        })
    }
}

#[async_trait]
impl ProtocolAdapter for MinimaxAdapter {
    fn provider_name(&self) -> &'static str {
        "minimax"
    }

    async fn execute(&self, api_key: &str, request: &NormalizedRequest) -> Result<NormalizedResponse, ProtocolError> {
        let payload = Self::serialize_request(request);
        let resp = self.client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let body = resp.text().await?;

        if status != 200 {
            return Err(ProtocolError::Api {
                status,
                message: body,
            });
        }

        Self::parse_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimax_serialization() {
        let req = NormalizedRequest::new(
            "MiniMax-Text-01",
            vec![
                NormalizedMessage::system("System prompt"),
                NormalizedMessage::user("Hello MiniMax"),
            ],
        );

        let json = MinimaxAdapter::serialize_request(&req);
        assert_eq!(json["model"], "MiniMax-Text-01");
        assert_eq!(json["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_minimax_cot_response_parsing() {
        let sample_resp = r#"{
            "id": "06d99ccae9028a38fe32f2eb7ddf6a41",
            "model": "MiniMax-Text-01",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "<!-- Internal reasoning: 1+1=2 -->\n\nFinal answer is 2"
                }
            }],
            "usage": {
                "prompt_tokens": 15,
                "completion_tokens": 20,
                "total_tokens": 35
            }
        }"#;

        let parsed = MinimaxAdapter::parse_response(sample_resp).unwrap();
        assert_eq!(parsed.id, "06d99ccae9028a38fe32f2eb7ddf6a41");
        assert_eq!(parsed.usage.total_tokens, 35);
        assert_eq!(parsed.message.parts.len(), 2);
        assert_eq!(parsed.message.extract_reasoning(), Some("Internal reasoning: 1+1=2".into()));
        assert_eq!(parsed.message.extract_text(), "Final answer is 2");
    }
}
