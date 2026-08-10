//! Fixture 5: in-process MCP 工具调用 (per RIVAL 蓝图 §3.7 缺口 5)
//!
//! 测 3 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. `TOOL_WHITELIST` 编译期 hardcode 包含 5 Image Relay 工具
//! 2. `validate_tool_call` 接受白名单内工具
//! 3. `validate_tool_call` 拒绝白名单外工具 (返回 RelayImageError::ToolNotWhitelisted)
//!
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_mcp_relay_image::{validate_tool_call, RelayImageError, TOOL_WHITELIST};

#[test]
fn test_whitelist_contains_five_relay_image_tools() {
    // 5 工具: relay / hash / decode / compress / list_cached
    assert_eq!(TOOL_WHITELIST.len(), 5);
    for tool in [
        "apeireth_relay_image_relay",
        "apeireth_relay_image_hash",
        "apeireth_relay_image_decode",
        "apeireth_relay_image_compress",
        "apeireth_relay_image_list_cached",
    ] {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺: {tool}"
        );
    }
}

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
    let result = validate_tool_call("apeireth_relay_image_edit", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        RelayImageError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_relay_image_edit");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}
