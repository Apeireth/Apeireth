//! MCP tools 协议 (从 v1 tools.rs 抄录升级核心)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<ToolDef>,
}

/// 0 装 PASS: 真 list tools
pub fn handle_tools_list(server: &super::McpServer) -> ToolsListResult {
    let tools = server.list_tools().into_iter().map(|t| ToolDef {
        name: t.name().to_string(),
        description: t.description().to_string(),
        input_schema: t.schema(),
    }).collect();
    ToolsListResult { tools }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// 0 装 PASS: 真 call tool
pub fn handle_tools_call(server: &super::McpServer, params: ToolsCallParams) -> Result<Value, String> {
    server.call_tool(&params.name, params.arguments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::McpServer;
    use super::super::ServerInfo;
    use super::super::Tool;
    struct Adder;
    impl Tool for Adder {
        fn name(&self) -> &str { "add" }
        fn description(&self) -> &str { "adds" }
        fn schema(&self) -> Value { json!({}) }
        fn call(&self, args: Value) -> Result<Value, String> {
            let a = args["a"].as_i64().unwrap_or(0);
            let b = args["b"].as_i64().unwrap_or(0);
            Ok(json!(a + b))
        }
    }
    #[test]
    fn test_list() {
        let mut s = McpServer::new(ServerInfo { name: "s".into(), version: "1".into(), protocol_version: "1".into() });
        s.register(Arc::new(Adder));
        let r = handle_tools_list(&s);
        assert_eq!(r.tools.len(), 1);
    }
    #[test]
    fn test_call() {
        let mut s = McpServer::new(ServerInfo { name: "s".into(), version: "1".into(), protocol_version: "1".into() });
        s.register(Arc::new(Adder));
        let r = handle_tools_call(&s, ToolsCallParams { name: "add".into(), arguments: json!({"a": 1, "b": 2}) }).unwrap();
        assert_eq!(r, 3);
    }
    #[test]
    fn test_unknown_tool() {
        let s = McpServer::new(ServerInfo { name: "s".into(), version: "1".into(), protocol_version: "1".into() });
        assert!(handle_tools_call(&s, ToolsCallParams { name: "x".into(), arguments: json!({}) }).is_err());
    }
}
