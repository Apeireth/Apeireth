//! river_topology: 浪潮流体拓扑动力学与双标度连续场求解器 (DualScaledField)
//!
//! 吸收自 VCP 1.0 / TagMemo V10 (`RiverMemoEngine.js`, `modules/tagmemoV10/`, `rust-vexus-lite`):
//! 1. LIF (Leaky Integrate-and-Fire) 神经元脉冲传导模型，具备软非回溯抑制 (`return_flow_penalty = 0.1`)；
//! 2. 节点内生残差 (Intrinsic Residual) 驱动的非对称张力，张力 >= 0.65 自动激活虫洞跃迁边 (Wormhole, 零动量损耗)；
//! 3. 双预解算子对偶连续场方程求解器：
//!    (I - α_L P_L) u_L = (1 - α_L) s_0 (局域聚焦场)
//!    (I - α_T P_T) u_T = (1 - α_T) s_0 (全域迁移场)
//! 4. DTSC (Dual-Scale Topology Closure) 4 维可观测张量与相对几何闭合度重排；
//! 5. Ω 河网可观测性标量门控三态机 (Collapsed, Sparse, Dense)。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// 拓扑图节点（Tag / 概念）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagNode {
    pub id: u64,
    pub name: String,
    pub vector: Vec<f32>,
    /// 概念内生残差 (0~1)：越不能被邻居解释，独特性与高阶势能越高
    pub intrinsic_residual: f32,
}

/// 有向河道边
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiverEdge {
    pub source_id: u64,
    pub target_id: u64,
    pub conductance: f32,
    pub is_wormhole: bool,
    pub accumulated_flow: f32,
}

/// LIF 脉冲信号包
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpikeSignal {
    pub node_id: u64,
    pub energy: f32,
    pub momentum: f32,
    pub prev_node_id: Option<u64>,
}

/// 浪潮流体拓扑动力学引擎 (LIF 脉冲与虫洞动力学)
#[derive(Debug, Clone)]
pub struct RiverDynamicsEngine {
    pub nodes: HashMap<u64, TagNode>,
    pub adjacency: HashMap<u64, Vec<RiverEdge>>,
    pub base_decay: f32,
    pub wormhole_decay: f32,
    pub return_flow_penalty: f32,
    pub tension_threshold: f32,
}

impl Default for RiverDynamicsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RiverDynamicsEngine {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            adjacency: HashMap::new(),
            base_decay: 0.65,
            wormhole_decay: 0.95,
            return_flow_penalty: 0.10,
            tension_threshold: 0.65,
        }
    }

    pub fn add_node(&mut self, node: TagNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn add_edge(&mut self, source_id: u64, target_id: u64, raw_conductance: f32) {
        let target_ir = self
            .nodes
            .get(&target_id)
            .map(|n| n.intrinsic_residual)
            .unwrap_or(1.0);
        let tension = raw_conductance * target_ir;
        let is_wormhole = tension >= self.tension_threshold;

        let edge = RiverEdge {
            source_id,
            target_id,
            conductance: raw_conductance,
            is_wormhole,
            accumulated_flow: 0.0,
        };
        self.adjacency.entry(source_id).or_default().push(edge);
    }

    /// 执行 LIF 脉冲非回溯传导与能量扩散
    pub fn propagate_spikes(&mut self, seeds: &[(u64, f32)], max_hops: usize) -> HashMap<u64, f32> {
        let mut activated_energies: HashMap<u64, f32> = HashMap::new();
        let mut queue: VecDeque<SpikeSignal> = VecDeque::new();

        for &(seed_id, energy) in seeds {
            queue.push_back(SpikeSignal {
                node_id: seed_id,
                energy,
                momentum: 3.0,
                prev_node_id: None,
            });
            *activated_energies.entry(seed_id).or_default() += energy;
        }

        for _ in 0..max_hops {
            let mut next_queue = VecDeque::new();
            while let Some(spike) = queue.pop_front() {
                if spike.energy < 0.01 || spike.momentum < 0.0 {
                    continue;
                }
                if let Some(edges) = self.adjacency.get_mut(&spike.node_id) {
                    for edge in edges.iter_mut() {
                        let is_return = spike.prev_node_id == Some(edge.target_id);
                        let flow_factor = if is_return {
                            self.return_flow_penalty
                        } else {
                            1.0
                        };
                        let decay = if edge.is_wormhole {
                            self.wormhole_decay
                        } else {
                            self.base_decay
                        };
                        let injected = spike.energy * edge.conductance * decay * flow_factor;

                        if injected < 0.005 {
                            continue;
                        }

                        edge.accumulated_flow += injected;
                        *activated_energies.entry(edge.target_id).or_default() += injected;

                        let next_momentum = if edge.is_wormhole {
                            spike.momentum // 虫洞不消耗动量
                        } else {
                            spike.momentum - 1.0
                        };

                        if next_momentum >= 0.0 || edge.is_wormhole {
                            next_queue.push_back(SpikeSignal {
                                node_id: edge.target_id,
                                energy: injected,
                                momentum: next_momentum,
                                prev_node_id: Some(spike.node_id),
                            });
                        }
                    }
                }
            }
            queue = next_queue;
        }

        activated_energies
    }
}

// =========================================================================
// DualScaledField: 连续双重场偏微分求解器与 DTSC / Ω 度量体系
// =========================================================================

/// DTSC (Dual-Scale Topology Closure) 4 维可观测张量
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DtscObservables {
    /// 直接余弦相似度 (0~1)
    pub direct: f32,
    /// 局域河道结构接触面积积分 (0~1)
    pub structural: f32,
    /// 全图迁移主题亲和力 (0~1)
    pub thematic: f32,
    /// 记忆向量与场加权质心的几何闭合度 (0~1)
    pub closure: f32,
}

impl DtscObservables {
    /// 综合拓扑重排评分
    pub fn compute_composite_score(&self, omega: f32) -> f32 {
        if omega < 0.12 {
            // Collapsed 态：退化为纯向量直接匹配
            self.direct
        } else if omega < 0.45 {
            // Sparse 态：保守拓扑增益
            self.direct * 0.70 + self.structural * 0.20 + self.closure * 0.10
        } else {
            // Dense 态：全拓扑几何重排
            self.direct * 0.35 + self.structural * 0.30 + self.thematic * 0.20 + self.closure * 0.15
        }
    }
}

/// Ω 河网可观测性状态三态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiverState {
    Collapsed, // Ω < 0.12 (纯向量直读)
    Sparse,    // 0.12 <= Ω < 0.45 (保守拓扑)
    Dense,     // Ω >= 0.45 (全量几何重排)
}

/// Ω 河网可观测性度量器
pub struct RiverObservability;

impl RiverObservability {
    /// 计算河网可观测性标量 Ω ∈ [0, 1]
    pub fn measure_omega(
        active_edge_count: usize,
        seed_count: usize,
        reached_node_count: usize,
        edge_flows: &[f32],
    ) -> (f32, RiverState) {
        if seed_count == 0 || active_edge_count == 0 {
            return (0.0, RiverState::Collapsed);
        }

        // 1. 边展开率
        let omega_edge = (active_edge_count as f32 / (2.5 * seed_count as f32)).clamp(0.0, 1.0);

        // 2. 节点涌现率
        let emerged = reached_node_count.saturating_sub(seed_count);
        let omega_emerge = (emerged as f32 / (2.0 * seed_count as f32)).clamp(0.0, 1.0);

        // 3. 流量分布香农信息熵
        let total_flow: f32 = edge_flows.iter().sum();
        let omega_flow = if total_flow > 1e-6 && active_edge_count > 1 {
            let mut entropy = 0.0;
            for &flow in edge_flows {
                let p = flow / total_flow;
                if p > 1e-6 {
                    entropy -= p * p.ln();
                }
            }
            let max_entropy = (active_edge_count as f32).ln().max(1e-6);
            (entropy / max_entropy).clamp(0.0, 1.0)
        } else {
            0.5
        };

        // 几何平均
        let omega = (omega_edge.max(0.01) * omega_emerge.max(0.01) * omega_flow.max(0.01)).cbrt();

        let state = if omega < 0.12 {
            RiverState::Collapsed
        } else if omega < 0.45 {
            RiverState::Sparse
        } else {
            RiverState::Dense
        };

        (omega, state)
    }
}

/// 双预解算子偏微分对偶连续场求解器
pub struct DualScaledFieldSolver {
    pub alpha_local: f32,    // 局域聚焦场阻尼 (默认 0.15)
    pub alpha_transfer: f32, // 全域迁移场阻尼 (默认 0.60)
    pub max_iterations: usize,
    pub tolerance: f32,
}

impl Default for DualScaledFieldSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DualScaledFieldSolver {
    pub fn new() -> Self {
        Self {
            alpha_local: 0.15,
            alpha_transfer: 0.60,
            max_iterations: 50,
            tolerance: 1e-4,
        }
    }

    /// 求解双对偶连续场分布 (u_local, u_transfer)
    pub fn solve(&self, source: &[f32], adjacency_matrix: &[Vec<f32>]) -> (Vec<f32>, Vec<f32>) {
        let n = source.len();
        if n == 0 || adjacency_matrix.len() != n {
            return (vec![], vec![]);
        }

        // 归一化源项分布
        let sum_src: f32 = source.iter().sum();
        let s0: Vec<f32> = if sum_src > 1e-6 {
            source.iter().map(|&x| x / sum_src).collect()
        } else {
            source.to_vec()
        };

        let mut u_local = s0.clone();
        let mut u_transfer = s0.clone();

        for _ in 0..self.max_iterations {
            let mut next_local = vec![0.0f32; n];
            let mut next_transfer = vec![0.0f32; n];

            // 矩阵乘法传播: P * u
            for i in 0..n {
                let mut prop_l = 0.0f32;
                let mut prop_t = 0.0f32;
                for j in 0..n {
                    let w = adjacency_matrix[j][i]; // 转移概率 P_{j->i}
                    prop_l += w * u_local[j];
                    prop_t += w * u_transfer[j];
                }
                // (I - α P) u = (1 - α) s0  ==>  u = (1 - α) s0 + α P u
                next_local[i] = (1.0 - self.alpha_local) * s0[i] + self.alpha_local * prop_l;
                next_transfer[i] =
                    (1.0 - self.alpha_transfer) * s0[i] + self.alpha_transfer * prop_t;
            }

            // 检查 L1 残差收敛
            let res_l: f32 = next_local
                .iter()
                .zip(&u_local)
                .map(|(a, b)| (a - b).abs())
                .sum();
            let res_t: f32 = next_transfer
                .iter()
                .zip(&u_transfer)
                .map(|(a, b)| (a - b).abs())
                .sum();

            u_local = next_local;
            u_transfer = next_transfer;

            if res_l < self.tolerance && res_t < self.tolerance {
                break;
            }
        }

        (u_local, u_transfer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_river_dynamics_spike_propagation_and_wormhole() {
        let mut engine = RiverDynamicsEngine::new();

        // 节点 1 (seed), 节点 2 (中间), 节点 3 (远端概念，高内生残差)
        engine.add_node(TagNode {
            id: 1,
            name: "AI".into(),
            vector: vec![1.0, 0.0],
            intrinsic_residual: 0.5,
        });
        engine.add_node(TagNode {
            id: 2,
            name: "Model".into(),
            vector: vec![0.8, 0.2],
            intrinsic_residual: 0.3,
        });
        engine.add_node(TagNode {
            id: 3,
            name: "Consciousness".into(),
            vector: vec![0.0, 1.0],
            intrinsic_residual: 0.9,
        });

        // 1 -> 2 普通边 (conductance = 0.5, tension = 0.5 * 0.3 = 0.15 < 0.65, 非虫洞)
        engine.add_edge(1, 2, 0.5);
        // 2 -> 3 强张力虫洞边 (conductance = 0.8, tension = 0.8 * 0.9 = 0.72 >= 0.65, 虫洞)
        engine.add_edge(2, 3, 0.8);

        assert!(!engine.adjacency[&1][0].is_wormhole);
        assert!(engine.adjacency[&2][0].is_wormhole);

        let activated = engine.propagate_spikes(&[(1, 1.0)], 3);
        assert!(activated.contains_key(&1));
        assert!(activated.contains_key(&2));
        assert!(activated.contains_key(&3));
        assert!(*activated.get(&3).unwrap() > 0.0);
    }

    #[test]
    fn test_dual_scaled_field_solver_convergence() {
        let solver = DualScaledFieldSolver::new();
        let source = vec![1.0, 0.0, 0.0];
        // 转移矩阵
        let p = vec![
            vec![0.0, 0.5, 0.5],
            vec![0.5, 0.0, 0.5],
            vec![0.5, 0.5, 0.0],
        ];

        let (u_local, u_transfer) = solver.solve(&source, &p);
        assert_eq!(u_local.len(), 3);
        assert_eq!(u_transfer.len(), 3);

        // Local 场聚焦在源节点 (u_local[0] > u_local[1])
        assert!(u_local[0] > u_local[1]);
        // Transfer 场扩散得更深，因此 transfer 中远端节点的能量高于 local 场中的远端能量
        assert!(u_transfer[1] > u_local[1]);
    }

    #[test]
    fn test_dtsc_and_omega_state_gating() {
        let (omega_collapsed, state_collapsed) = RiverObservability::measure_omega(0, 5, 0, &[]);
        assert_eq!(state_collapsed, RiverState::Collapsed);
        assert!(omega_collapsed < 0.12);

        let dtsc = DtscObservables {
            direct: 0.90,
            structural: 0.60,
            thematic: 0.40,
            closure: 0.80,
        };

        // Collapsed 状态直接取 direct 匹配分
        let score_collapsed = dtsc.compute_composite_score(omega_collapsed);
        assert!((score_collapsed - 0.90).abs() < 1e-4);

        // Dense 状态综合 4 维几何重排
        let score_dense = dtsc.compute_composite_score(0.65);
        assert!(score_dense > 0.0 && score_dense < 1.0);
    }
}
