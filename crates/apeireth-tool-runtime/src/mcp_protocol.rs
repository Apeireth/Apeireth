//! R127-2 P6-2 — **opencode 子代理 重试** 阶段 2: Tool execution (MCP 协议)
//!
//! # 背景
//!
//! R125-12 (opencode 子代理) ⏳ 限流持续. R127-2 P6-2 重试: 不再依赖 opencode 源码,
//! 改借鉴已 cloned 的 `modelcontextprotocol/servers 175` (per decision-56 §3) MCP 协议.
//!
//! # 借鉴 ID
//!
//! - `R127-2-P6-2-BORROW-modelcontextprotocol/servers-175-mcp-protocol-2026-08-10` (主, ✅ cloned)
//! - `R125-12-BORROW-anomalyco/opencode-tool-execution-2026-08-10` (⏳ 限流, 0 装, 借 ID 索引已写)
//!
//! # 设计 (1:1 翻译 MCP TypeScript SDK 公开语义)
//!
//! **MCP Tool 协议** (per `servers/src/everything/tools/echo.ts`):
//! - `McpToolDefinition` — name / title / description / inputSchema / annotations
//! - `McpAnnotations` — readOnlyHint / destructiveHint / idempotentHint / openWorldHint
//! - `McpToolCall` — name / arguments
//! - `McpContent` enum — Text(String) / Image { data, mime_type } / Resource { uri, text }
//! - `McpToolResult` — content: Vec<McpContent>, is_error: bool
//!
//! **MCP Server** (per `McpServer.registerTool(name, config, handler)`):
//! - `McpServer` struct — register_tool + call_tool + list_tools
//! - `McpToolAdapter` — wraps existing `Tool` trait impl to MCP format (跨战役 2-1 集成)
//!
//! # 0 装 PASS 严守 (per 决策 #33 §2.3 C2 + 决策 #56 §3)
//!
//! - ✅ **cloned = 真实施** (servers 175 ✅ cloned, 8/11)
//! - ✅ **真 src 改动** (本文件 + `apeireth-tool-runtime/src/lib.rs` +1 `pub mod mcp_protocol;`)
//! - ✅ **tests pass** (10+ unit tests, `cargo test -p apeireth-tool-runtime`)
//! - ❌ **0 假装"已对接 MCP Python/TS 私有"** (我们自实现, 0 抄 MCP Python 代码)
//!
//! # 0 越界 8 硬墙 (per 决策 #33 §2.3 + 决策 #55 §4)
//!
//! - **B1** 24 LOCKED 入口签名 0 改 (本文件 + lib.rs 仅 +1 `pub mod mcp_protocol;`)
//! - **B2** workspace.version 1.2.0 0 改 (本文件 0 触碰 Cargo.toml)
//! - **A1** R11 baseline 3 值 0 改 (本文件 0 触碰 integration_r_measure.rs)
//! - **A3** 13 键 0 改 (本文件 0 触碰)
//! - **C1** 0 commit (Mavis 整合 #5 拍板, 等 Mavis 调度)
//! - **C2** 0 装 PASS 严守 (本文件 真 src 改动 + tests pass, 0 装"已对接 MCP 私有")

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

// 跨战役 2-1 集成: Tool trait 1:1 复用
use apeireth_tool_registry::{Tool, ToolKind};

/// 内部 helper: 把 `Value` enum variant 翻译成可读名字 (避免 `match` 嵌套)
fn arg_kind_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ============================================================
// 1. McpAnnotations (4 提示, 1:1 翻译 MCP TS annotations)
// ============================================================

/// **MCP Tool Annotations** (per `servers 175` echo.ts:17-21 1:1)
///
/// 4 提示, 跟 MCP TS 公开 SDK 一致:
/// - `read_only_hint` — 工具是否只读
/// - `destructive_hint` — 工具是否破坏性
/// - `idempotent_hint` — 工具是否幂等
/// - `open_world_hint` — 工具是否与外部世界交互
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAnnotations {
    /// 工具是否只读
    pub read_only_hint: bool,
    /// 工具是否破坏性
    pub destructive_hint: bool,
    /// 工具是否幂等 (同 input → 同 output)
    pub idempotent_hint: bool,
    /// 工具是否与外部世界交互 (e.g. HTTP, file)
    pub open_world_hint: bool,
}

impl McpAnnotations {
    /// 读-only 工具 annotations (e.g. echo, search)
    pub const READ_ONLY: Self = Self {
        read_only_hint: true,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: false,
    };

    /// 写破坏性工具 annotations (e.g. delete)
    pub const WRITE_DESTRUCTIVE: Self = Self {
        read_only_hint: false,
        destructive_hint: true,
        idempotent_hint: false,
        open_world_hint: true,
    };

    /// 写幂等工具 annotations (e.g. UI 渲染)
    pub const WRITE_IDEMPOTENT: Self = Self {
        read_only_hint: false,
        destructive_hint: false,
        idempotent_hint: true,
        open_world_hint: false,
    };
}

impl Default for McpAnnotations {
    fn default() -> Self {
        Self::WRITE_IDEMPOTENT
    }
}

// ============================================================
// 2. McpToolDefinition (1:1 翻译 MCP TS registerTool config)
// ============================================================

/// **MCP Tool Definition** (per `servers 175` echo.ts:11-22 1:1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    /// 工具唯一名
    pub name: String,
    /// 工具显示名
    pub title: String,
    /// 工具描述
    pub description: String,
    /// 工具输入 schema (JSON Schema, 用 `serde_json::Value` 表达)
    pub input_schema: Value,
    /// 工具 annotations
    pub annotations: McpAnnotations,
}

impl McpToolDefinition {
    /// 创建 1 个 MCP tool definition
    pub fn new(
        name: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
        input_schema: Value,
        annotations: McpAnnotations,
    ) -> Self {
        Self {
            name: name.into(),
            title: title.into(),
            description: description.into(),
            input_schema,
            annotations,
        }
    }
}

// ============================================================
// 3. McpContent (1:1 翻译 MCP TS CallToolResult content 数组)
// ============================================================

/// **MCP Content** (per `servers 175` echo.ts:36-38 1:1)
///
/// MCP TS 公开 SDK 返 `content: [{ type: "text", text: "..." }]`,
/// 我们 typed 化支持 text / image / resource 3 种 content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpContent {
    /// 文本内容
    Text {
        /// 文本
        text: String,
    },
    /// 图片内容
    Image {
        /// base64 编码的二进制数据
        data: String,
        /// MIME type (e.g. "image/png")
        mime_type: String,
    },
    /// 嵌入资源
    Resource {
        /// 资源 URI
        uri: String,
        /// 资源文本内容
        text: String,
        /// MIME type (optional)
        mime_type: Option<String>,
    },
}

impl McpContent {
    /// 创建 text content (per `servers 175` echo.ts:37 1:1)
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// content 类型
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Text { .. } => "text",
            Self::Image { .. } => "image",
            Self::Resource { .. } => "resource",
        }
    }
}

impl fmt::Display for McpContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text { text } => f.write_str(text),
            Self::Image { mime_type, .. } => write!(f, "[image:{mime_type}]"),
            Self::Resource { uri, .. } => write!(f, "[resource:{uri}]"),
        }
    }
}

// ============================================================
// 4. McpToolCall + McpToolResult (1:1 翻译 MCP TS CallToolResult)
// ============================================================

/// **MCP Tool Call** (per `servers 175` CallToolRequest 1:1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCall {
    /// 工具名
    pub name: String,
    /// 工具参数 (JSON Value)
    pub arguments: Value,
}

impl McpToolCall {
    /// 创建 1 个 tool call
    pub fn new(name: impl Into<String>, arguments: Value) -> Self {
        Self {
            name: name.into(),
            arguments,
        }
    }
}

/// **MCP Tool Result** (per `servers 175` CallToolResult 1:1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// 内容数组
    pub content: Vec<McpContent>,
    /// 是否错误
    pub is_error: bool,
}

impl McpToolResult {
    /// 创建 1 个成功 text result (per `servers 175` echo.ts:36-38 1:1)
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent::text(text)],
            is_error: false,
        }
    }

    /// 创建 1 个错误 result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent::text(message)],
            is_error: true,
        }
    }

    /// 创建 1 个 multi-content result
    pub fn multi(content: Vec<McpContent>, is_error: bool) -> Self {
        Self { content, is_error }
    }
}

// ============================================================
// 5. McpToolHandler + McpError (typed 错误, 跟 tool-runtime 错误流一致)
// ============================================================

/// **MCP Tool Handler** (per `servers 175` `McpServer.registerTool` 1:1 简化)
///
/// **0 装**: 接受 tool call 返 tool result, 跟 MCP TS `registerTool(name, config, async fn)` 1:1 简化
pub type McpToolHandler = Arc<
    dyn Fn(McpToolCall) -> Pin<Box<dyn Future<Output = Result<McpToolResult, McpError>> + Send>>
        + Send
        + Sync,
>;

/// **MCP 错误** (typed, 跟 `Tool` trait `Result<Value, String>` 1:1 简化)
#[derive(Debug, Error)]
pub enum McpError {
    /// 工具未注册
    #[error("mcp tool `{0}` not registered")]
    UnknownTool(String),
    /// handler 内部错误
    #[error("mcp tool `{tool}` failed: {message}")]
    Execution {
        /// 失败工具
        tool: String,
        /// 错误信息
        message: String,
    },
    /// Schema 校验失败
    #[error("mcp tool `{tool}` schema validation failed: {message}")]
    SchemaValidation {
        /// 失败工具
        tool: String,
        /// 错误信息
        message: String,
    },
}

// ============================================================
// 6. McpServer (1:1 翻译 MCP TS McpServer.registerTool + callTool)
// ============================================================

/// **MCP Server** (per `servers 175` `McpServer` 1:1 简化)
///
/// **设计**:
/// - 内部 `BTreeMap<String, (McpToolDefinition, McpToolHandler)>` — 决定 iteration 顺序
/// - `register_tool(definition, handler)` — 注册 1 个 tool
/// - `call_tool(call)` — 调 1 个 tool, 返 McpToolResult
/// - `list_tools()` — 列出所有 tool definitions
/// - 0 装 JSON-RPC transport: 仅 typed Rust API, 0 抄 MCP TS 传输层
pub struct McpServer {
    /// tool 注册表: name -> (definition, handler)
    tools: RwLock<BTreeMap<String, (McpToolDefinition, McpToolHandler)>>,
}

impl McpServer {
    /// 创建空 MCP server
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(BTreeMap::new()),
        }
    }

    /// 注册 1 个 tool (per `McpServer.registerTool` 1:1)
    ///
    /// **0 装**: 同 name 重复注册覆盖 (跟 MCP TS `registerTool` 行为一致)
    pub fn register_tool(&self, definition: McpToolDefinition, handler: McpToolHandler) {
        let name = definition.name.clone();
        self.tools.write().insert(name, (definition, handler));
    }

    /// 调 1 个 tool (per `McpServer.callTool` 1:1)
    pub async fn call_tool(&self, call: McpToolCall) -> Result<McpToolResult, McpError> {
        let entry = {
            let tools = self.tools.read();
            tools
                .get(&call.name)
                .cloned()
                .ok_or_else(|| McpError::UnknownTool(call.name.clone()))?
        };
        let (def, handler) = entry;
        // Schema 校验 (简化: 校验 arguments 是 object 或 null)
        if !call.arguments.is_object() && !call.arguments.is_null() {
            return Err(McpError::SchemaValidation {
                tool: def.name,
                message: format!(
                    "expected object arguments, got {}",
                    arg_kind_name(&call.arguments)
                ),
            });
        }
        handler(call).await
    }

    /// 列出所有 tool definitions
    pub fn list_tools(&self) -> Vec<McpToolDefinition> {
        self.tools
            .read()
            .values()
            .map(|(def, _)| def.clone())
            .collect()
    }

    /// 工具数
    pub fn tool_count(&self) -> usize {
        self.tools.read().len()
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// 7. McpToolAdapter (跨战役 2-1 集成, wraps existing `Tool` to MCP format)
// ============================================================

/// **MCP Tool Adapter** (1:1 适配 `Tool` trait → `McpToolHandler`)
///
/// **设计**:
/// - 持有 `Arc<dyn Tool>` (战役 2-1 真 tool)
/// - 提供 `to_mcp_definition()` 把 `Tool` 描述转 MCP definition
/// - 提供 `to_mcp_handler()` 把 `Tool::call()` 包成 `McpToolHandler`
///
/// **0 装**: 仅 typed adapter, 0 抄 MCP TS 任何代码
pub struct McpToolAdapter {
    /// 内部 tool (战役 2-1)
    tool: Arc<dyn Tool>,
    /// 工具输入 schema (JSON Schema, 简化: `{"type": "object"}`)
    input_schema: Value,
    /// 工具 annotations
    annotations: McpAnnotations,
}

impl McpToolAdapter {
    /// 创建 1 个 adapter
    pub fn new(tool: Arc<dyn Tool>) -> Self {
        Self {
            tool,
            input_schema: serde_json::json!({"type": "object"}),
            annotations: McpAnnotations::default(),
        }
    }

    /// 自定义 annotations
    pub fn with_annotations(mut self, annotations: McpAnnotations) -> Self {
        self.annotations = annotations;
        self
    }

    /// 自定义 input schema
    pub fn with_input_schema(mut self, schema: Value) -> Self {
        self.input_schema = schema;
        self
    }

    /// 转 MCP tool definition
    pub fn to_mcp_definition(&self) -> McpToolDefinition {
        McpToolDefinition::new(
            self.tool.name(),
            self.tool.name(), // title = name 简化
            format!("Tool from ToolRegistry: {}", self.tool.name()),
            self.input_schema.clone(),
            self.annotations,
        )
    }

    /// 转 MCP handler (per `McpServer.registerTool` 1:1 简化)
    pub fn to_mcp_handler(&self) -> McpToolHandler {
        let tool = self.tool.clone();
        Arc::new(move |call: McpToolCall| {
            let tool = tool.clone();
            Box::pin(async move {
                // 调战役 2-1 tool call, 把 Value 错 (Result<Value, String>) 转 McpError
                tool.call(call.arguments)
                    .await
                    .map(|v| {
                        // 把 Value 转 text content (简化)
                        let text = match &v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        McpToolResult::text(text)
                    })
                    .map_err(|msg| McpError::Execution {
                        tool: call.name,
                        message: msg,
                    })
            })
        })
    }
}

// ============================================================
// 8. 编译期 hardcode (主哲学锚 #1 不漂移 + #6 工程铁律)
// ============================================================

/// 4 MCP annotations 提示数 (编译期 hardcode)
pub const MCP_ANNOTATION_COUNT: usize = 4;

/// 3 MCP content 类型数 (text / image / resource)
pub const MCP_CONTENT_TYPE_COUNT: usize = 3;

const _: () = {
    assert!(
        MCP_ANNOTATION_COUNT == 4,
        "MCP_ANNOTATION_COUNT = 4 (readOnly/destructive/idempotent/openWorld)"
    );
    assert!(
        MCP_CONTENT_TYPE_COUNT == 3,
        "MCP_CONTENT_TYPE_COUNT = 3 (text/image/resource)"
    );
};

// ============================================================
// 9. 单元测试 (10+ tests, 0 装 PASS 严守)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::MockSyncTool;

    #[test]
    fn mcp_annotations_constants_distinct() {
        let r = McpAnnotations::READ_ONLY;
        let w = McpAnnotations::WRITE_DESTRUCTIVE;
        let i = McpAnnotations::WRITE_IDEMPOTENT;
        assert!(r.read_only_hint);
        assert!(w.destructive_hint);
        assert!(i.idempotent_hint);
        assert_ne!(r, w);
        assert_ne!(r, i);
        assert_ne!(w, i);
    }

    #[test]
    fn mcp_annotations_default_idempotent() {
        let d = McpAnnotations::default();
        assert_eq!(d, McpAnnotations::WRITE_IDEMPOTENT);
    }

    #[test]
    fn mcp_content_3_kinds_distinct() {
        let t = McpContent::text("hi");
        let i = McpContent::Image {
            data: "AAAA".to_string(),
            mime_type: "image/png".to_string(),
        };
        let r = McpContent::Resource {
            uri: "file://x".to_string(),
            text: "x".to_string(),
            mime_type: None,
        };
        assert_eq!(t.kind(), "text");
        assert_eq!(i.kind(), "image");
        assert_eq!(r.kind(), "resource");
        assert_eq!(format!("{t}"), "hi");
    }

    #[test]
    fn mcp_tool_result_text_and_error() {
        let ok = McpToolResult::text("ok");
        assert!(!ok.is_error);
        assert_eq!(ok.content.len(), 1);
        let err = McpToolResult::error("bad");
        assert!(err.is_error);
    }

    #[test]
    fn mcp_tool_definition_construct() {
        let def = McpToolDefinition::new(
            "echo",
            "Echo",
            "echoes back",
            serde_json::json!({"type": "object", "properties": {"message": {"type": "string"}}}),
            McpAnnotations::READ_ONLY,
        );
        assert_eq!(def.name, "echo");
        assert_eq!(def.title, "Echo");
        assert!(def.annotations.read_only_hint);
    }

    #[test]
    fn mcp_server_register_and_list() {
        let server = McpServer::new();
        assert_eq!(server.tool_count(), 0);

        // 注册 1 个 echo 工具
        let handler: McpToolHandler = Arc::new(|call| {
            Box::pin(async move {
                let msg = call
                    .arguments
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none");
                Ok(McpToolResult::text(format!("Echo: {msg}")))
            })
        });
        let def = McpToolDefinition::new(
            "echo",
            "Echo Tool",
            "Echoes back the input string",
            serde_json::json!({"type": "object"}),
            McpAnnotations::READ_ONLY,
        );
        server.register_tool(def, handler);
        assert_eq!(server.tool_count(), 1);
        assert_eq!(server.list_tools()[0].name, "echo");
    }

    #[tokio::test]
    async fn mcp_server_call_tool_echo() {
        let server = McpServer::new();
        let handler: McpToolHandler = Arc::new(|call| {
            Box::pin(async move {
                let msg = call
                    .arguments
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none");
                Ok(McpToolResult::text(format!("Echo: {msg}")))
            })
        });
        let def = McpToolDefinition::new(
            "echo",
            "Echo Tool",
            "Echoes back the input string",
            serde_json::json!({"type": "object"}),
            McpAnnotations::READ_ONLY,
        );
        server.register_tool(def, handler);

        let call = McpToolCall::new("echo", serde_json::json!({"message": "hi"}));
        let result = server.call_tool(call).await.unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content[0].kind(), "text");
        assert!(format!("{}", result.content[0]).contains("hi"));
    }

    #[tokio::test]
    async fn mcp_server_call_unknown_tool_errors() {
        let server = McpServer::new();
        let call = McpToolCall::new("not-exist", serde_json::json!({}));
        let err = server.call_tool(call).await.unwrap_err();
        assert!(matches!(err, McpError::UnknownTool(_)));
    }

    #[tokio::test]
    async fn mcp_server_schema_validation_non_object_errors() {
        let server = McpServer::new();
        let handler: McpToolHandler = Arc::new(|_call| {
            Box::pin(async move { Ok(McpToolResult::text("ok")) })
        });
        let def = McpToolDefinition::new(
            "echo",
            "Echo",
            "echo",
            serde_json::json!({"type": "object"}),
            McpAnnotations::READ_ONLY,
        );
        server.register_tool(def, handler);

        // 传 string (非 object 也非 null) → schema validation 失败
        let call = McpToolCall::new("echo", serde_json::json!("not-an-object"));
        let err = server.call_tool(call).await.unwrap_err();
        assert!(matches!(err, McpError::SchemaValidation { .. }));
    }

    #[tokio::test]
    async fn mcp_tool_adapter_wraps_existing_tool() {
        // 跨战役 2-1 集成: 用 MockSyncTool 包成 MCP
        let mock = Arc::new(MockSyncTool {
            name: "mock_echo".to_string(),
        });
        let adapter = McpToolAdapter::new(mock).with_annotations(McpAnnotations::READ_ONLY);

        let def = adapter.to_mcp_definition();
        assert_eq!(def.name, "mock_echo");
        assert!(def.annotations.read_only_hint);

        // 调 handler (MockSyncTool 返 tool name 自身)
        let handler = adapter.to_mcp_handler();
        let call = McpToolCall::new("mock_echo", serde_json::json!({}));
        let result = handler(call).await.unwrap();
        assert!(!result.is_error);
        assert!(format!("{}", result.content[0]).contains("mock_echo"));
    }

    #[test]
    fn mcp_tool_call_serialize_deserialize() {
        let call = McpToolCall::new("echo", serde_json::json!({"message": "hi"}));
        let json = serde_json::to_string(&call).unwrap();
        let back: McpToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(call.name, back.name);
        assert_eq!(call.arguments, back.arguments);
    }
}
