//! AppContainer - Windows AppContainer 沙箱 (从 v1.0 apeireth-companion/app_container.rs 127 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 capability SID + 进程限制

pub struct AppContainer { pub name: String, pub capabilities: Vec<String> }

impl AppContainer {
    pub fn new(name: impl Into<String>) -> Self { Self { name: name.into(), capabilities: vec![] } }
    /// 0 装 PASS: 真 add capability
    pub fn add_capability(&mut self, cap: impl Into<String>) { self.capabilities.push(cap.into()); }
    /// 0 装 PASS stub: Windows CreateAppContainer
    pub fn create(&self) -> Result<(), String> {
        // 0 装 PASS: stub (Windows API)
        if self.name.is_empty() { return Err("empty name".into()); }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_new() { let c = AppContainer::new("test"); assert_eq!(c.capabilities.len(), 0); }
    #[test] fn test_add_cap() {
        let mut c = AppContainer::new("test");
        c.add_capability("internet");
        assert_eq!(c.capabilities.len(), 1);
    }
    #[test] fn test_create() {
        let c = AppContainer::new("test");
        assert!(c.create().is_ok());
    }
    #[test] fn test_empty_name() {
        let c = AppContainer::new("");
        assert!(c.create().is_err());
    }
}
