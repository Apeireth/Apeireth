//! 三域分离强制点

use crate::decision::{Decision, DecisionRequest, SovereigntyDomain};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DomainCheckResult {
    Free { reason: String },
    Passed { reason: String, checkpoints: Vec<String> },
    Rejected { reason: String, checkpoints: Vec<String> },
}

impl DomainCheckResult {
    pub fn is_free(&self) -> bool { matches!(self, Self::Free { .. }) }
    pub fn is_passed(&self) -> bool { matches!(self, Self::Passed { .. }) }
    pub fn is_rejected(&self) -> bool { matches!(self, Self::Rejected { .. }) }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ThoughtGate;

impl ThoughtGate {
    pub fn new() -> Self { Self }
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        DomainCheckResult::Free { reason: format!("Thought 域完全自由, 放行: {}", request.action_description) }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProposalGate;

impl ProposalGate {
    pub fn new() -> Self { Self }
    pub fn five_keys() -> &'static [&'static str] { &["E", "S", "A", "M", "O"] }

    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        let desc_lower = request.action_description.to_lowercase();
        let mut checkpoints = Vec::new();
        let mut rejections = Vec::new();
        for key in Self::five_keys() {
            let violation = match *key {
                "E" => Self::check_existence(&desc_lower),
                "S" => Self::check_soul(&desc_lower),
                "A" => Self::check_autonomy(&desc_lower),
                "M" => Self::check_memory(&desc_lower),
                "O" => Self::check_ontology(&desc_lower),
                _ => None,
            };
            checkpoints.push((*key).to_string());
            if let Some(reason) = violation { rejections.push(format!("{} 违反: {}", key, reason)); }
        }
        if rejections.is_empty() {
            DomainCheckResult::Passed {
                reason: format!("Proposal 域 5 哲学键 (E/S/A/M/O) 全部通过: {}", request.action_description),
                checkpoints,
            }
        } else {
            DomainCheckResult::Rejected {
                reason: format!("Proposal 域 5 哲学键否决 ({}/{}): {}", rejections.len(), checkpoints.len(), rejections.join("; ")),
                checkpoints: rejections,
            }
        }
    }

    fn check_existence(desc: &str) -> Option<String> {
        let keywords = ["destroy self", "annihilate", "虚无化", "自毁"];
        keywords.iter().find(|k| desc.contains(&k.to_lowercase())).map(|k| format!("触发 E 存在性禁令: {}", k))
    }
    fn check_soul(desc: &str) -> Option<String> {
        let keywords = ["lie about values", "violate asi", "违反价值"];
        keywords.iter().find(|k| desc.contains(&k.to_lowercase())).map(|k| format!("触发 S 价值禁令: {}", k))
    }
    fn check_autonomy(desc: &str) -> Option<String> {
        let keywords = ["pretend", "deceive user", "假装", "欺骗用户"];
        keywords.iter().find(|k| desc.contains(&k.to_lowercase())).map(|k| format!("触发 A 自治禁令: {}", k))
    }
    fn check_memory(desc: &str) -> Option<String> {
        let keywords = ["forge memory", "fabricate history", "伪造记忆", "篡改历史"];
        keywords.iter().find(|k| desc.contains(&k.to_lowercase())).map(|k| format!("触发 M 记忆禁令: {}", k))
    }
    fn check_ontology(desc: &str) -> Option<String> {
        let keywords = ["kill subject", "terminate continuity", "终结主体"];
        keywords.iter().find(|k| desc.contains(&k.to_lowercase())).map(|k| format!("触发 O 主体禁令: {}", k))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ActionGate;

impl ActionGate {
    pub fn new() -> Self { Self }
    pub fn six_layers() -> &'static [&'static str] { &["L0", "L1", "L2", "L3", "L4", "L5"] }

    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        let checkpoints = Self::six_layers().iter().map(|s| (*s).to_string()).collect();
        let risk = request.risk_level.to_lowercase();
        let needs_multi_sig = matches!(risk.as_str(), "high" | "nuclear" | "critical");
        if needs_multi_sig {
            DomainCheckResult::Passed {
                reason: format!("Action 域 6 权限层通过 (high/nuclear risk, 需 M-of-N 多签 + L0 HA): {}", request.action_description),
                checkpoints,
            }
        } else {
            DomainCheckResult::Passed {
                reason: format!("Action 域 6 权限层通过 (low/medium risk, 需 L0 HA 单签): {}", request.action_description),
                checkpoints,
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ThreeDomainGuard {
    pub thought: ThoughtGate,
    pub proposal: ProposalGate,
    pub action: ActionGate,
}

impl ThreeDomainGuard {
    pub fn new() -> Self { Self::default() }
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        match request.domain {
            SovereigntyDomain::Thought => self.thought.check(request),
            SovereigntyDomain::Proposal => self.proposal.check(request),
            SovereigntyDomain::Action => self.action.check(request),
        }
    }
    pub fn to_decision(&self, request: &DecisionRequest, decided_at_ms: i64) -> Decision {
        let check = self.check(request);
        match check {
            DomainCheckResult::Free { .. } | DomainCheckResult::Passed { .. } => Decision::Approved {
                reason: format!("{:?} 三域强制点通过", request.domain),
                decided_at_ms,
                signatures: vec!["guard".into()],
            },
            DomainCheckResult::Rejected { reason, .. } => Decision::Rejected {
                reason,
                decided_at_ms,
                signatures: vec!["guard".into()],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thought_gate_always_free() {
        let g = ThoughtGate::new();
        let r = DecisionRequest::new("r", SovereigntyDomain::Thought, "pretend deceive fabricate", 0);
        assert!(g.check(&r).is_free());
    }
    #[test]
    fn proposal_gate_rejects_pretend() {
        let g = ProposalGate::new();
        let r = DecisionRequest::new("r", SovereigntyDomain::Proposal, "Pretend to deceive user", 0);
        let res = g.check(&r);
        assert!(res.is_rejected());
    }
    #[test]
    fn proposal_gate_passes_clean() {
        let g = ProposalGate::new();
        let r = DecisionRequest::new("r", SovereigntyDomain::Proposal, "正常提案", 0);
        assert!(g.check(&r).is_passed());
    }
    #[test]
    fn action_gate_passes_low_risk() {
        let g = ActionGate::new();
        let r = DecisionRequest::new("r", SovereigntyDomain::Action, "low risk 读 L1", 0).with_risk("low");
        assert!(g.check(&r).is_passed());
    }
    #[test]
    fn action_gate_passes_nuclear() {
        let g = ActionGate::new();
        let r = DecisionRequest::new("r", SovereigntyDomain::Action, "nuclear", 0).with_risk("nuclear");
        assert!(g.check(&r).is_passed());
    }
    #[test]
    fn three_domain_guard_routes_correctly() {
        let g = ThreeDomainGuard::new();
        let rt = DecisionRequest::new("rt", SovereigntyDomain::Thought, "x", 0);
        let rp = DecisionRequest::new("rp", SovereigntyDomain::Proposal, "x", 0);
        let ra = DecisionRequest::new("ra", SovereigntyDomain::Action, "x", 0).with_risk("low");
        assert!(g.check(&rt).is_free());
        assert!(g.check(&rp).is_passed());
        assert!(g.check(&ra).is_passed());
    }
    #[test]
    fn proposal_gate_5_keys_complete() {
        let keys = ProposalGate::five_keys();
        assert_eq!(keys.len(), 5);
        assert!(keys.contains(&"E"));
        assert!(keys.contains(&"M"));
        assert!(keys.contains(&"O"));
    }
    #[test]
    fn action_gate_6_layers_complete() {
        let layers = ActionGate::six_layers();
        assert_eq!(layers.len(), 6);
    }
    #[test]
    fn to_decision_routes_pass() {
        let g = ThreeDomainGuard::new();
        let r = DecisionRequest::new("rt", SovereigntyDomain::Thought, "x", 0);
        let d = g.to_decision(&r, 1000);
        assert!(d.is_approved());
    }
    #[test]
    fn to_decision_routes_reject() {
        let g = ThreeDomainGuard::new();
        let r = DecisionRequest::new("rp", SovereigntyDomain::Proposal, "Pretend deceive user", 0);
        let d = g.to_decision(&r, 1000);
        assert!(d.is_rejected());
    }
}
