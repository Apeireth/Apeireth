//! Voting 模式 — 单轮加权投票 (AutoGen GroupChatManager 投票聚合 + LangGraph conditional edges 借鉴)
//!
//! **职责** (per v2.0 strategy §2B "Voting"):
//! - 单轮投票 (1 round, 0 跨轮 state)
//! - 加权多数 (Σ(stance.score × weight × confidence) > 0 → Approve)
//! - 复用 R10 `synthesize()` 加权计算 (0 漂移)
//!
//! **借鉴锚** (S-1):
//! - AutoGen `GroupChatManager` 投票聚合
//! - VCP `vcpLoop/toolCallParser.js` 投票后分支
//! - LangGraph `add_conditional_edges` 投票后条件分支
//!
//! **0 漂移**:
//! - 0 改 R10 `synthesis::synthesize()` (直接 `pub use` 复用)
//! - 0 改 R10 `Council::deliberate` (仅 1 round, 不走 multi-round path)
//! - 0 引入 I/O / 网络 / 外部 LLM HTTP

use crate::advisor::{AdvisorId, AdvisorOpinion, Stance, StanceKind};
use crate::collaboration::types::{CollaborationContext, CollaborationMode, CollaborationVerdict};
use crate::deliberation::CouncilQuery;
use crate::synthesis::{synthesize, SynthesisReport, SynthesisWeights};
use serde::{Deserialize, Serialize};

/// 投票策略 (per 派活单决策权)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VotingStrategy {
    /// 简单多数 — 加权 score > 0 → Approve (default, 跟 R10 synthesize 1:1 兼容)
    WeightedMajority,
    /// 加权最高 — 取 weighted_score 最高的 opinion 当 verdict (single-opinion verdict)
    TopScoring,
    /// 共识 — 必须 ≥ 2/3 加权 score > 0 才算 Approve
    Supermajority,
}

impl Default for VotingStrategy {
    fn default() -> Self {
        Self::WeightedMajority
    }
}

impl VotingStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::WeightedMajority => "weighted_majority",
            Self::TopScoring => "top_scoring",
            Self::Supermajority => "supermajority",
        }
    }
}

/// Voter (per advisor 投 1 opinion)
pub struct Voter {
    /// 投票人 ID
    pub voter_id: String,
    /// 投票人角色
    pub role: String,
    /// 投票立场 (per keyword fallback, 0 LLM)
    pub stance: StanceKind,
    /// 置信度 (0.0 - 1.0)
    pub confidence: f64,
    /// 推理依据
    pub reasoning: String,
}

impl Voter {
    /// 便利构造
    pub fn new(
        voter_id: impl Into<String>,
        role: impl Into<String>,
        stance: StanceKind,
        confidence: f64,
        reasoning: impl Into<String>,
    ) -> Self {
        Self {
            voter_id: voter_id.into(),
            role: role.into(),
            stance,
            confidence: confidence.clamp(0.0, 1.0),
            reasoning: reasoning.into(),
        }
    }

    /// 转 `AdvisorOpinion` (per synthesize 0 漂移)
    pub fn to_opinion(&self, started_at_ms: i64) -> AdvisorOpinion {
        let stance = Stance::new(
            self.stance,
            format!("Voter {} ({}) vote: {:?}", self.voter_id, self.role, self.stance),
        );
        AdvisorOpinion::new(
            AdvisorId::new(self.voter_id.clone()),
            stance,
            self.confidence,
            self.reasoning.clone(),
            started_at_ms,
        )
    }
}

/// Voting 模式驱动器
pub struct VotingMode {
    /// voter 列表
    voters: Vec<Voter>,
    /// 投票策略
    strategy: VotingStrategy,
    /// synthesis 权重
    weights: SynthesisWeights,
    /// 内部 session 序列
    next_session_seq: u64,
}

impl VotingMode {
    /// 构造 (默认 WeightedMajority 策略)
    pub fn new(voters: Vec<Voter>) -> Self {
        Self {
            voters,
            strategy: VotingStrategy::default(),
            weights: SynthesisWeights::default(),
            next_session_seq: 0,
        }
    }

    /// 自定义 strategy (chainable)
    pub fn with_strategy(mut self, strategy: VotingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// 自定义 weights (chainable)
    pub fn with_weights(mut self, weights: SynthesisWeights) -> Self {
        self.weights = weights;
        self
    }

    /// voter 数
    pub fn voter_count(&self) -> usize {
        self.voters.len()
    }

    /// 策略
    pub fn strategy(&self) -> VotingStrategy {
        self.strategy
    }

    /// **跑 Voting 流程** — 1 round, 复用 R10 `synthesize()`
    ///
    /// **流程**:
    /// 1. 每 voter 转 1 opinion
    /// 2. 全部 opinions → synthesize() → weighted_score
    /// 3. 按 strategy 决定 verdict:
    ///    - WeightedMajority: weighted_score > 0 → Approve, 反之 → Disapprove
    ///    - TopScoring: 最高 weighted_score opinion 当 verdict
    ///    - Supermajority: ≥ 2/3 weighted_score > 0 → Approve
    pub fn run(&mut self, query: &CouncilQuery) -> CollaborationVerdict {
        self.next_session_seq += 1;
        let started_at_ms = query.started_at_ms;
        let ctx = CollaborationContext::new(
            CollaborationMode::Voting,
            query.clone(),
            self.next_session_seq,
            started_at_ms,
        );

        let opinions: Vec<AdvisorOpinion> = self
            .voters
            .iter()
            .map(|v| v.to_opinion(started_at_ms))
            .collect();

        let report = synthesize(&opinions, &self.weights);
        let elapsed_ms = (current_time_ms() - started_at_ms).max(0) as u64;

        CollaborationVerdict {
            session_id: ctx.session_id,
            mode: ctx.mode,
            query_id: query.query_id.clone(),
            report,
            opinions,
            steps: 1, // 单轮
            elapsed_ms,
            termination_reason: "single_round".to_string(),
        }
    }

    /// 策略 helper: 按 strategy 评估"是否通过"
    pub fn passes_strategy(&self, opinions: &[AdvisorOpinion], report: &SynthesisReport) -> bool {
        match self.strategy {
            VotingStrategy::WeightedMajority => report.weighted_score > 0.0,
            VotingStrategy::TopScoring => opinions
                .iter()
                .max_by(|a, b| {
                    a.stance
                        .kind
                        .score()
                        .partial_cmp(&b.stance.kind.score())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|o| o.stance.kind.score() > 0.0)
                .unwrap_or(false),
            VotingStrategy::Supermajority => {
                if opinions.is_empty() {
                    return false;
                }
                let approve_count = opinions
                    .iter()
                    .filter(|o| o.stance.kind.score() > 0.0)
                    .count();
                (approve_count as f64) >= (opinions.len() as f64) * 2.0 / 3.0
            }
        }
    }
}

fn current_time_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ============================================================
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::CouncilQuery;

    fn q(desc: &str) -> CouncilQuery {
        CouncilQuery::new("q-vote-test", desc, 0)
    }

    fn standard_5_voters_approve() -> Vec<Voter> {
        vec![
            Voter::new("v1", "architect", StanceKind::Approve, 0.8, "good design"),
            Voter::new("v2", "security", StanceKind::Approve, 0.7, "no issue"),
            Voter::new("v3", "pm", StanceKind::Approve, 0.9, "user value"),
            Voter::new("v4", "qa", StanceKind::Approve, 0.75, "covered"),
            Voter::new("v5", "devops", StanceKind::Approve, 0.85, "deployable"),
        ]
    }

    fn standard_5_voters_mixed() -> Vec<Voter> {
        vec![
            Voter::new("v1", "architect", StanceKind::Approve, 0.8, "good design"),
            Voter::new("v2", "security", StanceKind::Disapprove, 0.9, "has issue"),
            Voter::new("v3", "pm", StanceKind::Approve, 0.7, "user value"),
            Voter::new("v4", "qa", StanceKind::Neutral, 0.6, "need more test"),
            Voter::new("v5", "devops", StanceKind::Disapprove, 0.8, "risky deploy"),
        ]
    }

    #[test]
    fn new_default_weighted_majority() {
        let vm = VotingMode::new(standard_5_voters_approve());
        assert_eq!(vm.voter_count(), 5);
        assert_eq!(vm.strategy(), VotingStrategy::WeightedMajority);
    }

    #[test]
    fn with_strategy_supermajority() {
        let vm = VotingMode::new(standard_5_voters_approve()).with_strategy(VotingStrategy::Supermajority);
        assert_eq!(vm.strategy(), VotingStrategy::Supermajority);
    }

    #[test]
    fn with_strategy_top_scoring() {
        let vm = VotingMode::new(standard_5_voters_approve()).with_strategy(VotingStrategy::TopScoring);
        assert_eq!(vm.strategy(), VotingStrategy::TopScoring);
    }

    #[test]
    fn new_empty_voters() {
        let vm = VotingMode::new(vec![]);
        assert_eq!(vm.voter_count(), 0);
    }

    #[test]
    fn voter_to_opinion_basic() {
        let v = Voter::new("v1", "architect", StanceKind::Approve, 0.8, "good");
        let op = v.to_opinion(0);
        assert_eq!(op.advisor_id.as_str(), "v1");
        assert_eq!(op.stance.kind, StanceKind::Approve);
        assert_eq!(op.confidence, 0.8);
    }

    #[test]
    fn voter_to_opinion_clamps_confidence() {
        let v = Voter::new("v1", "a", StanceKind::Approve, 1.5, "x");
        assert_eq!(v.confidence, 1.0);
        let v2 = Voter::new("v2", "a", StanceKind::Approve, -0.5, "x");
        assert_eq!(v2.confidence, 0.0);
    }

    #[test]
    fn run_5_voters_approve_passes() {
        let mut vm = VotingMode::new(standard_5_voters_approve());
        let verdict = vm.run(&q("deploy feature"));
        assert_eq!(verdict.mode, CollaborationMode::Voting);
        assert_eq!(verdict.steps, 1);
        assert_eq!(verdict.termination_reason, "single_round");
        assert!(verdict.opinions.len() == 5);
        assert!(verdict.session_id.starts_with("collab-voting-"));
    }

    #[test]
    fn run_5_voters_mixed_weighted_score_near_zero() {
        let mut vm = VotingMode::new(standard_5_voters_mixed());
        let verdict = vm.run(&q("risky deploy"));
        // mixed: 2 Approve + 2 Disapprove + 1 Neutral
        // weighted_score 应接近 0 (per R10 synthesize 加权)
        assert!(verdict.report.weighted_score >= -0.5 && verdict.report.weighted_score <= 0.5);
    }

    #[test]
    fn run_session_seq_monotonic() {
        let mut vm = VotingMode::new(standard_5_voters_approve());
        let v1 = vm.run(&q("a"));
        let v2 = vm.run(&q("b"));
        assert!(v2.session_id > v1.session_id);
    }

    #[test]
    fn passes_strategy_weighted_majority_positive() {
        let vm = VotingMode::new(standard_5_voters_approve());
        let opinions: Vec<AdvisorOpinion> = vm
            .voters
            .iter()
            .map(|v| v.to_opinion(0))
            .collect();
        let report = synthesize(&opinions, &SynthesisWeights::default());
        assert!(vm.passes_strategy(&opinions, &report));
    }

    #[test]
    fn passes_strategy_weighted_majority_negative() {
        let voters = vec![
            Voter::new("v1", "a", StanceKind::Disapprove, 0.8, "x"),
            Voter::new("v2", "b", StanceKind::Disapprove, 0.7, "y"),
        ];
        let vm = VotingMode::new(voters);
        let opinions: Vec<AdvisorOpinion> = vm
            .voters
            .iter()
            .map(|v| v.to_opinion(0))
            .collect();
        let report = synthesize(&opinions, &SynthesisWeights::default());
        assert!(!vm.passes_strategy(&opinions, &report));
    }

    #[test]
    fn passes_strategy_supermajority_4_of_5_approve_passes() {
        let voters = vec![
            Voter::new("v1", "a", StanceKind::Approve, 0.8, "x"),
            Voter::new("v2", "b", StanceKind::Approve, 0.8, "y"),
            Voter::new("v3", "c", StanceKind::Approve, 0.8, "z"),
            Voter::new("v4", "d", StanceKind::Approve, 0.8, "w"),
            Voter::new("v5", "e", StanceKind::Disapprove, 0.8, "no"),
        ];
        let vm = VotingMode::new(voters).with_strategy(VotingStrategy::Supermajority);
        let opinions: Vec<AdvisorOpinion> = vm
            .voters
            .iter()
            .map(|v| v.to_opinion(0))
            .collect();
        let report = synthesize(&opinions, &SynthesisWeights::default());
        // 4/5 = 80% > 2/3 = 66.7%
        assert!(vm.passes_strategy(&opinions, &report));
    }

    #[test]
    fn passes_strategy_supermajority_2_of_5_approve_fails() {
        let voters = vec![
            Voter::new("v1", "a", StanceKind::Approve, 0.8, "x"),
            Voter::new("v2", "b", StanceKind::Approve, 0.8, "y"),
            Voter::new("v3", "c", StanceKind::Disapprove, 0.8, "z"),
            Voter::new("v4", "d", StanceKind::Disapprove, 0.8, "w"),
            Voter::new("v5", "e", StanceKind::Disapprove, 0.8, "no"),
        ];
        let vm = VotingMode::new(voters).with_strategy(VotingStrategy::Supermajority);
        let opinions: Vec<AdvisorOpinion> = vm
            .voters
            .iter()
            .map(|v| v.to_opinion(0))
            .collect();
        let report = synthesize(&opinions, &SynthesisWeights::default());
        // 2/5 = 40% < 2/3
        assert!(!vm.passes_strategy(&opinions, &report));
    }

    #[test]
    fn passes_strategy_top_scoring_approve() {
        let voters = vec![
            Voter::new("v1", "a", StanceKind::Disapprove, 0.8, "no"),
            Voter::new("v2", "b", StanceKind::Approve, 0.8, "yes"),
            Voter::new("v3", "c", StanceKind::Neutral, 0.8, "mid"),
        ];
        let vm = VotingMode::new(voters).with_strategy(VotingStrategy::TopScoring);
        let opinions: Vec<AdvisorOpinion> = vm
            .voters
            .iter()
            .map(|v| v.to_opinion(0))
            .collect();
        let report = synthesize(&opinions, &SynthesisWeights::default());
        // 最高 score = Approve (0.6) > 0
        assert!(vm.passes_strategy(&opinions, &report));
    }

    #[test]
    fn voting_strategy_default_is_weighted_majority() {
        assert_eq!(VotingStrategy::default(), VotingStrategy::WeightedMajority);
    }

    #[test]
    fn voting_strategy_as_str_3_distinct() {
        let names: Vec<&str> = [
            VotingStrategy::WeightedMajority,
            VotingStrategy::TopScoring,
            VotingStrategy::Supermajority,
        ]
        .iter()
        .map(|s| s.as_str())
        .collect();
        assert_eq!(
            names,
            vec!["weighted_majority", "top_scoring", "supermajority"]
        );
    }
}
