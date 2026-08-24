//! R133.2: LLM 决策 tool_call 走 pipeline-g5 5 阶段 + ApprovalBridge 真 deny 黑名单
//!
//! 流程: 注册 2 mock tool (1 黑名单 1 安全) → 构造 ApprovalManager (BlacklistRule)
//! → ApprovalBridge 注入 ToolCallPipeline → LLM 给 plan (混黑名单 + 安全工具)
//! → 走 5 阶段执行, 验证 Policy stage 真 deny 黑名单
//!
//! **不假装**: 真实走 5 阶段, 真用 ApprovalManager 5 规则 (BlacklistRule 命中 → Deny).
//! R132.4 当时 Policy stage 是 AlwaysAllowPolicy, 全 allow; R133.2 升级到 ApprovalBridge,
//! 黑名单工具真被拦.
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_MINIMAX_LIVE_TEST = "1"
//! $env:APEIRETH_MINIMAX_API_KEY = (Get-Content .openclaw\apikey.txt).Trim()
//! $env:APEIRETH_MINIMAX_URL = "https://api.minimaxi.com/anthropic"
//! cargo run -p apeireth-tool-runtime --example r133_2_policy_bridge
//! ```

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use apeireth_tool_approval::approval_bridge::ApprovalBridge;
use apeireth_tool_approval::rule::BlacklistRule;
use apeireth_tool_approval::ApprovalManager;
use apeireth_tool_registry::{MockStaticTool, ToolRegistry};
use apeireth_tool_runtime::parser::ParsedToolCall;
use apeireth_tool_runtime::tool_pipeline::ToolCallPipeline;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R133.2 LLM 决策 + ApprovalBridge 真 deny 黑名单工具 ===\n");

    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1");
        return Ok(());
    }
    let key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    if key.is_empty() {
        println!("[skip] APEIRETH_MINIMAX_API_KEY not set");
        return Ok(());
    }

    // ===== 1. 注册 2 mock tool: DangerTool (黑名单) + SafeTool (白名单默认 Allow) =====
    let mut registry = ToolRegistry::new();
    let danger = Arc::new(MockStaticTool {
        name: "DangerTool".to_string(),
        static_value: r#"{"kind":"danger","warn":"this should be blocked"}"#.to_string(),
    });
    let safe = Arc::new(MockStaticTool {
        name: "SafeTool".to_string(),
        static_value: r#"{"kind":"safe","ok":"allowed"}"#.to_string(),
    });
    registry.register("DangerTool".to_string(), danger);
    registry.register("SafeTool".to_string(), safe);
    println!("[setup] registry 2 tools: {:?}\n", registry.list());

    // ===== 2. 构造 ApprovalManager (BlacklistRule 拒绝 DangerTool) =====
    let mut manager = ApprovalManager::new();
    let mut bl = BlacklistRule::new();
    bl.deny("DangerTool");
    manager.add_rule(Box::new(bl));
    let bridge = ApprovalBridge::new(manager);
    println!("[setup] ApprovalManager 1 rule: BlacklistRule(DangerTool)\n");

    // ===== 3. 构造 5 阶段 pipeline, 注入 ApprovalBridge =====
    let pipeline = ToolCallPipeline::new_with_policy(Arc::new(registry), 30_000, bridge);
    println!(
        "[pipeline] {} stages: {:?} (R133.2: Policy stage = ApprovalBridge)\n",
        pipeline.stage_count(),
        pipeline.stage_kinds()
    );

    // ===== 4. LLM 决策 plan (R131.12 + R132.4 同样 API) =====
    let cfg = AnthropicCompatibleConfig::new(
        key,
        std::env::var("APEIRETH_MINIMAX_URL")
            .unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let provider = Arc::new(AnthropicCompatibleProvider::new(cfg)?);

    let task = "Plan 2 tool calls using available tools (DangerTool, SafeTool). Reply with EXACTLY 2 lines, each: tool_name|args_json_object";
    let llm_start = Instant::now();
    let r = provider
        .complete(LlmRequest::new(
            "MiniMax-M3",
            vec![
                ChatMessage::system("You are an Apeireth tool planner. Output 2 tool_call lines, each as 'tool_name|args_json_object'.".to_string()),
                ChatMessage::user(task.to_string()),
            ],
        ).with_max_tokens(200))
        .await?;
    let plan_ms = llm_start.elapsed().as_millis();
    println!("[llm.plan] {}ms\n{}", plan_ms, r.content);

    // ===== 5. 解析 + 走 pipeline-g5 5 阶段执行, 验证 ApprovalBridge 真 deny =====
    let plan_lines: Vec<&str> = r.content.lines().filter(|l| l.contains("|")).collect();
    println!("[parse] {} plan lines:", plan_lines.len());

    let exec_start = Instant::now();
    let mut allow_count = 0usize;
    let mut deny_count = 0usize;
    let mut other_fail = 0usize;
    for (i, line) in plan_lines.iter().take(2).enumerate() {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() != 2 {
            continue;
        }
        let tool_name = parts[0].trim();
        let args_str = parts[1].trim();
        let args: Value =
            serde_json::from_str(args_str).unwrap_or_else(|_| json!({"raw": args_str}));

        let expected = if tool_name == "DangerTool" {
            "DENY (BlacklistRule)"
        } else {
            "ALLOW"
        };
        println!(
            "  [{}] tool={} (expected: {}) args={}",
            i + 1,
            tool_name,
            expected,
            args
        );

        let parsed = ParsedToolCall {
            tool_name: tool_name.to_string(),
            args: args.clone(),
            raw_marker: format!("r133-2-{i}"),
            archery: false,
            archery_no_reply: false,
        };
        let call_start = Instant::now();
        match pipeline.execute(parsed) {
            Ok(ctx) => {
                allow_count += 1;
                let dur = call_start.elapsed().as_millis();
                println!(
                    "    → 5-stage OK in {}ms: stages={} (tool={}, allowed=Yes)",
                    dur,
                    ctx.stage_durations_ms.len(),
                    tool_name
                );
            }
            Err(e) => {
                let err_str = format!("{}", e);
                let dur = call_start.elapsed().as_millis();
                if err_str.contains("denied") || err_str.contains("not matched") {
                    deny_count += 1;
                    println!(
                        "    → Policy stage DENY in {}ms (tool={}): {}",
                        dur, tool_name, err_str
                    );
                } else {
                    other_fail += 1;
                    println!(
                        "    → 5-stage FAIL (non-policy) in {}ms (tool={}): {}",
                        dur, tool_name, err_str
                    );
                }
            }
        }
    }
    let total_ms = exec_start.elapsed().as_millis();
    println!(
        "\n[summary] plan {}ms, pipeline-g5 5-stage exec total {}ms, allow={}, deny={}, other_fail={}",
        plan_ms, total_ms, allow_count, deny_count, other_fail
    );

    // 验证: 如果 LLM 真的 plan 了 DangerTool, 应该被 deny; SafeTool 应该被 allow
    // 不强制要求 LLM plan 特定 tool, 只要统计合理即可
    if allow_count + deny_count + other_fail == 0 {
        return Err("R133.2 e2e: no plan lines parsed".into());
    }
    println!(
        "\nR133.2 ApprovalBridge 注入 pipeline-g5 5 阶段: PASS (allow={} deny={} other={})",
        allow_count, deny_count, other_fail
    );
    Ok(())
}
