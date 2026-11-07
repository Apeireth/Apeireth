//! MCP wire-schema normalization.
//!
//! The earlier `apeireth-mcp` stored tool/resource fields as Rust snake_case
//! (`input_schema`, `mime_type`, `is_error`) **without** serde renames, so
//! the JSON it emitted did not match MCP 2025-03-26 (`inputSchema`,
//! `mimeType`, `isError`). The v2 client in `apeireth-tools::mcp` already
//! renames `inputSchema` / `isError` on its own types; this module is the
//! shared library so a later host (and resource/prompt models) speak the
//! same wire.
//!
//! Also included: kebab-case tool/prompt name check
//! and a tolerant object-key rewrite that accepts
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
    ("structured_content", "structuredContent"),
];

/// Recursively rewrite object keys that have a known MCP camelCase form.
///
/// Already-camelCase keys are left alone. Arrays are walked. Non-objects
/// are returned unchanged. Used to ingest earlier JSON that was serialized
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

/// MCP tool / prompt name (baseline `is_valid_tool_name`).
///
/// Rules: non-empty; ASCII lowercase / digit / `_` / `-`; no leading or
/// trailing `-`; no `--`. Underscores are accepted: the MCP ecosystem (and
/// the spec) permits snake_case tool names, and rejecting them breaks
/// interop with servers such as REA.
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
            if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
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
///
/// The **structured channel** is a first-class field: a server that answers
/// with a structured result carries it in [`ToolCallResult::structured`]. The
/// field was missing once, and serde's default (ignore unknown fields) then
/// dropped the structured payload on the floor — the channel was broken and
/// nothing said so.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallResult {
    pub content: Vec<ContentBlock>,
    #[serde(default, rename = "isError")]
    pub is_error: bool,
    /// The server's structured result payload (`structuredContent`).
    ///
    /// `None` = the server sent no structured payload. The payload is
    /// **server-authored**: whoever hands it to the model is responsible for
    /// bounding and sweeping it (see the bridge's result face).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "structuredContent"
    )]
    pub structured: Option<Value>,
}

/// `ToolCallResult` is an equivalence relation, so the `Eq` marker is sound.
///
/// `serde_json::Value` does not implement `Eq` (it could hold a float `NaN`,
/// and `NaN != NaN`). JSON cannot express `NaN` and `serde_json` refuses to
/// build one, so the derived `PartialEq` stays reflexive here and the marker
/// may be stated. Without this the type would lose `Eq` — a public contract
/// other crates may rely on.
impl Eq for ToolCallResult {}

impl ToolCallResult {
    pub fn ok(content: Vec<ContentBlock>) -> Self {
        Self {
            content,
            is_error: false,
            structured: None,
        }
    }

    pub fn err(content: Vec<ContentBlock>) -> Self {
        Self {
            content,
            is_error: true,
            structured: None,
        }
    }

    /// The same result, carrying the server's structured payload.
    #[must_use]
    pub fn with_structured(mut self, structured: Value) -> Self {
        self.structured = Some(structured);
        self
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

/// Normalize a tools/call result object that may use baseline snake_case.
///
/// The **structured payload** is data, not an envelope: its own keys are the
/// server's data keys, so they are kept exactly as the server wrote them.
/// Only the envelope around them is normalized — otherwise a data key such as
/// `is_error` would silently change name on the way to the model.
pub fn normalize_mcp_result(value: Value) -> Result<ToolCallResult, String> {
    let (envelope, structured) = split_structured_payload(value);
    let mut normalized = normalize_wire_object(envelope);
    if let Some(structured) = structured {
        let object = normalized
            .as_object_mut()
            .ok_or_else(|| "a tools/call result must be an object".to_string())?;
        object.insert("structuredContent".to_string(), structured);
    }
    serde_json::from_value(normalized).map_err(|e| e.to_string())
}

/// Take the structured payload out of a tools/call result envelope.
///
/// The payload is taken out **before** the tolerant key rewrite, so the
/// rewrite never reaches the server's own data keys.
fn split_structured_payload(value: Value) -> (Value, Option<Value>) {
    let Value::Object(mut map) = value else {
        return (value, None);
    };
    let structured = map
        .remove("structuredContent")
        .or_else(|| map.remove("structured_content"));
    (Value::Object(map), structured)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_mcp_name_kebab_and_snake() {
        assert!(is_valid_mcp_name("echo"));
        assert!(is_valid_mcp_name("summarize-text"));
        assert!(is_valid_mcp_name("a-1-b-2"));
        // MCP 生态常见 snake_case（如 REA 的 inspect_artifact）。
        assert!(is_valid_mcp_name("under_score"));
        assert!(is_valid_mcp_name("inspect_artifact"));
        assert!(is_valid_mcp_name("annotate_native_function"));
        assert!(!is_valid_mcp_name(""));
        assert!(!is_valid_mcp_name("-start"));
        assert!(!is_valid_mcp_name("end-"));
        assert!(!is_valid_mcp_name("CamelCase"));
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
        let payload = json!({
            "name": "echo",
            "input_schema": { "type": "object" },
            "mime_type": "text/plain",
            "nested": { "is_error": true, "list_changed": false }
        });
        let out = normalize_wire_object(payload);
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
        let payload = json!({"type": "text", "text": "hi", "mime_type": "text/plain"});
        let c = content_block_from_wire(payload).unwrap();
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
    fn normalize_mcp_result_from_baseline_snake_case() {
        let payload = json!({
            "content": [{"type": "text", "text": "ok", "mime_type": "text/plain"}],
            "is_error": false
        });
        let r = normalize_mcp_result(payload).unwrap();
        assert!(!r.is_error);
        assert_eq!(r.extract_text(), "ok");
    }

    #[test]
    fn the_structured_channel_survives_the_normalization() {
        let payload = json!({
            "content": [{"type": "text", "text": "42"}],
            "isError": false,
            "structuredContent": { "answer": 42 }
        });
        let r = normalize_mcp_result(payload).unwrap();
        assert_eq!(
            r.structured,
            Some(json!({ "answer": 42 })),
            "the structured payload must reach the caller"
        );
    }

    #[test]
    fn the_structured_channel_survives_the_baseline_snake_case_shape() {
        let payload = json!({
            "content": [{"type": "text", "text": "42"}],
            "is_error": false,
            "structured_content": { "answer": 42 }
        });
        let r = normalize_mcp_result(payload).unwrap();
        assert_eq!(r.structured, Some(json!({ "answer": 42 })));
    }

    #[test]
    fn the_structured_payloads_own_data_keys_are_not_rewritten() {
        let payload = json!({
            "content": [{"type": "text", "text": "one row"}],
            "isError": false,
            "structuredContent": { "rows": [{ "is_error": false }] }
        });
        let r = normalize_mcp_result(payload).unwrap();
        let rows = r.structured.as_ref().expect("the payload is there");
        assert_eq!(rows["rows"][0]["is_error"], json!(false));
        assert!(
            rows["rows"][0].get("isError").is_none(),
            "a data key must not be rewritten into the envelope's dialect"
        );
    }

    #[test]
    fn an_answer_without_a_structured_payload_carries_none() {
        let payload = json!({
            "content": [{"type": "text", "text": "ok"}],
            "isError": false
        });
        let r = normalize_mcp_result(payload).unwrap();
        assert_eq!(r.structured, None);
        // The old shape is unchanged: the field stays off the wire.
        let wire = serde_json::to_value(&r).unwrap();
        assert!(wire.get("structuredContent").is_none());
    }

    #[test]
    fn the_structured_payload_round_trips_on_the_wire() {
        let r = ToolCallResult::ok(vec![ContentBlock::text("42")])
            .with_structured(json!({ "answer": 42 }));
        let wire = serde_json::to_value(&r).unwrap();
        assert_eq!(wire["structuredContent"], json!({ "answer": 42 }));
        let back: ToolCallResult = serde_json::from_value(wire).unwrap();
        assert_eq!(back, r);
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
