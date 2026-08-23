use super::{ProtocolAdapter, ProtocolError};
use crate::normalized::{ContentPart, NormalizedMessage, NormalizedRequest, NormalizedResponse, Role, Usage};
use async_trait::async_trait;
use serde_json::Value;

pub struct GeminiAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl GeminiAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
        }
    }

    pub fn serialize_request(req: &NormalizedRequest) -> Value {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for msg in &req.messages {
            match msg.role {
                Role::System => {
                    system_instruction = Some(serde_json::json!({
                        "parts": [{ "text": msg.extract_text() }]
                    }));
                }
                Role::User => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": msg.extract_text() }]
                    }));
                }
                Role::Assistant => {
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{ "text": msg.extract_text() }]
                    }));
                }
                Role::Tool => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{ "text": msg.extract_text() }]
                    }));
                }
            }
        }

        let mut payload = serde_json::json!({
            "contents": contents,
        });

        if let Some(sys) = system_instruction {
            payload["system_instruction"] = sys;
        }

        let mut gen_config = serde_json::json!({});
        if let Some(t) = req.temperature {
            gen_config["temperature"] = serde_json::json!(t);
        }
        if let Some(m) = req.max_tokens {
            gen_config["maxOutputTokens"] = serde_json::json!(m);
        }
        payload["generationConfig"] = gen_config;

        payload
    }

    pub fn parse_response(body: &str) -> Result<NormalizedResponse, ProtocolError> {
        let json: Value = serde_json::from_str(body)?;

        if let Some(err) = json.get("error") {
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Gemini API error");
            return Err(ProtocolError::Api { status: 400, message: msg.to_string() });
        }

        let candidate = json.get("candidates")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ProtocolError::InvalidProtocol("Missing candidates in Gemini response".into()))?;

        let mut text_output = String::new();
        if let Some(parts) = candidate.get("content").and_then(|c| c.get("parts")).and_then(|p| p.as_array()) {
            for part in parts {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    text_output.push_str(text);
                }
            }
        }

        let prompt_tokens = json.get("usageMetadata").and_then(|u| u.get("promptTokenCount")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let completion_tokens = json.get("usageMetadata").and_then(|u| u.get("candidatesTokenCount")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let total_tokens = json.get("usageMetadata").and_then(|u| u.get("totalTokenCount")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        Ok(NormalizedResponse {
            id: "gemini_resp".into(),
            model: "gemini-2.0-flash".into(),
            message: NormalizedMessage {
                role: Role::Assistant,
                parts: vec![ContentPart::Text { text: text_output }],
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
impl ProtocolAdapter for GeminiAdapter {
    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    async fn execute(&self, api_key: &str, request: &NormalizedRequest) -> Result<NormalizedResponse, ProtocolError> {
        let model = if request.model.is_empty() { "gemini-2.0-flash" } else { &request.model };
        let url = format!("{}/{}:generateContent?key={}", self.base_url, model, api_key);
        let payload = Self::serialize_request(request);

        let resp = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = resp.status().as_u16();
        let body = resp.text().await?;

        if status != 200 {
            return Err(ProtocolError::Api { status, message: body });
        }

        Self::parse_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_serialization_and_parsing() {
        let req = NormalizedRequest::new(
            "gemini-2.0-flash",
            vec![
                NormalizedMessage::system("System instruction"),
                NormalizedMessage::user("Hello Gemini"),
            ],
        );

        let json = GeminiAdapter::serialize_request(&req);
        assert!(json.get("system_instruction").is_some());
        assert_eq!(json["contents"].as_array().unwrap().len(), 1);

        let sample_resp = r#"{
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Hello! I am Gemini." }],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 12,
                "candidatesTokenCount": 8,
                "totalTokenCount": 20
            }
        }"#;

        let parsed = GeminiAdapter::parse_response(sample_resp).unwrap();
        assert_eq!(parsed.usage.total_tokens, 20);
        assert_eq!(parsed.message.extract_text(), "Hello! I am Gemini.");
    }
}
