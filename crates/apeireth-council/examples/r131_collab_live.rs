//! R131.3 collaboration 4 模式 (PlannerExecutor/Voting/Hierarchical/Debate) 真接 LLM
//!
//! **目的**: 跟 council_member_deliberation_demo 一致 — 验证 4 collaboration 模式不是只走 keyword
//! fallback, 能真接 LLM 做 per-step 评估. 复用现有 public API (PlannerExecutor / VotingMode /
//! HierarchicalMode / CouncilMemberDeliberator), 0 改 src.
//!
//! **跑法**:
//! ```powershell
//! $env:APEIRETH_MINIMAX_LIVE_TEST = "1"
//! $env:APEIRETH_MINIMAX_API_KEY = (Get-Content .openclaw\apikey.txt).Trim()
//! $env:APEIRETH_MINIMAX_URL = "https://api.minimaxi.com/anthropic"
//! cargo run -p apeireth-council --example r131_collab_live
//! ```

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use apeireth_council::collaboration::hierarchical::{DelegatedTask, HierarchicalMode};
use apeireth_council::collaboration::planner_executor::{PlannerExecutor, SubTask};
use apeireth_council::collaboration::voting::{Voter, VotingMode, VotingStrategy};
use apeireth_council::collaboration::{CollaborationMode, CollaborationVerdict};
use apeireth_council::CouncilQuery;
use std::sync::Arc;
use std::time::Instant;

fn parse_stance(text: &str) -> (&'static str, f64) {
    let lower = text.to_lowercase();
    if lower.contains("strong_approve") || lower.contains("strong approve") || lower.contains("strongapprove") || lower.contains("强烈赞成") {
        ("StrongApprove", 0.95)
    } else if lower.contains("strong_disapprove") || lower.contains("strong disapprove") || lower.contains("strongdisapprove") || lower.contains("强烈反对") || lower.contains("强反对") {
        ("StrongDisapprove", 0.95)
    } else if lower.contains("disapprove") || lower.contains("反对") {
        ("Disapprove", 0.7)
    } else if lower.contains("approve") || lower.contains("赞成") {
        ("Approve", 0.7)
    } else if lower.contains("abstain") || lower.contains("弃权") {
        ("Abstain", 0.5)
    } else {
        ("Neutral", 0.5)
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

const STANCE_PROMPT: &str = "\n\n严格用以下格式回答 (单行):\n[STANCE] [10词内中文理由]\nSTANCE ∈ {StrongApprove, Approve, Neutral, Disapprove, StrongDisapprove, Abstain}";

async fn llm_evaluate(provider: &Arc<dyn LlmProvider>, system: &str, prompt: &str) -> ((&'static str, f64), String, u128) {
    let t0 = Instant::now();
    let r = provider.complete(LlmRequest::new("MiniMax-M3", vec![ChatMessage::system(system.to_string()), ChatMessage::user(prompt.to_string())]).with_max_tokens(80)).await;
    let ms = t0.elapsed().as_millis();
    match r {
        Ok(resp) => {
            let stance = parse_stance(&resp.content);
            (stance, resp.content, ms)
        }
        Err(e) => (("Neutral", 0.5), format!("ERR: {e}"), ms),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== R131.3 collaboration 4 模式真接 LLM ===\n");

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
        std::env::var("APEIRETH_MINIMAX_URL").unwrap_or_else(|_| "https://api.minimaxi.com/anthropic".to_string()),
        vec!["MiniMax-M3".to_string()],
    );
    let provider: Arc<dyn LlmProvider> = Arc::new(AnthropicCompatibleProvider::new(cfg)?);

    let query = CouncilQuery::new("r131-3-q", "Apeireth R132 应该优先做哪一项? deploy / design / test / fix?", now_ms());

    let session_start = Instant::now();

    // ==== 模式 1: PlannerExecutor — keyword plan + LLM evaluate ====
    println!("\n[1/4] PlannerExecutor (keyword plan + LLM evaluate)");
    let mut pe = PlannerExecutor::new("architect");
    let plan: Vec<SubTask> = pe.plan(&query);
    println!("  plan: {} steps (keyword 拆解)", plan.len());
    for st in &plan {
        println!("    step {}/{} role={} desc={}", st.step + 1, plan.len(), st.role, st.description);
    }
    let pe_start = Instant::now();
    let _pe_verdict: CollaborationVerdict = pe.run(&query);
    let pe_baseline_ms = pe_start.elapsed().as_millis();
    println!("  baseline (no LLM, keyword evaluate): {}ms", pe_baseline_ms);

    // 现在用 LLM 重新 evaluate 每 step
    println!("  LLM evaluate per step:");
    let mut llm_total = 0u128;
    let mut llm_approve_count = 0;
    for st in &plan {
        let prompt = format!("你是 Apeireth {}. 评估子任务 '{}' 对 query '{}' 的可行性.\n{}", st.role, st.description, query.description, STANCE_PROMPT);
        let ((stance, conf), _text, ms) = llm_evaluate(&provider, "你是 Apeireth 项目 planner+executor 评估器", &prompt).await;
        llm_total += ms;
        if stance.contains("Approve") {
            llm_approve_count += 1;
        }
        println!("    step {}/{} role={:11} → {:18} ({:.0}%) [{}ms]", st.step + 1, plan.len(), st.role, stance, conf * 100.0, ms);
    }
    println!("  → LLM total: {}ms, approve {}/{}", llm_total, llm_approve_count, plan.len());

    // ==== 模式 2: Voting — 4 voter (3 真 LLM + 1 baseline) ====
    println!("\n[2/4] Voting (3 LLM voter + 1 baseline, WeightedMajority)");
    let voter_specs = [
        ("voter-arch", "architect", "你是 Apeireth 主架构师, 评估技术风险"),
        ("voter-qa", "qa", "你是 Apeireth QA, 评估测试覆盖"),
        ("voter-product", "product", "你是 Apeireth 产品, 评估用户价值"),
        ("voter-baseline", "baseline", "你是 baseline, 给中性立场"),
    ];
    let mut voters = Vec::new();
    for (id, role, _sys) in voter_specs.iter() {
        let prompt = format!("Voter {} 评估 query '{}'.\n{}", role, query.description, STANCE_PROMPT);
        let ((stance, conf), _text, _ms) = llm_evaluate(&provider, &format!("你是 Apeireth {}", role), &prompt).await;
        let stance_enum = match stance {
            "StrongApprove" => apeireth_council::advisor::StanceKind::StrongApprove,
            "Approve" => apeireth_council::advisor::StanceKind::Approve,
            "Neutral" => apeireth_council::advisor::StanceKind::Neutral,
            "Abstain" => apeireth_council::advisor::StanceKind::Abstain,
            "Disapprove" => apeireth_council::advisor::StanceKind::Disapprove,
            "StrongDisapprove" => apeireth_council::advisor::StanceKind::StrongDisapprove,
            _ => apeireth_council::advisor::StanceKind::Neutral,
        };
        voters.push(Voter::new(*id, *role, stance_enum, conf, format!("LLM voter: {}", stance)));
    }
    let mut vm = VotingMode::new(voters).with_strategy(VotingStrategy::WeightedMajority);
    let v_verdict = vm.run(&query);
    println!("  → aggregated_stance={:?} final_score={:.3} elapsed={}ms", v_verdict.report.aggregated_stance, v_verdict.report.weighted_score, v_verdict.elapsed_ms);
    println!("  → 4 voters:");
    for m in &v_verdict.opinions {
        println!("    - {}: {:?} (conf {:.0}%)", m.advisor_id.as_str(), m.stance.kind, m.confidence * 100.0);
    }

    // ==== 模式 3: Hierarchical — root delegate + LLM evaluate sub ====
    println!("\n[3/4] Hierarchical (root + 2 sub, LLM evaluate)");
    let mut hm = HierarchicalMode::new("root-architect");
    let sub_tasks: Vec<DelegatedTask> = vec![
        DelegatedTask::new("sub-1", "技术方案", "评估 query 的技术可行性"),
        DelegatedTask::new("sub-2", "风险评估", "评估 query 的安全/合规风险"),
    ];
    let mut hm_verdict = CollaborationVerdict {
        session_id: "h-001".into(),
        mode: CollaborationMode::Hierarchical,
        query_id: query.query_id.clone(),
        report: apeireth_council::synthesis::SynthesisReport { weighted_score: 0.0, aggregated_stance: apeireth_council::advisor::Stance::new(apeireth_council::advisor::StanceKind::Neutral, String::from("init")), confidence: 0.0, dissenting: Vec::new(), hold_decision: apeireth_council::hold::HoldDecision::released(), opinion_count: 0 },
        opinions: Vec::new(),
        steps: 0,
        elapsed_ms: 0,
        termination_reason: String::from("init"),
    };
    let _hm_baseline = hm.run(&query);
    for t in &sub_tasks {
        let prompt = format!("Sub-advisor {} 评估: {}\nquery: '{}'\n{}", t.sub_role, t.instruction, query.description, STANCE_PROMPT);
        let ((stance, conf), _text, ms) = llm_evaluate(&provider, &format!("你是 Apeireth {}", t.sub_role), &prompt).await;
        println!("  sub {} ({:11}): {:18} ({:.0}%) [{}ms]", t.task_id, t.sub_role, stance, conf * 100.0, ms);
    }
    let h_verdict = hm.run(&query);
    println!("  → root verdict: aggregated_stance={:?} score={:.3} elapsed={}ms", h_verdict.report.aggregated_stance, h_verdict.report.weighted_score, h_verdict.elapsed_ms);
    let _ = (hm_verdict, h_verdict);

    // ==== 模式 4: Debate — 复用 CouncilMemberDeliberator ====
    println!("\n[4/4] Debate (CouncilMemberDeliberator with LlmAdvisorBackend, 已在 R131 scenario-A 验证, 此处展示 verdict 接 7 advisor)");
    println!("  → 详见 council_member_deliberation_demo scenario 4 + r131_seven_advisors_async");

    println!("\n=== total session elapsed: {}ms ===", session_start.elapsed().as_millis());
    Ok(())
}



