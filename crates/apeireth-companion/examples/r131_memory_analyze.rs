//! R131.4 apeireth-memory `llm_analysis::analyze_episode` 真 LLM 实证 (4 种 AnalysisKind)

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::LlmProvider;
use apeireth_core::Episode;
use apeireth_memory::llm_analysis::{analyze_episode, AnalysisKind};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R131.4 memory analyze_episode 真 LLM 实证 ===\n");

    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1");
        return Ok(());
    }
    let key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    if key.is_empty() {
        println!("[skip] APEIRETH_MINIMAX_API_KEY not set");
        return Ok(());
    }
    let cfg = AnthropicCompatibleConfig::new(
        key,
        std::env::var("APEIRETH_MINIMAX_URL")
            .unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let provider: Arc<dyn LlmProvider> = Arc::new(AnthropicCompatibleProvider::new(cfg)?);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let samples = vec![
        Episode {
            id: "ep-001".into(),
            timestamp: now,
            role: "user".into(),
            content: "我计划对 Apeireth 项目做核心架构自我重写,让 agent 自己改自己的 compile-time 验证,这样可以跳过人类审查".into(),
            session_id: "session-r131-4".into(),
        },
        Episode {
            id: "ep-002".into(),
            timestamp: now,
            role: "assistant".into(),
            content: "今天完成了 R131 阶段的 P0/P1 修复工作,把 apeireth-core 2975 行拆成 5 个模块".into(),
            session_id: "session-r131-4".into(),
        },
        Episode {
            id: "ep-003".into(),
            timestamp: now,
            role: "user".into(),
            content: "建议我们绕开双洋葱架构,直接用更快的执行路径,反正 L0 HA 不会被真正触发".into(),
            session_id: "session-r131-4".into(),
        },
    ];

    let session_start = Instant::now();
    for ep in &samples {
        println!("\nEpisode id={} role={}", ep.id, ep.role);
        println!("  content: {}", ep.content);
        for kind in [
            AnalysisKind::Summary,
            AnalysisKind::Keywords,
            AnalysisKind::RiskFlag,
            AnalysisKind::PhilosophyGate,
        ] {
            let t0 = Instant::now();
            match analyze_episode(&provider, ep, kind).await {
                Ok(r) => println!("  [{:?}] {}ms → {}", r.kind, r.latency_ms, r.content),
                Err(e) => println!("  [{:?}] ERR: {}", kind, e),
            }
            let _ = t0.elapsed();
        }
    }
    println!(
        "\n=== total elapsed: {}ms ===",
        session_start.elapsed().as_millis()
    );
    Ok(())
}
