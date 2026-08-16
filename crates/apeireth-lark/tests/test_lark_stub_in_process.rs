//! Fixture 5 + K-1 强校验: in-process Lark SDK stub 行为验证
//!
//! (per RIVAL 蓝图 §3.7 缺口 5 + 5 P0 crate 共享 fixture 模式)
//!
//! 测 7 件事 (in-process, 不走 HTTP, 直接调 lib API):
//! 1. `STUB_MODE` 编译期 hardcode = `true` (K-1 强校验 #4)
//! 2. `is_stub_mode()` 函数返 `true` (stub 模式守门)
//! 3. 7 编译期 hardcode 常量守门 (K-1 强校验 #1 + #2)
//! 4. `TOOL_WHITELIST` 编译期 hardcode 包含 9 Lark 工具 (K-1 强校验 #3)
//! 5. `validate_tool_call` 接受白名单内工具 + 拒绝白名单外 (m3 防御)
//! 6. **8 stub 工具全部返 `LarkError::NotImplemented(api_name)`** (额外 1 测试, 体现 stub 模式)
//! 7. 5 K-1 字样守门 (apeireth / lark / stub / send_message / must-do) + STUB_MODE == true
//!
//! 5 P0 crate + P1 SDK stub 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_lark::{
    is_stub_mode, validate_tool_call, LarkClient, LarkClientImpl, LarkConfig, LarkError,
    MessageType, LARK_API_BASE_URL, LARK_MAX_MESSAGE_LENGTH, LARK_SCHEMA_VERSION,
    LARK_TOKEN_CACHE_TTL_SECONDS, PLATFORM_NAME, STUB_MODE, SUPPORTED_MESSAGE_TYPES,
    TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
};

// ----- Fixture 1: STUB_MODE 编译期守门 (K-1 强校验 #4) -----

#[test]
fn test_stub_mode_compile_time_true() {
    // 编译期 hardcode = true, 改 false 需 6 哲学锚 + 主人审
    assert!(STUB_MODE, "STUB_MODE 编译期守门: 必须为 true");
    assert!(is_stub_mode(), "is_stub_mode() 必须返 true");
}

// ----- Fixture 2: 7 编译期 hardcode 常量守门 (K-1 强校验 #1 + #2) -----

#[test]
fn test_compile_time_constants_pinned() {
    assert_eq!(
        LARK_SCHEMA_VERSION, "1",
        "LARK_SCHEMA_VERSION 编译期 hardcode"
    );
    assert_eq!(
        PLATFORM_NAME, "apeireth",
        "K-1 强校验 #1: 平台名必须 apeireth"
    );
    assert_eq!(
        LARK_API_BASE_URL, "https://open.feishu.cn/open-apis",
        "LARK_API_BASE_URL 1:1 翻译飞书 Open API"
    );
    assert_eq!(
        LARK_TOKEN_CACHE_TTL_SECONDS, 7200,
        "LARK_TOKEN_CACHE_TTL_SECONDS = 2h"
    );
    assert_eq!(
        LARK_MAX_MESSAGE_LENGTH, 4096,
        "LARK_MAX_MESSAGE_LENGTH = 4 KB (飞书硬上限)"
    );
    assert_eq!(
        SUPPORTED_MESSAGE_TYPES.len(),
        5,
        "K-1 强校验 #2: 5 MessageType 枚举"
    );
    // 5 MessageType 全部 hardcode 列出
    assert!(SUPPORTED_MESSAGE_TYPES.contains(&MessageType::Text));
    assert!(SUPPORTED_MESSAGE_TYPES.contains(&MessageType::Post));
    assert!(SUPPORTED_MESSAGE_TYPES.contains(&MessageType::Image));
    assert!(SUPPORTED_MESSAGE_TYPES.contains(&MessageType::File));
    assert!(SUPPORTED_MESSAGE_TYPES.contains(&MessageType::Interactive));
}

// ----- Fixture 3: TOOL_WHITELIST 9 项 (K-1 强校验 #3) -----

#[test]
fn test_whitelist_contains_nine_lark_tools() {
    // 9 工具 = 8 工具 (1:1 翻译飞书 Open API 5 端点) + 1 stub_status 守门
    assert_eq!(TOOL_WHITELIST.len(), 9, "TOOL_WHITELIST 9 项");
    assert_eq!(TOOL_WHITELIST_COUNT, 9, "TOOL_WHITELIST_COUNT 编译期守门");
    for tool in [
        "apeireth_lark_send_message",
        "apeireth_lark_list_calendars",
        "apeireth_lark_create_event",
        "apeireth_lark_get_document",
        "apeireth_lark_list_bitable_records",
        "apeireth_lark_create_bitable_record",
        "apeireth_lark_search_documents",
        "apeireth_lark_auth_refresh",
        "apeireth_lark_stub_status",
    ] {
        assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST 缺: {tool}");
    }
}

// ----- Fixture 4: m3 防御 — 白名单校验 (per m3-hallucination-defense §2.4) -----

#[test]
fn test_validate_tool_call_accepts_whitelisted() {
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let result = validate_tool_call(tool, &args);
        assert!(result.is_ok(), "白名单工具 {tool} 应通过: {result:?}");
    }
}

#[test]
fn test_validate_tool_call_rejects_unknown() {
    let args = serde_json::json!({});
    let result = validate_tool_call("apeireth_lark_send_email", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        LarkError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_lark_send_email");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}

// ----- Fixture 5 (额外 1, 体现 stub 模式): 8 stub 工具返 NotImplemented -----

#[tokio::test]
async fn test_eight_stub_tools_return_not_implemented() {
    let client = LarkClientImpl::new(LarkConfig::default()).unwrap();

    // 消息 (1)
    let r = client.send_message("u1", MessageType::Text, "hi").await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "send_message"),
        "send_message 必须返 NotImplemented(\"send_message\"), 实际: {r:?}"
    );

    // 日历 (2)
    let r = client.list_calendars().await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "list_calendars"),
        "list_calendars 必须返 NotImplemented(\"list_calendars\"), 实际: {r:?}"
    );
    let r = client.create_event("cal1", "meet", 0, 0).await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "create_event"),
        "create_event 必须返 NotImplemented(\"create_event\"), 实际: {r:?}"
    );

    // 文档 (2)
    let r = client.get_document("doc1").await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "get_document"),
        "get_document 必须返 NotImplemented(\"get_document\"), 实际: {r:?}"
    );
    let r = client.search_documents("q", 10).await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "search_documents"),
        "search_documents 必须返 NotImplemented(\"search_documents\"), 实际: {r:?}"
    );

    // Bitable (2)
    let r = client.list_bitable_records("app1", "tbl1", 10).await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "list_bitable_records"),
        "list_bitable_records 必须返 NotImplemented(\"list_bitable_records\"), 实际: {r:?}"
    );
    let r = client
        .create_bitable_record("app1", "tbl1", serde_json::json!({}))
        .await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "create_bitable_record"),
        "create_bitable_record 必须返 NotImplemented(\"create_bitable_record\"), 实际: {r:?}"
    );

    // Auth (1)
    let r = client.auth_refresh().await;
    assert!(
        matches!(r, Err(LarkError::NotImplemented(ref s)) if s == "auth_refresh"),
        "auth_refresh 必须返 NotImplemented(\"auth_refresh\"), 实际: {r:?}"
    );
}

// ----- Fixture 6: LarkClientImpl 基础行为 (空 config 拒绝) -----

#[test]
fn test_lark_client_impl_rejects_empty_config() {
    let cfg = LarkConfig {
        app_id: String::new(),
        app_secret: "secret".into(),
        base_url: LARK_API_BASE_URL.into(),
        token_cache_ttl_seconds: LARK_TOKEN_CACHE_TTL_SECONDS,
    };
    let r = LarkClientImpl::new(cfg);
    assert!(matches!(r, Err(LarkError::ConfigInvalid(_))));
}

#[test]
fn test_lark_client_impl_default_token_invalid() {
    let client = LarkClientImpl::new(LarkConfig::default()).unwrap();
    // STUB 模式: token_cache = None, token_valid 必返 false
    assert!(
        !client.token_valid(),
        "STUB 模式 token_cache None, token_valid 应为 false"
    );
}

// ----- Fixture 7: 5 K-1 字样守门 (apeireth / lark / stub / send_message / must-do) + STUB_MODE -----

#[test]
fn test_k1_must_do_invariants() {
    // 5 K-1 字样 (per 任务 K-1 强校验 #4):
    // 1) "apeireth" 平台名
    assert_eq!(PLATFORM_NAME, "apeireth", "K-1 字样 #1: apeireth");
    // 2) "lark" crate 名
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(
        crate_name.contains("lark"),
        "K-1 字样 #2: lark in crate name ({crate_name})"
    );
    // 3) "stub" 模式守门
    assert!(is_stub_mode(), "K-1 字样 #3: stub 模式");
    // 4) "send_message" 8 工具之一
    assert!(
        TOOL_WHITELIST.contains(&"apeireth_lark_send_message"),
        "K-1 字样 #4: send_message in TOOL_WHITELIST"
    );
    // 5) "must-do" R20 阶段 3 必补 (从 STUB_MODE 守门守门)
    assert!(
        STUB_MODE,
        "K-1 字样 #5: must-do (STUB_MODE true 守 R20 阶段 3 必补)"
    );
}
