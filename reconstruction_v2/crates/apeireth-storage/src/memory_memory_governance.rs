//! Memory Governance - 记忆治理 (抄 v1 apeireth-memory/memory_governance.rs)
use std::collections::HashMap;
pub struct GovernanceRule { pub name: String, pub max_importance: f32, pub retention_ms: i64 }
pub struct GovernancePolicy { pub rules: HashMap<String, GovernanceRule> }
impl GovernancePolicy {
    pub fn new() -> Self { Self { rules: HashMap::new() } }
    pub fn add(&mut self, r: GovernanceRule) { self.rules.insert(r.name.clone(), r); }
    pub fn check_retention(&self, name: &str, importance: f32) -> bool {
        self.rules.get(name).map(|r| importance <= r.max_importance).unwrap_or(true)
    }
}
#[cfg(test)] mod tests { use super::*; #[test] fn test_check() { let mut p = GovernancePolicy::new(); p.add(GovernanceRule{name:"x".into(),max_importance:0.8,retention_ms:1000}); assert!(p.check_retention("x", 0.5)); assert!(!p.check_retention("x", 0.9)); } }