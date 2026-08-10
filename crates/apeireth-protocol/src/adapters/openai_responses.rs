//! OpenAI Responses API 协议 adapter
//!
//! **端点**: `POST /v1/responses`
//! **鉴权**: `Authorization: Bearer <api_key>`
//!
//! **协议差异** (vs Chat Completions):
//! - **顶层 `instructions`** 替代 messages 里的 system (类似 Anthropic)
//! - **`input`** 字段 (字符串或数组) 替代 `messages`
//! - **响应 `output`** 是结构化数组 (跟 Chat Completions 的 choices 不同)
//! - **顶层 `output_text`** 字段 (server 拼出来的纯文本)
//!
//! **借鉴 VCP**:
//! - `routes/protocolBridge.js:210-220` `buildImmediateResponsesPayload` 的 output
//!   数组结构 (item with `content[0].text`)
//! - 字段命名约定: `output_text` 顶层 + `output[0].content[0].text` (VCP 真代码)

use crate::adapter::ProtocolAdapter;
use crate::error::ProtocolError;
use crate::normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedToolChoice, ToolCall,
};
use serde_json::{json, Map, Value};

/// OpenAI Responses API adapter
pub struct OpenAiResponsesAdapter;

impl OpenAiResponsesAdapter {
    /// 创建
    pub fn new() -> Self {
        Self
    }
}

impl Default for OpenAiResponsesAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolAdapter for OpenAiResponsesAdapter {
    fn name(&self) -> &'static str {
        "openai_responses"
    }

    fn endpoint_path(&self) -> &'static str {
        "/v1/responses"
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

        // 借鉴 VCP: system 提到顶层 instructions
        let mut instructions: Option<String> = None;
        let mut non_system: Vec<&crate::normalized::NormalizedMessage> = Vec::new();
        for m in &req.messages {
            if m.role == MessageRole::System {
                if instructions.is_none() {
                    instructions = Some(ContentPart::join_text(&m.content));
                }
                // 多段 system: 拼起来
                else {
                    let prev = instructions.take().unwrap_or_default();
                    let add = ContentPart::join_text(&m.content);
                    instructions = Some(format!("{}\n{}", prev, add));
                }
            } else {
                non_system.push(m);
            }
        }
        if let Some(ins) = instructions {
            body.insert("instructions".into(), Value::String(ins));
        }

        // 借鉴 VCP `routes/protocolBridge.js:210-220` buildImmediateResponsesPayload:
        // output 是结构化数组; input 归一化为 OpenAI Responses 的 input item 数组
        let input_items: Vec<Value> = non_system
            .iter()
            .map(|m| {
                let mut item = Map::new();
                match m.role {
                    MessageRole::User => {
                        let parts: Vec<Value> = m
                            .content
                            .iter()
                            .map(|p| match p {
                                ContentPart::Text { text } => {
                                    json!({"type": "input_text", "text": text})
                                }
                                ContentPart::ImageUrl { url, detail } => {
                                    let mut iu = Map::new();
                                    iu.insert("url".into(), Value::String(url.clone()));
                                    if let Some(d) = detail {
                                        iu.insert("detail".into(), Value::String(d.clone()));
                                    }
                                    json!({"type": "input_image", "image_url": Value::Object(iu)})
                                }
                            })
                            .collect();
                        item.insert("role".into(), Value::String("user".into()));
                        item.insert("content".into(), Value::Array(parts));
                    }
                    MessageRole::Assistant => {
                        // assistant → output_text + function_call items
                        let text = ContentPart::join_text(&m.content);
                        if !text.is_empty() {
                            // 注: 简单实现只输出 1 个 message item
                            let mut inner = Map::new();
                            inner.insert("type".into(), Value::String("output_text".into()));
                            inner.insert("text".into(), Value::String(text));
                            inner.insert("role".into(), Value::String("assistant".into()));
                            // 直接作为 input item 即可 (Responses API 接受 history)
                            return Value::Object({
                                let mut outer = Map::new();
                                outer.insert("role".into(), Value::String("assistant".into()));
                                outer.insert(
                                    "content".into(),
                                    Value::Array(vec![Value::Object(inner)]),
                                );
                                outer
                            });
                        }
                        item.insert("role".into(), Value::String("assistant".into()));
                        item.insert("content".into(), Value::Array(Vec::new()));
                    }
                    MessageRole::Tool => {
                        // tool → function_call_output
                        let output_text = ContentPart::join_text(&m.content);
                        item.insert("type".into(), Value::String("function_call_output".into()));
                        if let Some(id) = &m.tool_call_id {
                            item.insert("call_id".into(), Value::String(id.clone()));
                        }
                        item.insert("output".into(), Value::String(output_text));
                    }
                    MessageRole::System => {
                        // 已经在外层 instructions 处理过, 不应到这里
                    }
                }
                Value::Object(item)
            })
            .collect();

        body.insert("input".into(), Value::Array(input_items));

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
            body.insert("max_output_tokens".into(), json!(n));
        }

        if req.stream {
            body.insert("stream".into(), Value::Bool(true));
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
                    if t.strict {
                        func.insert("strict".into(), Value::Bool(true));
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

        // parallel_tool_calls 借鉴 VCP protocolBridge.js:169-171
        if let Some(p) = req.metadata.get("parallel_tool_calls") {
            if let Ok(b) = p.parse::<bool>() {
                body.insert("parallel_tool_calls".into(), Value::Bool(b));
            }
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

        // 借鉴 VCP `buildImmediateResponsesPayload`:
        // output 是数组,每个 item 有 type + content[0].text
        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        if let Some(output) = raw.get("output").and_then(|v| v.as_array()) {
            for item in output {
                let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match item_type {
                    "message" => {
                        if let Some(arr) = item.get("content").and_then(|v| v.as_array()) {
                            for part in arr {
                                let t = part.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                if t == "output_text" {
                                    if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                        if !content.is_empty() {
                                            content.push('\n');
                                        }
                                        content.push_str(text);
                                    }
                                }
                            }
                        }
                    }
                    "function_call" => {
                        let id = item
                            .get("call_id")
                            .or_else(|| item.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let name = item
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        // OpenAI Responses 的 arguments 是 JSON string, 跟 OpenAI Chat 一致
                        let arguments = item
                            .get("arguments")
                            .map(|v| {
                                if let Some(s) = v.as_str() {
                                    serde_json::from_str(s)
                                        .unwrap_or_else(|_| Value::Object(Map::new()))
                                } else {
                                    v.clone()
                                }
                            })
                            .unwrap_or_else(|| Value::Object(Map::new()));
                        tool_calls.push(ToolCall {
                            id,
                            name,
                            arguments,
                        });
                    }
                    _ => {}
                }
            }
        }

        // 优先用顶层 output_text (VCP 拼出来的纯文本)
        if let Some(top_text) = raw.get("output_text").and_then(|v| v.as_str()) {
            if !top_text.is_empty() && content.is_empty() {
                content = top_text.to_string();
            }
        }

        let finish_reason = match raw.get("status").and_then(|v| v.as_str()) {
            Some("completed") => Some(NormalizedFinishReason::Stop),
            Some("incomplete") => Some(NormalizedFinishReason::Length),
            Some("failed") => Some(NormalizedFinishReason::Other),
            Some("in_progress") => Some(NormalizedFinishReason::Other),
            _ => raw
                .get("incomplete_details")
                .and_then(|d| d.get("reason"))
                .and_then(|v| v.as_str())
                .map(|_| NormalizedFinishReason::Length),
        };

        let usage = raw
            .get("usage")
            .map(|u| {
                let p = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let c = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                crate::normalized::NormalizedUsage::new(p, c)
            })
            .unwrap_or_default();

        let mut raw_metadata = Map::new();
        if let Some(s) = raw.get("status").and_then(|v| v.as_str()) {
            raw_metadata.insert("status".into(), Value::String(s.to_string()));
        }
        if let Some(o) = raw.get("object").and_then(|v| v.as_str()) {
            raw_metadata.insert("object".into(), Value::String(o.to_string()));
        }
        if let Some(c) = raw.get("created_at").and_then(|v| v.as_i64()) {
            raw_metadata.insert("created_at".into(), Value::Number(c.into()));
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
    use serde_json::Map;

    #[test]
    fn request_system_promoted_to_instructions() {
        // 借鉴 VCP protocolBridge.js:210-220: system → 顶层 instructions
        let req = NormalizedRequest::new(
            "gpt-4o",
            vec![
                NormalizedMessage::system("You are helpful"),
                NormalizedMessage::user("hi"),
            ],
        );
        let v = OpenAiResponsesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["instructions"], "You are helpful");
        // input 不含 system
        let input = v["input"].as_array().unwrap();
        assert_eq!(input.len(), 1);
        assert_eq!(input[0]["role"], "user");
    }

    #[test]
    fn request_multi_system_concatenated() {
        // 借鉴 VCP: 多段 system 拼起来
        let req = NormalizedRequest::new(
            "gpt-4o",
            vec![
                NormalizedMessage::system("System A"),
                NormalizedMessage::system("System B"),
                NormalizedMessage::user("hi"),
            ],
        );
        let v = OpenAiResponsesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["instructions"], "System A\nSystem B");
    }

    #[test]
    fn request_input_uses_input_text() {
        // 借鉴 VCP: input item 的 content 用 input_text 不是 text
        let req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("Hello")]);
        let v = OpenAiResponsesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["input"][0]["content"][0]["type"], "input_text");
        assert_eq!(v["input"][0]["content"][0]["text"], "Hello");
    }

    #[test]
    fn request_tool_as_function_call_output() {
        // 借鉴 VCP: tool message → function_call_output
        let req = NormalizedRequest::new(
            "gpt-4o",
            vec![
                NormalizedMessage::user("weather?"),
                NormalizedMessage::tool_result("call_1", Some("get_weather".into()), "sunny"),
            ],
        );
        let v = OpenAiResponsesAdapter::new().adapt_request(&req).unwrap();
        let input = v["input"].as_array().unwrap();
        assert_eq!(input[1]["type"], "function_call_output");
        assert_eq!(input[1]["call_id"], "call_1");
        assert_eq!(input[1]["output"], "sunny");
    }

    #[test]
    fn request_temperature_out_of_range() {
        let mut req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        req.temperature = Some(5.0);
        assert!(matches!(
            OpenAiResponsesAdapter::new()
                .adapt_request(&req)
                .unwrap_err(),
            ProtocolError::Invalid { .. }
        ));
    }

    #[test]
    fn request_with_tools() {
        let mut req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        let mut p = Map::new();
        p.insert("type".into(), Value::String("object".into()));
        req.tools.push(NormalizedTool::new("fn").with_parameters(p));
        req.tool_choice = Some(NormalizedToolChoice::Required);
        let v = OpenAiResponsesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["tools"][0]["type"], "function");
        assert_eq!(v["tools"][0]["function"]["name"], "fn");
        assert_eq!(v["tool_choice"], "required");
    }

    #[test]
    fn request_parallel_tool_calls_via_metadata() {
        // 借鉴 VCP protocolBridge.js:169-171
        let mut req = NormalizedRequest::new("gpt-4o", vec![NormalizedMessage::user("hi")]);
        req.metadata
            .insert("parallel_tool_calls".into(), "true".into());
        let v = OpenAiResponsesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["parallel_tool_calls"], true);
    }

    #[test]
    fn response_with_output_array_message() {
        // 借鉴 VCP buildImmediateResponsesPayload: output[0].content[0].text
        let raw = json!({
            "id": "resp_123",
            "object": "response",
            "created_at": 1700000000,
            "model": "gpt-4o",
            "status": "completed",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{
                    "type": "output_text",
                    "text": "Hello back"
                }]
            }],
            "usage": {"input_tokens": 5, "output_tokens": 3, "total_tokens": 8}
        });
        let r = OpenAiResponsesAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.id, "resp_123");
        assert_eq!(r.content, "Hello back");
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(r.usage.prompt_tokens, 5);
        assert_eq!(r.usage.completion_tokens, 3);
    }

    #[test]
    fn response_with_function_call() {
        let raw = json!({
            "id": "resp_456",
            "model": "gpt-4o",
            "status": "completed",
            "output": [{
                "type": "function_call",
                "call_id": "call_1",
                "name": "get_weather",
                "arguments": "{\"city\":\"Beijing\"}"
            }],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let r = OpenAiResponsesAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].id, "call_1");
        assert_eq!(r.tool_calls[0].name, "get_weather");
        assert_eq!(r.tool_calls[0].arguments, json!({"city": "Beijing"}));
    }

    #[test]
    fn response_status_incomplete_maps_to_length() {
        let raw = json!({
            "id": "x", "model": "gpt-4o",
            "status": "incomplete",
            "output": [], "usage": {}
        });
        let r = OpenAiResponsesAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Length));
    }

    #[test]
    fn response_uses_top_level_output_text_when_no_output() {
        // 借鉴 VCP buildImmediateResponsesPayload: 顶层 output_text 兜底
        let raw = json!({
            "id": "x", "model": "gpt-4o",
            "status": "completed",
            "output_text": "Top level text",
            "output": [],
            "usage": {}
        });
        let r = OpenAiResponsesAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.content, "Top level text");
    }

    #[test]
    fn response_missing_id_errors() {
        let raw = json!({"model": "gpt-4o", "output": []});
        let err = OpenAiResponsesAdapter::new()
            .adapt_response(&raw)
            .unwrap_err();
        assert!(matches!(err, ProtocolError::Missing { .. }));
    }
}
