use super::{ProtocolAdapter, ProtocolError};
use crate::normalized::{ContentPart, NormalizedMessage, NormalizedRequest, NormalizedResponse, Role, ToolCall, Usage};
use async_trait::async_trait;
use serde_json::Value;

pub struct OpenAiAdapter {
    client: reqwest::Client,
    base_url: String,
}

impl Default for OpenAiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAiAdapter {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.openai.com/v1/chat/completions".to_string(),
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

            match msg.role {
                Role::Tool => {
                    for part in &msg.parts {
                        if let ContentPart::ToolResult { tool_call_id, result } = part {
                            messages.push(serde_json::json!({
                                "role": "tool",
                                "tool_call_id": tool_call_id,
                                "content": result,
                            }));
                        }
                    }
                }
                Role::Assistant => {
                    let text = msg.extract_text();
                    let tool_calls = msg.extract_tool_calls();
                    let mut msg_json = serde_json::json!({
                        "role": "assistant",
                        "content": text,
                    });
                    if !tool_calls.is_empty() {
                        let tc_json: Vec<Value> = tool_calls.iter().map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments,
                                }
                            })
                        }).collect();
                        msg_json["tool_calls"] = serde_json::json!(tc_json);
                    }
                    messages.push(msg_json);
                }
                _ => {
                    messages.push(serde_json::json!({
                        "role": role_str,
                        "content": msg.extract_text(),
                    }));
                }
            }
        }


        let mut payload = serde_json::json!({
            "model": req.model,
            "messages": messages,
        });

        if let Some(temp) = req.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }
        if let Some(tokens) = req.max_tokens {
            payload["max_tokens"] = serde_json::json!(tokens);
        }

        if let Some(tools) = &req.tools {
            let tools_json: Vec<Value> = tools.iter().map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            }).collect();
            payload["tools"] = serde_json::json!(tools_json);
        }

        payload
    }

    pub fn parse_response(body: &str) -> Result<NormalizedResponse, ProtocolError> {
        let json: Value = serde_json::from_str(body)?;

        if let Some(err) = json.get("error") {
            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("OpenAI error");
            return Err(ProtocolError::Api { status: 400, message: msg.to_string() });
        }

        let id = json.get("id").and_then(|v| v.as_str()).unwrap_or("unknown_id").to_string();
        let model = json.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4").to_string();

        let choice = json.get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ProtocolError::InvalidProtocol("Missing choices".into()))?;

        let mut parts = Vec::new();
        if let Some(content) = choice.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_str()) {
            if !content.is_empty() {
                parts.push(ContentPart::Text { text: content.to_string() });
            }
        }

        if let Some(tool_calls) = choice.get("message").and_then(|m| m.get("tool_calls")).and_then(|tc| tc.as_array()) {
            for tc in tool_calls {
                let call_id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let fn_name = tc.get("function").and_then(|f| f.get("name")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                let fn_args = tc.get("function").and_then(|f| f.get("arguments")).and_then(|v| v.as_str()).unwrap_or("{}").to_string();
                parts.push(ContentPart::ToolCall {
                    tool_call: ToolCall {
                        id: call_id,
                        name: fn_name,
                        arguments: fn_args,
                    },
                });
            }
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
impl ProtocolAdapter for OpenAiAdapter {
    fn provider_name(&self) -> &'static str {
        "openai"
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
            return Err(ProtocolError::Api { status, message: body });
        }

        Self::parse_response(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_tool_call_parsing() {
        let sample = r#"{
            "id": "chatcmpl-123",
            "model": "gpt-4o",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "shell",
                            "arguments": "{\"command\":\"cargo test\"}"
                        }
                    }]
                }
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 12, "total_tokens": 22 }
        }"#;

        let res = OpenAiAdapter::parse_response(sample).unwrap();
        assert_eq!(res.id, "chatcmpl-123");
        assert_eq!(res.message.parts.len(), 1);
        if let ContentPart::ToolCall { tool_call } = &res.message.parts[0] {
            assert_eq!(tool_call.name, "shell");
            assert_eq!(tool_call.arguments, "{\"command\":\"cargo test\"}");
        } else {
            panic!("Expected ToolCall content part");
        }
    }
}

