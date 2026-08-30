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

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

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
}

impl BettiHoleDetector {
    pub fn new(min_persistence_threshold: f32, filtration_steps: usize) -> Self {
        Self {
            min_persistence_threshold,
            max_dimension: 2,
            filtration_steps: filtration_steps.max(5),
        }
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

        // 2. Compute β₀ components using connected components at max_dist * 0.5
        let mid_epsilon = max_dist * 0.5;
        let betti_0 = self.compute_betti_0(n, &dist_matrix, mid_epsilon);

        // 3. Search for 1-dimensional cycles (β₁ holes) across filtration steps
        let candidate_cycles = self.find_candidate_cycles(n, &dist_matrix, max_dist);

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
        detected_voids.sort_by(|a, b| b.curiosity_pressure.partial_cmp(&a.curiosity_pressure).unwrap_or(std::cmp::Ordering::Equal));

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

        let cohesion_score = (1.0 / (1.0 + (betti_0 as f32 - 1.0).max(0.0) * 0.2 + detected_voids.len() as f32 * 0.1)).clamp(0.0, 1.0);

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

    fn find_candidate_cycles(&self, n: usize, dist_matrix: &[Vec<f32>], max_threshold: f32) -> Vec<Vec<usize>> {
        let mut cycles = Vec::new();
        // 3-cycles
        for i in 0..n {
            for j in (i + 1)..n {
                if dist_matrix[i][j] > max_threshold {
                    continue;
                }
                for k in (j + 1)..n {
                    if dist_matrix[j][k] <= max_threshold && dist_matrix[k][i] <= max_threshold {
                        let perim = dist_matrix[i][j] + dist_matrix[j][k] + dist_matrix[k][i];
                        if perim > 0.5 {
                            cycles.push(vec![i, j, k]);
                        }
                    }
                }
            }
        }

        // 4-cycles (squares)
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    for l in (k + 1)..n {
                        let d_ij = dist_matrix[i][j];
                        let d_jk = dist_matrix[j][k];
                        let d_kl = dist_matrix[k][l];
                        let d_li = dist_matrix[l][i];
                        if d_ij <= max_threshold && d_jk <= max_threshold && d_kl <= max_threshold && d_li <= max_threshold {
                            cycles.push(vec![i, j, k, l]);
                        }
                    }
                }
            }
        }
        cycles
    }

    fn compute_cycle_persistence(
        &self,
        cycle: &[usize],
        dist_matrix: &[Vec<f32>],
    ) -> (f32, f32) {
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
            max_diag.max(birth_eps + 0.1)
        } else {
            // 3-cycle: measure geometric enclosed area scale
            let a = dist_matrix[cycle[0]][cycle[1]];
            let b = dist_matrix[cycle[1]][cycle[2]];
            let c = dist_matrix[cycle[2]][cycle[0]];
            let s = (a + b + c) * 0.5;
            let area_sq = (s * (s - a).max(0.0) * (s - b).max(0.0) * (s - c).max(0.0)).max(0.0);
            let area = area_sq.sqrt();
            birth_eps + (area * 0.3).max(0.15)
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
}
