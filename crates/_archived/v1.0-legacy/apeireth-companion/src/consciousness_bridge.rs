//! bridge 5: consciousness -> companion (R173 2026-08-14)
//!
//! 目标: apeireth-consciousness::PlutchikEmotion -> apeireth-companion::Bond / BondCharacter.
//!
//! 关系里没有情感回响 = 关系是死的. 让情感进入关系是伙伴的核心.
//! 桥 5 把 Plutchik 情感状态翻译为 companion::BondCharacter.apply_emotion 的 8 维输入.
//!
//! 不漂移:
//! - 0 改 apeireth-consciousness 任何已实装类型
//! - 0 改 apeireth-companion 任何已实装类型 (复用 BondCharacter::apply_emotion)
//! - 0 副作用, 纯函数 (除最终 apply_emotion 调用)
//!
//! 当前状态: R173 最小可用落地 (P0 桥 5 of 7)

#![deny(unsafe_code)]

use crate::bond::{Bond, BondCharacter};
use apeireth_consciousness::plutchik::{PlutchikBasic, PlutchikEmotion, PlutchikIntensity};

/// 8 维 Plutchik 基本情感输入 (按 PlutchikBasic::ALL 顺序 + intensity 加权)
///
/// 与 BondCharacter::apply_emotion 的 8 参数一一对应:
/// joy / trust / fear / surprise / sadness / disgust / anger / anticipation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlutchikBondInputs {
    pub joy: f64,
    pub trust: f64,
    pub fear: f64,
    pub surprise: f64,
    pub sadness: f64,
    pub disgust: f64,
    pub anger: f64,
    pub anticipation: f64,
}

impl Default for PlutchikBondInputs {
    fn default() -> Self {
        Self {
            joy: 0.0,
            trust: 0.0,
            fear: 0.0,
            surprise: 0.0,
            sadness: 0.0,
            disgust: 0.0,
            anger: 0.0,
            anticipation: 0.0,
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

fn clamp_unit(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn clamp_inputs(i: &mut PlutchikBondInputs) {
    i.joy = clamp_unit(i.joy);
    i.trust = clamp_unit(i.trust);
    i.fear = clamp_unit(i.fear);
    i.surprise = clamp_unit(i.surprise);
    i.sadness = clamp_unit(i.sadness);
    i.disgust = clamp_unit(i.disgust);
    i.anger = clamp_unit(i.anger);
    i.anticipation = clamp_unit(i.anticipation);
}

fn apply_basic(b: &PlutchikBasic, inputs: &mut PlutchikBondInputs, intensity: f64) {
    match b {
        PlutchikBasic::Joy => inputs.joy = intensity,
        PlutchikBasic::Trust => inputs.trust = intensity,
        PlutchikBasic::Fear => inputs.fear = intensity,
        PlutchikBasic::Surprise => inputs.surprise = intensity,
        PlutchikBasic::Sadness => inputs.sadness = intensity,
        PlutchikBasic::Disgust => inputs.disgust = intensity,
        PlutchikBasic::Anger => inputs.anger = intensity,
        PlutchikBasic::Anticipation => inputs.anticipation = intensity,
    }
}

/// 把 Plutchik 情绪转换为 8 维 Bond 情感输入. 0 副作用, 0 改源/目标. 纯函数.
///
/// 设计要点 (per youyou 哲学杂谈):
/// - 用户感受是真理: 高级情感暂时不直接进 Bond (只 basic 进), 留 bridge 5 的设计延展性
/// - bond.char.apply_emotion 本身已有 trust, resonance 调权逻辑; 我们不动它
pub fn plutchik_to_bond_emotion(e: &PlutchikEmotion) -> PlutchikBondInputs {
    let mut inputs = PlutchikBondInputs::default();
    let intensity = intensity_weight(e.intensity());
    match e {
        PlutchikEmotion::Basic(b, _) => apply_basic(b, &mut inputs, intensity),
        // Advanced 不直接进 Bond; 设计上的理由是 Bond character 反映可观察的"基本情感", 高级是基本组合. 留白.
        PlutchikEmotion::Advanced(_, _) => {}
    }
    clamp_inputs(&mut inputs);
    inputs
}

/// 在 BondCharacter 上注入 Plutchik 情感 (per 桥 5 入口)
pub fn apply_plutchik_to_character(character: &mut BondCharacter, e: &PlutchikEmotion) {
    let inputs = plutchik_to_bond_emotion(e);
    character.apply_emotion(
        inputs.joy,
        inputs.trust,
        inputs.fear,
        inputs.surprise,
        inputs.sadness,
        inputs.disgust,
        inputs.anger,
        inputs.anticipation,
    );
}

/// 在 Bond 上注入 Plutchik 情感 (穿透到 Character)
pub fn apply_plutchik_to_bond(bond: &mut Bond, e: &PlutchikEmotion) {
    let inputs = plutchik_to_bond_emotion(e);
    bond.apply_emotion(
        inputs.joy,
        inputs.trust,
        inputs.fear,
        inputs.surprise,
        inputs.sadness,
        inputs.disgust,
        inputs.anger,
        inputs.anticipation,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_joy_strong_raises_resonance() {
        let mut character = BondCharacter::new();
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        apply_plutchik_to_character(&mut character, &e);
        // bond.character.apply_emotion: resonance = (0 * 0.7 + (joy+trust+anticipation)*0.1)
        // joy=0.75 -> resonance += 0.075
        assert!(character.resonance > 0.0);
    }

    #[test]
    fn t02_trust_extreme_raises_trust_field() {
        let mut character = BondCharacter::new();
        let e = PlutchikEmotion::basic(PlutchikBasic::Trust, PlutchikIntensity::Extreme);
        apply_plutchik_to_character(&mut character, &e);
        // trust = (0 * 0.8 + trust * 0.2) = 1.0 * 0.2 = 0.2
        assert!(character.trust > 0.0);
        assert!(character.trust <= 1.0);
    }

    #[test]
    fn t03_plutchik_inputs_match_basic_for_basic_emotion() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Anger, PlutchikIntensity::Moderate);
        let inputs = plutchik_to_bond_emotion(&e);
        assert_eq!(inputs.anger, 0.5);
        assert_eq!(inputs.joy, 0.0);
        assert_eq!(inputs.trust, 0.0);
        assert_eq!(inputs.fear, 0.0);
        assert_eq!(inputs.surprise, 0.0);
        assert_eq!(inputs.sadness, 0.0);
        assert_eq!(inputs.disgust, 0.0);
        assert_eq!(inputs.anticipation, 0.0);
    }

    #[test]
    fn t04_advanced_emotion_maps_to_zero_inputs() {
        // 高级情绪暂时不出现在 Bond 输入 (设计上: Bond character 反映可观察基本情感)
        // 这一行为的正确性本身就是测试 — 设计延展性
        let e = PlutchikEmotion::advanced(
            apeireth_consciousness::plutchik::PlutchikAdvanced::Optimism,
            PlutchikIntensity::Extreme,
        );
        let inputs = plutchik_to_bond_emotion(&e);
        assert_eq!(inputs.joy, 0.0);
        assert_eq!(inputs.trust, 0.0);
        assert_eq!(inputs.anticipation, 0.0);
    }

    #[test]
    fn t05_apply_to_bond_passes_through() {
        let mut bond = Bond::new();
        let before_resonance = bond.character().resonance;
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        apply_plutchik_to_bond(&mut bond, &e);
        let after_resonance = bond.character().resonance;
        assert!(after_resonance > before_resonance);
    }

    #[test]
    fn t06_intensity_scales_inputs() {
        let mild = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Mild);
        let extreme = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let m = plutchik_to_bond_emotion(&mild);
        let e = plutchik_to_bond_emotion(&extreme);
        assert!(e.joy > m.joy);
        assert_eq!(m.joy, 0.25);
        assert_eq!(e.joy, 1.0);
    }
}
