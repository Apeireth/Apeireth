//! Suites - 套件 (从 v1.0 apeireth-companion/suites.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 SuiteCatalog + expiry check
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SuiteKind { Eval, Smoke, Integration, Custom }

#[derive(Debug, Clone)]
pub struct SuiteDef {
    pub name: String,
    pub kind: SuiteKind,
    pub ttl_ms: i64,
}

pub struct SuiteCatalog {
    suites: HashMap<String, SuiteDef>,
}

impl SuiteCatalog {
    pub fn new() -> Self { Self { suites: HashMap::new() } }

    /// 0 装 PASS: 真注册
    pub fn register(&mut self, def: SuiteDef) {
        self.suites.insert(def.name.clone(), def);
    }

    /// 0 装 PASS: 真 expiry check
    pub fn check_expiry(&self, name: &str, last_run_ms: i64, now_ms: i64) -> bool {
        self.suites.get(name).map(|s| now_ms - last_run_ms > s.ttl_ms).unwrap_or(false)
    }

    pub fn count(&self) -> usize { self.suites.len() }
}

impl Default for SuiteCatalog { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut c = SuiteCatalog::new();
        c.register(SuiteDef { name: "smoke".into(), kind: SuiteKind::Smoke, ttl_ms: 3600_000 });
        assert_eq!(c.count(), 1);
    }
    #[test] fn test_expiry_expired() {
        let mut c = SuiteCatalog::new();
        c.register(SuiteDef { name: "smoke".into(), kind: SuiteKind::Smoke, ttl_ms: 1000 });
        assert!(c.check_expiry("smoke", 0, 2000));
    }
    #[test] fn test_expiry_fresh() {
        let mut c = SuiteCatalog::new();
        c.register(SuiteDef { name: "smoke".into(), kind: SuiteKind::Smoke, ttl_ms: 10000 });
        assert!(!c.check_expiry("smoke", 5000, 6000));
    }
    #[test] fn test_unknown() {
        let c = SuiteCatalog::new();
        assert!(!c.check_expiry("missing", 0, 1000));
    }
    #[test] fn test_kind_eq() {
        assert_eq!(SuiteKind::Smoke, SuiteKind::Smoke);
    }
}
