//! R131.12 LLM 决策 tool_call 端到端

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use apeireth_tool_registry::{
token_budget::estimate_tool_tokens, MockStaticTool, ToolRegistry,
};
use apeireth_tool_runtime::executor::ToolExecutor;
use apeireth_tool_runtime::parser::ParsedToolCall;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R131.12 LLM 决策 tool_call 端到端 ===\n");

    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1");
        return Ok(());
    }
    let key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    if key.is_empty() {
        println!("[skip] APEIRETH_MINIMAX_API_KEY not set");
        return Ok(());
    }

    // ===== 1. 注册 2 个工具 =====
    let mut registry = ToolRegistry::new();
    let echo = Arc::new(MockStaticTool {
        name: "EchoSync".to_string(),
        static_value: serde_json::to_string(&json!({"kind": "sync", "tool": "EchoSync", "echo": "static_value"}))?,
    });
    let config = Arc::new(MockStaticTool {
        name: "ConfigVersion".to_string(),
        static_value: serde_json::to_string(&json!({"kind": "static", "tool": "ConfigVersion", "value": "1.2.0"}))?,
    });
    registry.register("EchoSync".to_string(), echo);
    registry.register("ConfigVersion".to_string(), config);
    println!("[setup] registry 2 tools: {:?}\n", registry.list());

    let executor = ToolExecutor::new(Arc::new(registry));
    println!("[executor] timeout = {}ms\n", executor.timeout_ms());

    // ===== 2. LLM 决策 plan =====
    let cfg = AnthropicCompatibleConfig::new(
        key,
        std::env::var("APEIRETH_MINIMAX_URL").unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let provider = Arc::new(AnthropicCompatibleProvider::new(cfg)?);

    let task = "Decide 3 tool calls using available tools (EchoSync, ConfigVersion). Reply with EXACTLY 3 lines, each: tool_name|args_json_object";
    let t0 = Instant::now();
    let r = provider
        .complete(LlmRequest::new(
            "MiniMax-M3",
            vec![
                ChatMessage::system("You are an Apeireth tool planner. Output 3 tool_call lines, each as 'tool_name|args_json_object'.".to_string()),
                ChatMessage::user(task.to_string()),
            ],
        ).with_max_tokens(200))
        .await?;
    let plan_ms = t0.elapsed().as_millis();
    println!("[llm.plan] {}ms\n{}", plan_ms, r.content);

    // ===== 3. 解析 + 执行 =====
    let plan_lines: Vec<&str> = r.content.lines().filter(|l| l.contains("|")).collect();
    println!("[parse] {} plan lines:", plan_lines.len());

    let session_start = Instant::now();
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
            raw_marker: format!("r131-{i}"),
            archery: false,
            archery_no_reply: false,
        };
        let t0 = Instant::now();
        let result = executor.execute(&parsed).await;
        let exec_ms = t0.elapsed().as_millis();
        println!("    → success={} duration={}ms output={}", result.success, exec_ms, result.output);
        if result.success { total_success += 1; }
    }
    let total_ms = session_start.elapsed().as_millis();
    println!("\n[summary] plan {}ms, execute total {}ms, {}/{} calls success", plan_ms, total_ms, total_success, plan_lines.len().min(3));

    let tokens = estimate_tool_tokens("EchoSync", "同步 echo 输入");
    println!("[token_budget] estimate_tool_tokens = {}", tokens);

    Ok(())
}
