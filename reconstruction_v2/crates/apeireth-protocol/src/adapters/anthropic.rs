use super::{ProtocolAdapter, ProtocolError};
use crate::normalized::{ContentPart, NormalizedMessage, NormalizedRequest, NormalizedResponse, Role, ToolCall, Usage};
use async_trait::async_trait;
use serde_json::Value;

pub struct AnthropicAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl Default for AnthropicAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.anthropic.com/v1/messages".to_string(),
        }
    }

    pub fn serialize_request(req: &NormalizedRequest) -> Value {
        let mut system_str = String::new();
        let mut messages = Vec::new();

        for msg in &req.messages {
            match msg.role {
                Role::System => {
                    system_str.push_str(&msg.extract_text());
                    system_str.push('\n');
                }
                Role::User => {
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": msg.extract_text(),
                    }));
                }
                Role::Assistant => {
                    messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": msg.extract_text(),
                    }));
                }
                Role::Tool => {
                    messages.push(serde_json::json!({
                        "role": "user",
                        "content": msg.extract_text(),
                    }));
                }
            }
        }

        let mut payload = serde_json::json!({
            "model": if req.model.is_empty() { "claude-3-5-sonnet-20241022" } else { &req.model },
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(2048),
        });

        if !system_str.trim().is_empty() {
            payload["system"] = serde_json::json!(system_str.trim());
        }

        if let Some(temp) = req.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }

        payload
    }

    pub fn parse_response(body: &str) -> Result<NormalizedResponse, ProtocolError> {
        let json: Value = serde_json::from_str(body)?;

        if let Some(err) = json.get("error") {
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Anthropic API error");
            return Err(ProtocolError::Api { status: 400, message: msg.to_string() });
        }

        let id = json.get("id").and_then(|v| v.as_str()).unwrap_or("msg_unknown").to_string();
        let model = json.get("model").and_then(|v| v.as_str()).unwrap_or("claude-3-5-sonnet").to_string();

        let mut parts = Vec::new();
        if let Some(content_blocks) = json.get("content").and_then(|c| c.as_array()) {
            for block in content_blocks {
                let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                if block_type == "text" {
                    let text = block.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    parts.push(ContentPart::Text { text: text.to_string() });
                } else if block_type == "tool_use" {
                    let call_id = block.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let input = block.get("input").map(|v| v.to_string()).unwrap_or_else(|| "{}".to_string());
                    parts.push(ContentPart::ToolCall {
                        tool_call: ToolCall {
                            id: call_id,
                            name,
                            arguments: input,
                        },
                    });
                }
            }
        }

        let prompt_tokens = json.get("usage").and_then(|u| u.get("input_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let completion_tokens = json.get("usage").and_then(|u| u.get("output_tokens")).and_then(|v| v.as_u64()).unwrap_or(0) as u32;

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
                total_tokens: prompt_tokens + completion_tokens,
            },
        })
    }
}

#[async_trait]
impl ProtocolAdapter for AnthropicAdapter {
    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    async fn execute(&self, api_key: &str, request: &NormalizedRequest) -> Result<NormalizedResponse, ProtocolError> {
        let payload = Self::serialize_request(request);
        let resp = self.client
            .post(&self.base_url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
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
    fn test_anthropic_serialization_and_parsing() {
        let req = NormalizedRequest::new(
            "claude-3-5-sonnet-20241022",
            vec![
                NormalizedMessage::system("System anchor"),
                NormalizedMessage::user("Hello Claude"),
            ],
        );

        let json = AnthropicAdapter::serialize_request(&req);
        assert_eq!(json["system"], "System anchor");
        assert_eq!(json["messages"].as_array().unwrap().len(), 1);

        let sample_resp = r#"{
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{ "type": "text", "text": "Hello! How can I assist you today?" }],
            "usage": { "input_tokens": 20, "output_tokens": 10 }
        }"#;

        let parsed = AnthropicAdapter::parse_response(sample_resp).unwrap();
        assert_eq!(parsed.id, "msg_01XFDUDYJgAACzvnptvVoYEL");
        assert_eq!(parsed.usage.total_tokens, 30);
        assert_eq!(parsed.message.extract_text(), "Hello! How can I assist you today?");
    }
}
