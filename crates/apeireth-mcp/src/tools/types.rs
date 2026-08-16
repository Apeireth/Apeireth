//! R125-4: MCP tools protocol — types module
//!
//! **拆分自原 `tools/mod.rs` (R65)**: 1 大文件 → 5 子文件, per-tool 1 file (借鉴
//! `modelcontextprotocol/servers/src/everything/tools/<tool_name>.ts` 模式).
//!
//! **本文件**: `Tool` / `ToolContent` / `ToolCallResult` 3 个核心 struct
//!  (per MCP spec §tools/list item + §tools/call content[] item + §tools/call result).
//!
//! **不漂移 (主哲学锚 #1, 8 硬墙 #3)**:
//! - 0 改 struct pub field 名 (入口签名 0 改, R125-4 内部 fn 实施可改授权)
//! - 0 改 Serialize/Deserialize 行为 (per spec 字段 1:1)
//! - 0 改 ToolContent enum 变体 (text/image/resource 跟原 mod.rs 1:1)
//! - 0 引入 I/O / 网络 (server 注入, 0 真接)

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================
// MCP 错误码 (per MCP spec, -32000 ~ -32099 范围 server-define)
// ============================================================

/// **tool not found** 错误码 (per MCP spec, server-define)
pub const TOOL_NOT_FOUND: i32 = -32010;
/// **tool invalid args** 错误码 (per MCP spec, server-define)
pub const TOOL_INVALID_ARGS: i32 = -32011;
/// **tool call failed** 错误码 (per MCP spec, server-define)
pub const TOOL_CALL_FAILED: i32 = -32012;
/// **tool internal error** 错误码 (per MCP spec, server-define)
pub const TOOL_INTERNAL: i32 = -32013;

// ============================================================
// Tool (per spec §tools/list item)
// ============================================================

/// **MCP Tool (per spec §tools/list item)**
///
/// ```json
/// { "name": "summarize", "description": "...", "inputSchema": { "type": "object", ... } }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tool {
    /// 工具名 (kebab-case, e.g. "summarize-text")
    pub name: String,
    /// 工具描述
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// 输入参数 JSON Schema (per MCP spec §tools inputSchema)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
}

impl Tool {
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
}

// ============================================================
// ToolContent (per spec §tools/call content[] item)
// ============================================================

/// **MCP ToolContent (per spec §tools/call content[] item)**
///
/// 至少 1 个 content block (text / image / resource), VCP + LangChain 双借鉴统一.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContent {
    /// 文本块
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    /// 图片块 (per MCP spec, base64)
    Image { data: String, mime_type: String },
    /// 资源引用块 (per MCP spec, embed resource URI)
    Resource {
        uri: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

impl ToolContent {
    /// 构造一个文本块
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mime_type: None,
        }
    }
    /// 构造一个带 mime type 的文本块
    pub fn text_with_mime(text: impl Into<String>, mime: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            mime_type: Some(mime.into()),
        }
    }
}

// ============================================================
// ToolCallResult (per spec §tools/call result)
// ============================================================

/// **MCP ToolCallResult (per spec §tools/call result)**
///
/// ```json
/// { "content": [...], "isError": false }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    #[serde(default)]
    pub is_error: bool,
}

impl ToolCallResult {
    /// 构造成功结果
    pub fn ok(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: false,
        }
    }
    /// 构造错误结果
    pub fn err(content: Vec<ToolContent>) -> Self {
        Self {
            content,
            is_error: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_new_basic() {
        let t = Tool::new("summarize");
        assert_eq!(t.name, "summarize");
        assert!(t.description.is_none());
        assert!(t.input_schema.is_none());
    }

    #[test]
    fn tool_with_description_and_schema() {
        let t = Tool::new("summarize")
            .with_description("Summarize text")
            .with_input_schema(json!({"type": "object"}));
        assert_eq!(t.description.as_deref(), Some("Summarize text"));
        assert!(t.input_schema.is_some());
    }

    #[test]
    fn tool_content_text_constructors() {
        let c = ToolContent::text("hi");
        match c {
            ToolContent::Text { text, mime_type } => {
                assert_eq!(text, "hi");
                assert!(mime_type.is_none());
            }
            _ => panic!("expected Text"),
        }
        let c2 = ToolContent::text_with_mime("hi", "text/plain");
        match c2 {
            ToolContent::Text { text, mime_type } => {
                assert_eq!(text, "hi");
                assert_eq!(mime_type.as_deref(), Some("text/plain"));
            }
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn tool_call_result_ok_err() {
        let ok = ToolCallResult::ok(vec![ToolContent::text("ok")]);
        assert!(!ok.is_error);
        let err = ToolCallResult::err(vec![ToolContent::text("fail")]);
        assert!(err.is_error);
    }

    #[test]
    fn tool_serialize_round_trip() {
        let t = Tool::new("test-tool")
            .with_description("A test tool")
            .with_input_schema(json!({"type": "object", "properties": {}}));
        let json_str = serde_json::to_string(&t).unwrap();
        let restored: Tool = serde_json::from_str(&json_str).unwrap();
        assert_eq!(t, restored);
    }

    #[test]
    fn tool_content_serialize_round_trip() {
        let cases = vec![
            ToolContent::text("hello"),
            ToolContent::text_with_mime("hello", "text/plain"),
            ToolContent::Image {
                data: "AAAA".into(),
                mime_type: "image/png".into(),
            },
            ToolContent::Resource {
                uri: "file:///x.rs".into(),
                text: Some("hi".into()),
                mime_type: Some("text/x-rust".into()),
            },
        ];
        for c in cases {
            let json_str = serde_json::to_string(&c).unwrap();
            let restored: ToolContent = serde_json::from_str(&json_str).unwrap();
            assert_eq!(c, restored);
        }
    }
}
