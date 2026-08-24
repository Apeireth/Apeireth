//! Plugin - 插件管理 (从 v1.0 apeireth-companion/plugin.rs 209 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 Plugin trait + 插件注册表

use std::collections::HashMap;

pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
    fn init(&self) -> Result<(), String>;
    fn shutdown(&self) -> Result<(), String>;
}

pub struct PluginRegistry { pub plugins: HashMap<String, Box<dyn Plugin>> }

impl PluginRegistry {
    pub fn new() -> Self { Self { plugins: HashMap::new() } }
    pub fn register(&mut self, p: Box<dyn Plugin>) -> Result<(), String> {
        if self.plugins.contains_key(p.name()) { return Err(format!("duplicate plugin: {}", p.name())); }
        self.plugins.insert(p.name().to_string(), p);
        Ok(())
    }
    pub fn get(&self, name: &str) -> Option<&Box<dyn Plugin>> { self.plugins.get(name) }
    pub fn count(&self) -> usize { self.plugins.len() }
}

impl Default for PluginRegistry { fn default() -> Self { Self::new() } }

/// 0 装 PASS: 真 mock 插件
pub struct MockPlugin { pub name: String }
impl Plugin for MockPlugin {
    fn name(&self) -> &str { &self.name }
    fn version(&self) -> &str { "1.0.0" }
    fn init(&self) -> Result<(), String> { Ok(()) }
    fn shutdown(&self) -> Result<(), String> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut r = PluginRegistry::new();
        r.register(Box::new(MockPlugin { name: "p1".into() })).unwrap();
        assert_eq!(r.count(), 1);
    }
    #[test] fn test_duplicate() {
        let mut r = PluginRegistry::new();
        r.register(Box::new(MockPlugin { name: "p".into() })).unwrap();
        assert!(r.register(Box::new(MockPlugin { name: "p".into() })).is_err());
    }
    #[test] fn test_get_unknown() {
        let r = PluginRegistry::new();
        assert!(r.get("missing").is_none());
    }
    #[test] fn test_default() { let r: PluginRegistry = Default::default(); assert_eq!(r.count(), 0); }
}
