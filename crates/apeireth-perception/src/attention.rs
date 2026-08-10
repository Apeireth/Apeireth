//! 注意力 — `Attention` trait + TopK / Threshold 实现.
//!
//! **架构位置**: 阶段 4 §3.1 `Perception::attention_filter` 的独立器官版.
//! **职责**: 在大量信号中筛出值得 cognition 处理的子集.
//!
//! ponytail: 仅 2 个内置策略 (TopK + Threshold), 阶段 5 可扩展为更复杂模型
//! (显著性 / 情感 / 上下文相关).

use crate::input::PerceptionInput;

/// 注意力 trait — 把一批输入筛成更少.
pub trait Attention: Send + Sync {
    /// 一条输入的关注分数 (0.0 - 1.0). 默认实现直接复用 `priority()`.
    fn score<I: PerceptionInput>(&self, input: &I) -> f64 {
        input.priority()
    }
    /// 过滤.
    fn filter<I: PerceptionInput>(&self, inputs: Vec<I>) -> Vec<I>;
}

/// Top-K 注意力 — 取分数最高的 K 条.
#[derive(Debug, Clone, Copy)]
pub struct TopKAttention {
    /// 保留数量.
    pub k: usize,
}

impl TopKAttention {
    /// 构造.
    pub fn new(k: usize) -> Self {
        Self { k }
    }
}

impl Default for TopKAttention {
    fn default() -> Self {
        Self { k: 5 }
    }
}

impl Attention for TopKAttention {
    fn filter<I: PerceptionInput>(&self, mut inputs: Vec<I>) -> Vec<I> {
        // ponytail: 简单 sort + truncate. K 通常 < 50, 性能足够.
        // 用 stable_sort 保证相等分数时保持 FIFO (符合"先来先服务"直觉).
        inputs.sort_by(|a, b| {
            b.priority()
                .partial_cmp(&a.priority())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        inputs.truncate(self.k);
        inputs
    }
}

/// 阈值注意力 — 只保留分数 >= threshold 的输入.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdAttention {
    /// 通过阈值 (0.0 - 1.0).
    pub threshold: f64,
}

impl ThresholdAttention {
    /// 构造.
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
        }
    }
}

impl Default for ThresholdAttention {
    fn default() -> Self {
        Self { threshold: 0.5 }
    }
}

impl Attention for ThresholdAttention {
    fn filter<I: PerceptionInput>(&self, inputs: Vec<I>) -> Vec<I> {
        inputs
            .into_iter()
            .filter(|i| i.priority() >= self.threshold)
            .collect()
    }
}

/// 便捷函数: Top-K.
pub fn top_k_filter<I: PerceptionInput>(inputs: Vec<I>, k: usize) -> Vec<I> {
    TopKAttention::new(k).filter(inputs)
}

/// 便捷函数: 阈值.
pub fn threshold_filter<I: PerceptionInput>(inputs: Vec<I>, threshold: f64) -> Vec<I> {
    ThresholdAttention::new(threshold).filter(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{SignalSource, TextInput};

    fn texts(priorities: &[f64]) -> Vec<TextInput> {
        priorities
            .iter()
            .map(|p| TextInput::new("x", SignalSource::Cli).with_priority(*p))
            .collect()
    }

    #[test]
    fn top_k_keeps_highest_priorities() {
        let out = top_k_filter(texts(&[0.1, 0.9, 0.3, 0.8, 0.5]), 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].priority, 0.9);
        assert_eq!(out[1].priority, 0.8);
    }

    #[test]
    fn top_k_k_greater_than_len_returns_all() {
        let out = top_k_filter(texts(&[0.1, 0.2]), 10);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn top_k_k_zero_returns_empty() {
        let out = top_k_filter(texts(&[0.1, 0.2]), 0);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn threshold_filters_below() {
        let out = threshold_filter(texts(&[0.1, 0.5, 0.9]), 0.5);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|t| t.priority >= 0.5));
    }

    #[test]
    fn threshold_clamps_argument() {
        let att = ThresholdAttention::new(2.0);
        assert_eq!(att.threshold, 1.0);
        let att = ThresholdAttention::new(-1.0);
        assert_eq!(att.threshold, 0.0);
    }

    #[test]
    fn default_top_k_is_five() {
        assert_eq!(TopKAttention::default().k, 5);
    }

    #[test]
    fn default_threshold_is_half() {
        assert!((ThresholdAttention::default().threshold - 0.5).abs() < 1e-9);
    }
}
