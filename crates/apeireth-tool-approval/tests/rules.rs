//! Integration tests for apeireth-tool-approval
//!
//! **R18 第 2 阶段第 6 项**: 5 规则真实现 + ApprovalManager 集成

use apeireth_tool_approval::{
    decision::ApprovalDecision,
    manager::ApprovalManager,
    rule::{BlacklistRule, FrequencyRule, RiskRule, TrustRule, WhitelistRule},
    rule_trait::ApprovalRule,
};
use apeireth_tool_runtime::{parser::ToolCallParser, ParsedToolCall};

// =====================================================================
// Helper: 从 LLM 输出解析 → ParsedToolCall
// =====================================================================

fn parse_one(output: &str) -> ParsedToolCall {
    let calls = ToolCallParser::parse(output).expect("parse");
    assert_eq!(calls.len(), 1);
    calls.into_iter().next().unwrap()
}

// =====================================================================
// 5 规则真实现测试
// =====================================================================

#[test]
fn trust_rule_allows_listed_tool() {
    let rule = TrustRule::with_trusted(vec!["FileOperator".to_string()]);
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<FileOperator>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }
}

#[test]
fn trust_rule_no_match_for_unlisted_tool() {
    let rule = TrustRule::with_trusted(vec!["FileOperator".to_string()]);
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<WebSearch>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::NoMatch => {}
        other => panic!("expected NoMatch, got {:?}", other),
    }
}

#[test]
fn blacklist_rule_denies_silently() {
    let rule = BlacklistRule::with_blacklist(vec!["Dangerous".to_string()], true);
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<Dangerous>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::Deny { silent, .. } => assert!(silent, "blacklist should be silent"),
        other => panic!("expected Deny silent, got {:?}", other),
    }
}

#[test]
fn blacklist_rule_non_silent() {
    let rule = BlacklistRule::with_blacklist(vec!["Banned".to_string()], false);
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<Banned>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::Deny { silent, .. } => assert!(!silent, "non-silent blacklist"),
        other => panic!("expected Deny, got {:?}", other),
    }
}

#[test]
fn risk_rule_requires_approval() {
    let rule = RiskRule::new(5 * 60 * 1000); // 5min
                                             // RiskRule 高风险类别前缀 ["system", "network", "file"] (per `rule.rs:137 DEFAULT_HIGH_RISK_CATEGORIES`)
                                             // 用 file_delete 触发前缀匹配 → RequireApproval
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<file_delete>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::RequireApproval { timeout_ms } => {
            assert_eq!(timeout_ms, 5 * 60 * 1000);
        }
        other => panic!("expected RequireApproval, got {:?}", other),
    }
}

#[test]
fn frequency_rule_no_match_on_empty_history() {
    let rule = FrequencyRule::new();
    let call = parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<AnyTool>>>\n<<<[END_TOOL_REQUEST]>>>");
    match rule.check(&call, &[]) {
        ApprovalDecision::NoMatch => {}
        other => panic!("expected NoMatch on empty history, got {:?}", other),
    }
}

#[test]
fn whitelist_rule_allows_listed_tool() {
    let rule = WhitelistRule::with_whitelist(vec!["Safe".to_string()]);
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<Safe>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::Allow => {}
        other => panic!("expected Allow, got {:?}", other),
    }
}

#[test]
fn whitelist_rule_no_match_for_unlisted_tool() {
    let rule = WhitelistRule::with_whitelist(vec!["Safe".to_string()]);
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<Other>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::NoMatch => {}
        other => panic!("expected NoMatch, got {:?}", other),
    }
}

// =====================================================================
// ApprovalManager 集成 — builder + 5 规则按顺序检查
// =====================================================================

#[test]
fn manager_default_has_no_rules() {
    let m = ApprovalManager::new();
    assert_eq!(m.rule_count(), 0);
}

#[test]
fn manager_with_rules_builder() {
    let trust = TrustRule::with_trusted(vec!["T".to_string()]);
    let m = ApprovalManager::with_rules(vec![Box::new(trust)]);
    assert_eq!(m.rule_count(), 1);
}

#[test]
fn manager_with_trust_rule_allows() {
    let trust = TrustRule::with_trusted(vec!["T".to_string()]);
    let m = ApprovalManager::with_rules(vec![Box::new(trust)]);
    let call =
        parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<T>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>");
    let decision = m.check(&call);
    assert!(matches!(decision, ApprovalDecision::Allow));
}

#[test]
fn manager_short_circuits_on_first_match() {
    // Trust 在前 → Allow (即使后还有 Blacklist 也不会被检查)
    let trust = TrustRule::with_trusted(vec!["T".to_string()]);
    let blacklist = BlacklistRule::with_blacklist(vec!["T".to_string()], true);
    let m = ApprovalManager::with_rules(vec![Box::new(trust), Box::new(blacklist)]);
    assert_eq!(m.rule_count(), 2);

    let call =
        parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<T>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>");
    let decision = m.check(&call);
    assert!(
        matches!(decision, ApprovalDecision::Allow),
        "Trust should short-circuit before Blacklist, got {:?}",
        decision
    );
}

#[test]
fn manager_timeout_default_is_5min() {
    let m = ApprovalManager::new();
    assert_eq!(m.approval_timeout_ms(), 5 * 60 * 1000);
}

#[test]
fn manager_timeout_can_be_overridden() {
    let m = ApprovalManager::new().with_timeout(30_000);
    assert_eq!(m.approval_timeout_ms(), 30_000);
}

#[test]
fn manager_add_rule_dynamic() {
    let mut m = ApprovalManager::new();
    assert_eq!(m.rule_count(), 0);
    m.add_rule(Box::new(TrustRule::with_trusted(vec!["A".to_string()])));
    assert_eq!(m.rule_count(), 1);
    m.add_rule(Box::new(TrustRule::with_trusted(vec!["B".to_string()])));
    assert_eq!(m.rule_count(), 2);
}

#[test]
fn manager_history_starts_empty() {
    let m = ApprovalManager::new();
    assert_eq!(m.history_len(), 0);
}

// =====================================================================
// RiskRule 类别自定义 + FrequencyRule 频率窗口
// =====================================================================

#[test]
fn risk_rule_with_custom_categories() {
    // RiskRule 允许自定义高风险类别 (实战中主人可改)
    let rule =
        RiskRule::with_categories(5 * 60 * 1000, vec!["db".to_string(), "shell".to_string()]);
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<db_query>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    match rule.check(&call, &[]) {
        ApprovalDecision::RequireApproval { .. } => {}
        other => panic!("db_query 应 RequireApproval (自定义类别), 实际: {other:?}"),
    }
    // 类别外的工具 NoMatch
    let call2 =
        parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<file_read>>>\n<<<[END_TOOL_REQUEST]>>>");
    assert!(matches!(rule.check(&call2, &[]), ApprovalDecision::NoMatch));
}

#[test]
fn risk_rule_is_high_risk_helper() {
    let rule = RiskRule::new(5 * 60 * 1000);
    assert!(rule.is_high_risk("system_exec"));
    assert!(rule.is_high_risk("network_connect"));
    assert!(rule.is_high_risk("file_delete"));
    assert!(
        !rule.is_high_risk("hello_world"),
        "hello_world 不以高风险前缀开始"
    );
    // 大小写不敏感
    assert!(rule.is_high_risk("SYSTEM_exec"));
}

#[test]
fn frequency_rule_within_window_triggers_deny() {
    // FrequencyRule 1min/3 次: 当前 call + 2 历史 → 第 3 次触发 Deny
    use apeireth_tool_approval::decision::ApprovalDecision;
    use apeireth_tool_approval::history::CallRecord;
    use apeireth_tool_runtime::parser::ToolCallParser;
    let rule = FrequencyRule::with_limits(60_000, 3);
    // 模拟 2 次历史 (CallRecord::new 自动用 now_ms(), 在 1min 窗口内)
    let calls = ToolCallParser::parse(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<spam>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    )
    .expect("parse");
    let call = &calls[0];
    let history = vec![
        CallRecord::new(call, ApprovalDecision::Allow, Some("trust".to_string())),
        CallRecord::new(call, ApprovalDecision::Allow, Some("trust".to_string())),
    ];
    // 当前是第 3 次 → 触发
    let d = rule.check(call, &history);
    match d {
        ApprovalDecision::Deny { reason, silent } => {
            assert!(
                reason.contains("频率") || reason.contains("阈值"),
                "reason 应含频率说明: {reason}"
            );
            assert!(!silent, "frequency deny 应非静默");
        }
        other => panic!("应 Deny, 实际: {other:?}"),
    }
}

#[test]
fn frequency_rule_empty_history_no_match() {
    // 0 历史 → 第 1 次调用, NoMatch
    let rule = FrequencyRule::with_limits(60_000, 3);
    let call = parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>");
    assert!(matches!(rule.check(&call, &[]), ApprovalDecision::NoMatch));
}

#[test]
fn frequency_rule_different_tool_no_match() {
    // 历史是 tool A, 当前 call tool B → 不同工具不计数
    use apeireth_tool_approval::decision::ApprovalDecision;
    use apeireth_tool_approval::history::CallRecord;
    use apeireth_tool_runtime::parser::ToolCallParser;
    let rule = FrequencyRule::with_limits(60_000, 3);
    let calls_a =
        ToolCallParser::parse("<<<[TOOL_REQUEST]>>>\ntool_name:<<<A>>>\n<<<[END_TOOL_REQUEST]>>>")
            .expect("parse");
    let calls_b =
        ToolCallParser::parse("<<<[TOOL_REQUEST]>>>\ntool_name:<<<B>>>\n<<<[END_TOOL_REQUEST]>>>")
            .expect("parse");
    let history = vec![
        CallRecord::new(
            &calls_a[0],
            ApprovalDecision::Allow,
            Some("trust".to_string()),
        ),
        CallRecord::new(
            &calls_a[0],
            ApprovalDecision::Allow,
            Some("trust".to_string()),
        ),
    ];
    // 当前是 B, 历史都是 A → 不应触发
    let d = rule.check(&calls_b[0], &history);
    assert!(
        matches!(d, ApprovalDecision::NoMatch),
        "不同工具应 NoMatch, 实际: {d:?}"
    );
}

// =====================================================================
// fuzzy_bridge 集成 (VCP §6.2.2 #18)
// =====================================================================

#[test]
fn fuzzy_bridge_resolves_typo_to_registered_tool() {
    use apeireth_tool_approval::fuzzy_bridge::match_tool_name;
    use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
    use std::sync::Arc;
    let r = ToolRegistry::new();
    r.register(
        "FileOperator".to_string(),
        Arc::new(MockSyncTool {
            name: "FileOperator".to_string(),
        }),
    );
    // LLM 拼错 "FileOperater" (a↔e) → fuzzy 命中 FileOperator
    let m = match_tool_name("FileOperater", &r);
    assert_eq!(m, Some("FileOperator".to_string()));
}

#[test]
fn fuzzy_bridge_returns_none_for_completely_different() {
    use apeireth_tool_approval::fuzzy_bridge::match_tool_name;
    use apeireth_tool_registry::ToolRegistry;
    let r = ToolRegistry::new();
    assert!(match_tool_name("XyzNotInRegistry", &r).is_none());
}

#[test]
fn fuzzy_bridge_threshold_one_rejects_distance_two() {
    use apeireth_tool_approval::fuzzy_bridge::match_tool_name_threshold;
    use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
    use std::sync::Arc;
    let r = ToolRegistry::new();
    r.register(
        "read".to_string(),
        Arc::new(MockSyncTool {
            name: "read".to_string(),
        }),
    );
    // "raed" 距离 "read" = 2
    let m = match_tool_name_threshold("raed", &r, 1);
    assert!(m.is_none(), "距离 2 在 threshold=1 应拒识");
    let m2 = match_tool_name_threshold("raed", &r, 2);
    assert_eq!(m2, Some("read".to_string()));
}

// =====================================================================
// Manager 历史累积 (FrequencyRule 真实使用场景)
// =====================================================================

#[test]
fn manager_records_decision_to_history() {
    // ApprovalManager 调 check 时应把 NoMatch 决策也记录到 history
    let mut mgr = ApprovalManager::new();
    mgr.add_rule(Box::new(TrustRule::with_trusted(vec!["T".to_string()])));
    let call =
        parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<T>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>");
    let _ = mgr.check(&call);
    // history 应至少有 1 条
    assert!(
        mgr.history_len() >= 1,
        "history 应记录, got {}",
        mgr.history_len()
    );
}

#[test]
fn manager_with_timeout_zero() {
    // 0 超时也合法 (per ms, 0 = 立即失败, 但代码应不 panic)
    let m = ApprovalManager::new().with_timeout(0);
    assert_eq!(m.approval_timeout_ms(), 0);
}

#[test]
fn black_list_rule_silent_vs_loud() {
    // 静默 vs 非静默 边界
    let silent = BlacklistRule::silent();
    let loud = BlacklistRule::new();
    assert!(silent.is_silent(), "silent() 应静默");
    assert!(!loud.is_silent(), "new() 应非静默");
}

#[test]
fn black_list_rule_with_blacklist_helper() {
    // BlacklistRule::with_blacklist 显式传入 blacklist + silent
    let r = BlacklistRule::with_blacklist(vec!["Banned".to_string()], true);
    let call = parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<Banned>>>\n<<<[END_TOOL_REQUEST]>>>");
    match r.check(&call, &[]) {
        ApprovalDecision::Deny { silent, .. } => assert!(silent),
        other => panic!("应 Deny silent, 实际: {other:?}"),
    }
    // 不同工具 NoMatch
    let call2 = parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<Safe>>>\n<<<[END_TOOL_REQUEST]>>>");
    assert!(matches!(r.check(&call2, &[]), ApprovalDecision::NoMatch));
}
