//! Google Gemini GenerateContent adapter (v1 era transcription)
//!
//! Transcribed from `crates/_archived/v1.0-legacy/apeireth-protocol/src/adapters/gemini.rs`.

use crate::v1_era::adapters::ProtocolAdapter;
use crate::v1_era::error::ProtocolError;
use crate::v1_era::normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedRequest, NormalizedResponse,
    NormalizedUsage,
};
use serde_json::{json, Map, Value};

/// Gemini GenerateContent adapter (ZST)
pub struct GeminiAdapter;

impl GeminiAdapter {
    pub fn new() -> Self { Self }
}

impl Default for GeminiAdapter {
    fn default() -> Self { Self::new() }
}

fn parse_finish_reason_gemini(s: &str) -> NormalizedFinishReason {
    match s {
        "STOP" => NormalizedFinishReason::Stop,
        "MAX_TOKENS" => NormalizedFinishReason::Length,
        "SAFETY" => NormalizedFinishReason::ContentFilter,
        _ => NormalizedFinishReason::Other,
    }
}

impl ProtocolAdapter for GeminiAdapter {
    fn name(&self) -> &'static str { "gemini" }
    fn endpoint_path(&self) -> &'static str { "/v1beta/models/{model}:generateContent" }

    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError> {
        if req.model.is_empty() {
            return Err(ProtocolError::missing("model"));
        }
        if req.messages.is_empty() {
            return Err(ProtocolError::missing("messages"));
        }
        let mut body = Map::new();

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
            body.insert(
                "systemInstruction".into(),
                json!({"role": "system", "parts": [{"text": s}]}),
            );
        }

        let contents: Vec<Value> = non_system.iter().map(|m| {
            let role_str = match m.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "model",
                MessageRole::Tool | MessageRole::System => "user",
            };
            let parts: Vec<Value> = m.content.iter().map(|p| match p {
                ContentPart::Text { text } => json!({"text": text}),
                ContentPart::ImageUrl { url, .. } => {
                    json!({"inline_data": {"mime_type": "image/jpeg", "data": url}})
                }
            }).collect();
            json!({"role": role_str, "parts": parts})
        }).collect();
        body.insert("contents".into(), Value::Array(contents));

        // generationConfig
        let mut gen_config = Map::new();
        if let Some(t) = req.temperature {
            gen_config.insert("temperature".into(), json!(t));
        }
        if let Some(n) = req.max_tokens {
            gen_config.insert("maxOutputTokens".into(), json!(n));
        }
        if !gen_config.is_empty() {
            body.insert("generationConfig".into(), Value::Object(gen_config));
        }

        if !req.tools.is_empty() {
            let declarations: Vec<Value> = req.tools.iter().map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                })
            }).collect();
            body.insert("tools".into(), json!([{"functionDeclarations": declarations}]));
        }
        Ok(Value::Object(body))
    }

    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError> {
        let mut content = String::new();
        let mut finish: Option<NormalizedFinishReason> = None;
        if let Some(candidates) = raw.get("candidates").and_then(|v| v.as_array()) {
            if let Some(first) = candidates.first() {
                if let Some(content_obj) = first.get("content") {
                    if let Some(parts) = content_obj.get("parts").and_then(|v| v.as_array()) {
                        for part in parts {
                            if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                                content.push_str(text);
                            }
                        }
                    }
                }
                if let Some(fr) = first.get("finishReason").and_then(|v| v.as_str()) {
                    finish = Some(parse_finish_reason_gemini(fr));
                }
            }
        }
        let usage = raw.get("usageMetadata").map(|u| NormalizedUsage {
            prompt_tokens: u.get("promptTokenCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("candidatesTokenCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: u.get("totalTokenCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        }).unwrap_or_default();
        Ok(NormalizedResponse {
            id: raw.get("responseId").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            model: raw.get("modelVersion").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            content,
            finish_reason: finish,
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
        let a = GeminiAdapter::new();
        assert_eq!(a.name(), "gemini");
        assert!(a.endpoint_path().contains("generateContent"));
    }

    #[test]
    fn encode_basic() {
        let req = NormalizedRequest::new("gemini-1.5-pro", vec![NormalizedMessage::user("hi")]);
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        assert!(v["contents"].is_array());
    }

    #[test]
    fn encode_system_to_system_instruction() {
        let req = NormalizedRequest::new("gemini-1.5-pro", vec![
            NormalizedMessage::system("be brief"),
            NormalizedMessage::user("hi"),
        ]);
        let v = GeminiAdapter::new().adapt_request(&req).unwrap();
        assert!(v.get("systemInstruction").is_some());
        let contents = v["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn decode_basic_response() {
        let raw = json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "hello"}]},
                "finishReason": "STOP"
            }],
            "modelVersion": "gemini-1.5-pro",
            "responseId": "r1",
            "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 2, "totalTokenCount": 3}
        });
        let r = GeminiAdapter::new().adapt_response(&raw).unwrap();
        assert_eq!(r.content, "hello");
        assert_eq!(r.finish_reason, Some(NormalizedFinishReason::Stop));
        assert_eq!(r.usage.total_tokens, 3);
    }
}
