//! Anthropic Messages API adapter (v1 era transcription)
//!
//! Transcribed from `crates/_archived/v1.0-legacy/apeireth-protocol/src/adapters/anthropic_messages.rs`.

use crate::v1_era::adapters::ProtocolAdapter;
use crate::v1_era::error::ProtocolError;
use crate::v1_era::normalized::{
    ContentPart, MessageRole, NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use serde_json::{json, Map, Value};

/// Anthropic Messages adapter (ZST)
pub struct AnthropicMessagesAdapter;

impl AnthropicMessagesAdapter {
    pub fn new() -> Self { Self }
}

impl Default for AnthropicMessagesAdapter {
    fn default() -> Self { Self::new() }
}

impl ProtocolAdapter for AnthropicMessagesAdapter {
    fn name(&self) -> &'static str { "anthropic_messages" }
    fn endpoint_path(&self) -> &'static str { "/v1/messages" }

    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        if req.model.is_empty() {
            return Err(ProtocolError::missing("model"));
        }
        if req.messages.is_empty() {
            return Err(ProtocolError::missing("messages"));
        }
        // Anthropic requires max_tokens
        if req.max_tokens.is_none() {
            return Err(ProtocolError::missing("max_tokens"));
        }
        let mut body = Map::new();
        body.insert("model".into(), Value::String(req.model.clone()));
        body.insert("max_tokens".into(), json!(req.max_tokens.unwrap()));

        // Anthropic separates system message
        let mut system_text: Option<String> = None;
        let mut non_system: Vec<&crate::v1_era::normalized::NormalizedMessage> = Vec::new();
        for m in &req.messages {
            if m.role == MessageRole::System {
                let t: String = m.content.iter().filter_map(|p| match p {
                    ContentPart::Text { text } => Some(text.as_str()),
                    _ => None,
                }).collect::<Vec<_>>().join("");
                system_text = Some(match system_text {
                    Some(prev) => format!("{prev}\n{t}"),
                    None => t,
                });
            } else {
                non_system.push(m);
            }
        }
        if let Some(s) = system_text {
            body.insert("system".into(), Value::String(s));
        }

        let messages: Vec<Value> = non_system.iter().map(|m| {
            let role_str = match m.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "user", // tool results come as user
                MessageRole::System => "user", // already stripped
            };
            let mut msg = Map::new();
            msg.insert("role".into(), Value::String(role_str.into()));
            let parts: Vec<Value> = m.content.iter().map(|p| match p {
                ContentPart::Text { text } => json!({"type": "text", "text": text}),
                ContentPart::ImageUrl { url, .. } => {
                    // Anthropic uses different image format; simplified stub
                    json!({"type": "image", "source": {"type": "url", "url": url}})
                }
            }).collect();
            msg.insert("content".into(), Value::Array(parts));
            Value::Object(msg)
        }).collect();
        body.insert("messages".into(), Value::Array(messages));

        if let Some(t) = req.temperature {
            body.insert("temperature".into(), json!(t));
        }
        if !req.tools.is_empty() {
            let tools: Vec<Value> = req.tools.iter().map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            }).collect();
            body.insert("tools".into(), Value::Array(tools));
        }
        Ok(Value::Object(body))
    }

    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        let mut content = String::new();
        if let Some(blocks) = raw.get("content").and_then(|v| v.as_array()) {
            for block in blocks {
                if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                    if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                        content.push_str(text);
                    }
                }
            }
        }
        let usage = raw.get("usage").map(|u| NormalizedUsage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: 0,
        }).unwrap_or_default();
        Ok(NormalizedResponse {
            id: raw.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            model: raw.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            content,
            finish_reason: None,
            usage,
            tool_calls: Vec::new(),
            raw_metadata: Map::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_era::normalized::NormalizedMessage;

    #[test]
    fn name_and_endpoint() {
        let a = AnthropicMessagesAdapter::new();
        assert_eq!(a.name(), "anthropic_messages");
        assert_eq!(a.endpoint_path(), "/v1/messages");
    }

    #[test]
    fn encode_basic() {
        let mut req = NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        req.max_tokens = Some(1024);
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["model"], "claude-sonnet-4");
        assert!(v["messages"].is_array());
    }

    #[test]
    fn encode_requires_max_tokens() {
        let req = NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        let r = AnthropicMessagesAdapter::new().adapt_request(&req);
        assert!(r.is_err());
    }

    #[test]
    fn encode_system_separated() {
        let mut req = NormalizedRequest::new("claude-sonnet-4", vec![
            NormalizedMessage::system("you are helpful"),
            NormalizedMessage::user("hi"),
        ]);
        req.max_tokens = Some(1024);
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["system"], "you are helpful");
        let msgs = v["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["role"], "user");
    }

    #[test]
    fn decode_basic_response() {
        let raw = json!({
            "id": "msg_x", "type": "message", "role": "assistant",
            "model": "claude-sonnet-4",
            "content": [{"type": "text", "text": "hello"}],
            "stop_reason": "end_turn", "usage": {"input_tokens": 1, "output_tokens": 2}
        });
        let r = AnthropicMessagesAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.content, "hello");
        assert_eq!(r.usage.prompt_tokens, 1);
    }
}
