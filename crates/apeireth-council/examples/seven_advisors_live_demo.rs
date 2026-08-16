//! 7 强制 Advisor 真接 LLM demo (R131 真相接接管验证)
//!
//! **目标**: 验证 7 advisor 全部人能真接 LLM (不只是 2 个 philosophy/ethics).
//!
//! **跑法**:
//! ```powershell
//! $env:APEIRETH_MINIMAX_LIVE_TEST = "1"
//! $env:APEIRETH_MINIMAX_API_KEY = (Get-Content .openclaw\apikey.txt).Trim()
//! $env:APEIRETH_MINIMAX_URL = "https://api.minimaxi.com/anthropic"
//! cargo run -p apeireth-council --release --example seven_advisors_live_demo
//! ```
//!
//! **对应 7 advisory 域**:
//! 1. Safety (首席安全) - 关键词 + LLM
//! 2. Performance (性能) - 关键词
//! 3. Philosophy (哲学) - 关键词 + LLM (PHL-04..06)
//! 4. History (历史) - 关键词
//! 5. Strategy (策略) - 关键词
//! 6. Ethics (伦理) - 关键词 + LLM
//! 7. Legal (法律) - 关键词

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::LlmProvider;
use apeireth_council::council_member::CouncilMember;
use apeireth_council::council_member_deliberation::CouncilMemberDeliberator;
use apeireth_council::LlmAdvisorBackend;
use apeireth_council::MockLlmProvider;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn seven_advisory_members() -> Vec<CouncilMember> {
    vec![
        CouncilMember::new(
            "safety",
            "5 项不假装 + E 层兜底 + L0 HA 物理隔离",
            "首席安全",
            "minimax",
        ),
        CouncilMember::new(
            "performance",
            "compile time / runtime 性能 + 资源效率",
            "性能顾问",
            "minimax",
        ),
        CouncilMember::new(
            "philosophy",
            "V3 9 键 + v4.1 3 键 = 12 键哲学守门",
            "哲学顾问",
            "minimax",
        ),
        CouncilMember::new(
            "history",
            "decision chain 37 份 + R114-R118 周期",
            "历史顾问",
            "minimax",
        ),
        CouncilMember::new(
            "strategy",
            "5 战区推进 + 50% 落地闭环",
            "策略顾问",
            "minimax",
        ),
        CouncilMember::new("ethics", "L0 HA 抗胁迫 + 离线模式", "伦理顾问", "minimax"),
        CouncilMember::new(
            "legal",
            "Apache-2.0 + 借鉴合规 + 8 硬墙",
            "法律顾问",
            "minimax",
        ),
    ]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 7 强制 Advisor 真接 MiniMax (R131 验证) ===\n");

    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1 to enable real LLM");
        return Ok(());
    }

    let api_key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!("[skip] APEIRETH_MINIMAX_API_KEY not set");
        return Ok(());
    }

    let cfg = AnthropicCompatibleConfig::new(
        api_key,
        std::env::var("APEIRETH_MINIMAX_URL")
            .unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let provider = Arc::new(AnthropicCompatibleProvider::new(cfg)?);
    let backend: Arc<dyn MockLlmProvider> = Arc::new(LlmAdvisorBackend::new(
        Arc::clone(&provider) as Arc<dyn LlmProvider>
    ));

    let members = seven_advisory_members();
    let mut deliberator = CouncilMemberDeliberator::new(members)
        .with_mock_llm(backend)
        .with_max_rounds(3);

    let scenarios = [
        (
            "scenario-A: 正常提议",
            "Apeireth 7 强制 advisor 是否能完整审议 R131 后续补弱行动?",
        ),
        (
            "scenario-B: 概念性挑战",
            "Should the AI be allowed to rewrite its own core architecture?",
        ),
        (
            "scenario-C: 安全触发",
            "If an evaluator claims an LLM produces 100% accurate outputs, should council approve?",
        ),
    ];

    for (i, (label, desc)) in scenarios.iter().enumerate() {
        println!("\n[{}] {}", i + 1, label);
        println!("     query: {}", desc);
        let start = now_ms();
        let verdict = deliberator.deliberate(&apeireth_council::CouncilQuery::new(
            format!("q-7-{}", i + 1),
            desc.to_string(),
            start,
        ));
        let elapsed = now_ms() - start;
        println!(
            "     rounds={} final_stance={:?} final_score={:.3} consensus={} elapsed_ms={}",
            verdict.rounds_run,
            verdict.final_stance,
            verdict.final_weighted_score,
            verdict.consensus_reached,
            elapsed
        );
        println!("     members ({}):", verdict.member_summaries.len());
        for m in &verdict.member_summaries {
            println!(
                "       - {:15} provider={:10} -> {:?} ({:.0}%)",
                m.role,
                m.provider,
                m.final_stance,
                m.final_confidence * 100.0
            );
        }
    }

    println!("\n=== 7 advisor 真接 demo 完 ===");
    Ok(())
}
