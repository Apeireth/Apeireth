//! 图构造 (协议 §2.2): 节点准备 + 节点上限截断 + ManifoldConceptNode 组装。

use crate::embed::l2_normalize;
use crate::topo::betti_hole_detector::ManifoldConceptNode;
use std::collections::HashMap;

/// 构图前的一个节点输入 (尚未绑定嵌入向量)。
#[derive(Debug, Clone)]
pub struct NodeInput {
    pub dia_id: String,
    pub text: String,
    pub created_turn: usize,
    pub activation_energy: f32,
}

/// activation_energy = clamp(evidence_citation_count / 5.0, 0.0, 1.0) (协议 §2.2)。
pub fn activation_energy(evidence_citations: usize) -> f32 {
    (evidence_citations as f32 / 5.0).clamp(0.0, 1.0)
}

/// 节点上限硬约束 (协议 §2.2): n_s > max_nodes 时按 activation_energy 降序截断,
/// 平局按 created_turn 新者优先, 再按 dia_id 兜底 (确定性)。
/// 返回 (截断后节点, n_raw)。
pub fn truncate_nodes(mut inputs: Vec<NodeInput>, max_nodes: usize) -> (Vec<NodeInput>, usize) {
    let n_raw = inputs.len();
    if n_raw <= max_nodes {
        return (inputs, n_raw);
    }
    inputs.sort_by(|a, b| {
        b.activation_energy
            .partial_cmp(&a.activation_energy)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.created_turn.cmp(&a.created_turn))
            .then_with(|| a.dia_id.cmp(&b.dia_id))
    });
    inputs.truncate(max_nodes);
    (inputs, n_raw)
}

/// 用 dia_id → 嵌入向量 组装 ManifoldConceptNode (嵌入 L2 归一化)。
/// 缺嵌入的 dia_id 直接 panic (调用方须先校验完整性)。
pub fn build_nodes(inputs: &[NodeInput], embeddings: &HashMap<String, Vec<f32>>) -> Vec<ManifoldConceptNode> {
    inputs
        .iter()
        .map(|i| {
            let mut emb = embeddings
                .get(&i.dia_id)
                .unwrap_or_else(|| panic!("缺失嵌入: {}", i.dia_id))
                .clone();
            l2_normalize(&mut emb);
            ManifoldConceptNode {
                name: i.dia_id.clone(),
                embedding: emb,
                activation_energy: i.activation_energy,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, ae: f32, turn: usize) -> NodeInput {
        NodeInput {
            dia_id: id.into(),
            text: String::new(),
            created_turn: turn,
            activation_energy: ae,
        }
    }

    #[test]
    fn activation_energy_clamps_at_1() {
        assert_eq!(activation_energy(0), 0.0);
        assert_eq!(activation_energy(3), 0.6);
        assert_eq!(activation_energy(5), 1.0);
        assert_eq!(activation_energy(99), 1.0);
    }

    #[test]
    fn truncate_below_limit_is_identity() {
        let v = vec![node("a", 0.0, 0), node("b", 1.0, 1)];
        let (out, n_raw) = truncate_nodes(v, 150);
        assert_eq!(n_raw, 2);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn truncate_keeps_high_activation_then_newer_turn() {
        // 4 个节点, 上限 2: 保留 activation 最高的 2 个; 平局按 created_turn 新者优先。
        let v = vec![
            node("low0", 0.0, 0),
            node("high0", 1.0, 1),
            node("mid0", 0.5, 2),
            node("high1", 1.0, 3),
        ];
        let (out, n_raw) = truncate_nodes(v, 2);
        assert_eq!(n_raw, 4);
        let ids: Vec<_> = out.iter().map(|n| n.dia_id.as_str()).collect();
        // high1 (turn 3) 比 high0 (turn 1) 新 → 排前; 两者都高于 mid0。
        assert_eq!(ids, vec!["high1", "high0"]);
    }

    #[test]
    fn build_nodes_l2_normalizes_embeddings() {
        let inputs = vec![node("a", 0.5, 0)];
        let mut embs = HashMap::new();
        embs.insert("a".to_string(), vec![3.0, 4.0]);
        let nodes = build_nodes(&inputs, &embs);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, "a");
        assert!((nodes[0].embedding[0] - 0.6).abs() < 1e-6);
        assert!((nodes[0].embedding[1] - 0.8).abs() < 1e-6);
    }
}
