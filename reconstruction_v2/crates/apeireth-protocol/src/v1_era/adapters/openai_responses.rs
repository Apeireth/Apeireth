//! OpenAI Responses API adapter (v1 era transcription)
//!
//! Transcribed from `crates/_archived/v1.0-legacy/apeireth-protocol/src/adapters/openai_responses.rs`.
//! Provides minimal encode/decode shape for the OpenAI Responses API format.

use crate::v1_era::adapters::ProtocolAdapter;
use crate::v1_era::error::ProtocolError;
use crate::v1_era::normalized::{
    ContentPart, MessageRole, NormalizedRequest, NormalizedResponse, NormalizedUsage,
};
use serde_json::{json, Map, Value};

/// OpenAI Responses API adapter (ZST)
pub struct OpenAiResponsesAdapter;

impl OpenAiResponsesAdapter {
    pub fn new() -> Self { Self }
}

impl Default for OpenAiResponsesAdapter {
    fn default() -> Self { Self::new() }
}

fn message_to_response_input(role: &MessageRole, content: &[ContentPart]) -> Value {
    let role_str = match role {
        MessageRole::System => "developer",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    };
    let mut item = Map::new();
    item.insert("role".into(), Value::String(role_str.into()));
    if !content.is_empty() {
        let parts: Vec<Value> = content.iter().map(|p| match p {
            ContentPart::Text { text } => json!({"type": "input_text", "text": text}),
            ContentPart::ImageUrl { url, detail } => {
                let mut iu = Map::new();
                iu.insert("url".into(), Value::String(url.clone()));
                if let Some(d) = detail {
                    iu.insert("detail".into(), Value::String(d.clone()));
                }
                json!({"type": "input_image", "image_url": Value::Object(iu)})
            }
        }).collect();
        item.insert("content".into(), Value::Array(parts));
    }
    Value::Object(item)
}

impl ProtocolAdapter for OpenAiResponsesAdapter {
    fn name(&self) -> &'static str { "openai_responses" }
    fn endpoint_path(&self) -> &'static str { "/v1/responses" }

    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        if req.model.is_empty() {
            return Err(ProtocolError::missing("model"));
        }
        if req.messages.is_empty() {
            return Err(ProtocolError::missing("messages"));
        }
        let mut body = Map::new();
        body.insert("model".into(), Value::String(req.model.clone()));

        let input: Vec<Value> = req.messages.iter().map(|m| {
            message_to_response_input(&m.role, &m.content)
        }).collect();
        body.insert("input".into(), Value::Array(input));

        if let Some(t) = req.temperature {
            body.insert("temperature".into(), json!(t));
        }
        if let Some(n) = req.max_tokens {
            body.insert("max_output_tokens".into(), json!(n));
        }
        if req.stream {
            body.insert("stream".into(), Value::Bool(true));
        }
        if !req.tools.is_empty() {
            let tools: Vec<Value> = req.tools.iter().map(|t| {
                json!({
                    "type": "function",
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                })
            }).collect();
            body.insert("tools".into(), Value::Array(tools));
        }
        Ok(Value::Object(body))
    }

    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        let mut content = String::new();
        if let Some(output_arr) = raw.get("output").and_then(|v| v.as_array()) {
            for out_item in output_arr {
                if let Some(content_arr) = out_item.get("content").and_then(|v| v.as_array()) {
                    for c in content_arr {
                        if c.get("type").and_then(|v| v.as_str()) == Some("output_text") {
                            if let Some(text) = c.get("text").and_then(|v| v.as_str()) {
                                content.push_str(text);
                            }
                        }
                    }
                }
            }
        }
        let usage = raw.get("usage").map(|u| NormalizedUsage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
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
        let a = OpenAiResponsesAdapter::new();
        assert_eq!(a.name(), "openai_responses");
        assert_eq!(a.endpoint_path(), "/v1/responses");
    }

    #[test]
    fn encode_basic() {
        let req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        let v = OpenAiResponsesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["model"], "gpt-4o");
        assert!(v["input"].is_array());
    }

    #[test]
    fn decode_basic_response() {
        let raw = json!({
            "id": "resp_x", "model": "gpt-4o",
            "status": "completed",
            "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "hello"}]}],
            "usage": {"input_tokens": 1, "output_tokens": 2}
        });
        let r = OpenAiResponsesAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.content, "hello");
        assert_eq!(r.usage.prompt_tokens, 1);
        assert_eq!(r.usage.completion_tokens, 2);
    }
}
