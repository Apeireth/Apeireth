//! `apeireth-tools::result` — 工具统一返回类型
//!
//! **战役 2-5**: 5 trait 全部返 `Result<ToolResult, String>`,
//! 借鉴战役 2-2 `ExecutionResult` 的"成功/失败 + 详细字段"模式, 做成 typed enum.
//!
//! **VCP 字段级引用**:
//! - VCP `toolExecutor.js:475-482 _createErrorResult` 错误格式 → `ToolResult::Err { code, message }`
//! - VCP `isToolResultError` 多级判断 (success/ok/status/code/httpStatus 5 字段) → `code: i32` 字段
//!
//! **设计**:
//! - 成功路径: `Ok(Value)` (JSON Value, 因为工具返值多样)
//! - 失败路径: `Err { code, message }` (typed 错误码 + 人类可读 message)
//! - serde 兼容 (实战可序列化进 record store)

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// **工具调用统一返回类型**
///
/// **字段** (per VCP `isToolResultError` 真代码 + 战役 2-2 `ExecutionResult` 字段):
/// - `Err { code, message }` — 错误码 + 人类可读 message
/// - `Ok(Value)` — 工具返值, 任意 JSON (text / list / struct...)
///
/// **serde 顺序**: `Err` 在前, 因为 untagged 匹配是顺序的, Err 只匹配 `{"code": i32, "message": String}` 形态,
/// `Ok(Value)` 会匹配其他所有情况, 所以把 Err 放第一能避免 `Ok({"code": 500, ...})` 误判
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolResult {
    /// 失败, 带错误码 + message
    Err {
        /// 错误码 (HTTP status / business code / 0 = 未知)
        code: i32,
        /// 人类可读错误消息
        message: String,
    },
    /// 成功, 返 JSON Value
    Ok(Value),
}

impl ToolResult {
    /// 成功快捷构造 (从 JSON Value)
    pub fn ok(value: Value) -> Self {
        Self::Ok(value)
    }

    /// 成功快捷构造 (从字符串)
    pub fn ok_str(s: impl Into<String>) -> Self {
        Self::Ok(Value::String(s.into()))
    }

    /// 错误快捷构造
    pub fn err(code: i32, message: impl Into<String>) -> Self {
        Self::Err {
            code,
            message: message.into(),
        }
    }

    /// 是否成功
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    /// 是否失败
    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err { .. })
    }

    /// 取 Ok 的 value (失败返 None)
    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Ok(v) => Some(v),
            Self::Err { .. } => None,
        }
    }

    /// 取 Err 的 message (成功返 None)
    pub fn err_message(&self) -> Option<&str> {
        match self {
            Self::Ok(_) => None,
            Self::Err { message, .. } => Some(message.as_str()),
        }
    }

    /// 取 Err 的 code (成功返 None)
    pub fn err_code(&self) -> Option<i32> {
        match self {
            Self::Ok(_) => None,
            Self::Err { code, .. } => Some(*code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ok_constructors() {
        let r1 = ToolResult::ok(json!({"key": "value"}));
        assert!(r1.is_ok());
        assert!(!r1.is_err());
        assert_eq!(r1.value().unwrap()["key"], "value");

        let r2 = ToolResult::ok_str("hello world");
        assert!(r2.is_ok());
        assert_eq!(r2.value().unwrap().as_str().unwrap(), "hello world");
    }

    #[test]
    fn err_constructors_and_accessors() {
        let r = ToolResult::err(404, "Not found");
        assert!(r.is_err());
        assert!(!r.is_ok());
        assert_eq!(r.err_code(), Some(404));
        assert_eq!(r.err_message(), Some("Not found"));
        assert!(r.value().is_none());
    }

    #[test]
    fn serde_round_trip_ok() {
        let r = ToolResult::ok(json!({"count": 3, "items": ["a", "b"]}));
        let json = serde_json::to_string(&r).unwrap();
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
        assert!(back.is_ok());
        assert_eq!(back.value().unwrap()["count"], 3);
    }

    #[test]
    fn serde_round_trip_err() {
        let r = ToolResult::err(500, "Internal error");
        let json = serde_json::to_string(&r).unwrap();
        let back: ToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
        assert!(back.is_err());
        assert_eq!(back.err_code(), Some(500));
    }

    #[test]
    fn empty_ok_value() {
        // 空 JSON 对象也是 Ok
        let r = ToolResult::ok(json!({}));
        assert!(r.is_ok());
        assert_eq!(r.value().unwrap().as_object().unwrap().len(), 0);
    }
}
