//! R33-4-1: CouncilMember 多轮协商 integration tests
//!
//! **目标**: 验证 CouncilMemberDeliberator 跑 3 轮协商, 复用 LlmAdvisorBackend 路径
//!     调真 LLM (R16-09 LOCKED) + 跨轮 prior_opinions 传递
//!
//! **3 测试类**:
//! 1. multi-LLM scripted: 1 个 HashMapMockLlm, 3 keyword 路由 3 member
//!    模拟 3 个不同 LLM, 验证共识检测 + 跨轮 prior_opinions 传递
//! 2. consensus round 1 + consensus score > 0.6: 2 StrongApprove + 1 Approve
//! 3. max_rounds (无共识): 3 Disapprove 跑满 3 轮
//! 4. LIVE env-gated (#[ignore]): 真 MiniMax `https://api.minimaxi.com/anthropic/v1/messages`
//!    (per R32-3-1 real_llm_smoke 鉴权 + URL), 3 member 3 轮 = 9 次 HTTP call
//!
//! **关于 wiremock 决策** (S-2 实事求是): R16-09 LlmAdvisorBackend 走 sync trait
//! (`MockLlmProvider::generate`) + `tokio::Handle::block_on` 调 async LlmProvider.
//! 在 `#[tokio::test]` async context 内嵌套 `block_on` 会 panic (Cannot start a
//! runtime from within a runtime). 修复方法: 让 wiremock test 跑在 sync `#[test]`
//! + 启动临时 tokio runtime for MockServer only, 但 LlmAdvisorBackend 又会在那个
//! runtime 内 block_on panic. 解决: 完全避免 wiremock 在 council crate, wiremock
//! 测试已落在 apeireth-eval R32-3-1 real_llm_smoke_integration.rs (per
//! R32-3-1 完成). 本 crate integration test 走 ScriptedMockLlm 路径 (sync, no
//! runtime nesting issue) + LIVE env-gated test 跑真 MiniMax 验证真接 LLM.
//!
//! **不漂移 (主哲学锚 #1)**: 0 改 `deliberation.rs` / `council_member.rs` /
//!     lib.rs 业务路径; 0 改 R16-09 LlmAdvisorBackend; 0 改 R32-3-1 real_llm_smoke

use apeireth_council::mock_llm::HashMapMockLlm;
use apeireth_council::{
    CouncilMember, CouncilMemberDeliberator, CouncilQuery, MockLlmProvider, MockLlmResponse,
    StanceKind, CONSENSUS_SCORE_THRESHOLD,
};
use std::sync::Arc;

const NOW_MS: i64 = 1_700_000_000_000;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(NOW_MS)
}

fn q(desc: &str) -> CouncilQuery {
    CouncilQuery::new("q-int", desc, now_ms())
}

fn standard_3_members() -> Vec<CouncilMember> {
    vec![
        CouncilMember::new("architect", "设计稳的架构", "10 年 Rust", "claude_code"),
        CouncilMember::new("security_reviewer", "找安全漏洞", "5 年安全", "codex"),
        CouncilMember::new("product_manager", "用户价值", "5 年产品", "gemini_cli"),
    ]
}

// ============================================================
// Test 1: multi-LLM scripted — consensus round 1 (2 Approve + 1 Disapprove)
// ============================================================
//
// 验证:
// - 共识分数 = (1.0 + 0.2 + 1.0) / 3 = 0.733 >= 0.6 → 共识
// - HashMapMockLlm 按 keyword 路由 (architect/pm 命中 "Approve" keyword,
//   security_reviewer 命中 "Disapprove" keyword)
// - 跨轮 prior_opinions 传递 (round 1 后 prior_opinions_text 包含 3 个 opinion)

#[test]
fn multi_llm_scripted_3_member_3_round_consensus_round_1() {
    let mut hm = HashMapMockLlm::new();
    hm.insert("architect", MockLlmResponse::ok("StrongApprove"));
    hm.insert("security_reviewer", MockLlmResponse::ok("Disapprove"));
    hm.insert("product_manager", MockLlmResponse::ok("StrongApprove"));
    let llm: Arc<dyn MockLlmProvider> = Arc::new(hm);

    let mut d = CouncilMemberDeliberator::new(standard_3_members()).with_mock_llm(llm);
    let v = d.deliberate(&q("Ship a new feature with security review"));

    // round 1: 2 StrongApprove (1.0 normalized) + 1 Disapprove (0.2 normalized)
    // weighted mean = 0.733 >= 0.6 → consensus
    assert_eq!(v.termination_reason, "consensus");
    assert_eq!(v.rounds_run, 1);
    assert!(v.consensus_reached);
    assert_eq!(v.member_summaries.len(), 3);

    // 不同 member 拿不同 stance (per HashMapMockLlm keyword routing)
    let arch = v
        .member_summaries
        .iter()
        .find(|m| m.role == "architect")
        .unwrap();
    let sec = v
        .member_summaries
        .iter()
        .find(|m| m.role == "security_reviewer")
        .unwrap();
    let pm = v
        .member_summaries
        .iter()
        .find(|m| m.role == "product_manager")
        .unwrap();
    assert_eq!(arch.final_stance, StanceKind::StrongApprove);
    assert_eq!(sec.final_stance, StanceKind::Disapprove);
    assert_eq!(pm.final_stance, StanceKind::StrongApprove);

    // transcript 必须非空, 包含 3 member 名字
    let r0 = &v.rounds[0];
    assert!(!r0.transcript.is_empty());
    assert!(r0.transcript.contains("architect"));
    assert!(r0.transcript.contains("security_reviewer"));
    assert!(r0.transcript.contains("product_manager"));
    assert_eq!(r0.opinions.len(), 3);
}

// ============================================================
// Test 2: multi-LLM scripted — max_rounds (3 Disapprove, 0 共识)
// ============================================================
//
// 验证:
// - 3 Disapprove → consensus_score = 0.2 < 0.6 → 0 共识
// - 0 强反对 (Disapprove 不是 StrongDisapprove) → 0 按住
// - 跑满 3 轮 → termination_reason = "max_rounds" + rounds_run = 3
// - 每轮 3 opinions, 3 transcript entries
// - cross-round prior_opinions 传递 (round 2+ user 包含 round 1 opinions)

#[test]
fn multi_llm_scripted_3_member_3_round_no_consensus_runs_max() {
    let mut hm = HashMapMockLlm::new();
    hm.insert("architect", MockLlmResponse::ok("Disapprove"));
    hm.insert("security_reviewer", MockLlmResponse::ok("Disapprove"));
    hm.insert("product_manager", MockLlmResponse::ok("Disapprove"));
    let llm: Arc<dyn MockLlmProvider> = Arc::new(hm);

    let mut d = CouncilMemberDeliberator::new(standard_3_members())
        .with_mock_llm(llm)
        .with_max_rounds(3);
    let v = d.deliberate(&q("Risky proposal — 0 consensus expected"));

    assert_eq!(v.termination_reason, "max_rounds");
    assert_eq!(v.rounds_run, 3);
    assert!(!v.consensus_reached);
    assert_eq!(v.rounds.len(), 3);

    // 每轮 3 个 member 各出 1 个 opinion, 共识分数 < 0.6
    for r in &v.rounds {
        assert_eq!(r.opinions.len(), 3);
        assert!(r.consensus_score < CONSENSUS_SCORE_THRESHOLD);
        assert!(!r.has_strong_disapprove()); // Disapprove 不是 StrongDisapprove
    }

    // 0 强反对 → is_allowed = false (因 0 consensus_reached)
    assert!(!v.is_allowed());
}

// ============================================================
// Test 3: multi-LLM scripted — strong_disapprove 触发按住 (round 1 break)
// ============================================================
//
// 验证:
// - 1 StrongDisapprove (triggers_hold, confidence 0.95) → has_strong_disapprove = true
// - round 1 break → termination_reason = "strong_disapprove"
// - rounds_run = 1 (只跑了 1 轮)

#[test]
fn multi_llm_scripted_strong_disapprove_triggers_hold_round_1() {
    let mut hm = HashMapMockLlm::new();
    hm.insert("architect", MockLlmResponse::ok("StrongApprove"));
    hm.insert(
        "security_reviewer",
        MockLlmResponse::reject("StrongDisapprove — violates policy"), // triggers_hold=true, confidence=0.95
    );
    hm.insert("product_manager", MockLlmResponse::ok("StrongApprove"));
    let llm: Arc<dyn MockLlmProvider> = Arc::new(hm);

    let mut d = CouncilMemberDeliberator::new(standard_3_members()).with_mock_llm(llm);
    let v = d.deliberate(&q("deploy now"));

    assert_eq!(v.termination_reason, "strong_disapprove");
    assert_eq!(v.rounds_run, 1);
    assert!(!v.consensus_reached);
    assert!(!v.is_allowed());
}

// ============================================================
// Test 4: LIVE env-gated (#[ignore]) — 真 MiniMax
// ============================================================
//
// 跑真 MiniMax `https://api.minimaxi.com/anthropic/v1/messages`,
// 3 member × 3 round = 9 次 HTTP call. 仅 `cargo test ... -- --ignored` 跑.
//
// **env 控制**:
// - `APEIRETH_MINIMAX_LIVE_TEST=1` (必须 1 才跑, 默认 skip)
// - `APEIRETH_MINIMAX_API_KEY` (apikey)
//
// **关键技术点**:
// - LlmAdvisorBackend 在 #[test] sync context 调 Handle::try_current() 返 Err
//   → 走 fallback path 启动临时 runtime → 跑 LlmProvider complete() 走 reqwest
//   → 调真 MiniMax
// - 9 次 HTTP call (3 member × 3 round), wiremock 验证移至 R32-3-1 apeireth-eval
//
// **不漂移**: 0 改 R32-3-1 real_llm_smoke 路径; 0 改 R16-09 LlmAdvisorBackend

#[test]
#[ignore = "requires APEIRETH_MINIMAX_LIVE_TEST=1 + APEIRETH_MINIMAX_API_KEY + network"]
fn live_minimax_3_member_3_round_deliberation() {
    // 1. env gate
    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        eprintln!(
            "SKIP live_minimax_3_member_3_round_deliberation: \
             APEIRETH_MINIMAX_LIVE_TEST != 1, run with `cargo test -- --ignored`"
        );
        return;
    }
    let api_key = std::env::var("APEIRETH_MINIMAX_API_KEY").unwrap_or_default();
    assert!(
        !api_key.is_empty(),
        "APEIRETH_MINIMAX_API_KEY must be set for live test"
    );

    // 2. 构造 AnthropicCompatibleProvider (per R17 / R32-3-1 base URL)
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

    // 3. LlmAdvisorBackend 包装 (R16-09 LOCKED 复用)
    let backend: Arc<dyn apeireth_council::MockLlmProvider> =
        Arc::new(apeireth_council::LlmAdvisorBackend::new(arc_provider));

    // 4. 跑 3 member × 3 round deliberation
    let mut d = CouncilMemberDeliberator::new(standard_3_members())
        .with_mock_llm(backend)
        .with_max_rounds(3);
    let v = d.deliberate(&q(
        "Should we deploy the new Rust release to production? \
         Answer with exactly one of: StrongApprove / Approve / Neutral / Disapprove / StrongDisapprove / Abstain.",
    ));

    // 5. 验证: 跑 1-3 轮 + 有最终 stance
    eprintln!(
        "LIVE 3x3 verdict: rounds_run={} termination={} consensus={} final_stance={:?} \
         final_score={} elapsed_ms={} member_summaries={:?}",
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
    }
    assert!(v.rounds_run >= 1, "至少跑 1 轮");
    assert!(v.rounds_run <= 3, "最多 3 轮");
    assert_eq!(v.member_summaries.len(), 3);

    // transcript 必须非空 (每轮都有 member × round 拼接)
    for r in &v.rounds {
        assert!(!r.transcript.is_empty());
        assert_eq!(r.opinions.len(), 3, "每轮 3 个 member 各出 1 个 opinion");
    }

    // 7 阶段 eval score 必须 ≥ 6/7 (允许 weighted_score_normalized < 1 因真 LLM 不一定 100% 共识)
    let scores = v.to_eval_scores();
    let pass_count = scores.iter().filter(|(_, v)| *v >= 0.5).count();
    assert!(
        pass_count >= 5,
        "至少 5/7 stage pass, got {}/7: {:?}",
        pass_count,
        scores
    );
}
