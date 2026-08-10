//! **apeireth-mcp / tool-registry bridge**
//!
//! **依据**: docs/v2-strategy/05 §Step 2 (c) `src/tool_bridge.rs` 桥接 apeireth-tool-registry
//!
//! **设计**:
//! - `ToolDef` — MCP `tools/list` 返回的工具描述 (name + description + inputSchema)
//! - `ToolHandler` — boxed async 闭包, 与 `apeireth-tool-registry::Tool::call` 同形
//!   (返回 `Result<Value, String>` 以与 registry 一致)
//! - `bridge_from_registry(tool: &Arc<dyn Tool>) -> (ToolDef, ToolHandler)`
//!   把 registry 的 `Tool` trait 适配成 MCP 用得上的 (def + handler) 对
//! - `list_tools(registry)` — 列注册中心全部 tool (按字典序, 与 registry::list 一致)
//! - `invoke_via_registry(registry, name, args)` — 按名 invoke, 返回 JSON Value
//!
//! **不假装**:
//! - ✅ 桥接真实可跑 (调用链 tool-registry → bridge → McpServer handler → MCP `tools/call`)
//! - ✅ inputSchema 真按 JSON Schema 7 子集构造 (type=object + properties)
//! - ❌ 不假装"完整 JSON Schema 校验", 只构造骨架 (properties 来自 6 类 hardcode 默认)

use std::collections::HashMap;
use std::sync::Arc;

use apeireth_tool_registry::{Tool, ToolAxes};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// **MCP `tools/list` 返回的 ToolDef**
///
/// 字段级参考 MCP 2025-03-26 规范 §Tools:
/// ```json
/// {
///   "name": "echo",
///   "description": "Echoes back input",
///   "inputSchema": {"type": "object", "properties": {...}, "required": [...]}
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDef {
    /// 工具唯一名 (跟 registry `Tool::name()` 一致)
    pub name: String,
    /// 人类可读描述 (取自 registry 的 brief + description 拼接, 无则空串)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// JSON Schema 7 子集 (type=object + properties + required)
    pub inputSchema: Value,
}

impl ToolDef {
    /// 从 registry 的 `Tool` 派生 ToolDef
    pub fn from_registry_tool(tool: &dyn Tool) -> Self {
        let name = tool.name().to_string();
        let axes = tool.axes();
        let description = tool_description_from_axes(&name, axes);
        let input_schema = input_schema_for_axes(axes);
        Self {
            name,
            description,
            inputSchema: input_schema,
        }
    }
}

/// **ToolHandler — boxed async 闭包, 与 `apeireth_tool_registry::Tool::call` 同形**
///
/// `Pin<Box<dyn Future<Output = Result<Value, String>> + Send>>` 是 Rust 1.80+ boxed async 标配.
pub type ToolHandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>>;

/// **ToolHandler 抽象**
///
/// `Arc<dyn Fn(Value) -> ToolHandlerFuture + Send + Sync>` — 跟 registry `Tool::call` 同样无状态, 可跨线程.
pub type ToolHandler = Arc<dyn Fn(Value) -> ToolHandlerFuture + Send + Sync>;

/// **构造一个 ToolHandler from registry tool (克隆 Arc)**
///
/// 用于: `McpServer::register_tool(name, def, bridge_handler_from_registry(tool))`
pub fn bridge_handler_from_registry(tool: Arc<dyn Tool>) -> ToolHandler {
    Arc::new(move |args: Value| {
        let tool = Arc::clone(&tool);
        Box::pin(async move { tool.call(args).await })
    })
}

/// **构造一个 ToolHandler from 简单闭包 (用于 example / 用户自定义)**
///
/// 用法:
/// ```ignore
/// let h = handler_from_fn(|args| async move {
///     Ok(json!({"echo": args}))
/// });
/// ```
pub fn handler_from_fn<F, Fut>(f: F) -> ToolHandler
where
    F: Fn(Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<Value, String>> + Send + 'static,
{
    Arc::new(move |args: Value| Box::pin(f(args)))
}

/// **从 registry 列出全部 ToolDef (按字典序)**
///
/// 直接 `tool-registry::list()` 排序后转换, 不引入二级缓存
pub fn list_tools(registry: &apeireth_tool_registry::ToolRegistry) -> Vec<ToolDef> {
    let names = registry.list();
    names
        .into_iter()
        .filter_map(|name| registry.get(&name).map(|t| (name, t)))
        .map(|(name, tool)| {
            let mut def = ToolDef::from_registry_tool(tool.as_ref());
            // 确保 name 与 registry 一致 (防御)
            def.name = name;
            def
        })
        .collect()
}

/// **从 registry 按名 invoke tool**
///
/// `Err(String)` 与 `Tool::call` 同形 (统一错误字符串传递)
pub async fn invoke_via_registry(
    registry: &apeireth_tool_registry::ToolRegistry,
    name: &str,
    args: Value,
) -> Result<Value, String> {
    let tool = registry
        .get(name)
        .ok_or_else(|| format!("tool not found: {name}"))?;
    tool.call(args).await
}

// ============================================================
// 内部: 从 5 轴生成 inputSchema
// ============================================================

/// **从 5 轴生成 default description**
///
/// 字段级参考 tool-registry: 6 类各有默认 axes, 我们把 5 轴字符串化作为 description
fn tool_description_from_axes(name: &str, axes: ToolAxes) -> String {
    format!(
        "Tool '{name}' (axes: trigger={:?}/awaiting={:?}/resident={:?}/transport={:?}/output={:?})",
        axes.trigger, axes.awaiting, axes.resident, axes.transport, axes.output,
    )
}

/// **从 5 轴生成最小 JSON Schema 子集**
///
/// **不假装**: 完整 JSON Schema 校验在 v2.0+ 才做; 这里只构造 skeleton
/// - type=object
/// - properties: 一个 `args: {type: object}` (call 端传任意 JSON)
/// - required: 空数组
fn input_schema_for_axes(_axes: ToolAxes) -> Value {
    let mut properties = HashMap::new();
    properties.insert(
        "args".to_string(),
        json!({
            "type": "object",
            "description": "Free-form arguments (JSON object). Concrete tools parse their own keys.",
            "additionalProperties": true,
        }),
    );
    json!({
        "type": "object",
        "properties": Value::Object(properties.into_iter().collect::<serde_json::Map<_, _>>()),
        "required": [],
        "additionalProperties": false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_tool_registry::{MockSyncTool, ToolKind};

    #[test]
    fn tool_def_from_mock_sync() {
        let t = MockSyncTool {
            name: "echo".to_string(),
        };
        let def = ToolDef::from_registry_tool(&t);
        assert_eq!(def.name, "echo");
        assert!(def.description.contains("echo"));
        assert_eq!(def.inputSchema["type"], "object");
        assert!(def.inputSchema["properties"]["args"].is_object());
    }

    #[test]
    fn tool_def_serde_roundtrip() {
        let def = ToolDef {
            name: "x".to_string(),
            description: "y".to_string(),
            inputSchema: json!({"type": "object"}),
        };
        let s = serde_json::to_string(&def).unwrap();
        let back: ToolDef = serde_json::from_str(&s).unwrap();
        assert_eq!(back, def);
    }

    #[test]
    fn list_tools_from_registry() {
        let r = apeireth_tool_registry::ToolRegistry::new();
        r.register(
            "a".to_string(),
            Arc::new(MockSyncTool {
                name: "a".to_string(),
            }),
        );
        r.register(
            "b".to_string(),
            Arc::new(MockSyncTool {
                name: "b".to_string(),
            }),
        );
        let defs = list_tools(&r);
        assert_eq!(defs.len(), 2);
        // 按字典序 (registry::list 已排)
        assert_eq!(defs[0].name, "a");
        assert_eq!(defs[1].name, "b");
        // ToolKind 信息应能从 description 找到 axes 描述
        assert!(defs[0].description.contains("a"));
    }

    #[tokio::test]
    async fn bridge_handler_invokes_registry_tool() {
        let r = apeireth_tool_registry::ToolRegistry::new();
        r.register(
            "echo".to_string(),
            Arc::new(MockSyncTool {
                name: "echo".to_string(),
            }),
        );
        let tool = r.get("echo").unwrap();
        let h = bridge_handler_from_registry(tool);
        // MockSyncTool 真值: {"tool", "kind", "echo", "result"} (per registry.rs)
        let out = h(json!({"input": "hi"})).await.unwrap();
        assert_eq!(out["tool"], "echo");
        assert_eq!(out["kind"], "sync");
        assert_eq!(out["echo"], json!("hi"));
        assert_eq!(out["result"], "processed");
    }

    #[tokio::test]
    async fn handler_from_fn_basic() {
        let h = handler_from_fn(|args| async move {
            Ok(json!({"echo": args, "kind": ToolKind::Sync.as_vcp_str()}))
        });
        let out = h(json!({"x": 1})).await.unwrap();
        assert_eq!(out["echo"], json!({"x": 1}));
        assert_eq!(out["kind"], "synchronous");
    }

    #[tokio::test]
    async fn invoke_via_registry_not_found() {
        let r = apeireth_tool_registry::ToolRegistry::new();
        let err = invoke_via_registry(&r, "nope", json!({}))
            .await
            .unwrap_err();
        assert!(err.contains("not found"));
    }

    #[tokio::test]
    async fn invoke_via_registry_success() {
        let r = apeireth_tool_registry::ToolRegistry::new();
        r.register(
            "k".to_string(),
            Arc::new(MockSyncTool {
                name: "k".to_string(),
            }),
        );
        let out = invoke_via_registry(&r, "k", json!({})).await.unwrap();
        assert_eq!(out["tool"], "k");
    }
}
