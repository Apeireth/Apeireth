//! R125-4: MCP tools protocol — server trait + handlers
//!
//! **拆分自原 `tools/mod.rs` (R65)**: ToolServer trait + handle_tools_list +
//! handle_tools_call 3 个核心 handler, 1:1 镜像 MCP spec §tools.
//!
//! **不漂移 (主哲学锚 #1, 8 硬墙 #3)**:
//! - 0 改 `ToolServer` trait 签名 (内部 fn 实施可改授权, 0 改入口签名)
//! - 0 改 `handle_tools_list` / `handle_tools_call` 签名
//! - 0 改 错误码常量 (TOOL_NOT_FOUND / TOOL_INVALID_ARGS / TOOL_CALL_FAILED)
//! - 0 改 JSON 响应结构 ({tools: [...]} / {content: [...], isError: bool})

use serde_json::{json, Value};

use crate::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

use super::types::{Tool, ToolCallResult, ToolContent, TOOL_INVALID_ARGS, TOOL_NOT_FOUND};

// ============================================================
// ToolServer trait
// ============================================================

/// **MCP ToolServer trait (server 端抽象, 跟 ResourceServer 对偶)**
///
/// 任意 server impl (e.g. `SkillToolServer` / `EvalToolServer` / `CouncilToolServer`)
/// 都能挂到 mcp handler.
pub trait ToolServer: Send + Sync {
    /// 列所有 tools
    fn list(&self) -> Vec<Tool>;
    /// 按 name + arguments 调 tool
    fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError>;
}

// ============================================================
// Handlers
// ============================================================

/// **处理 `tools/list` 请求 → JSON-RPC 响应**
pub fn handle_tools_list(req: &JsonRpcRequest, server: &dyn ToolServer) -> JsonRpcResponse {
    let tools = server.list();
    JsonRpcResponse::ok(req.id.clone(), json!({ "tools": tools }))
}

/// **处理 `tools/call` 请求 → JSON-RPC 响应**
pub fn handle_tools_call(req: &JsonRpcRequest, server: &dyn ToolServer) -> JsonRpcResponse {
    // params 必填含 name + arguments
    let Some(params) = req.params.as_ref() else {
        return JsonRpcResponse::err(
            req.id.clone(),
            JsonRpcError::new(TOOL_INVALID_ARGS, "params missing"),
        );
    };
    let name = match params.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => {
            return JsonRpcResponse::err(
                req.id.clone(),
                JsonRpcError::new(TOOL_INVALID_ARGS, "params.name missing or not string"),
            );
        }
    };
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
    match server.call(&name, &arguments) {
        Ok(result) => JsonRpcResponse::ok(
            req.id.clone(),
            json!({
                "content": result.content,
                "isError": result.is_error,
            }),
        ),
        Err(e) => {
            use super::types::TOOL_CALL_FAILED;
            let code = match e.code {
                TOOL_NOT_FOUND => TOOL_NOT_FOUND,
                TOOL_INVALID_ARGS => TOOL_INVALID_ARGS,
                _ => TOOL_CALL_FAILED,
            };
            JsonRpcResponse::err(req.id.clone(), JsonRpcError::new(code, e.message))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Id, JsonRpcRequest};
    use serde_json::json;

    /// 最小 in-memory ToolServer (测试用)
    struct EchoToolServer;

    impl ToolServer for EchoToolServer {
        fn list(&self) -> Vec<Tool> {
            vec![Tool::new("echo")
                .with_description("Echo back the input text")
                .with_input_schema(json!({
                    "type": "object",
                    "properties": { "text": { "type": "string" } },
                    "required": ["text"]
                }))]
        }
        fn call(&self, name: &str, arguments: &Value) -> Result<ToolCallResult, JsonRpcError> {
            if name != "echo" {
                return Err(JsonRpcError::new(
                    TOOL_NOT_FOUND,
                    format!("tool `{name}` not found"),
                ));
            }
            let text = arguments
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Ok(ToolCallResult::ok(vec![ToolContent::text(text)]))
        }
    }

    fn req_list() -> JsonRpcRequest {
        JsonRpcRequest::new("tools/list", None, Id::Num(1))
    }

    fn req_call(name: &str, args: Value) -> JsonRpcRequest {
        JsonRpcRequest::new(
            "tools/call",
            Some(json!({ "name": name, "arguments": args })),
            Id::Num(2),
        )
    }

    #[test]
    fn handle_tools_list_returns_tools() {
        let req = req_list();
        let resp = handle_tools_list(&req, &EchoToolServer);
        assert!(resp.error.is_none());
        let result = resp.result.expect("result present");
        let tools = result
            .get("tools")
            .and_then(|v| v.as_array())
            .expect("tools array");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].get("name").and_then(|v| v.as_str()), Some("echo"));
    }

    #[test]
    fn handle_tools_call_echo_returns_text() {
        let req = req_call("echo", json!({"text": "hello world"}));
        let resp = handle_tools_call(&req, &EchoToolServer);
        assert!(resp.error.is_none());
        let result = resp.result.expect("result present");
        let content = result
            .get("content")
            .and_then(|v| v.as_array())
            .expect("content array");
        assert_eq!(content.len(), 1);
        assert_eq!(
            content[0].get("type").and_then(|v| v.as_str()),
            Some("text")
        );
        assert_eq!(
            content[0].get("text").and_then(|v| v.as_str()),
            Some("hello world")
        );
    }

    #[test]
    fn handle_tools_call_unknown_returns_error() {
        let req = req_call("nonexistent", json!({}));
        let resp = handle_tools_call(&req, &EchoToolServer);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, TOOL_NOT_FOUND);
    }

    #[test]
    fn handle_tools_call_missing_params_returns_error() {
        let req = JsonRpcRequest::new("tools/call", None, Id::Num(99));
        let resp = handle_tools_call(&req, &EchoToolServer);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, TOOL_INVALID_ARGS);
    }

    #[test]
    fn handle_tools_call_missing_name_returns_error() {
        let req = JsonRpcRequest::new("tools/call", Some(json!({ "arguments": {} })), Id::Num(100));
        let resp = handle_tools_call(&req, &EchoToolServer);
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, TOOL_INVALID_ARGS);
    }
}
