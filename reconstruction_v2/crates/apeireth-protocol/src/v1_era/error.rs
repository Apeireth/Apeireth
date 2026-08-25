//! 协议层错误类型
//!
//! **借鉴 VCP 真代码** (`research/source/vcptoolbox/modules/chatCompletionHandler.js:286-323`):
//! `isToolResultError` 多级字段判断 (success / ok / status / code / httpStatus),
//! 我们把这种"5 字段多级判断"作为错误分类的依据。

use thiserror::Error;

/// 协议层错误。
///
/// **变体** (借鉴 VCP `isToolResultError` 真代码的 5 字段 + HTTP 错误码):
/// - `Parse` — JSON 解析失败 / 字段类型不匹配
/// - `Missing` — 必填字段缺失 (e.g. model / messages)
/// - `Invalid` — 字段值非法 (e.g. temperature < 0)
/// - `Unsupported` — 该协议不支持的特性 (e.g. Anthropic 不支持 audio content)
/// - `Inconsistent` — 字段之间矛盾 (e.g. tool_choice=required 但 tools 为空)
/// - `Remote` — 上游 HTTP / 协议错误 (status code + body)
/// - `Internal` — 内部 panic / invariant 违反
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// JSON 解析失败 / 字段类型不匹配
    #[error("parse error at {path}: {message}")]
    Parse {
        /// 字段路径 (e.g. `messages[2].content`)
        path: String,
        /// 错误详情
        message: String,
    },

    /// 必填字段缺失
    #[error("missing required field: {field}")]
    Missing {
        /// 缺失字段名
        field: String,
    },

    /// 字段值非法
    #[error("invalid value for {field}: {message}")]
    Invalid {
        /// 字段名
        field: String,
        /// 错误详情
        message: String,
    },

    /// 不支持
    #[error("unsupported feature: {feature}")]
    Unsupported {
        /// 不支持的特性名
        feature: String,
    },

    /// 字段间矛盾
    #[error("inconsistent state: {message}")]
    Inconsistent {
        /// 矛盾说明
        message: String,
    },

    /// 上游错误
    #[error("remote error (status={status}): {body}")]
    Remote {
        /// HTTP status code
        status: u16,
        /// 错误 body
        body: String,
    },

    /// 内部错误
    #[error("internal error: {0}")]
    Internal(String),
}

impl ProtocolError {
    /// Parse 错误
    pub fn parse(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Parse {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Missing 字段
    pub fn missing(field: impl Into<String>) -> Self {
        Self::Missing {
            field: field.into(),
        }
    }

    /// Invalid 字段值
    pub fn invalid(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Unsupported 特性
    pub fn unsupported(feature: impl Into<String>) -> Self {
        Self::Unsupported {
            feature: feature.into(),
        }
    }

    /// Inconsistent
    pub fn inconsistent(message: impl Into<String>) -> Self {
        Self::Inconsistent {
            message: message.into(),
        }
    }
}

/// 工具结果错误检测 (借鉴 VCP `isToolResultError` 真代码语义)。
///
/// VCP `chatCompletionHandler.js:286-323` 的多级判断:
/// 1. **成功优先** (`success===true` / `status==='success'` / `status==='ok'` / `ok===true` → 不算错误)
/// 2. **失败字段** (`error===true` / `success===false` / `status==='error'` / `status==='failed'` /
///    `status==='failure'` / `ok===false` → 算错误)
/// 3. **HTTP 错误码** (`code` / `statusCode` / `httpStatus` 在 400-599 范围 → 算错误)
/// 4. 字符串前缀: `[error]` / `[错误]` / `error:` / `失败：` 等
pub fn is_tool_result_error(result: &serde_json::Value) -> bool {
    if result.is_null() {
        return false;
    }

    if let Some(obj) = result.as_object() {
        // 1. 成功优先
        if obj.get("success").and_then(|v| v.as_bool()) == Some(true)
            || obj.get("status").and_then(|v| v.as_str()) == Some("success")
            || obj.get("status").and_then(|v| v.as_str()) == Some("ok")
            || obj.get("ok").and_then(|v| v.as_bool()) == Some(true)
        {
            return false;
        }

        // 2. 失败字段
        if obj.get("error").and_then(|v| v.as_bool()) == Some(true)
            || obj.get("success").and_then(|v| v.as_bool()) == Some(false)
            || obj.get("status").and_then(|v| v.as_str()) == Some("error")
            || obj.get("status").and_then(|v| v.as_str()) == Some("failed")
            || obj.get("status").and_then(|v| v.as_str()) == Some("failure")
            || obj.get("ok").and_then(|v| v.as_bool()) == Some(false)
        {
            return true;
        }

        // 3. HTTP 错误码
        let code_val = obj
            .get("code")
            .or_else(|| obj.get("statusCode"))
            .or_else(|| obj.get("httpStatus"));
        if let Some(v) = code_val {
            if let Some(n) = v.as_u64() {
                if (400..600).contains(&n) {
                    return true;
                }
            }
        }

        return false;
    }

    if let Some(s) = result.as_str() {
        let lower = s.to_lowercase();
        for prefix in ["[error]", "[错误]", "[失败]", "error:", "错误：", "失败："] {
            if lower.starts_with(prefix) {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn is_tool_result_error_null_is_not_error() {
        // VCP: 空结果不视为错误
        assert!(!is_tool_result_error(&json!(null)));
    }

    #[test]
    fn is_tool_result_error_success_fields() {
        // VCP: 成功优先 (success=true / status=ok / ok=true)
        assert!(!is_tool_result_error(
            &json!({"success": true, "data": "x"})
        ));
        assert!(!is_tool_result_error(&json!({"status": "success"})));
        assert!(!is_tool_result_error(&json!({"status": "ok"})));
        assert!(!is_tool_result_error(&json!({"ok": true})));
    }

    #[test]
    fn is_tool_result_error_failure_fields() {
        // VCP: 失败字段
        assert!(is_tool_result_error(&json!({"error": true})));
        assert!(is_tool_result_error(&json!({"success": false})));
        assert!(is_tool_result_error(&json!({"status": "error"})));
        assert!(is_tool_result_error(&json!({"status": "failed"})));
        assert!(is_tool_result_error(&json!({"status": "failure"})));
        assert!(is_tool_result_error(&json!({"ok": false})));
    }

    #[test]
    fn is_tool_result_error_http_code() {
        // VCP: code/statusCode/httpStatus 在 400-599
        assert!(is_tool_result_error(&json!({"code": 404})));
        assert!(is_tool_result_error(&json!({"code": 500})));
        assert!(is_tool_result_error(&json!({"statusCode": 429})));
        assert!(is_tool_result_error(&json!({"httpStatus": 401})));
        assert!(!is_tool_result_error(&json!({"code": 200})));
        assert!(!is_tool_result_error(&json!({"code": 399})));
    }

    #[test]
    fn is_tool_result_error_string_prefix() {
        // VCP: 字符串前缀
        assert!(is_tool_result_error(&json!("[error] something went wrong")));
        assert!(is_tool_result_error(&json!("[错误] 出错了")));
        assert!(is_tool_result_error(&json!("error: bad request")));
        assert!(is_tool_result_error(&json!("失败：网络超时")));
        assert!(!is_tool_result_error(&json!("Everything is fine")));
        // VCP: 业务正文里包含"错误"不算
        assert!(!is_tool_result_error(&json!(
            "the user typed an error message"
        )));
    }

    #[test]
    fn is_tool_result_error_ambiguous() {
        // 既没成功标志,也没失败字段,也没 HTTP code → 不算
        assert!(!is_tool_result_error(&json!({"foo": "bar"})));
        assert!(!is_tool_result_error(&json!({})));
    }
}
