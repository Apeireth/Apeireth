//! MCP resource types (per MCP spec §resources).

use serde::{Deserialize, Serialize};
use crate::mcp_runtime::protocol::JsonRpcError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl Resource {
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self { uri: uri.into(), name: name.into(), description: None, mime_type: None }
    }
    pub fn with_description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into()); self
    }
    pub fn with_mime_type(mut self, m: impl Into<String>) -> Self {
        self.mime_type = Some(m.into()); self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceContent {
    pub uri: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl ResourceContent {
    pub fn new(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self { uri: uri.into(), text: text.into(), mime_type: None }
    }
    pub fn with_mime_type(mut self, m: impl Into<String>) -> Self {
        self.mime_type = Some(m.into()); self
    }
}

pub trait ResourceServer {
    fn list(&self) -> Vec<Resource>;
    fn read(&self, uri: &str) -> Result<ResourceContent, JsonRpcError>;
}

pub const RESOURCE_NOT_FOUND: i32 = -32001;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resource_with_mime() {
        let r = Resource::new("u", "n").with_mime_type("application/json");
        assert_eq!(r.mime_type.as_deref(), Some("application/json"));
    }
}
