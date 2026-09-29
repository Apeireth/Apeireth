//! vendored Betti 三角洞测试 (复用 betti_hole_detector.rs 自带测试语义)。
//!
//! 用与引擎源码 `test_betti_hole_detector_triangular_void` 相同的等边三角形 fixture,
//! 验证 vendored 副本通过公开 API 行为一致: 3 节点成 1 个 β1 环。

use apeireth_research_b1::topo::betti_hole_detector::{BettiHoleDetector, ManifoldConceptNode};

#[test]
fn vendored_betti_triangular_void() {
    let nodes = vec![
        ManifoldConceptNode {
            name: "Cryptography_Merkle".into(),
            embedding: vec![0.0, 0.0, 0.0],
            activation_energy: 0.9,
        },
        ManifoldConceptNode {
            name: "Distributed_Raft".into(),
            embedding: vec![2.0, 0.0, 0.0],
            activation_energy: 0.8,
        },
        ManifoldConceptNode {
            name: "Byzantine_Fault".into(),
            embedding: vec![1.0, 1.732, 0.0],
            activation_energy: 0.85,
        },
    ];

    let detector = BettiHoleDetector::new(0.05, 10);
    let report = detector.analyze(&nodes);

    assert!(!report.betti_1_voids.is_empty());
    let top = &report.betti_1_voids[0];
    assert_eq!(top.boundary_node_names.len(), 3);
    assert!(top.curiosity_pressure > 0.0);
    assert!(top.generated_inquiry.contains("conceptual gap"));
    assert_eq!(report.global_curiosity_gradient.len(), 3);
    // 诚实边界 (协议 §8.2): β2 未实现, 恒为 0。
    assert_eq!(report.betti_2_cavities_count, 0);
}
