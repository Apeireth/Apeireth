//! Fixture 5: in-process MCP 工具调用 (per RIVAL 蓝图 §3.7 缺口 5)
//!
//! 测 3 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. `TOOL_WHITELIST` 编译期 hardcode 包含 8 SSH 工具
//! 2. `validate_tool_call` 接受白名单内工具
//! 3. `validate_tool_call` 拒绝白名单外工具 (返回 SshMcpError::ToolNotWhitelisted)
//!
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_mcp_ssh::{validate_tool_call, SshMcpError, TOOL_WHITELIST};

#[test]
fn test_whitelist_contains_eight_ssh_tools() {
    // 8 工具: connect / disconnect / exec / upload / download / list / jump / keepalive
    assert_eq!(TOOL_WHITELIST.len(), 8);
    for tool in [
        "apeireth_ssh_connect",
        "apeireth_ssh_disconnect",
        "apeireth_ssh_exec",
        "apeireth_ssh_upload",
        "apeireth_ssh_download",
        "apeireth_ssh_list",
        "apeireth_ssh_jump",
        "apeireth_ssh_keepalive",
    ] {
        assert!(
            TOOL_WHITELIST.contains(&tool),
            "TOOL_WHITELIST 缺: {tool}"
        );
    }
}

#[test]
fn test_validate_tool_call_accepts_whitelisted() {
    // 白名单内应通过 (返回 Ok(()))
    let args = serde_json::json!({});
    for tool in TOOL_WHITELIST {
        let result = validate_tool_call(tool, &args);
        assert!(result.is_ok(), "白名单工具 {tool} 应通过: {result:?}");
    }
}

#[test]
fn test_validate_tool_call_rejects_unknown() {
    // m3 hallucination 防御: 不在白名单的工具必须拒绝
    let args = serde_json::json!({});
    let result = validate_tool_call("apeireth_ssh_run_shell", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        SshMcpError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_ssh_run_shell");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}
