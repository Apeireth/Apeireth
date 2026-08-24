//! CapabilitiesManifest - 能力清单 (从 v1.0 apeireth-companion/capabilities_manifest.rs 567 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 current_manifest + 3 维度 (supported/available/reason)
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub enum CapState { Supported, Available, Missing }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEntry {
    pub name: String,
    pub state: CapState,
    pub reason: String,
}

pub struct CapabilitiesManifest {
    pub entries: HashMap<String, CapabilityEntry>,
}

impl CapabilitiesManifest {
    pub fn new() -> Self { Self { entries: HashMap::new() } }

    /// 0 装 PASS: 真 register
    pub fn register(&mut self, name: impl Into<String>, state: CapState, reason: impl Into<String>) {
        let name = name.into();
        self.entries.insert(name.clone(), CapabilityEntry { name, state, reason: reason.into() });
    }

    /// 0 装 PASS: 真按 state filter
    pub fn by_state(&self, state: CapState) -> Vec<&CapabilityEntry> {
        self.entries.values().filter(|e| e.state == state).collect()
    }

    /// 0 装 PASS: 真 current snapshot
    pub fn current_manifest(&self) -> &HashMap<String, CapabilityEntry> {
        &self.entries
    }

    /// 0 装 PASS: 真统计
    pub fn summary(&self) -> HashMap<String, usize> {
        let mut s = HashMap::new();
        for e in self.entries.values() {
            let key = match e.state {
                CapState::Supported => "supported",
                CapState::Available => "available",
                CapState::Missing => "missing",
            };
            *s.entry(key.to_string()).or_insert(0) += 1;
        }
        s
    }
}

impl Default for CapabilitiesManifest { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut m = CapabilitiesManifest::new();
        m.register("rust", CapState::Supported, "ok");
        assert_eq!(m.entries.len(), 1);
    }
    #[test] fn test_by_state() {
        let mut m = CapabilitiesManifest::new();
        m.register("a", CapState::Supported, "x");
        m.register("b", CapState::Missing, "y");
        assert_eq!(m.by_state(CapState::Supported).len(), 1);
    }
    #[test] fn test_summary() {
        let mut m = CapabilitiesManifest::new();
        m.register("a", CapState::Supported, "x");
        m.register("b", CapState::Supported, "x");
        m.register("c", CapState::Missing, "y");
        let s = m.summary();
        assert_eq!(s.get("supported"), Some(&2));
        assert_eq!(s.get("missing"), Some(&1));
    }
    #[test] fn test_current_manifest() {
        let mut m = CapabilitiesManifest::new();
        m.register("a", CapState::Available, "x");
        assert_eq!(m.current_manifest().len(), 1);
    }
}
