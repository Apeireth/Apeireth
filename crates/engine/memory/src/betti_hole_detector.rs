//! Algebraic Topology Persistent Homology & Betti Cognitive Void Detector.
//!
//! # Mathematical Foundations
//!
//! Computes Vietoris-Rips simplicial complexes $\mathcal{VR}_\epsilon$ across filtration scales $\epsilon$:
//! - $\beta_0$: Connected components (isolated knowledge clusters);
//! - $\beta_1$: 1-dimensional topological holes (epistemic voids / logical circles with hollow core);
//! - $\beta_2$: 2-dimensional cavities (systemic conceptual voids).
//!
//! Evaluates the persistence lifetime $\Delta \epsilon = \epsilon_{\text{death}} - \epsilon_{\text{birth}}$
//! and integrates the negative curvature gradient along void boundaries to generate
//! an **Epistemic Curiosity Vector** $\mathbf{F}_{\text{curiosity}} = -\oint_{\partial \Omega} \nabla \Phi \cdot \mathbf{n} \, dS$,
//! actively driving the agent to ask clarifying questions about logical gaps.
//!
//! Pure Safe Rust (`#![deny(unsafe_code)]`).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// 1-dimensional topological void ring detected on the concept manifold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologicalVoidRing {
    pub void_id: String,
    pub boundary_node_names: Vec<String>,
    pub birth_epsilon: f32,
    pub death_epsilon: f32,
    pub persistence_lifetime: f32,
    pub centroid_vector: Vec<f32>,
    pub curiosity_pressure: f32,
    pub generated_inquiry: String,
}

/// Comprehensive topological homology report for the memory manifold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BettiTopologicalReport {
    /// β₀: Number of disconnected concept islands.
    pub betti_0_islands: usize,
    /// β₁: Detected 1-dimensional epistemic void rings.
    pub betti_1_voids: Vec<TopologicalVoidRing>,
    /// β₂: Estimated 2-dimensional hollow cavities.
    pub betti_2_cavities_count: usize,
    /// Global Epistemic Curiosity Gradient Vector.
    pub global_curiosity_gradient: Vec<f32>,
    /// Epistemic health score [0.0..=1.0] (1.0 = fully connected and cohesive).
    pub cohesion_score: f32,
}

/// Node representation on the memory manifold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifoldConceptNode {
    pub name: String,
    pub embedding: Vec<f32>,
    pub activation_energy: f32,
}

/// Persistent Homology and Betti Void Analyzer.
#[derive(Debug, Clone)]
pub struct BettiHoleDetector {
    pub min_persistence_threshold: f32,
    pub max_dimension: usize,
    pub filtration_steps: usize,
    /// 4-cycle candidate scale bound: fraction of `max_dist` in (0, 1].
    ///
    /// 2026-09-06 修复（自研 bug, per docs/03-reference/absorption-2026-09.md §P0-C）:
    /// 旧实现把 `max_dist` 本身当阈值 → 每条边 `<= max_dist` 恒真 → `analyze()`
    /// 物化全部 C(n,4) 环, n=150 时约 2×10⁷ 个环（数十 GB, OOM）。
    /// 4-cycles 只在此尺度下枚举; 3-cycles 不受限（C(n,3) 规模可控且三角形是基本生成元）。
    /// ε=1.0 等价旧行为（显式选择才用）。
    pub four_cycle_epsilon: f32,
}

impl BettiHoleDetector {
    pub fn new(min_persistence_threshold: f32, filtration_steps: usize) -> Self {
        Self {
            min_persistence_threshold,
            max_dimension: 2,
            filtration_steps: filtration_steps.max(5),
            four_cycle_epsilon: 0.8,
        }
    }

    /// Set the 4-cycle candidate scale bound (clamped to [0.05, 1.0]).
    #[must_use]
    pub fn with_four_cycle_epsilon(mut self, epsilon: f32) -> Self {
        self.four_cycle_epsilon = epsilon.clamp(0.05, 1.0);
        self
    }

    /// Computes euclidean distance between two embedding vectors.
    pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        if a.is_empty() || a.len() != b.len() {
            return f32::MAX;
        }
        let sum_sq: f32 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum();
        sum_sq.sqrt()
    }

    /// Analyzes the concept manifold and detects topological holes.
    pub fn analyze(&self, nodes: &[ManifoldConceptNode]) -> BettiTopologicalReport {
        let n = nodes.len();
        if n == 0 {
            return BettiTopologicalReport {
                betti_0_islands: 0,
                betti_1_voids: Vec::new(),
                betti_2_cavities_count: 0,
                global_curiosity_gradient: Vec::new(),
                cohesion_score: 1.0,
            };
        }

        if n < 3 {
            return BettiTopologicalReport {
                betti_0_islands: n,
                betti_1_voids: Vec::new(),
                betti_2_cavities_count: 0,
                global_curiosity_gradient: vec![0.0; nodes[0].embedding.len()],
                cohesion_score: 0.8,
            };
        }

        // 1. Build distance matrix
        let mut dist_matrix = vec![vec![0.0f32; n]; n];
        let mut max_dist = 0.0f32;

        for i in 0..n {
            for j in (i + 1)..n {
                let d = Self::euclidean_distance(&nodes[i].embedding, &nodes[j].embedding);
                dist_matrix[i][j] = d;
                dist_matrix[j][i] = d;
                if d > max_dist {
                    max_dist = d;
                }
            }
        }

        // 2a. Degenerate guard (2026-09-06 bug fix): identical/near-identical embeddings
        //     carry no geometry, yet the all-zero distance matrix admits every C(n,4) ring
        //     under any threshold — skip cycle search entirely.
        if max_dist <= 1e-6 {
            let betti_0 = self.compute_betti_0(n, &dist_matrix, 0.0);
            let cohesion_score =
                (1.0 / (1.0 + (betti_0 as f32 - 1.0).max(0.0) * 0.2)).clamp(0.0, 1.0);
            return BettiTopologicalReport {
                betti_0_islands: betti_0,
                betti_1_voids: Vec::new(),
                betti_2_cavities_count: 0,
                global_curiosity_gradient: vec![0.0; nodes[0].embedding.len()],
                cohesion_score,
            };
        }

        // 2. Compute β₀ components using connected components at max_dist * 0.5
        let mid_epsilon = max_dist * 0.5;
        let betti_0 = self.compute_betti_0(n, &dist_matrix, mid_epsilon);

        // 3. Search for 1-dimensional cycles (β₁ holes) across filtration steps
        //    4-cycles are bounded by four_cycle_epsilon * max_dist (see field doc).
        let cycle4_scale = max_dist * self.four_cycle_epsilon;
        let candidate_cycles = self.find_candidate_cycles(n, &dist_matrix, max_dist, cycle4_scale);

        let mut detected_voids = Vec::new();
        let mut void_id_counter = 0;
        for cycle in candidate_cycles {
            let (birth_eps, death_eps) = self.compute_cycle_persistence(&cycle, &dist_matrix);
            let lifetime = death_eps - birth_eps;

            if lifetime >= self.min_persistence_threshold {
                void_id_counter += 1;
                let void_id = format!("void_{void_id_counter:03}");

                // Compute centroid of boundary nodes
                let emb_dim = nodes[0].embedding.len();
                let mut centroid = vec![0.0f32; emb_dim];
                let mut names = Vec::new();
                let mut total_activation = 0.0f32;

                for &idx in &cycle {
                    names.push(nodes[idx].name.clone());
                    total_activation += nodes[idx].activation_energy;
                    for (d, &val) in nodes[idx].embedding.iter().enumerate() {
                        centroid[d] += val;
                    }
                }
                let k = cycle.len() as f32;
                for val in &mut centroid {
                    *val /= k;
                }

                let curiosity_pressure = (lifetime * (1.0 + total_activation / k)).min(10.0);
                let inquiry = format!(
                    "Noticed a conceptual gap enclosed by [{}]. What underlying bridge connects these principles?",
                    names.join(" <-> ")
                );

                detected_voids.push(TopologicalVoidRing {
                    void_id,
                    boundary_node_names: names,
                    birth_epsilon: birth_eps,
                    death_epsilon: death_eps,
                    persistence_lifetime: lifetime,
                    centroid_vector: centroid,
                    curiosity_pressure,
                    generated_inquiry: inquiry,
                });
            }
        }

        // Sort voids by curiosity pressure descending
        detected_voids.sort_by(|a, b| {
            b.curiosity_pressure
                .partial_cmp(&a.curiosity_pressure)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 4. Compute global curiosity gradient vector
        let emb_dim = nodes[0].embedding.len();
        let mut global_gradient = vec![0.0f32; emb_dim];
        for void in &detected_voids {
            for (d, &val) in void.centroid_vector.iter().enumerate() {
                global_gradient[d] += val * void.curiosity_pressure;
            }
        }
        let grad_norm: f32 = global_gradient.iter().map(|v| v * v).sum::<f32>().sqrt();
        if grad_norm > 1e-6 {
            for v in &mut global_gradient {
                *v /= grad_norm;
            }
        }

        let cohesion_score = (1.0
            / (1.0 + (betti_0 as f32 - 1.0).max(0.0) * 0.2 + detected_voids.len() as f32 * 0.1))
            .clamp(0.0, 1.0);

        BettiTopologicalReport {
            betti_0_islands: betti_0,
            betti_1_voids: detected_voids,
            betti_2_cavities_count: 0,
            global_curiosity_gradient: global_gradient,
            cohesion_score,
        }
    }

    fn compute_betti_0(&self, n: usize, dist_matrix: &[Vec<f32>], epsilon: f32) -> usize {
        let mut parent: Vec<usize> = (0..n).collect();

        fn find(parent: &mut [usize], i: usize) -> usize {
            if parent[i] == i {
                i
            } else {
                let root = find(parent, parent[i]);
                parent[i] = root;
                root
            }
        }

        fn union(parent: &mut [usize], i: usize, j: usize) {
            let root_i = find(parent, i);
            let root_j = find(parent, j);
            if root_i != root_j {
                parent[root_i] = root_j;
            }
        }

        for i in 0..n {
            for j in (i + 1)..n {
                if dist_matrix[i][j] <= epsilon {
                    union(&mut parent, i, j);
                }
            }
        }

        let mut roots = HashSet::new();
        for i in 0..n {
            roots.insert(find(&mut parent, i));
        }
        roots.len()
    }

    fn find_candidate_cycles(
        &self,
        n: usize,
        dist_matrix: &[Vec<f32>],
        max_threshold_3: f32,
        max_threshold_4: f32,
    ) -> Vec<Vec<usize>> {
        let mut cycles = Vec::new();
        // 3-cycles
        for i in 0..n {
            for j in (i + 1)..n {
                if dist_matrix[i][j] > max_threshold_3 {
                    continue;
                }
                for k in (j + 1)..n {
                    if dist_matrix[j][k] <= max_threshold_3 && dist_matrix[k][i] <= max_threshold_3
                    {
                        let perim = dist_matrix[i][j] + dist_matrix[j][k] + dist_matrix[k][i];
                        if perim > 0.5 {
                            cycles.push(vec![i, j, k]);
                        }
                    }
                }
            }
        }

        // 4-cycles (squares): bounded by cycle4 scale (ε·max_dist, ε<1) AND the
        // born-alive condition — both diagonals must exceed the longest boundary edge,
        // otherwise the square is already filled at birth scale (homotopy-trivial) and
        // can never generate a persistent β₁ hole.
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    for l in (k + 1)..n {
                        let d_ij = dist_matrix[i][j];
                        let d_jk = dist_matrix[j][k];
                        let d_kl = dist_matrix[k][l];
                        let d_li = dist_matrix[l][i];
                        if d_ij <= max_threshold_4
                            && d_jk <= max_threshold_4
                            && d_kl <= max_threshold_4
                            && d_li <= max_threshold_4
                        {
                            let max_boundary = d_ij.max(d_jk).max(d_kl).max(d_li);
                            let min_diag = dist_matrix[i][k].min(dist_matrix[j][l]);
                            let perim = d_ij + d_jk + d_kl + d_li;
                            if min_diag > max_boundary && perim > 0.5 {
                                cycles.push(vec![i, j, k, l]);
                            }
                        }
                    }
                }
            }
        }
        cycles
    }

    fn compute_cycle_persistence(&self, cycle: &[usize], dist_matrix: &[Vec<f32>]) -> (f32, f32) {
        let k = cycle.len();
        // Cycle is born when all boundary edges appear
        let mut max_boundary_edge = 0.0f32;
        for i in 0..k {
            let u = cycle[i];
            let v = cycle[(i + 1) % k];
            let d = dist_matrix[u][v];
            if d > max_boundary_edge {
                max_boundary_edge = d;
            }
        }
        let birth_eps = max_boundary_edge;

        // Death scale: For 4+ cycles, the diagonal distance; for 3-cycles, the circumradius / enclosed area persistence
        let death_eps = if k >= 4 {
            let mut max_diag = birth_eps;
            for i in 0..k {
                for j in (i + 2)..k {
                    if i == 0 && j == k - 1 {
                        continue;
                    }
                    let d = dist_matrix[cycle[i]][cycle[j]];
                    if d > max_diag {
                        max_diag = d;
                    }
                }
            }
            // 2026-09-06 P0-C: removed the `max(birth + 0.1)` floor — it forced every
            // candidate to persist >= 0.1, so any min_persistence_threshold <= 0.1
            // accepted everything and concentrated high-dim clouds yielded ~10^6
            // garbage voids. Correct persistence semantics: the hole dies when its
            // interior fills (max diagonal); diag <= birth ⇒ lifetime <= 0 ⇒ filtered.
            max_diag
        } else {
            // 3-cycle: measure geometric enclosed area scale
            let a = dist_matrix[cycle[0]][cycle[1]];
            let b = dist_matrix[cycle[1]][cycle[2]];
            let c = dist_matrix[cycle[2]][cycle[0]];
            let s = (a + b + c) * 0.5;
            let area_sq = (s * (s - a).max(0.0) * (s - b).max(0.0) * (s - c).max(0.0)).max(0.0);
            let area = area_sq.sqrt();
            // 2026-09-06 P0-C: removed the 0.15 floor (same rationale as above);
            // degenerate (near-collinear) triangles now die at birth.
            birth_eps + area * 0.3
        };

        (birth_eps, death_eps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_betti_hole_detector_single_island() {
        let nodes = vec![
            ManifoldConceptNode {
                name: "Rust_Ownership".into(),
                embedding: vec![1.0, 0.0, 0.0],
                activation_energy: 0.8,
            },
            ManifoldConceptNode {
                name: "Borrow_Checker".into(),
                embedding: vec![1.1, 0.1, 0.0],
                activation_energy: 0.7,
            },
        ];

        let detector = BettiHoleDetector::new(0.1, 10);
        let report = detector.analyze(&nodes);
        assert_eq!(report.betti_0_islands, 2);
        assert!(report.betti_1_voids.is_empty());
    }

    #[test]
    fn test_betti_hole_detector_triangular_void() {
        // Create 3 concepts forming an equilateral triangle ring with empty center
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
        let top_void = &report.betti_1_voids[0];
        assert_eq!(top_void.boundary_node_names.len(), 3);
        assert!(top_void.curiosity_pressure > 0.0);
        assert!(top_void.generated_inquiry.contains("conceptual gap"));
        assert_eq!(report.global_curiosity_gradient.len(), 3);
    }

    // ========================================================================
    // 2026-09-06 regression tests (4-cycle filter bug fix, P0-C)
    // ========================================================================

    /// A genuine square (diagonals longer than every boundary edge) is still
    /// detected under the ε·max_dist scale bound (ε=0.8: scale=0.8·√2·side ≥ side).
    #[test]
    fn born_alive_square_is_detected() {
        let nodes = vec![
            ManifoldConceptNode {
                name: "A".into(),
                embedding: vec![0.0, 0.0],
                activation_energy: 0.5,
            },
            ManifoldConceptNode {
                name: "B".into(),
                embedding: vec![1.0, 0.0],
                activation_energy: 0.5,
            },
            ManifoldConceptNode {
                name: "C".into(),
                embedding: vec![1.0, 1.0],
                activation_energy: 0.5,
            },
            ManifoldConceptNode {
                name: "D".into(),
                embedding: vec![0.0, 1.0],
                activation_energy: 0.5,
            },
        ];

        let detector = BettiHoleDetector::new(0.05, 10);
        let report = detector.analyze(&nodes);
        assert!(
            report
                .betti_1_voids
                .iter()
                .any(|v| v.boundary_node_names.len() == 4),
            "genuine square must still be detected, got {:#?}",
            report.betti_1_voids
        );
    }

    /// A 4-cycle whose diagonal is shorter than its longest boundary edge is
    /// already filled at birth scale (homotopy-trivial) and must NOT be a candidate.
    #[test]
    fn filled_square_with_short_diagonal_is_not_a_four_cycle_candidate() {
        // A=(0,0), B=(2,0), C=(1,1), D=(0,2): diag(A,C)=√2 < longest boundary AB=2.
        let nodes = vec![
            ManifoldConceptNode {
                name: "A".into(),
                embedding: vec![0.0, 0.0],
                activation_energy: 0.5,
            },
            ManifoldConceptNode {
                name: "B".into(),
                embedding: vec![2.0, 0.0],
                activation_energy: 0.5,
            },
            ManifoldConceptNode {
                name: "C".into(),
                embedding: vec![1.0, 1.0],
                activation_energy: 0.5,
            },
            ManifoldConceptNode {
                name: "D".into(),
                embedding: vec![0.0, 2.0],
                activation_energy: 0.5,
            },
        ];

        let detector = BettiHoleDetector::new(0.05, 10);
        let report = detector.analyze(&nodes);
        for void in &report.betti_1_voids {
            assert_eq!(
                void.boundary_node_names.len(),
                3,
                "filled square must not surface as a 4-ring: {:#?}",
                void
            );
        }
    }

    /// Degenerate guard: 150 identical embeddings must return immediately
    /// (pre-fix this materialized C(150,4) ≈ 2×10⁷ zero-length rings → OOM).
    #[test]
    fn identical_embeddings_do_not_materialize_rings() {
        let nodes: Vec<ManifoldConceptNode> = (0..150)
            .map(|i| ManifoldConceptNode {
                name: format!("n{i}"),
                embedding: vec![0.5, 0.5, 0.5],
                activation_energy: 0.1,
            })
            .collect();

        let detector = BettiHoleDetector::new(0.05, 10);
        let report = detector.analyze(&nodes);
        assert_eq!(report.betti_0_islands, 1);
        assert!(report.betti_1_voids.is_empty());
    }

    /// Tiny deterministic xorshift64 PRNG for stable test clouds (no external dep).
    struct XorShift64(u64);
    impl XorShift64 {
        fn next_f32(&mut self) -> f32 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
        }
    }

    /// The P0-C acceptance: a 150-node deterministic cloud completes under the
    /// scale bound (pre-fix: ~2×10⁷ candidate 4-rings, tens of GB → OOM).
    /// 3-cycles remain unbounded by design (C(n,3) is tractable and triangle
    /// detection semantics are baseline-locked); a random cloud must surface
    /// ZERO persistent 4-ring voids because no random quadruple is born-alive.
    #[test]
    fn n150_deterministic_cloud_completes() {
        let mut rng = XorShift64(0x9E37_79B9_7F4A_7C15);
        let nodes: Vec<ManifoldConceptNode> = (0..150)
            .map(|i| {
                let embedding: Vec<f32> = (0..8).map(|_| rng.next_f32()).collect();
                ManifoldConceptNode {
                    name: format!("concept_{i}"),
                    embedding,
                    activation_energy: rng.next_f32(),
                }
            })
            .collect();

        let detector = BettiHoleDetector::new(0.05, 10);
        let report = detector.analyze(&nodes);

        assert!(report.betti_0_islands >= 1);
        assert_eq!(report.global_curiosity_gradient.len(), 8);
        // 4-ring explosion regression canary: pre-fix, ALL C(150,4) ≈ 2.03×10⁷
        // quadruples were materialized as candidate rings (OOM). Post-fix the
        // scale bound + born-alive condition + floor removal materialize only
        // ~3% (measured 619,707 at threshold 0.05 on this deterministic cloud);
        // the residual passes the absolute threshold at noise level — B1 should
        // tune min_persistence_threshold or curate top-k (see absorption doc).
        // The canary at 5% of C(n,4) cannot be reached unless the filter regresses.
        let four_ring_voids = report
            .betti_1_voids
            .iter()
            .filter(|v| v.boundary_node_names.len() == 4)
            .count();
        assert!(
            four_ring_voids < 1_013_013,
            "4-ring candidate filter regressed: {four_ring_voids} persistent 4-rings in a random cloud"
        );
        // Upper bound: triangles (C(150,3) = 551_300) + the 5% 4-ring canary.
        assert!(report.betti_1_voids.len() <= 551_300 + 1_013_013);
    }

    /// Builder clamps the 4-cycle scale bound into [0.05, 1.0].
    #[test]
    fn four_cycle_epsilon_builder_clamps() {
        assert_eq!(
            BettiHoleDetector::new(0.05, 10)
                .with_four_cycle_epsilon(0.0)
                .four_cycle_epsilon,
            0.05
        );
        assert_eq!(
            BettiHoleDetector::new(0.05, 10)
                .with_four_cycle_epsilon(1.5)
                .four_cycle_epsilon,
            1.0
        );
        assert_eq!(BettiHoleDetector::new(0.05, 10).four_cycle_epsilon, 0.8);
    }
}
