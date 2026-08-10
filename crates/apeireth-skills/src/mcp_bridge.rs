//! R86: Skill → MCP `tools` 桥接 (SkillDescriptor → ToolServer)
//!
//! **目标**: 让 apeireth-skills 的 SkillDescriptor 可以作为 MCP server 暴露,
//! 任意 MCP client (Claude Desktop / IDE / ...) 都能 list/call.
//!
//! **Apeireth 真接 (本 module)**:
//! - `SkillToolServer` 实现 `apeireth_mcp::tools::ToolServer`
//!   - `list()` — 每个 SkillDescriptor → Tool (name=id, description, inputSchema from input_example JSON parse)
//!   - `call(name, args)` — 按 name 找 descriptor, 返 `output_example` 作 result, or 走 custom handler
//! - `SkillCallHandler` trait — 让消费者 (TUI/council/...) 注入真执行逻辑, 0 假装
//! - `SkillToolServer::new(descriptors)` — 纯 metadata 模式 (call 返 output_example, 标 isError=false)
//! - `SkillToolServer::with_handler(descriptors, handler)` — 委托模式 (handler 真接执行)
//! - `descriptor_to_tool(desc)` — 单 descriptor 转 Tool (便利函数, 复用)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `apeireth-skills/src/lib.rs` 已有 8 pub fn / Skill / Registry (R23 LOCKED)
//! - 0 改 `apeireth-skills/src/descriptor.rs` 已有 SkillDescriptor 7 字段
//! - 0 改 `apeireth-mcp/src/tools.rs` 已有 Tool / ToolServer / handle_tools_* (R65 LOCKED)
//! - 0 引入 I/O / 网络 (handler 由 caller 注入, 0 自创 I/O)
//!
//! **借鉴锚 (S-4)**:
//! - VCP `vcptoolbox/modules` 1:1 字段映射 (description / tags / source / examples)
//! - LangChain `@tool` decorator (Skill 整体作为 Tool, 自动 description + schema)
//! - MCP spec 2025-03-26 §tools (Tool.name + Tool.description + Tool.inputSchema)

use std::sync::Arc;

use apeireth_mcp::protocol::JsonRpcError;
use apeireth_mcp::tools::{Tool, ToolCallResult, ToolContent, ToolServer, TOOL_NOT_FOUND};
use serde_json::{json, Value};

use crate::descriptor::SkillDescriptor;

// ============================================================
// SkillCallHandler trait — 让消费者注入真执行逻辑 (0 假装)
// ============================================================

/// **Skill call handler 抽象** — 消费者实现这个 trait 来注入真执行逻辑.
///
/// 返回 `ToolCallResult` (MCP §tools/call 1:1). 实现者可以是:
/// - TUI dispatcher (按 skill id 调本地命令)
/// - Council executor (走 council deliberation)
/// - Eval runner (走 eval scenario)
/// - HTTP proxy (转发到 remote skill runtime)
///
/// 不实现 trait 的消费者用 `SkillToolServer::new(descriptors)` (纯 metadata 模式),
/// call 返 `output_example` 字符串.
pub trait SkillCallHandler: Send + Sync {
    /// 按 skill id + 实际 args 调, 返 MCP ToolCallResult
    fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError>;
}

// ============================================================
// SkillToolServer
// ============================================================

/// **MCP ToolServer impl, 把 SkillDescriptor 列表暴露为 MCP tools**
///
/// 两种模式:
/// 1. `new(descriptors)` — 纯 metadata 模式, call 返 `output_example` 字符串
/// 2. `with_handler(descriptors, handler)` — 委托模式, call 走 custom handler
pub struct SkillToolServer {
    descriptors: Vec<SkillDescriptor>,
    handler: Option<Arc<dyn SkillCallHandler>>,
}

impl std::fmt::Debug for SkillToolServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkillToolServer")
            .field("count", &self.descriptors.len())
            .field("has_handler", &self.handler.is_some())
            .finish()
    }
}

impl SkillToolServer {
    /// **纯 metadata 模式** — call 返 `output_example` 字符串 (0 假装, 字段真值)
    pub fn new(descriptors: Vec<SkillDescriptor>) -> Self {
        Self {
            descriptors,
            handler: None,
        }
    }

    /// **委托模式** — call 走 caller 提供的 handler (真执行逻辑)
    pub fn with_handler(
        descriptors: Vec<SkillDescriptor>,
        handler: Arc<dyn SkillCallHandler>,
    ) -> Self {
        Self {
            descriptors,
            handler: Some(handler),
        }
    }

    /// 加 1 个 descriptor (运行时 hot-add)
    pub fn add(&mut self, desc: SkillDescriptor) {
        self.descriptors.push(desc);
    }

    /// 当前暴露的 skill 数量
    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    /// 按 name 找 descriptor (helper, 给 handler 用)
    pub fn find(&self, name: &str) -> Option<&SkillDescriptor> {
        self.descriptors.iter().find(|d| d.id == name)
    }
}

// ============================================================
// ToolServer trait impl
// ============================================================

impl ToolServer for SkillToolServer {
    fn list(&self) -> Vec<Tool> {
        self.descriptors.iter().map(descriptor_to_tool).collect()
    }

    fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError> {
        // 1. 找 descriptor
        let desc = self.find(name).ok_or_else(|| {
            JsonRpcError::new(
                TOOL_NOT_FOUND,
                format!("skill `{}` not found in SkillToolServer", name),
            )
        })?;

        // 2. 委托 handler (真接) or metadata fallback (返 output_example, 标 simulated)
        if let Some(handler) = &self.handler {
            return handler.call(name, arguments);
        }

        // metadata 模式: 返 output_example + 注解 (0 假装: 标 is_error=false 但 content 注明)
        let output_value: Value = serde_json::from_str(&desc.output_example).unwrap_or_else(|_| {
            json!({ "raw": desc.output_example, "note": "output_example 非 JSON 字符串, 原样回传" })
        });
        let text = serde_json::to_string_pretty(&output_value)
            .unwrap_or_else(|_| desc.output_example.clone());
        Ok(ToolCallResult::ok(vec![ToolContent::text_with_mime(
            format!(
                "[metadata mode] skill `{}` v{} → 静态 output_example (无 handler 注入)\n\n{}",
                desc.id, desc.version, text
            ),
            "application/json",
        )]))
    }
}

// ============================================================
// 便利函数
// ============================================================

/// **单个 SkillDescriptor → Tool 转换** (per MCP spec §tools/list item 1:1)
///
/// - name = descriptor.id
/// - description = descriptor.description
/// - input_schema = parse descriptor.input_example as JSON Schema (best-effort;
///   if parse fails, fall back to {"type": "object"})
pub fn descriptor_to_tool(desc: &SkillDescriptor) -> Tool {
    // 解析 input_example 当 JSON Schema
    let input_schema = serde_json::from_str::<Value>(&desc.input_example)
        .unwrap_or_else(|_| {
            json!({
                "type": "object",
                "description": format!("input schema for skill `{}` (未提供 JSON Schema, 用 object fallback)", desc.id),
            })
        });
    let description = if desc.tags.is_empty() {
        desc.description.clone()
    } else {
        format!("{} [tags: {}]", desc.description, desc.tags.iter().cloned().collect::<Vec<_>>().join(", "))
    };
    Tool::new(desc.id.clone())
        .with_description(description)
        .with_input_schema(input_schema)
}

// ============================================================
// 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn make_desc(id: &str, version: &str, description: &str, tags: &[&str], source: &str) -> SkillDescriptor {
        SkillDescriptor::new(
            id,
            version,
            description,
            tags.iter().map(|s| (*s).to_string()),
            source,
        )
        .with_examples(
            json!({"input": "test"}).to_string(),
            json!({"output": "ok"}).to_string(),
        )
    }

    struct EchoHandler;
    impl SkillCallHandler for EchoHandler {
        fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError> {
            Ok(ToolCallResult::ok(vec![ToolContent::text_with_mime(
                format!("handler: skill={}, args={}", name, arguments),
                "application/json",
            )]))
        }
    }

    struct AlwaysFailHandler;
    impl SkillCallHandler for AlwaysFailHandler {
        fn call(&self, _name: &str, _arguments: &Value) -> Result<ToolCallResult, JsonRpcError> {
            Err(JsonRpcError::new(-32099, "handler always fails (test)"))
        }
    }

    #[test]
    fn descriptor_to_tool_basic() {
        let desc = make_desc("summarize-text", "1.0.0", "Summarize text", &["summarize", "text"], "vcptoolbox");
        let tool = descriptor_to_tool(&desc);
        assert_eq!(tool.name, "summarize-text");
        assert!(tool.description.is_some());
        assert!(tool.description.unwrap().contains("Summarize text"));
        assert!(tool.input_schema.is_some());
    }

    #[test]
    fn descriptor_to_tool_without_tags() {
        let desc = make_desc("plain", "1.0.0", "Plain skill", &[], "local");
        let tool = descriptor_to_tool(&desc);
        assert_eq!(tool.name, "plain");
        assert_eq!(tool.description.as_deref(), Some("Plain skill"));
    }

    #[test]
    fn descriptor_to_tool_with_invalid_input_example_uses_fallback() {
        let desc = SkillDescriptor::new(
            "bad-input",
            "1.0.0",
            "Skill with bad input example",
            vec![],
            "local",
        )
        .with_examples("not-json-{{{", "{}");
        let tool = descriptor_to_tool(&desc);
        assert_eq!(tool.name, "bad-input");
        let schema = tool.input_schema.unwrap();
        assert_eq!(schema["type"], "object");
    }

    #[test]
    fn skill_tool_server_new_lists_all() {
        let server = SkillToolServer::new(vec![
            make_desc("a", "1.0.0", "Skill A", &[], "local"),
            make_desc("b", "1.0.0", "Skill B", &[], "local"),
        ]);
        let tools = server.list();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "a");
        assert_eq!(tools[1].name, "b");
    }

    #[test]
    fn skill_tool_server_add_runtime() {
        let mut server = SkillToolServer::new(vec![]);
        assert!(server.is_empty());
        server.add(make_desc("late", "1.0.0", "Late add", &[], "local"));
        assert_eq!(server.len(), 1);
        let tools = server.list();
        assert_eq!(tools[0].name, "late");
    }

    #[test]
    fn skill_tool_server_find_by_name() {
        let server = SkillToolServer::new(vec![
            make_desc("alpha", "1.0.0", "Alpha", &[], "local"),
            make_desc("beta", "1.0.0", "Beta", &[], "local"),
        ]);
        assert!(server.find("alpha").is_some());
        assert!(server.find("nope").is_none());
    }

    #[test]
    fn skill_tool_server_metadata_mode_call_returns_output_example() {
        let server = SkillToolServer::new(vec![
            make_desc("mock", "1.0.0", "Mock skill", &[], "test"),
        ]);
        let result = server.call("mock", &json!({})).unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        match &result.content[0] {
            ToolContent::Text { text, .. } => {
                assert!(text.contains("mock"));
                assert!(text.contains("[metadata mode]"));
            }
            _ => panic!("expected Text content"),
        }
    }

    #[test]
    fn skill_tool_server_metadata_mode_call_unknown_skill_errors() {
        let server = SkillToolServer::new(vec![
            make_desc("known", "1.0.0", "Known", &[], "test"),
        ]);
        let err = server.call("nope", &json!({})).unwrap_err();
        assert_eq!(err.code, TOOL_NOT_FOUND);
        assert!(err.message.contains("nope"));
    }

    #[test]
    fn skill_tool_server_with_handler_delegates_call() {
        let server = SkillToolServer::with_handler(
            vec![make_desc("delegated", "1.0.0", "Delegated skill", &[], "test")],
            Arc::new(EchoHandler),
        );
        let result = server.call("delegated", &json!({"x": 1})).unwrap();
        assert!(!result.is_error);
        match &result.content[0] {
            ToolContent::Text { text, .. } => {
                assert!(text.contains("handler:"));
                assert!(text.contains("delegated"));
            }
            _ => panic!("expected Text content"),
        }
    }

    #[test]
    fn skill_tool_server_handler_can_fail() {
        let server = SkillToolServer::with_handler(
            vec![make_desc("always-fail", "1.0.0", "Failing skill", &[], "test")],
            Arc::new(AlwaysFailHandler),
        );
        let err = server.call("always-fail", &json!({})).unwrap_err();
        assert_eq!(err.code, -32099);
    }

    #[test]
    fn skill_tool_server_handler_unknown_skill_errors() {
        let server = SkillToolServer::with_handler(
            vec![make_desc("known", "1.0.0", "Known", &[], "test")],
            Arc::new(EchoHandler),
        );
        // handler 不会被调到, 因为 server 先校验 skill 是否存在
        let err = server.call("nope", &json!({})).unwrap_err();
        assert_eq!(err.code, TOOL_NOT_FOUND);
    }

    #[test]
    fn debug_impl_works() {
        let server = SkillToolServer::new(vec![make_desc("x", "1.0.0", "x", &[], "local")]);
        let s = format!("{:?}", server);
        assert!(s.contains("SkillToolServer"));
        assert!(s.contains("count"));
    }

    #[test]
    fn descriptor_with_tags_includes_them_in_description() {
        let mut tags = BTreeSet::new();
        tags.insert("summarize".to_string());
        tags.insert("text".to_string());
        let desc = SkillDescriptor::new("s", "1.0.0", "Summarize", tags, "vcptoolbox")
            .with_examples("{}", "{}");
        let tool = descriptor_to_tool(&desc);
        let desc_text = tool.description.unwrap();
        assert!(desc_text.contains("summarize"));
        assert!(desc_text.contains("text"));
    }
}
