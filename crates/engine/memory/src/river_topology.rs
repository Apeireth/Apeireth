//! river_topology: 浪潮流体拓扑动力学与双标度连续场求解器 (DualScaledField)
//!
//! 本模块为独立实现，数学基础均为公开文献中的经典模型：
//! 1. LIF (Leaky Integrate-and-Fire) 脉冲传导模型
//!    [Gerstner & Kistler, *Spiking Neuron Models*, Cambridge, 2002]，
//!    在此之上叠加软回溯抑制（回溯边流量按 `return_flow_penalty` 折减）；
//! 2. 节点内生残差 (Intrinsic Residual) 驱动的非对称张力判据：
//!    `tension = conductance × intrinsic_residual`，超过阈值的边升级为
//!    虫洞跃迁边（零动量损耗、低衰减）；
//! 3. 双预解算子对偶连续场方程的定点迭代求解：
//!    (I − α_L P) u_L = (1 − α_L) s₀ (局域聚焦场)
//!    (I − α_T P) u_T = (1 − α_T) s₀ (全域迁移场)
//! 4. DTSC (Dual-Scale Topology Closure) 4 维可观测张量与相对几何闭合度重排；
//! 5. Ω 河网可观测性标量门控三态机 (Collapsed / Sparse / Dense)。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 种子脉冲的初始动量。
const SEED_MOMENTUM: f32 = 3.0;
/// 脉冲能量下限：低于此值不再向外传导。
const ENERGY_FLOOR: f32 = 0.01;
/// 单边注入电流下限：低于此值不计流量。
const INJECTION_FLOOR: f32 = 0.005;
/// 普通边每次跃迁消耗的动量。
const MOMENTUM_COST: f32 = 1.0;
/// Ω 三态机阈值：低于此值为 Collapsed。
const OMEGA_COLLAPSED_AT: f32 = 0.12;
/// Ω 三态机阈值：达到此值为 Dense。
const OMEGA_DENSE_AT: f32 = 0.45;
/// 边展开率标定常数（活跃边数 / (EDGE_SPREAD × 种子数)）。
const EDGE_SPREAD: f32 = 2.5;
/// 节点涌现率标定常数（新增节点数 / (EMERGE_SPREAD × 种子数)）。
const EMERGE_SPREAD: f32 = 2.0;

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

/// 一次跃迁的传导结果（内部暂存，随后统一入账）。
struct Transmission {
    edge_origin: u64,
    edge_slot: usize,
    target: u64,
    injected: f32,
    wormhole: bool,
    next_momentum: f32,
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
        let target_residual = self
            .nodes
            .get(&target_id)
            .map(|node| node.intrinsic_residual)
            .unwrap_or(1.0);
        let is_wormhole = raw_conductance * target_residual >= self.tension_threshold;

        let edge = RiverEdge {
            source_id,
            target_id,
            conductance: raw_conductance,
            is_wormhole,
            accumulated_flow: 0.0,
        };
        self.adjacency.entry(source_id).or_default().push(edge);
    }

    /// 执行 LIF 脉冲非回溯传导与能量扩散。
    ///
    /// 每一波（hop）分两步：先不可变地算出全部跃迁电流，再统一写入边流量
    /// 并生成下一波前沿；动量耗尽或能量跌破下限的脉冲自然熄灭。
    pub fn propagate_spikes(&mut self, seeds: &[(u64, f32)], max_hops: usize) -> HashMap<u64, f32> {
        let mut activation: HashMap<u64, f32> = HashMap::new();
        let mut frontier: Vec<SpikeSignal> = Vec::new();

        for &(seed_id, energy) in seeds {
            frontier.push(SpikeSignal {
                node_id: seed_id,
                energy,
                momentum: SEED_MOMENTUM,
                prev_node_id: None,
            });
            *activation.entry(seed_id).or_default() += energy;
        }

        for _ in 0..max_hops {
            let transmissions = self.collect_transmissions(&frontier);
            frontier = self.settle(transmissions, &mut activation);
        }

        activation
    }

    /// 计算当前波前沿所有脉冲的跃迁电流（纯读取，不改状态）。
    fn collect_transmissions(&self, frontier: &[SpikeSignal]) -> Vec<Transmission> {
        let mut out = Vec::new();
        for spike in frontier {
            if spike.energy < ENERGY_FLOOR || spike.momentum < 0.0 {
                continue;
            }
            let Some(edges) = self.adjacency.get(&spike.node_id) else {
                continue;
            };
            for (slot, edge) in edges.iter().enumerate() {
                let backflow = spike.prev_node_id == Some(edge.target_id);
                let flow_factor = if backflow {
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
                if injected < INJECTION_FLOOR {
                    continue;
                }
                let next_momentum = if edge.is_wormhole {
                    spike.momentum
                } else {
                    spike.momentum - MOMENTUM_COST
                };
                out.push(Transmission {
                    edge_origin: spike.node_id,
                    edge_slot: slot,
                    target: edge.target_id,
                    injected,
                    wormhole: edge.is_wormhole,
                    next_momentum,
                });
            }
        }
        out
    }

    /// 把跃迁电流入账（边流量 + 节点激活），并产出下一波前沿。
    fn settle(
        &mut self,
        transmissions: Vec<Transmission>,
        activation: &mut HashMap<u64, f32>,
    ) -> Vec<SpikeSignal> {
        let mut next_wave = Vec::new();
        for t in transmissions {
            if let Some(edges) = self.adjacency.get_mut(&t.edge_origin) {
                if let Some(edge) = edges.get_mut(t.edge_slot) {
                    edge.accumulated_flow += t.injected;
                }
            }
            *activation.entry(t.target).or_default() += t.injected;
            if t.next_momentum >= 0.0 || t.wormhole {
                next_wave.push(SpikeSignal {
                    node_id: t.target,
                    energy: t.injected,
                    momentum: t.next_momentum,
                    prev_node_id: Some(t.edge_origin),
                });
            }
        }
        next_wave
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

/// Sparse 态的 4 维混合权重 [direct, structural, thematic, closure]。
const SPARSE_WEIGHTS: [f32; 4] = [0.70, 0.20, 0.00, 0.10];
/// Dense 态的 4 维混合权重 [direct, structural, thematic, closure]。
const DENSE_WEIGHTS: [f32; 4] = [0.35, 0.30, 0.20, 0.15];

impl DtscObservables {
    /// 综合拓扑重排评分：按 Ω 门控态选择混合权重。
    pub fn compute_composite_score(&self, omega: f32) -> f32 {
        match RiverState::classify(omega) {
            // Collapsed：退化为纯向量直接匹配
            RiverState::Collapsed => self.direct,
            RiverState::Sparse => self.blend(&SPARSE_WEIGHTS),
            RiverState::Dense => self.blend(&DENSE_WEIGHTS),
        }
    }

    fn blend(&self, weights: &[f32; 4]) -> f32 {
        weights[0] * self.direct
            + weights[1] * self.structural
            + weights[2] * self.thematic
            + weights[3] * self.closure
    }
}

/// Ω 河网可观测性状态三态机
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiverState {
    Collapsed, // Ω < 0.12 (纯向量直读)
    Sparse,    // 0.12 <= Ω < 0.45 (保守拓扑)
    Dense,     // Ω >= 0.45 (全量几何重排)
}

impl RiverState {
    /// 按 Ω 标量归类门控态。
    pub fn classify(omega: f32) -> Self {
        if omega < OMEGA_COLLAPSED_AT {
            RiverState::Collapsed
        } else if omega < OMEGA_DENSE_AT {
            RiverState::Sparse
        } else {
            RiverState::Dense
        }
    }
}

/// Ω 河网可观测性度量器
pub struct RiverObservability;

impl RiverObservability {
    /// 计算河网可观测性标量 Ω ∈ [0, 1]（三分量几何平均）。
    pub fn measure_omega(
        active_edge_count: usize,
        seed_count: usize,
        reached_node_count: usize,
        edge_flows: &[f32],
    ) -> (f32, RiverState) {
        if seed_count == 0 || active_edge_count == 0 {
            return (0.0, RiverState::Collapsed);
        }

        let expansion = Self::edge_expansion(active_edge_count, seed_count);
        let emergence = Self::node_emergence(reached_node_count, seed_count);
        let balance = Self::flow_balance(edge_flows, active_edge_count);

        let omega = (expansion.max(0.01) * emergence.max(0.01) * balance.max(0.01)).cbrt();
        (omega, RiverState::classify(omega))
    }

    /// 1. 边展开率：活跃边数相对种子数的展开程度。
    fn edge_expansion(active_edges: usize, seeds: usize) -> f32 {
        (active_edges as f32 / (EDGE_SPREAD * seeds as f32)).clamp(0.0, 1.0)
    }

    /// 2. 节点涌现率：种子之外新生节点的占比。
    fn node_emergence(reached: usize, seeds: usize) -> f32 {
        let emerged = reached.saturating_sub(seeds);
        (emerged as f32 / (EMERGE_SPREAD * seeds as f32)).clamp(0.0, 1.0)
    }

    /// 3. 流量分布均衡度：香农信息熵对最大熵的归一化。
    fn flow_balance(edge_flows: &[f32], active_edges: usize) -> f32 {
        let total_flow: f32 = edge_flows.iter().sum();
        if total_flow <= 1e-6 || active_edges <= 1 {
            return 0.5;
        }
        let entropy: f32 = edge_flows
            .iter()
            .map(|&flow| flow / total_flow)
            .filter(|&p| p > 1e-6)
            .map(|p| -p * p.ln())
            .sum();
        let max_entropy = (active_edges as f32).ln().max(1e-6);
        (entropy / max_entropy).clamp(0.0, 1.0)
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

    /// 求解双对偶连续场分布 (u_local, u_transfer)。
    ///
    /// 对 (I − α P) u = (1 − α) s₀ 做定点迭代 u ← (1 − α) s₀ + α P u，
    /// 两场各按自己的阻尼系数松弛，双双 L1 收敛后停机。
    pub fn solve(&self, source: &[f32], adjacency_matrix: &[Vec<f32>]) -> (Vec<f32>, Vec<f32>) {
        let n = source.len();
        if n == 0 || adjacency_matrix.len() != n {
            return (vec![], vec![]);
        }

        let s0 = Self::normalize_source(source);
        let mut u_local = s0.clone();
        let mut u_transfer = s0.clone();

        for _ in 0..self.max_iterations {
            let (next_local, delta_local) =
                Self::relax(&u_local, &s0, self.alpha_local, adjacency_matrix);
            let (next_transfer, delta_transfer) =
                Self::relax(&u_transfer, &s0, self.alpha_transfer, adjacency_matrix);

            u_local = next_local;
            u_transfer = next_transfer;

            if delta_local < self.tolerance && delta_transfer < self.tolerance {
                break;
            }
        }

        (u_local, u_transfer)
    }

    /// 源项归一化为概率分布；全零源保持原样。
    fn normalize_source(source: &[f32]) -> Vec<f32> {
        let sum: f32 = source.iter().sum();
        if sum > 1e-6 {
            source.iter().map(|&x| x / sum).collect()
        } else {
            source.to_vec()
        }
    }

    /// 单场单步松弛：u′ = (1 − α) s₀ + α P u，返回新场与 L1 变化量。
    /// 其中 (P u)_i = Σ_j P_{j→i} u_j（`adjacency_matrix[j][i]` 为 j→i 转移概率）。
    fn relax(
        field: &[f32],
        s0: &[f32],
        alpha: f32,
        adjacency_matrix: &[Vec<f32>],
    ) -> (Vec<f32>, f32) {
        let n = field.len();
        let mut next = vec![0.0f32; n];
        for (i, slot) in next.iter_mut().enumerate() {
            let propagated: f32 = (0..n).map(|j| adjacency_matrix[j][i] * field[j]).sum();
            *slot = (1.0 - alpha) * s0[i] + alpha * propagated;
        }
        let delta: f32 = next.iter().zip(field).map(|(a, b)| (a - b).abs()).sum();
        (next, delta)
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
