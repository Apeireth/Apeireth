//! Anthropic Messages API 协议 adapter
//!
//! **端点**: `POST /v1/messages`
//! **鉴权**: `x-api-key: <api_key>` + `anthropic-version: 2023-06-01`
//!
//! **协议差异** (vs OpenAI Chat):
//! - **顶层 `system`** 字段 (OpenAI 是 messages 第一个)
//! - **`max_tokens` 必填** (Anthropic 必须显式给)
//! - **`messages` 不含 system** 角色
//! - **tool_use** 块 (id `toolu_xxx`) 替代 `tool_calls` 数组
//! - **tool_result** 块 (with `tool_use_id`) 替代 `role: "tool"` + `tool_call_id`
//! - 响应 `content` 是块数组 (text / tool_use), 不是 `choices`
//! - 响应 `stop_reason`: `end_turn` / `max_tokens` / `stop_sequence` / `tool_use`
//!
//! **借鉴 VCP**:
//! - `routes/protocolBridge.js:120-156` `normalizeToolChoice` (Anthropic 不支持 tool_choice
//!   "none" 跟 "auto", 用 tools 列表控制)
//! - `chatCompletionHandler.js:286-323` `isToolResultError` 的 5 字段语义

use crate::adapter::ProtocolAdapter;
use crate::error::ProtocolError;
use crate::normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedTool, NormalizedToolChoice, ToolCall,
};
use serde_json::{json, Map, Value};

/// Anthropic Messages API adapter
pub struct AnthropicMessagesAdapter;

impl AnthropicMessagesAdapter {
    /// 创建
    pub fn new() -> Self {
        Self
    }

    /// 校验 max_tokens 必填 (Anthropic 强制)
    fn ensure_max_tokens(req: &NormalizedRequest) -> Result<u32, ProtocolError> {
        match req.max_tokens {
            Some(n) if n > 0 => Ok(n),
            _ => Err(ProtocolError::invalid(
                "max_tokens",
                "Anthropic Messages API requires max_tokens (must be > 0)",
            )),
        }
    }
}

impl Default for AnthropicMessagesAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolAdapter for AnthropicMessagesAdapter {
    fn name(&self) -> &'static str {
        "anthropic_messages"
    }

    fn endpoint_path(&self) -> &'static str {
        "/v1/messages"
    }

    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        if req.model.is_empty() {
            return Err(ProtocolError::missing("model"));
        }
        if req.messages.is_empty() {
            return Err(ProtocolError::missing("messages"));
        }

        let max_tokens = Self::ensure_max_tokens(req)?;

        let mut body = Map::new();
        body.insert("model".into(), Value::String(req.model.clone()));
        body.insert("max_tokens".into(), json!(max_tokens));

        // 借鉴 VCP: system 提到顶层
        let mut system_text: Option<String> = None;
        let mut non_system: Vec<&crate::normalized::NormalizedMessage> = Vec::new();
        for m in &req.messages {
            if m.role == MessageRole::System {
                let t = ContentPart::join_text(&m.content);
                system_text = Some(match system_text {
                    Some(prev) => format!("{}\n{}", prev, t),
                    None => t,
                });
            } else {
                non_system.push(m);
            }
        }
        if let Some(s) = system_text {
            body.insert("system".into(), Value::String(s));
        }

        // 借鉴 VCP `routes/protocolBridge.js:120-156`:
        // Anthropic 不支持 tool_choice = "none" / "auto" / "specific"
        // 通过 tools 列表控制 (VCP 真代码: tool_choice === 'none' 时去掉 tools)
        let mut tools = req.tools.clone();
        let tool_choice_v: Option<Value> = match &req.tool_choice {
            Some(NormalizedToolChoice::None) => {
                tools.clear();
                None
            }
            Some(NormalizedToolChoice::Auto) => None, // Anthropic 默认就是 auto
            Some(NormalizedToolChoice::Required) => {
                return Err(ProtocolError::unsupported(
                    "Anthropic tool_choice=required (Anthropic 不支持, 请用 tools 列表控制)",
                ));
            }
            Some(NormalizedToolChoice::Specific { name }) => {
                // Anthropic 没有 specific tool_choice, 退化为只保留 name 匹配的 tool
                tools.retain(|t| t.name == *name);
                None
            }
            None => None,
        };

        // messages: user / assistant / tool
        let messages: Vec<Value> = non_system
            .iter()
            .map(|m| {
                let mut msg = Map::new();
                match m.role {
                    MessageRole::User => {
                        msg.insert("role".into(), Value::String("user".into()));
                        msg.insert("content".into(), self.build_user_content(m));
                    }
                    MessageRole::Assistant => {
                        msg.insert("role".into(), Value::String("assistant".into()));
                        let mut blocks: Vec<Value> = Vec::new();
                        let text = ContentPart::join_text(&m.content);
                        if !text.is_empty() {
                            blocks.push(json!({"type": "text", "text": text}));
                        }
                        // 借鉴 VCP: assistant tool_calls → Anthropic tool_use 块
                        for tc in &m.tool_calls {
                            blocks.push(json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.name,
                                "input": tc.arguments,
                            }));
                        }
                        msg.insert("content".into(), Value::Array(blocks));
                    }
                    MessageRole::Tool => {
                        // Anthropic: tool_result 是 user message 的 content block,
                        // 角色还是 user (Anthropic 没有 tool 角色)
                        msg.insert("role".into(), Value::String("user".into()));
                        let mut block = Map::new();
                        block.insert("type".into(), Value::String("tool_result".into()));
                        if let Some(id) = &m.tool_call_id {
                            block.insert("tool_use_id".into(), Value::String(id.clone()));
                        }
                        if let Some(name) = &m.name {
                            // Anthropic 不需要 name 在 tool_result 里, 但保留允许
                            block.insert("tool_name".into(), Value::String(name.clone()));
                        }
                        let text = ContentPart::join_text(&m.content);
                        block.insert("content".into(), Value::String(text));
                        if let Some(c) = &req.metadata.get("is_error") {
                            if c.parse::<bool>().unwrap_or(false) {
                                block.insert("is_error".into(), Value::Bool(true));
                            }
                        }
                        msg.insert("content".into(), Value::Array(vec![Value::Object(block)]));
                    }
                    MessageRole::System => {
                        // 已经在外层处理过, 不应到这里
                    }
                }
                Value::Object(msg)
            })
            .collect();

        body.insert("messages".into(), Value::Array(messages));

        if let Some(t) = req.temperature {
            if !(0.0..=1.0).contains(&t) {
                return Err(ProtocolError::invalid(
                    "temperature",
                    format!("Anthropic temperature must be in [0.0, 1.0], got {}", t),
                ));
            }
            body.insert("temperature".into(), json!(t));
        }

        if req.stream {
            body.insert("stream".into(), Value::Bool(true));
        }

        if !req.stop.is_empty() {
            body.insert(
                "stop_sequences".into(),
                Value::Array(req.stop.iter().map(|s| json!(s)).collect()),
            );
        }

        if !tools.is_empty() {
            let tool_arr: Vec<Value> = tools
                .iter()
                .map(|t: &NormalizedTool| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect();
            body.insert("tools".into(), Value::Array(tool_arr));
        }

        if let Some(v) = tool_choice_v {
            body.insert("tool_choice".into(), v);
        }

        // thinking (extended thinking) — R17 留空, 战役 1-2 接 Claude 3.7 时实装
        if let Some(budget) = req.metadata.get("thinking_budget") {
            if let Ok(b) = budget.parse::<u32>() {
                body.insert(
                    "thinking".into(),
                    json!({"type": "enabled", "budget_tokens": b}),
                );
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

        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        if let Some(arr) = raw.get("content").and_then(|v| v.as_array()) {
            for block in arr {
                let btype = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match btype {
                    "text" => {
                        if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                            if !content.is_empty() {
                                content.push('\n');
                            }
                            content.push_str(text);
                        }
                    }
                    "tool_use" => {
                        let id = block
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let name = block
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let input = block
                            .get("input")
                            .cloned()
                            .unwrap_or(Value::Object(Map::new()));
                        tool_calls.push(ToolCall {
                            id,
                            name,
                            arguments: input,
                        });
                    }
                    "thinking" => {
                        // 借鉴 VCP: thinking 块不进入主 content, 进 raw_metadata
                        if let Some(thinking) = block.get("thinking").and_then(|v| v.as_str()) {
                            // 不追加到 content, 留给 raw_metadata
                            let _ = thinking;
                        }
                    }
                    _ => {}
                }
            }
        }

        let finish_reason = raw
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(NormalizedFinishReason::from_anthropic);

        let usage = raw
            .get("usage")
            .map(|u| {
                let p = u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let c = u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                crate::normalized::NormalizedUsage::new(p, c)
            })
            .unwrap_or_default();

        let mut raw_metadata = Map::new();
        if let Some(s) = raw.get("stop_sequence").and_then(|v| v.as_str()) {
            raw_metadata.insert("stop_sequence".into(), Value::String(s.to_string()));
        }
        if let Some(t) = raw.get("type").and_then(|v| v.as_str()) {
            raw_metadata.insert("type".into(), Value::String(t.to_string()));
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

impl AnthropicMessagesAdapter {
    /// 构建 user message content (text or array of text/image blocks)
    fn build_user_content(&self, m: &crate::normalized::NormalizedMessage) -> Value {
        // 单 part text → 字符串; 多 part / 含 image → 数组
        if m.content.len() == 1 {
            if let ContentPart::Text { text } = &m.content[0] {
                return Value::String(text.clone());
            }
        }
        let parts: Vec<Value> = m
            .content
            .iter()
            .map(|p| match p {
                ContentPart::Text { text } => json!({"type": "text", "text": text}),
                ContentPart::ImageUrl { url, .. } => {
                    // Anthropic image: source.type=url or base64
                    if url.starts_with("data:") {
                        // base64 → source.type=base64
                        let comma = url.find(',').unwrap_or(url.len());
                        let meta = &url[5..comma]; // "image/png;base64"
                        let data = &url[comma + 1..];
                        let parts: Vec<&str> = meta.split(';').collect();
                        let media_type = parts.first().copied().unwrap_or("image/png");
                        json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": media_type,
                                "data": data,
                            }
                        })
                    } else {
                        json!({
                            "type": "image",
                            "source": {
                                "type": "url",
                                "url": url,
                            }
                        })
                    }
                }
            })
            .collect();
        Value::Array(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalized::{NormalizedMessage, NormalizedTool};
    use serde_json::Map;

    #[test]
    fn request_system_promoted_to_top_level() {
        let mut req = NormalizedRequest::new(
            "claude-sonnet-4",
            vec![
                NormalizedMessage::system("You are Claude"),
                NormalizedMessage::user("hi"),
            ],
        );
        req.max_tokens = Some(1024);
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["system"], "You are Claude");
        assert_eq!(v["max_tokens"], 1024);
        // messages 不含 system
        assert_eq!(v["messages"].as_array().unwrap().len(), 1);
        assert_eq!(v["messages"][0]["role"], "user");
    }

    #[test]
    fn request_max_tokens_required() {
        let req = NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        let err = AnthropicMessagesAdapter::new()
            .adapt_request(&req)
            .unwrap_err();
        assert!(matches!(err, ProtocolError::Invalid { .. }));
    }

    #[test]
    fn request_temperature_out_of_anthropic_range() {
        let mut req =
            NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        req.max_tokens = Some(100);
        req.temperature = Some(1.5);
        let err = AnthropicMessagesAdapter::new()
            .adapt_request(&req)
            .unwrap_err();
        assert!(matches!(err, ProtocolError::Invalid { .. }));
    }

    #[test]
    fn request_tool_choice_none_drops_tools() {
        // 借鉴 VCP protocolBridge.js:120-156: tool_choice=none → 去掉 tools
        let mut req =
            NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        req.max_tokens = Some(100);
        req.tools.push(NormalizedTool::new("fn"));
        req.tool_choice = Some(NormalizedToolChoice::None);
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        assert!(v.get("tools").is_none());
    }

    #[test]
    fn request_tool_choice_required_unsupported() {
        let mut req =
            NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        req.max_tokens = Some(100);
        req.tool_choice = Some(NormalizedToolChoice::Required);
        let err = AnthropicMessagesAdapter::new()
            .adapt_request(&req)
            .unwrap_err();
        assert!(matches!(err, ProtocolError::Unsupported { .. }));
    }

    #[test]
    fn request_tool_choice_specific_filters_tools() {
        // 借鉴 VCP: specific 退化为 filter
        let mut req =
            NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        req.max_tokens = Some(100);
        req.tools.push(NormalizedTool::new("a"));
        req.tools.push(NormalizedTool::new("b"));
        req.tool_choice = Some(NormalizedToolChoice::Specific { name: "b".into() });
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["tools"].as_array().unwrap().len(), 1);
        assert_eq!(v["tools"][0]["name"], "b");
    }

    #[test]
    fn request_assistant_tool_calls_become_tool_use_blocks() {
        // 借鉴 VCP: OpenAI tool_calls → Anthropic tool_use 块
        let tc = ToolCall {
            id: "toolu_1".into(),
            name: "get_weather".into(),
            arguments: json!({"city": "Beijing"}),
        };
        let mut req = NormalizedRequest::new(
            "claude-sonnet-4",
            vec![NormalizedMessage::assistant_with_tool_calls("", vec![tc])],
        );
        req.max_tokens = Some(100);
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        let content = v["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content[0]["type"], "tool_use");
        assert_eq!(content[0]["id"], "toolu_1");
        assert_eq!(content[0]["name"], "get_weather");
        assert_eq!(content[0]["input"], json!({"city": "Beijing"}));
    }

    #[test]
    fn request_tool_message_becomes_user_with_tool_result_block() {
        // 借鉴 VCP: OpenAI tool → Anthropic user role + tool_result block
        let mut req = NormalizedRequest::new(
            "claude-sonnet-4",
            vec![NormalizedMessage::tool_result(
                "toolu_1",
                Some("get_weather".into()),
                "sunny",
            )],
        );
        req.max_tokens = Some(100);
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["messages"][0]["role"], "user");
        let blocks = v["messages"][0]["content"].as_array().unwrap();
        assert_eq!(blocks[0]["type"], "tool_result");
        assert_eq!(blocks[0]["tool_use_id"], "toolu_1");
        assert_eq!(blocks[0]["content"], "sunny");
    }

    #[test]
    fn request_image_url_becomes_anthropic_image_block() {
        // 借鉴 VCP: image 块两种 source (url / base64)
        use crate::normalized::ContentPart;
        let msg = NormalizedMessage {
            role: MessageRole::User,
            content: vec![
                ContentPart::text_only("see "),
                ContentPart::ImageUrl {
                    url: "https://x.com/a.png".into(),
                    detail: None,
                },
            ],
            tool_calls: Vec::new(),
            tool_call_id: None,
            name: None,
        };
        let mut req = NormalizedRequest::new("claude-sonnet-4", vec![msg]);
        req.max_tokens = Some(100);
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        let parts = v["messages"][0]["content"].as_array().unwrap();
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(parts[1]["type"], "image");
        assert_eq!(parts[1]["source"]["type"], "url");
        assert_eq!(parts[1]["source"]["url"], "https://x.com/a.png");
    }

    #[test]
    fn request_thinking_metadata_enables_extended_thinking() {
        let mut req =
            NormalizedRequest::new("claude-sonnet-4", vec![NormalizedMessage::user("hi")]);
        req.max_tokens = Some(2000);
        req.metadata.insert("thinking_budget".into(), "1024".into());
        let v = AnthropicMessagesAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["thinking"]["type"], "enabled");
        assert_eq!(v["thinking"]["budget_tokens"], 1024);
    }

    #[test]
    fn response_text_only() {
        let raw = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-20250514",
            "content": [{"type": "text", "text": "Hello there"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 20, "output_tokens": 10}
        });
        let r = AnthropicMessagesAdapter::new()
            .adapt_response(&raw)
            .unwrap();
        assert_eq!(r.id, "msg_123");
        assert_eq!(r.model, "claude-sonnet-4-20250514");
        assert_eq!(r.content, "Hello there");
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(r.usage.prompt_tokens, 20);
        assert_eq!(r.usage.completion_tokens, 10);
    }

    #[test]
    fn response_with_tool_use_block() {
        let raw = json!({
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4",
            "content": [
                {"type": "text", "text": "Let me check"},
                {
                    "type": "tool_use",
                    "id": "toolu_1",
                    "name": "get_weather",
                    "input": {"city": "Beijing"}
                }
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 30, "output_tokens": 20}
        });
        let r = AnthropicMessagesAdapter::new()
            .adapt_response(&raw)
            .unwrap();
        assert_eq!(r.content, "Let me check");
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::ToolCalls));
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].id, "toolu_1");
        assert_eq!(r.tool_calls[0].name, "get_weather");
        assert_eq!(r.tool_calls[0].arguments, json!({"city": "Beijing"}));
    }

    #[test]
    fn response_max_tokens_maps_to_length() {
        let raw = json!({
            "id": "x", "type": "message", "role": "assistant",
            "model": "claude-sonnet-4",
            "content": [{"type": "text", "text": "..."}],
            "stop_reason": "max_tokens", "usage": {}
        });
        let r = AnthropicMessagesAdapter::new()
            .adapt_response(&raw)
            .unwrap();
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Length));
    }

    #[test]
    fn response_stop_sequence_maps() {
        let raw = json!({
            "id": "x", "type": "message", "role": "assistant",
            "model": "claude-sonnet-4",
            "content": [{"type": "text", "text": "..."}],
            "stop_reason": "stop_sequence", "stop_sequence": "END",
            "usage": {}
        });
        let r = AnthropicMessagesAdapter::new()
            .adapt_response(&raw)
            .unwrap();
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::StopSequence));
        assert_eq!(r.raw_metadata["stop_sequence"], "END");
    }
}
