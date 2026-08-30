//! JSON-RPC 2.0 envelopes used by MCP.
//!
//! Donor: `legacy/donor/apeireth-mcp/src/protocol.rs`.
//!
//! The production client in `apeireth-tools::mcp` uses a `u64` request id.
//! MCP (and JSON-RPC 2.0) allow String | Number | Null, and notifications
//! omit `id` entirely. This module recovers that wire shape as a library
//! primitive so a later host can speak real MCP without a second registry.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 version field (fixed `"2.0"`).
pub const JSON_RPC_VERSION: &str = "2.0";

/// JSON-RPC 2.0 request id.
///
/// Spec §4: `id` MUST be a String, Number, or NULL. Notifications omit the
/// field rather than sending `null`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Id {
    /// String id (e.g. UUID).
    Str(String),
    /// Numeric id (e.g. client incrementing counter).
    Num(i64),
    /// Explicit null id (rare; notifications should omit the field).
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

impl From<u64> for Id {
    fn from(n: u64) -> Self {
        Self::Num(n as i64)
    }
}

/// JSON-RPC 2.0 request object (spec §4).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest {
    /// Fixed `"2.0"`.
    pub jsonrpc: String,
    /// Method name (e.g. `"initialize"`, `"tools/list"`).
    pub method: String,
    /// Parameters (object for MCP; array is legal JSON-RPC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    /// Request id. `None` means a notification (spec §4.1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

impl JsonRpcRequest {
    /// Ordinary request with an id.
    pub fn new(method: impl Into<String>, params: Option<Value>, id: Id) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: Some(id),
        }
    }

    /// Notification: no `id` field on the wire.
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: None,
        }
    }

    /// True when this request is a notification (no response expected).
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC 2.0 error object (spec §5.1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JsonRpcError {
    /// Error code (`-32700` parse / `-32600` invalid request / …).
    pub code: i32,
    /// Human-readable description.
    pub message: String,
    /// Optional extra data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Parse error (invalid JSON).
    pub const CODE_PARSE_ERROR: i32 = -32700;
    /// Invalid Request (not a JSON-RPC object).
    pub const CODE_INVALID_REQUEST: i32 = -32600;
    /// Method not found.
    pub const CODE_METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params.
    pub const CODE_INVALID_PARAMS: i32 = -32602;
    /// Internal error.
    pub const CODE_INTERNAL_ERROR: i32 = -32603;

    /// Construct an error object.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Attach a `data` field.
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

/// JSON-RPC 2.0 response object (spec §5).
///
/// Invariant: `result` and `error` are mutually exclusive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcResponse {
    /// Fixed `"2.0"`.
    pub jsonrpc: String,
    /// Success payload (mutually exclusive with `error`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Failure payload (mutually exclusive with `result`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Matching request id. Notifications never produce a response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

impl JsonRpcResponse {
    /// Success response.
    pub fn ok(id: Option<Id>, result: Value) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Error response.
    pub fn err(id: Option<Id>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }

    /// Extract `result`, or the RPC error if present.
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

/// JSON-RPC 2.0 §6 batch: a single object or an array.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonRpcBatch<T> {
    /// Single request / response (backward compatible).
    Single(T),
    /// Array of requests / responses.
    Batch(Vec<T>),
}

impl<T: Serialize> Serialize for JsonRpcBatch<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            JsonRpcBatch::Single(t) => t.serialize(serializer),
            JsonRpcBatch::Batch(v) => v.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for JsonRpcBatch<T>
where
    T: serde::de::DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = serde_json::Value::deserialize(deserializer)?;
        if v.is_array() {
            let arr = serde_json::from_value::<Vec<T>>(v).map_err(serde::de::Error::custom)?;
            Ok(JsonRpcBatch::Batch(arr))
        } else {
            let single = serde_json::from_value::<T>(v).map_err(serde::de::Error::custom)?;
            Ok(JsonRpcBatch::Single(single))
        }
    }
}

impl<T> JsonRpcBatch<T> {
    /// Flatten to a `Vec`. A single item becomes length 1.
    pub fn into_vec(self) -> Vec<T> {
        match self {
            JsonRpcBatch::Single(t) => vec![t],
            JsonRpcBatch::Batch(v) => v,
        }
    }

    /// Length (single counts as 1).
    pub fn len(&self) -> usize {
        match self {
            JsonRpcBatch::Single(_) => 1,
            JsonRpcBatch::Batch(v) => v.len(),
        }
    }

    /// True only for an empty array batch.
    pub fn is_empty(&self) -> bool {
        match self {
            JsonRpcBatch::Single(_) => false,
            JsonRpcBatch::Batch(v) => v.is_empty(),
        }
    }

    /// True when this is an array form.
    pub fn is_batch(&self) -> bool {
        matches!(self, JsonRpcBatch::Batch(_))
    }

    /// Build from a vec. A one-element vec collapses to [`JsonRpcBatch::Single`].
    pub fn from_vec(v: Vec<T>) -> Self {
        if v.len() == 1 {
            JsonRpcBatch::Single(v.into_iter().next().expect("len == 1"))
        } else {
            JsonRpcBatch::Batch(v)
        }
    }
}

/// Wire heuristic: first non-whitespace character is `[`.
///
/// Spec §6: empty array is an Invalid Request (the caller still has to
/// distinguish that after parsing).
pub fn looks_like_batch(line: &str) -> bool {
    line.trim_start().starts_with('[')
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
        assert!(!s.contains("\"id\""));
        assert!(n.is_notification());
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
    fn into_result_neither_is_internal_error() {
        let r = JsonRpcResponse {
            jsonrpc: JSON_RPC_VERSION.to_string(),
            result: None,
            error: None,
            id: Some(Id::Num(1)),
        };
        let err = r.into_result().unwrap_err();
        assert_eq!(err.code, JsonRpcError::CODE_INTERNAL_ERROR);
    }

    #[test]
    fn id_untagged_serde() {
        let s_num = Id::Num(42);
        let s_str = Id::Str("abc".to_string());
        let s_null = Id::Null;
        assert_eq!(serde_json::to_value(&s_num).unwrap(), json!(42));
        assert_eq!(serde_json::to_value(&s_str).unwrap(), json!("abc"));
        assert_eq!(serde_json::to_value(&s_null).unwrap(), json!(null));
        let from_u64: Id = 7u64.into();
        assert_eq!(from_u64, Id::Num(7));
    }

    #[test]
    fn batch_request_serialize() {
        let req1 = JsonRpcRequest::new("tools/list", None, Id::Num(1));
        let req2 = JsonRpcRequest::notification("notifications/initialized", None);
        let batch = JsonRpcBatch::Batch(vec![req1, req2]);
        let s = serde_json::to_string(&batch).unwrap();
        assert!(s.starts_with('['));
        assert!(s.ends_with(']'));
        assert!(s.contains("\"tools/list\""));
    }

    #[test]
    fn batch_request_deserialize() {
        let json = r#"[{"jsonrpc":"2.0","method":"tools/list","id":1},{"jsonrpc":"2.0","method":"notifications/initialized"}]"#;
        let b: JsonRpcBatch<JsonRpcRequest> = serde_json::from_str(json).unwrap();
        assert!(b.is_batch());
        let v = b.into_vec();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].method, "tools/list");
        assert_eq!(v[1].method, "notifications/initialized");
        assert!(v[1].id.is_none());
    }

    #[test]
    fn batch_single_fallback() {
        let json = r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#;
        let b: JsonRpcBatch<JsonRpcRequest> = serde_json::from_str(json).unwrap();
        assert!(!b.is_batch());
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn batch_response_roundtrip() {
        let r1 = JsonRpcResponse::ok(Some(Id::Num(1)), json!({"tools": []}));
        let r2 = JsonRpcResponse::ok(Some(Id::Num(2)), json!({"x": 1}));
        let batch = JsonRpcBatch::Batch(vec![r1.clone(), r2.clone()]);
        let s = serde_json::to_string(&batch).unwrap();
        let back: JsonRpcBatch<JsonRpcResponse> = serde_json::from_str(&s).unwrap();
        let v = back.into_vec();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].result, r1.result);
        assert_eq!(v[1].result, r2.result);
    }

    #[test]
    fn batch_empty_is_invalid_batch() {
        let json = r#"[]"#;
        let b: JsonRpcBatch<JsonRpcRequest> = serde_json::from_str(json).unwrap();
        assert!(b.is_empty());
        assert!(b.is_batch());
    }

    #[test]
    fn looks_like_batch_heuristic() {
        assert!(looks_like_batch(r#"[{"jsonrpc":"2.0","method":"x"}]"#));
        assert!(looks_like_batch(r#"  [1,2,3]"#));
        assert!(!looks_like_batch(r#"{"jsonrpc":"2.0"}"#));
        assert!(!looks_like_batch(r#"  {"a":1}"#));
    }

    #[test]
    fn batch_from_vec_single_collapses() {
        let req = JsonRpcRequest::new("tools/list", None, Id::Num(1));
        let b = JsonRpcBatch::from_vec(vec![req]);
        assert!(!b.is_batch());
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn error_codes_match_spec() {
        assert_eq!(JsonRpcError::CODE_PARSE_ERROR, -32700);
        assert_eq!(JsonRpcError::CODE_INVALID_REQUEST, -32600);
        assert_eq!(JsonRpcError::CODE_METHOD_NOT_FOUND, -32601);
        assert_eq!(JsonRpcError::CODE_INVALID_PARAMS, -32602);
        assert_eq!(JsonRpcError::CODE_INTERNAL_ERROR, -32603);
    }

    #[test]
    fn string_id_roundtrip_on_request() {
        let req = JsonRpcRequest::new("tools/list", None, Id::Str("abc".into()));
        let s = serde_json::to_string(&req).unwrap();
        let back: JsonRpcRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(back.id, Some(Id::Str("abc".into())));
    }
}
