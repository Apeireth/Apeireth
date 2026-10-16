//! residual_pyramid: 修正 Gram-Schmidt 多层正交残差金字塔
//!
//! 本模块为独立实现，数学基础均为公开数值线性代数中的经典方法：
//! 1. Modified Gram-Schmidt (MGS) 正交化
//!    [Golub & Van Loan, *Matrix Computations*, 4th ed., §5.2]，
//!    将查询向量逐层投影到召回标签张成的正交子空间上；
//! 2. 多层能量级联分解：每层结算一次解释能量占比，残差过小或层数用尽即停机；
//! 3. 方向一致性 (Direction Coherence)：归一化差向量均值的范数，
//!    用于量化查询与召回集的整体漂移方向；
//! 4. 语义新颖度与白噪音抑制门控（Coverage × Coherence × (1 − Noise)）。

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

/// 正交基底及其存活标签：逐标签正交化后范数过小的候选被丢弃（线性相关）。
struct OrthogonalBasis {
    vectors: Vec<Vec<f32>>,
    kept_tags: Vec<u64>,
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

    /// 执行修正 Gram-Schmidt 正交化投影与多层残差分解。
    ///
    /// 每层：召回 → MGS 构基 → 残差投影剥离 → 能量结算；
    /// 首层召回集另用于方向一致性分析。
    pub fn analyze<F>(&self, query: &[f32], tag_retriever: F) -> PyramidAnalysis
    where
        F: Fn(&[f32], usize) -> Vec<(u64, Vec<f32>)>,
    {
        let original_energy = dot(query, query);
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

        let mut residual = query.to_vec();
        let mut levels = Vec::new();
        let mut total_explained = 0.0f32;
        let mut first_wave: Vec<(u64, Vec<f32>)> = Vec::new();

        for level in 0..self.max_levels {
            let tags = tag_retriever(&residual, 10);
            if tags.is_empty() {
                break;
            }
            if level == 0 {
                first_wave = tags.clone();
            }

            let basis = Self::orthonormal_basis(&tags);
            if basis.vectors.is_empty() {
                break;
            }

            let contributions: Vec<(u64, f32)> = basis
                .kept_tags
                .iter()
                .zip(&basis.vectors)
                .map(|(tag_id, axis)| (*tag_id, dot(&residual, axis).abs()))
                .collect();

            let projection = project_onto(&basis.vectors, &residual);
            let next_residual = subtract(&residual, &projection);

            let next_energy = dot(&next_residual, &next_residual);
            let current_energy = dot(&residual, &residual);
            let explained = (current_energy - next_energy).max(0.0) / original_energy;

            levels.push(PyramidLevel {
                level,
                explained_energy_ratio: explained,
                residual_magnitude: next_energy.sqrt(),
                tag_contributions: contributions,
            });

            total_explained += explained;
            residual = next_residual;

            if (dot(&residual, &residual) / original_energy) < self.min_energy_ratio {
                break;
            }
        }

        let (coherence, noise_signal) = directional_agreement(query, &first_wave);
        let novelty_signal = ((1.0 - total_explained) * 0.70 + coherence * 0.30).clamp(0.0, 1.0);

        PyramidAnalysis {
            levels,
            total_explained_ratio: total_explained.clamp(0.0, 1.0),
            coherence,
            novelty_signal,
            noise_signal,
            final_residual: residual,
        }
    }

    /// Modified Gram-Schmidt：按召回顺序逐一正交化并单位化，
    /// 与已保留轴几乎共线（范数 ≤ 1e-6）的候选直接丢弃。
    fn orthonormal_basis(tags: &[(u64, Vec<f32>)]) -> OrthogonalBasis {
        let mut basis = OrthogonalBasis {
            vectors: Vec::new(),
            kept_tags: Vec::new(),
        };
        for (tag_id, tag_vec) in tags {
            let mut axis = tag_vec.clone();
            for accepted in &basis.vectors {
                let coeff = dot(&axis, accepted);
                for (a, &u) in axis.iter_mut().zip(accepted) {
                    *a -= coeff * u;
                }
            }
            let magnitude = dot(&axis, &axis).sqrt();
            if magnitude > 1e-6 {
                for a in &mut axis {
                    *a /= magnitude;
                }
                basis.kept_tags.push(*tag_id);
                basis.vectors.push(axis);
            }
        }
        basis
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

/// 向量内积（长度按短者截断，与逐维 zip 语义一致）。
fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(&x, &y)| x * y).sum()
}

/// 把 `v` 投影到正交基底张成的子空间：P v = Σ ⟨v, uᵢ⟩ uᵢ。
fn project_onto(basis: &[Vec<f32>], v: &[f32]) -> Vec<f32> {
    let mut acc = vec![0.0f32; v.len()];
    for axis in basis {
        let coeff = dot(v, axis);
        for (a, &u) in acc.iter_mut().zip(axis) {
            *a += coeff * u;
        }
    }
    acc
}

/// 逐维相减 `a - b`（长度按 `a`）。
fn subtract(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(&x, &y)| x - y).collect()
}

/// 方向一致性：把每个召回向量对查询的差向量单位化后求均值，
/// 均值范数即 coherence（一致漂移越强越大），其余量记为 noise。
fn directional_agreement(query: &[f32], tags: &[(u64, Vec<f32>)]) -> (f32, f32) {
    if tags.is_empty() {
        return (0.0, 1.0);
    }
    let mut drift = vec![0.0f32; query.len()];
    for (_, tag_vec) in tags {
        let diff = subtract(query, tag_vec);
        let magnitude = dot(&diff, &diff).sqrt();
        if magnitude > 1e-6 {
            for (d, &delta) in drift.iter_mut().zip(&diff) {
                *d += delta / magnitude;
            }
        }
    }
    let n = tags.len() as f32;
    for d in &mut drift {
        *d /= n;
    }
    let coherence = dot(&drift, &drift).sqrt().clamp(0.0, 1.0);
    let noise_signal = (1.0 - coherence).clamp(0.0, 1.0);
    (coherence, noise_signal)
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
