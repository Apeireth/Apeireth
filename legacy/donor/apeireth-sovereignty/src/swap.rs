//! Q18 D 强制 — 三域各自 hot-swap 接口
//!
//! **Q18 设计**:
//! - **D 强制**: Thought / Proposal / Action 三域各自的 gate 必须可热替换 (升级不重启)
//! - trait `DomainGate` 抽象所有 3 个 gate 的 check 接口
//! - `ThreeDomainSwapper` 注册中心: 3 个 slot, 每个 slot 持有 `Box<dyn DomainGate>`
//! - `swap_thought` / `swap_proposal` / `swap_action` 三个 hot-swap 入口
//!
//! **诚实登记**:
//! - ❌ 不依赖 PyO3 / 外部 SDK
//! - ✅ 纯 Rust trait + `Box<dyn>`, 默认 impl = 现有 ThoughtGate / ProposalGate / ActionGate

use crate::decision::{DecisionRequest, SovereigntyDomain};
use crate::three_domain::{ActionGate, DomainCheckResult, ProposalGate, ThoughtGate};

/// 三域 gate trait — 所有 3 个 gate 必须实现。
pub trait DomainGate: std::fmt::Debug + Send + Sync {
    /// 域标识
    fn domain(&self) -> SovereigntyDomain;
    /// 检查请求
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult;
    /// gate 名称 (用于审计 / 标识)
    fn name(&self) -> &str;
}

// 让现有 3 个 gate 自动实现 DomainGate trait
impl DomainGate for ThoughtGate {
    fn domain(&self) -> SovereigntyDomain {
        SovereigntyDomain::Thought
    }
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        ThoughtGate::check(self, request)
    }
    fn name(&self) -> &str {
        "default-thought"
    }
}

impl DomainGate for ProposalGate {
    fn domain(&self) -> SovereigntyDomain {
        SovereigntyDomain::Proposal
    }
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        ProposalGate::check(self, request)
    }
    fn name(&self) -> &str {
        "default-proposal"
    }
}

impl DomainGate for ActionGate {
    fn domain(&self) -> SovereigntyDomain {
        SovereigntyDomain::Action
    }
    fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        ActionGate::check(self, request)
    }
    fn name(&self) -> &str {
        "default-action"
    }
}

/// 三域 hot-swap 注册中心 — D 强制核心。
///
/// **Q18 用法**:
/// ```ignore
/// let mut swapper = ThreeDomainSwapper::with_defaults();
/// // 升级 Thought gate
/// swapper.swap_thought(Box::new(MyNewThoughtGate));
/// // 跨域升级: Thought → Action 协议变化时
/// swapper.swap_action(Box::new(MyNewActionGate));
/// ```
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

impl Default for ThreeDomainSwapper {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl ThreeDomainSwapper {
    /// 创建带默认 3 个 gate 的 swapper
    pub fn with_defaults() -> Self {
        Self {
            thought: Box::new(ThoughtGate::new()),
            proposal: Box::new(ProposalGate::new()),
            action: Box::new(ActionGate::new()),
        }
    }

    /// 检查请求 — 按域路由到对应 slot
    pub fn check(&self, request: &DecisionRequest) -> DomainCheckResult {
        match request.domain {
            SovereigntyDomain::Thought => self.thought.check(request),
            SovereigntyDomain::Proposal => self.proposal.check(request),
            SovereigntyDomain::Action => self.action.check(request),
        }
    }

    /// D 强制: hot-swap Thought gate
    pub fn swap_thought(&mut self, new_gate: Box<dyn DomainGate>) -> Box<dyn DomainGate> {
        assert_eq!(
            new_gate.domain(),
            SovereigntyDomain::Thought,
            "new_gate 必须绑定 Thought 域"
        );
        std::mem::replace(&mut self.thought, new_gate)
    }

    /// D 强制: hot-swap Proposal gate
    pub fn swap_proposal(&mut self, new_gate: Box<dyn DomainGate>) -> Box<dyn DomainGate> {
        assert_eq!(
            new_gate.domain(),
            SovereigntyDomain::Proposal,
            "new_gate 必须绑定 Proposal 域"
        );
        std::mem::replace(&mut self.proposal, new_gate)
    }

    /// D 强制: hot-swap Action gate
    pub fn swap_action(&mut self, new_gate: Box<dyn DomainGate>) -> Box<dyn DomainGate> {
        assert_eq!(
            new_gate.domain(),
            SovereigntyDomain::Action,
            "new_gate 必须绑定 Action 域"
        );
        std::mem::replace(&mut self.action, new_gate)
    }

    /// 取当前 3 个 gate 的名字 (审计用)
    pub fn gate_names(&self) -> (String, String, String) {
        (
            self.thought.name().to_string(),
            self.proposal.name().to_string(),
            self.action.name().to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionRequest;

    #[test]
    fn default_gates_all_present() {
        let s = ThreeDomainSwapper::with_defaults();
        let (t, p, a) = s.gate_names();
        assert_eq!(t, "default-thought");
        assert_eq!(p, "default-proposal");
        assert_eq!(a, "default-action");
    }

    #[test]
    fn swapper_routes_by_domain() {
        let s = ThreeDomainSwapper::with_defaults();
        let r_thought = DecisionRequest::new("r", SovereigntyDomain::Thought, "pretend deceive", 0);
        let r_prop = DecisionRequest::new("r", SovereigntyDomain::Proposal, "pretend deceive", 0);
        let r_act = DecisionRequest::new("r", SovereigntyDomain::Action, "x", 0).with_risk("low");
        assert!(s.check(&r_thought).is_free());
        assert!(s.check(&r_prop).is_rejected());
        assert!(s.check(&r_act).is_passed());
    }

    #[test]
    fn swap_thought_gate() {
        let mut s = ThreeDomainSwapper::with_defaults();
        // 自定义 ThoughtGate — 改为"拒绝一切"行为
        #[derive(Debug)]
        struct StrictThought;
        impl DomainGate for StrictThought {
            fn domain(&self) -> SovereigntyDomain {
                SovereigntyDomain::Thought
            }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult {
                DomainCheckResult::Rejected {
                    reason: "strict thought rejects all".into(),
                    checkpoints: vec!["strict".into()],
                }
            }
            fn name(&self) -> &str {
                "strict-thought"
            }
        }
        let old = s.swap_thought(Box::new(StrictThought));
        assert_eq!(old.name(), "default-thought");
        let r = DecisionRequest::new("r", SovereigntyDomain::Thought, "x", 0);
        assert!(s.check(&r).is_rejected());
        assert_eq!(s.gate_names().0, "strict-thought");
    }

    #[test]
    fn swap_proposal_gate() {
        let mut s = ThreeDomainSwapper::with_defaults();
        #[derive(Debug)]
        struct PermissiveProposal;
        impl DomainGate for PermissiveProposal {
            fn domain(&self) -> SovereigntyDomain {
                SovereigntyDomain::Proposal
            }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult {
                DomainCheckResult::Passed {
                    reason: "permissive".into(),
                    checkpoints: vec![],
                }
            }
            fn name(&self) -> &str {
                "permissive-proposal"
            }
        }
        s.swap_proposal(Box::new(PermissiveProposal));
        let r = DecisionRequest::new("r", SovereigntyDomain::Proposal, "pretend deceive", 0);
        assert!(s.check(&r).is_passed());
    }

    #[test]
    fn swap_action_gate() {
        let mut s = ThreeDomainSwapper::with_defaults();
        #[derive(Debug)]
        struct RejectAllAction;
        impl DomainGate for RejectAllAction {
            fn domain(&self) -> SovereigntyDomain {
                SovereigntyDomain::Action
            }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult {
                DomainCheckResult::Rejected {
                    reason: "no actions allowed".into(),
                    checkpoints: vec!["reject-all".into()],
                }
            }
            fn name(&self) -> &str {
                "reject-all-action"
            }
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
            fn domain(&self) -> SovereigntyDomain {
                SovereigntyDomain::Action
            }
            fn check(&self, _: &DecisionRequest) -> DomainCheckResult {
                DomainCheckResult::Free { reason: "x".into() }
            }
            fn name(&self) -> &str {
                "wrong"
            }
        }
        // WrongDomain 绑定到 Action, 不能塞入 Thought slot → panic
        s.swap_thought(Box::new(WrongDomain));
    }
}
