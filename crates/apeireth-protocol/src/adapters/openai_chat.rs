//! OpenAI Chat Completions 协议 adapter
//!
//! **端点**: `POST /v1/chat/completions`
//! **鉴权**: `Authorization: Bearer <api_key>`
//!
//! **协议字段**:
//! - `model` / `messages` / `temperature` / `max_tokens` / `stream` / `stop` / `tools` / `tool_choice`
//! - 响应: `choices[0].message.{content, tool_calls}` / `usage` / `finish_reason`
//!
//! **借鉴 VCP**:
//! - `routes/protocolBridge.js:63-89` `toOpenAiChatTool` 3 步判定
//! - `routes/protocolBridge.js:120-156` `normalizeToolChoice` (auto / required / specific)

use crate::adapter::ProtocolAdapter;
use crate::error::ProtocolError;
use crate::normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedToolChoice, ToolCall,
};
use serde_json::{json, Map, Value};

/// OpenAI Chat Completions adapter
pub struct OpenAiChatAdapter;

impl OpenAiChatAdapter {
    /// 创建
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenAiChatAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolAdapter for OpenAiChatAdapter {
    fn name(&self) -> &'static str {
        "openai_chat"
    }

    fn endpoint_path(&self) -> &'static str {
        "/v1/chat/completions"
    }

    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        if req.model.is_empty() {
            return Err(ProtocolError::missing("model"));
        }
        if req.messages.is_empty() {
            return Err(ProtocolError::missing("messages"));
        }

        let mut body = Map::new();
        body.insert("model".into(), Value::String(req.model.clone()));

        let messages: Vec<Value> = req
            .messages
            .iter()
            .map(|m| {
                let mut msg = Map::new();
                let role_str = match m.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                };
                msg.insert("role".into(), Value::String(role_str.into()));

                // content: 多模态 part 数组 (OpenAI Chat 也支持 array)
                if !m.content.is_empty() {
                    let parts: Vec<Value> = m
                        .content
                        .iter()
                        .map(|p| match p {
                            ContentPart::Text { text } => json!({"type": "text", "text": text}),
                            ContentPart::ImageUrl { url, detail } => {
                                let mut iu = Map::new();
                                iu.insert("url".into(), Value::String(url.clone()));
                                if let Some(d) = detail {
                                    iu.insert("detail".into(), Value::String(d.clone()));
                                }
                                json!({"type": "image_url", "image_url": Value::Object(iu)})
                            }
                        })
                        .collect();
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
                    let tcs: Vec<Value> = m
                        .tool_calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": serde_json::to_string(&tc.arguments)
                                        .unwrap_or_else(|_| "{}".to_string()),
                                }
                            })
                        })
                        .collect();
                    msg.insert("tool_calls".into(), Value::Array(tcs));
                }

                if let Some(id) = &m.tool_call_id {
                    msg.insert("tool_call_id".into(), Value::String(id.clone()));
                }
                if let Some(name) = &m.name {
                    msg.insert("name".into(), Value::String(name.clone()));
                }
                Value::Object(msg)
            })
            .collect();

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
            body.insert(
                "stop".into(),
                Value::Array(req.stop.iter().map(|s| json!(s)).collect()),
            );
        }

        if !req.tools.is_empty() {
            let tools: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    let mut func = Map::new();
                    func.insert("name".into(), Value::String(t.name.clone()));
                    if let Some(d) = &t.description {
                        func.insert("description".into(), Value::String(d.clone()));
                    }
                    if !t.parameters.is_empty() {
                        func.insert("parameters".into(), Value::Object(t.parameters.clone()));
                    }
                    json!({"type": "function", "function": Value::Object(func)})
                })
                .collect();
            body.insert("tools".into(), Value::Array(tools));
        }

        if let Some(tc) = &req.tool_choice {
            let v = match tc {
                NormalizedToolChoice::Auto => Value::String("auto".into()),
                NormalizedToolChoice::None => Value::String("none".into()),
                NormalizedToolChoice::Required => Value::String("required".into()),
                NormalizedToolChoice::Specific { name } => {
                    json!({"type": "function", "function": {"name": name}})
                }
            };
            body.insert("tool_choice".into(), v);
        }

        if !req.metadata.is_empty() {
            let mut m = Map::new();
            for (k, v) in &req.metadata {
                m.insert(k.clone(), Value::String(v.clone()));
            }
            body.insert("metadata".into(), Value::Object(m));
        }

        Ok(Value::Object(body))
    }

    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        let id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing("id"))?
            .to_string();
        let model = raw
            .get("model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing("model"))?
            .to_string();

        let choice = raw
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| ProtocolError::missing("choices[0]"))?;

        let message = choice
            .get("message")
            .ok_or_else(|| ProtocolError::missing("message"))?;
        let content = message
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut tool_calls = Vec::new();
        if let Some(arr) = message.get("tool_calls").and_then(|v| v.as_array()) {
            for tc in arr {
                let id = tc
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let name = tc
                    .get("function")
                    .and_then(|f| f.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let args_str = tc
                    .get("function")
                    .and_then(|f| f.get("arguments"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let arguments =
                    serde_json::from_str(args_str).unwrap_or_else(|_| Value::Object(Map::new()));
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments,
                });
            }
        }

        let finish_reason = choice
            .get("finish_reason")
            .and_then(|v| v.as_str())
            .map(NormalizedFinishReason::from_openai);

        let usage = raw
            .get("usage")
            .map(|u| {
                let p = u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let c = u
                    .get("completion_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                crate::normalized::NormalizedUsage::new(p, c)
            })
            .unwrap_or_default();

        let mut raw_metadata = Map::new();
        if let Some(obj) = raw.get("system_fingerprint").and_then(|v| v.as_str()) {
            raw_metadata.insert("system_fingerprint".into(), Value::String(obj.to_string()));
        }
        if let Some(obj) = raw.get("object").and_then(|v| v.as_str()) {
            raw_metadata.insert("object".into(), Value::String(obj.to_string()));
        }
        if let Some(obj) = raw.get("created").and_then(|v| v.as_i64()) {
            raw_metadata.insert("created".into(), Value::Number(obj.into()));
        }

        Ok(NormalizedResponse {
            id,
            model,
            content,
            finish_reason,
            usage,
            tool_calls,
            raw_metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalized::{NormalizedMessage, NormalizedTool};

    #[test]
    fn request_basic_text_only() {
        let req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("Hello")]);
        let v = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["model"], "gpt-4o");
        assert_eq!(v["messages"][0]["role"], "user");
        assert_eq!(v["messages"][0]["content"], "Hello");
        assert!(v.get("stream").is_none() || v["stream"] == Value::Bool(false));
    }

    #[test]
    fn request_with_temperature_and_max_tokens() {
        let mut req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        req.temperature = Some(0.5);
        req.max_tokens = Some(100);
        req.stream = true;
        let v = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["temperature"], 0.5);
        assert_eq!(v["max_tokens"], 100);
        assert_eq!(v["stream"], true);
    }

    #[test]
    fn request_temperature_out_of_range() {
        let mut req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        req.temperature = Some(3.0);
        let err = OpenAiChatAdapter::new().adapt_request(&req).unwrap_err();
        assert!(matches!(err, ProtocolError::Invalid { .. }));
    }

    #[test]
    fn request_missing_model_or_messages() {
        let req = NormalizedRequest {
            model: String::new(),
            messages: vec![NormalizedMessage::user("hi")],
            ..Default::default()
        };
        assert!(matches!(
            OpenAiChatAdapter::new().adapt_request(&req).unwrap_err(),
            ProtocolError::Missing { .. }
        ));

        let req = NormalizedRequest::new("gpt-4o", vec![]);
        assert!(matches!(
            OpenAiChatAdapter::new().adapt_request(&req).unwrap_err(),
            ProtocolError::Missing { .. }
        ));
    }

    #[test]
    fn request_with_tools_and_choice() {
        let mut req = NormalizedRequest::new(
            "gpt-4o",
            vec![NormalizedMessage::user("What's the weather?")],
        );
        let mut params = Map::new();
        params.insert("type".into(), Value::String("object".into()));
        req.tools.push(
            NormalizedTool::new("get_weather")
                .with_description("Get weather")
                .with_parameters(params),
        );
        req.tool_choice = Some(NormalizedToolChoice::Specific {
            name: "get_weather".into(),
        });
        let v = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["tools"][0]["type"], "function");
        assert_eq!(v["tools"][0]["function"]["name"], "get_weather");
        assert_eq!(v["tool_choice"]["type"], "function");
        assert_eq!(v["tool_choice"]["function"]["name"], "get_weather");
    }

    #[test]
    fn request_tool_calls_in_assistant_message() {
        let tc = ToolCall {
            id: "call_1".into(),
            name: "get_weather".into(),
            arguments: json!({"city": "Beijing"}),
        };
        let m = NormalizedMessage::assistant_with_tool_calls("", vec![tc]);
        let req = NormalizedRequest::new("gpt-4o", vec![m]);
        let v = OpenAiChatAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["messages"][0]["role"], "assistant");
        assert!(
            v["messages"][0].get("content").is_none() || v["messages"][0]["content"] == Value::Null
        );
        assert_eq!(v["messages"][0]["tool_calls"][0]["id"], "call_1");
        assert_eq!(
            v["messages"][0]["tool_calls"][0]["function"]["name"],
            "get_weather"
        );
        // arguments 应该是 JSON string
        assert!(v["messages"][0]["tool_calls"][0]["function"]["arguments"].is_string());
    }

    #[test]
    fn response_text_only() {
        let raw = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "gpt-4o-2024-08-06",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Hello there"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
        });
        let r = OpenAiChatAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.id, "chatcmpl-123");
        assert_eq!(r.model, "gpt-4o-2024-08-06");
        assert_eq!(r.content, "Hello there");
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(r.usage.prompt_tokens, 10);
        assert_eq!(r.usage.completion_tokens, 5);
        assert_eq!(r.usage.total_tokens, 15);
        assert!(r.tool_calls.is_empty());
        assert_eq!(r.raw_metadata["object"], "chat.completion");
    }

    #[test]
    fn response_with_tool_calls() {
        let raw = json!({
            "id": "chatcmpl-456",
            "model": "gpt-4o",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"Beijing\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 20, "completion_tokens": 10, "total_tokens": 30}
        });
        let r = OpenAiChatAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::ToolCalls));
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].id, "call_1");
        assert_eq!(r.tool_calls[0].name, "get_weather");
        assert_eq!(r.tool_calls[0].arguments, json!({"city": "Beijing"}));
    }

    #[test]
    fn response_length_finish_reason() {
        let raw = json!({
            "id": "x", "model": "gpt-4o",
            "choices": [{"message": {"role": "assistant", "content": "..."}, "finish_reason": "length"}],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        });
        let r = OpenAiChatAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Length));
    }

    #[test]
    fn response_missing_id_errors() {
        let raw = json!({"model": "gpt-4o", "choices": []});
        let err = OpenAiChatAdapter::new().adapt_response(&raw).unwrap_err();
        assert!(matches!(err, ProtocolError::Missing { .. }));
    }

    #[test]
    fn response_missing_choices_errors() {
        let raw = json!({"id": "x", "model": "gpt-4o"});
        let err = OpenAiChatAdapter::new().adapt_response(&raw).unwrap_err();
        assert!(matches!(err, ProtocolError::Missing { .. }));
    }
}
