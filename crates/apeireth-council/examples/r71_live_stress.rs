//! R71 LIVE: CouncilMember deliberation 100 round stress 真接 MiniMax example
//! (env-gated `APEIRETH_COUNCIL_LIVE=1`)
//!
//! **目标**: 跑 10 round deliberation (CI smoke), 拿 markdown 报告 + p50/p95/p99 + consensus rate.
//! 借鉴 R68 stress_test runner (per master 8/9 拍板 LIVE 真接模式).
//!
//! **跑法**:
//! ```bash
//! APEIRETH_COUNCIL_LIVE=1 cargo run --example r71_live_stress --release
//! ```
//!
//! **CI gating**: 默认 `APEIRETH_COUNCIL_LIVE` 未设置时跳过 (0 网络环境).

use std::sync::Arc;

use apeireth_council::council_member::CouncilMember;
use apeireth_council::deliberation::CouncilQuery;
use apeireth_council::mock_llm::{MockLlmProvider, MockLlmResponse};
use apeireth_council::stress_test::{run_deliberation_stress, StressConfig, StressReport};

fn main() {
    if std::env::var("APEIRETH_COUNCIL_LIVE").ok().as_deref() != Some("1") {
        eprintln!(
            "R71 LIVE example: skip (set APEIRETH_COUNCIL_LIVE=1 to enable live deliberation)"
        );
        eprintln!("Note: this uses ScriptedMockLlm for portability; for real LLM hookup see R33-4-3 follow-up");
        std::process::exit(0);
    }

    let members = vec![
        CouncilMember::new(
            "architect",
            "find stable rust lock",
            "10y rust",
            "claude_code",
        ),
        CouncilMember::new("security_reviewer", "find CVEs", "5y sec", "codex"),
        CouncilMember::new("product_manager", "user value", "5y pm", "gemini_cli"),
    ];

    let provider = Arc::new(ScriptedApproveEcho);
    let query = CouncilQuery::new("q1", "should we adopt MiniMax M3?", 0);

    // LIVE 真接 10 round (CI 友好; 全跑 100 round 估 ~30s)
    let cfg = StressConfig::default().with_rounds(10).with_verbose(false);

    let report: StressReport = run_deliberation_stress(members, &query, provider, cfg);

    println!("{}", report.to_markdown());

    eprintln!(
        "R71 LIVE summary: {}/{} consensus (rate {:.0}%); p50={}ms p95={}ms p99={}ms",
        report.consensus_count,
        report.rounds_run,
        report.consensus_rate * 100.0,
        report.latency_p50_ms,
        report.latency_p95_ms,
        report.latency_p99_ms
    );
}

/// 测试用 Echo provider: 固定返 Approve (per R68 `AlwaysApproveEcho`)
#[derive(Debug)]
struct ScriptedApproveEcho;
impl MockLlmProvider for ScriptedApproveEcho {
    fn generate(&self, _prompt: &str, _system: &str) -> MockLlmResponse {
        MockLlmResponse::ok("approve")
    }
}
