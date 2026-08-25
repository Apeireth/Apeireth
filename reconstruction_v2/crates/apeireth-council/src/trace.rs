//! R25 D-3: Reasoning trace 可视化 (per v2.0 strategy §2B "加 reasoning trace 可视化")
//!
//! **职责**:
//! - 捕获每步协作 step (per Planner+Executor / Debate / Voting / Hierarchical)
//! - 输出 3 格式: Pretty (人类可读) / JSON (机器) / JSONL (claude_code trace 风格)
//!
//! **借鉴锚** (S-1):
//! - LangGraph `MemorySaver.get_tuple()` → graph state timeline
//! - VCP `vcpLoop/traceLog.js` step-by-step log
//! - AutoGen `GroupChat.messages` transcript
//! - Anthropic `claude_code/trace.jsonl` 风格 (每行 1 step JSON)
//!
//! **0 漂移**:
//! - 0 改 R10 既有
//! - 0 引入 I/O / 网络
//! - 仅序列化/反序列化现有 struct (SynthesisReport + AdvisorOpinion + CollaborationVerdict)

#![deny(unsafe_code)]

use crate::advisor::AdvisorOpinion;
use crate::advisor::StanceKind;
use crate::collaboration::types::{CollaborationMode, CollaborationVerdict};
use crate::synthesis::SynthesisReport;
use serde::{Deserialize, Serialize};

/// 单步协作产出
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceStep {
    /// 步号 (0-based, per CollaborationMode 内部 step)
    pub step_id: u32,
    /// 协作模式
    pub mode: CollaborationMode,
    /// 角色 (e.g. "planner" / "executor.1" / "debate.member.2" / "voter.3" / "root.cto" / "sub.1")
    pub actor: String,
    /// 动作 (e.g. "plan" / "execute" / "vote" / "delegate" / "synthesize" / "consensus")
    pub action: String,
    /// 该步输入 (简短)
    pub input: String,
    /// 该步输出 (简短)
    pub output: String,
    /// 立场 (per 6 种 StanceKind + None)
    pub stance: Option<StanceKind>,
    /// 耗时 (ms, 该步耗时)
    pub elapsed_ms: u64,
}

/// 完整 trace 报告 — 1 次协作执行的完整时间线
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceReport {
    /// session ID
    pub session_id: String,
    /// 模式
    pub mode: CollaborationMode,
    /// query 描述
    pub query: String,
    /// 每步 trace
    pub steps: Vec<TraceStep>,
    /// 最终 verdict
    pub final_verdict: SynthesisReport,
}

impl TraceReport {
    /// 从 `CollaborationVerdict` + 协作详情构造 trace report
    ///
    /// **0 漂移**: 0 改 `CollaborationVerdict`, 仅从 `opinions` + `steps` 推 trace
    pub fn from_verdict(verdict: &CollaborationVerdict) -> Self {
        let steps = build_steps_from_opinions(verdict);
        Self {
            session_id: verdict.session_id.clone(),
            mode: verdict.mode,
            query: String::new(), // 由 caller 填 (verdict 0 透传 query.description)
            steps,
            final_verdict: verdict.report.clone(),
        }
    }

    /// 设置 query 描述 (chainable)
    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// 步数
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Pretty-print 人类可读 (per 派活单 "3 advisor 协作任务 + trace 打印")
    pub fn to_pretty_print(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("=== Council Trace: {} ===\n", self.session_id));
        out.push_str(&format!(
            "Mode: {}\n",
            pretty_mode_name(self.mode, self.steps.len())
        ));
        if !self.query.is_empty() {
            out.push_str(&format!("Query: {}\n", self.query));
        }
        out.push('\n');

        for step in &self.steps {
            out.push_str(&format!(
                "[Step {}] {}\n",
                step.step_id,
                step.action.to_uppercase()
            ));
            out.push_str(&format!("  Actor: {}\n", step.actor));
            if !step.input.is_empty() {
                out.push_str(&format!("  Input: {}\n", truncate(&step.input, 80)));
            }
            if !step.output.is_empty() {
                out.push_str(&format!("  Output: {}\n", truncate(&step.output, 80)));
            }
            if let Some(stance) = step.stance {
                out.push_str(&format!("  Stance: {:?}\n", stance));
            }
            out.push_str(&format!("  Elapsed: {}ms\n", step.elapsed_ms));
            out.push('\n');
        }

        out.push_str("=== Final Verdict ===\n");
        out.push_str(&format!(
            "weighted_score: {:.2}\n",
            self.final_verdict.weighted_score
        ));
        out.push_str(&format!(
            "stance: {:?}\n",
            self.final_verdict.aggregated_stance.kind
        ));
        out.push_str(&format!("held: {}\n", self.final_verdict.is_held()));
        out.push_str(&format!(
            "opinion_count: {}\n",
            self.final_verdict.opinion_count
        ));
        out
    }

    /// JSON 序列化 (整张 trace report)
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"))
    }

    /// JSONL 序列化 (claude_code trace 风格, 每行 1 step)
    pub fn to_step_jsonl(&self) -> String {
        let mut out = String::new();
        for step in &self.steps {
            let json = serde_json::to_string(step)
                .unwrap_or_else(|e| format!("{{\"error\": \"step serialization failed: {e}\"}}"));
            out.push_str(&json);
            out.push('\n');
        }
        out
    }
}

/// 从 `CollaborationVerdict` 的 opinions 推 steps
///
/// **0 漂移**: 复用 verdict.opinions 0 改, 1:1 推 step
fn build_steps_from_opinions(verdict: &CollaborationVerdict) -> Vec<TraceStep> {
    match verdict.mode {
        CollaborationMode::PlannerExecutor => {
            // Planner+Executor: 1 plan step + N executor steps
            let mut steps = Vec::new();
            steps.push(TraceStep {
                step_id: 0,
                mode: verdict.mode,
                actor: "planner".to_string(),
                action: "plan".to_string(),
                input: String::new(), // verdict 0 透传 plan input
                output: format!("Plan: {} steps", verdict.steps),
                stance: None,
                elapsed_ms: 0,
            });
            for (i, op) in verdict.opinions.iter().enumerate() {
                steps.push(TraceStep {
                    step_id: (i + 1) as u32,
                    mode: verdict.mode,
                    actor: format!("executor.{}", i + 1),
                    action: "execute".to_string(),
                    input: op.reasoning.clone(),
                    output: format!("{:?}", op.stance.kind),
                    stance: Some(op.stance.kind),
                    elapsed_ms: 0, // verdict 0 透传 per-step elapsed
                });
            }
            steps
        }
        CollaborationMode::Debate => {
            // Debate: 1 步 = 1 member opinion (per round)
            let mut steps = Vec::new();
            for (i, op) in verdict.opinions.iter().enumerate() {
                steps.push(TraceStep {
                    step_id: i as u32,
                    mode: verdict.mode,
                    actor: format!("debate.member.{}", i + 1),
                    action: "debate".to_string(),
                    input: op.reasoning.clone(),
                    output: format!("{:?}", op.stance.kind),
                    stance: Some(op.stance.kind),
                    elapsed_ms: 0,
                });
            }
            steps
        }
        CollaborationMode::Voting => {
            // Voting: 1 步 = 1 voter opinion
            let mut steps = Vec::new();
            for (i, op) in verdict.opinions.iter().enumerate() {
                steps.push(TraceStep {
                    step_id: i as u32,
                    mode: verdict.mode,
                    actor: format!("voter.{}", i + 1),
                    action: "vote".to_string(),
                    input: op.reasoning.clone(),
                    output: format!("{:?}", op.stance.kind),
                    stance: Some(op.stance.kind),
                    elapsed_ms: 0,
                });
            }
            steps
        }
        CollaborationMode::Hierarchical => {
            // Hierarchical: 1 步 = 1 sub opinion (root 自己不参与 vote)
            let mut steps = Vec::new();
            for (i, op) in verdict.opinions.iter().enumerate() {
                steps.push(TraceStep {
                    step_id: i as u32,
                    mode: verdict.mode,
                    actor: format!("sub.{}", i + 1),
                    action: "delegate".to_string(),
                    input: op.reasoning.clone(),
                    output: format!("{:?}", op.stance.kind),
                    stance: Some(op.stance.kind),
                    elapsed_ms: 0,
                });
            }
            steps
        }
    }
}

/// Pretty mode name
fn pretty_mode_name(mode: CollaborationMode, steps: usize) -> String {
    let base = match mode {
        CollaborationMode::PlannerExecutor => "Planner+Executor",
        CollaborationMode::Debate => "Debate",
        CollaborationMode::Voting => "Voting",
        CollaborationMode::Hierarchical => "Hierarchical",
    };
    format!("{base} ({steps} steps)")
}

/// 截断过长 string (per pretty-print 80 char 上限)
fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{truncated}...")
    }
}

/// 从 4 模式 verdict 提取 trace (helper for 集成)
pub fn trace_from_collaboration(verdict: &CollaborationVerdict) -> TraceReport {
    TraceReport::from_verdict(verdict)
}

/// 从单条 opinion 提取 TraceStep (helper for 单步 trace)
pub fn trace_step_from_opinion(
    step_id: u32,
    mode: CollaborationMode,
    actor: impl Into<String>,
    action: impl Into<String>,
    opinion: &AdvisorOpinion,
) -> TraceStep {
    TraceStep {
        step_id,
        mode,
        actor: actor.into(),
        action: action.into(),
        input: opinion.reasoning.clone(),
        output: format!("{:?}", opinion.stance.kind),
        stance: Some(opinion.stance.kind),
        elapsed_ms: 0,
    }
}

// ============================================================
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisor::{AdvisorId, Stance, StanceKind};
    use crate::deliberation::CouncilQuery;
    use crate::synthesis::{SynthesisReport, SynthesisWeights};

    fn q(id: &str) -> CouncilQuery {
        CouncilQuery::new(id, "design auth system", 0)
    }

    fn opinion(stance: StanceKind, reasoning: &str) -> AdvisorOpinion {
        AdvisorOpinion::new(
            AdvisorId::new("test"),
            Stance::new(stance, "test stance"),
            0.8,
            reasoning,
            0,
        )
    }

    fn verdict_3_steps_approve() -> CollaborationVerdict {
        let opinions = vec![
            opinion(StanceKind::Approve, "step 1: design"),
            opinion(StanceKind::Approve, "step 2: implement"),
            opinion(StanceKind::Approve, "step 3: verify"),
        ];
        let report = crate::synthesis::synthesize(&opinions, &SynthesisWeights::default());
        CollaborationVerdict {
            session_id: "collab-test-000001".to_string(),
            mode: CollaborationMode::PlannerExecutor,
            query_id: "q1".to_string(),
            report,
            opinions,
            steps: 3,
            elapsed_ms: 100,
            termination_reason: "plan_completed".to_string(),
        }
    }

    #[test]
    fn trace_step_new_basic() {
        let step = TraceStep {
            step_id: 0,
            mode: CollaborationMode::PlannerExecutor,
            actor: "planner".to_string(),
            action: "plan".to_string(),
            input: "design".to_string(),
            output: "plan: 3 steps".to_string(),
            stance: None,
            elapsed_ms: 10,
        };
        assert_eq!(step.step_id, 0);
        assert_eq!(step.actor, "planner");
    }

    #[test]
    fn trace_step_serde_round_trip() {
        let step = TraceStep {
            step_id: 1,
            mode: CollaborationMode::Voting,
            actor: "voter.1".to_string(),
            action: "vote".to_string(),
            input: "approve".to_string(),
            output: "Approve".to_string(),
            stance: Some(StanceKind::Approve),
            elapsed_ms: 5,
        };
        let json = serde_json::to_string(&step).unwrap();
        let back: TraceStep = serde_json::from_str(&json).unwrap();
        assert_eq!(step, back);
    }

    #[test]
    fn trace_report_from_verdict_planner_executor_4_steps() {
        let v = verdict_3_steps_approve();
        let report = TraceReport::from_verdict(&v);
        // 1 plan + 3 executor = 4 steps
        assert_eq!(report.step_count(), 4);
        assert_eq!(report.steps[0].action, "plan");
        assert_eq!(report.steps[0].actor, "planner");
        assert_eq!(report.steps[1].action, "execute");
        assert_eq!(report.steps[1].actor, "executor.1");
        assert_eq!(report.steps[2].actor, "executor.2");
        assert_eq!(report.steps[3].actor, "executor.3");
    }

    #[test]
    fn trace_report_from_verdict_voting_3_steps() {
        let opinions = vec![
            opinion(StanceKind::Approve, "v1"),
            opinion(StanceKind::Disapprove, "v2"),
            opinion(StanceKind::Neutral, "v3"),
        ];
        let report = crate::synthesis::synthesize(&opinions, &SynthesisWeights::default());
        let v = CollaborationVerdict {
            session_id: "collab-vote-001".to_string(),
            mode: CollaborationMode::Voting,
            query_id: "q1".to_string(),
            report,
            opinions,
            steps: 1,
            elapsed_ms: 50,
            termination_reason: "single_round".to_string(),
        };
        let report = TraceReport::from_verdict(&v);
        assert_eq!(report.step_count(), 3);
        assert_eq!(report.steps[0].action, "vote");
        assert_eq!(report.steps[0].actor, "voter.1");
    }

    #[test]
    fn trace_report_from_verdict_debate_3_steps() {
        let opinions = vec![
            opinion(StanceKind::Approve, "r1 m1"),
            opinion(StanceKind::Disapprove, "r1 m2"),
            opinion(StanceKind::Neutral, "r1 m3"),
        ];
        let report = crate::synthesis::synthesize(&opinions, &SynthesisWeights::default());
        let v = CollaborationVerdict {
            session_id: "collab-debate-001".to_string(),
            mode: CollaborationMode::Debate,
            query_id: "q1".to_string(),
            report,
            opinions,
            steps: 1,
            elapsed_ms: 200,
            termination_reason: "max_rounds".to_string(),
        };
        let report = TraceReport::from_verdict(&v);
        assert_eq!(report.step_count(), 3);
        assert_eq!(report.steps[0].actor, "debate.member.1");
        assert_eq!(report.steps[0].action, "debate");
    }

    #[test]
    fn trace_report_from_verdict_hierarchical_2_steps() {
        let opinions = vec![
            opinion(StanceKind::Approve, "sub 1: design"),
            opinion(StanceKind::Disapprove, "sub 2: risk"),
        ];
        let report = crate::synthesis::synthesize(&opinions, &SynthesisWeights::default());
        let v = CollaborationVerdict {
            session_id: "collab-hier-001".to_string(),
            mode: CollaborationMode::Hierarchical,
            query_id: "q1".to_string(),
            report,
            opinions,
            steps: 2,
            elapsed_ms: 30,
            termination_reason: "delegation_completed".to_string(),
        };
        let report = TraceReport::from_verdict(&v);
        assert_eq!(report.step_count(), 2);
        assert_eq!(report.steps[0].actor, "sub.1");
        assert_eq!(report.steps[1].actor, "sub.2");
    }

    #[test]
    fn trace_report_with_query() {
        let v = verdict_3_steps_approve();
        let report = TraceReport::from_verdict(&v).with_query(q("q1").description);
        assert_eq!(report.query, "design auth system");
    }

    #[test]
    fn trace_report_to_pretty_print_contains_key_fields() {
        let v = verdict_3_steps_approve();
        let report = TraceReport::from_verdict(&v).with_query("deploy auth system");
        let pp = report.to_pretty_print();
        assert!(pp.contains("Council Trace"));
        assert!(pp.contains("collab-test-000001"));
        assert!(pp.contains("Planner+Executor"));
        assert!(pp.contains("deploy auth system"));
        assert!(pp.contains("[Step 0] PLAN"));
        assert!(pp.contains("[Step 1] EXECUTE"));
        assert!(pp.contains("Final Verdict"));
        assert!(pp.contains("weighted_score"));
        assert!(pp.contains("Approve"));
    }

    #[test]
    fn trace_report_to_json_contains_all_fields() {
        let v = verdict_3_steps_approve();
        let report = TraceReport::from_verdict(&v);
        let json = report.to_json();
        assert!(json.contains("session_id"));
        assert!(json.contains("collab-test-000001"));
        // mode 序列化为 enum 名字 "PlannerExecutor" (snake_case 在 as_str, JSON 用 enum 名字)
        assert!(json.contains("PlannerExecutor"));
        assert!(json.contains("steps"));
    }

    #[test]
    fn trace_report_to_jsonl_one_step_per_line() {
        let v = verdict_3_steps_approve();
        let report = TraceReport::from_verdict(&v);
        let jsonl = report.to_step_jsonl();
        let lines: Vec<&str> = jsonl.lines().collect();
        // 4 steps = 4 lines
        assert_eq!(lines.len(), 4);
        for line in &lines {
            assert!(line.contains("step_id"));
            assert!(line.contains("actor"));
            assert!(line.contains("action"));
        }
    }

    #[test]
    fn trace_step_from_opinion_helper() {
        let op = opinion(StanceKind::Approve, "test reasoning");
        let step = trace_step_from_opinion(0, CollaborationMode::Voting, "voter.1", "vote", &op);
        assert_eq!(step.step_id, 0);
        assert_eq!(step.actor, "voter.1");
        assert_eq!(step.action, "vote");
        assert_eq!(step.stance, Some(StanceKind::Approve));
        assert_eq!(step.input, "test reasoning");
    }

    #[test]
    fn trace_from_collaboration_helper() {
        let v = verdict_3_steps_approve();
        let report = trace_from_collaboration(&v);
        assert_eq!(report.session_id, "collab-test-000001");
    }

    #[test]
    fn truncate_basic() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world this is long", 10), "hello worl...");
    }

    #[test]
    fn pretty_mode_name_4_modes() {
        assert_eq!(
            pretty_mode_name(CollaborationMode::PlannerExecutor, 3),
            "Planner+Executor (3 steps)"
        );
        assert_eq!(
            pretty_mode_name(CollaborationMode::Voting, 1),
            "Voting (1 steps)"
        );
    }
}
