//! Fixture 5: in-process MCP 工具调用 (per RIVAL 蓝图 §3.7 缺口 5)
//!
//! 测 3 件事 (in-process, 不走 stdio / HTTP, 直接调 lib API):
//! 1. `TOOL_WHITELIST` 编译期 hardcode 包含 14 Team Lead 工具 (8 调度 + 3 worktree + 3 感知)
//! 2. `validate_tool_call` 接受白名单内工具
//! 3. `validate_tool_call` 拒绝白名单外工具 (返回 TeamLeadError::ToolNotWhitelisted)
//!
//! 5 P0 crate 共享同一 fixture 模式, 避免重复造轮子 (per 蓝图 §3.7 缺口 5).

use apeireth_team_lead::{validate_tool_call, TeamLeadError, TOOL_WHITELIST};

#[test]
fn test_whitelist_contains_fourteen_team_lead_tools() {
    // 14 工具 = 8 调度 + 3 worktree + 3 感知
    assert_eq!(TOOL_WHITELIST.len(), 14);
    for tool in [
        // 8 调度
        "apeireth_team_lead_spawn_agent",
        "apeireth_team_lead_send_to_agent",
        "apeireth_team_lead_get_agent_output",
        "apeireth_team_lead_wait_agent_idle",
        "apeireth_team_lead_wait_agent",
        "apeireth_team_lead_get_agent_status",
        "apeireth_team_lead_list_agents",
        "apeireth_team_lead_cancel_agent",
        // 3 worktree
        "apeireth_team_lead_get_task_info",
        "apeireth_team_lead_check_merge",
        "apeireth_team_lead_merge_worktree",
        // 3 感知
        "apeireth_team_lead_list_sessions",
        "apeireth_team_lead_get_session_summary",
        "apeireth_team_lead_search_sessions",
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
    let result = validate_tool_call("apeireth_team_lead_destroy_agent", &args);
    assert!(result.is_err(), "白名单外工具必须拒绝");
    match result.unwrap_err() {
        TeamLeadError::ToolNotWhitelisted(t) => {
            assert_eq!(t, "apeireth_team_lead_destroy_agent");
        }
        other => panic!("期望 ToolNotWhitelisted, 实际: {other:?}"),
    }
}
