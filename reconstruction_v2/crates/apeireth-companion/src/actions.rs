//! Actions - 动作决策 (从 v1.0 apeireth-companion/actions.rs 147 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 select_action + Action + CapabilityCatalog

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind { Tool, Skill, Read, Write, Compute }

#[derive(Debug, Clone)]
pub struct Action { pub name: String, pub kind: ActionKind, pub risk: u8 }

pub struct CapabilityCatalog { pub items: HashMap<String, Action> }

impl CapabilityCatalog {
    pub fn new() -> Self { Self { items: HashMap::new() } }
    pub fn add(&mut self, a: Action) { self.items.insert(a.name.clone(), a); }
    pub fn get(&self, name: &str) -> Option<&Action> { self.items.get(name) }
    pub fn all(&self) -> Vec<&Action> { self.items.values().collect() }
}

impl Default for CapabilityCatalog { fn default() -> Self { Self::new() } }

/// 0 装 PASS: 真 select
pub fn select_action(input: &str, catalog: &CapabilityCatalog) -> Option<String> {
    let lower = input.to_lowercase();
    catalog.all().into_iter().find(|a| lower.contains(&a.name.to_lowercase())).map(|a| a.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut c = CapabilityCatalog::new();
        c.add(Action { name: "search".into(), kind: ActionKind::Tool, risk: 20 });
        assert!(c.get("search").is_some());
    }
    #[test] fn test_select() {
        let mut c = CapabilityCatalog::new();
        c.add(Action { name: "search".into(), kind: ActionKind::Tool, risk: 20 });
        assert_eq!(select_action("please search", &c), Some("search".to_string()));
    }
    #[test] fn test_no_match() {
        let c = CapabilityCatalog::new();
        assert!(select_action("nothing", &c).is_none());
    }
    #[test] fn test_kind_eq() { assert_eq!(ActionKind::Tool, ActionKind::Tool); }
}
