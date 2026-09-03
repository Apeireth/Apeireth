//! residual_pyramid: 修正 Gram-Schmidt 多层正交残差金字塔
//!
//! 吸收自 VCP 1.0 (`ResidualPyramid.js`):
//! 1. 基于 Modified Gram-Schmidt (MGS) 正交化算法，将 Query 向量投影到已知标签张成的子空间中；
//! 2. 多层级能量级联分解（60% 主导语义 -> 25% 次级语义 -> 5% 隐蔽微弱残差）；
//! 3. 握手差向量（Handshake Vectors）与方向一致性（Direction Coherence），量化领域漂移意图；
//! 4. 语义新颖度与白噪音抑制门控。

use serde::{Deserialize, Serialize};

/// 金字塔单层分析结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PyramidLevel {
    pub level: usize,
    pub explained_energy_ratio: f32,
    pub residual_magnitude: f32,
    pub tag_contributions: Vec<(u64, f32)>,
}

/// 残差金字塔全量分析结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PyramidAnalysis {
    pub levels: Vec<PyramidLevel>,
    pub total_explained_ratio: f32,
    pub coherence: f32,
    pub novelty_signal: f32,
    pub noise_signal: f32,
    pub final_residual: Vec<f32>,
}

/// 修正 Gram-Schmidt 正交残差金字塔引擎
#[derive(Debug, Clone)]
pub struct OrthogonalResidualPyramid {
    pub dimension: usize,
    pub max_levels: usize,
    pub min_energy_ratio: f32, // 默认 0.10 (解释 90% 后停机)
}

impl Default for OrthogonalResidualPyramid {
    fn default() -> Self {
        Self::new(3072)
    }
}

impl OrthogonalResidualPyramid {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            max_levels: 3,
            min_energy_ratio: 0.10,
        }
    }

    /// 执行修正 Gram-Schmidt 正交化投影与多层残差分解
    pub fn analyze<F>(&self, query: &[f32], tag_retriever: F) -> PyramidAnalysis
    where
        F: Fn(&[f32], usize) -> Vec<(u64, Vec<f32>)>,
    {
        let original_energy: f32 = query.iter().map(|&x| x * x).sum();
        if original_energy < 1e-12 {
            return PyramidAnalysis {
                levels: vec![],
                total_explained_ratio: 0.0,
                coherence: 0.0,
                novelty_signal: 0.0,
                noise_signal: 0.0,
                final_residual: query.to_vec(),
            };
        }

        let mut current_residual = query.to_vec();
        let mut levels = Vec::new();
        let mut total_explained = 0.0;
        let mut all_retrieved_tags = Vec::new();

        for level in 0..self.max_levels {
            let tags = tag_retriever(&current_residual, 10);
            if tags.is_empty() {
                break;
            }

            // 保存全部召回的标签用于握手分析
            if level == 0 {
                all_retrieved_tags = tags.clone();
            }

            // 1. Modified Gram-Schmidt (MGS) 构建正交基底
            let mut basis: Vec<Vec<f32>> = Vec::new();
            let mut contributions = Vec::new();

            for (tag_id, tag_vec) in &tags {
                let mut v = tag_vec.clone();
                // 逐一减去已有正交基上的投影分量
                for u in &basis {
                    let dot: f32 = v.iter().zip(u).map(|(&a, &b)| a * b).sum();
                    for (vi, &ui) in v.iter_mut().zip(u) {
                        *vi -= dot * ui;
                    }
                }
                let mag = (v.iter().map(|&x| x * x).sum::<f32>()).sqrt();
                if mag > 1e-6 {
                    for x in &mut v {
                        *x /= mag;
                    }
                    let coeff = current_residual
                        .iter()
                        .zip(&v)
                        .map(|(&a, &b)| a * b)
                        .sum::<f32>()
                        .abs();
                    contributions.push((*tag_id, coeff));
                    basis.push(v);
                }
            }

            if basis.is_empty() {
                break;
            }

            // 2. 计算当前残差在子空间上的总投影向量 P = Σ <R, u_i> * u_i
            let mut projection = vec![0.0f32; self.dimension];
            for u in &basis {
                let dot: f32 = current_residual.iter().zip(u).map(|(&a, &b)| a * b).sum();
                for (pi, &ui) in projection.iter_mut().zip(u) {
                    *pi += dot * ui;
                }
            }

            // 3. 计算新残差 R_new = R_old - P
            let mut new_residual = vec![0.0f32; self.dimension];
            for i in 0..self.dimension {
                new_residual[i] = current_residual[i] - projection[i];
            }

            let new_res_energy: f32 = new_residual.iter().map(|&x| x * x).sum();
            let current_energy: f32 = current_residual.iter().map(|&x| x * x).sum();
            let energy_explained = (current_energy - new_res_energy).max(0.0) / original_energy;

            levels.push(PyramidLevel {
                level,
                explained_energy_ratio: energy_explained,
                residual_magnitude: new_res_energy.sqrt(),
                tag_contributions: contributions,
            });

            total_explained += energy_explained;
            current_residual = new_residual;

            // 能量阈值截断 (90% 解释度)
            if (new_res_energy / original_energy) < self.min_energy_ratio {
                break;
            }
        }

        // 4. 分析握手差向量与相干度 (Direction Coherence)
        let (coherence, noise_signal) =
            Self::compute_handshake_coherence(query, &all_retrieved_tags);
        let novelty_signal = ((1.0 - total_explained) * 0.70 + coherence * 0.30).clamp(0.0, 1.0);

        PyramidAnalysis {
            levels,
            total_explained_ratio: total_explained.clamp(0.0, 1.0),
            coherence,
            novelty_signal,
            noise_signal,
            final_residual: current_residual,
        }
    }

    /// 计算握手差向量与相干度
    fn compute_handshake_coherence(query: &[f32], tags: &[(u64, Vec<f32>)]) -> (f32, f32) {
        if tags.is_empty() {
            return (0.0, 1.0);
        }

        let dim = query.len();
        let mut mean_diff = vec![0.0f32; dim];

        for (_, tag_vec) in tags {
            let mut diff = vec![0.0f32; dim];
            for i in 0..dim {
                diff[i] = query[i] - tag_vec[i];
            }
            let mag = (diff.iter().map(|&x| x * x).sum::<f32>()).sqrt();
            if mag > 1e-6 {
                for i in 0..dim {
                    mean_diff[i] += diff[i] / mag;
                }
            }
        }

        let n = tags.len() as f32;
        for x in &mut mean_diff {
            *x /= n;
        }

        let coherence = (mean_diff.iter().map(|&x| x * x).sum::<f32>())
            .sqrt()
            .clamp(0.0, 1.0);
        let noise_signal = (1.0 - coherence).clamp(0.0, 1.0);

        (coherence, noise_signal)
    }
}

/// 记忆激活门控度量
pub struct FieldActivationGate;

impl FieldActivationGate {
    /// 计算综合激活度: Coverage * Coherence * (1 - Noise)
    pub fn compute_activation(analysis: &PyramidAnalysis) -> f32 {
        (analysis.total_explained_ratio * analysis.coherence * (1.0 - analysis.noise_signal))
            .clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mgs_orthogonal_projection_energy_decomposition() {
        let pyramid = OrthogonalResidualPyramid::new(3);
        let query = vec![1.0, 1.0, 1.0]; // E0 = 3.0

        let mock_retriever = |residual: &[f32], _top_k: usize| -> Vec<(u64, Vec<f32>)> {
            if residual[0] > 0.5 {
                vec![(1, vec![1.0, 0.0, 0.0])]
            } else if residual[1] > 0.5 {
                vec![(2, vec![0.0, 1.0, 0.0])]
            } else {
                vec![(3, vec![0.0, 0.0, 1.0])]
            }
        };

        let result = pyramid.analyze(&query, mock_retriever);
        assert!(!result.levels.is_empty());
        assert!(result.total_explained_ratio > 0.60);
        assert!(result.coherence >= 0.0 && result.coherence <= 1.0);

        let activation = FieldActivationGate::compute_activation(&result);
        assert!(activation >= 0.0 && activation <= 1.0);
    }
}
