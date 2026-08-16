//! VCP 联想网络 — 河道能量 + 联想传播 (per stage2 §3 + stage1 inspiration §12.3 联想网络).
//!
//! 借鉴 vcptoolbox compound_eye 联想网络: 节点有"能量", 联想传播会激活关联节点.
//!
//! 设计:
//! - 每个节点有能量 [0.0, 1.0]
//! - 联想激活: 节点 A 激活, 邻居 B 能量 += weight * decay
//! - 能量衰减: 每步 × decay_factor (默认 0.9)
//! - 联想查询: 给定关键词, 找出关联节点

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociationNode {
    pub id: Uuid,
    pub label: String,
    pub energy: f64,
}

impl AssociationNode {
    pub fn new(label: impl Into<String>, initial_energy: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: label.into(),
            energy: initial_energy.clamp(0.0, 1.0),
        }
    }

    pub fn boost(&mut self, amount: f64) {
        self.energy = (self.energy + amount).clamp(0.0, 1.0);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AssociationEdge {
    pub weight: f64,
}

impl AssociationEdge {
    pub fn new(weight: f64) -> Self {
        Self {
            weight: weight.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Default)]
pub struct AssociationNetwork {
    nodes: HashMap<Uuid, AssociationNode>,
    adj: HashMap<Uuid, Vec<(Uuid, f64)>>,
    decay_factor: f64,
    propagation_factor: f64,
}

impl AssociationNetwork {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            adj: HashMap::new(),
            decay_factor: 0.9,
            propagation_factor: 0.1,
        }
    }

    pub fn with_decay(mut self, decay: f64) -> Self {
        self.decay_factor = decay.clamp(0.0, 1.0);
        self
    }

    pub fn add_node(&mut self, node: AssociationNode) -> Uuid {
        let id = node.id;
        self.nodes.insert(id, node);
        id
    }

    pub fn connect(&mut self, from: Uuid, to: Uuid, weight: f64) {
        let w = weight.clamp(0.0, 1.0);
        self.adj.entry(from).or_default().push((to, w));
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn seed(&mut self, id: Uuid, energy: f64) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.boost(energy);
        }
    }

    /// 联想传播单步: 从激活节点向邻居传播能量.
    /// 返回本步激活的节点数.
    pub fn propagate_step(&mut self) -> usize {
        let mut updates: Vec<(Uuid, f64)> = Vec::new();
        for (from, neighbors) in &self.adj {
            let from_energy = self.nodes.get(from).map(|n| n.energy).unwrap_or(0.0);
            if from_energy < 0.01 {
                continue;
            }
            for (to, w) in neighbors {
                let bump = from_energy * w * self.propagation_factor;
                updates.push((*to, bump));
            }
        }
        let mut activated = 0;
        for (to, bump) in updates {
            if let Some(n) = self.nodes.get_mut(&to) {
                let before = n.energy;
                n.boost(bump);
                if n.energy > before {
                    activated += 1;
                }
            }
        }
        activated
    }

    /// 应用全局能量衰减 (per step).
    pub fn decay_all(&mut self) {
        for n in self.nodes.values_mut() {
            n.energy *= self.decay_factor;
        }
    }

    /// 联想: 给定 seed 节点, 跑 N 步联想传播, 返回激活节点 (按能量降序).
    pub fn associate(&mut self, seed: Uuid, steps: usize) -> Vec<(Uuid, String, f64)> {
        if !self.nodes.contains_key(&seed) {
            return Vec::new();
        }
        self.seed(seed, 1.0);
        for _ in 0..steps {
            self.propagate_step();
            self.decay_all();
        }
        let mut result: Vec<(Uuid, String, f64)> = self
            .nodes
            .values()
            .map(|n| (n.id, n.label.clone(), n.energy))
            .collect();
        result.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    pub fn node(&self, id: &Uuid) -> Option<&AssociationNode> {
        self.nodes.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_node_returns_unique_id() {
        let mut net = AssociationNetwork::new();
        let id1 = net.add_node(AssociationNode::new("a", 0.0));
        let id2 = net.add_node(AssociationNode::new("b", 0.0));
        assert_ne!(id1, id2);
        assert_eq!(net.node_count(), 2);
    }

    #[test]
    fn seed_boosts_node_energy() {
        let mut net = AssociationNetwork::new();
        let id = net.add_node(AssociationNode::new("a", 0.0));
        net.seed(id, 0.5);
        assert!((net.node(&id).unwrap().energy - 0.5).abs() < 1e-9);
    }

    #[test]
    fn decay_reduces_energy() {
        let mut net = AssociationNetwork::new().with_decay(0.5);
        let id = net.add_node(AssociationNode::new("a", 1.0));
        net.decay_all();
        assert!((net.node(&id).unwrap().energy - 0.5).abs() < 1e-9);
    }

    #[test]
    fn propagate_activates_neighbor() {
        let mut net = AssociationNetwork::new();
        let id1 = net.add_node(AssociationNode::new("a", 1.0));
        let id2 = net.add_node(AssociationNode::new("b", 0.0));
        net.connect(id1, id2, 1.0);
        net.propagate_step();
        let energy_after = net.node(&id2).unwrap().energy;
        assert!(
            energy_after > 0.0,
            "neighbor should be activated, got {}",
            energy_after
        );
    }

    #[test]
    fn associate_returns_sorted_by_energy() {
        let mut net = AssociationNetwork::new();
        let a = net.add_node(AssociationNode::new("a", 0.0));
        let b = net.add_node(AssociationNode::new("b", 0.0));
        let c = net.add_node(AssociationNode::new("c", 0.0));
        net.connect(a, b, 1.0);
        net.connect(b, c, 1.0);
        let result = net.associate(a, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0, a, "highest energy should be seed");
        for i in 1..result.len() {
            assert!(
                result[i - 1].2 >= result[i].2,
                "result must be sorted by energy"
            );
        }
    }

    #[test]
    fn empty_network_associate_returns_empty() {
        let mut net = AssociationNetwork::new();
        let result = net.associate(Uuid::new_v4(), 3);
        assert!(result.is_empty());
    }

    #[test]
    fn weight_clamped_to_unit() {
        let edge = AssociationEdge::new(1.5);
        assert_eq!(edge.weight, 1.0);
        let edge = AssociationEdge::new(-0.5);
        assert_eq!(edge.weight, 0.0);
    }
}
