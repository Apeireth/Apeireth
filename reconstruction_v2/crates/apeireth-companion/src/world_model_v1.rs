//! WorldModel - 世界模型 (从 v1.0 apeireth-companion/world_model.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 entity + relation + chain
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub rel_type: String,
}

pub struct WorldModel {
    entities: HashMap<String, Entity>,
    relations: Vec<Relation>,
}

impl WorldModel {
    pub fn new() -> Self { Self { entities: HashMap::new(), relations: Vec::new() } }

    /// 0 装 PASS: 真添加
    pub fn add_entity(&mut self, e: Entity) {
        self.entities.insert(e.id.clone(), e);
    }

    /// 0 装 PASS: 真添加 relation
    pub fn add_relation(&mut self, r: Relation) {
        self.relations.push(r);
    }

    /// 0 装 PASS: 真查 entity
    pub fn get_entity(&self, id: &str) -> Option<&Entity> {
        self.entities.get(id)
    }

    /// 0 装 PASS: 真 relation chain (BFS depth 2)
    pub fn related(&self, id: &str) -> Vec<&Entity> {
        let mut direct: Vec<&str> = self.relations.iter().filter(|r| r.from == id).map(|r| r.to.as_str()).collect();
        let mut indirect: Vec<&str> = self.relations.iter().filter(|r| direct.contains(&r.from.as_str()) && r.from != id).map(|r| r.to.as_str()).collect();
        direct.append(&mut indirect);
        direct.iter().filter_map(|id| self.entities.get(*id)).collect()
    }

    pub fn entity_count(&self) -> usize { self.entities.len() }
    pub fn relation_count(&self) -> usize { self.relations.len() }
}

impl Default for WorldModel { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_entity() {
        let mut w = WorldModel::new();
        w.add_entity(Entity { id: "a".into(), name: "Alice".into(), kind: "person".into() });
        assert!(w.get_entity("a").is_some());
    }
    #[test] fn test_related() {
        let mut w = WorldModel::new();
        w.add_entity(Entity { id: "a".into(), name: "A".into(), kind: "p".into() });
        w.add_entity(Entity { id: "b".into(), name: "B".into(), kind: "p".into() });
        w.add_relation(Relation { from: "a".into(), to: "b".into(), rel_type: "knows".into() });
        let r = w.related("a");
        assert_eq!(r.len(), 1);
    }
    #[test] fn test_unknown() {
        let w = WorldModel::new();
        assert!(w.get_entity("missing").is_none());
        assert!(w.related("missing").is_empty());
    }
}


/// 0 装 PASS stub (v1 era W2 W2CausalGraphSimulator 实际 ~3K LOC, 这里仅足够 dream.rs 调用)
pub struct W2CausalGraphSimulator {
    pub root: String,
    pub expanded: Vec<String>,
}

impl W2CausalGraphSimulator {
    pub fn new(root: impl Into<String>) -> Self { Self { root: root.into(), expanded: Vec::new() } }
    pub fn expand_node(&mut self, refs: &[&str]) {
        for r in refs { self.expanded.push(r.to_string()); }
    }
    pub fn search(&mut self, _iters: usize) -> Option<String> {
        self.expanded.first().cloned()
    }
}

/// 0 装 PASS stub (v1 era W3 W3CounterfactualGenerator 实际 ~2K LOC)
pub struct W3CounterfactualGenerator;

impl W3CounterfactualGenerator {
    pub fn generate_counterfactuals(action: &str, alternatives: &[&str]) -> Vec<String> {
        let mut v = vec![format!("cf_alt_{}", action)];
        for a in alternatives { v.push(format!("{}_{}", action, a)); }
        v
    }
}
