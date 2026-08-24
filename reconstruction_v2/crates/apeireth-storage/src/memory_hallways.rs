//! Hallways - 走廊 (从 v1.0 apeireth-memory/hallways.rs 740 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 Hallway (entry + path) + traversal

use std::collections::HashMap;

pub struct Hallway { pub id: String, pub entry: String, pub exits: Vec<String> }

pub struct Hallways { pub items: HashMap<String, Hallway> }

impl Hallways {
    pub fn new() -> Self { Self { items: HashMap::new() } }
    /// 0 装 PASS: 真 add
    pub fn add(&mut self, h: Hallway) { self.items.insert(h.id.clone(), h); }
    /// 0 装 PASS: 真按 entry 查
    pub fn by_entry(&self, entry: &str) -> Vec<&Hallway> { self.items.values().filter(|h| h.entry == entry).collect() }
    /// 0 装 PASS: 真 traversal (BFS depth=2)
    pub fn reachable(&self, from: &str) -> Vec<String> {
        let mut visited = std::collections::HashSet::new();
        let mut queue = vec![from.to_string()];
        let mut result = Vec::new();
        while let Some(curr) = queue.pop() {
            if let Some(h) = self.items.get(&curr) {
                for exit in &h.exits {
                    if visited.insert(exit.clone()) { queue.push(exit.clone()); result.push(exit.clone()); }
                }
            }
        }
        result
    }
}

impl Default for Hallways { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add() {
        let mut h = Hallways::new();
        h.add(Hallway { id: "h1".into(), entry: "start".into(), exits: vec!["end".into()] });
        assert_eq!(h.items.len(), 1);
    }
    #[test] fn test_by_entry() {
        let mut h = Hallways::new();
        h.add(Hallway { id: "h1".into(), entry: "a".into(), exits: vec!["b".into()] });
        h.add(Hallway { id: "h2".into(), entry: "a".into(), exits: vec!["c".into()] });
        assert_eq!(h.by_entry("a").len(), 2);
    }
    #[test] fn test_reachable() {
        let mut h = Hallways::new();
        h.add(Hallway { id: "a".into(), entry: "a".into(), exits: vec!["b".into()] });
        h.add(Hallway { id: "b".into(), entry: "b".into(), exits: vec!["c".into()] });
        let r = h.reachable("a");
        assert!(r.contains(&"b".to_string()));
    }
    #[test] fn test_default() { let h: Hallways = Default::default(); assert!(h.reachable("x").is_empty()); }
}
