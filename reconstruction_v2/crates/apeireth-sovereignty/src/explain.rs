//! Sovereignty 可解释性 — 决策路径追踪

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const STAGE_KIND_COUNT_HARDCODE: usize = 5;
pub const K1_STRICT_CHECK_COUNT_HARDCODE: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageKind {
    RequestReceived, EvidenceCollected, AuthorityConsulted, VerdictReached, RationaleStated,
}

impl StageKind {
    pub fn as_str(self) -> &'static str {
        match self { Self::RequestReceived => "request_received", Self::EvidenceCollected => "evidence_collected", Self::AuthorityConsulted => "authority_consulted", Self::VerdictReached => "verdict_reached", Self::RationaleStated => "rationale_stated" }
    }
    pub fn is_terminal(self) -> bool { matches!(self, Self::VerdictReached | Self::RationaleStated) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VerdictOutcome { Approved, Rejected, PendingReview }

impl VerdictOutcome {
    pub fn as_str(self) -> &'static str {
        match self { Self::Approved => "approved", Self::Rejected => "rejected", Self::PendingReview => "pending_review" }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage {
    pub kind: StageKind,
    pub description: String,
    pub timestamp_ms: i64,
}

impl Stage {
    pub fn new(kind: StageKind, description: impl Into<String>) -> Self {
        Self { kind, description: description.into(), timestamp_ms: chrono::Utc::now().timestamp_millis() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionTrace {
    pub decision_id: String,
    pub initiator: String,
    pub stages: Vec<Stage>,
    pub verdict: Option<VerdictOutcome>,
    pub rationale: Option<String>,
    pub created_at_ms: i64,
}

impl DecisionTrace {
    pub fn new(decision_id: impl Into<String>, initiator: impl Into<String>) -> Self {
        Self { decision_id: decision_id.into(), initiator: initiator.into(), stages: Vec::new(), verdict: None, rationale: None, created_at_ms: chrono::Utc::now().timestamp_millis() }
    }
    pub fn try_push_stage(&mut self, kind: StageKind, description: impl Into<String>) -> Result<(), ExplainError> {
        if self.decision_id.trim().is_empty() { return Err(ExplainError::K1DecisionIdEmpty); }
        if self.verdict.is_some() { return Err(ExplainError::AlreadyFinalized); }
        if let Some(last) = self.stages.last() { if last.kind.is_terminal() { return Err(ExplainError::StageAfterTerminal); } }
        self.stages.push(Stage::new(kind, description));
        Ok(())
    }
    pub fn try_finalize(&mut self, verdict: VerdictOutcome, rationale: impl Into<String>) -> Result<(), ExplainError> {
        if self.decision_id.trim().is_empty() { return Err(ExplainError::K1DecisionIdEmpty); }
        if self.stages.len() < 2 { return Err(ExplainError::K1StagesTooFew { actual: self.stages.len(), min: 2 }); }
        let last_kind = self.stages.last().unwrap().kind;
        if !last_kind.is_terminal() { return Err(ExplainError::K1LastStageNotTerminal { actual: last_kind }); }
        self.verdict = Some(verdict);
        self.rationale = Some(rationale.into());
        Ok(())
    }
    pub fn is_complete(&self) -> bool { self.verdict.is_some() }
    pub fn len(&self) -> usize { self.stages.len() }
    pub fn is_empty(&self) -> bool { self.stages.is_empty() }
    pub fn validate_k1(&self) -> Result<(), ExplainError> {
        if self.decision_id.trim().is_empty() { return Err(ExplainError::K1DecisionIdEmpty); }
        if self.stages.len() < 2 { return Err(ExplainError::K1StagesTooFew { actual: self.stages.len(), min: 2 }); }
        let last_kind = self.stages.last().unwrap().kind;
        if !last_kind.is_terminal() { return Err(ExplainError::K1LastStageNotTerminal { actual: last_kind }); }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ExplainError {
    #[error("K-1.a 强校验失败: decision_id 为空")]
    K1DecisionIdEmpty,
    #[error("K-1.b 强校验失败: stages 数 {actual} < 最小值 {min}")]
    K1StagesTooFew { actual: usize, min: usize },
    #[error("K-1.c 强校验失败: 最后一个 stage {actual:?} 不是终止态")]
    K1LastStageNotTerminal { actual: StageKind },
    #[error("trace 已 finalized")]
    AlreadyFinalized,
    #[error("终止态之后不可再 push stage")]
    StageAfterTerminal,
}

const _: () = {
    assert!(STAGE_KIND_COUNT_HARDCODE == 5);
    assert!(K1_STRICT_CHECK_COUNT_HARDCODE == 3);
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn stage_count_5() {
        assert_eq!(STAGE_KIND_COUNT_HARDCODE, 5);
        assert!(StageKind::VerdictReached.is_terminal());
        assert!(StageKind::RationaleStated.is_terminal());
        assert!(!StageKind::RequestReceived.is_terminal());
    }
    #[test] fn k1_three_failures() {
        let mut t1 = DecisionTrace::new("", "alice");
        assert_eq!(t1.try_push_stage(StageKind::RequestReceived, "x").err(), Some(ExplainError::K1DecisionIdEmpty));
        let mut t2 = DecisionTrace::new("d", "alice");
        assert_eq!(t2.try_finalize(VerdictOutcome::Approved, "x").err(), Some(ExplainError::K1StagesTooFew { actual: 0, min: 2 }));
        t2.try_push_stage(StageKind::RequestReceived, "x").unwrap();
        assert_eq!(t2.try_finalize(VerdictOutcome::Approved, "x").err(), Some(ExplainError::K1StagesTooFew { actual: 1, min: 2 }));
        t2.try_push_stage(StageKind::EvidenceCollected, "x").unwrap();
        assert_eq!(t2.try_finalize(VerdictOutcome::Approved, "x").err(), Some(ExplainError::K1LastStageNotTerminal { actual: StageKind::EvidenceCollected }));
    }
    #[test] fn complete_lifecycle() {
        let mut t = DecisionTrace::new("d1", "alice");
        t.try_push_stage(StageKind::RequestReceived, "x").unwrap();
        t.try_push_stage(StageKind::EvidenceCollected, "x").unwrap();
        t.try_push_stage(StageKind::AuthorityConsulted, "x").unwrap();
        t.try_push_stage(StageKind::VerdictReached, "x").unwrap();
        assert!(t.try_push_stage(StageKind::RationaleStated, "x").is_err());
        t.try_finalize(VerdictOutcome::Approved, "ok").unwrap();
        assert!(t.is_complete());
        assert!(t.try_push_stage(StageKind::RationaleStated, "x").is_err());
        t.validate_k1().unwrap();
    }
    #[test] fn stage_str_round_trip() {
        assert_eq!(StageKind::RequestReceived.as_str(), "request_received");
        assert_eq!(StageKind::EvidenceCollected.as_str(), "evidence_collected");
        assert_eq!(StageKind::AuthorityConsulted.as_str(), "authority_consulted");
        assert_eq!(StageKind::VerdictReached.as_str(), "verdict_reached");
        assert_eq!(StageKind::RationaleStated.as_str(), "rationale_stated");
    }
    #[test] fn verdict_outcome_str() {
        assert_eq!(VerdictOutcome::Approved.as_str(), "approved");
        assert_eq!(VerdictOutcome::Rejected.as_str(), "rejected");
        assert_eq!(VerdictOutcome::PendingReview.as_str(), "pending_review");
    }
    #[test] fn new_trace_empty() {
        let t = DecisionTrace::new("d1", "alice");
        assert!(t.is_empty());
        assert!(!t.is_complete());
        assert_eq!(t.len(), 0);
    }
}
