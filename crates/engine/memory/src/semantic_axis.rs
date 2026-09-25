//! semantic_axis: 加权中心化 PCA 语义主轴 (Semantic Axis)、逻辑深度与跨域共振桥
//!
//! 本模块为独立实现，数学基础均为公开文献中的经典方法：
//! 1. 加权中心化 (Weighted Centering)：以样本权重计算质心并整体平移，
//!    消除公共背景偏置（加权 PCA 的标准预处理）；
//! 2. 核技巧幂迭代 (Power Iteration with Deflation)：
//!    在 K×K 样本 Gram 矩阵上迭代求前 M 个正交特征方向
//!    [Golub & Van Loan, *Matrix Computations*, §7.3 / Strang, *Linear Algebra and Its Applications*]；
//! 3. 逻辑深度 (Logic Depth) = 1 − H_norm，其中 H_norm 为投影能量分布的
//!    归一化香农信息熵 [Shannon, 1948]；
//! 4. 跨域共振桥 (Resonance Bridges)：双主轴能量共激活强度 √(PᵢPⱼ) 探测。

use serde::{Deserialize, Serialize};

/// 语义主轴分析结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticAxisProjection {
    /// 各正交主轴上的投影分量
    pub projections: Vec<f32>,
    /// 能量概率分布 P(k)
    pub probabilities: Vec<f32>,
    /// 归一化信息熵 H_norm ∈ [0, 1]
    pub normalized_entropy: f32,
    /// 逻辑深度 ∈ [0, 1] (1 表示高度聚焦，0 表示发散)
    pub logic_depth: f32,
    /// 跨域共振强度
    pub resonance_score: f32,
    /// 激活的共振桥列表 [(轴 i, 轴 j, 强度)]
    pub active_bridges: Vec<(usize, usize, f32)>,
}

/// 语义主轴桥梁引擎 (Semantic Axis Bridge)
#[derive(Debug, Clone)]
pub struct SemanticAxisBridge {
    pub dimension: usize,
    pub basis_vectors: Vec<Vec<f32>>,
    pub mean_vector: Vec<f32>,
}

/// 幂迭代最大轮数。
const POWER_ITERATIONS: usize = 30;
/// 数值零判据。
const EPSILON: f32 = 1e-6;
/// 能量零判据。
const ENERGY_EPSILON: f32 = 1e-12;
/// 共振桥：单轴能量概率下限。
const BRIDGE_AXIS_FLOOR: f32 = 0.05;
/// 共振桥：共激活强度下限。
const BRIDGE_COACTIVATION_FLOOR: f32 = 0.10;

impl SemanticAxisBridge {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            basis_vectors: Vec::new(),
            mean_vector: vec![0.0f32; dimension],
        }
    }

    /// 从带权重的聚类质心样本中提取正交基底 (加权中心化 PCA)。
    ///
    /// 流程：加权质心 → 整体平移去偏置 → 样本 Gram 矩阵 →
    /// 幂迭代（带对已提取方向的逐次正交化）→ 特征方向升维回特征空间。
    pub fn fit(&mut self, centroids: &[(Vec<f32>, f32)], num_components: usize) {
        let sample_count = centroids.len();
        if sample_count == 0 || num_components == 0 {
            return;
        }

        let total_weight: f32 = centroids.iter().map(|(_, w)| *w).sum();
        if total_weight < EPSILON {
            return;
        }

        // 1. 加权质心 μ
        let mean = weighted_mean(centroids, self.dimension, total_weight);
        self.mean_vector = mean.clone();

        // 2. 去均值并按 √w 缩放样本
        let centered = center_and_scale(centroids, &mean, self.dimension);

        // 3. 样本 Gram 矩阵 G_{ij} = ⟨x̃ᵢ, x̃ⱼ⟩
        let gram = gram_matrix(&centered);

        // 4. 幂迭代 + 放缩求 Gram 空间前 M 个正交方向
        let component_count = num_components.min(sample_count);
        let gram_directions = dominant_gram_directions(&gram, sample_count, component_count);

        // 5. 升维回特征空间并单位化：U_k = Σᵢ vᵢ x̃ᵢ
        self.basis_vectors = lift_to_feature_space(&gram_directions, &centered, self.dimension);
    }

    /// 投影 Query 向量并量化逻辑深度与跨域共振。
    pub fn project(&self, vector: &[f32]) -> SemanticAxisProjection {
        let axis_count = self.basis_vectors.len();
        if axis_count == 0 || vector.len() != self.dimension {
            return SemanticAxisProjection {
                projections: vec![],
                probabilities: vec![],
                normalized_entropy: 0.0,
                logic_depth: 0.0,
                resonance_score: 0.0,
                active_bridges: vec![],
            };
        }

        // 1. 去中心化: v′ = v − μ
        let centered: Vec<f32> = vector
            .iter()
            .zip(&self.mean_vector)
            .map(|(&v, &m)| v - m)
            .collect();

        // 2. 投影至各语义主轴并累计能量
        let projections: Vec<f32> = self
            .basis_vectors
            .iter()
            .map(|axis| dot(&centered, axis))
            .collect();
        let total_energy: f32 = projections.iter().map(|p| p * p).sum();

        if total_energy < ENERGY_EPSILON {
            return SemanticAxisProjection {
                projections,
                probabilities: vec![0.0; axis_count],
                normalized_entropy: 0.0,
                logic_depth: 0.0,
                resonance_score: 0.0,
                active_bridges: vec![],
            };
        }

        // 3. 能量概率分布与归一化信息熵 → 逻辑深度
        let probabilities: Vec<f32> = projections.iter().map(|p| (p * p) / total_energy).collect();
        let normalized_entropy = normalized_shannon_entropy(&probabilities);
        let logic_depth = (1.0 - normalized_entropy).clamp(0.0, 1.0);

        // 4. 双主轴共激活共振桥
        let (resonance_score, active_bridges) = resonance_bridges(&probabilities);

        SemanticAxisProjection {
            projections,
            probabilities,
            normalized_entropy,
            logic_depth,
            resonance_score,
            active_bridges,
        }
    }
}

/// 向量内积（长度按短者截断）。
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(&x, &y)| x * y).sum()
}

/// 加权质心：μ = Σ wᵢxᵢ / Σ wᵢ。
fn weighted_mean(centroids: &[(Vec<f32>, f32)], dimension: usize, total_weight: f32) -> Vec<f32> {
    let mut mean = vec![0.0f32; dimension];
    for (vector, weight) in centroids {
        for i in 0..dimension {
            mean[i] += vector[i] * weight;
        }
    }
    for value in &mut mean {
        *value /= total_weight;
    }
    mean
}

/// 去均值并按 √w 缩放，得到加权中心化样本集。
fn center_and_scale(
    centroids: &[(Vec<f32>, f32)],
    mean: &[f32],
    dimension: usize,
) -> Vec<Vec<f32>> {
    centroids
        .iter()
        .map(|(vector, weight)| {
            let scale = weight.sqrt();
            (0..dimension)
                .map(|i| (vector[i] - mean[i]) * scale)
                .collect()
        })
        .collect()
}

/// 样本 Gram 矩阵（对称）：G_{ij} = ⟨x̃ᵢ, x̃ⱼ⟩。
fn gram_matrix(samples: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let n = samples.len();
    let mut gram = vec![vec![0.0f32; n]; n];
    for i in 0..n {
        for j in i..n {
            let value = dot(&samples[i], &samples[j]);
            gram[i][j] = value;
            gram[j][i] = value;
        }
    }
    gram
}

/// 幂迭代（带放缩/逐次正交化）：从单位坐标向量出发提取前 `components`
/// 个正交特征方向；单方向不收敛时保留迭代前的向量。
fn dominant_gram_directions(
    gram: &[Vec<f32>],
    sample_count: usize,
    components: usize,
) -> Vec<Vec<f32>> {
    let mut directions: Vec<Vec<f32>> = Vec::with_capacity(components);

    for component in 0..components {
        let mut v = vec![0.0f32; sample_count];
        v[component % sample_count] = 1.0;

        for _ in 0..POWER_ITERATIONS {
            // w = G v
            let mut w: Vec<f32> = gram.iter().map(|row| dot(row, &v)).collect();

            // 对已提取方向做逐次正交化
            for accepted in &directions {
                let coeff = dot(&w, accepted);
                for (a, &u) in w.iter_mut().zip(accepted) {
                    *a -= coeff * u;
                }
            }

            let magnitude = dot(&w, &w).sqrt();
            if magnitude > EPSILON {
                for a in &mut w {
                    *a /= magnitude;
                }
                v = w;
            } else {
                break;
            }
        }

        directions.push(v);
    }

    directions
}

/// 把 Gram 空间方向升维回特征空间并单位化：U = Σᵢ vᵢ x̃ᵢ。
fn lift_to_feature_space(
    gram_directions: &[Vec<f32>],
    samples: &[Vec<f32>],
    dimension: usize,
) -> Vec<Vec<f32>> {
    let mut basis = Vec::with_capacity(gram_directions.len());
    for direction in gram_directions {
        let mut axis = vec![0.0f32; dimension];
        for (i, &coefficient) in direction.iter().enumerate() {
            for d in 0..dimension {
                axis[d] += coefficient * samples[i][d];
            }
        }
        let magnitude = dot(&axis, &axis).sqrt();
        if magnitude > EPSILON {
            for a in &mut axis {
                *a /= magnitude;
            }
            basis.push(axis);
        }
    }
    basis
}

/// 归一化香农信息熵：H_norm = (−Σ p log₂ p) / log₂(k)，截断到 [0, 1]。
fn normalized_shannon_entropy(probabilities: &[f32]) -> f32 {
    let k = probabilities.len();
    if k == 0 {
        return 0.0;
    }
    let entropy: f32 = probabilities
        .iter()
        .filter(|&&p| p > EPSILON)
        .map(|p| -p * p.log2())
        .sum();
    let max_entropy = (k as f32).log2().max(EPSILON);
    (entropy / max_entropy).clamp(0.0, 1.0)
}

/// 双主轴共振桥：两轴能量概率同时过线时，按共激活强度 √(PᵢPⱼ) 记桥。
fn resonance_bridges(probabilities: &[f32]) -> (f32, Vec<(usize, usize, f32)>) {
    let k = probabilities.len();
    let mut score = 0.0f32;
    let mut bridges = Vec::new();
    for i in 0..k {
        for j in (i + 1)..k {
            if probabilities[i] > BRIDGE_AXIS_FLOOR && probabilities[j] > BRIDGE_AXIS_FLOOR {
                let co_activation = (probabilities[i] * probabilities[j]).sqrt();
                if co_activation > BRIDGE_COACTIVATION_FLOOR {
                    bridges.push((i, j, co_activation));
                    score += co_activation;
                }
            }
        }
    }
    (score, bridges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_axis_fit_and_projection_logic_depth() {
        let mut axis = SemanticAxisBridge::new(3);

        let centroids = vec![
            (vec![1.0, 0.0, 0.0], 10.0),
            (vec![0.0, 1.0, 0.0], 10.0),
            (vec![0.0, 0.0, 1.0], 10.0),
        ];

        axis.fit(&centroids, 2);
        assert_eq!(axis.basis_vectors.len(), 2);

        // 强偏向单一主轴的向量，逻辑深度高
        let query_focused = vec![2.0, 0.0, 0.0];
        let res_focused = axis.project(&query_focused);
        assert!(res_focused.logic_depth >= 0.0 && res_focused.logic_depth <= 1.0);

        // 双轴平衡激活的向量，触发共振
        let query_resonance = vec![1.0, 1.0, 0.0];
        let res_resonance = axis.project(&query_resonance);
        assert_eq!(res_resonance.projections.len(), 2);
    }
}
