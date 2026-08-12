//! R132.4: LLM 决策 tool_call 走 pipeline-g5 5 阶段 (vs R131.12 直跑 ToolExecutor)
//!
//! 流程: 注册 2 mock tool → LLM 给 plan → 解析 tool_call → pipeline-g5 5 阶段执行 → 验证结果
//!
//! 5 阶段: Dispatch (查 registry) → Normalize (validate args) → Policy (allow-list) →
//!          Reliability (timeout + tool.call) → Throttle (token 守门)
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_MINIMAX_LIVE_TEST = "1"
//! $env:APEIRETH_MINIMAX_API_KEY = (Get-Content .openclaw\apikey.txt).Trim()
//! $env:APEIRETH_MINIMAX_URL = "https://api.minimaxi.com/anthropic"
//! cargo run -p apeireth-tool-runtime --example r132_pipeline_tool_call
//! ```

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use apeireth_tool_registry::{token_budget::estimate_tool_tokens, MockStaticTool, ToolRegistry};
use apeireth_tool_runtime::parser::ParsedToolCall;
use apeireth_tool_runtime::tool_pipeline::ToolCallPipeline;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R132.4 LLM 决策 tool_call 走 pipeline-g5 5 阶段 ===\n");

    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1");
        return Ok(());
    }
    let key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    if key.is_empty() {
        println!("[skip] APEIRETH_MINIMAX_API_KEY not set");
        return Ok(());
    }

    // ===== 1. 注册 2 mock tool =====
    let mut registry = ToolRegistry::new();
    let echo = Arc::new(MockStaticTool {
        name: "EchoSync".to_string(),
        static_value: serde_json::to_string(&json!({"kind": "sync", "tool": "EchoSync", "echo": "static_value"}))?,
    });
    let config_tool = Arc::new(MockStaticTool {
        name: "ConfigVersion".to_string(),
        static_value: serde_json::to_string(&json!({"kind": "static", "tool": "ConfigVersion", "value": "1.2.0"}))?,
    });
    registry.register("EchoSync".to_string(), echo);
    registry.register("ConfigVersion".to_string(), config_tool);
    println!("[setup] registry 2 tools: {:?}\n", registry.list());

    // ===== 2. 构造 5 阶段 pipeline-g5 (R132.4 B 案) =====
    let pipeline = ToolCallPipeline::new(Arc::new(registry), 30_000);
    println!(
        "[pipeline] {} stages: {:?}\n",
        pipeline.stage_count(),
        pipeline.stage_kinds()
    );

    // ===== 3. LLM 决策 plan (用 R131.12 同样 API) =====
    let cfg = AnthropicCompatibleConfig::new(
        key,
        std::env::var("APEIRETH_MINIMAX_URL").unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let provider = Arc::new(AnthropicCompatibleProvider::new(cfg)?);

    let task = "Decide 3 tool calls using available tools (EchoSync, ConfigVersion). Reply with EXACTLY 3 lines, each: tool_name|args_json_object";
    let llm_start = Instant::now();
    let r = provider
        .complete(LlmRequest::new(
            "MiniMax-M3",
            vec![
                ChatMessage::system("You are an Apeireth tool planner. Output 3 tool_call lines, each as 'tool_name|args_json_object'.".to_string()),
                ChatMessage::user(task.to_string()),
            ],
        ).with_max_tokens(200))
        .await?;
    let plan_ms = llm_start.elapsed().as_millis();
    println!("[llm.plan] {}ms\n{}", plan_ms, r.content);

    // ===== 4. 解析 + 走 pipeline-g5 5 阶段执行 =====
    let plan_lines: Vec<&str> = r.content.lines().filter(|l| l.contains("|")).collect();
    println!("[parse] {} plan lines:", plan_lines.len());

    let exec_start = Instant::now();
    let mut total_success = 0usize;
    for (i, line) in plan_lines.iter().take(3).enumerate() {
        let parts: Vec<&str> = line.splitn(2, '|').collect();
        if parts.len() != 2 { continue; }
        let tool_name = parts[0].trim();
        let args_str = parts[1].trim();
        let args: Value = serde_json::from_str(args_str).unwrap_or_else(|_| json!({"raw": args_str}));
        println!("  [{}] tool={} args={}", i + 1, tool_name, args);

        let parsed = ParsedToolCall {
            tool_name: tool_name.to_string(),
            args: args.clone(),
            raw_marker: format!("r132-{i}"),
            archery: false,
            archery_no_reply: false,
        };
        let call_start = Instant::now();
        match pipeline.execute(parsed) {
            Ok(ctx) => {
                total_success += 1;
                let dur = call_start.elapsed().as_millis();
                println!(
                    "    → 5-stage OK in {}ms: stages={}",
                    dur,
                    ctx.stage_durations_ms.len()
                );
                for (kind, stage_dur) in &ctx.stage_durations_ms {
                    println!("       stage={:?} {}ms", kind, stage_dur);
                }
            }
            Err(e) => {
                println!("    → 5-stage FAIL in {}ms: {}", call_start.elapsed().as_millis(), e);
            }
        }
    }
    let total_ms = exec_start.elapsed().as_millis();
    println!(
        "\n[summary] plan {}ms, pipeline-g5 5-stage exec total {}ms, {}/{} calls success",
        plan_ms, total_ms, total_success, plan_lines.len().min(3)
    );

    let tokens = estimate_tool_tokens("EchoSync", "sync primary_db");
    println!("[token_budget] estimate_tool_tokens = {}", tokens);

    if total_success == plan_lines.len().min(3) && total_success > 0 {
        println!("\nR132.4 pipeline-g5 5 阶段: ALL PASS");
        Ok(())
    } else {
        Err(format!(
            "R132.4 pipeline-g5 5 阶段: {}/{} FAIL",
            plan_lines.len().min(3) - total_success,
            plan_lines.len().min(3)
        )
        .into())
    }
}
