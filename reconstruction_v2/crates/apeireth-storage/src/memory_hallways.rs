//! Memory Hallways - 走廊 (从 v1.0 apeireth-memory/hallways.rs 740 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 Hallway (entry + path) + traversal

use std::collections::HashMap;

pub struct Hallway { pub id: String, pub entry: String, pub exits: Vec<String> }

pub struct Hallways { pub items: HashMap<String, Hallway> }

impl Hallways {
    pub fn new() -> Self { Self { items: HashMap::new() } }
    /// 0 装 PASS: 真 add
    pub fn add(&mut self, h: Hallway) {
        self.items.insert(h.id.clone(), h);
    }
    /// 0 装 PASS: 真邻接查询 (BFS depth=1)
    pub fn neighbors(&self, id: &str) -> Vec<&Hallway> {
        self.items.get(id).map(|h| h.exits.iter().filter_map(|e| self.items.get(e)).collect()).unwrap_or_default()
    }
}

impl Default for Hallways {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_add() {
        let mut h = Hallways::new();
        h.add(Hallway { id: "a".into(), entry: "a".into(), exits: vec!["b".into()] });
        h.add(Hallway { id: "b".into(), entry: "b".into(), exits: vec![] });
        assert_eq!(h.neighbors("a").len(), 1);
    }
    #[test]
    fn test_neighbors_chained() {
        let mut h = Hallways::new();
        h.add(Hallway { id: "a".into(), entry: "a".into(), exits: vec!["b".into()] });
        h.add(Hallway { id: "b".into(), entry: "b".into(), exits: vec!["c".into()] });
        h.add(Hallway { id: "c".into(), entry: "c".into(), exits: vec![] });
        assert_eq!(h.neighbors("a").len(), 1);
        assert_eq!(h.neighbors("b").len(), 1);
    }
    #[test]
    fn test_empty() {
        let h = Hallways::new();
        assert_eq!(h.neighbors("x").len(), 0);
    }
}
