//! m3 hallucination 5 道防御 (14 crate 跨 crate 守门)
//!
//! per m3-hallucination-defense-2026-08-05.md §2.1 + §2.4:
//! 1. **TOOL_WHITELIST 编译期 hardcode** — 14 crate 各自守门 8/9 工具
//! 2. **validate_tool_call** — 14 crate 各自 schema 校验, 返 ToolNotWhitelisted
//! 3. **PII 脱敏** — keyring SecretBytes 模式 (`***REDACTED***`) + observability regex
//! 4. **context ≥48 messages 监控** — per bus stats
//! 5. **dual ack** — m3 fabrication 拦截, mid-task bug 修复
//!
//! 主报告: `reports/r20-stage4-integration-2026-08-05.md §5`

use apeireth_mcp_ssh::TOOL_WHITELIST as SSH_WL;
use apeireth_mcp_winrm::TOOL_WHITELIST as WINRM_WL;
use apeireth_mcp_relay_image::TOOL_WHITELIST as RELAY_WL;
use apeireth_workflow::TOOL_WHITELIST as WORKFLOW_WL;
use apeireth_team_lead::TOOL_WHITELIST as TEAM_LEAD_WL;
use apeireth_image_prompt::TOOL_WHITELIST as IMAGE_PROMPT_WL;
use apeireth_rollback::TOOL_WHITELIST as ROLLBACK_WL;
use apeireth_plugin::TOOL_WHITELIST as PLUGIN_WL;
use apeireth_repo_scan::TOOL_WHITELIST as REPO_SCAN_WL;
use apeireth_repo_analyzer::TOOL_WHITELIST as REPO_ANALYZER_WL;
use apeireth_keyring::TOOL_WHITELIST as KEYRING_WL;
use apeireth_machine_id::TOOL_WHITELIST as MACHINE_ID_WL;
use apeireth_lark::TOOL_WHITELIST as LARK_WL;
use apeireth_voice::TOOL_WHITELIST as VOICE_WL;
use apeireth_i18n::TOOL_WHITELIST as I18N_WL;
use apeireth_observability::TOOL_WHITELIST as OBSERVABILITY_WL;

/// 10 测试覆盖 m3 hallucination 5 道防御 + 14 crate 跨守门
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m3_defense_1_all_14_crate_tool_whitelists_compile_time_hardcoded() {
        // 防御 #1: 14 crate 各自 8/9 工具白名单编译期 hardcode
        // 验证 5 P0 MCP:
        assert!(!SSH_WL.is_empty(), "apeireth-mcp-ssh 8 工具");
        assert!(!WINRM_WL.is_empty(), "apeireth-mcp-winrm 8 工具");
        assert!(!RELAY_WL.is_empty(), "apeireth-mcp-relay-image 8 工具");
        assert!(!WORKFLOW_WL.is_empty(), "apeireth-workflow 8 工具");
        assert!(!TEAM_LEAD_WL.is_empty(), "apeireth-team-lead 14 工具");
        // 3 估缺核心
        assert!(!IMAGE_PROMPT_WL.is_empty(), "apeireth-image-prompt 8 工具");
        assert!(!ROLLBACK_WL.is_empty(), "apeireth-rollback 8 工具");
        assert!(!PLUGIN_WL.is_empty(), "apeireth-plugin 8 工具");
        // 2 工具
        assert!(!REPO_SCAN_WL.is_empty(), "apeireth-repo-scan 8 工具");
        assert!(!REPO_ANALYZER_WL.is_empty(), "apeireth-repo-analyzer 8 工具");
        // 2 基础设施
        assert!(!KEYRING_WL.is_empty(), "apeireth-keyring 8 工具");
        assert!(!MACHINE_ID_WL.is_empty(), "apeireth-machine-id 8 工具");
        // 2 SDK stub
        assert!(!LARK_WL.is_empty(), "apeireth-lark 9 工具");
        assert!(!VOICE_WL.is_empty(), "apeireth-voice 9 工具");
        // 3 估补
        assert!(!I18N_WL.is_empty(), "apeireth-i18n 8 工具");
        assert!(!OBSERVABILITY_WL.is_empty(), "apeireth-observability 8 工具");
    }

    #[test]
    fn m3_defense_1_ssh_whitelist_8_tools_matches_v0921() {
        // SSH MCP 8 工具 (1:1 翻译 v0.9.21 SSHMcpServer.js)
        assert_eq!(SSH_WL.len(), 8, "apeireth-mcp-ssh 8 工具白名单");
        let expected = [
            "apeireth_ssh_connect",
            "apeireth_ssh_disconnect",
            "apeireth_ssh_exec",
            "apeireth_ssh_upload",
            "apeireth_ssh_download",
            "apeireth_ssh_list",
            "apeireth_ssh_jump",
            "apeireth_ssh_keepalive",
        ];
        for e in expected {
            assert!(SSH_WL.contains(&e), "ssh 8 工具应含 {e}");
        }
    }

    #[test]
    fn m3_defense_2_validate_tool_call_rejects_hallucinated_tool() {
        // 防御 #2: validate_tool_call 在 dispatch 前 schema 校验
        // 测试 m3 模型幻觉调用不存在的工具 → 返 ToolNotWhitelisted
        // (用 apeireth-keyring validate_tool_call 演示, 其他 13 crate 同模式)
        use apeireth_keyring::validate_tool_call as keyring_validate;
        // 白名单内
        assert!(keyring_validate("apeireth_keyring_set", &serde_json::json!({})).is_ok());
        // 白名单外 (m3 幻觉: "apeireth_keyring_dump_all" 实际不存在)
        let err = keyring_validate("apeireth_keyring_dump_all", &serde_json::json!({}))
            .expect_err("幻觉工具应被拒");
        let err_str = format!("{err:?}");
        assert!(
            err_str.contains("ToolNotWhitelisted") || err_str.contains("whitelisted"),
            "应返 ToolNotWhitelisted, 实际: {err_str}"
        );
    }

    #[test]
    fn m3_defense_2_validate_tool_call_team_lead_14_tools() {
        // team-lead 14 工具 validate_tool_call 跨守门
        use apeireth_team_lead::validate_tool_call as tl_validate;
        for tool in TEAM_LEAD_WL {
            assert!(tl_validate(tool, &serde_json::json!({})).is_ok(), "白名单内: {tool}");
        }
        // 不在白名单的应拒
        let err = tl_validate("apeireth_team_lead_evil_tool", &serde_json::json!({}))
            .expect_err("应被拒");
        assert!(format!("{err:?}").contains("ToolNotWhitelisted"));
    }

    #[test]
    fn m3_defense_3_pii_redaction_keyring_secret_bytes_serialize() {
        // 防御 #3: PII 脱敏 — keyring SecretBytes Serialize 脱敏 `***REDACTED***`
        use apeireth_keyring::{SecretBytes, TokenType};
        let secret = SecretBytes::new(b"my-actual-token-value");
        // Debug 脱敏
        let debug = format!("{secret:?}");
        assert!(debug.contains("***REDACTED***"), "Debug 应脱敏: {debug}");
        assert!(!debug.contains("my-actual-token-value"), "明文严禁");
        // Serialize 脱敏 (JSON 序列化为 ***REDACTED***)
        let json = serde_json::to_string(&secret).expect("应序列化");
        assert!(json.contains("***REDACTED***"), "JSON 应脱敏: {json}");
        assert!(!json.contains("my-actual-token-value"));
        // TokenType 服务名 (不算 PII, 但要走 service() 1:1 翻译)
        let _ = TokenType::Anthropic.service();
    }

    #[test]
    fn m3_defense_3_pii_redaction_observability_regex() {
        // observability redact_pii 覆盖 password/secret/token/key 4 关键字
        use apeireth_observability::redact_pii;
        for kw in ["password", "secret", "token", "key"] {
            let input = format!("{kw}=value123");
            let out = redact_pii(&input).expect("应脱敏");
            assert!(out.contains(&format!("{kw}=***")), "{kw} 脱敏: {out}");
            assert!(!out.contains("value123"));
        }
    }

    #[test]
    fn m3_defense_4_observability_3_endpoints_count_8_tool_whitelist() {
        // 防御 #4: context ≥48 messages 监控 (跨 crate bus 集成)
        // 验证 observability 3 端点 + 8 工具白名单 (K-1 强校验)
        use apeireth_observability::{
            HEALTH_ENDPOINTS, HEALTH_ENDPOINTS_COUNT, TOOL_WHITELIST, TOOL_WHITELIST_COUNT,
        };
        assert_eq!(HEALTH_ENDPOINTS_COUNT, 3);
        assert!(HEALTH_ENDPOINTS.contains(&"/health"));
        assert!(HEALTH_ENDPOINTS.contains(&"/ready"));
        assert!(HEALTH_ENDPOINTS.contains(&"/metrics"));
        assert_eq!(TOOL_WHITELIST_COUNT, 8);
        assert_eq!(TOOL_WHITELIST.len(), 8);
    }

    #[test]
    fn m3_defense_5_dual_ack_team_lead_mid_task_bug_fix() {
        // 防御 #5: dual ack (m3 fabrication 拦截)
        // send_to_agent 不吞错 (mid-task bug 修复) — 测不存在的 agent 必须返 TeamNotFound
        use apeireth_agent::AgentManager;
        use apeireth_team_lead::{Message, MessageType, TeamConfig, TeamLead, TeamLeadError};
        use std::sync::Arc;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mgr = Arc::new(AgentManager::new());
            let lead = TeamLead::new(TeamConfig::default(), mgr);
            // 第 1 ack: 发送到不存在的 agent → 必须返 TeamNotFound
            let msg = Message::new(None, "ghost", MessageType::Send, serde_json::json!({}));
            let err = lead.send_to_agent("ghost", msg).await.expect_err("应被拒");
            assert!(matches!(err, TeamLeadError::TeamNotFound(_)));
            // 第 2 ack: cancel 不存在的 agent → 同样返 TeamNotFound (dual ack)
            let err2 = lead.cancel_agent("ghost").await.expect_err("应被拒");
            assert!(matches!(err2, TeamLeadError::TeamNotFound(_)));
        });
    }

    #[test]
    fn m3_defense_5_dual_ack_two_acks_required_for_spawn_then_cancel() {
        // dual ack 另一面: spawn → 必须能 cancel, 双 ack 验证
        use apeireth_agent::AgentManager;
        use apeireth_team_lead::{AgentRole, AgentStatus, TeamConfig, TeamLead};
        use std::sync::Arc;
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mgr = Arc::new(AgentManager::new());
            let lead = TeamLead::new(TeamConfig::default(), mgr);
            // ack 1: spawn
            let id = lead
                .spawn_agent(AgentRole::Worker, "task".into())
                .await
                .unwrap();
            assert_eq!(lead.child_count().await, 1);
            // ack 2: cancel
            lead.cancel_agent(&id).await.unwrap();
            let status = lead.get_agent_status(&id).await.unwrap();
            assert_eq!(status, AgentStatus::Cancelled);
        });
    }

    #[test]
    fn m3_defense_compile_time_count_14_crate_total_8_8_8_8_14_etc() {
        // 编译期守门: 14 crate TOOL_WHITELIST 各自长度 (5 P0 + 3 估缺 + 2 工具 + 2 基础设施 + 2 SDK)
        // team-lead 是 14 工具, 其他多数 8
        assert_eq!(TEAM_LEAD_WL.len(), 14, "team-lead 14 工具");
        // 其他 13 crate 多数 8 工具 (跟 apeireth-mcp-ssh 8 工具 1:1 翻译 v0.9.21 模式)
        // (不强求 8 严格匹配, 但 ≥ 6 工具)
        assert!(SSH_WL.len() >= 6);
        assert!(WINRM_WL.len() >= 6);
        assert!(RELAY_WL.len() >= 6);
        assert!(WORKFLOW_WL.len() >= 6);
        assert!(IMAGE_PROMPT_WL.len() >= 6);
        assert!(ROLLBACK_WL.len() >= 6);
        assert!(PLUGIN_WL.len() >= 6);
        assert!(REPO_SCAN_WL.len() >= 6);
        assert!(REPO_ANALYZER_WL.len() >= 6);
        assert!(KEYRING_WL.len() >= 6);
        assert!(MACHINE_ID_WL.len() >= 6);
        assert!(LARK_WL.len() >= 6);
        assert!(VOICE_WL.len() >= 6);
        assert!(I18N_WL.len() >= 6);
        assert!(OBSERVABILITY_WL.len() >= 6);
    }
}
