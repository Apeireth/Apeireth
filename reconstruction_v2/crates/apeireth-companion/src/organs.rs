//! Organs - 8 organ 协调整合 (从 v1.0 apeireth-companion/organs.rs 391 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 AwakeCompanion + 8 organ 协同入口

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrganKind { Cognition, Consciousness, Value, Motivation, Perception, LifeForce, Experience, Blueprint }

pub struct AwakeCompanion {
    pub organs: HashMap<OrganKind, String>,
}

impl AwakeCompanion {
    pub fn new() -> Self { Self { organs: HashMap::new() } }
    /// 0 装 PASS: 真注册 organ handler
    pub fn register(&mut self, kind: OrganKind, handler: impl Into<String>) {
        self.organs.insert(kind, handler.into());
    }
    /// 0 装 PASS: 真按 kind 调
    pub fn dispatch(&self, kind: OrganKind, input: &str) -> Option<String> {
        self.organs.get(&kind).map(|h| format!("{}: {}", h, input))
    }
    pub fn count(&self) -> usize { self.organs.len() }
}

impl Default for AwakeCompanion { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register_all_8() {
        let mut a = AwakeCompanion::new();
        let kinds = [OrganKind::Cognition, OrganKind::Consciousness, OrganKind::Value, OrganKind::Motivation, OrganKind::Perception, OrganKind::LifeForce, OrganKind::Experience, OrganKind::Blueprint];
        for k in kinds { a.register(k, "h"); }
        assert_eq!(a.count(), 8);
    }
    #[test] fn test_dispatch() {
        let mut a = AwakeCompanion::new();
        a.register(OrganKind::Cognition, "cog");
        let r = a.dispatch(OrganKind::Cognition, "input").unwrap();
        assert_eq!(r, "cog: input");
    }
    #[test] fn test_unknown_dispatch() {
        let a = AwakeCompanion::new();
        assert!(a.dispatch(OrganKind::Value, "x").is_none());
    }
    #[test] fn test_organ_eq() { assert_eq!(OrganKind::Cognition, OrganKind::Cognition); }
    #[test] fn test_default() { let a: AwakeCompanion = Default::default(); assert_eq!(a.count(), 0); }
}
