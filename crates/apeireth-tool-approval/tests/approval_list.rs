//! 集成测试: toolApprovalManager 增强 P1 (命令级粒度 + 静默拒绝 + 结构化拒绝)
//!
//! 端到端路径: LLM 输出解析 → ApprovalListRule 命令级匹配 → 审批通道 →
//! 结构化结果 {rejected_by_user, error_type} + 审计留痕.

use std::sync::Arc;

use apeireth_tool_approval::manager::ApprovalHandler;
use apeireth_tool_approval::{
    extract_commands, parse_approval_entry, ApprovalDecision, ApprovalListRule, ApprovalManager,
    ApprovalOutcome, ApprovalRule, BlacklistRule, RejectErrorType, RiskRule, TrustRule,
};
use apeireth_tool_runtime::parser::ToolCallParser;
use apeireth_tool_runtime::ParsedToolCall;
use async_trait::async_trait;

fn parse_one(output: &str) -> ParsedToolCall {
    let calls = ToolCallParser::parse(output).expect("parse");
    assert_eq!(calls.len(), 1);
    calls.into_iter().next().unwrap()
}

/// 主人批准 handler (带理由, 批准时理由不入结果, VCP 协议)
struct MasterApprove;
#[async_trait]
impl ApprovalHandler for MasterApprove {
    async fn handle(&self, _call: &ParsedToolCall) -> bool {
        true
    }
}

/// 主人拒绝 handler (带理由)
struct MasterReject(&'static str);
#[async_trait]
impl ApprovalHandler for MasterReject {
    async fn handle(&self, _call: &ParsedToolCall) -> bool {
        false
    }
    async fn handle_with_reason(&self, _call: &ParsedToolCall) -> (bool, Option<String>) {
        (false, Some(self.0.to_string()))
    }
}

// =====================================================================
// ① 命令级粒度: tool:command 审批键 (从真解析的 ParsedToolCall 走)
// =====================================================================

#[test]
fn command_level_entry_parses_and_matches() {
    // 真 LLM 输出解析: arg 键值对进 args
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<FileOperator>>>\ncommand:<<<delete>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    let commands = extract_commands(&call.args);
    assert!(
        commands.contains(&"delete".to_string()),
        "解析后应提取到 command, 实际: {commands:?}"
    );

    let rule = ApprovalListRule::with_entries(["FileOperator:delete".to_string()], 300_000);
    let d = rule.check(&call, &[]);
    assert!(d.is_require_approval(), "命令级命中应需审批, 实际: {d:?}");
    assert_eq!(rule.matched_command(&call), Some("delete".to_string()));
}

#[test]
fn command_level_does_not_match_other_commands() {
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<FileOperator>>>\ncommand:<<<read>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    let rule = ApprovalListRule::with_entries(["FileOperator:delete".to_string()], 300_000);
    assert!(
        rule.check(&call, &[]).is_no_match(),
        "read 命令不应命中 delete 审批键"
    );
}

#[test]
fn tool_level_entry_still_matches_without_command() {
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<PowerShellExecutor>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    let rule = ApprovalListRule::with_entries(["PowerShellExecutor".to_string()], 300_000);
    assert!(rule.check(&call, &[]).is_require_approval());
    assert_eq!(rule.matched_command(&call), None, "工具级命中无命令");
}

// =====================================================================
// ② 静默拒绝: ::SilentReject 命中被拒 → silent=true, 审计留痕
// =====================================================================

#[tokio::test]
async fn silent_reject_full_flow_audit_trail() {
    let mut mgr = ApprovalManager::new();
    mgr.add_rule(Box::new(ApprovalListRule::with_entries(
        ["Shell:reboot::SilentReject".to_string()],
        300_000,
    )));
    mgr.set_handler(Arc::new(MasterReject("危险命令")));

    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<Shell>>>\ncommand:<<<reboot>>>\n<<<[END_TOOL_REQUEST]>>>",
    );

    // 决策器: RequireApproval (高危仍走主人批准通道)
    let (decision, detail) = mgr.check_detailed(&call);
    assert!(decision.is_require_approval());
    assert!(detail.silent_on_reject, "::SilentReject 应标记静默");
    assert_eq!(detail.matched_command.as_deref(), Some("reboot"));

    // 审批通道: 主人拒绝 → 结构化拒绝 + silent
    let outcome = mgr.wait_for_approval_outcome(&call).await;
    assert!(outcome.is_silent_rejection());
    let r = outcome.rejection().unwrap();
    assert!(r.rejected_by_user);
    assert_eq!(r.error_type, RejectErrorType::RejectedByUser);
    assert!(r.silent);

    // 审计留痕: 静默拒绝可查 (不打扰 AI ≠ 无痕)
    let audit = mgr.silent_rejection_audit();
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].tool_name, "Shell");
    assert_eq!(audit[0].matched_command.as_deref(), Some("reboot"));
}

// =====================================================================
// ③ 结构化拒绝: {rejected_by_user, error_type} 各路径错误码
// =====================================================================

#[tokio::test]
async fn structured_error_codes_all_paths() {
    // 路径 1: 主人批准 → Approved
    let mut mgr = ApprovalManager::new();
    mgr.add_rule(Box::new(RiskRule::new(300_000)));
    mgr.set_handler(Arc::new(MasterApprove));
    let o = mgr
        .wait_for_approval_outcome(&parse_one(
            "<<<[TOOL_REQUEST]>>>\ntool_name:<<<system.shutdown>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
        ))
        .await;
    assert!(o.is_approved(), "主人批准应 Approved, 实际: {o:?}");

    // 路径 2: 主人拒绝 → rejected_by_user=true
    let mut mgr2 = ApprovalManager::new();
    mgr2.add_rule(Box::new(RiskRule::new(300_000)));
    mgr2.set_handler(Arc::new(MasterReject("不批")));
    let o2 = mgr2
        .wait_for_approval_outcome(&parse_one(
            "<<<[TOOL_REQUEST]>>>\ntool_name:<<<system.shutdown>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
        ))
        .await;
    let r2 = o2.rejection().expect("主人拒绝");
    assert!(r2.rejected_by_user);
    assert_eq!(r2.error_type, RejectErrorType::RejectedByUser);

    // 路径 3: 规则直接拒 (黑名单) → PolicyDeny, rejected_by_user=false
    let mut mgr3 = ApprovalManager::new();
    mgr3.add_rule(Box::new(BlacklistRule::with_blacklist(
        ["Evil".to_string()],
        false,
    )));
    mgr3.set_handler(Arc::new(MasterApprove));
    let o3 = mgr3
        .wait_for_approval_outcome(&parse_one(
            "<<<[TOOL_REQUEST]>>>\ntool_name:<<<Evil>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
        ))
        .await;
    let r3 = o3.rejection().expect("黑名单拒绝");
    assert!(!r3.rejected_by_user);
    assert_eq!(r3.error_type, RejectErrorType::PolicyDeny);

    // 路径 4: 超时 → ApprovalTimeout
    struct SlowApprove;
    #[async_trait]
    impl ApprovalHandler for SlowApprove {
        async fn handle(&self, _call: &ParsedToolCall) -> bool {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            true
        }
    }
    let mut mgr4 = ApprovalManager::new();
    mgr4.add_rule(Box::new(RiskRule::new(15)));
    mgr4.set_handler(Arc::new(SlowApprove));
    let o4 = mgr4
        .wait_for_approval_outcome(&parse_one(
            "<<<[TOOL_REQUEST]>>>\ntool_name:<<<system.exec>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
        ))
        .await;
    let r4 = o4.rejection().expect("超时拒绝");
    assert!(!r4.rejected_by_user);
    assert_eq!(r4.error_type, RejectErrorType::ApprovalTimeout);

    // 路径 5: 无 handler → ChannelUnavailable
    let mut mgr5 = ApprovalManager::new();
    mgr5.add_rule(Box::new(RiskRule::new(300_000)));
    let o5 = mgr5
        .wait_for_approval_outcome(&parse_one(
            "<<<[TOOL_REQUEST]>>>\ntool_name:<<<system.exec>>>\narg:<<<x>>>\n<<<[END_TOOL_REQUEST]>>>",
        ))
        .await;
    let r5 = o5.rejection().expect("无通道拒绝");
    assert_eq!(r5.error_type, RejectErrorType::ChannelUnavailable);
}

// =====================================================================
// ④ 洋葱安全不破: 高危仍 RequireApproval → 主人批准; 信任/白名单不绕过审批清单
// =====================================================================

#[tokio::test]
async fn onion_safety_high_risk_still_requires_master() {
    // ApprovalListRule 命中高危命令 → 必须走通道, AI 无法自行放行
    let mut mgr = ApprovalManager::new();
    mgr.add_rule(Box::new(ApprovalListRule::with_entries(
        ["Shell:rm -rf".to_string()],
        300_000,
    )));
    // 无 handler = 主人通道不可用 → 拒绝 (fail-safe, 绝不放行)
    let call = parse_one(
        "<<<[TOOL_REQUEST]>>>\ntool_name:<<<Shell>>>\ncommand:<<<rm -rf>>>\n<<<[END_TOOL_REQUEST]>>>",
    );
    let outcome = mgr.wait_for_approval_outcome(&call).await;
    assert!(outcome.is_rejected(), "无主人通道时高危操作必须拒绝");

    // 信任规则在审批清单之前注册也不能绕过 (清单优先于信任: 顺序由装配决定,
    // 此处验证清单在前的典型高危装配)
    let mut mgr2 = ApprovalManager::new();
    mgr2.add_rule(Box::new(ApprovalListRule::with_entries(
        ["Shell:rm -rf".to_string()],
        300_000,
    )));
    mgr2.add_rule(Box::new(TrustRule::with_trusted(["Shell".to_string()])));
    mgr2.set_handler(Arc::new(MasterApprove));
    let o2 = mgr2.wait_for_approval_outcome(&call).await;
    assert!(o2.is_approved(), "主人批准后放行");
    if let ApprovalOutcome::Approved { matched_rule, .. } = o2 {
        assert_eq!(matched_rule.as_deref(), Some("approval_list"));
    }
}

#[test]
fn parse_entry_silent_suffix_field_level_vcp() {
    // 字段级对照 VCP parseApprovalRule: 后缀剥离 + 静默标记
    let p = parse_approval_entry("FileOperator:delete::SilentReject").unwrap();
    assert_eq!(p.base, "FileOperator:delete");
    assert!(p.silent);
    assert!(parse_approval_entry("").is_none());
    assert!(parse_approval_entry("::SilentReject").is_none());
}

#[test]
fn check_signature_backcompat() {
    // 旧 check(&call) -> ApprovalDecision 签名不变 (消费方零改动)
    let mut mgr = ApprovalManager::new();
    mgr.add_rule(Box::new(ApprovalListRule::with_entries(
        ["X".to_string()],
        300_000,
    )));
    let call =
        parse_one("<<<[TOOL_REQUEST]>>>\ntool_name:<<<X>>>\narg:<<<1>>>\n<<<[END_TOOL_REQUEST]>>>");
    let d: ApprovalDecision = mgr.check(&call);
    assert!(d.is_require_approval());
}
