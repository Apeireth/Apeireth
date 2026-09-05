//! MCP wire-schema normalization.
//!
//! Donor `apeireth-mcp` stored tool/resource fields as Rust snake_case
//! (`input_schema`, `mime_type`, `is_error`) **without** serde renames, so
//! the JSON it emitted did not match MCP 2025-03-26 (`inputSchema`,
//! `mimeType`, `isError`). The v2 client in `apeireth-tools::mcp` already
//! renames `inputSchema` / `isError` on its own types; this module is the
//! shared library so a later host (and resource/prompt models) speak the
//! same wire.
//!
//! Also recovered: kebab-case tool/prompt name check (donor
//! `tools/naming.rs`) and a tolerant object-key rewrite that accepts
//! either snake_case or camelCase inbound JSON.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use apeireth_protocol::canonical::NormalizedTool;

/// Tool not found (MCP server-defined range −32000..−32099).
pub const TOOL_NOT_FOUND: i32 = -32010;
/// Invalid arguments.
pub const TOOL_INVALID_ARGS: i32 = -32011;
/// Call failed.
pub const TOOL_CALL_FAILED: i32 = -32012;
/// Internal tool error.
pub const TOOL_INTERNAL: i32 = -32013;

/// Known snake_case → MCP camelCase keys.
const WIRE_KEY_MAP: &[(&str, &str)] = &[
    ("input_schema", "inputSchema"),
    ("mime_type", "mimeType"),
    ("is_error", "isError"),
    ("protocol_version", "protocolVersion"),
    ("server_info", "serverInfo"),
    ("client_info", "clientInfo"),
    ("list_changed", "listChanged"),
];

/// Recursively rewrite object keys that have a known MCP camelCase form.
///
/// Already-camelCase keys are left alone. Arrays are walked. Non-objects
/// are returned unchanged. Used to ingest donor JSON that was serialized
/// without serde renames.
pub fn normalize_wire_object(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (k, v) in map {
                let key = rewrite_key(&k).to_string();
                out.insert(key, normalize_wire_object(v));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_wire_object).collect()),
        other => other,
    }
}

fn rewrite_key(k: &str) -> &str {
    for (from, to) in WIRE_KEY_MAP {
        if k == *from {
            return to;
        }
    }
    k
}

/// Kebab-case MCP tool / prompt name (donor `is_valid_tool_name`).
///
/// Rules: non-empty; ASCII lowercase / digit / `-`; no leading/trailing
/// `-`; no `--`.
pub fn is_valid_mcp_name(name: &str) -> bool {
    if name.is_empty() || name.starts_with('-') || name.ends_with('-') {
        return false;
    }
    let mut prev_dash = false;
    for c in name.chars() {
        if c == '-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
            if !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
                return false;
            }
        }
    }
    true
}

/// MCP tool descriptor (spec §tools/list item) with **wire** camelCase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "inputSchema"
    )]
    pub input_schema: Option<Value>,
}

impl McpTool {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            input_schema: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Convert a canonical [`NormalizedTool`] into an MCP list item.
    ///
    /// Parameters become `inputSchema`. Empty parameters serialize as an
    /// object schema with no properties rather than being dropped, because
    /// MCP clients expect `inputSchema` to be present. A non-empty parameter
    /// map is used as-is (callers store a JSON Schema object in the map).
    pub fn from_normalized(tool: &NormalizedTool) -> Self {
        let schema = if tool.parameters.is_empty() {
            json!({ "type": "object", "properties": {} })
        } else {
            Value::Object(tool.parameters.clone())
        };
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: Some(schema),
        }
    }
}

/// MCP content block (tools/call content[], prompts content, resource
/// contents). Tagged `type` with snake_case variants per spec (`text`,
/// `image`, `resource`). `mimeType` is camelCase on the wire.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "mimeType")]
        mime_type: Option<String>,
    },
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    Resource {
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "mimeType")]
        mime_type: Option<String>,
    },
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mime_type: None,
        }
    }

    pub fn text_with_mime(text: impl Into<String>, mime: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mime_type: Some(mime.into()),
        }
    }
}

/// Parse a content block from either camelCase or snake_case JSON.
pub fn content_block_from_wire(value: Value) -> Result<ContentBlock, String> {
    let normalized = normalize_wire_object(value);
    serde_json::from_value(normalized).map_err(|e| e.to_string())
}

/// Serialize a content block to spec camelCase JSON.
pub fn content_block_to_wire(block: &ContentBlock) -> Value {
    serde_json::to_value(block).unwrap_or(Value::Null)
}

/// MCP tools/call result (spec §tools/call).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
    #[serde(default, rename = "isError")]
    pub is_error: bool,
}

impl ToolCallResult {
    pub fn ok(content: Vec<ContentBlock>) -> Self {
        Self {
            content,
            is_error: false,
        }
    }

    pub fn err(content: Vec<ContentBlock>) -> Self {
        Self {
            content,
            is_error: true,
        }
    }

    /// Concatenate text blocks, skipping image/resource.
    pub fn extract_text(&self) -> String {
        let mut out = String::new();
        for c in &self.content {
            if let ContentBlock::Text { text, .. } = c {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(text);
            }
        }
        out
    }
}

/// Normalize a tools/call result object that may use donor snake_case.
pub fn normalize_mcp_result(value: Value) -> Result<ToolCallResult, String> {
    let normalized = normalize_wire_object(value);
    serde_json::from_value(normalized).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_mcp_name_kebab() {
        assert!(is_valid_mcp_name("echo"));
        assert!(is_valid_mcp_name("summarize-text"));
        assert!(is_valid_mcp_name("a-1-b-2"));
        assert!(!is_valid_mcp_name(""));
        assert!(!is_valid_mcp_name("-start"));
        assert!(!is_valid_mcp_name("end-"));
        assert!(!is_valid_mcp_name("CamelCase"));
        assert!(!is_valid_mcp_name("under_score"));
        assert!(!is_valid_mcp_name("double--dash"));
    }

    #[test]
    fn tool_serializes_input_schema_camel_case() {
        let t = McpTool::new("summarize")
            .with_description("Summarize text")
            .with_input_schema(json!({"type": "object"}));
        let v = serde_json::to_value(&t).unwrap();
        assert!(v.get("inputSchema").is_some());
        assert!(v.get("input_schema").is_none());
        let back: McpTool = serde_json::from_value(v).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn normalize_wire_object_rewrites_known_keys() {
        let donor = json!({
            "name": "echo",
            "input_schema": { "type": "object" },
            "mime_type": "text/plain",
            "nested": { "is_error": true, "list_changed": false }
        });
        let out = normalize_wire_object(donor);
        assert_eq!(out["inputSchema"]["type"], "object");
        assert_eq!(out["mimeType"], "text/plain");
        assert_eq!(out["nested"]["isError"], true);
        assert_eq!(out["nested"]["listChanged"], false);
        assert!(out.get("input_schema").is_none());
    }

    #[test]
    fn normalize_wire_object_leaves_camel_case_alone() {
        let already = json!({"inputSchema": {"type": "object"}, "isError": false});
        let out = normalize_wire_object(already.clone());
        assert_eq!(out, already);
    }

    #[test]
    fn content_block_text_serializes_mime_type() {
        let c = ContentBlock::text_with_mime("hi", "text/plain");
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["type"], "text");
        assert_eq!(v["mimeType"], "text/plain");
        assert!(v.get("mime_type").is_none());
    }

    #[test]
    fn content_block_from_wire_accepts_snake_case() {
        let donor = json!({"type": "text", "text": "hi", "mime_type": "text/plain"});
        let c = content_block_from_wire(donor).unwrap();
        match c {
            ContentBlock::Text { text, mime_type } => {
                assert_eq!(text, "hi");
                assert_eq!(mime_type.as_deref(), Some("text/plain"));
            }
            other => panic!("expected Text, got {other:?}"),
        }
    }

    #[test]
    fn tool_call_result_is_error_camel_case() {
        let r = ToolCallResult::err(vec![ContentBlock::text("fail")]);
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["isError"], true);
        assert!(v.get("is_error").is_none());
        let back: ToolCallResult = serde_json::from_value(v).unwrap();
        assert!(back.is_error);
        assert_eq!(back.extract_text(), "fail");
    }

    #[test]
    fn normalize_mcp_result_from_donor_snake_case() {
        let donor = json!({
            "content": [{"type": "text", "text": "ok", "mime_type": "text/plain"}],
            "is_error": false
        });
        let r = normalize_mcp_result(donor).unwrap();
        assert!(!r.is_error);
        assert_eq!(r.extract_text(), "ok");
    }

    #[test]
    fn from_normalized_always_has_input_schema() {
        let tool = NormalizedTool::new("calculator");
        let mcp = McpTool::from_normalized(&tool);
        assert_eq!(mcp.name, "calculator");
        let schema = mcp.input_schema.as_ref().expect("inputSchema present");
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"], json!({}));
        let v = serde_json::to_value(&mcp).unwrap();
        assert!(v.get("inputSchema").is_some());
        assert!(v.get("input_schema").is_none());
    }

    #[test]
    fn from_normalized_keeps_non_empty_parameter_map() {
        let mut params = apeireth_protocol::canonical::ToolParameters::new();
        params.insert("type".into(), json!("object"));
        params.insert(
            "properties".into(),
            json!({ "query": { "type": "string" } }),
        );
        let tool = NormalizedTool::new("sqlite-query").with_parameters(params);
        let mcp = McpTool::from_normalized(&tool);
        let schema = mcp.input_schema.expect("inputSchema present");
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["query"]["type"], "string");
    }

    #[test]
    fn image_and_resource_round_trip() {
        let cases = vec![
            ContentBlock::Image {
                data: "AAAA".into(),
                mime_type: "image/png".into(),
            },
            ContentBlock::Resource {
                uri: "file:///x.rs".into(),
                text: Some("hi".into()),
                mime_type: Some("text/x-rust".into()),
            },
        ];
        for c in cases {
            let v = content_block_to_wire(&c);
            assert!(v.get("mimeType").is_some());
            let back = content_block_from_wire(v).unwrap();
            assert_eq!(c, back);
        }
    }
}
