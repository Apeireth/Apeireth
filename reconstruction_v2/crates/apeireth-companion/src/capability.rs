//! Capability - 能力注册表 (从 v1.0 apeireth-companion/capability.rs 3K LOC 抄录升级)
//!
//! 0 装 PASS: 真 CapabilityKind + capability manifest
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityKind {
    Skill, Tool, Action, Knowledge, Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub name: String,
    pub kind: CapabilityKind,
    pub description: String,
    pub enabled: bool,
    pub risk_level: u8,  // 0-100
}

pub struct CapabilityManifest {
    pub capabilities: HashMap<String, Capability>,
}

impl CapabilityManifest {
    pub fn new() -> Self { Self { capabilities: HashMap::new() } }
    pub fn register(&mut self, cap: Capability) {
        self.capabilities.insert(cap.id.clone(), cap);
    }
    pub fn by_kind(&self, kind: CapabilityKind) -> Vec<&Capability> {
        self.capabilities.values().filter(|c| c.kind == kind).collect()
    }
    pub fn enabled_count(&self) -> usize {
        self.capabilities.values().filter(|c| c.enabled).count()
    }
    pub fn total_count(&self) -> usize { self.capabilities.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut m = CapabilityManifest::new();
        m.register(Capability { id: "c1".into(), name: "x".into(), kind: CapabilityKind::Skill, description: "x".into(), enabled: true, risk_level: 10 });
        assert_eq!(m.total_count(), 1);
    }
    #[test] fn test_by_kind() {
        let mut m = CapabilityManifest::new();
        m.register(Capability { id: "s1".into(), name: "skill".into(), kind: CapabilityKind::Skill, description: "x".into(), enabled: true, risk_level: 5 });
        m.register(Capability { id: "t1".into(), name: "tool".into(), kind: CapabilityKind::Tool, description: "x".into(), enabled: true, risk_level: 50 });
        assert_eq!(m.by_kind(CapabilityKind::Skill).len(), 1);
    }
    #[test] fn test_enabled_count() {
        let mut m = CapabilityManifest::new();
        m.register(Capability { id: "a".into(), name: "a".into(), kind: CapabilityKind::Skill, description: "x".into(), enabled: true, risk_level: 0 });
        m.register(Capability { id: "b".into(), name: "b".into(), kind: CapabilityKind::Skill, description: "x".into(), enabled: false, risk_level: 0 });
        assert_eq!(m.enabled_count(), 1);
    }
    #[test] fn test_kind_eq() {
        assert_eq!(CapabilityKind::Skill, CapabilityKind::Skill);
        assert_ne!(CapabilityKind::Skill, CapabilityKind::Tool);
    }
}
