//! ConstitutionGate - 宪法门 (从 v1.0 apeireth-companion/constitution_gate.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 gate decision
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GateDecision { Allow, Deny, RequireApproval }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateRule {
    pub name: String,
    pub description: String,
    pub risk_level: u8,  // 0-100
}

pub struct ConstitutionGate {
    pub rules: Vec<GateRule>,
    pub risk_threshold: u8,
}

impl ConstitutionGate {
    pub fn new(rules: Vec<GateRule>, risk_threshold: u8) -> Self { Self { rules, risk_threshold } }

    /// 0 装 PASS: 真 evaluate
    pub fn evaluate(&self, action_risk: u8) -> GateDecision {
        if action_risk > self.risk_threshold * 2 { GateDecision::Deny }
        else if action_risk > self.risk_threshold { GateDecision::RequireApproval }
        else { GateDecision::Allow }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_allow_low() {
        let g = ConstitutionGate::new(vec![], 50);
        assert_eq!(g.evaluate(10), GateDecision::Allow);
    }
    #[test] fn test_require_approval() {
        let g = ConstitutionGate::new(vec![], 50);
        assert_eq!(g.evaluate(75), GateDecision::RequireApproval);
    }
    #[test] fn test_deny_high() {
        let g = ConstitutionGate::new(vec![], 50);
        assert_eq!(g.evaluate(150), GateDecision::Deny);
    }
    #[test] fn test_boundary() {
        let g = ConstitutionGate::new(vec![], 50);
        assert_eq!(g.evaluate(50), GateDecision::Allow);
        assert_eq!(g.evaluate(51), GateDecision::RequireApproval);
    }
    #[test] fn test_decision_eq() {
        assert_eq!(GateDecision::Allow, GateDecision::Allow);
        assert_ne!(GateDecision::Allow, GateDecision::Deny);
    }
}
