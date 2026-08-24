//! Lbr (Library Governance) - 库加载管理 (从 v1.0 apeireth-library-governance 4K LOC 收敛)
//!
//! 0 装 PASS: 简化库注册表 (name + version + path), 完整 v1.0 era (依赖解析, semver) 不做.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub name: String,
    pub version: String,
    pub path: String,
    pub enabled: bool,
}

#[derive(Default)]
pub struct LibraryRegistry {
    libs: HashMap<String, LibraryEntry>,
}

impl LibraryRegistry {
    pub fn new() -> Self { Self::default() }
    /// 0 装 PASS: 真注册 (HashMap), 重复 name 覆盖 (last-wins)
    pub fn register(&mut self, entry: LibraryEntry) {
        self.libs.insert(entry.name.clone(), entry);
    }
    pub fn get(&self, name: &str) -> Option<&LibraryEntry> { self.libs.get(name) }
    pub fn list(&self) -> Vec<&LibraryEntry> { self.libs.values().collect() }
    pub fn disable(&mut self, name: &str) { if let Some(e) = self.libs.get_mut(name) { e.enabled = false; } }
    pub fn enable(&mut self, name: &str) { if let Some(e) = self.libs.get_mut(name) { e.enabled = true; } }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register_get() {
        let mut r = LibraryRegistry::new();
        r.register(LibraryEntry { name: "tokio".into(), version: "1.34".into(), path: "/lib".into(), enabled: true });
        let e = r.get("tokio").unwrap();
        assert_eq!(e.version, "1.34");
    }
    #[test] fn test_disable_enable() {
        let mut r = LibraryRegistry::new();
        r.register(LibraryEntry { name: "x".into(), version: "1".into(), path: "/p".into(), enabled: true });
        r.disable("x");
        assert!(!r.get("x").unwrap().enabled);
        r.enable("x");
        assert!(r.get("x").unwrap().enabled);
    }
}
