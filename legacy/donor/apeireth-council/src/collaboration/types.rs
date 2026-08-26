//! 4 协作模式共享类型 — `CollaborationMode` + `CollaborationContext` + `CollaborationVerdict`

use crate::advisor::AdvisorOpinion;
use crate::deliberation::CouncilQuery;
use crate::synthesis::SynthesisReport;
use serde::{Deserialize, Serialize};
use std::fmt;

/// 4 协作模式 (per v2.0 strategy §2B)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CollaborationMode {
    /// Planner 拆 query → Executor 顺序执行 (LangGraph PlanAndExecute 借鉴)
    PlannerExecutor,
    /// 多轮协商辩论 (R33-4-1 `CouncilMemberDeliberator` 1:1 复用)
    Debate,
    /// 单轮加权投票 (AutoGen GroupChatManager 投票聚合借鉴)
    Voting,
    /// 主 + 子 advisor 委派 (OpenAI swarm handoff 借鉴)
    Hierarchical,
}

impl CollaborationMode {
    /// 4 模式稳定顺序 (编译期 hardcode)
    pub const ALL: [CollaborationMode; 4] = [
        Self::PlannerExecutor,
        Self::Debate,
        Self::Voting,
        Self::Hierarchical,
    ];

    /// 模式数 (编译期 hardcode)
    pub const COUNT: usize = 4;

    /// 模式名 (snake_case 风格, 给 trace / serde 用)
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PlannerExecutor => "planner_executor",
            Self::Debate => "debate",
            Self::Voting => "voting",
            Self::Hierarchical => "hierarchical",
        }
    }

    /// 模式是否多轮 (Planner+Executor 3 steps / Debate 3 rounds / Voting 1 / Hierarchical 1)
    pub const fn is_multi_round(self) -> bool {
        match self {
            Self::PlannerExecutor => true, // 3 steps
            Self::Debate => true,          // 3 rounds
            Self::Voting => false,         // 1 round
            Self::Hierarchical => false,   // 1 round (root + 2 sub)
        }
    }
}

impl fmt::Display for CollaborationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 协作上下文 — 一次协作执行的 state (per 模式独立消费)
#[derive(Debug, Clone)]
pub struct CollaborationContext {
    /// 内部 session ID (格式: `collab-<mode>-<seq>`)
    pub session_id: String,
    /// 协作模式
    pub mode: CollaborationMode,
    /// query (从 `CouncilQuery` 透传)
    pub query: CouncilQuery,
    /// 开始时间 (epoch ms)
    pub started_at_ms: i64,
}

impl CollaborationContext {
    /// 便利构造
    pub fn new(
        mode: CollaborationMode,
        query: CouncilQuery,
        session_seq: u64,
        started_at_ms: i64,
    ) -> Self {
        Self {
            session_id: format!("collab-{}-{:06}", mode.as_str(), session_seq),
            mode,
            query,
            started_at_ms,
        }
    }
}

/// 协作 verdict — 4 模式统一产出
#[derive(Debug, Clone)]
pub struct CollaborationVerdict {
    /// session ID
    pub session_id: String,
    /// 模式
    pub mode: CollaborationMode,
    /// query ID (从 `CouncilQuery` 透传)
    pub query_id: String,
    /// 综合报告 (per `synthesize()` 0 漂移)
    pub report: SynthesisReport,
    /// 全部 opinions (跨步/跨轮累积)
    pub opinions: Vec<AdvisorOpinion>,
    /// 内部步数 (Planner+Executor=3 steps / Debate=1..3 rounds / Voting=1 / Hierarchical=3)
    pub steps: u32,
    /// 耗时 (ms)
    pub elapsed_ms: u64,
    /// 终止原因:
    /// - Planner+Executor: "plan_completed" | "strong_disapprove"
    /// - Debate: "consensus" | "max_rounds" | "strong_disapprove" | "empty_members" (复用 R33-4-1 4 reasons)
    /// - Voting: "single_round"
    /// - Hierarchical: "delegation_completed" | "strong_disapprove"
    pub termination_reason: String,
}

impl CollaborationVerdict {
    /// 是否允许 (per `SynthesisReport` aggregated_stance.score > 0 + 0 strong_disapprove)
    pub fn is_allowed(&self) -> bool {
        self.report.weighted_score > 0.0 && !self.opinions.iter().any(|o| o.triggers_hold())
    }

    /// 是否被按住 (per `SynthesisReport::is_held()`)
    pub fn is_held(&self) -> bool {
        self.report.is_held()
    }
}

// ============================================================
// 编译期 hardcode (per 主哲学锚 #1 不漂移)
// ============================================================
const _: () = {
    assert!(CollaborationMode::COUNT == 4);
    assert!(CollaborationMode::ALL.len() == 4);
};

// ============================================================
// 单元测试
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::deliberation::CouncilQuery;

    #[test]
    fn mode_count_is_4() {
        assert_eq!(CollaborationMode::COUNT, 4);
    }

    #[test]
    fn mode_all_has_4_distinct() {
        let mut seen = std::collections::HashSet::new();
        for m in CollaborationMode::ALL {
            assert!(seen.insert(m), "duplicate mode: {m:?}");
        }
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn mode_as_str_4_distinct() {
        let names: Vec<&str> = CollaborationMode::ALL.iter().map(|m| m.as_str()).collect();
        assert_eq!(
            names,
            vec!["planner_executor", "debate", "voting", "hierarchical"]
        );
    }

    #[test]
    fn mode_display_equals_as_str() {
        for m in CollaborationMode::ALL {
            assert_eq!(format!("{m}"), m.as_str());
        }
    }

    #[test]
    fn mode_is_multi_round_classification() {
        assert!(CollaborationMode::PlannerExecutor.is_multi_round());
        assert!(CollaborationMode::Debate.is_multi_round());
        assert!(!CollaborationMode::Voting.is_multi_round());
        assert!(!CollaborationMode::Hierarchical.is_multi_round());
    }

    #[test]
    fn mode_serde_round_trip() {
        for m in CollaborationMode::ALL {
            let json = serde_json::to_string(&m).unwrap();
            let back: CollaborationMode = serde_json::from_str(&json).unwrap();
            assert_eq!(m, back);
        }
    }

    #[test]
    fn mode_hash_consistent() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for m in CollaborationMode::ALL {
            set.insert(m);
        }
        assert_eq!(set.len(), 4);
    }

    fn q(id: &str) -> CouncilQuery {
        CouncilQuery::new(id, "test query", 0)
    }

    #[test]
    fn collab_context_new_format() {
        let ctx = CollaborationContext::new(CollaborationMode::Debate, q("q1"), 1, 0);
        assert_eq!(ctx.session_id, "collab-debate-000001");
        assert_eq!(ctx.mode, CollaborationMode::Debate);
        assert_eq!(ctx.query.query_id, "q1");
    }

    #[test]
    fn collab_context_4_modes_4_prefixes() {
        for m in CollaborationMode::ALL {
            let ctx = CollaborationContext::new(m, q("q"), 42, 0);
            assert!(
                ctx.session_id
                    .starts_with(&format!("collab-{}-", m.as_str())),
                "session_id {} should start with collab-{}-",
                ctx.session_id,
                m.as_str()
            );
            assert!(ctx.session_id.ends_with("000042"));
        }
    }
}
