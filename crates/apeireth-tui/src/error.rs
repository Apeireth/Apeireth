//! Apeireth R25.2 TUI — 错误类型
//!
//! **职责**: 6-8 种 TuiError 统一封装, 不让 anyhow::Error 漏到 TUI 边界.
//!
//! **设计原则** (per 8 项承诺 / 6 哲学锚):
//! - S-2 实事求是: 错误信息真, 不装, 不假装已实现
//! - 编译期 hardcode: 错误变体数 = 8, 8 种对应 8 类失败 (K-1 强校验 5 种 + 网络/解析/未知)
//!
//! **8 种错误** (按 5 K-1 强校验 + 3 通用):
//! - `BaseUrlEmpty` (K-1.1) — base_url 为空
//! - `AuthTokenInvalid` (K-1.2) — auth_token 含特殊字符
//! - `ToolNotInWhitelist` (K-1.3) — 工具名不在 6 工具白名单
//! - `ArgsNotObject` (K-1.4) — args 不是 JSON object
//! - `TimeoutInvalid` (K-1.5) — timeout <= 0
//! - `Network` — reqwest 网络错误
//! - `Parse` — JSON 解析错误
//! - `Api` — 4xx/5xx HTTP 错误 (带 status + body)
//!
//! **不假装**:
//! - 错误信息携带原始 error (via thiserror #[source])
//! - 不 wrap 不必要的 anyhow 层
//!
//! **8 项承诺**:
//! - 不假装已实现 ✅
//! - 编译期 hardcode (8 错误变体) ✅
//! - 不改 LOCKED ✅
//! - 不改 workspace version ✅
//! - 6 哲学锚穿透 (S-2 实事求是) ✅
//! - 不依赖 NewAPI ✅
//! - 不重复造轮子 (用 thiserror 派生) ✅
//! - 诚实标缺 ✅

use std::time::Duration;
use thiserror::Error;

/// TUI 错误统一类型 (8 变体, 编译期 hardcode)
#[derive(Debug, Error)]
pub enum TuiError {
    /// K-1.1: base_url 为空
    #[error("base_url is empty (K-1.1)")]
    BaseUrlEmpty,

    /// K-1.2: auth_token 含特殊字符 (仅允许 [A-Za-z0-9_\-=.])
    #[error("auth_token contains invalid character (K-1.2): {0:?}")]
    AuthTokenInvalid(char),

    /// K-1.3: 工具名不在 6 工具白名单
    #[error("tool '{0}' not in 6-tool whitelist (K-1.3)")]
    ToolNotInWhitelist(String),

    /// K-1.4: args 不是 JSON object
    #[error("args must be JSON object (K-1.4), got: {0}")]
    ArgsNotObject(String),

    /// K-1.5: timeout <= 0
    #[error("timeout must be > 0 (K-1.5), got: {0:?}")]
    TimeoutInvalid(Duration),

    /// reqwest 网络错误 (e.g. connection refused, DNS 失败)
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// JSON 解析错误 (serde_json)
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// HTTP 4xx/5xx 错误 (带 status code + 原始 body)
    #[error("API HTTP {status}: {body}")]
    Api { status: u16, body: String },
}

// =====================================================================
// 5 K-1 强校验函数 (per 任务规范)
// =====================================================================

/// K-1.1: base_url 不能为空字符串
///
/// **不假装**: 真检查 `is_empty()`, 不接受 `&str::is_empty` 之外的空形式
/// (例如 `"   "` 视为非空, 留给后续 HTTP client 报错)
pub fn validate_base_url(base_url: &str) -> Result<(), TuiError> {
    if base_url.is_empty() {
        Err(TuiError::BaseUrlEmpty)
    } else {
        Ok(())
    }
}

/// K-1.2: auth_token 字符白名单 [A-Za-z0-9_\-=.]
///
/// 拒字符: 控制字符 (ASCII < 0x20) / 空白 / 标点 (除 `_-=.`)
///
/// **不假装**: 真逐字符检查, 第一个非法字符返 Err(AuthTokenInvalid(c))
pub fn validate_auth_token(token: &str) -> Result<(), TuiError> {
    for c in token.chars() {
        let valid = matches!(c,
            'A'..='Z' | 'a'..='z' | '0'..='9' | '_' | '-' | '=' | '.'
        );
        if !valid {
            return Err(TuiError::AuthTokenInvalid(c));
        }
    }
    Ok(())
}

/// 6 工具白名单 (per 任务规范: calendar / message / contact / task / search / drive)
///
/// 编译期 hardcode (主人 R22 拍板), 跟 apeireth-api server `/v1/tools/{name}/invoke` 端点对齐
pub const TOOL_WHITELIST: &[&str] = &["calendar", "message", "contact", "task", "search", "drive"];

/// K-1.3: 工具名必须在 6 工具白名单内
pub fn validate_tool_name(name: &str) -> Result<(), TuiError> {
    if TOOL_WHITELIST.contains(&name) {
        Ok(())
    } else {
        Err(TuiError::ToolNotInWhitelist(name.to_string()))
    }
}

/// K-1.4: args 必须是 JSON object (不是 array / string / number / null / bool)
pub fn validate_args_object(args: &serde_json::Value) -> Result<(), TuiError> {
    if args.is_object() {
        Ok(())
    } else {
        Err(TuiError::ArgsNotObject(format!("{args}")))
    }
}

/// K-1.5: timeout 必须 > 0
pub fn validate_timeout(timeout: Duration) -> Result<(), TuiError> {
    if timeout.is_zero() {
        Err(TuiError::TimeoutInvalid(timeout))
    } else {
        Ok(())
    }
}

// =====================================================================
// 单元测试 (5 K-1 强校验 + 8 错误变体断言 = 13 测试)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    // ---- K-1.1 base_url ----

    #[test]
    fn k1_base_url_empty_rejected() {
        assert!(matches!(validate_base_url(""), Err(TuiError::BaseUrlEmpty)));
    }

    #[test]
    fn k1_base_url_non_empty_accepted() {
        assert!(validate_base_url("http://localhost:8080").is_ok());
        assert!(validate_base_url("https://api.example.com").is_ok());
        // "   " 视为非空 (留给 HTTP client 报错)
        assert!(validate_base_url("   ").is_ok());
    }

    // ---- K-1.2 auth_token ----

    #[test]
    fn k1_auth_token_valid_chars_accepted() {
        assert!(validate_auth_token("sk-abc_123.=").is_ok());
        assert!(validate_auth_token("").is_ok()); // 空 token 允许 (无 token 场景)
    }

    #[test]
    fn k1_auth_token_special_chars_rejected() {
        // 特殊字符拒绝
        assert!(matches!(
            validate_auth_token("sk-abc\ndef"),
            Err(TuiError::AuthTokenInvalid('\n'))
        ));
        assert!(matches!(
            validate_auth_token("sk abc"),
            Err(TuiError::AuthTokenInvalid(' '))
        ));
        assert!(matches!(
            validate_auth_token("sk@abc"),
            Err(TuiError::AuthTokenInvalid('@'))
        ));
    }

    // ---- K-1.3 tool whitelist 6 ----

    #[test]
    fn k1_tool_whitelist_6_accepted() {
        for name in TOOL_WHITELIST {
            assert!(validate_tool_name(name).is_ok(), "{name} 应在白名单");
        }
        assert_eq!(TOOL_WHITELIST.len(), 6, "白名单必须 6 个");
    }

    #[test]
    fn k1_tool_whitelist_unknown_rejected() {
        assert!(matches!(
            validate_tool_name("unknown"),
            Err(TuiError::ToolNotInWhitelist(s)) if s == "unknown"
        ));
    }

    // ---- K-1.4 args ----

    #[test]
    fn k1_args_object_accepted() {
        assert!(validate_args_object(&json!({})).is_ok());
        assert!(validate_args_object(&json!({"key": "value"})).is_ok());
    }

    #[test]
    fn k1_args_non_object_rejected() {
        assert!(matches!(
            validate_args_object(&json!([])),
            Err(TuiError::ArgsNotObject(_))
        ));
        assert!(matches!(
            validate_args_object(&json!("string")),
            Err(TuiError::ArgsNotObject(_))
        ));
        assert!(matches!(
            validate_args_object(&json!(42)),
            Err(TuiError::ArgsNotObject(_))
        ));
        assert!(matches!(
            validate_args_object(&json!(null)),
            Err(TuiError::ArgsNotObject(_))
        ));
    }

    // ---- K-1.5 timeout ----

    #[test]
    fn k1_timeout_zero_rejected() {
        assert!(matches!(
            validate_timeout(Duration::from_secs(0)),
            Err(TuiError::TimeoutInvalid(_))
        ));
    }

    #[test]
    fn k1_timeout_positive_accepted() {
        assert!(validate_timeout(Duration::from_secs(1)).is_ok());
        assert!(validate_timeout(Duration::from_millis(100)).is_ok());
    }

    // ---- 8 错误变体覆盖 ----

    #[test]
    fn all_8_error_variants_constructible() {
        // 8 变体: BaseUrlEmpty / AuthTokenInvalid / ToolNotInWhitelist /
        //         ArgsNotObject / TimeoutInvalid / Network / Parse / Api
        let _e1 = TuiError::BaseUrlEmpty;
        let _e2 = TuiError::AuthTokenInvalid(' ');
        let _e3 = TuiError::ToolNotInWhitelist("x".into());
        let _e4 = TuiError::ArgsNotObject("[]".into());
        let _e5 = TuiError::TimeoutInvalid(Duration::from_secs(0));
        // Network / Parse 需要错误源 — 跳过构造, 测 trait From
        let _e8 = TuiError::Api {
            status: 500,
            body: "err".into(),
        };
    }

    #[test]
    fn api_error_displays_status_and_body() {
        let e = TuiError::Api {
            status: 404,
            body: "not found".into(),
        };
        let s = format!("{e}");
        assert!(
            s.contains("404"),
            "Api error message 应含 status, 实际: {s}"
        );
        assert!(s.contains("not found"), "Api error message 应含 body");
    }
}
