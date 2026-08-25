//! consciousness -> cognition bridge (R172 2026-08-14)
//!
//! Goal: PlutchikEmotion -> DecisionBias
//!
//! **v2 architecture note**: v2 does not depend on a separate `apeireth-consciousness` crate.
//! Instead, this module defines a minimal local `PlutchikEmotion` enum (Basic + Advanced + Intensity)
//! that exactly captures the v1 bridge contract. The conversion algorithm is preserved bit-for-bit
//! from v1 (same intensity weights, same delta table).
//!
//! **No drift**:
//! - 0 changes to apeireth-consciousness (v2 has no such crate)
//! - 0 changes to apeireth-cognition installed types
//! - 0 side effects, pure function
//!
//! Current state: minimal viable landing (P0 bridge 1 of 7).

//! Plutchik emotion intensity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlutchikIntensity {
    /// Mild intensity (weight 0.25).
    Mild,
    /// Moderate intensity (weight 0.50).
    Moderate,
    /// Strong intensity (weight 0.75).
    Strong,
    /// Extreme intensity (weight 1.00).
    Extreme,
}

impl PlutchikIntensity {
    /// Numeric weight (matches v1: Mild=0.25, Moderate=0.5, Strong=0.75, Extreme=1.0).
    pub fn weight(&self) -> f64 {
        match self {
            Self::Mild => 0.25,
            Self::Moderate => 0.5,
            Self::Strong => 0.75,
            Self::Extreme => 1.0,
        }
    }
}

/// Plutchik basic 8 emotions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlutchikBasic {
    Joy,
    Trust,
    Fear,
    Surprise,
    Sadness,
    Disgust,
    Anger,
    Anticipation,
}

/// Plutchik advanced 8 emotions (diatonic combinations).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlutchikAdvanced {
    Love,
    Submission,
    Awe,
    Disapproval,
    Remorse,
    Contempt,
    Aggressiveness,
    Optimism,
}

/// Plutchik emotion (Basic or Advanced + Intensity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlutchikEmotion {
    /// Basic emotion with intensity.
    Basic(PlutchikBasic, PlutchikIntensity),
    /// Advanced emotion with intensity.
    Advanced(PlutchikAdvanced, PlutchikIntensity),
}

impl PlutchikEmotion {
    /// Construct a Basic emotion.
    pub fn basic(b: PlutchikBasic, i: PlutchikIntensity) -> Self {
        Self::Basic(b, i)
    }
    /// Construct an Advanced emotion.
    pub fn advanced(a: PlutchikAdvanced, i: PlutchikIntensity) -> Self {
        Self::Advanced(a, i)
    }
    /// Intensity accessor.
    pub fn intensity(&self) -> PlutchikIntensity {
        match self {
            Self::Basic(_, i) => *i,
            Self::Advanced(_, i) => *i,
        }
    }
}

/// Decision bias (4 dimensions, 0.0 - 1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionBias {
    /// Creativity bias.
    pub creativity: f64,
    /// Caution bias.
    pub caution: f64,
    /// Cooperation bias.
    pub cooperation: f64,
    /// Exploration bias.
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
    intensity.weight()
}

/// Convert a Plutchik emotion to DecisionBias. Pure function, 0 side effects.
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

/// Accumulate multiple DecisionBias (average, clamped to [0, 1]).
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

    #[test]
    fn intensity_weights_match_v1() {
        assert!((PlutchikIntensity::Mild.weight() - 0.25).abs() < 1e-9);
        assert!((PlutchikIntensity::Moderate.weight() - 0.5).abs() < 1e-9);
        assert!((PlutchikIntensity::Strong.weight() - 0.75).abs() < 1e-9);
        assert!((PlutchikIntensity::Extreme.weight() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn accumulate_empty_returns_default() {
        let bias = accumulate_biases(&[]);
        assert_eq!(bias, DecisionBias::default());
    }
}
