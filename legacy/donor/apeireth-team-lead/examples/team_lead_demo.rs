//! `apeireth-team-lead` 示例: **Team Lead / Orchestrator 端到端 demo**.
//!
//! **本示例演示** (per `v09021-commercial-extract-2026-08-05.md` §3.1 #4 + `supervisor-prompt-818-summary-2026-08-05.md`):
//! 1. 创建 `TeamConfig` (4 Provider fallback 顺序: claude-code → gemini-cli → codex → opencode)
//! 2. 创建 `TeamLead` orchestrator (持有 `AgentManager`)
//! 3. `spawn_agent` 创建 1 个 Researcher Worker + 1 个 Observer (并行 2 子 Agent)
//! 4. `send_to_agent` 发送 task 给 Researcher (await + 不吞错)
//! 5. `wait_agent_idle` 短轮询等待 (per §2.3 supervisor-prompt-818 60-90s 一次)
//! 6. `get_agent_output` 读取输出
//! 7. `cancel_agent` 终止 Observer
//! 8. 验证 `SUPERVISOR_PROMPT` 编译期嵌入 (K-1 强校验: "Claude Code" 字样保留)
//!
//! **运行**:
//! ```bash
//! cargo run --example team_lead_demo
//! ```
//!
//! **期望输出** (stdout):
//! ```text
//! team_name=research-2026-08-05 max_concurrent=8
//! spawn_agent researcher -> agent-1
//! spawn_agent observer   -> agent-2
//! send_to_agent(agent-1, "research Rust async runtime") ok
//! wait_agent_idle(agent-1, 90000ms) ok
//! get_agent_output(agent-1) = ...
//! cancel_agent(agent-2) ok
//! supervisor_prompt_size = <N> bytes
//! K-1 invariant: 'Claude Code' preserved: true
//! ```

#![warn(missing_docs)]

use std::sync::Arc;

use apeireth_agent::AgentManager;
use apeireth_team_lead::{
    build_supervisor_prompt, AgentRole, Message, MessageType, Orchestrator, TeamConfig, TeamLead,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 构造 TeamConfig (per §2.2 supervisor-prompt-818 Provider fallback 顺序)
    let config = TeamConfig {
        team_name: "research-2026-08-05".to_string(),
        max_concurrent: 8,
        // per §2.3 supervisor-prompt-818: codex-based supervisor wait_agent ≤ 90000ms
        default_wait_timeout_ms: 90_000,
        auto_worktree: false, // demo 不开 worktree
        available_providers: vec![
            "claude-code".to_string(),
            "gemini-cli".to_string(),
            "codex".to_string(),
            "opencode".to_string(),
        ],
    };
    println!(
        "team_name={} max_concurrent={}",
        config.team_name, config.max_concurrent
    );

    // 3. 创建 TeamLead (持有 AgentManager 引用)
    let agent_manager = Arc::new(AgentManager::new());
    let lead = TeamLead::new(config, agent_manager);

    // 4. spawn_agent × 2 (并行创建 Researcher + Observer)
    let researcher_id = lead
        .spawn_agent(
            AgentRole::Worker,
            "You are a Rust researcher. Investigate tokio's scheduler.".to_string(),
        )
        .await?;
    println!("spawn_agent researcher -> {researcher_id}");

    let observer_id = lead
        .spawn_agent(
            AgentRole::Observer,
            "You are an observer. Monitor the researcher and report progress.".to_string(),
        )
        .await?;
    println!("spawn_agent observer   -> {observer_id}");

    // 5. send_to_agent (await + 不吞错, per sub-agent 1 architect §6 修复)
    let msg = Message::new(
        None,
        &researcher_id,
        MessageType::Send,
        serde_json::json!({
            "task": "research Rust async runtime",
            "deadline_ms": 60_000,
        }),
    );
    lead.send_to_agent(&researcher_id, msg).await?;
    println!("send_to_agent({researcher_id}, \"research Rust async runtime\") ok");

    // 6. wait_agent_idle 短轮询 (per §2.3 supervisor-prompt-818: 60-90s 一次, 不单次长阻塞)
    lead.wait_agent_idle(&researcher_id, 90_000).await?;
    println!("wait_agent_idle({researcher_id}, 90000ms) ok");

    // 7. get_agent_output
    let output = lead.get_agent_output(&researcher_id).await?;
    println!("get_agent_output({researcher_id}) = {output:?}");

    // 8. cancel_agent 终止 Observer
    lead.cancel_agent(&observer_id).await?;
    println!("cancel_agent({observer_id}) ok");

    // 9. 验证 supervisorPrompt 编译期嵌入 (K-1 强校验 per §5.3 supervisor-prompt-818)
    let prompt = build_supervisor_prompt(&["claude-code", "gemini-cli", "codex", "opencode"]);
    println!("supervisor_prompt_size = {} bytes", prompt.len());
    let k1_claude_code = prompt.contains("Claude Code");
    let k1_claude_dash = prompt.contains("claude-code");
    let k1_spawn = prompt.contains("spawn_agent");
    let k1_wait_idle = prompt.contains("wait_agent_idle");
    println!("K-1 invariant: 'Claude Code' preserved: {k1_claude_code}");
    println!("K-1 invariant: 'claude-code' provider: {k1_claude_dash}");
    println!("K-1 invariant: 'spawn_agent' tool: {k1_spawn}");
    println!("K-1 invariant: 'wait_agent_idle' tool: {k1_wait_idle}");

    // 10. 总结
    let child_count = lead.child_count().await;
    println!("\n=== demo 完成 === child_count={child_count}");

    Ok(())
}
