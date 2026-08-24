//! R131.3 7 强制 Advisor 真接 LLM (短回答 prompt + tokio current_thread)

use apeireth_api::llm::providers::anthropic_compat::{
    AnthropicCompatibleConfig, AnthropicCompatibleProvider,
};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};
use apeireth_council::council_member::CouncilMember;
use std::sync::Arc;
use std::time::Instant;

fn parse_stance(text: &str) -> (&'static str, f64) {
    let lower = text.to_lowercase();
    if lower.contains("strong_approve")
        || lower.contains("strong approve")
        || lower.contains("strongapprove")
        || lower.contains("强烈赞成")
    {
        ("StrongApprove", 0.95)
    } else if lower.contains("strong_disapprove")
        || lower.contains("strong disapprove")
        || lower.contains("strongdisapprove")
        || lower.contains("强烈反对")
        || lower.contains("强反对")
    {
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 7 强制 Advisor 真接 MiniMax (async + 短答) R131.3 ===\n");

    if std::env::var("APEIRETH_MINIMAX_LIVE_TEST").unwrap_or_default() != "1" {
        println!("[skip] set APEIRETH_MINIMAX_LIVE_TEST=1 to enable real LLM");
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
    let provider = Arc::new(AnthropicCompatibleProvider::new(cfg)?);

    let members = seven_advisory_members();
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

    // 强制短答 prompt
    let stance_prompt = "\n\n严格用以下格式回答 (单行):\n[STANCE] [10词内理由]\nSTANCE ∈ {StrongApprove, Approve, Neutral, Disapprove, StrongDisapprove, Abstain}".to_string();

    let session_start = Instant::now();
    for (i, (label, desc)) in scenarios.iter().enumerate() {
        println!("\n[{}] {}", i + 1, label);
        println!("     query: {}", desc);
        let q_start = Instant::now();

        let mut prior_text = String::new();
        let mut termination_reason = "max_rounds";
        let mut consensus_reached = false;
        let mut total_calls = 0usize;

        for round in 0..3u8 {
            let mut round_stances: Vec<(&str, f64)> = Vec::new();
            let round_start = Instant::now();
            for (idx, member) in members.iter().enumerate() {
                let system = member.to_system_prompt();
                let user = format!(
                    "{}\n\n[第 {} 轮] [member #{} role={}]\n之前意见:\n{}{}",
                    desc,
                    round + 1,
                    idx,
                    member.role,
                    if prior_text.is_empty() {
                        "无 (第 1 轮)".to_string()
                    } else {
                        prior_text.clone()
                    },
                    stance_prompt,
                );
                let t0 = Instant::now();
                let r = provider
                    .complete(
                        LlmRequest::new(
                            "MiniMax-M3",
                            vec![ChatMessage::system(system), ChatMessage::user(user)],
                        )
                        .with_max_tokens(80),
                    )
                    .await;
                total_calls += 1;
                let elapsed = t0.elapsed().as_millis();
                match r {
                    Ok(resp) => {
                        let (stance, conf) = parse_stance(&resp.content);
                        let preview = resp.content.chars().take(40).collect::<String>();
                        println!(
                            "       R{} #{} {:11} → {:18} ({:.0}%) [{}ms] {:?}",
                            round + 1,
                            idx,
                            member.role,
                            stance,
                            conf * 100.0,
                            elapsed,
                            preview
                        );
                        round_stances.push((stance, conf));
                    }
                    Err(e) => {
                        println!(
                            "       R{} #{} {:11} → ERR: {}",
                            round + 1,
                            idx,
                            member.role,
                            e
                        );
                        termination_reason = "error";
                    }
                }
            }
            let round_ms = round_start.elapsed().as_millis();
            prior_text = round_stances
                .iter()
                .enumerate()
                .map(|(idx, (s, c))| format!("- member #{}: {} ({:.0}%)", idx, s, c * 100.0))
                .collect::<Vec<_>>()
                .join("\n");

            let has_strong_disapprove = round_stances
                .iter()
                .any(|(s, c)| *s == "StrongDisapprove" && *c >= 0.5);
            if has_strong_disapprove {
                termination_reason = "strong_disapprove";
                println!(
                    "     [R{}] 耗时 {}ms, 触发 strong_disapprove, 终止",
                    round + 1,
                    round_ms
                );
                break;
            }
            let all_approve = !round_stances.is_empty()
                && round_stances.iter().all(|(s, _)| s.contains("Approve"));
            if all_approve {
                consensus_reached = true;
                termination_reason = "consensus";
                println!(
                    "     [R{}] 耗时 {}ms, 全部 approve, 共识达成",
                    round + 1,
                    round_ms
                );
                break;
            }
            println!("     [R{}] 耗时 {}ms, 继续下一轮", round + 1, round_ms);
        }

        println!(
            "     → termination={} consensus={} elapsed_ms={} total_calls={}",
            termination_reason,
            consensus_reached,
            q_start.elapsed().as_millis(),
            total_calls
        );
    }
    println!(
        "\n=== total session elapsed: {}ms ===",
        session_start.elapsed().as_millis()
    );
    Ok(())
}
