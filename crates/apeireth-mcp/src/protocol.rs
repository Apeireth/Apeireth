//! **apeireth-mcp / JSON-RPC 2.0 基础类型**
//!
//! **依据**: docs/v2-strategy/05 §Step 2 (P0 战区 5 MCP skeleton)
//!
//! **设计**:
//! - `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` — JSON-RPC 2.0 最小子集
//!   (字段级参考 <https://www.jsonrpc.org/specification> §4 Request object / §5 Response object)
//! - `Id` — 请求 id (Number | String | Null), 不强制, 由 caller 决定
//! - 不假装: 完整双向支持 (request/response/notification/error), 没有的字段 (params 默认 None)
//!
//! **不修改承诺**:
//! - ✅ 不引入 unsafe (`#![deny(unsafe_code)]`)
//! - ✅ 不假装"完整 MCP 规范", 只做 hello-example 必需的最小集
//!   (initialize / tools/list / tools/call)
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// **JSON-RPC 2.0 版本字段 (固定字符串 `"2.0"`)**
pub const JSON_RPC_VERSION: &str = "2.0";

/// **JSON-RPC 2.0 请求 id**
///
/// 字段级参考 <https://www.jsonrpc.org/specification> §4 Request object:
/// `id` MUST be a String, Number, or NULL. 用 enum 而非 String 防止误用。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Id {
    /// 字符串 id (e.g. UUID)
    Str(String),
    /// 数字 id (e.g. 客户端递增计数器)
    Num(i64),
    /// null id (用于 fire-and-forget notification, 但 JSON-RPC 2.0 规定 notification 的 id 必须省略, 这里保留以备扩展)
    Null,
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}
impl From<i64> for Id {
    fn from(n: i64) -> Self {
        Self::Num(n)
    }
}

/// **JSON-RPC 2.0 请求对象**
///
/// 字段级参考 <https://www.jsonrpc.org/specification> §4:
/// ```json
/// {"jsonrpc": "2.0", "method": "subtract", "params": [42, 23], "id": 1}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// 固定 `"2.0"`
    pub jsonrpc: String,
    /// 方法名 (e.g. `"initialize"`, `"tools/list"`, `"tools/call"`)
    pub method: String,
    /// 参数 (可为 null / 对象 / 数组; MCP 全部用对象 params)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// 请求 id (notification 时为 None, JSON-RPC 2.0 §4.1 不允许 id)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

impl JsonRpcRequest {
    /// 新建一个普通请求 (带 id)
    pub fn new(method: impl Into<String>, params: Option<Value>, id: Id) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: Some(id),
        }
    }

    /// 新建一个 notification (id = None)
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: None,
        }
    }
}

/// **JSON-RPC 2.0 错误对象**
///
/// 字段级参考 <https://www.jsonrpc.org/specification> §5.1 Error object:
/// ```json
/// {"code": -32601, "message": "Method not found"}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JsonRpcError {
    /// 错误码 (e.g. -32600 invalid request / -32601 method not found / -32602 invalid params / -32603 internal error)
    pub code: i32,
    /// 人类可读错误描述
    pub message: String,
    /// 附加错误数据 (可为 null)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// 预定义错误码: Parse error (无效 JSON)
    pub const CODE_PARSE_ERROR: i32 = -32700;
    /// 预定义错误码: Invalid Request (无效 JSON-RPC)
    pub const CODE_INVALID_REQUEST: i32 = -32600;
    /// 预定义错误码: Method not found
    pub const CODE_METHOD_NOT_FOUND: i32 = -32601;
    /// 预定义错误码: Invalid params
    pub const CODE_INVALID_PARAMS: i32 = -32602;
    /// 预定义错误码: Internal error
    pub const CODE_INTERNAL_ERROR: i32 = -32603;

    /// 构造一个错误对象
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// 附带 data 字段
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

/// **JSON-RPC 2.0 响应对象**
///
/// 字段级参考 <https://www.jsonrpc.org/specification> §5 Response object:
/// ```json
/// {"jsonrpc": "2.0", "result": 19, "id": 1}
/// // 或
/// {"jsonrpc": "2.0", "error": {"code": -32601, "message": "..."}, "id": 1}
/// ```
///
/// **不变量**: `result` 和 `error` 互斥, 至少有一个存在
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// 固定 `"2.0"`
    pub jsonrpc: String,
    /// 成功时的结果 (与 error 互斥)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// 失败时的错误 (与 result 互斥)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// 对应请求的 id (notification 不应出现 response)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

impl JsonRpcResponse {
    /// 构造成功响应
    pub fn ok(id: Option<Id>, result: Value) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// 构造错误响应
    pub fn err(id: Option<Id>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }

    /// 提取 result; 若无 result 返回内部错误
    pub fn into_result(self) -> Result<Value, JsonRpcError> {
        if let Some(e) = self.error {
            Err(e)
        } else {
            self.result.ok_or_else(|| {
                JsonRpcError::new(
                    JsonRpcError::CODE_INTERNAL_ERROR,
                    "response has neither result nor error",
                )
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn request_roundtrip() {
        let req = JsonRpcRequest::new("initialize", Some(json!({"x": 1})), Id::Num(1));
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("\"method\":\"initialize\""));
        let back: JsonRpcRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.method, "initialize");
        assert_eq!(back.id, Some(Id::Num(1)));
    }

    #[test]
    fn notification_omits_id() {
        let n = JsonRpcRequest::notification("notifications/initialized", None);
        let s = serde_json::to_string(&n).unwrap();
        // id 字段不存在
        assert!(!s.contains("\"id\""));
    }

    #[test]
    fn response_ok() {
        let r = JsonRpcResponse::ok(Some(Id::Num(1)), json!({"serverInfo": "x"}));
        let s = serde_json::to_string(&r).unwrap();
        let back: JsonRpcResponse = serde_json::from_str(&s).unwrap();
        assert!(back.error.is_none());
        assert_eq!(back.result, Some(json!({"serverInfo": "x"})));
    }

    #[test]
    fn response_err() {
        let r = JsonRpcResponse::err(
            Some(Id::Num(2)),
            JsonRpcError::new(JsonRpcError::CODE_METHOD_NOT_FOUND, "Method not found"),
        );
        let s = serde_json::to_string(&r).unwrap();
        let back: JsonRpcResponse = serde_json::from_str(&s).unwrap();
        assert!(back.result.is_none());
        assert_eq!(back.error.unwrap().code, -32601);
    }

    #[test]
    fn into_result_ok() {
        let r = JsonRpcResponse::ok(Some(Id::Num(1)), json!({"ok": true}));
        let v = r.into_result().unwrap();
        assert_eq!(v, json!({"ok": true}));
    }

    #[test]
    fn into_result_err() {
        let r = JsonRpcResponse::err(Some(Id::Num(1)), JsonRpcError::new(-32601, "no"));
        assert!(r.into_result().is_err());
    }

    #[test]
    fn id_untagged_serde() {
        let s_num = Id::Num(42);
        let s_str = Id::Str("abc".to_string());
        let s_null = Id::Null;
        assert_eq!(serde_json::to_value(&s_num).unwrap(), json!(42));
        assert_eq!(serde_json::to_value(&s_str).unwrap(), json!("abc"));
        assert_eq!(serde_json::to_value(&s_null).unwrap(), json!(null));
    }

    #[test]
    fn error_codes_match_spec() {
        assert_eq!(JsonRpcError::CODE_PARSE_ERROR, -32700);
        assert_eq!(JsonRpcError::CODE_INVALID_REQUEST, -32600);
        assert_eq!(JsonRpcError::CODE_METHOD_NOT_FOUND, -32601);
        assert_eq!(JsonRpcError::CODE_INVALID_PARAMS, -32602);
        assert_eq!(JsonRpcError::CODE_INTERNAL_ERROR, -32603);
    }
}
