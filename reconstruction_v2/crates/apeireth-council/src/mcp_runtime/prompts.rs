//! MCP prompt types (per MCP spec §prompts).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::mcp_runtime::protocol::JsonRpcError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

impl PromptArgument {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), description: None, required: false }
    }
    pub fn required(mut self) -> Self { self.required = true; self }
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into()); self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Prompt {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

impl Prompt {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), description: None, arguments: None }
    }
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into()); self
    }
    pub fn with_arguments(mut self, args: Vec<PromptArgument>) -> Self {
        self.arguments = Some(args); self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PromptRole { System, User, Assistant }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum PromptContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: PromptContent,
}

impl PromptMessage {
    pub fn assistant_text(text: impl Into<String>) -> Self {
        Self { role: PromptRole::Assistant, content: PromptContent::Text { text: text.into() } }
    }
    pub fn user_text(text: impl Into<String>) -> Self {
        Self { role: PromptRole::User, content: PromptContent::Text { text: text.into() } }
    }
    pub fn system_text(text: impl Into<String>) -> Self {
        Self { role: PromptRole::System, content: PromptContent::Text { text: text.into() } }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

impl GetPromptResult {
    pub fn new(messages: Vec<PromptMessage>) -> Self {
        Self { description: None, messages }
    }
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into()); self
    }
}

pub trait PromptServer {
    fn list(&self) -> Vec<Prompt>;
    fn get(&self, name: &str, arguments: &Value) -> Result<GetPromptResult, JsonRpcError>;
}

pub const PROMPT_NOT_FOUND: i32 = -32601;
pub const PROMPT_INVALID_ARGS: i32 = -32602;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn argument_builder() {
        let a = PromptArgument::new("q").required().with_description("d");
        assert_eq!(a.name, "q"); assert!(a.required);
        assert_eq!(a.description.as_deref(), Some("d"));
    }
    #[test]
    fn role_roundtrip() {
        let j = serde_json::to_string(&PromptRole::Assistant).unwrap();
        let r: PromptRole = serde_json::from_str(&j).unwrap();
        assert_eq!(r, PromptRole::Assistant);
    }
    #[test]
    fn content_tag() {
        let v = serde_json::to_value(&PromptContent::Text { text: "hi".into() }).unwrap();
        assert_eq!(v["type"], "text");
    }
    #[test]
    fn message_factory() {
        let m = PromptMessage::user_text("hi");
        assert_eq!(m.role, PromptRole::User);
    }
    #[test]
    fn result_new() {
        let r = GetPromptResult::new(vec![PromptMessage::assistant_text("hi")]).with_description("d");
        assert_eq!(r.messages.len(), 1);
        assert_eq!(r.description.as_deref(), Some("d"));
    }
}
