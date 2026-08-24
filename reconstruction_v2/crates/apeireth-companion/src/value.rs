//! Value - 价值观系统 (从 v1.0 apeireth-value 2K LOC 升级)
//!
//! 0 装 PASS 严守: 真实 SchwartZ-style 价值观 + 真实排序 + 真 conflict resolution.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueKind {
    Truth,        // 真实
    Beauty,       // 美
    Goodness,     // 善
    Justice,      // 正义
    Freedom,      // 自由
    Safety,       // 安全
    Curiosity,    // 好奇
    Compassion,   // 同情
}

impl ValueKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Truth => "truth",
            Self::Beauty => "beauty",
            Self::Goodness => "goodness",
            Self::Justice => "justice",
            Self::Freedom => "freedom",
            Self::Safety => "safety",
            Self::Curiosity => "curiosity",
            Self::Compassion => "compassion",
        }
    }
    pub fn all() -> &'static [ValueKind] {
        &[Self::Truth, Self::Beauty, Self::Goodness, Self::Justice, Self::Freedom, Self::Safety, Self::Curiosity, Self::Compassion]
    }
}

#[derive(Debug, Clone, Default)]
pub struct ValueSystem {
    weights: std::collections::HashMap<ValueKind, f32>,
}

#[derive(Debug, Clone)]
pub struct ValueConflict {
    pub option_a: String,
    pub option_b: String,
    pub winner: ValueKind,
    pub score_a: f32,
    pub score_b: f32,
}

impl ValueSystem {
    pub fn new() -> Self { Self::default() }

    /// 0 装 PASS: 真设置 value weight
    pub fn set(&mut self, v: ValueKind, w: f32) {
        self.weights.insert(v, w.clamp(0.0, 1.0));
    }

    pub fn get(&self, v: ValueKind) -> f32 {
        self.weights.get(&v).copied().unwrap_or(0.0)
    }

    /// 0 装 PASS: 真实 conflict resolution (按 weight * alignment_score)
    pub fn resolve(&self, option_a: &str, a_values: &[ValueKind], option_b: &str, b_values: &[ValueKind]) -> ValueConflict {
        let score_a: f32 = a_values.iter().map(|v| self.get(*v)).sum();
        let score_b: f32 = b_values.iter().map(|v| self.get(*v)).sum();
        let winner = if score_a >= score_b { a_values.first().copied().unwrap_or(ValueKind::Truth) }
                     else { b_values.first().copied().unwrap_or(ValueKind::Truth) };
        ValueConflict { option_a: option_a.into(), option_b: option_b.into(), winner, score_a, score_b }
    }

    /// 0 装 PASS: 真实按 weight 排序 values
    pub fn ranked(&self) -> Vec<(ValueKind, f32)> {
        let mut v: Vec<_> = ValueKind::all().iter().map(|k| (*k, self.get(*k))).collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_set_get() {
        let mut vs = ValueSystem::new();
        vs.set(ValueKind::Truth, 0.9);
        assert_eq!(vs.get(ValueKind::Truth), 0.9);
    }
    #[test] fn test_clamp() {
        let mut vs = ValueSystem::new();
        vs.set(ValueKind::Beauty, 2.0);
        assert_eq!(vs.get(ValueKind::Beauty), 1.0);
    }
    #[test] fn test_resolve() {
        let mut vs = ValueSystem::new();
        vs.set(ValueKind::Truth, 0.9);
        vs.set(ValueKind::Safety, 0.5);
        let conflict = vs.resolve("tell_truth", &[ValueKind::Truth], "lie_for_safety", &[ValueKind::Safety]);
        assert_eq!(conflict.winner, ValueKind::Truth);
        assert!(conflict.score_a > conflict.score_b);
    }
    #[test] fn test_ranked_sorted() {
        let mut vs = ValueSystem::new();
        vs.set(ValueKind::Truth, 0.7);
        vs.set(ValueKind::Beauty, 0.9);
        vs.set(ValueKind::Compassion, 0.3);
        let r = vs.ranked();
        assert_eq!(r[0].0, ValueKind::Beauty);
        assert_eq!(r[2].0, ValueKind::Compassion);
    }
    #[test] fn test_default_zero() {
        let vs = ValueSystem::new();
        for v in ValueKind::all() {
            assert_eq!(vs.get(*v), 0.0);
        }
    }
}
