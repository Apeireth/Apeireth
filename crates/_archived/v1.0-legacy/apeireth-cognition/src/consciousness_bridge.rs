//! consciousness -> cognition bridge (R172 2026-08-14)
//!
//! 目标: apeireth-consciousness::PlutchikEmotion -> apeireth-cognition::DecisionBias
//!
//! 用户的 Plutchik 情感状态不喂决策 = AI 永远是冷推理. 这是 AI 表现冷的核心原因.
//! 桥 1 让 plutchik 状态影响 cognition 决策风格, 但不改变决策本身 (决策本身仍由 verdict 守门).
//!
//! 不漂移:
//! - 0 改 apeireth-consciousness 任何已实装类型
//! - 0 改 apeireth-cognition 任何已实装类型
//! - 0 副作用, 纯函数
//!
//! 当前状态: R172 最小可用落地 (P0 桥 1 of 7)

#![allow(missing_docs)]

use apeireth_consciousness::plutchik::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};

/// 决策偏置 (4 维, 0.0 - 1.0)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionBias {
    pub creativity: f64,
    pub caution: f64,
    pub cooperation: f64,
    pub exploration: f64,
}

impl Default for DecisionBias {
    fn default() -> Self {
        Self {
            creativity: 0.5,
            caution: 0.5,
            cooperation: 0.5,
            exploration: 0.5,
        }
    }
}

fn intensity_weight(intensity: PlutchikIntensity) -> f64 {
    match intensity {
        PlutchikIntensity::Mild => 0.25,
        PlutchikIntensity::Moderate => 0.5,
        PlutchikIntensity::Strong => 0.75,
        PlutchikIntensity::Extreme => 1.0,
    }
}

/// 把 Plutchik 情绪转换为 DecisionBias. 0 副作用, 0 改源/目标. 纯函数.
pub fn plutchik_to_decision_bias(e: &PlutchikEmotion) -> DecisionBias {
    let mut bias = DecisionBias::default();
    let intensity = intensity_weight(e.intensity());
    match e {
        PlutchikEmotion::Basic(b, _) => apply_basic(b, &mut bias, intensity),
        PlutchikEmotion::Advanced(a, _) => apply_advanced(a, &mut bias, intensity),
    }
    bias.creativity = bias.creativity.clamp(0.0, 1.0);
    bias.caution = bias.caution.clamp(0.0, 1.0);
    bias.cooperation = bias.cooperation.clamp(0.0, 1.0);
    bias.exploration = bias.exploration.clamp(0.0, 1.0);
    bias
}

fn apply_basic(b: &PlutchikBasic, bias: &mut DecisionBias, intensity: f64) {
    match b {
        PlutchikBasic::Joy => {
            bias.creativity += 0.3 * intensity;
            bias.exploration += 0.2 * intensity;
        }
        PlutchikBasic::Trust => {
            bias.cooperation += 0.4 * intensity;
            bias.caution -= 0.1 * intensity;
        }
        PlutchikBasic::Fear => {
            bias.caution += 0.4 * intensity;
            bias.exploration -= 0.2 * intensity;
        }
        PlutchikBasic::Surprise => {
            bias.exploration += 0.3 * intensity;
            bias.creativity += 0.2 * intensity;
        }
        PlutchikBasic::Sadness => {
            bias.caution += 0.2 * intensity;
            bias.creativity -= 0.1 * intensity;
        }
        PlutchikBasic::Disgust => {
            bias.cooperation -= 0.3 * intensity;
            bias.caution += 0.2 * intensity;
        }
        PlutchikBasic::Anger => {
            bias.caution -= 0.2 * intensity;
            bias.creativity += 0.1 * intensity;
        }
        PlutchikBasic::Anticipation => {
            bias.exploration += 0.3 * intensity;
            bias.creativity += 0.1 * intensity;
        }
    }
}

fn apply_advanced(a: &PlutchikAdvanced, bias: &mut DecisionBias, intensity: f64) {
    match a {
        PlutchikAdvanced::Love => {
            bias.creativity += 0.2 * intensity;
            bias.cooperation += 0.4 * intensity;
        }
        PlutchikAdvanced::Submission => {
            bias.cooperation += 0.3 * intensity;
            bias.caution += 0.2 * intensity;
        }
        PlutchikAdvanced::Awe => {
            bias.caution += 0.3 * intensity;
            bias.exploration += 0.2 * intensity;
        }
        PlutchikAdvanced::Disapproval => {
            bias.caution += 0.2 * intensity;
            bias.cooperation -= 0.2 * intensity;
        }
        PlutchikAdvanced::Remorse => {
            bias.caution += 0.3 * intensity;
            bias.creativity -= 0.2 * intensity;
        }
        PlutchikAdvanced::Contempt => {
            bias.cooperation -= 0.4 * intensity;
            bias.caution += 0.1 * intensity;
        }
        PlutchikAdvanced::Aggressiveness => {
            bias.caution -= 0.3 * intensity;
            bias.creativity += 0.2 * intensity;
        }
        PlutchikAdvanced::Optimism => {
            bias.exploration += 0.3 * intensity;
            bias.creativity += 0.2 * intensity;
        }
    }
}

/// 累积多个 DecisionBias (求和 + 平均, 起点为 0, 修复后累加正确)
pub fn accumulate_biases(biases: &[DecisionBias]) -> DecisionBias {
    if biases.is_empty() {
        return DecisionBias::default();
    }
    let mut sum_creativity = 0.0_f64;
    let mut sum_caution = 0.0_f64;
    let mut sum_cooperation = 0.0_f64;
    let mut sum_exploration = 0.0_f64;
    let n = biases.len() as f64;
    for b in biases {
        sum_creativity += b.creativity;
        sum_caution += b.caution;
        sum_cooperation += b.cooperation;
        sum_exploration += b.exploration;
    }
    DecisionBias {
        creativity: (sum_creativity / n).clamp(0.0, 1.0),
        caution: (sum_caution / n).clamp(0.0, 1.0),
        cooperation: (sum_cooperation / n).clamp(0.0, 1.0),
        exploration: (sum_exploration / n).clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_joy_boosts_creativity() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        let bias = plutchik_to_decision_bias(&e);
        assert!(bias.creativity > 0.5);
    }

    #[test]
    fn t02_fear_boosts_caution() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Fear, PlutchikIntensity::Strong);
        let bias = plutchik_to_decision_bias(&e);
        assert!(bias.caution > 0.5);
    }

    #[test]
    fn t03_trust_boosts_cooperation() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Trust, PlutchikIntensity::Strong);
        let bias = plutchik_to_decision_bias(&e);
        assert!(bias.cooperation > 0.5);
    }

    #[test]
    fn t04_disgust_reduces_cooperation() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Disgust, PlutchikIntensity::Strong);
        let bias = plutchik_to_decision_bias(&e);
        assert!(bias.cooperation < 0.5);
    }

    #[test]
    fn t05_intensity_scales_effect() {
        let mild = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Mild);
        let extreme = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let bias_mild = plutchik_to_decision_bias(&mild);
        let bias_extreme = plutchik_to_decision_bias(&extreme);
        assert!(bias_extreme.creativity > bias_mild.creativity);
    }

    #[test]
    fn t06_advanced_emotion_works() {
        let e = PlutchikEmotion::advanced(PlutchikAdvanced::Optimism, PlutchikIntensity::Moderate);
        let bias = plutchik_to_decision_bias(&e);
        assert!(bias.exploration > 0.5);
    }

    #[test]
    fn t07_accumulate_averages() {
        let e1 = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        let e2 = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Strong);
        let biases = vec![
            plutchik_to_decision_bias(&e1),
            plutchik_to_decision_bias(&e2),
        ];
        let acc = accumulate_biases(&biases);
        assert!((acc.creativity - 0.5).abs() < 0.1);
    }

    #[test]
    fn t08_biases_clamps_to_unit() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let b = plutchik_to_decision_bias(&e);
        assert!(b.creativity <= 1.0);
        assert!(b.creativity >= 0.0);
    }
}
