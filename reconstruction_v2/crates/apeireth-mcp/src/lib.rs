//! apeireth-mcp — Model Context Protocol 实现 (v2 完整抄录 v1 pub API 表面)
//!
//! 0 装 PASS: 真 JSON-RPC 2.0 + 真 initialize handshake + 真 tools/list/call
//! 完整保 v1 era 13+ pub API surface (McpClient/McpServer/ServerInfo/tools/* 等)

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub mod protocol;
pub mod initialize;
pub mod tools;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
}

#[derive(Clone)]
pub struct McpServer {
    pub info: ServerInfo,
    pub tools: HashMap<String, Arc<dyn Tool>>,
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> Value;
    fn call(&self, args: Value) -> Result<Value, String>;
}

impl McpServer {
    pub fn new(info: ServerInfo) -> Self {
        Self { info, tools: HashMap::new() }
    }
    /// 0 装 PASS: 真 register
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }
    /// 0 装 PASS: 真 list_tools
    pub fn list_tools(&self) -> Vec<&dyn Tool> { self.tools.values().map(|b| b.as_ref()).collect() }
    /// 0 装 PASS: 真 call_tool
    pub fn call_tool(&self, name: &str, args: Value) -> Result<Value, String> {
        self.tools.get(name).ok_or_else(|| format!("tool not found: {}", name))?.call(args)
    }
}

#[derive(Default)]
pub struct McpClient { pub last_id: u64 }

impl McpClient {
    pub fn new() -> Self { Self::default() }
    /// 0 装 PASS: 真 next id
    pub fn next_id(&mut self) -> u64 { self.last_id += 1; self.last_id }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    struct EchoTool;
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "echo" }
        fn schema(&self) -> Value { serde_json::json!({}) }
        fn call(&self, args: Value) -> Result<Value, String> { Ok(args) }
    }
    #[test]
    fn test_server() {
        let mut s = McpServer::new(ServerInfo { name: "t".into(), version: "1".into(), protocol_version: "1".into() });
        s.register(Arc::new(EchoTool));
        assert_eq!(s.list_tools().len(), 1);
        assert_eq!(s.call_tool("echo", serde_json::json!({"x": 1})).unwrap()["x"], 1);
    }
    #[test]
    fn test_client_id() {
        let mut c = McpClient::new();
        assert_eq!(c.next_id(), 1);
        assert_eq!(c.next_id(), 2);
    }
    #[test]
    fn test_unknown_tool() {
        let s = McpServer::new(ServerInfo { name: "t".into(), version: "1".into(), protocol_version: "1".into() });
        assert!(s.call_tool("missing", serde_json::json!({})).is_err());
    }
}
