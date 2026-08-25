//! apeireth-tool-runtime - Tool execution runtime (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 ToolExecutor trait + 真 parse + 真 execute

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedToolCall {
    /// Tool name (v1: tool_name, v2: name — both supported via name field, tool_name is alias).
    #[serde(alias = "tool_name")]
    pub name: String,
    pub args: Value,
    /// Raw LLM marker string (v1 compat).
    #[serde(default)]
    pub raw_marker: String,
    /// Archery field (VCP `archery: true / no_reply`) (v1 compat).
    #[serde(default)]
    pub archery: bool,
    /// archery: no_reply (v1 compat).
    #[serde(default)]
    pub archery_no_reply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionResult {
    Ok(Value),
    Err { code: i32, message: String },
}

pub trait ToolExecutor: Send + Sync {
    fn parse(&self, raw: &str) -> Result<ParsedToolCall, String>;
    fn execute(&self, call: &ParsedToolCall) -> ExecutionResult;
}

pub struct SimpleExecutor { pub tools: HashMap<String, Box<dyn Fn(Value) -> Result<Value, String> + Send + Sync>> }

impl SimpleExecutor {
    pub fn new() -> Self { Self { tools: HashMap::new() } }
    pub fn register<F: Fn(Value) -> Result<Value, String> + Send + Sync + 'static>(&mut self, name: &str, f: F) {
        self.tools.insert(name.to_string(), Box::new(f));
    }
}

impl ToolExecutor for SimpleExecutor {
    fn parse(&self, raw: &str) -> Result<ParsedToolCall, String> {
        let v: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        let name = v.get("name").and_then(|n| n.as_str()).ok_or_else(|| "no name".to_string())?.to_string();
        let args = v.get("arguments").cloned().unwrap_or(Value::Null);
        Ok(ParsedToolCall { name, args, raw_marker: String::new(), archery: false, archery_no_reply: false })
    }
    fn execute(&self, call: &ParsedToolCall) -> ExecutionResult {
        match self.tools.get(&call.name) {
            Some(f) => match f(call.args.clone()) {
                Ok(v) => ExecutionResult::Ok(v),
                Err(e) => ExecutionResult::Err { code: -32000, message: e },
            },
            None => ExecutionResult::Err { code: -32601, message: format!("tool not found: {}", call.name) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_ok() {
        let e = SimpleExecutor::new();
        let c = e.parse(r#"{"name": "x", "arguments": {"a": 1}}"#).unwrap();
        assert_eq!(c.name, "x");
        assert_eq!(c.args["a"], 1);
    }
    #[test]
    fn test_parse_bad_json() {
        let e = SimpleExecutor::new();
        assert!(e.parse("not json").is_err());
    }
    #[test]
    fn test_execute_ok() {
        let mut e = SimpleExecutor::new();
        e.register("add", |args| Ok(serde_json::json!(args["a"].as_i64().unwrap() + args["b"].as_i64().unwrap())));
        let r = e.execute(&ParsedToolCall { name: "add".into(), args: serde_json::json!({"a": 1, "b": 2}), raw_marker: String::new(), archery: false, archery_no_reply: false });
        assert!(matches!(r, ExecutionResult::Ok(_)));
    }
    #[test]
    fn test_execute_unknown_tool() {
        let e = SimpleExecutor::new();
        let r = e.execute(&ParsedToolCall { name: "x".into(), args: serde_json::json!({}), raw_marker: String::new(), archery: false, archery_no_reply: false });
        assert!(matches!(r, ExecutionResult::Err { .. }));
    }
}
