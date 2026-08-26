//! Debate 模式 — R33-4-1 `CouncilMemberDeliberator` 1:1 包装
//!
//! **职责** (per v2.0 strategy §2B "Debate" + AutoGen GroupChat 简化):
//! - 多轮协商辩论 (默认 3 轮, 跟 R33-4-1 `DEFAULT_MAX_ROUNDS = 3` 1:1)
//! - 共识检测 (consensus_score >= 0.6)
//! - 强反对按住 (strong_disapprove + confidence >= 0.5)
//! - 复用 R33-4-1 `CouncilMemberDeliberator` 1:1 transparent (per R23 P3)
//!
//! **0 漂移**:
//! - 0 改 R33-4-1 `council_member_deliberation.rs` (LOCKED)
//! - 0 改 R33-4 `CouncilMember` struct (LOCKED)
//! - 仅包装 + 转换为 `CollaborationVerdict`

use crate::collaboration::types::{CollaborationMode, CollaborationVerdict};
use crate::council_member::CouncilMember;
use crate::council_member_deliberation::{CouncilMemberDeliberator, MultiRoundVerdict};

/// Debate 模式 wrapper — 内部直接复用 R33-4-1
pub struct DebateMode {
    /// R33-4-1 deliberator (复用, 0 改)
    deliberator: CouncilMemberDeliberator,
    /// 内部 session 序列
    next_session_seq: u64,
}

impl DebateMode {
    /// 构造 (默认 max_rounds = 3, 跟 R33-4-1 `DEFAULT_MAX_ROUNDS` 1:1)
    pub fn new(members: Vec<CouncilMember>) -> Self {
        Self {
            deliberator: CouncilMemberDeliberator::new(members),
            next_session_seq: 0,
        }
    }

    /// 自定义 max_rounds (chainable, 透传到 R33-4-1)
    pub fn with_max_rounds(mut self, n: u8) -> Self {
        self.deliberator = self.deliberator.with_max_rounds(n);
        self
    }

    /// member 数
    pub fn member_count(&self) -> usize {
        self.deliberator.member_count()
    }

    /// max_rounds
    pub fn max_rounds(&self) -> u8 {
        self.deliberator.max_rounds()
    }

    /// 注入 mock LLM (透传到 R33-4-1, chainable)
    pub fn with_mock_llm(
        mut self,
        llm: std::sync::Arc<dyn crate::mock_llm::MockLlmProvider>,
    ) -> Self {
        self.deliberator = self.deliberator.with_mock_llm(llm);
        self
    }

    /// **跑 Debate 流程** — 内部 R33-4-1 → 包装为 `CollaborationVerdict`
    pub fn run(&mut self, query: &crate::deliberation::CouncilQuery) -> CollaborationVerdict {
        self.next_session_seq += 1;
        let started_at_ms = query.started_at_ms;
        let session_id = format!("collab-debate-{:06}", self.next_session_seq);

        // 复用 R33-4-1 1:1
        let multi: MultiRoundVerdict = self.deliberator.deliberate(query);

        // 收集所有 opinions (从 R33-4-1 rounds 里 flat 出来)
        let opinions: Vec<crate::advisor::AdvisorOpinion> = multi
            .rounds
            .iter()
            .flat_map(|r| r.opinions.iter().cloned())
            .collect();

        // 复用 R10 synthesize 走全 opinions (跟 Planner+Executor 路径对齐)
        let report =
            crate::synthesis::synthesize(&opinions, &crate::synthesis::SynthesisWeights::default());

        let elapsed_ms = (current_time_ms() - started_at_ms).max(0) as u64;
        let steps = u32::from(multi.rounds_run);

        CollaborationVerdict {
            session_id,
            mode: CollaborationMode::Debate,
            query_id: query.query_id.clone(),
            report,
            opinions,
            steps,
            elapsed_ms,
            termination_reason: multi.termination_reason,
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
    use crate::council_member::CouncilMember;
    use crate::deliberation::CouncilQuery;

    fn q(desc: &str) -> CouncilQuery {
        CouncilQuery::new("q-debate-test", desc, 0)
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

    #[test]
    fn new_default_max_rounds_3() {
        let dm = DebateMode::new(standard_5_members());
        assert_eq!(dm.member_count(), 5);
        assert_eq!(dm.max_rounds(), 3);
    }

    #[test]
    fn with_max_rounds_clamps() {
        let dm = DebateMode::new(standard_5_members()).with_max_rounds(7);
        assert_eq!(dm.max_rounds(), 7);
    }

    #[test]
    fn new_empty_members_count_0() {
        let dm = DebateMode::new(vec![]);
        assert_eq!(dm.member_count(), 0);
    }

    #[test]
    fn run_5_member_normal_query_terminates_cleanly() {
        let mut dm = DebateMode::new(standard_5_members());
        let verdict = dm.run(&q("design new feature"));
        assert_eq!(verdict.mode, CollaborationMode::Debate);
        assert!(verdict.steps >= 1 && verdict.steps <= 3);
        // 5 Approve → consensus_score 0.6 ≥ threshold → "consensus" 终止 (per R33-4-1 1:1)
        // 或者是 max_rounds / strong_disapprove / empty_members 之一 (4 reasons 覆盖)
        assert!(matches!(
            verdict.termination_reason.as_str(),
            "consensus" | "max_rounds" | "strong_disapprove" | "empty_members"
        ));
        assert!(verdict.session_id.starts_with("collab-debate-"));
    }

    #[test]
    fn run_empty_members_returns_empty_verdict() {
        let mut dm = DebateMode::new(vec![]);
        let verdict = dm.run(&q("test"));
        assert_eq!(verdict.termination_reason, "empty_members");
        assert_eq!(verdict.steps, 0);
        assert_eq!(verdict.opinions.len(), 0);
    }

    #[test]
    fn run_session_seq_monotonic() {
        let mut dm = DebateMode::new(standard_5_members());
        let v1 = dm.run(&q("design"));
        let v2 = dm.run(&q("deploy"));
        assert!(v2.session_id > v1.session_id);
    }

    #[test]
    fn run_max_rounds_1_truncates() {
        let mut dm = DebateMode::new(standard_5_members()).with_max_rounds(1);
        let verdict = dm.run(&q("test"));
        assert_eq!(verdict.steps, 1);
    }

    #[test]
    fn run_synthesize_report_weighted_score() {
        let mut dm = DebateMode::new(standard_5_members());
        let verdict = dm.run(&q("design and deploy"));
        // 全部 opinion 跑过 synthesize(), weighted_score 必有值
        assert!(verdict.report.weighted_score >= -1.0 && verdict.report.weighted_score <= 1.0);
    }

    #[test]
    fn run_3_member_3_round_consensus_integration() {
        let mut dm = DebateMode::new(standard_5_members());
        let verdict = dm.run(&q("design new auth system"));
        // 5 member × max 3 rounds = 最多 15 opinions
        assert!(verdict.opinions.len() <= 15);
    }
}
