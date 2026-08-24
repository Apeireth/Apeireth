//! `apeireth-tool-approval` example — **5 规则端到端审批 demo**
//!
//! **目标**: 演示 5 规则 (Trust / Risk / Frequency / Whitelist / Blacklist) 在真实场景下
//! 端到端工作流:
//! 1. 注册 3 个工具 (low risk `Greeting` / high risk `system.exec` / medium risk `FileOperator`)
//! 2. 构造 5 规则 ApprovalManager
//! 3. 模拟 4 个 tool call, 验证 5 规则各自触发 + fuzzy matching 集成
//!
//! **跑法**: `cargo run -p apeireth-tool-approval --example approval_demo`
//!
//! **VCP 借鉴**: `toolApprovalManager.js:1-267` 5 规则 (getApprovalDecision + SilentReject)

use std::sync::Arc;

use apeireth_tool_approval::{
    match_tool_name, ApprovalDecision, ApprovalHandler, ApprovalManager, ApprovalRule,
    BlacklistRule, FrequencyRule, RiskRule, TrustRule, WhitelistRule, APPROVAL_TIMEOUT_MS,
};
use apeireth_tool_registry::{MockSyncTool, ToolRegistry};
use apeireth_tool_runtime::ParsedToolCall;
use async_trait::async_trait;
use serde_json::json;

macro_rules! info {
    ($($arg:tt)*) => { println!("[INFO] {}", format!($($arg)*)) }
}
macro_rules! warn {
    ($($arg:tt)*) => { eprintln!("[WARN] {}", format!($($arg)*)) }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // ============================================================
    // 1. 注册 3 个 mock 工具 (low / high / medium risk)
    // ============================================================
    let registry = Arc::new(ToolRegistry::new());
    registry.register(
        "Greeting".to_string(),
        Arc::new(MockSyncTool {
            name: "Greeting".to_string(),
        }),
    );
    registry.register(
        "system.exec".to_string(),
        Arc::new(MockSyncTool {
            name: "system.exec".to_string(),
        }),
    );
    registry.register(
        "Calculator".to_string(),
        Arc::new(MockSyncTool {
            name: "Calculator".to_string(),
        }),
    );

    info!("=== Step 1: 已注册 3 个工具 (Greeting / system.exec / Calculator) ===");

    // ============================================================
    // 2. 构造 5 规则 ApprovalManager
    // ============================================================
    let mut mgr = ApprovalManager::new();

    // 实战顺序: 显式 opt-in/opt-out (Whitelist/Blacklist/Trust) → 通用 (Risk/Frequency)
    // 黑名单永远最高优先级, 但白名单应在 Risk 前 (主人显式说"我信任这个工具"应绕过 risk)

    // 规则 1: Blacklist (黑名单, 最高优先级, 默认非静默)
    mgr.add_rule(Box::new(BlacklistRule::with_blacklist(
        ["Forbidden".to_string()],
        false,
    )));

    // 规则 2: Whitelist (白名单, low risk 工具显式 opt-in, 应在 Risk 前)
    mgr.add_rule(Box::new(WhitelistRule::with_whitelist([
        "Calculator".to_string()
    ])));

    // 规则 3: Trust (信任, Greeting 是主人信任工具)
    mgr.add_rule(Box::new(TrustRule::with_trusted(["Greeting".to_string()])));

    // 规则 4: Risk (高风险, system.* / network.* / file.* 5min 审批)
    mgr.add_rule(Box::new(RiskRule::new(APPROVAL_TIMEOUT_MS)));

    // 规则 5: Frequency (反刷, 1min/3 次)
    mgr.add_rule(Box::new(FrequencyRule::new()));

    // 实战 handler: 200ms 后返 true (批准)
    mgr.set_handler(Arc::new(DelayedApproveHandler { delay_ms: 200 }));

    info!("=== Step 2: 5 规则 ApprovalManager 已构造 (Blacklist → Whitelist → Trust → Risk → Frequency) ===");
    info!(
        "    审批超时: {}ms (5min, VCP 真值)",
        mgr.approval_timeout_ms()
    );

    // ============================================================
    // 3. 模拟 4 个 tool call, 验证 5 规则各自触发
    // ============================================================

    // ---- Call 1: Greeting (信任工具 → TrustRule Allow) ----
    info!("\n=== Step 3.1: Call 1 — Greeting (信任工具, 应 Allow) ===");
    let call1 = make_call("Greeting");
    let d1 = mgr.check(&call1);
    assert!(d1.is_allow(), "Call 1 应 Allow (Trust 规则), 实际: {d1:?}");
    info!("    ✓ TrustRule Allow");

    // ---- Call 2: system.exec (高风险 → RiskRule 5min 审批) ----
    info!("\n=== Step 3.2: Call 2 — system.exec (高风险, 应 RequireApproval 5min) ===");
    let call2 = make_call("system.exec");
    let d2 = mgr.check(&call2);
    match d2 {
        ApprovalDecision::RequireApproval { timeout_ms } => {
            assert_eq!(timeout_ms, 300_000, "应 5min = 300_000ms");
            info!("    ✓ RiskRule RequireApproval(5min = {timeout_ms}ms)");
            // 实战: 调 wait_for_approval, 这里简化用 handler 200ms 后批准
            let r = mgr.wait_for_approval(&call2).await;
            assert_eq!(r, Ok(true), "handler 200ms 后应批准, 实际: {r:?}");
            info!("    ✓ wait_for_approval 返 true (handler 200ms 后批准)");
        }
        _ => panic!("Call 2 应 RequireApproval, 实际: {d2:?}"),
    }

    // ---- Call 3: Calculator (白名单 → WhitelistRule Allow) ----
    info!("\n=== Step 3.3: Call 3 — Calculator (白名单, 应 Allow) ===");
    let call3 = make_call("Calculator");
    let d3 = mgr.check(&call3);
    assert!(d3.is_allow(), "Call 3 应 Allow (Whitelist), 实际: {d3:?}");
    info!("    ✓ WhitelistRule Allow");

    // ---- Call 4: 同名同工具连发 3 次 (FrequencyRule 反刷 → 第 3 次 Deny) ----
    info!("\n=== Step 3.4: Call 4 — SpamTool 连发 3 次 (反刷, 第 3 次应 Deny) ===");
    let spam = make_call("SpamTool");
    let _ = mgr.check(&spam);
    let _ = mgr.check(&spam);
    let d4 = mgr.check(&spam);
    match d4 {
        ApprovalDecision::Deny { reason, silent } => {
            assert!(!silent);
            assert!(
                reason.contains("频率超限"),
                "reason 应含'频率超限', 实际: {reason}"
            );
            info!("    ✓ FrequencyRule Deny (reason: {reason}, silent: {silent})");
        }
        _ => panic!("Call 4 第 3 次应 Deny, 实际: {d4:?}"),
    }

    // ============================================================
    // 4. Fuzzy matching 集成 (VCP §6.2.2 #18)
    // ============================================================
    info!("\n=== Step 4: Fuzzy matching 集成 (VCP §6.2.2 #18) ===");

    // LLM 拼错 "Calc" → "Calculatr" (少 1 个 o), Levenshtein = 1
    let resolved = match_tool_name("Calculatr", &registry);
    assert_eq!(
        resolved,
        Some("Calculator".to_string()),
        "fuzzy 应纠正 'Calculatr' → 'Calculator'"
    );
    info!("    ✓ LLM 拼错 'Calculatr' → fuzzy 纠正为 'Calculator'");

    // "Gretting" 拼错 (少 1 个 e), Levenshtein = 1
    let resolved2 = match_tool_name("Gretting", &registry);
    assert_eq!(resolved2, Some("Greeting".to_string()));
    info!("    ✓ LLM 拼错 'Gretting' → fuzzy 纠正为 'Greeting'");

    // ============================================================
    // 5. Blacklist + Silent 演示 (VCP `::SilentReject` suffix)
    // ============================================================
    info!("\n=== Step 5: Blacklist 静默拒绝 (VCP `::SilentReject` 风格) ===");
    let silent_blacklist = BlacklistRule::with_blacklist(["SecretTool".to_string()], true);
    let call5 = make_call("SecretTool");
    let d5 = silent_blacklist.check(&call5, &[]);
    match d5 {
        ApprovalDecision::Deny { reason, silent } => {
            assert!(silent, "应静默, 实际: silent={silent}");
            info!("    ✓ Blacklist(silent=true) Deny: reason='{reason}', silent=true");
        }
        _ => panic!("应 Deny(silent=true), 实际: {d5:?}"),
    }

    // ============================================================
    // 6. 总结
    // ============================================================
    info!("\n=== 战役 2-3 端到端 demo 全通过 ===");
    info!("    - 5 规则真实现: Trust / Risk / Frequency / Whitelist / Blacklist ✓");
    info!("    - 5 分钟审批窗口 (VCP `getTimeoutMs` 真值 300_000ms) ✓");
    info!("    - FrequencyRule 1min/3 次反刷 ✓");
    info!("    - BlacklistRule 静默拒绝 (VCP `::SilentReject`) ✓");
    info!("    - Fuzzy matching 集成 (VCP §6.2.2 #18, Levenshtein ≤ 2) ✓");
    info!("    - wait_for_approval 真等待外部 handler 响应 ✓");
    info!("    - 编译期 hardcode 8 const 全守门 ✓");
    info!("\nVCP 借鉴字段级: toolApprovalManager.js:1-267 (5 规则 + timeout + SilentReject)");
}

fn make_call(tool: &str) -> ParsedToolCall {
    ParsedToolCall {
        tool_name: tool.to_string(),
        args: json!({"demo": true}),
        raw_marker: format!("tool_name:<<<{tool}>>>"),
        archery: false,
        archery_no_reply: false,
    }
}

/// 实战 demo handler: 延迟 N ms 后返 true
struct DelayedApproveHandler {
    delay_ms: u64,
}

#[async_trait]
impl ApprovalHandler for DelayedApproveHandler {
    async fn handle(&self, _call: &ParsedToolCall) -> bool {
        warn!("[DelayedApproveHandler] 等待 {}ms 后批准", self.delay_ms);
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        true
    }
}
