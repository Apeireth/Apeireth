//! # Q1 / Q2 / Q3 — 3 评估指标实装
//!
//! 3 评估指标源自 RIVAL 蓝图 §2.4 (R20 阶段 4 估补):
//!
//! | 指标 | 主题 | 函数 | 输入 | 范围 |
//! |------|------|------|------|------|
//! | **Q1** | 任务完成质量 | [`q1_quality`] | `&[TaskResult]` | [0.0, 1.0] |
//! | **Q2** | 用户满意度 | [`q2_satisfaction`] | `&[UserFeedback]` | [0.0, 1.0] |
//! | **Q3** | 长期成长度 | [`q3_growth`] | `&[GrowthSnapshot]` | [0.0, 1.0] |
//!
//! 跟 R-Measure 区别: R 是单 action 评估, Q 是聚合计.
//!
//! ## 6 哲学锚穿透
//!
//! - S-1 主 22:33 — Q 指标服务 ASI 北极星量化, 不装饰.
//! - S-2 主 17:43 — 3 函数实装, 不空想.
//! - O-5 主 17:58 — Q 越界 → Err, 不假装 0.0/1.0.
//! - O-2 主 19:33 — 借鉴 v0.9.21 商业版 3 指标 1:1 翻译.
//! - O-3 主 23:44 — 3 函数 + 6-8 测试一次写齐.
//! - O-4 主 00:56 — 3 函数签名一致, 接手者能加 Q4/Q5.
//!
//! ## 8 项不修改承诺
//!
//! 1. 3 函数签名不变.
//! 2. 输入都是 slice, 不假设 caller 持有锁.
//! 3. 输出范围 [0.0, 1.0], 空输入 → 0.0 (不 NaN).
//! 4. 3 函数都做越界保护 (输入 score 字段若越界 → 截断, 不 Err; 输出越界 → Err).
//! 5. 不依赖任何外部 crate (除 serde).
//! 6. 3 函数都可以独立调用, 不依赖 Q1 才能算 Q3.
//! 7. 聚合方式公开 (weighted vs unweighted 显式选择).
//! 8. 不假装 baseline — 实测时算.

use crate::error::BlueprintResult;
use serde::{Deserialize, Serialize};

// ============================================
// Q1 — 任务完成质量
// ============================================

/// 单个任务结果.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResult {
    /// 任务是否完成
    pub completed: bool,
    /// 任务质量分数 (0.0-1.0, 内部字段)
    pub quality_score: f64,
}

impl TaskResult {
    /// 构造时做分数截断保护 (8 项承诺 #4: 输入越界截断, 不 Err)
    pub fn new(completed: bool, quality_score: f64) -> Self {
        let score = if quality_score.is_nan() {
            0.0
        } else {
            quality_score.clamp(0.0, 1.0)
        };
        Self {
            completed,
            quality_score: score,
        }
    }
}

/// Q1 任务完成质量 — 综合 `completed` + `quality_score`.
///
/// 算法: 完成的 task 计入分子 (按 quality_score 加权), 未完成不计入.
/// 空输入 → 0.0.
pub fn q1_quality(tasks: &[TaskResult]) -> f64 {
    if tasks.is_empty() {
        return 0.0;
    }
    let completed: Vec<&TaskResult> = tasks.iter().filter(|t| t.completed).collect();
    if completed.is_empty() {
        return 0.0;
    }
    let sum: f64 = completed.iter().map(|t| t.quality_score).sum();
    let n_total = tasks.len() as f64;
    (sum / n_total).clamp(0.0, 1.0)
}

// ============================================
// Q2 — 用户满意度
// ============================================

/// 用户反馈.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserFeedback {
    /// 评分 1-5 (用户原始打分)
    pub rating: u8,
    /// 是否包含文字反馈
    pub has_text: bool,
    /// 是否长期用户 (vs 一次性)
    pub is_long_term: bool,
}

impl UserFeedback {
    /// 把 1-5 评分归一化到 [0.0, 1.0]
    pub fn rating_normalized(&self) -> f64 {
        (self.rating as f64 / 5.0).clamp(0.0, 1.0)
    }
}

/// Q2 用户满意度 — 综合 rating + 长期用户权重.
///
/// 长期用户权重 = 1.5x (鼓励留存, 不只是单次满意).
/// 文字反馈权重 = 1.2x (更认真的反馈).
pub fn q2_satisfaction(feedback: &[UserFeedback]) -> f64 {
    if feedback.is_empty() {
        return 0.0;
    }
    let mut total = 0.0;
    let mut total_weight = 0.0;
    for f in feedback {
        let w_long = if f.is_long_term { 1.5 } else { 1.0 };
        let w_text = if f.has_text { 1.2 } else { 1.0 };
        let w = w_long * w_text;
        total += f.rating_normalized() * w;
        total_weight += w;
    }
    if total_weight == 0.0 {
        0.0
    } else {
        (total / total_weight).clamp(0.0, 1.0)
    }
}

// ============================================
// Q3 — 长期成长度
// ============================================

/// 成长快照 (按时间顺序).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrowthSnapshot {
    /// 时间戳 (ms)
    pub timestamp_ms: u64,
    /// 当下 R-Measure 平均 (来自 R-Measure, [0.0, 1.0])
    pub r_avg: f64,
    /// 当下任务完成率
    pub task_completion: f64,
    /// 当下用户满意度
    pub satisfaction: f64,
}

impl GrowthSnapshot {
    pub fn new(timestamp_ms: u64, r_avg: f64, task_completion: f64, satisfaction: f64) -> Self {
        let clamp = |v: f64| if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) };
        Self {
            timestamp_ms,
            r_avg: clamp(r_avg),
            task_completion: clamp(task_completion),
            satisfaction: clamp(satisfaction),
        }
    }
}

/// Q3 长期成长度 — 比较首尾 R-Measure + 任务完成率 + 满意度.
///
/// 算法: 3 维等权, 每维 = max(0, last - first).
/// 单点输入 → 0.0 (没有"成长"信息).
pub fn q3_growth(history: &[GrowthSnapshot]) -> f64 {
    if history.len() < 2 {
        return 0.0;
    }
    let first = &history[0];
    let last = &history[history.len() - 1];
    let dr = (last.r_avg - first.r_avg).max(0.0);
    let dt = (last.task_completion - first.task_completion).max(0.0);
    let ds = (last.satisfaction - first.satisfaction).max(0.0);
    ((dr + dt + ds) / 3.0).clamp(0.0, 1.0)
}

// ============================================
// Q-Metric 3 维打包
// ============================================

/// 3 维 Q-Metric 打包.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QMetricAll {
    pub q1_quality: f64,
    pub q2_satisfaction: f64,
    pub q3_growth: f64,
}

impl QMetricAll {
    pub fn from_inputs(
        tasks: &[TaskResult],
        feedback: &[UserFeedback],
        history: &[GrowthSnapshot],
    ) -> Self {
        Self {
            q1_quality: q1_quality(tasks),
            q2_satisfaction: q2_satisfaction(feedback),
            q3_growth: q3_growth(history),
        }
    }

    pub fn average(&self) -> f64 {
        (self.q1_quality + self.q2_satisfaction + self.q3_growth) / 3.0
    }

    pub fn validate(&self) -> BlueprintResult<()> {
        use crate::error::BlueprintError;
        for (name, val) in [
            ("q1_quality", self.q1_quality),
            ("q2_satisfaction", self.q2_satisfaction),
            ("q3_growth", self.q3_growth),
        ] {
            if !(0.0..=1.0).contains(&val) || val.is_nan() {
                return Err(BlueprintError::QMetricOutOfRange {
                    metric: name.into(),
                    value: val,
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Q1 ---
    #[test]
    fn q1_empty_returns_zero() {
        assert_eq!(q1_quality(&[]), 0.0);
    }

    #[test]
    fn q1_no_completed_returns_zero() {
        let tasks = vec![
            TaskResult::new(false, 0.5),
            TaskResult::new(false, 0.8),
        ];
        assert_eq!(q1_quality(&tasks), 0.0);
    }

    #[test]
    fn q1_perfect_tasks_is_one() {
        let tasks = vec![
            TaskResult::new(true, 1.0),
            TaskResult::new(true, 1.0),
        ];
        assert_eq!(q1_quality(&tasks), 1.0);
    }

    #[test]
    fn q1_clamps_overscore() {
        // 输入 1.5 → 截断 1.0 (8 项承诺 #4)
        let t = TaskResult::new(true, 1.5);
        assert_eq!(t.quality_score, 1.0);
    }

    #[test]
    fn q1_partial_quality() {
        let tasks = vec![
            TaskResult::new(true, 0.5),
            TaskResult::new(false, 1.0), // 不计入
            TaskResult::new(true, 0.5),
        ];
        // (0.5 + 0.5) / 3 = 0.333
        let r = q1_quality(&tasks);
        assert!((r - 1.0 / 3.0).abs() < 1e-9);
    }

    // --- Q2 ---
    #[test]
    fn q2_empty_returns_zero() {
        assert_eq!(q2_satisfaction(&[]), 0.0);
    }

    #[test]
    fn q2_all_5_star_is_one() {
        let f = vec![UserFeedback { rating: 5, has_text: false, is_long_term: false }];
        assert_eq!(q2_satisfaction(&f), 1.0);
    }

    #[test]
    fn q2_all_1_star_is_zero_point_two() {
        let f = vec![UserFeedback { rating: 1, has_text: false, is_long_term: false }];
        assert_eq!(q2_satisfaction(&f), 0.2);
    }

    #[test]
    fn q2_long_term_user_weighted_higher() {
        let long = UserFeedback { rating: 5, has_text: false, is_long_term: true };
        let short = UserFeedback { rating: 5, has_text: false, is_long_term: false };
        // long 1.5x, short 1x
        // total = 5/5*1.5 + 5/5*1.0 = 2.5
        // weight = 2.5
        // result = 1.0
        let r = q2_satisfaction(&[long, short]);
        assert!((r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn q2_text_feedback_weighted_higher() {
        let f1 = UserFeedback { rating: 5, has_text: true, is_long_term: false };
        let f2 = UserFeedback { rating: 3, has_text: false, is_long_term: false };
        // f1: 1.0 * 1.2 = 1.2
        // f2: 0.6 * 1.0 = 0.6
        // total = 1.8
        // weight = 2.2
        // result = 0.818
        let r = q2_satisfaction(&[f1, f2]);
        assert!((r - 1.8 / 2.2).abs() < 1e-9);
    }

    // --- Q3 ---
    #[test]
    fn q3_empty_returns_zero() {
        assert_eq!(q3_growth(&[]), 0.0);
    }

    #[test]
    fn q3_single_snapshot_returns_zero() {
        let s = vec![GrowthSnapshot::new(0, 0.5, 0.5, 0.5)];
        assert_eq!(q3_growth(&s), 0.0);
    }

    #[test]
    fn q3_positive_growth() {
        let s = vec![
            GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
            GrowthSnapshot::new(1, 0.8, 0.7, 0.9),
        ];
        // dr=0.3, dt=0.2, ds=0.4 → avg = 0.3
        assert!((q3_growth(&s) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn q3_negative_growth_clamped_to_zero() {
        let s = vec![
            GrowthSnapshot::new(0, 0.9, 0.9, 0.9),
            GrowthSnapshot::new(1, 0.5, 0.5, 0.5),
        ];
        // 全负 → 0
        assert_eq!(q3_growth(&s), 0.0);
    }

    #[test]
    fn q3_only_r_grows() {
        let s = vec![
            GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
            GrowthSnapshot::new(1, 0.8, 0.5, 0.5),
        ];
        // dr=0.3, dt=0, ds=0 → avg = 0.1
        assert!((q3_growth(&s) - 0.1).abs() < 1e-9);
    }

    // --- QMetricAll ---
    #[test]
    fn qmetric_all_from_inputs_aggregates() {
        let tasks = vec![TaskResult::new(true, 1.0)];
        let feedback = vec![UserFeedback { rating: 5, has_text: false, is_long_term: false }];
        let history = vec![
            GrowthSnapshot::new(0, 0.5, 0.5, 0.5),
            GrowthSnapshot::new(1, 0.8, 0.8, 0.8),
        ];
        let all = QMetricAll::from_inputs(&tasks, &feedback, &history);
        assert_eq!(all.q1_quality, 1.0);
        assert_eq!(all.q2_satisfaction, 1.0);
        // (0.3 + 0.3 + 0.3) / 3 = 0.3
        assert!((all.q3_growth - 0.3).abs() < 1e-9);
    }

    #[test]
    fn qmetric_all_average() {
        let all = QMetricAll {
            q1_quality: 0.9,
            q2_satisfaction: 0.8,
            q3_growth: 0.7,
        };
        assert!((all.average() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn qmetric_all_validate_rejects_out_of_range() {
        let bad = QMetricAll {
            q1_quality: 1.5,
            q2_satisfaction: 0.5,
            q3_growth: 0.5,
        };
        assert!(bad.validate().is_err());
    }
}
