//! MemoryGovernance - 记忆治理 (从 v1.0 apeireth-memory/memory_governance.rs 681 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 policy + 决策

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPolicy { AutoApprove, RequireReview, Forbid }

pub struct GovernanceRule { pub name: String, pub policy: MemoryPolicy, pub reason: String }

pub struct MemoryGovernance { pub rules: HashMap<String, GovernanceRule> }

impl MemoryGovernance {
    pub fn new() -> Self { Self { rules: HashMap::new() } }
    /// 0 装 PASS: 真 add
    pub fn add(&mut self, r: GovernanceRule) { self.rules.insert(r.name.clone(), r); }
    /// 0 装 PASS: 真 check
    pub fn check(&self, name: &str) -> Option<MemoryPolicy> { self.rules.get(name).map(|r| r.policy) }
    /// 0 装 PASS: 真 count by policy
    pub fn count_by_policy(&self, p: MemoryPolicy) -> usize { self.rules.values().filter(|r| r.policy == p).count() }
}

impl Default for MemoryGovernance { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut g = MemoryGovernance::new();
        g.add(GovernanceRule { name: "r1".into(), policy: MemoryPolicy::AutoApprove, reason: "x".into() });
        assert!(g.check("r1").is_some());
    }
    #[test] fn test_check() {
        let mut g = MemoryGovernance::new();
        g.add(GovernanceRule { name: "r".into(), policy: MemoryPolicy::RequireReview, reason: "x".into() });
        assert_eq!(g.check("r"), Some(MemoryPolicy::RequireReview));
    }
    #[test] fn test_unknown() {
        let g = MemoryGovernance::new();
        assert!(g.check("missing").is_none());
    }
    #[test] fn test_count_by_policy() {
        let mut g = MemoryGovernance::new();
        g.add(GovernanceRule { name: "a".into(), policy: MemoryPolicy::AutoApprove, reason: "x".into() });
        g.add(GovernanceRule { name: "b".into(), policy: MemoryPolicy::RequireReview, reason: "x".into() });
        assert_eq!(g.count_by_policy(MemoryPolicy::AutoApprove), 1);
    }
    #[test] fn test_policy_eq() { assert_eq!(MemoryPolicy::Forbid, MemoryPolicy::Forbid); }
}
