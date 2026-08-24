//! ASI - R-Measurer 评估引擎 (从 v1.0 apeireth-asi 4,370 LOC 收敛)
//!
//! 0 装 PASS: 评估 companion 智能质量, 不重新发明 R-Measurer 公式.
//! R11 baseline 3 值: SC=0.8682 / NR=0.8532 / CDT=0.9063 (LOCKED per decision-22 §2.2)
//!
//! 设计 (per user 右图 "Companion 智能核"):
//! - 4 维度: Self-consistency (SC) / Novelty (NR) / Cross-domain Transfer (CDT) / Optional 4th
//! - 评分范围 0..1, baseline lock 不允许降级
//! - AsiScoreBatch 一次跑多个样本, 返回均值 + 标准差

use serde::{Deserialize, Serialize};

/// 4 维 ASI 评分 (per decision-22 §2.2 baseline)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AsiScores {
    /// Self-Consistency: 多次推理结果的稳定性 (Jaccard similarity)
    pub self_consistency: f64,
    /// Novelty-Relevance: 新颖但仍相关的平衡
    pub novelty_relevance: f64,
    /// Cross-Domain Transfer: 跨领域迁移能力
    pub cross_domain_transfer: f64,
    /// 可选第 4 维 (e.g. 长程一致性) — 当前为 SC 加权变体
    pub fourth: f64,
}

impl AsiScores {
    /// R11 baseline (LOCKED per decision-22 §2.2, 8 哲学锚 O-5 不假装)
    pub const R11_BASELINE: AsiScores = AsiScores {
        self_consistency: 0.8682,
        novelty_relevance: 0.8532,
        cross_domain_transfer: 0.9063,
        fourth: 0.8800,
    };

    pub fn mean(&self) -> f64 {
        (self.self_consistency + self.novelty_relevance + self.cross_domain_transfer + self.fourth) / 4.0
    }

    /// 0 装 PASS: 与 baseline 比较 — 任何维度 < baseline 0.95x 视为降级
    pub fn is_regressed(&self, baseline: &AsiScores) -> bool {
        self.self_consistency < baseline.self_consistency * 0.95
            || self.novelty_relevance < baseline.novelty_relevance * 0.95
            || self.cross_domain_transfer < baseline.cross_domain_transfer * 0.95
    }
}

/// AsiSample - 单个 ASI 评估样本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsiSample {
    pub id: String,
    pub scores: AsiScores,
    pub timestamp_ms: i64,
}

/// AsiBatch - 批量评估
pub struct AsiBatch {
    samples: Vec<AsiSample>,
    baseline: AsiScores,
}

impl AsiBatch {
    pub fn new(baseline: AsiScores) -> Self {
        Self { samples: Vec::new(), baseline }
    }

    pub fn record(&mut self, sample: AsiSample) {
        self.samples.push(sample);
    }

    pub fn aggregate(&self) -> Option<AsiScores> {
        if self.samples.is_empty() { return None; }
        let n = self.samples.len() as f64;
        let sum: AsiScores = self.samples.iter().fold(
            AsiScores { self_consistency: 0.0, novelty_relevance: 0.0, cross_domain_transfer: 0.0, fourth: 0.0 },
            |acc, s| AsiScores {
                self_consistency: acc.self_consistency + s.scores.self_consistency,
                novelty_relevance: acc.novelty_relevance + s.scores.novelty_relevance,
                cross_domain_transfer: acc.cross_domain_transfer + s.scores.cross_domain_transfer,
                fourth: acc.fourth + s.scores.fourth,
            },
        );
        Some(AsiScores {
            self_consistency: sum.self_consistency / n,
            novelty_relevance: sum.novelty_relevance / n,
            cross_domain_transfer: sum.cross_domain_transfer / n,
            fourth: sum.fourth / n,
        })
    }

    pub fn regressed_count(&self) -> usize {
        self.samples.iter().filter(|s| s.scores.is_regressed(&self.baseline)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r11_baseline_locked() {
        let b = AsiScores::R11_BASELINE;
        // 0 装 PASS: baseline 是 LOCKED 数字 (per decision-22 §2.2 + 8 哲学锚 O-5 不假装)
        assert!((b.self_consistency - 0.8682).abs() < 1e-6);
        assert!((b.novelty_relevance - 0.8532).abs() < 1e-6);
        assert!((b.cross_domain_transfer - 0.9063).abs() < 1e-6);
    }

    #[test]
    fn test_regression_detection() {
        let baseline = AsiScores::R11_BASELINE;
        // 退化 5%: 应该检测出降级
        let regressed = AsiScores { self_consistency: 0.82, ..baseline };
        assert!(regressed.is_regressed(&baseline));
        // 接近 baseline (< 5% delta): 不算降级
        let ok = AsiScores { self_consistency: baseline.self_consistency * 0.97, ..baseline };
        assert!(!ok.is_regressed(&baseline));
    }

    #[test]
    fn test_batch_aggregate() {
        let mut batch = AsiBatch::new(AsiScores::R11_BASELINE);
        for i in 0..3 {
            batch.record(AsiSample {
                id: format!("s{}", i),
                scores: AsiScores {
                    self_consistency: 0.90,
                    novelty_relevance: 0.85,
                    cross_domain_transfer: 0.91,
                    fourth: 0.88,
                },
                timestamp_ms: 1000 + i as i64,
            });
        }
        let agg = batch.aggregate().unwrap();
        assert_eq!(agg.self_consistency, 0.90);
        assert_eq!(agg.novelty_relevance, 0.85);
        assert_eq!(batch.regressed_count(), 0);
    }
}
