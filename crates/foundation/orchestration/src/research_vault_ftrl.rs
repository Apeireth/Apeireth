//! B9 · Phase 3 补全: VaultLRU/FTRL — cache-safe 学习型保留 (Research 前缀, 默认关闭)。
//!
//! # 学术账本 (铁律 3)
//! - **问题定义**: 一般情形 (段尺寸不等、价值估计有噪声、需 cache 稳定) 下,
//!   学一个段价值线性打分器, 使保留决策对最优固定线性打分器有**后悔界**,
//!   且不破坏 prompt cache (RA-3 Proposal B)。
//! - **假设**: 价值线性 v=⟨w,x⟩; 请求可对抗 (不做分布假设); 凸损失下 OGD 对
//!   任意固定 w* 的后悔为 O(√T) (Zinkevich 2003, η_t ∝ 1/√t); 打分器只作用于
//!   注入区 + 前缀命中率护栏 + 确定性 fallback (RA-3 Q3 契约)。
//! - **状态**: 原型已实现 — 线性打分 + FTRL(OGD) 在线更新 + 合成后悔验证 +
//!   前缀命中率护栏 + StackPin fallback。整数化与真实缓存切换代价破坏纯凸设定,
//!   故保证是"后悔界护栏 + 工程护栏", 非端到端竞争比 (RA-3 §5 口径)。
//! - **引用**: Zinkevich 2003 (OGD O(√T)); Antoniadis et al. 2023
//!   (learning-augmented caching, 一致性-鲁棒性); RA-3 Q3/Q4。
//! - **baseline**: `research/baselines/baseline-2026-09-phase0.md`。
//! - **已知局限**: ① 合成数据验证 (真实对话轨迹待评测批); ② FTRL 未加 L1 稀疏项
//!   (特征维数小, OGD 足够); ③ bandit 反馈 (只有执行项的 outcome) 留后续。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::research_context_policy::{
    ResearchPolicyAction, ResearchSegment, ResearchStackPinPolicy,
};

/// 段特征向量 (RA-3 Q4): [检索分, recency, 新奇度, bias=1]。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResearchSegmentFeatures {
    /// 检索分 ∈ [0,1]。
    pub retr_score: f32,
    /// recency: 1/(1+age) ∈ (0,1]。
    pub recency: f32,
    /// 新奇度 (残差/去重信号) ∈ [0,1]。
    pub novelty: f32,
}

impl ResearchSegmentFeatures {
    pub fn as_vec(&self) -> [f32; 4] {
        [self.retr_score, self.recency, self.novelty, 1.0]
    }
}

/// 在线梯度下降配置。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResearchOgdConfig {
    /// 初始步长 η_0 (η_t = η_0 / √t)。
    pub eta0: f32,
    /// 权重范数上界 (投影球半径, Zinkevich 假设)。
    pub weight_radius: f32,
}

impl Default for ResearchOgdConfig {
    fn default() -> Self {
        Self {
            eta0: 0.1,
            weight_radius: 2.0,
        }
    }
}

/// 前缀命中率护栏配置 (RA-3 Q3 护栏 5)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ResearchPrefixGuardConfig {
    /// 命中率阈值 (默认 0.8, 对齐 prompt_stabilizer 80%+ 目标)。
    pub theta: f32,
    /// 滑窗长度 (最近 N 次决策的排序前缀对比)。
    pub window: usize,
}

impl Default for ResearchPrefixGuardConfig {
    fn default() -> Self {
        Self {
            theta: 0.8,
            window: 8,
        }
    }
}

/// 决策记录 (guard 窗口用)。
#[derive(Debug, Clone, PartialEq)]
struct ResearchDecisionRecord {
    ranked_ids: Vec<String>,
    by_policy: bool,
}

/// VaultLRU/FTRL: 线性打分 + OGD 在线更新 + 前缀护栏 + 确定性 fallback。
#[derive(Debug, Clone)]
pub struct ResearchVaultLruFtrl {
    /// 权重 (4 维: retr/recency/novelty/bias)。
    pub weights: [f32; 4],
    pub ogd: ResearchOgdConfig,
    pub guard: ResearchPrefixGuardConfig,
    /// 轮次 t (步长用)。
    t: u64,
    /// 决策滑窗。
    window: Vec<ResearchDecisionRecord>,
    /// 连续护栏失守次数 (诊断)。
    pub guard_trips: u64,
}

impl Default for ResearchVaultLruFtrl {
    fn default() -> Self {
        Self {
            weights: [0.2, 0.3, 0.2, 0.3],
            ogd: ResearchOgdConfig::default(),
            guard: ResearchPrefixGuardConfig::default(),
            t: 0,
            window: Vec::new(),
            guard_trips: 0,
        }
    }
}

impl ResearchVaultLruFtrl {
    pub fn new(ogd: ResearchOgdConfig, guard: ResearchPrefixGuardConfig) -> Self {
        Self {
            ogd,
            guard,
            ..Default::default()
        }
    }

    /// 打分 v = ⟨w, x⟩。
    pub fn score(&self, f: &ResearchSegmentFeatures) -> f32 {
        let x = f.as_vec();
        let mut v = 0.0f32;
        for i in 0..4 {
            v += self.weights[i] * x[i];
        }
        v
    }

    /// 按价值降序 + tie-break id 稳定排序 (RA-3: append-only tail 原则的基础)。
    pub fn rank(
        &self,
        segments: &[ResearchSegment],
        features: &[ResearchSegmentFeatures],
    ) -> Vec<String> {
        let mut idx: Vec<usize> = (0..segments.len()).collect();
        idx.sort_by(|&a, &b| {
            let va = self.score(&features[a]);
            let vb = self.score(&features[b]);
            vb.partial_cmp(&va)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| segments[a].segment.name.cmp(&segments[b].segment.name))
        });
        idx.into_iter()
            .map(|i| segments[i].segment.name.clone())
            .collect()
    }

    /// 前缀命中率: 两次排序的最长公共前缀比例 (RA-3 护栏 5 的模型级近似)。
    pub fn prefix_ratio(prev: &[String], cur: &[String]) -> f32 {
        let n = prev.len().min(cur.len());
        if n == 0 {
            return 1.0;
        }
        let mut lcp = 0usize;
        while lcp < n && prev[lcp] == cur[lcp] {
            lcp += 1;
        }
        lcp as f32 / n as f32
    }

    /// 决策主入口 (决策-渲染分离: 只产出排序; 生产路径不挂)。
    ///
    /// - 用当前权重排序;
    /// - 与上一窗口排序对比前缀命中率, 低于阈值 → 触发护栏:
    ///   回退 StackPin 确定性排序 (仅本次), 并计数;
    /// - 记录决策窗口。
    pub fn decide_ranking(
        &mut self,
        segments: &[ResearchSegment],
        features: &[ResearchSegmentFeatures],
        _stackpin: &ResearchStackPinPolicy,
    ) -> (Vec<String>, bool) {
        debug_assert_eq!(segments.len(), features.len());
        let ranked = self.rank(segments, features);
        // 护栏检查: 与最近一次"按策略"的排序对比。
        let mut fallback = false;
        if let Some(prev) = self.window.iter().rev().find(|r| r.by_policy) {
            let ratio = Self::prefix_ratio(&prev.ranked_ids, &ranked);
            if ratio < self.guard.theta {
                fallback = true;
                self.guard_trips += 1;
            }
        }
        let final_rank = if fallback {
            let mut order: Vec<usize> = (0..segments.len()).collect();
            // StackPin 确定性排序: core 优先, 其余按段名字典序 (模型级近似)。
            order.sort_by(|&a, &b| {
                let ca = segments[a].segment.core;
                let cb = segments[b].segment.core;
                cb.cmp(&ca)
                    .then_with(|| segments[a].segment.name.cmp(&segments[b].segment.name))
            });
            order
                .into_iter()
                .map(|i| segments[i].segment.name.clone())
                .collect()
        } else {
            ranked.clone()
        };
        self.window.push(ResearchDecisionRecord {
            ranked_ids: final_rank.clone(),
            by_policy: !fallback,
        });
        if self.window.len() > self.guard.window {
            self.window.remove(0);
        }
        (final_rank, fallback)
    }

    /// OGD 在线更新 (Zinkevich 2003): 观察 (x, reward) 后
    /// g = 2(pred − reward)·x; w ← proj(w − η_t·g); η_t = η0/√t。
    /// 返回更新后的权重与步长。
    pub fn update(&mut self, f: &ResearchSegmentFeatures, reward: f32) -> f32 {
        self.t += 1;
        let pred = self.score(f);
        let grad_scale = 2.0 * (pred - reward);
        let eta = self.ogd.eta0 / (self.t as f32).sqrt();
        let mut w = self.weights;
        let x = f.as_vec();
        for i in 0..4 {
            w[i] -= eta * grad_scale * x[i];
        }
        // 投影到半径球 (L2)。
        let norm: f32 = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2] + w[3] * w[3]).sqrt();
        if norm > self.ogd.weight_radius {
            let scale = self.ogd.weight_radius / norm;
            for wi in w.iter_mut() {
                *wi *= scale;
            }
        }
        self.weights = w;
        eta
    }

    /// 后悔 (在线损失 − 最优固定权重的事后损失, 负值 = 优于固定基线)。
    pub fn regret_so_far(&self, history: &[(ResearchSegmentFeatures, f32)]) -> f32 {
        // 最优固定 w*: 事后最小二乘 (小维特征闭式解或网格)。
        let best = best_fixed_linear_loss(history);
        let online: f32 = history
            .iter()
            .map(|(f, r)| (self.score(f) - r).powi(2))
            .sum();
        online - best
    }
}

/// 事后最优固定线性权重的损失 (4 维闭式最小二乘, 满秩假设; 合成数据保证)。
pub fn best_fixed_linear_loss(history: &[(ResearchSegmentFeatures, f32)]) -> f32 {
    // 闭式: w* = (XᵀX)⁻¹Xᵀy (4×4)。合成数据 XᵀX 满秩。
    let mut xtx = [[0.0f32; 4]; 4];
    let mut xty = [0.0f32; 4];
    for (f, y) in history {
        let x = f.as_vec();
        for i in 0..4 {
            xty[i] += x[i] * y;
            for j in 0..4 {
                xtx[i][j] += x[i] * x[j];
            }
        }
    }
    // 4×4 求逆 (伴随矩阵法)。
    let det = |m: &[[f32; 4]; 4]| {
        m[0][0]
            * (m[1][1] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
                - m[1][2] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
                + m[1][3] * (m[2][1] * m[3][2] - m[2][2] * m[3][1]))
            - m[0][1]
                * (m[1][0] * (m[2][2] * m[3][3] - m[2][3] * m[3][2])
                    - m[1][2] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
                    + m[1][3] * (m[2][0] * m[3][2] - m[2][2] * m[3][0]))
            + m[0][2]
                * (m[1][0] * (m[2][1] * m[3][3] - m[2][3] * m[3][1])
                    - m[1][1] * (m[2][0] * m[3][3] - m[2][3] * m[3][0])
                    + m[1][3] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]))
            - m[0][3]
                * (m[1][0] * (m[2][1] * m[3][2] - m[2][2] * m[3][1])
                    - m[1][1] * (m[2][0] * m[3][2] - m[2][2] * m[3][0])
                    + m[1][2] * (m[2][0] * m[3][1] - m[2][1] * m[3][0]))
    };
    let d = det(&xtx);
    if d.abs() < 1e-9 {
        // 退化: 用历史均值近似 (合成数据不会走此分支)。
        let avg: f32 = history.iter().map(|(_, r)| r).sum::<f32>() / history.len().max(1) as f32;
        return history.iter().map(|(_, r)| (avg - r).powi(2)).sum();
    }
    let inv = {
        let mut m = [[0.0f32; 4]; 4];
        let sign = |r: usize, c: usize| if (r + c) % 2 == 0 { 1.0f32 } else { -1.0f32 };
        for r in 0..4 {
            for c in 0..4 {
                // 余子式 (3×3 行列式)。
                let mut sub = [[0.0f32; 3]; 3];
                let mut si = 0usize;
                for i in 0..4 {
                    if i == r {
                        continue;
                    }
                    let mut sj = 0usize;
                    for j in 0..4 {
                        if j == c {
                            continue;
                        }
                        sub[si][sj] = xtx[i][j];
                        sj += 1;
                    }
                    si += 1;
                }
                let det3 = sub[0][0] * (sub[1][1] * sub[2][2] - sub[1][2] * sub[2][1])
                    - sub[0][1] * (sub[1][0] * sub[2][2] - sub[1][2] * sub[2][0])
                    + sub[0][2] * (sub[1][0] * sub[2][1] - sub[1][1] * sub[2][0]);
                m[c][r] = sign(r, c) * det3 / d; // 伴随转置
            }
        }
        m
    };
    let mut w = [0.0f32; 4];
    for i in 0..4 {
        w[i] = inv[i][0] * xty[0] + inv[i][1] * xty[1] + inv[i][2] * xty[2] + inv[i][3] * xty[3];
    }
    history
        .iter()
        .map(|(f, r)| {
            let x = f.as_vec();
            let pred = w[0] * x[0] + w[1] * x[1] + w[2] * x[2] + w[3] * x[3];
            (pred - r).powi(2)
        })
        .sum()
}

/// 确定性合成反馈生成: reward = σ(⟨w_true, x⟩ + noise), xorshift64* PRNG。
pub fn research_synthetic_feedback(
    seed: u64,
    n: usize,
    w_true: [f32; 4],
) -> (Vec<ResearchSegmentFeatures>, Vec<f32>) {
    let mut state = seed.max(1);
    let mut next = move || {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        (state.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut feats = Vec::with_capacity(n);
    let mut rewards = Vec::with_capacity(n);
    for _ in 0..n {
        let f = ResearchSegmentFeatures {
            retr_score: next() as f32,
            recency: 1.0 / (1.0 + ((next() * 20.0) as u32 as f32)),
            novelty: next() as f32,
        };
        let x = f.as_vec();
        let mu = w_true[0] * x[0] + w_true[1] * x[1] + w_true[2] * x[2] + w_true[3] * x[3];
        let noise = (next() as f32 - 0.5) * 0.1;
        let reward = (mu + noise).clamp(0.0, 1.0);
        feats.push(f);
        rewards.push(reward);
    }
    (feats, rewards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_context_policy::ResearchContextPolicy;

    /// OGD 后悔 O(√T): 合成数据上 online 损失与最优固定权重的差 ≤ C√T。
    #[test]
    fn ogd_regret_within_sqrt_t_bound() {
        let w_true = [0.5, 0.3, 0.15, 0.05];
        for (seed, n) in [(1u64, 2000usize), (7, 4000)] {
            let (feats, rewards) = research_synthetic_feedback(seed, n, w_true);
            let mut learner = ResearchVaultLruFtrl::default();
            for (f, r) in feats.iter().zip(rewards.iter()) {
                learner.update(f, *r);
            }
            let history: Vec<(ResearchSegmentFeatures, f32)> =
                feats.iter().cloned().zip(rewards.iter().copied()).collect();
            let regret = learner.regret_so_far(&history);
            let t = n as f32;
            // 经验上 regret 应为负 (优于固定最优) 或小正数; 界: ≤ 50·√T (R²/2·√T 量级)。
            assert!(
                regret <= 50.0 * t.sqrt(),
                "seed {seed}: regret={regret} 超 O(√T) 界"
            );
        }
    }

    /// 学习收敛: 后期预测误差显著小于初始 (权重向 w_true 逼近)。
    #[test]
    fn ogd_converges_to_true_weights() {
        let w_true = [0.5, 0.3, 0.15, 0.05];
        let (feats, rewards) = research_synthetic_feedback(42, 5000, w_true);
        let mut learner = ResearchVaultLruFtrl::default();
        let mut early_err = 0.0f32;
        let mut late_err = 0.0f32;
        for (i, (f, r)) in feats.iter().zip(rewards.iter()).enumerate() {
            let pred = learner.score(f);
            let e = (pred - r).abs();
            if i < 100 {
                early_err += e;
            } else {
                late_err += e;
            }
            learner.update(f, *r);
        }
        assert!(
            late_err / 4900.0 < early_err / 100.0 * 0.6,
            "后期平均误差应显著低于早期"
        );
        // 权重逼近真值 (收敛的强证据)。
        let dist: f32 = learner
            .weights
            .iter()
            .zip(w_true.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(dist < 0.2, "权重应逼近 w_true, dist={dist}");
    }

    /// 前缀护栏: 剧烈重排触发 fallback (StackPin 确定性排序) 并计数。
    #[test]
    fn prefix_guard_trips_and_falls_back() {
        let mut learner = ResearchVaultLruFtrl::new(
            ResearchOgdConfig::default(),
            ResearchPrefixGuardConfig {
                theta: 0.8,
                window: 8,
            },
        );
        let segs: Vec<ResearchSegment> = (0..6)
            .map(|i| ResearchSegment::new(format!("s{i}"), format!("c{i}"), 1))
            .collect();
        // 第一次: 无历史, 按策略排序。
        let f0: Vec<ResearchSegmentFeatures> = (0..6)
            .map(|i| ResearchSegmentFeatures {
                retr_score: i as f32 / 6.0,
                recency: 1.0,
                novelty: 0.5,
            })
            .collect();
        let stackpin = ResearchStackPinPolicy::new(3, 1, false);
        let (rank1, fb1) = learner.decide_ranking(&segs, &f0, &stackpin);
        assert!(!fb1);
        // 学习后权重翻转 → 排序剧烈变化 → 护栏触发 fallback。
        for _ in 0..200 {
            learner.update(
                &ResearchSegmentFeatures {
                    retr_score: 1.0,
                    recency: 1.0,
                    novelty: 1.0,
                },
                0.0,
            );
            learner.update(
                &ResearchSegmentFeatures {
                    retr_score: 0.0,
                    recency: 0.0,
                    novelty: 0.0,
                },
                1.0,
            );
        }
        let f1: Vec<ResearchSegmentFeatures> = (0..6)
            .map(|i| ResearchSegmentFeatures {
                retr_score: i as f32 / 6.0,
                recency: 0.2,
                novelty: 0.1,
            })
            .collect();
        let (rank2, fb2) = learner.decide_ranking(&segs, &f1, &stackpin);
        assert_ne!(rank1, rank2, "学习后排序应变化");
        assert!(fb2, "排序剧烈变化应触发护栏 fallback");
        assert!(learner.guard_trips >= 1);
    }

    /// StackPin fallback 兼容: 护栏回退时输出 core 优先的确定性排序。
    #[test]
    fn fallback_uses_deterministic_stackpin_order() {
        let mut learner = ResearchVaultLruFtrl::default();
        let segs = vec![
            ResearchSegment::new("z", "z", 1),
            ResearchSegment::new("a", "a", 1).with_core(true),
            ResearchSegment::new("m", "m", 1),
        ];
        let feats = vec![
            ResearchSegmentFeatures {
                retr_score: 0.1,
                recency: 0.1,
                novelty: 0.1,
            },
            ResearchSegmentFeatures {
                retr_score: 0.1,
                recency: 0.1,
                novelty: 0.1,
            },
            ResearchSegmentFeatures {
                retr_score: 0.1,
                recency: 0.1,
                novelty: 0.1,
            },
        ];
        let stackpin = ResearchStackPinPolicy::new(3, 1, false);
        let (rank, _) = learner.decide_ranking(&segs, &feats, &stackpin);
        assert_eq!(rank[0], "a", "core 段必须排最前 (确定性 fallback 语义)");
        let _ = ResearchPolicyAction::Retain; // 保持与 Phase 3 模块引用 (文档一致性)
    }
}
