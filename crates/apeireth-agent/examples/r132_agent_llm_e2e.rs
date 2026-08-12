//! R132.6: agent 真接 LLM 跑 tool_call e2e (A 路 #1 5/5 战区)
//!
//! 流程: 注册 1 agent (researcher) → LLM 接收 agent.system_prompt + user message →
//!       LLM 给 tool_call plan → R132.4 ToolCallPipeline 5 阶段执行 → 验证结果
//!
//! 这是 5 战区 #1 (terminal-coding-agent) 第一个真接 LLM 的 e2e, 之前 R131 期间
//! 只跑了 unit test (87 passed), 没有真接 LLM 验证.
//!
//! 跑法:
//! ```powershell
//! $env:APEIRETH_MINIMAX_LIVE_TEST = "1"
//! $env:APEIRETH_MINIMAX_API_KEY = (Get-Content .openclaw\apikey.txt).Trim()
//! $env:APEIRETH_MINIMAX_URL = "https://api.minimaxi.com/anthropic"
//! cargo run -p apeireth-agent --example r132_agent_llm_e2e
//! ```

use apeireth_agent::{Agent, AgentManager};
use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use apeireth_tool_registry::{MockStaticTool, ToolRegistry};
use apeireth_tool_runtime::parser::ParsedToolCall;
use apeireth_tool_runtime::tool_pipeline::ToolCallPipeline;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R132.6 agent 真接 LLM 跑 tool_call e2e ===\n");

    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1");
        return Ok(());
    }
    let key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    if key.is_empty() {
        println!("[skip] APEIRETH_MINIMAX_API_KEY not set");
        return Ok(());
    }

    // ===== 1. 注册 1 agent (researcher) + 2 mock tool =====
    let mut mgr = AgentManager::new();
    let mut registry = ToolRegistry::new();

    let researcher = Agent::new(
        "researcher",
        "Research Agent",
        vec!["researcher".to_string(), "@researcher".to_string()],
        vec!["WebSearch".to_string(), "NoteStore".to_string()],
        "You are a research agent. Use available tools to find information and store findings. Output tool calls as: tool_name|args_json",
    );
    let web_search = Arc::new(MockStaticTool {
        name: "WebSearch".to_string(),
        static_value: serde_json::to_string(&json!({"kind": "search", "tool": "WebSearch", "results": ["doc1", "doc2"]}))?,
    });
    let note_store = Arc::new(MockStaticTool {
        name: "NoteStore".to_string(),
        static_value: serde_json::to_string(&json!({"kind": "store", "tool": "NoteStore", "stored_id": "note-r132-001"}))?,
    });
    registry.register("WebSearch".to_string(), web_search);
    registry.register("NoteStore".to_string(), note_store);
    mgr.register(researcher.clone())?;

    println!("[setup] agent: {} (aliases: {:?})", researcher.id, researcher.aliases);
    println!("[setup] tools: {:?}\n", registry.list());

    // ===== 2. resolve agent via alias =====
    let resolved = mgr.resolve("@researcher").expect("alias should resolve");
    println!("[agent.resolve] @researcher -> {} ({})\n", resolved.id, resolved.name);

    // ===== 3. 构造 5 阶段 pipeline (R132.4) =====
    let pipeline = ToolCallPipeline::new(Arc::new(registry), 30_000);
    println!("[pipeline] 5 stages: {:?}\n", pipeline.stage_kinds());

    // ===== 4. LLM 决策 plan (用 agent.system_prompt) =====
    let cfg = AnthropicCompatibleConfig::new(
        key,
        std::env::var("APEIRETH_MINIMAX_URL").unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let provider = Arc::new(AnthropicCompatibleProvider::new(cfg)?);

    let llm_start = Instant::now();
    let r = provider
        .complete(LlmRequest::new(
            "MiniMax-M3",
            vec![
                ChatMessage::system(resolved.system_prompt.clone()),
                ChatMessage::user("Search for 'Apeireth R132' and store 1 note. Output 2 tool calls as: tool_name|args_json_object".to_string()),
            ],
        ).with_max_tokens(200))
        .await?;
    let plan_ms = llm_start.elapsed().as_millis();
    println!("[llm.plan] {}ms (using agent.system_prompt)\n{}", plan_ms, r.content);

    // ===== 5. 解析 + 走 5 阶段执行 =====
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

        // 验证 tool 在 agent.tools 白名单 (VCP agentManager.js:323 isAgent 守门)
        if !resolved.tools.iter().any(|t| t == tool_name) {
            println!("    → tool '{}' not in agent.tools whitelist, skip", tool_name);
            continue;
        }

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
                println!(
                    "    → 5-stage OK in {}ms: stages={}",
                    call_start.elapsed().as_millis(),
                    ctx.stage_durations_ms.len()
                );
            }
            Err(e) => {
                println!("    → 5-stage FAIL in {}ms: {}", call_start.elapsed().as_millis(), e);
            }
        }
    }
    let total_ms = exec_start.elapsed().as_millis();
    println!(
        "\n[summary] plan {}ms, pipeline 5-stage exec {}ms, {}/{} calls success",
        plan_ms, total_ms, total_success, plan_lines.len().min(3)
    );

    if total_success == plan_lines.len().min(3) && total_success > 0 {
        println!("\nR132.6 agent + LLM + pipeline-g5: ALL PASS (5/5 战区 A 路 #1 完成)");
        Ok(())
    } else {
        Err(format!(
            "R132.6 agent e2e: {}/{} FAIL",
            plan_lines.len().min(3) - total_success,
            plan_lines.len().min(3)
        )
        .into())
    }
}
