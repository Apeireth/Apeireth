//! OpenAI Chat Completions 协议 adapter (v1 era transcription)
//!
//! Transcribed from `crates/_archived/v1.0-legacy/apeireth-protocol/src/adapters/openai_chat.rs`.
//! Provides minimal encode/decode shape used by `bridge.rs`.

use crate::v1_era::adapters::ProtocolAdapter;
use crate::v1_era::error::ProtocolError;
use crate::v1_era::normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedToolChoice, NormalizedUsage, ToolCall,
};
use serde_json::{json, Map, Value};

/// OpenAI Chat Completions adapter (ZST)
pub struct OpenAiChatAdapter;

impl OpenAiChatAdapter {
    pub fn new() -> Self { Self }
}

impl Default for OpenAiChatAdapter {
    fn default() -> Self { Self::new() }
}

fn parse_finish_reason(s: &str) -> NormalizedFinishReason {
    match s {
        "stop" => NormalizedFinishReason::Stop,
        "length" => NormalizedFinishReason::Length,
        "tool_calls" | "function_call" => NormalizedFinishReason::ToolCalls,
        "content_filter" => NormalizedFinishReason::ContentFilter,
        _ => NormalizedFinishReason::Other,
    }
}

fn tool_choice_to_str(tc: &NormalizedToolChoice) -> Value {
    match tc {
        NormalizedToolChoice::Auto => Value::String("auto".into()),
        NormalizedToolChoice::None => Value::String("none".into()),
        NormalizedToolChoice::Required => Value::String("required".into()),
        NormalizedToolChoice::Specific { name } => json!({"type": "function", "function": {"name": name}}),
    }
}

impl ProtocolAdapter for OpenAiChatAdapter {
    fn name(&self) -> &'static str { "openai_chat" }
    fn endpoint_path(&self) -> &'static str { "/v1/chat/completions" }

    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        if req.model.is_empty() {
            return Err(ProtocolError::missing("model"));
        }
        if req.messages.is_empty() {
            return Err(ProtocolError::missing("messages"));
        }
        let mut body = Map::new();
        body.insert("model".into(), Value::String(req.model.clone()));
        let messages: Vec<Value> = req.messages.iter().map(|m| {
            let role_str = match m.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            };
            let mut msg = Map::new();
            msg.insert("role".into(), Value::String(role_str.into()));
            if !m.content.is_empty() {
                let parts: Vec<Value> = m.content.iter().map(|p| match p {
                    ContentPart::Text { text } => json!({"type": "text", "text": text}),
                    ContentPart::ImageUrl { url, detail } => {
                        let mut iu = Map::new();
                        iu.insert("url".into(), Value::String(url.clone()));
                        if let Some(d) = detail {
                            iu.insert("detail".into(), Value::String(d.clone()));
                        }
                        json!({"type": "image_url", "image_url": Value::Object(iu)})
                    }
                }).collect();
                if parts.len() == 1 {
                    if let ContentPart::Text { text } = &m.content[0] {
                        msg.insert("content".into(), Value::String(text.clone()));
                    } else {
                        msg.insert("content".into(), Value::Array(parts));
                    }
                } else {
                    msg.insert("content".into(), Value::Array(parts));
                }
            }
            if !m.tool_calls.is_empty() {
                let tcs: Vec<Value> = m.tool_calls.iter().map(|tc: &ToolCall| {
                    json!({
                        "id": tc.id,
                        "type": "function",
                        "function": {
                            "name": tc.name,
                            "arguments": serde_json::to_string(&tc.arguments)
                                .unwrap_or_else(|_| "{}".to_string()),
                        }
                    })
                }).collect();
                msg.insert("tool_calls".into(), Value::Array(tcs));
            }
            if let Some(id) = &m.tool_call_id {
                msg.insert("tool_call_id".into(), Value::String(id.clone()));
            }
            if let Some(name) = &m.name {
                msg.insert("name".into(), Value::String(name.clone()));
            }
            Value::Object(msg)
        }).collect();
        body.insert("messages".into(), Value::Array(messages));
        if let Some(t) = req.temperature {
            if !(0.0..=2.0).contains(&t) {
                return Err(ProtocolError::invalid(
                    "temperature",
                    format!("must be in [0.0, 2.0], got {}", t),
                ));
            }
            body.insert("temperature".into(), json!(t));
        }
        if let Some(n) = req.max_tokens {
            body.insert("max_tokens".into(), json!(n));
        }
        if req.stream {
            body.insert("stream".into(), Value::Bool(true));
        }
        if !req.stop.is_empty() {
            body.insert("stop".into(), Value::Array(req.stop.iter().cloned().map(Value::String).collect()));
        }
        if !req.tools.is_empty() {
            let tools: Vec<Value> = req.tools.iter().map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            }).collect();
            body.insert("tools".into(), Value::Array(tools));
        }
        if let Some(ref tc) = req.tool_choice {
            body.insert("tool_choice".into(), tool_choice_to_str(tc));
        }
        Ok(Value::Object(body))
    }

    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        let choices = raw.get("choices").and_then(|v| v.as_array())
            .ok_or_else(|| ProtocolError::invalid("choices", "missing choices array"))?;
        let first = choices.first()
            .ok_or_else(|| ProtocolError::invalid("choices", "empty choices array"))?;
        let content = first.get("message").and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let finish_reason = first.get("finish_reason").and_then(|v| v.as_str())
            .map(parse_finish_reason);
        let usage = raw.get("usage").map(|u| NormalizedUsage {
            prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        }).unwrap_or_default();
        Ok(NormalizedResponse {
            id: raw.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            model: raw.get("model").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            content,
            finish_reason,
            usage,
            tool_calls: Vec::new(),
            raw_metadata: Map::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v1_era::normalized::{NormalizedMessage, NormalizedRequest};

    #[test]
    fn name_and_endpoint() {
        let a = OpenAiChatAdapter::new();
        assert_eq!(a.name(), "openai_chat");
        assert_eq!(a.endpoint_path(), "/v1/chat/completions");
    }

    #[test]
    fn encode_basic() {
        let req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        let v = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["model"], "gpt-4o");
        assert!(v["messages"].is_array());
    }

    #[test]
    fn encode_missing_model_errors() {
        let req = NormalizedRequest::new("", vec![NormalizedMessage::user("hi")]);
        let r = OpenAiChatAdapter::new().adapt_request(&req);
        assert!(r.is_err());
    }

    #[test]
    fn encode_missing_messages_errors() {
        let req = NormalizedRequest::new("m", vec![]);
        let r = OpenAiChatAdapter::new().adapt_request(&req);
        assert!(r.is_err());
    }

    #[test]
    fn decode_basic_response() {
        let raw = json!({
            "id": "x", "model": "gpt-4o",
            "choices": [{"message": {"role": "assistant", "content": "hi"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2}
        });
        let r = OpenAiChatAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.content, "hi");
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(r.usage.prompt_tokens, 1);
    }

    #[test]
    fn temperature_range_check() {
        let mut req = NormalizedRequest::new("m", vec![NormalizedMessage::user("hi")]);
        req.temperature = Some(3.0);
        let r = OpenAiChatAdapter::new().adapt_request(&req);
        assert!(r.is_err());
    }

    #[test]
    fn role_mapping() {
        let req = NormalizedRequest::new("m", vec![
            NormalizedMessage::system("sys"),
            NormalizedMessage::user("u"),
            NormalizedMessage::assistant("a"),
            NormalizedMessage {
                role: MessageRole::Tool,
                content: vec![ContentPart::Text { text: "t".into() }],
                tool_calls: vec![],
                tool_call_id: Some("x".into()),
                name: None,
            },
        ]);
        let v = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
        let msgs = v["messages"].as_array().unwrap();
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "user");
        assert_eq!(msgs[2]["role"], "assistant");
        assert_eq!(msgs[3]["role"], "tool");
        assert_eq!(msgs[3]["tool_call_id"], "x");
    }

    #[test]
    fn tool_choice_specific() {
        let mut req = NormalizedRequest::new("m", vec![NormalizedMessage::user("hi")]);
        req.tool_choice = Some(NormalizedToolChoice::Specific { name: "foo".into() });
        let v = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
        assert!(v["tool_choice"].is_object());
    }
}
