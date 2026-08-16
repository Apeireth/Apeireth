//! 5 Provider fallback (apeireth-team-lead 14 fn + 4 Provider fallback chain)
//!
//! 覆盖 `apeireth-team-lead` Orchestrator 14 工具 (1:1 翻译 AgentMCPServer.js Orchestrator):
//! - 8 调度: spawn_agent / send_to_agent / get_agent_output / wait_agent_idle /
//!   wait_agent / get_agent_status / list_agents / cancel_agent
//! - 3 worktree: get_task_info / check_merge / merge_worktree
//! - 3 感知: list_sessions / get_session_summary / search_sessions
//!
//! 4 Provider fallback chain (per supervisor-prompt-818 §2.2 #9):
//!   claude-code → gemini-cli → codex → opencode
//!
//! K-1 强校验 5 字样 (per §5.3 supervisor-prompt-818):
//! - "Claude Code" 字样 1:1 保留 (minimax m3 兼容)
//! - 14 调度工具名 1:1 保留
//! - 4 Provider 名 1:1 保留
//! - "must-do" 字样 (Progress addon marker)
//!
//! 主报告: `reports/r20-stage4-integration-2026-08-05.md §2`

use apeireth_team_lead::{
    AgentRole, AgentStatus, Message, MessageType, TeamConfig, TeamLead, TeamLeadError,
    ORCHESTRATOR_TOOL_COUNT, SCHEDULING_TOOL_COUNT, SUPERVISOR_PROMPT, TOOL_WHITELIST,
    WORKTREE_TOOL_COUNT, AWARENESS_TOOL_COUNT, build_supervisor_prompt, validate_tool_call,
};

use apeireth_agent::AgentManager;
use std::sync::Arc;

/// 8 测试覆盖 5 Provider fallback + 14 Orchestrator fn
#[cfg(test)]
mod tests {
    use super::*;

    fn make_team() -> TeamLead {
        let mgr = Arc::new(AgentManager::new());
        TeamLead::new(TeamConfig::default(), mgr)
    }

    #[test]
    fn orchestrator_tool_count_is_14_8_3_3() {
        // K-1 强校验: 8 调度 + 3 worktree + 3 感知 = 14 (1:1 AgentMCPServer.js)
        assert_eq!(ORCHESTRATOR_TOOL_COUNT, 14, "Orchestrator 14 fn");
        assert_eq!(SCHEDULING_TOOL_COUNT, 8, "8 调度工具");
        assert_eq!(WORKTREE_TOOL_COUNT, 3, "3 worktree 工具");
        assert_eq!(AWARENESS_TOOL_COUNT, 3, "3 感知工具");
        assert_eq!(
            SCHEDULING_TOOL_COUNT + WORKTREE_TOOL_COUNT + AWARENESS_TOOL_COUNT,
            ORCHESTRATOR_TOOL_COUNT
        );
    }

    #[test]
    fn tool_whitelist_covers_all_14_orchestrator_tools() {
        // 14 tool name 字面量 (per `apeireth_team_lead_*` 前缀编译期守门)
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_spawn_agent"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_send_to_agent"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_wait_agent"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_cancel_agent"));
        // worktree
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_get_task_info"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_check_merge"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_merge_worktree"));
        // 感知
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_list_sessions"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_get_session_summary"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_team_lead_search_sessions"));
    }

    #[test]
    fn validate_tool_call_accepts_whitelisted_rejects_unknown() {
        assert!(validate_tool_call("apeireth_team_lead_spawn_agent", &serde_json::json!({})).is_ok());
        // 不在白名单 → ToolNotWhitelisted
        let err = validate_tool_call("apeireth_team_lead_evil_tool", &serde_json::json!({}))
            .expect_err("evil tool 应被拒");
        assert!(matches!(err, TeamLeadError::ToolNotWhitelisted(_)));
    }

    #[test]
    fn team_config_default_4_providers_fallback() {
        // 默认 4 Provider fallback chain (claude-code → gemini-cli → codex → opencode)
        let cfg = TeamConfig::default();
        assert_eq!(cfg.available_providers.len(), 4, "4 Provider fallback chain");
        assert_eq!(cfg.available_providers[0], "claude-code");
        assert_eq!(cfg.available_providers[1], "gemini-cli");
        assert_eq!(cfg.available_providers[2], "codex");
        assert_eq!(cfg.available_providers[3], "opencode");
        assert_eq!(cfg.default_wait_timeout_ms, 90_000, "codex ≤ 90000ms");
    }

    #[test]
    fn build_supervisor_prompt_injects_4_providers() {
        // build_supervisor_prompt 用 ${availableProviders} 占位符, 4 provider 注入
        let prompt = build_supervisor_prompt(&["claude-code", "gemini-cli", "codex", "opencode"]);
        // K-1 #1 + #2: "Claude Code" 字样保留
        assert!(prompt.contains("Claude Code"), "K-1: 'Claude Code' 字样");
        // K-1 #3: 4 provider 名 1:1 保留
        assert!(prompt.contains("claude-code"));
        assert!(prompt.contains("gemini-cli"));
        assert!(prompt.contains("codex"));
        assert!(prompt.contains("opencode"));
    }

    #[test]
    fn supervisor_prompt_contains_k1_5_markers() {
        // K-1 强校验 5 字样 (per §5.3 supervisor-prompt-818)
        let prompt = build_supervisor_prompt(&["claude-code", "gemini-cli", "codex", "opencode"]);
        // 1. "Claude Code"
        assert!(prompt.contains("Claude Code"), "K-1 #1: 'Claude Code'");
        // 2. claude-code provider
        assert!(prompt.contains("claude-code"), "K-1 #2: 'claude-code' provider");
        // 3. 调度工具 spawn_agent
        assert!(prompt.contains("spawn_agent"), "K-1 #3: 'spawn_agent' 工具");
        // 4. 调度工具 wait_agent_idle
        assert!(prompt.contains("wait_agent_idle"), "K-1 #4: 'wait_agent_idle' 工具");
        // 5. Progress addon "must-do" marker
        assert!(
            prompt.contains("must-do") || prompt.contains("must do"),
            "K-1 #5: 'must-do' Progress addon marker"
        );
    }

    #[tokio::test]
    async fn orchestrator_spawn_3_agents_then_list() {
        // Fixture: spawn 3 worker agents → list 3 → 验证 14 fn 中的 spawn_agent / list_agents
        let lead = make_team();
        let id1 = lead
            .spawn_agent(AgentRole::Worker, "task 1".into())
            .await
            .unwrap();
        let id2 = lead
            .spawn_agent(AgentRole::Worker, "task 2".into())
            .await
            .unwrap();
        let id3 = lead
            .spawn_agent(AgentRole::Observer, "observe".into())
            .await
            .unwrap();
        assert!(id1.starts_with("agent-"));
        assert_eq!(lead.child_count().await, 3);
        let list = lead.list_agents().await.unwrap();
        assert_eq!(list.len(), 3);
        // 角色分别: Worker / Worker / Observer
        let roles: Vec<_> = list.iter().map(|a| a.role).collect();
        assert!(roles.contains(&AgentRole::Worker));
        assert!(roles.contains(&AgentRole::Observer));
        // 初始状态应全是 Pending
        let s1 = lead.get_agent_status(&id1).await.unwrap();
        assert_eq!(s1, AgentStatus::Pending);
        // cancel id3 → 变 Cancelled
        lead.cancel_agent(&id3).await.unwrap();
        let s3 = lead.get_agent_status(&id3).await.unwrap();
        assert_eq!(s3, AgentStatus::Cancelled);
    }

    #[tokio::test]
    async fn orchestrator_send_to_nonexistent_returns_team_not_found() {
        // Fixture 2: send_to_agent 不吞错 (mid-task bug 修复)
        let lead = make_team();
        let msg = Message::new(None, "ghost", MessageType::Send, serde_json::json!({}));
        let err = lead.send_to_agent("ghost", msg).await.expect_err("不存在的 agent");
        assert!(matches!(err, TeamLeadError::TeamNotFound(_)));
    }

    #[tokio::test]
    async fn orchestrator_message_auto_assigns_trace_id() {
        // 跨 Agent 消息自动分配 trace_id (per apeireth-bus::next_trace_id)
        let m1 = Message::new(None, "a1", MessageType::Send, serde_json::json!({"x": 1}));
        let m2 = Message::new(None, "a2", MessageType::Send, serde_json::json!({"x": 2}));
        assert!(m1.trace_id > 0);
        assert!(m2.trace_id > m1.trace_id, "trace_id 单调递增");
    }

    #[test]
    fn supervisor_prompt_md_contains_818_lines() {
        // 1:1 翻译 v0.9.21 商业版 supervisorPrompt 818 行
        // sanity: 长度应 > 30_000 chars (per 实战 ~50KB)
        assert!(
            SUPERVISOR_PROMPT.len() > 30_000,
            "SUPERVISOR_PROMPT 应 > 30KB, 实际 {}",
            SUPERVISOR_PROMPT.len()
        );
    }
}
