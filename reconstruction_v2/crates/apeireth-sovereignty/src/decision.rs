//! Decision: 主权决策请求与产出
//!
//! **设计**:
//! - `SovereigntyDomain` 标识请求来源域 (Thought / Proposal / Action)
//! - `DecisionRequest` 是输入 (含 risk_level, action_description, timestamps)
//! - `Decision` 是产出 (Approved / Rejected / Pending)
//! - `DecisionOutcome` 含完整追溯 (签名 / 决策时间戳 / 决策理由)

use serde::{Deserialize, Serialize};
use std::fmt;

/// 主权决策请求来源域 (三域分离对应)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SovereigntyDomain {
    /// 思维域 (自由, 无强制点)
    Thought,
    /// 提案域 (过 5 哲学键 — E/S/A/M/O 原则洋葱)
    Proposal,
    /// 行动域 (过 6 权限洋葱 — L0-L5)
    Action,
}

impl fmt::Display for SovereigntyDomain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Thought => "thought",
            Self::Proposal => "proposal",
            Self::Action => "action",
        };
        f.write_str(s)
    }
}

/// 主权决策请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionRequest {
    /// 唯一请求 ID
    pub id: String,
    /// 请求来源域
    pub domain: SovereigntyDomain,
    /// 动作描述
    pub action_description: String,
    /// 风险分级 (low / medium / high / nuclear)
    pub risk_level: String,
    /// 提交时间 (epoch ms)
    pub submitted_at_ms: i64,
    /// 引用历史 ID
    pub history_refs: Vec<String>,
}

impl DecisionRequest {
    pub fn new(
        id: impl Into<String>,
        domain: SovereigntyDomain,
        action_description: impl Into<String>,
        submitted_at_ms: i64,
    ) -> Self {
        Self {
            id: id.into(),
            domain,
            action_description: action_description.into(),
            risk_level: "low".into(),
            submitted_at_ms,
            history_refs: Vec::new(),
        }
    }

    pub fn with_risk(mut self, risk: impl Into<String>) -> Self {
        self.risk_level = risk.into();
        self
    }

    pub fn with_history_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.history_refs.push(ref_id.into());
        self
    }
}

/// 主权决策产出。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Decision {
    Approved {
        reason: String,
        decided_at_ms: i64,
        signatures: Vec<String>,
    },
    Rejected {
        reason: String,
        decided_at_ms: i64,
        signatures: Vec<String>,
    },
    Pending {
        reason: String,
        decided_at_ms: i64,
        review_at_ms: i64,
    },
}

impl Decision {
    pub fn is_approved(&self) -> bool { matches!(self, Self::Approved { .. }) }
    pub fn is_rejected(&self) -> bool { matches!(self, Self::Rejected { .. }) }
    pub fn is_pending(&self) -> bool { matches!(self, Self::Pending { .. }) }
    pub fn decided_at_ms(&self) -> i64 {
        match self {
            Self::Approved { decided_at_ms, .. } => *decided_at_ms,
            Self::Rejected { decided_at_ms, .. } => *decided_at_ms,
            Self::Pending { decided_at_ms, .. } => *decided_at_ms,
        }
    }
    pub fn signatures(&self) -> &[String] {
        match self {
            Self::Approved { signatures, .. } => signatures,
            Self::Rejected { signatures, .. } => signatures,
            Self::Pending { .. } => &[],
        }
    }
    pub fn reason(&self) -> &str {
        match self {
            Self::Approved { reason, .. } => reason,
            Self::Rejected { reason, .. } => reason,
            Self::Pending { reason, .. } => reason,
        }
    }
}

/// 主权决策产出 (完整追溯).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionOutcome {
    pub request_id: String,
    pub domain: SovereigntyDomain,
    pub decision: Decision,
    pub issued_at_ms: i64,
}

impl DecisionOutcome {
    pub fn new(
        request_id: impl Into<String>,
        domain: SovereigntyDomain,
        decision: Decision,
        issued_at_ms: i64,
    ) -> Self {
        Self { request_id: request_id.into(), domain, decision, issued_at_ms }
    }
    pub fn is_allowed(&self) -> bool { self.decision.is_approved() }
    pub fn is_rejected(&self) -> bool { self.decision.is_rejected() }
}

impl fmt::Display for DecisionOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DecisionOutcome(req={}, domain={}, {:?})", self.request_id, self.domain, self.decision)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn decision_request_new_defaults_low_risk() {
        let r = DecisionRequest::new("r", SovereigntyDomain::Action, "x", 1000);
        assert_eq!(r.risk_level, "low");
        assert_eq!(r.submitted_at_ms, 1000);
    }
    #[test] fn decision_outcome_is_allowed() {
        let d = Decision::Approved { reason: "ok".into(), decided_at_ms: 0, signatures: vec![] };
        let o = DecisionOutcome::new("r", SovereigntyDomain::Action, d, 0);
        assert!(o.is_allowed());
        assert!(!o.is_rejected());
    }
    #[test] fn decision_pending_and_rejected_predicates() {
        let p = Decision::Pending { reason: "x".into(), decided_at_ms: 0, review_at_ms: 100 };
        assert!(p.is_pending());
        let r = Decision::Rejected { reason: "x".into(), decided_at_ms: 0, signatures: vec![] };
        assert!(r.is_rejected());
    }
}
