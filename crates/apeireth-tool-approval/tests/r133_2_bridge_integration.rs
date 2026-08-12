//! **R133.2 — ApprovalBridge 端到端集成测试** (in `apeireth-tool-approval/tests/`).
//!
//! **背景**: tool-runtime dev-dep tool-approval 构成循环 (tool-approval 主 dep tool-runtime).
//!   集成测试放在 tool-approval 侧 (tests/ 是独立 crate, 不算 main), 允许同时用
//!   tool-approval + tool-runtime, 验证 ApprovalBridge 注入 `ToolCallPipeline::new_with_policy`
//!   后 BlacklistRule 真 deny 黑名单工具.
//!
//! **覆盖**:
//! 1. `approval_bridge_denies_blacklist_in_pipeline` — BlacklistRule 真让 Echo 5 阶段 fail
//! 2. `approval_bridge_allows_whitelist_in_pipeline` — WhitelistRule 让 Echo 5 阶段走通
//! 3. `approval_bridge_default_allow_in_pipeline` — 5 规则 NoMatch → manager 默认 Allow
//!
//! **跑法**: `cargo test -p apeireth-tool-approval --test r133_2_bridge_integration`

use apeireth_tool_approval::approval_bridge::ApprovalBridge;
use apeireth_tool_approval::rule::{BlacklistRule, WhitelistRule};
use apeireth_tool_approval::ApprovalManager;
use apeireth_tool_registry::{MockStaticTool, ToolRegistry};
use apeireth_tool_runtime::parser::ParsedToolCall;
use apeireth_tool_runtime::tool_pipeline::ToolCallPipeline;
use serde_json::json;
use std::sync::Arc;

fn make_registry() -> Arc<ToolRegistry> {
    let mut r = ToolRegistry::new();
    let echo = Arc::new(MockStaticTool {
        name: "Echo".to_string(),
        static_value: r#"{"echo":"hello"}"#.to_string(),
    });
    r.register("Echo".to_string(), echo);
    Arc::new(r)
}

fn make_call(tool: &str, args: serde_json::Value) -> ParsedToolCall {
    ParsedToolCall {
        tool_name: tool.to_string(),
        args,
        raw_marker: format!("{tool}|{{}}"),
        archery: false,
        archery_no_reply: false,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn approval_bridge_denies_blacklist_in_pipeline() {
    // 1. manager: BlacklistRule 把 Echo 加入黑名单
    let mut manager = ApprovalManager::new();
    let mut bl = BlacklistRule::new();
    bl.deny("Echo");
    manager.add_rule(Box::new(bl));

    // 2. bridge + pipeline
    let bridge = ApprovalBridge::new(manager);
    let p = ToolCallPipeline::new_with_policy(make_registry(), 5000, bridge);

    // 3. 调 Echo → 期望 Policy stage 拒 (PipelineError::PolicyDenied)
    let call = make_call("Echo", json!({"x": 1}));
    let result = p.execute(call);
    assert!(result.is_err(), "BlacklistRule must deny Echo at Policy stage");
    let err_str = format!("{}", result.unwrap_err());
    assert!(
        err_str.contains("Echo") || err_str.contains("denied") || err_str.contains("黑名单"),
        "error should mention Echo or deny: {}",
        err_str
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn approval_bridge_allows_whitelist_in_pipeline() {
    let mut manager = ApprovalManager::new();
    let mut wl = WhitelistRule::new();
    wl.allow("Echo");
    manager.add_rule(Box::new(wl));

    let bridge = ApprovalBridge::new(manager);
    let p = ToolCallPipeline::new_with_policy(make_registry(), 5000, bridge);

    let call = make_call("Echo", json!({"x": 1}));
    let ctx = p.execute(call).expect("WhitelistRule must allow Echo");
    assert!(ctx.is_success());
    assert!(ctx.approved);
    assert_eq!(ctx.stage_durations_ms.len(), 5);
}

#[tokio::test(flavor = "multi_thread")]
async fn approval_bridge_default_allow_in_pipeline() {
    // 5 规则 NoMatch → manager 默认 Allow (VCP 行为)
    let mut manager = ApprovalManager::new();
    let mut wl = WhitelistRule::new();
    wl.allow("OtherTool"); // Echo 不在白名单
    manager.add_rule(Box::new(wl));

    let bridge = ApprovalBridge::new(manager);
    let p = ToolCallPipeline::new_with_policy(make_registry(), 5000, bridge);

    let call = make_call("Echo", json!({"x": 1}));
    let ctx = p.execute(call).expect("default Allow must pass 5 stages");
    assert!(ctx.is_success());
}
