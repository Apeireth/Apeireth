//! Q18 D 强制 — 三域 hot-swap

use crate::decision::{DecisionRequest, SovereigntyDomain};
use crate::three_domain::{ActionGate, DomainCheckResult, ProposalGate, ThoughtGate};

pub trait DomainGate: std::fmt::Debug + Send + Sync {
    fn domain(&self) -> SovereigntyDomain;
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult;
    fn name(&self) -> &str;
}

impl DomainGate for ThoughtGate {
    fn domain(&self) -> SovereigntyDomain { SovereigntyDomain::Thought }
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult { ThoughtGate::check(self, request) }
    fn name(&self) -> &str { "default-thought" }
}
impl DomainGate for ProposalGate {
    fn domain(&self) -> SovereigntyDomain { SovereigntyDomain::Proposal }
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult { ProposalGate::check(self, request) }
    fn name(&self) -> &str { "default-proposal" }
}
impl DomainGate for ActionGate {
    fn domain(&self) -> SovereigntyDomain { SovereigntyDomain::Action }
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult { ActionGate::check(self, request) }
    fn name(&self) -> &str { "default-action" }
}

pub struct ThreeDomainSwapper {
    thought: Box<dyn DomainGate>,
    proposal: Box<dyn DomainGate>,
    action: Box<dyn DomainGate>,
}

impl std::fmt::Debug for ThreeDomainSwapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreeDomainSwapper")
            .field("thought", &self.thought.name())
            .field("proposal", &self.proposal.name())
            .field("action", &self.action.name())
            .finish()
    }
}

impl Default for ThreeDomainSwapper { fn default() -> Self { Self::with_defaults() } }

impl ThreeDomainSwapper {
    pub fn with_defaults() -> Self {
        Self { thought: Box::new(ThoughtGate::new()), proposal: Box::new(ProposalGate::new()), action: Box::new(ActionGate::new()) }
    }
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        match request.domain {
            SovereigntyDomain::Thought => self.thought.check(request),
            SovereigntyDomain::Proposal => self.proposal.check(request),
            SovereigntyDomain::Action => self.action.check(request),
        }
    }
    pub fn swap_thought(&mut self, new_gate: Box<dyn DomainGate>) -> Box<dyn DomainGate> {
        assert_eq!(new_gate.domain(), SovereigntyDomain::Thought, "new_gate 必须绑定 Thought 域");
        std::mem::replace(&mut self.thought, new_gate)
    }
    pub fn swap_proposal(&mut self, new_gate: Box<dyn DomainGate>) -> Box<dyn DomainGate> {
        assert_eq!(new_gate.domain(), SovereigntyDomain::Proposal, "new_gate 必须绑定 Proposal 域");
        std::mem::replace(&mut self.proposal, new_gate)
    }
    pub fn swap_action(&mut self, new_gate: Box<dyn DomainGate>) -> Box<dyn DomainGate> {
        assert_eq!(new_gate.domain(), SovereigntyDomain::Action, "new_gate 必须绑定 Action 域");
        std::mem::replace(&mut self.action, new_gate)
    }
    pub fn gate_names(&self) -> (String, String, String) {
        (self.thought.name().to_string(), self.proposal.name().to_string(), self.action.name().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionRequest;

    #[test] fn default_gates_all_present() {
        let s = ThreeDomainSwapper::with_defaults();
        let (t, p, a) = s.gate_names();
        assert_eq!(t, "default-thought");
        assert_eq!(p, "default-proposal");
        assert_eq!(a, "default-action");
    }

    #[test] fn swapper_routes_by_domain() {
        let s = ThreeDomainSwapper::with_defaults();
        let rt = DecisionRequest::new("rt", SovereigntyDomain::Thought, "pretend deceive", 0);
        let rp = DecisionRequest::new("rp", SovereigntyDomain::Proposal, "pretend deceive", 0);
        let ra = DecisionRequest::new("ra", SovereigntyDomain::Action, "x", 0).with_risk("low");
        assert!(s.check(&rt).is_free());
        assert!(s.check(&rp).is_rejected());
        assert!(s.check(&ra).is_passed());
    }

    #[test] fn swap_thought_gate() {
        let mut s = ThreeDomainSwapper::with_defaults();
        #[derive(Debug)]
        struct StrictThought;
        impl DomainGate for StrictThought {
            fn domain(&self) -> SovereigntyDomain { SovereigntyDomain::Thought }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult { DomainCheckResult::Rejected { reason: "strict".into(), checkpoints: vec!["strict".into()] } }
            fn name(&self) -> &str { "strict-thought" }
        }
        let old = s.swap_thought(Box::new(StrictThought));
        assert_eq!(old.name(), "default-thought");
        let r = DecisionRequest::new("r", SovereigntyDomain::Thought, "x", 0);
        assert!(s.check(&r).is_rejected());
        assert_eq!(s.gate_names().0, "strict-thought");
    }

    #[test] fn swap_proposal_gate() {
        let mut s = ThreeDomainSwapper::with_defaults();
        #[derive(Debug)]
        struct PermissiveProposal;
        impl DomainGate for PermissiveProposal {
            fn domain(&self) -> SovereigntyDomain { SovereigntyDomain::Proposal }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult { DomainCheckResult::Passed { reason: "p".into(), checkpoints: vec![] } }
            fn name(&self) -> &str { "permissive-proposal" }
        }
        s.swap_proposal(Box::new(PermissiveProposal));
        let r = DecisionRequest::new("r", SovereigntyDomain::Proposal, "pretend deceive", 0);
        assert!(s.check(&r).is_passed());
    }

    #[test] fn swap_action_gate() {
        let mut s = ThreeDomainSwapper::with_defaults();
        #[derive(Debug)]
        struct RejectAllAction;
        impl DomainGate for RejectAllAction {
            fn domain(&self) -> SovereigntyDomain { SovereigntyDomain::Action }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult { DomainCheckResult::Rejected { reason: "n".into(), checkpoints: vec!["n".into()] } }
            fn name(&self) -> &str { "reject-all-action" }
        }
        s.swap_action(Box::new(RejectAllAction));
        let r = DecisionRequest::new("r", SovereigntyDomain::Action, "x", 0).with_risk("low");
        assert!(s.check(&r).is_rejected());
    }

    #[test]
    #[should_panic(expected = "必须绑定")]
    fn swap_rejects_wrong_domain() {
        let mut s = ThreeDomainSwapper::with_defaults();
        #[derive(Debug)]
        struct WrongDomain;
        impl DomainGate for WrongDomain {
            fn domain(&self) -> SovereigntyDomain { SovereigntyDomain::Action }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult { DomainCheckResult::Free { reason: "x".into() } }
            fn name(&self) -> &str { "wrong" }
        }
        s.swap_thought(Box::new(WrongDomain));
    }
}
