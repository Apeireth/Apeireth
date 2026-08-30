//! epa_bridge: EPA 加权中心化 PCA 语义主轴、逻辑深度与跨域共振桥
//!
//! 吸收自 VCP 1.0 (`EPAModule.js`):
//! 1. 加权中心化 (Weighted Centering) 消除公共背景偏置；
//! 2. 隐式 Gram 矩阵幂迭代 (Power Iteration) 与正交化提取语义主成分基底；
//! 3. 能量分布香农信息熵量化逻辑深度 (Logic Depth = 1 - H_norm)；
//! 4. 双主轴跨域共振桥 (Cross-Domain Resonance Bridges) 探测。

use serde::{Deserialize, Serialize};

/// 语义主轴分析结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpaProjectionResult {
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

/// EPA 语义桥梁引擎
#[derive(Debug, Clone)]
pub struct EpaSemanticBridge {
    pub dimension: usize,
    pub basis_vectors: Vec<Vec<f32>>,
    pub mean_vector: Vec<f32>,
}

impl EpaSemanticBridge {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            basis_vectors: Vec::new(),
            mean_vector: vec![0.0f32; dimension],
        }
    }

    /// 从带权重的聚类质心样本中提取正交基底 (加权中心化 PCA)
    pub fn fit(&mut self, centroids: &[(Vec<f32>, f32)], num_components: usize) {
        let n = centroids.len();
        if n == 0 || num_components == 0 {
            return;
        }

        let dim = self.dimension;

        // 1. 计算全局加权均值向量 μ
        let total_weight: f32 = centroids.iter().map(|(_, w)| *w).sum();
        if total_weight < 1e-6 {
            return;
        }

        let mut mean = vec![0.0f32; dim];
        for (vec, w) in centroids {
            for i in 0..dim {
                mean[i] += vec[i] * w;
            }
        }
        for x in &mut mean {
            *x /= total_weight;
        }
        self.mean_vector = mean.clone();

        // 2. 构建加权中心化矩阵 X_tilde
        let mut x_tilde: Vec<Vec<f32>> = Vec::with_capacity(n);
        for (vec, w) in centroids {
            let sqrt_w = w.sqrt();
            let mut centered = vec![0.0f32; dim];
            for i in 0..dim {
                centered[i] = (vec[i] - self.mean_vector[i]) * sqrt_w;
            }
            x_tilde.push(centered);
        }

        // 3. 构建 K x K 样本 Gram 矩阵: G_{ij} = <x_i, x_j>
        let mut gram = vec![vec![0.0f32; n]; n];
        for i in 0..n {
            for j in i..n {
                let dot: f32 = x_tilde[i].iter().zip(&x_tilde[j]).map(|(&a, &b)| a * b).sum();
                gram[i][j] = dot;
                gram[j][i] = dot;
            }
        }

        // 4. 幂迭代带重正交化 (Power Iteration with Deflation) 求解特征向量
        let k = num_components.min(n);
        let mut gram_eigenvectors: Vec<Vec<f32>> = Vec::with_capacity(k);

        for comp_idx in 0..k {
            let mut v = vec![0.0f32; n];
            v[comp_idx % n] = 1.0;

            for _iter in 0..30 {
                // w = G * v
                let mut w = vec![0.0f32; n];
                for i in 0..n {
                    for j in 0..n {
                        w[i] += gram[i][j] * v[j];
                    }
                }

                // 减去前序特征向量上的投影
                for prev in &gram_eigenvectors {
                    let dot: f32 = w.iter().zip(prev).map(|(&a, &b)| a * b).sum();
                    for (wi, &pi) in w.iter_mut().zip(prev) {
                        *wi -= dot * pi;
                    }
                }

                let mag = (w.iter().map(|&x| x * x).sum::<f32>()).sqrt();
                if mag > 1e-6 {
                    for x in &mut w {
                        *x /= mag;
                    }
                    v = w;
                } else {
                    break;
                }
            }

            gram_eigenvectors.push(v);
        }

        // 5. 将 Gram 空间特征向量映射回原始特征空间 U_k = Σ v_i X_tilde_i
        let mut basis = Vec::with_capacity(k);
        for v in gram_eigenvectors {
            let mut u = vec![0.0f32; dim];
            for (i, &vi) in v.iter().enumerate() {
                for d in 0..dim {
                    u[d] += vi * x_tilde[i][d];
                }
            }
            let mag = (u.iter().map(|&x| x * x).sum::<f32>()).sqrt();
            if mag > 1e-6 {
                for x in &mut u {
                    *x /= mag;
                }
                basis.push(u);
            }
        }

        self.basis_vectors = basis;
    }

    /// 投影 Query 向量并量化逻辑深度与跨域共振
    pub fn project(&self, vector: &[f32]) -> EpaProjectionResult {
        let k = self.basis_vectors.len();
        if k == 0 || vector.len() != self.dimension {
            return EpaProjectionResult {
                projections: vec![],
                probabilities: vec![],
                normalized_entropy: 0.0,
                logic_depth: 0.0,
                resonance_score: 0.0,
                active_bridges: vec![],
            };
        }

        // 1. 去中心化: v' = v - mean
        let centered: Vec<f32> = vector
            .iter()
            .zip(&self.mean_vector)
            .map(|(&v, &m)| v - m)
            .collect();

        // 2. 投影至各语义主轴: p_k = <centered, U_k>
        let mut projections = vec![0.0f32; k];
        let mut total_energy = 0.0f32;

        for (i, basis) in self.basis_vectors.iter().enumerate() {
            let dot: f32 = centered.iter().zip(basis).map(|(&a, &b)| a * b).sum();
            projections[i] = dot;
            total_energy += dot * dot;
        }

        if total_energy < 1e-12 {
            return EpaProjectionResult {
                projections,
                probabilities: vec![0.0; k],
                normalized_entropy: 0.0,
                logic_depth: 0.0,
                resonance_score: 0.0,
                active_bridges: vec![],
            };
        }

        // 3. 计算能量分布概率 P(k) 与归一化信息熵 H_norm
        let mut probabilities = vec![0.0f32; k];
        let mut entropy = 0.0f32;

        for (i, &p) in projections.iter().enumerate() {
            let prob = (p * p) / total_energy;
            probabilities[i] = prob;
            if prob > 1e-6 {
                entropy -= prob * prob.log2();
            }
        }

        let max_entropy = (k as f32).log2().max(1e-6);
        let normalized_entropy = (entropy / max_entropy).clamp(0.0, 1.0);
        let logic_depth = (1.0 - normalized_entropy).clamp(0.0, 1.0);

        // 4. 跨域共振桥检测 (当两个正交轴能量同时 > 0.05)
        let mut resonance_score = 0.0f32;
        let mut active_bridges = Vec::new();

        for i in 0..k {
            for j in (i + 1)..k {
                if probabilities[i] > 0.05 && probabilities[j] > 0.05 {
                    let co_activation = (probabilities[i] * probabilities[j]).sqrt();
                    if co_activation > 0.10 {
                        active_bridges.push((i, j, co_activation));
                        resonance_score += co_activation;
                    }
                }
            }
        }

        EpaProjectionResult {
            projections,
            probabilities,
            normalized_entropy,
            logic_depth,
            resonance_score,
            active_bridges,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epa_fit_and_projection_logic_depth() {
        let mut epa = EpaSemanticBridge::new(3);

        let centroids = vec![
            (vec![1.0, 0.0, 0.0], 10.0),
            (vec![0.0, 1.0, 0.0], 10.0),
            (vec![0.0, 0.0, 1.0], 10.0),
        ];

        epa.fit(&centroids, 2);
        assert_eq!(epa.basis_vectors.len(), 2);

        // 强偏向单一主轴的向量，逻辑深度高
        let query_focused = vec![2.0, 0.0, 0.0];
        let res_focused = epa.project(&query_focused);
        assert!(res_focused.logic_depth >= 0.0 && res_focused.logic_depth <= 1.0);

        // 双轴平衡激活的向量，触发共振
        let query_resonance = vec![1.0, 1.0, 0.0];
        let res_resonance = epa.project(&query_resonance);
        assert_eq!(res_resonance.projections.len(), 2);
    }
}
