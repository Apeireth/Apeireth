//! Google Gemini GenerateContent 协议 adapter
//!
//! **端点**: `POST /v1beta/models/{model}:generateContent`
//! **鉴权**: `x-goog-api-key: <api_key>` (或 query param `?key=`)
//!
//! **协议差异** (vs OpenAI / Anthropic):
//! - **URL 路径带 model** (`/v1beta/models/{model}:generateContent`)
//! - **`contents[]`** 数组 (role: `user` / `model`), 不是 messages
//! - **`systemInstruction`** 顶层 (类似 Anthropic `system`)
//! - **`tools[].functionDeclarations[]`** 数组 (复刻 VCP `extractProtectedTools` 真机制)
//! - **`parts[]`** 数组, 每 part 是 `text` / `inlineData` / `functionCall` / `functionResponse`
//! - **响应 `candidates[0].content.parts[]`** 而不是 `choices[0].message`
//! - **`finishReason`** 枚举: `STOP` / `MAX_TOKENS` / `SAFETY` / `RECITATION` / `OTHER`
//!
//! **借鉴 VCP**:
//! - `routes/protocolBridge.js:91-118` `extractProtectedTools` (Gemini functionDeclarations
//!   + legacy `functions` 处理, **关键字段级引用**)

use crate::adapter::ProtocolAdapter;
use crate::error::ProtocolError;
use crate::normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedToolChoice, ToolCall,
};
use serde_json::{json, Map, Value};

/// Gemini GenerateContent adapter
pub struct GeminiAdapter;

impl GeminiAdapter {
    /// 创建
    pub fn new() -> Self {
        Self
    }
}

impl Default for GeminiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtocolAdapter for GeminiAdapter {
    fn name(&self) -> &'static str {
        "gemini"
    }

    fn endpoint_path(&self) -> &'static str {
        // URL 路径里有 model, 实际调用时需要把 {model} 替换
        "/v1beta/models/{model}:generateContent"
    }

    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        if req.model.is_empty() {
            return Err(ProtocolError::missing("model"));
        }
        if req.messages.is_empty() {
            return Err(ProtocolError::missing("messages"));
        }

        let mut body = Map::new();

        // 借鉴 VCP: system → systemInstruction 顶层
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
            body.insert(
                "systemInstruction".into(),
                json!({"role": "system", "parts": [{"text": s}]}),
            );
        }

        // 借鉴 VCP `extractProtectedTools` (protocolBridge.js:91-118):
        // Gemini tools 用 functionDeclarations 数组 (跟 OpenAI 嵌套格式不同)
        if !req.tools.is_empty() {
            let declarations: Vec<Value> = req
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    })
                })
                .collect();
            body.insert(
                "tools".into(),
                json!([{
                    "functionDeclarations": declarations
                }]),
            );
        }

        // 借鉴 VCP `normalizeToolChoice` (protocolBridge.js:120-156):
        // Gemini 用 toolConfig.functionCallingConfig.mode (NONE / AUTO / ANY)
        if let Some(tc) = &req.tool_choice {
            let mode = match tc {
                NormalizedToolChoice::None => "NONE",
                NormalizedToolChoice::Auto => "AUTO",
                NormalizedToolChoice::Required => "ANY",
                NormalizedToolChoice::Specific { name } => {
                    body.insert(
                        "toolConfig".into(),
                        json!({
                            "functionCallingConfig": {
                                "mode": "ANY",
                                "allowedFunctionNames": [name],
                            }
                        }),
                    );
                    return self.finalize_body(body, req);
                }
            };
            body.insert(
                "toolConfig".into(),
                json!({
                    "functionCallingConfig": {
                        "mode": mode,
                    }
                }),
            );
        }

        // 借鉴 VCP legacy functions (protocolBridge.js:110-115): 兼容旧 Gemini
        // 已经在 NormalizedRequest 里归一化为 tools, 这里不重复

        // contents
        let contents: Vec<Value> = non_system
            .iter()
            .map(|m| {
                let mut content = Map::new();
                match m.role {
                    MessageRole::User => {
                        content.insert("role".into(), Value::String("user".into()));
                        content.insert("parts".into(), self.build_user_parts(m));
                    }
                    MessageRole::Assistant => {
                        // Gemini: assistant → "model" role
                        content.insert("role".into(), Value::String("model".into()));
                        let mut parts: Vec<Value> = Vec::new();
                        let text = ContentPart::join_text(&m.content);
                        if !text.is_empty() {
                            parts.push(json!({"text": text}));
                        }
                        for tc in &m.tool_calls {
                            // 借鉴 VCP: functionCall 块
                            parts.push(json!({
                                "functionCall": {
                                    "name": tc.name,
                                    "args": tc.arguments,
                                }
                            }));
                        }
                        content.insert("parts".into(), Value::Array(parts));
                    }
                    MessageRole::Tool => {
                        // Gemini: tool → user role + functionResponse part
                        content.insert("role".into(), Value::String("user".into()));
                        let response = json!({
                            "name": m.name.clone().unwrap_or_default(),
                            "response": {
                                "content": ContentPart::join_text(&m.content),
                            }
                        });
                        // 借鉴 VCP `is_tool_result_error` 真代码: 检查 is_error
                        let is_error = m
                            .name
                            .as_deref()
                            .map(|_| {
                                req.metadata
                                    .get("is_error")
                                    .map(|s| s == "true")
                                    .unwrap_or(false)
                            })
                            .unwrap_or(false);
                        let mut part = Map::new();
                        part.insert("functionResponse".into(), response);
                        if is_error {
                            // Gemini 不直接支持 is_error, 用 response 里加 error 字段
                            if let Some(obj) = part
                                .get_mut("functionResponse")
                                .and_then(|v| v.get_mut("response"))
                                .and_then(|v| v.as_object_mut())
                            {
                                obj.insert("error".into(), Value::Bool(true));
                            }
                        }
                        content.insert("parts".into(), Value::Array(vec![Value::Object(part)]));
                    }
                    MessageRole::System => {
                        // 已经在 systemInstruction 处理
                    }
                }
                Value::Object(content)
            })
            .collect();

        body.insert("contents".into(), Value::Array(contents));

        self.finalize_body(body, req)
    }

    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        // Gemini 响应可能有 candidates 数组 (正常) 或 promptFeedback (block)
        let id = raw
            .get("responseId")
            .or_else(|| raw.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing("responseId"))?
            .to_string();
        let model = raw
            .get("modelVersion")
            .or_else(|| raw.get("model"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProtocolError::missing("modelVersion"))?
            .to_string();

        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut finish_reason: Option<NormalizedFinishReason> = None;

        if let Some(candidates) = raw.get("candidates").and_then(|v| v.as_array()) {
            if let Some(c0) = candidates.first() {
                if let Some(parts) = c0
                    .get("content")
                    .and_then(|c| c.get("parts"))
                    .and_then(|p| p.as_array())
                {
                    for part in parts {
                        if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                            if !content.is_empty() {
                                content.push('\n');
                            }
                            content.push_str(text);
                        }
                        if let Some(fc) = part.get("functionCall") {
                            let name = fc
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            // 借鉴 VCP: Gemini functionCall 没有 id, 用 name 替代 (保持唯一性靠 user 端)
                            let id = format!("gemini_call_{}", name);
                            let args = fc.get("args").cloned().unwrap_or(Value::Object(Map::new()));
                            tool_calls.push(ToolCall {
                                id,
                                name,
                                arguments: args,
                            });
                        }
                    }
                }
                if let Some(fr) = c0.get("finishReason").and_then(|v| v.as_str()) {
                    finish_reason = Some(NormalizedFinishReason::from_gemini(fr));
                }
            }
        }

        let usage = raw
            .get("usageMetadata")
            .map(|u| {
                let p = u
                    .get("promptTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let c = u
                    .get("candidatesTokenCount")
                    .or_else(|| u.get("outputTokenCount"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                crate::normalized::NormalizedUsage::new(p, c)
            })
            .unwrap_or_default();

        let mut raw_metadata = Map::new();
        if let Some(fr) = raw
            .get("promptFeedback")
            .and_then(|p| p.get("blockReason"))
            .and_then(|v| v.as_str())
        {
            raw_metadata.insert("block_reason".into(), Value::String(fr.to_string()));
        }
        if let Some(vm) = raw.get("modelVersion").and_then(|v| v.as_str()) {
            raw_metadata.insert("modelVersion".into(), Value::String(vm.to_string()));
        }
        if let Some(rid) = raw.get("responseId").and_then(|v| v.as_str()) {
            raw_metadata.insert("responseId".into(), Value::String(rid.to_string()));
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

impl GeminiAdapter {
    fn finalize_body(
        &self,
        mut body: Map<String, Value>,
        req: &NormalizedRequest,
    ) -> Result<Value, ProtocolError> {
        // generationConfig
        let mut gen_config = Map::new();
        if let Some(t) = req.temperature {
            if !(0.0..=2.0).contains(&t) {
                return Err(ProtocolError::invalid(
                    "temperature",
                    format!("Gemini temperature must be in [0.0, 2.0], got {}", t),
                ));
            }
            gen_config.insert("temperature".into(), json!(t));
        }
        if let Some(n) = req.max_tokens {
            gen_config.insert("maxOutputTokens".into(), json!(n));
        }
        if !req.stop.is_empty() {
            gen_config.insert(
                "stopSequences".into(),
                Value::Array(req.stop.iter().map(|s| json!(s)).collect()),
            );
        }
        if !gen_config.is_empty() {
            body.insert("generationConfig".into(), Value::Object(gen_config));
        }
        Ok(Value::Object(body))
    }

    fn build_user_parts(&self, m: &crate::normalized::NormalizedMessage) -> Value {
        let parts: Vec<Value> = m
            .content
            .iter()
            .map(|p| match p {
                ContentPart::Text { text } => json!({"text": text}),
                ContentPart::ImageUrl { url, .. } => {
                    if url.starts_with("data:") {
                        let comma = url.find(',').unwrap_or(url.len());
                        let meta = &url[5..comma];
                        let data = &url[comma + 1..];
                        let media_type = meta.split(';').next().unwrap_or("image/png");
                        json!({
                            "inlineData": {
                                "mimeType": media_type,
                                "data": data,
                            }
                        })
                    } else {
                        // Gemini 也支持 fileData 引用 URL (不下载)
                        json!({
                            "fileData": {
                                "mimeType": "image/jpeg",
                                "fileUri": url,
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
    fn request_system_promoted_to_system_instruction() {
        // 借鉴 VCP: system → systemInstruction 顶层
        let req = NormalizedRequest::new(
            "gemini-1.5-pro",
            vec![
                NormalizedMessage::system("You are Gemini"),
                NormalizedMessage::user("hi"),
            ],
        );
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["systemInstruction"]["parts"][0]["text"], "You are Gemini");
        assert_eq!(v["contents"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn request_contents_uses_user_and_model_roles() {
        // 借鉴 VCP: assistant → model role
        let req = NormalizedRequest::new(
            "gemini-1.5-pro",
            vec![
                NormalizedMessage::user("hi"),
                NormalizedMessage::assistant("hello back"),
                NormalizedMessage::user("how are you?"),
            ],
        );
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        let contents = v["contents"].as_array().unwrap();
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[2]["role"], "user");
        assert_eq!(contents[1]["parts"][0]["text"], "hello back");
    }

    #[test]
    fn request_tools_use_function_declarations() {
        // 借鉴 VCP protocolBridge.js:91-118 extractProtectedTools 真机制
        let mut req = NormalizedRequest::new("gemini-1.5-pro", vec![NormalizedMessage::user("hi")]);
        let mut p = Map::new();
        p.insert("type".into(), Value::String("object".into()));
        req.tools
            .push(NormalizedTool::new("get_weather").with_parameters(p));
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(
            v["tools"][0]["functionDeclarations"][0]["name"],
            "get_weather"
        );
    }

    #[test]
    fn request_tool_choice_specific_uses_allowed_function_names() {
        // 借鉴 VCP normalizeToolChoice (protocolBridge.js:120-156): mode=ANY + allowedFunctionNames
        let mut req = NormalizedRequest::new("gemini-1.5-pro", vec![NormalizedMessage::user("hi")]);
        req.tools.push(NormalizedTool::new("get_weather"));
        req.tools.push(NormalizedTool::new("get_news"));
        req.tool_choice = Some(NormalizedToolChoice::Specific {
            name: "get_weather".into(),
        });
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["toolConfig"]["functionCallingConfig"]["mode"], "ANY");
        assert_eq!(
            v["toolConfig"]["functionCallingConfig"]["allowedFunctionNames"][0],
            "get_weather"
        );
    }

    #[test]
    fn request_tool_choice_none_uses_mode_none() {
        let req = NormalizedRequest::new("gemini-1.5-pro", vec![NormalizedMessage::user("hi")]);
        let mut req = req;
        req.tool_choice = Some(NormalizedToolChoice::None);
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["toolConfig"]["functionCallingConfig"]["mode"], "NONE");
    }

    #[test]
    fn request_assistant_tool_calls_become_function_call_parts() {
        // 借鉴 VCP: OpenAI tool_calls → Gemini functionCall parts
        let tc = ToolCall {
            id: "call_1".into(),
            name: "get_weather".into(),
            arguments: json!({"city": "Beijing"}),
        };
        let req = NormalizedRequest::new(
            "gemini-1.5-pro",
            vec![NormalizedMessage::assistant_with_tool_calls("", vec![tc])],
        );
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        let parts = v["contents"][0]["parts"].as_array().unwrap();
        assert_eq!(parts[0]["functionCall"]["name"], "get_weather");
        assert_eq!(parts[0]["functionCall"]["args"], json!({"city": "Beijing"}));
    }

    #[test]
    fn request_tool_message_becomes_user_function_response() {
        // 借鉴 VCP: OpenAI tool → Gemini user + functionResponse
        let req = NormalizedRequest::new(
            "gemini-1.5-pro",
            vec![NormalizedMessage::tool_result(
                "call_1",
                Some("get_weather".into()),
                "sunny",
            )],
        );
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        assert_eq!(v["contents"][0]["role"], "user");
        let parts = v["contents"][0]["parts"].as_array().unwrap();
        assert_eq!(parts[0]["functionResponse"]["name"], "get_weather");
        assert_eq!(parts[0]["functionResponse"]["response"]["content"], "sunny");
    }

    #[test]
    fn request_generation_config_with_temperature_and_max_tokens() {
        let mut req = NormalizedRequest::new("gemini-1.5-pro", vec![NormalizedMessage::user("hi")]);
        req.temperature = Some(0.3);
        req.max_tokens = Some(512);
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        // f32 → JSON 会有精度问题, 用 delta 比较
        let temp = v["generationConfig"]["temperature"].as_f64().unwrap();
        assert!(
            (temp - 0.3).abs() < 1e-5,
            "temperature 应该是 0.3, got {}",
            temp
        );
        assert_eq!(v["generationConfig"]["maxOutputTokens"], 512);
    }

    #[test]
    fn request_temperature_out_of_range() {
        let mut req = NormalizedRequest::new("gemini-1.5-pro", vec![NormalizedMessage::user("hi")]);
        req.temperature = Some(3.0);
        assert!(matches!(
            GeminiAdapter::new().adapt_request(&req).unwrap_err(),
            ProtocolError::Invalid { .. }
        ));
    }

    #[test]
    fn response_text_only() {
        let raw = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [{"text": "Hello from Gemini"}]
                },
                "finishReason": "STOP",
                "index": 0
            }],
            "usageMetadata": {
                "promptTokenCount": 12,
                "candidatesTokenCount": 8,
                "totalTokenCount": 20
            },
            "modelVersion": "gemini-1.5-pro-002",
            "responseId": "resp_abc"
        });
        let r = GeminiAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.id, "resp_abc");
        assert_eq!(r.model, "gemini-1.5-pro-002");
        assert_eq!(r.content, "Hello from Gemini");
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(r.usage.prompt_tokens, 12);
        assert_eq!(r.usage.completion_tokens, 8);
    }

    #[test]
    fn response_with_function_call() {
        // 借鉴 VCP: Gemini functionCall 块
        let raw = json!({
            "candidates": [{
                "content": {
                    "role": "model",
                    "parts": [
                        {"text": "Let me check"},
                        {
                            "functionCall": {
                                "name": "get_weather",
                                "args": {"city": "Beijing"}
                            }
                        }
                    ]
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 5},
            "modelVersion": "gemini-1.5-pro",
            "responseId": "r1"
        });
        let r = GeminiAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.content, "Let me check");
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "get_weather");
        assert_eq!(r.tool_calls[0].arguments, json!({"city": "Beijing"}));
        // Gemini 没有 native id, 我们用 name 合成
        assert!(r.tool_calls[0].id.starts_with("gemini_call_"));
    }

    #[test]
    fn response_safety_finish_reason() {
        let raw = json!({
            "candidates": [{
                "content": {"role": "model", "parts": []},
                "finishReason": "SAFETY"
            }],
            "modelVersion": "gemini-1.5-pro",
            "responseId": "r2"
        });
        let r = GeminiAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::ContentFilter));
    }

    #[test]
    fn response_block_reason_recorded_in_raw_metadata() {
        let raw = json!({
            "promptFeedback": {"blockReason": "SAFETY"},
            "candidates": [],
            "modelVersion": "gemini-1.5-pro",
            "responseId": "r3"
        });
        let r = GeminiAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.raw_metadata["block_reason"], "SAFETY");
        assert_eq!(r.content, "");
        assert_eq!(r.finish_reason, None);
    }
}
