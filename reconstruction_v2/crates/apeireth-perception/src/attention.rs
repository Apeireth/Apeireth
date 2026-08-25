//! Attention - `Attention` trait + TopK / Threshold implementations.
//!
//! **Architecture position**: stage-4 §3.1 `Perception::attention_filter` extracted organ.
//! **Responsibility**: filter a batch of signals down to those worth cognition processing.
//!
//! Ponytail: only 2 built-in policies (TopK + Threshold). Stage 5 can extend to richer models
//! (saliency / sentiment / context-relevant).

use crate::input::PerceptionInput;

/// Attention trait - filter a batch of inputs down to a smaller subset.
pub trait Attention: Send + Sync {
    /// Score an individual input (0.0 - 1.0). Default impl reuses `priority()`.
    fn score<I: PerceptionInput>(&self, input: &I) -> f64 {
        input.priority()
    }
    /// Filter.
    fn filter<I: PerceptionInput>(&self, inputs: Vec<I>) -> Vec<I>;
}

/// Top-K attention - keep the K highest-scoring inputs.
#[derive(Debug, Clone, Copy)]
pub struct TopKAttention {
    /// Retention count.
    pub k: usize,
}

impl TopKAttention {
    /// Construct.
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
        // ponytail: simple sort + truncate. K typically < 50, performance is fine.
        // stable_sort keeps FIFO order for equal scores (matches "first-come first-served").
        inputs.sort_by(|a, b| {
            b.priority()
                .partial_cmp(&a.priority())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        inputs.truncate(self.k);
        inputs
    }
}

/// Threshold attention - keep only inputs with score >= threshold.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdAttention {
    /// Pass-through threshold (0.0 - 1.0).
    pub threshold: f64,
}

impl ThresholdAttention {
    /// Construct.
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

/// Convenience function: Top-K.
pub fn top_k_filter<I: PerceptionInput>(inputs: Vec<I>, k: usize) -> Vec<I> {
    TopKAttention::new(k).filter(inputs)
}

/// Convenience function: threshold.
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
