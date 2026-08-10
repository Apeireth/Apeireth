//! Fixture 5: in-process MCP 工具调用 (per RIVAL 蓝图 §3.7 缺口 5)
//!
//! 测 3 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. `TOOL_WHITELIST` 编译期 hardcode 包含 8 WinRM 工具
//! 2. `validate_tool_call` 接受白名单内工具
//! 3. `validate_tool_call` 拒绝白名单外工具 (返回 WinRmMcpError::ToolNotWhitelisted)
//!
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_mcp_winrm::{validate_tool_call, WinRmMcpError, TOOL_WHITELIST};

#[test]
fn test_whitelist_contains_eight_winrm_tools() {
    // 8 工具: connect / disconnect / list_connections / run_command / get_command_output /
    //         command / copy_to / copy_from
    assert_eq!(TOOL_WHITELIST.len(), 8);
    for tool in [
        "apeireth_winrm_connect",
        "apeireth_winrm_disconnect",
        "apeireth_winrm_list_connections",
        "apeireth_winrm_run_command",
        "apeireth_winrm_get_command_output",
        "apeireth_winrm_command",
        "apeireth_winrm_copy_to",
        "apeireth_winrm_copy_from",
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
    let result = validate_tool_call("apeireth_winrm_run_powershell", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        WinRmMcpError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_winrm_run_powershell");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}
