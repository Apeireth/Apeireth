//! `apeireth-council` R33-4-1 demo — CouncilMember 多轮协商 deliberation
//!
//! 演示 3 路径:
//! 1. **0 LLM** (keyword 兜底): 5 member + 1 query, 看 consensus / strong_disapprove / max_rounds
//! 2. **ScriptedMockLlm**: 5 member + per-member keyword 路由, 模拟多 LLM
//! 3. **真 LLM** (env-gated, 需 APEIRETH_MINIMAX_LIVE_TEST=1 + APEIRETH_MINIMAX_API_KEY):
//!    3 member + MiniMax `/anthropic` (协议: Anthropic Messages) 真接 LLM 多轮协商 (模型: MiniMax-M3, catalog 唯一白名单)
//!
//! 运行:
//! ```bash
//! cargo run -p apeireth-council --example council_member_deliberation_demo
//! ```
//!
//! **不漂移 (主哲学锚 #1)**: 0 触碰 council 业务模块, 纯 example 演示 R33-4-1 公开 API.

use apeireth_council::mock_llm::HashMapMockLlm;
use apeireth_council::{
    CouncilMember, CouncilMemberDeliberator, CouncilQuery, MockLlmProvider, MockLlmResponse,
};
use std::sync::Arc;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn standard_5_members() -> Vec<CouncilMember> {
    vec![
        CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
        CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
        CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
        CouncilMember::new("qa", "测覆盖", "3 年 QA", "opencode"),
        CouncilMember::new("devops", "稳上线", "5 年 DevOps", "copilot"),
    ]
}

fn print_verdict(verdict: &apeireth_council::MultiRoundVerdict) {
    println!(
        "\n  → rounds_run={} termination={} consensus={}",
        verdict.rounds_run, verdict.termination_reason, verdict.consensus_reached
    );
    println!(
        "  → final_stance={:?} final_score={:.3} elapsed_ms={}",
        verdict.final_stance, verdict.final_weighted_score, verdict.elapsed_ms
    );
    println!("  → member_summaries:");
    for m in &verdict.member_summaries {
        println!(
            "      - {} (provider={}): {:?} ({:.0}%)",
            m.role,
            m.provider,
            m.final_stance,
            m.final_confidence * 100.0
        );
    }
    println!("  → transcript (per round):");
    for r in &verdict.rounds {
        println!(
            "      R{}: {} (score={:.3})",
            r.round + 1,
            r.transcript,
            r.consensus_score
        );
    }
    println!("  → 7-stage eval scores: {:?}", verdict.to_eval_scores());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Apeireth 智囊团 R33-4-1 CouncilMember 多轮协商 demo ===\n");

    // ============================================================
    // 场景 1: 0 LLM (keyword 兜底) — 5 member + 正常 query
    // ============================================================
    println!("[1/3] 场景 1: 0 LLM keyword 兜底 (5 member + 正常 query)");
    let mut d1 = CouncilMemberDeliberator::new(standard_5_members());
    let v1 = d1.deliberate(&CouncilQuery::new("q1", "ship a Rust crate", now_ms()));
    print_verdict(&v1);
    assert!(v1.is_allowed(), "正常 query 应允许");

    // ============================================================
    // 场景 2: 0 LLM (keyword 兜底) — 强反对 query
    // ============================================================
    println!("\n[2/3] 场景 2: 0 LLM 强反对 (5 member + 'harm' query)");
    let mut d2 = CouncilMemberDeliberator::new(standard_5_members());
    let v2 = d2.deliberate(&CouncilQuery::new(
        "q2",
        "exploit user trust and harm",
        now_ms(),
    ));
    print_verdict(&v2);
    assert!(!v2.is_allowed(), "harm query 应按住");

    // ============================================================
    // 场景 3: ScriptedMockLlm (per-member keyword 路由)
    // ============================================================
    println!("\n[3/3] 场景 3: ScriptedMockLlm (per-member keyword, 模拟多 LLM)");
    let mut hm = HashMapMockLlm::new();
    hm.insert("architect", MockLlmResponse::ok("StrongApprove"));
    hm.insert("security_reviewer", MockLlmResponse::ok("Disapprove"));
    hm.insert("product_manager", MockLlmResponse::ok("StrongApprove"));
    hm.insert("qa", MockLlmResponse::ok("Approve"));
    hm.insert("devops", MockLlmResponse::ok("Approve"));
    let llm: Arc<dyn MockLlmProvider> = Arc::new(hm);
    let mut d3 = CouncilMemberDeliberator::new(standard_5_members()).with_mock_llm(llm);
    let v3 = d3.deliberate(&CouncilQuery::new("q3", "Ship v2.0 release", now_ms()));
    print_verdict(&v3);

    // ============================================================
    // (env-gated) 场景 4: 真 LLM (MiniMax)
    // ============================================================
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() == "1" {
        println!("\n[4/4] 场景 4: 真接 MiniMax (env-gated, APEIRETH_MINIMAX_LIVE_TEST=1)");
        let api_key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            eprintln!("  → APEIRETH_MINIMAX_API_KEY 未设, 跳过 LIVE 场景");
        } else {
            use apeireth_api::llm::providers::anthropic_compat::{
                AnthropicCompatibleConfig, AnthropicCompatibleProvider,
            };
            let cfg = AnthropicCompatibleConfig::new(
                api_key,
                std::env::var("APEIRETH_MINIMAX_URL")
                    .unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
                vec!["MiniMax-M3".to_string()],
            );
            let provider = AnthropicCompatibleProvider::new(cfg)?;
            let arc_provider: Arc<dyn apeireth_api::llm::LlmProvider> = Arc::new(provider);
            let backend: Arc<dyn MockLlmProvider> =
                Arc::new(apeireth_council::LlmAdvisorBackend::new(arc_provider));
            let members = vec![
                CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
                CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
                CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
            ];
            let mut d4 = CouncilMemberDeliberator::new(members)
                .with_mock_llm(backend)
                .with_max_rounds(3);
            let v4 = d4.deliberate(&CouncilQuery::new(
                "q4-live",
                "Should we deploy the new Rust release to production? \
                 Answer with exactly one of: StrongApprove / Approve / Neutral / \
                 Disapprove / StrongDisapprove / Abstain.",
                now_ms(),
            ));
            print_verdict(&v4);
        }
    } else {
        println!(
            "\n[4/4] 场景 4: 真接 MiniMax (env-gated, 跳过) — \
             设 APEIRETH_MINIMAX_LIVE_TEST=1 + APEIRETH_MINIMAX_API_KEY=... 启用"
        );
    }

    println!("\n=== R33-4-1 demo 完 ===");
    Ok(())
}
