//! R68: CouncilMember Deliberation Stress Test Runner
//!
//! **目标**: 跑 N 轮 (默认 100) deliberation + 收集 metrics — 真接 MockLlmProvider (CI 默认 0 网络),
//! 留 env-gated `APEIRETH_MINIMAX_LIVE_TEST=1` 跑真 LLM stress (后续 R33-4-3 真接 LlmProvider 路线).
//!
//! **借鉴锚 (S-1)**:
//! - AutoGen `GroupChat.run_chat` 多轮 driver (`autogen/agentchat/groupchat.py`)
//! - LangChain `ConversationChain` stress test pattern
//! - k6 / vegeta stress test metric 集合 (latency_p50/p95/p99 + error_rate)
//!
//! **不漂移 (主哲学锚 #1)**:
//! - 0 改 `CouncilMemberDeliberator` / `CouncilMember` / `MockLlmProvider` (R33-4-1 LOCKED, R33-4 LOCKED)
//! - 0 改 persona / advisor / deliberation / synthesis / hold (LOCKED)
//! - 0 改 workspace 1.0.0 / 24 LOCKED crate
//! - 0 引 I/O / 网络 (默认 mock; 真 LLM env-gated)

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::council_member::CouncilMember;
use crate::council_member_deliberation::{
    CouncilMemberDeliberator, MultiRoundVerdict,
};
use crate::deliberation::CouncilQuery;
use crate::mock_llm::{MockLlmProvider, MockLlmResponse};

/// 默认 stress rounds (per master 8/9 实测 100 round 200s pass)
pub const DEFAULT_STRESS_ROUNDS: u32 = 100;

/// 默认 max deliberation rounds per call (per DEFAULT_MAX_ROUNDS R33-4-1)
pub const DEFAULT_DELIBERATION_ROUNDS: u8 = 3;

/// Stress test 终止原因
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerminationReason {
    /// 达到 stress rounds 上限
    MaxRounds,
    /// 共识达成
    Consensus,
    /// 强反对触发按住
    Hold,
    /// 单 round 内错误 (panic / LLM 错 / parse 错)
    Error,
}

/// StressConfig (per master 8/9 实拍参数)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StressConfig {
    /// 总 stress rounds (默认 100)
    pub rounds: u32,
    /// 单 deliberation max rounds (默认 3)
    pub max_deliberation_rounds: u8,
    /// 是否打 verbose log (默认 false, CI 0 网络环境友好)
    pub verbose: bool,
}

impl Default for StressConfig {
    fn default() -> Self {
        Self {
            rounds: DEFAULT_STRESS_ROUNDS,
            max_deliberation_rounds: DEFAULT_DELIBERATION_ROUNDS,
            verbose: false,
        }
    }
}

impl StressConfig {
    pub fn with_rounds(mut self, rounds: u32) -> Self {
        self.rounds = rounds;
        self
    }
    pub fn with_seed(self, _seed: u64) -> Self {
        // seed 占位 (per master 8/9 拍板, mock LLM 不依赖 RNG, 真 LLM 后续可加)
        self
    }
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

/// 单 round result (per k6 per-iteration metric)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoundResult {
    /// round index (0-based)
    pub round_index: u32,
    /// 该 round 的 deliberation verdict (cloned)
    pub consensus_reached: bool,
    pub final_score: f64, // R68 field name shadow OK; map from final_weighted_score at call site
    pub final_stance: String,
    pub rounds: u8,
    /// 该 round 耗时 (ms)
    pub elapsed_ms: u64,
    /// 终止原因
    pub termination_reason: TerminationReason,
    /// error message (any stage fail)
    pub error: Option<String>,
}

/// StressReport (汇总)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StressReport {
    /// 实际跑的 rounds 数
    pub rounds_run: u32,
    /// 总耗时 (ms)
    pub total_elapsed_ms: u128,
    /// 共识达成次数
    pub consensus_count: u32,
    /// 平均 consensus_score (per round)
    pub avg_consensus_score: f64,
    /// 共识达成率 (consensus_count / rounds_run)
    pub consensus_rate: f64,
    /// latency p50 / p95 / p99 (ms)
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub latency_p99_ms: u64,
    /// 错误率 (errors / rounds_run)
    pub error_rate: f64,
    /// 终止原因 histogram
    pub termination_histogram: std::collections::BTreeMap<String, u32>,
    /// 单 round 详细 (verbose 模式 / reporting 用)
    pub round_results: Vec<RoundResult>,
    /// 配置 (echo)
    pub config: StressConfig,
}

impl StressReport {
    /// Markdown report (per master 8/9 实拍 markdown 风格)
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Stress Test Report ({})\n\n", chrono_placeholder_iso()));
        out.push_str(&format!("- Rounds run: **{}**\n", self.rounds_run));
        out.push_str(&format!("- Total elapsed: **{} ms**\n", self.total_elapsed_ms));
        out.push_str(&format!("- Consensus count: **{}**\n", self.consensus_count));
        out.push_str(&format!("- Consensus rate: **{:.1}%**\n", self.consensus_rate * 100.0));
        out.push_str(&format!("- Avg consensus score: **{:.3}**\n", self.avg_consensus_score));
        out.push_str(&format!(
            "- Latency p50/p95/p99: **{} / {} / {} ms**\n",
            self.latency_p50_ms, self.latency_p95_ms, self.latency_p99_ms
        ));
        out.push_str(&format!("- Error rate: **{:.1}%**\n", self.error_rate * 100.0));
        out.push_str("\n## Termination histogram\n\n");
        out.push_str("| Reason | Count |\n|---|---|\n");
        for (k, v) in &self.termination_histogram {
            out.push_str(&format!("| `{}` | {} |\n", k, v));
        }
        out
    }
}

fn chrono_placeholder_iso() -> String {
    // 简化 placeholder: 不引 chrono dep (per master 8/9 不引新 dep 节奏)
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("unix_ms_{}", d.as_millis()))
        .unwrap_or_else(|_| "now".to_string())
}

fn percentile(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).floor() as usize;
    sorted[idx]
}

fn reason_label(r: TerminationReason) -> &'static str {
    match r {
        TerminationReason::MaxRounds => "max_rounds",
        TerminationReason::Consensus => "consensus",
        TerminationReason::Hold => "hold",
        TerminationReason::Error => "error",
    }
}

fn extract_verdict_summary(v: &MultiRoundVerdict) -> (bool, f64, String, u8) {
    let stance = format!("{:?}", v.final_stance);
    (v.consensus_reached, v.final_weighted_score, stance, v.rounds_run)
}

/// R68 真正实现: 用 Arc<dyn MockLlmProvider> (per master 8/9 拍板的 stress 真接 LLM 模式)
pub fn run_deliberation_stress(
    members: Vec<CouncilMember>,
    query: &CouncilQuery,
    provider: Arc<dyn MockLlmProvider>,
    config: StressConfig,
) -> StressReport {
    let start = Instant::now();
    let mut round_results = Vec::with_capacity(config.rounds as usize);
    let mut latencies = Vec::with_capacity(config.rounds as usize);
    let mut consensus_count = 0u32;
    let mut error_count = 0u32;
    let mut consensus_score_sum = 0.0f64;
    let mut term_hist = std::collections::BTreeMap::<String, u32>::new();

    for i in 0..config.rounds {
        let round_start = Instant::now();
        let provider_clone = Arc::clone(&provider);
        let mut d = CouncilMemberDeliberator::new(members.clone())
            .with_mock_llm(provider_clone)
            .with_max_rounds(config.max_deliberation_rounds);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| d.deliberate(query)));
        let elapsed_ms = round_start.elapsed().as_millis() as u64;
        match result {
            Ok(verdict) => {
                let score = verdict.final_weighted_score;
                consensus_score_sum += score;
                let (reached, _s, _st, _r) = extract_verdict_summary(&verdict);
                if reached {
                    consensus_count += 1;
                }
                let reason = if reached {
                    TerminationReason::Consensus
                } else {
                    TerminationReason::MaxRounds
                };
                *term_hist.entry(reason_label(reason).to_string()).or_insert(0) += 1;
                round_results.push(RoundResult {
                    round_index: i,
                    consensus_reached: reached,
                    final_score: score, // mapped from verdict.final_weighted_score
                    final_stance: format!("{:?}", verdict.final_stance),
                    rounds: verdict.rounds_run,
                    elapsed_ms,
                    termination_reason: reason,
                    error: None,
                });
            }
            Err(_) => {
                error_count += 1;
                *term_hist
                    .entry(reason_label(TerminationReason::Error).to_string())
                    .or_insert(0) += 1;
                round_results.push(RoundResult {
                    round_index: i,
                    consensus_reached: false,
                    final_score: 0.0,
                    final_stance: "Error".into(),
                    rounds: 0,
                    elapsed_ms,
                    termination_reason: TerminationReason::Error,
                    error: Some("deliberator panic caught".into()),
                });
            }
        }
        latencies.push(elapsed_ms);
    }
    let total_elapsed = start.elapsed().as_millis();

    latencies.sort_unstable();
    let n = round_results.len();
    let consensus_rate = if n > 0 { f64::from(consensus_count) / (n as f64) } else { 0.0 };
    let error_rate = if n > 0 { f64::from(error_count) / (n as f64) } else { 0.0 };

    StressReport {
        rounds_run: n as u32,
        total_elapsed_ms: total_elapsed,
        consensus_count,
        consensus_rate,
        avg_consensus_score: consensus_score_sum / n.max(1) as f64,
        latency_p50_ms: percentile(&latencies, 0.50),
        latency_p95_ms: percentile(&latencies, 0.95),
        latency_p99_ms: percentile(&latencies, 0.99),
        error_rate,
        termination_histogram: term_hist,
        round_results,
        config,
    }
}

// ============================================================
// R68 单元测试 (per master 8/9 实拍: 10 round smoke + 真实 echo provider)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 Echo provider: 固定返 Approve (per R33-4-1 deliberation 默认 fallback)
    #[derive(Debug)]
    struct AlwaysApproveEcho;
    impl MockLlmProvider for AlwaysApproveEcho {
        fn generate(&self, _prompt: &str, _system: &str) -> MockLlmResponse {
            MockLlmResponse::ok("approve")
        }
    }

    fn three_members() -> Vec<CouncilMember> {
        vec![
            CouncilMember::new("architect", "find stable rust", "10y rust", "claude_code"),
            CouncilMember::new("security_reviewer", "find vulns", "5y sec", "codex"),
            CouncilMember::new("product_manager", "user value", "5y pm", "gemini_cli"),
        ]
    }

    fn dummy_query() -> CouncilQuery {
        CouncilQuery::new("q1", "should we deploy v2?", 0)
    }

    #[test]
    fn stress_config_default_values() {
        let cfg = StressConfig::default();
        assert_eq!(cfg.rounds, DEFAULT_STRESS_ROUNDS);
        assert_eq!(cfg.max_deliberation_rounds, DEFAULT_DELIBERATION_ROUNDS);
        assert!(!cfg.verbose);
    }

    #[test]
    fn stress_config_with_methods() {
        let cfg = StressConfig::default()
            .with_rounds(10)
            .with_seed(42)
            .with_verbose(true);
        assert_eq!(cfg.rounds, 10);
        assert!(cfg.verbose);
    }

    #[test]
    fn stress_report_10_rounds_smoke() {
        let members = three_members();
        let provider = Arc::new(AlwaysApproveEcho);
        let cfg = StressConfig::default().with_rounds(10);
        let report = run_deliberation_stress(members, &dummy_query(), provider, cfg);
        assert_eq!(report.rounds_run, 10);
        // R68: mock LLM deliberation is too fast (sub-ms) — 0 ms valid (CI 0 网络环境); total elapsed 0 OK for smoke
        assert_eq!(report.error_rate, 0.0);
    }

    #[test]
    fn stress_report_percentiles_monotonic() {
        let members = three_members();
        let provider = Arc::new(AlwaysApproveEcho);
        let cfg = StressConfig::default().with_rounds(20);
        let report = run_deliberation_stress(members, &dummy_query(), provider, cfg);
        assert!(report.latency_p50_ms <= report.latency_p95_ms);
        assert!(report.latency_p95_ms <= report.latency_p99_ms);
    }

    #[test]
    fn stress_report_termination_histogram_records_reasons() {
        let members = three_members();
        let provider = Arc::new(AlwaysApproveEcho);
        let cfg = StressConfig::default().with_rounds(15);
        let report = run_deliberation_stress(members, &dummy_query(), provider, cfg);
        assert!(!report.termination_histogram.is_empty());
        let total_in_hist: u32 = report.termination_histogram.values().sum();
        assert_eq!(total_in_hist, 15);
    }

    #[test]
    fn stress_report_markdown_contains_key_sections() {
        let members = three_members();
        let provider = Arc::new(AlwaysApproveEcho);
        let cfg = StressConfig::default().with_rounds(5);
        let report = run_deliberation_stress(members, &dummy_query(), provider, cfg);
        let md = report.to_markdown();
        assert!(md.contains("Rounds run"));
        assert!(md.contains("Total elapsed"));
        assert!(md.contains("Consensus rate"));
        assert!(md.contains("Latency p50/p95/p99"));
        assert!(md.contains("Termination histogram"));
    }

    #[test]
    fn stress_report_avg_consensus_score_in_range() {
        let members = three_members();
        let provider = Arc::new(AlwaysApproveEcho);
        let cfg = StressConfig::default().with_rounds(15);
        let report = run_deliberation_stress(members, &dummy_query(), provider, cfg);
        assert!(report.avg_consensus_score >= 0.0);
        assert!(report.avg_consensus_score <= 1.0);
    }

    #[test]
    fn stress_report_round_results_length_matches() {
        let members = three_members();
        let provider = Arc::new(AlwaysApproveEcho);
        let cfg = StressConfig::default().with_rounds(7);
        let report = run_deliberation_stress(members, &dummy_query(), provider, cfg);
        assert_eq!(report.round_results.len(), 7);
        for (i, r) in report.round_results.iter().enumerate() {
            assert_eq!(r.round_index, i as u32);
        }
    }

    #[test]
    fn percentile_empty_returns_zero() {
        assert_eq!(percentile(&[], 0.5), 0);
    }

    #[test]
    fn percentile_basic() {
        let v = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        assert_eq!(percentile(&v, 0.50), 50);
        assert_eq!(percentile(&v, 0.95), 90);
    }

    #[test]
    fn reason_label_all_returns_snake_case() {
        assert_eq!(reason_label(TerminationReason::MaxRounds), "max_rounds");
        assert_eq!(reason_label(TerminationReason::Consensus), "consensus");
        assert_eq!(reason_label(TerminationReason::Hold), "hold");
        assert_eq!(reason_label(TerminationReason::Error), "error");
    }
}




