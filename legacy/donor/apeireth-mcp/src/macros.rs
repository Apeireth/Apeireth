//! R125-4: MCP JSON-RPC macros (借鉴 modelcontextprotocol/servers dispatch pattern)
//!
//! **依据**: `R124-3-BORROW-modelcontextprotocol/servers-primitive-namespace-2026-08-10`
//!
//! **目的**: 减少 `McpServer::dispatch` 等地方 5+ 处重复的 `JsonRpcResponse::ok` /
//! `JsonRpcResponse::err` 构造, 用 macro 统一 envelope 模式.
//!
//! **不漂移 (主哲学锚 #1, 8 硬墙 #3)**:
//! - 0 改 `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` 公共 API
//! - 0 改 `dispatch` 行为, macro 仅是 syntactic sugar
//! - 0 引入新依赖, 用 `macro_rules!` 标准库即可

// ============================================================
// jsonrpc_envelope! macro
// ============================================================

/// **构造 JSON-RPC 2.0 envelope** (per spec §4 Request object + §5 Response object)
///
/// **用法**:
/// ```ignore
/// // 请求
/// let req = jsonrpc_envelope!(request, "tools/list", Some(json!({})), Id::Num(1));
/// // 响应 (ok)
/// let resp = jsonrpc_envelope!(ok, req.id.clone(), json!({"tools": []}));
/// // 响应 (err)
/// let err = jsonrpc_envelope!(err, req.id.clone(), -32601, "method not found");
/// ```
///
/// **优势 (借鉴 pattern)**:
/// - 1 处定义 envelope 字段 (`jsonrpc: "2.0"`), 5+ 处调用, 0 重复
/// - 编译期确保 `jsonrpc` 字段始终为 `"2.0"`
/// - 减少 `JsonRpcRequest::new` / `JsonRpcResponse::ok` / `JsonRpcResponse::err` 调用重复
#[macro_export]
macro_rules! jsonrpc_envelope {
    // ---- request (with id) ----
    (request, $method:expr, $params:expr, $id:expr) => {{
        use $crate::protocol::{JsonRpcRequest, JSON_RPC_VERSION};
        JsonRpcRequest {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: $method.into(),
            params: $params,
            id: Some($id),
        }
    }};

    // ---- notification (no id) ----
    (notification, $method:expr, $params:expr) => {{
        use $crate::protocol::{JsonRpcRequest, JSON_RPC_VERSION};
        JsonRpcRequest {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: $method.into(),
            params: $params,
            id: None,
        }
    }};

    // ---- response ok ----
    (ok, $id:expr, $result:expr) => {{
        use $crate::protocol::{JsonRpcResponse, JSON_RPC_VERSION};
        JsonRpcResponse {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            result: Some($result),
            error: None,
            id: $id,
        }
    }};

    // ---- response err ----
    (err, $id:expr, $code:expr, $message:expr) => {{
        use $crate::protocol::{JsonRpcError, JsonRpcResponse, JSON_RPC_VERSION};
        JsonRpcResponse {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            result: None,
            error: Some(JsonRpcError::new($code, $message)),
            id: $id,
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Id, JsonRpcError, JsonRpcRequest, JsonRpcResponse, JSON_RPC_VERSION};
    use serde_json::json;

    /// **test_jsonrpc_macro_generates_correct_envelope** — 验证 macro 生成的 envelope
    /// 字段正确 (`jsonrpc: "2.0"` + method/result/error/id)
    #[test]
    fn test_jsonrpc_macro_generates_correct_envelope() {
        // 1) request (with id)
        let req: JsonRpcRequest =
            jsonrpc_envelope!(request, "tools/list", Some(json!({})), Id::Num(1));
        assert_eq!(req.jsonrpc, JSON_RPC_VERSION);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.params, Some(json!({})));
        assert_eq!(req.id, Some(Id::Num(1)));

        // 2) notification (no id)
        let n: JsonRpcRequest = jsonrpc_envelope!(notification, "notifications/initialized", None);
        assert_eq!(n.jsonrpc, "2.0");
        assert_eq!(n.method, "notifications/initialized");
        assert_eq!(n.params, None);
        assert_eq!(n.id, None);

        // 3) response ok
        let resp: JsonRpcResponse = jsonrpc_envelope!(ok, Some(Id::Num(1)), json!({"tools": []}));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.error.is_none());
        assert_eq!(resp.result, Some(json!({"tools": []})));
        assert_eq!(resp.id, Some(Id::Num(1)));

        // 4) response err
        let err_resp: JsonRpcResponse = jsonrpc_envelope!(
            err,
            Some(Id::Num(2)),
            JsonRpcError::CODE_METHOD_NOT_FOUND,
            "Method not found"
        );
        assert_eq!(err_resp.jsonrpc, "2.0");
        assert!(err_resp.result.is_none());
        assert_eq!(
            err_resp.error.as_ref().unwrap().code,
            JsonRpcError::CODE_METHOD_NOT_FOUND
        );
        assert_eq!(err_resp.error.as_ref().unwrap().message, "Method not found");
        assert_eq!(err_resp.id, Some(Id::Num(2)));

        // 5) 序列化验证 (跟手写 `JsonRpcRequest::new` 等价)
        let req_serialized = serde_json::to_string(&req).unwrap();
        assert!(req_serialized.contains("\"jsonrpc\":\"2.0\""));
        assert!(req_serialized.contains("\"method\":\"tools/list\""));
        assert!(req_serialized.contains("\"id\":1"));
    }
}
