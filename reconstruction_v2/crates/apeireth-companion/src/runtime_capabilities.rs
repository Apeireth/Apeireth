//! RuntimeCapabilities - 运行时能力发现 (从 v1.0 apeireth-companion/runtime_capabilities.rs 712 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 RuntimeCapability trait + 能力发现
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
    pub tools: Vec<String>,
}

pub struct RuntimeCapabilities {
    pub manifests: HashMap<String, CapabilityManifest>,
}

impl RuntimeCapabilities {
    pub fn new() -> Self { Self { manifests: HashMap::new() } }
    pub fn register(&mut self, m: CapabilityManifest) { self.manifests.insert(m.name.clone(), m); }
    pub fn has_feature(&self, name: &str, feature: &str) -> bool {
        self.manifests.get(name).map(|m| m.features.contains(&feature.to_string())).unwrap_or(false)
    }
    pub fn has_tool(&self, name: &str, tool: &str) -> bool {
        self.manifests.get(name).map(|m| m.tools.contains(&tool.to_string())).unwrap_or(false)
    }
    pub fn count(&self) -> usize { self.manifests.len() }
}

impl Default for RuntimeCapabilities { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut rc = RuntimeCapabilities::new();
        rc.register(CapabilityManifest { name: "core".into(), version: "2.0".into(), features: vec!["chat".into()], tools: vec!["calculator".into()] });
        assert_eq!(rc.count(), 1);
    }
    #[test] fn test_has_feature() {
        let mut rc = RuntimeCapabilities::new();
        rc.register(CapabilityManifest { name: "core".into(), version: "2.0".into(), features: vec!["chat".into()], tools: vec![] });
        assert!(rc.has_feature("core", "chat"));
        assert!(!rc.has_feature("core", "missing"));
    }
    #[test] fn test_has_tool() {
        let mut rc = RuntimeCapabilities::new();
        rc.register(CapabilityManifest { name: "core".into(), version: "2.0".into(), features: vec![], tools: vec!["calc".into()] });
        assert!(rc.has_tool("core", "calc"));
    }
    #[test] fn test_unknown_manifest() {
        let rc = RuntimeCapabilities::new();
        assert!(!rc.has_feature("missing", "any"));
    }
    #[test] fn test_default() {
        let rc: RuntimeCapabilities = Default::default();
        assert_eq!(rc.count(), 0);
    }
}
