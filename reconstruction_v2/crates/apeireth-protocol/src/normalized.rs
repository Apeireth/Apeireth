use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    Reasoning { reasoning: String },
    ToolCall { tool_call: ToolCall },
    ToolResult { tool_call_id: String, result: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String, // JSON encoded args
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedMessage {
    pub role: Role,
    pub parts: Vec<ContentPart>,
}

impl NormalizedMessage {
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            parts: vec![ContentPart::Text { text: text.into() }],
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            parts: vec![ContentPart::Text { text: text.into() }],
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            parts: vec![ContentPart::Text { text: text.into() }],
        }
    }

    pub fn extract_text(&self) -> String {
        let mut out = String::new();
        for part in &self.parts {
            if let ContentPart::Text { text } = part {
                out.push_str(text);
            }
        }
        out
    }

    pub fn extract_reasoning(&self) -> Option<String> {
        let mut out = String::new();
        for part in &self.parts {
            if let ContentPart::Reasoning { reasoning } = part {
                out.push_str(reasoning);
            }
        }
        if out.is_empty() { None } else { Some(out) }
    }

    pub fn extract_tool_calls(&self) -> Vec<ToolCall> {
        let mut calls: Vec<ToolCall> = self.parts.iter().filter_map(|p| {
            if let ContentPart::ToolCall { tool_call } = p {
                Some(tool_call.clone())
            } else {
                None
            }
        }).collect();

        if calls.is_empty() {
            let text = self.extract_text();
            let marker = "functions.";
            if let Some(pos) = text.find(marker) {
                let rest = &text[pos + marker.len()..];
                if let Some(paren_open) = rest.find('(') {
                    let tool_name = rest[..paren_open].trim();
                    let args_rest = &rest[paren_open + 1..];
                    if let Some(paren_close) = args_rest.rfind(')') {
                        let raw_args = args_rest[..paren_close].trim();
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0);
                        calls.push(ToolCall {
                            id: format!("text_{}", ts),
                            name: tool_name.to_string(),
                            arguments: raw_args.to_string(),
                        });


                    }
                }
            }
        }

        calls
    }


    pub fn tool_result(tool_call_id: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            parts: vec![ContentPart::ToolResult {
                tool_call_id: tool_call_id.into(),
                result: result.into(),
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedRequest {
    pub model: String,
    pub messages: Vec<NormalizedMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<NormalizedTool>>,
    pub stream: bool,
}

impl NormalizedRequest {
    pub fn new(model: impl Into<String>, messages: Vec<NormalizedMessage>) -> Self {
        Self {
            model: model.into(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(2048),
            tools: None,
            stream: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedResponse {
    pub id: String,
    pub model: String,
    pub message: NormalizedMessage,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}
