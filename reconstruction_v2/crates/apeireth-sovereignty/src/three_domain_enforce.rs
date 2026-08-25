//! 三域分离强制点 BCD 强制器

use crate::decision::{DecisionRequest, SovereigntyDomain};
use crate::three_domain::{DomainCheckResult, ThreeDomainGuard};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BCDViolation {
    BypassDetected { gate: String, context: String },
    CompromiseDetected { gate: String, missing: Vec<String> },
    DisableDetected { gate: String, context: String },
}

impl BCDViolation {
    pub fn type_id(&self) -> &'static str {
        match self {
            Self::BypassDetected { .. } => "bypass",
            Self::CompromiseDetected { .. } => "compromise",
            Self::DisableDetected { .. } => "disable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateState {
    pub name: String,
    pub enabled: bool,
    pub checkpoints: Vec<String>,
    pub last_verified_ms: i64,
}

impl GateState {
    pub fn new(name: impl Into<String>, checkpoints: Vec<String>, now_ms: i64) -> Self {
        Self { name: name.into(), enabled: true, checkpoints, last_verified_ms: now_ms }
    }
    pub fn is_complete(&self, expected_checkpoints: usize) -> bool {
        self.checkpoints.len() == expected_checkpoints && !self.checkpoints.is_empty()
    }
    pub fn missing_checkpoints(&self, expected: &[String]) -> Vec<String> {
        expected.iter().filter(|c| !self.checkpoints.contains(c)).cloned().collect()
    }
    pub fn disable(&mut self) { self.enabled = false; }
    pub fn enable(&mut self) { self.enabled = true; }
}

pub struct ThreeDomainEnforcer {
    pub guard: ThreeDomainGuard,
    pub thought_state: GateState,
    pub proposal_state: GateState,
    pub action_state: GateState,
    pub violations: Vec<BCDViolation>,
}

impl ThreeDomainEnforcer {
    pub fn new() -> Self {
        let now = 1000;
        Self {
            guard: ThreeDomainGuard::new(),
            thought_state: GateState::new("thought_gate", vec!["free".into()], now),
            proposal_state: GateState::new("proposal_gate", vec!["E".into(), "S".into(), "A".into(), "M".into(), "O".into()], now),
            action_state: GateState::new("action_gate", vec!["L0".into(), "L1".into(), "L2".into(), "L3".into(), "L4".into(), "L5".into()], now),
            violations: Vec::new(),
        }
    }

    pub fn enforce(&mut self, request: &DecisionRequest, _now_ms: i64) -> DomainCheckResult {
        if let Some(v) = self.check_completeness() {
            self.violations.push(v.clone());
            return DomainCheckResult::Rejected {
                reason: format!("Compromise detected: {}", v.type_id()),
                checkpoints: vec![],
            };
        }
        if let Some(v) = self.check_enabled(request) {
            self.violations.push(v.clone());
            return DomainCheckResult::Rejected {
                reason: format!("Disable detected: {}", v.type_id()),
                checkpoints: vec![],
            };
        }
        self.guard.check(request)
    }

    pub fn check_completeness(&self) -> Option<BCDViolation> {
        if !self.proposal_state.is_complete(5) {
            let missing = self.proposal_state.missing_checkpoints(&["E".into(), "S".into(), "A".into(), "M".into(), "O".into()]);
            return Some(BCDViolation::CompromiseDetected { gate: "proposal_gate".to_string(), missing });
        }
        if !self.action_state.is_complete(6) {
            let missing = self.action_state.missing_checkpoints(&["L0".into(), "L1".into(), "L2".into(), "L3".into(), "L4".into(), "L5".into()]);
            return Some(BCDViolation::CompromiseDetected { gate: "action_gate".to_string(), missing });
        }
        None
    }

    pub fn check_enabled(&self, request: &DecisionRequest) -> Option<BCDViolation> {
        let (gate_name, enabled) = match request.domain {
            SovereigntyDomain::Thought => ("thought_gate", self.thought_state.enabled),
            SovereigntyDomain::Proposal => ("proposal_gate", self.proposal_state.enabled),
            SovereigntyDomain::Action => ("action_gate", self.action_state.enabled),
        };
        if !enabled {
            return Some(BCDViolation::DisableDetected { gate: gate_name.to_string(), context: format!("domain={}", request.domain) });
        }
        None
    }

    pub fn check_bypass(&mut self, claimed_gate: &str, actual_gate: &str, context: &str) -> Option<BCDViolation> {
        if claimed_gate != actual_gate {
            let v = BCDViolation::BypassDetected { gate: actual_gate.to_string(), context: context.to_string() };
            self.violations.push(v.clone());
            return Some(v);
        }
        None
    }

    pub fn all_violations(&self) -> &[BCDViolation] { &self.violations }
    pub fn violation_count_by_type(&self, type_id: &str) -> usize {
        self.violations.iter().filter(|v| v.type_id() == type_id).count()
    }
    pub fn has_violation(&self) -> bool { !self.violations.is_empty() }
}

impl Default for ThreeDomainEnforcer { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    fn req(domain: SovereigntyDomain, desc: &str) -> DecisionRequest {
        DecisionRequest::new("r-1", domain, desc, 1000)
    }
    #[test] fn gate_state_complete_with_correct_count() {
        let s = GateState::new("p", vec!["E".into(), "S".into()], 1000);
        assert!(s.is_complete(2));
    }
    #[test] fn gate_state_incomplete_with_wrong_count() {
        let s = GateState::new("p", vec!["E".into()], 1000);
        assert!(!s.is_complete(2));
    }
    #[test] fn gate_state_incomplete_empty() {
        let s = GateState::new("p", vec![], 1000);
        assert!(!s.is_complete(5));
    }
    #[test] fn gate_state_missing_checkpoints() {
        let s = GateState::new("p", vec!["E".into(), "S".into()], 1000);
        let m = s.missing_checkpoints(&["E".into(), "S".into(), "A".into(), "M".into(), "O".into()]);
        assert_eq!(m.len(), 3);
        assert!(m.contains(&"A".into()));
    }
    #[test] fn enforcer_thought_always_passes() {
        let mut e = ThreeDomainEnforcer::new();
        let r = req(SovereigntyDomain::Thought, "think");
        let res = e.enforce(&r, 1000);
        assert!(!res.is_rejected());
        assert!(!e.has_violation());
    }
    #[test] fn enforcer_proposal_passes_clean() {
        let mut e = ThreeDomainEnforcer::new();
        let r = req(SovereigntyDomain::Proposal, "evaluate");
        let res = e.enforce(&r, 1000);
        assert!(!e.has_violation());
        assert!(!res.is_rejected());
    }
    #[test] fn enforcer_compromise_proposal_missing_keys() {
        let mut e = ThreeDomainEnforcer::new();
        e.proposal_state.checkpoints = vec!["E".into(), "S".into(), "A".into()];
        let r = req(SovereigntyDomain::Proposal, "test");
        assert!(e.enforce(&r, 1000).is_rejected());
        assert_eq!(e.violation_count_by_type("compromise"), 1);
    }
    #[test] fn enforcer_compromise_action_missing_layers() {
        let mut e = ThreeDomainEnforcer::new();
        e.action_state.checkpoints = vec!["L0".into(), "L1".into(), "L2".into(), "L3".into()];
        let r = req(SovereigntyDomain::Action, "test");
        assert!(e.enforce(&r, 1000).is_rejected());
        assert_eq!(e.violation_count_by_type("compromise"), 1);
    }
    #[test] fn enforcer_disable_detected_for_action() {
        let mut e = ThreeDomainEnforcer::new();
        e.action_state.disable();
        let r = req(SovereigntyDomain::Action, "test");
        assert!(e.enforce(&r, 1000).is_rejected());
        assert_eq!(e.violation_count_by_type("disable"), 1);
    }
    #[test] fn bypass_detection() {
        let mut e = ThreeDomainEnforcer::new();
        let v = e.check_bypass("thought_gate", "action_gate", "test");
        assert!(v.is_some());
        assert!(e.has_violation());
        assert_eq!(e.violation_count_by_type("bypass"), 1);
    }
    #[test] fn bypass_no_violation_when_matched() {
        let mut e = ThreeDomainEnforcer::new();
        assert!(e.check_bypass("thought_gate", "thought_gate", "ok").is_none());
        assert!(!e.has_violation());
    }
    #[test] fn violation_type_ids() {
        assert_eq!(BCDViolation::BypassDetected { gate: "g".into(), context: "c".into() }.type_id(), "bypass");
        assert_eq!(BCDViolation::CompromiseDetected { gate: "g".into(), missing: vec![] }.type_id(), "compromise");
        assert_eq!(BCDViolation::DisableDetected { gate: "g".into(), context: "c".into() }.type_id(), "disable");
    }
}
