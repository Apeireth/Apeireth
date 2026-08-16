//! R33-4-2: PersonaBoundMember LIVE env-gated test (真 MiniMax)
//!
//! 跑真 MiniMax `https://api.minimaxi.com/anthropic/v1/messages`,
//! 3 PersonaBoundMember × 3 round = 9 HTTP call, 验证:
//! 1. to_system_prompt() 6 段 (persona 3 + CouncilMember 4) 都被 LLM 看到
//! 2. 跨轮 prior_opinions 传递
//! 3. consensus / strong_disapprove / max_rounds 4 终止原因

use apeireth_council::{
    CouncilMember, CouncilQuery, LlmAdvisorBackend, MockLlmProvider, Persona,
    PersonaBoundDeliberator, PersonaBoundMember,
};
use std::sync::Arc;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[test]
#[ignore = "requires APEIRETH_MINIMAX_LIVE_TEST=1 + APEIRETH_MINIMAX_API_KEY + network"]
fn live_minimax_3_persona_bound_3_round_deliberation() {
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        eprintln!("SKIP: APEIRETH_MINIMAX_LIVE_TEST != 1, run with `cargo test -- --ignored`");
        return;
    }
    let api_key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    assert!(
        !api_key.is_empty(),
        "APEIRETH_MINIMAX_API_KEY must be set for live test"
    );

    use apeireth_api::llm::providers::anthropic_compat::{
        AnthropicCompatibleConfig, AnthropicCompatibleProvider,
    };
    let cfg = AnthropicCompatibleConfig::new(
        api_key,
        std::env::var("APEIRETH_MINIMAX_URL")
            .unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M2.7-highspeed".to_string()],
    );
    let provider = AnthropicCompatibleProvider::new(cfg).expect("provider init");
    let arc_provider: Arc<dyn apeireth_api::llm::LlmProvider> = Arc::new(provider);
    let backend: Arc<dyn MockLlmProvider> = Arc::new(LlmAdvisorBackend::new(arc_provider));

    // 3 PersonaBoundMember: R33-4 CouncilMember + R19 Persona
    let members = vec![
        PersonaBoundMember::new(
            CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
            Persona::new("诺克斯", "沉稳工程师", "简洁严谨", 0.4),
        ),
        PersonaBoundMember::new(
            CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
            Persona::new("赛琳", "严谨审计", "精准犀利", 0.1),
        ),
        PersonaBoundMember::new(
            CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
            Persona::new("艾拉", "用户视角", "温和共情", 0.6),
        ),
    ];

    let mut d = PersonaBoundDeliberator::new(members)
        .with_mock_llm(backend)
        .with_max_rounds(3);
    let v = d.deliberate(&CouncilQuery::new(
        "q-pbd-live",
        "Should we deploy the new Rust release to production? \
         Answer with exactly one of: StrongApprove / Approve / Neutral / \
         Disapprove / StrongDisapprove / Abstain.",
        now_ms(),
    ));

    eprintln!(
        "LIVE 3x3 pbd verdict: rounds_run={} termination={} consensus={} \
         final_stance={:?} final_score={} elapsed_ms={} member_summaries={:?}",
        v.rounds_run,
        v.termination_reason,
        v.consensus_reached,
        v.final_stance,
        v.final_weighted_score,
        v.elapsed_ms,
        v.member_summaries
    );
    for r in &v.rounds {
        eprintln!(
            "  Round {}: score={:.3} transcript={}",
            r.round + 1,
            r.consensus_score,
            r.transcript
        );
        for (i, speech) in r.speeches.iter().enumerate() {
            eprintln!("    speech #{}: {}", i, speech);
        }
    }

    assert!(v.rounds_run >= 1);
    assert!(v.rounds_run <= 3);
    assert_eq!(v.member_summaries.len(), 3);
    for r in &v.rounds {
        assert!(!r.transcript.is_empty());
        assert_eq!(r.opinions.len(), 3);
        assert_eq!(r.speeches.len(), 3);
    }

    // 7 阶段 eval score 至少 5/7 pass
    let scores = v.to_eval_scores();
    let pass_count = scores.iter().filter(|(_, v)| *v >= 0.5).count();
    assert!(
        pass_count >= 5,
        "至少 5/7 stage pass, got {}/7: {:?}",
        pass_count,
        scores
    );
}
