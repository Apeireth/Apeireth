/// 基础设施 × HTTP 单元测试 (R25.2 partial, 1.0 release 估补)
///
/// **测试范围** (per 主人派活单 2026-08-05):
/// - 5 K-1 强校验: base_url / auth_token / tool_whitelist / args / timeout
/// - 5 测试函数 (主人要求)
///
/// **6 哲学锚穿透**:
/// - S-1 北极星: HTTP 客户端服务 ASI 北极星 (真接 → 平台不孤立)
/// - S-2 实事求是: 5 K-1 强校验在构造期过, 5xx 透传, 不装成功
/// - O-2 走在前人肩上: 借 reqwest + tokio 业界惯例
/// - O-3 干到底: 5 K-1 + 5 端点 + 5xx 全覆盖
/// - O-4 任何人都能接手: 错误信息带 status + body
/// - O-5 不假装: 5xx 返 TuiError::Api, 透传 (不假装成功)
///
/// **8 项承诺**: 全部遵守
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)
#[path = "../src/config_watcher.rs"] mod config_watcher;
#[path = "../src/app.rs"] mod app;
#[path = "../src/backend.rs"] mod backend;
#[path = "../src/http_llm.rs"] mod http_llm;
#[path = "../src/observability.rs"] mod observability;
#[path = "../src/pages/mod.rs"] mod pages;
#[path = "../src/organ/mod.rs"] mod organ;
#[path = "../src/command/mod.rs"] mod command;
#[path = "../src/persistence.rs"] mod persistence;
#[path = "../src/llm_config.rs"] mod llm_config;
#[path = "../src/onboarding.rs"] mod onboarding;
#[path = "../src/theme.rs"] mod theme;

#[path = "../src/error.rs"] mod error;
#[path = "../src/http.rs"] mod http;
#[path = "../src/nav/mod.rs"] mod nav;
// R31 fix: 12 mod 声明 (跟 src/main.rs 顶层 mod 同步, 让 test binary root 解析 crate::xxx)


use error::{
    TuiError, validate_args_object, validate_auth_token, validate_base_url, validate_timeout,
    validate_tool_name, TOOL_WHITELIST,
};
use http::ApeirethClient;
use serde_json::json;
use std::time::Duration;

// =====================================================================
// 1. K-1.1 base_url 校验
// =====================================================================

#[test]
fn k1_base_url_validate() {
    // 空拒绝
    assert!(matches!(
        validate_base_url(""),
        Err(TuiError::BaseUrlEmpty)
    ));
    // 非空接受 (含 whitespace)
    assert!(validate_base_url("http://localhost:8080").is_ok());
    assert!(validate_base_url("https://api.example.com/v1").is_ok());
    assert!(validate_base_url("   ").is_ok()); // 留给 HTTP client 报错
}

// =====================================================================
// 2. K-1.2 auth_token 字符白名单
// =====================================================================

#[test]
fn k1_auth_token_validate_chars() {
    // 合法字符: A-Z a-z 0-9 _ - = .
    assert!(validate_auth_token("sk-abc_123.=xyz").is_ok());
    // 空 token 允许
    assert!(validate_auth_token("").is_ok());
    // 非法字符: 空白, 控制符, 标点 (除 `_-.=`)
    assert!(matches!(
        validate_auth_token("sk abc"),
        Err(TuiError::AuthTokenInvalid(' '))
    ));
    assert!(matches!(
        validate_auth_token("sk\nabc"),
        Err(TuiError::AuthTokenInvalid('\n'))
    ));
    assert!(matches!(
        validate_auth_token("sk@abc"),
        Err(TuiError::AuthTokenInvalid('@'))
    ));
}

// =====================================================================
// 3. K-1.3 tool_whitelist 6 工具白名单
// =====================================================================

#[test]
fn k1_tool_whitelist_6_tools() {
    assert_eq!(TOOL_WHITELIST.len(), 6);
    for tool in TOOL_WHITELIST {
        assert!(validate_tool_name(tool).is_ok(), "{tool} 应在白名单");
    }
    // 未知拒绝
    assert!(matches!(
        validate_tool_name("unknown"),
        Err(TuiError::ToolNotInWhitelist(s)) if s == "unknown"
    ));
}

// =====================================================================
// 4. K-1.4 args 必须是 JSON object
// =====================================================================

#[test]
fn k1_args_must_be_object() {
    // object 接受
    assert!(validate_args_object(&json!({})).is_ok());
    assert!(validate_args_object(&json!({"k": "v"})).is_ok());
    // 非 object 拒绝 (array / string / number / null / bool)
    assert!(matches!(
        validate_args_object(&json!([])),
        Err(TuiError::ArgsNotObject(_))
    ));
    assert!(matches!(
        validate_args_object(&json!("s")),
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
    assert!(matches!(
        validate_args_object(&json!(true)),
        Err(TuiError::ArgsNotObject(_))
    ));
}

// =====================================================================
// 5. K-1.5 timeout 必须 > 0
// =====================================================================

#[test]
fn k1_timeout_must_be_positive() {
    // 0 拒绝
    assert!(matches!(
        validate_timeout(Duration::from_secs(0)),
        Err(TuiError::TimeoutInvalid(_))
    ));
    // 正数接受
    assert!(validate_timeout(Duration::from_secs(1)).is_ok());
    assert!(validate_timeout(Duration::from_millis(100)).is_ok());
    assert!(validate_timeout(Duration::from_secs(3600)).is_ok());

    // 完整构造路径: 5 K-1 联合校验
    let r = ApeirethClient::new("", Some("ok"), Duration::from_secs(30));
    assert!(matches!(r, Err(TuiError::BaseUrlEmpty)));
    let r = ApeirethClient::new("http://x", Some("bad token"), Duration::from_secs(30));
    assert!(matches!(r, Err(TuiError::AuthTokenInvalid(' '))));
    let r = ApeirethClient::new("http://x", None, Duration::from_secs(0));
    assert!(matches!(r, Err(TuiError::TimeoutInvalid(_))));
    // 全部合法 → Ok
    let r = ApeirethClient::new("http://localhost:8080/", Some("sk-abc_123.="), Duration::from_secs(30));
    assert!(r.is_ok());
    let c = r.unwrap();
    assert_eq!(c.base_url(), "http://localhost:8080"); // trailing / 剥掉
}

