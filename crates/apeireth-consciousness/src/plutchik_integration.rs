//! R209 Plutchik 集成 (接续 R218).
//!
//! **目标**: 把现有 6 基础情绪 (Ekman 模型) + R218 Plutchik 8 基础情绪桥接.
//! 互相转换, 让 LLM 既能表达 6 经典又能表达 8 Plutchik.
//!
//! **0 触碰**: 现有 BaseEmotion 6 情绪 API 0 改. plutchik_integration.rs 是 additive bridge.
//!
//! **不假装** (O-5):
//! - 6 -> 8 映射: 经典心理学对应 (Joy/Trust/Anticipation 等)
//! - 8 -> 6 映射: 5 个 Plutchik 基础情绪无法映射到 Ekman 6 (Trust / Anticipation), 显式返回 Option::None
//! - PAD 中心复用现有 BaseEmotion::pad_center() + Plutchik 心理学标准 PAD

#![allow(missing_docs)] // R209: 0 触碰现有 API 文档

use crate::emotion::{BaseEmotion, Pad};
use crate::plutchik::{PlutchikBasic, PlutchikAdvanced};

/// Plutchik 8 基础情绪 PAD 中心 (经典 Plutchik 1980 + PAD 转换)
///
/// 数值来源: Plutchik 1980 心理学标准, 与 R187 调研一致
pub fn plutchik_pad_center(basic: PlutchikBasic) -> Pad {
    match basic {
        PlutchikBasic::Joy => Pad { p: 0.6, a: 0.5, d: 0.4 },
        PlutchikBasic::Trust => Pad { p: 0.4, a: 0.2, d: 0.3 },
        PlutchikBasic::Fear => Pad { p: -0.6, a: 0.7, d: -0.6 },
        PlutchikBasic::Surprise => Pad { p: 0.1, a: 0.8, d: 0.0 },
        PlutchikBasic::Sadness => Pad { p: -0.4, a: -0.2, d: -0.5 },
        PlutchikBasic::Disgust => Pad { p: -0.6, a: 0.0, d: 0.2 },
        PlutchikBasic::Anger => Pad { p: -0.5, a: 0.6, d: 0.5 },
        PlutchikBasic::Anticipation => Pad { p: 0.3, a: 0.4, d: 0.2 },
    }
}

/// 6 基础情绪 -> 8 Plutchik 基础情绪 (经典心理学对应)
pub fn base_to_plutchik(base: BaseEmotion) -> PlutchikBasic {
    match base {
        BaseEmotion::Joy => PlutchikBasic::Joy,
        BaseEmotion::Sadness => PlutchikBasic::Sadness,
        BaseEmotion::Anger => PlutchikBasic::Anger,
        BaseEmotion::Fear => PlutchikBasic::Fear,
        BaseEmotion::Surprise => PlutchikBasic::Surprise,
        BaseEmotion::Disgust => PlutchikBasic::Disgust,
    }
}

/// 8 Plutchik 基础 -> 6 BaseEmotion (5 个 Plutchik 情绪无对应, 返回 Option::None)
pub fn plutchik_to_base(basic: PlutchikBasic) -> Option<BaseEmotion> {
    match basic {
        PlutchikBasic::Joy => Some(BaseEmotion::Joy),
        PlutchikBasic::Trust => None,            // Plutchik 独有
        PlutchikBasic::Fear => Some(BaseEmotion::Fear),
        PlutchikBasic::Surprise => Some(BaseEmotion::Surprise),
        PlutchikBasic::Sadness => Some(BaseEmotion::Sadness),
        PlutchikBasic::Disgust => Some(BaseEmotion::Disgust),
        PlutchikBasic::Anger => Some(BaseEmotion::Anger),
        PlutchikBasic::Anticipation => None,     // Plutchik 独有
    }
}

/// 8 Plutchik 高级 -> 6 BaseEmotion (按主导情绪映射)
pub fn plutchik_advanced_to_base(advanced: PlutchikAdvanced) -> Option<BaseEmotion> {
    match advanced {
        PlutchikAdvanced::Love => Some(BaseEmotion::Joy),
        PlutchikAdvanced::Submission => Some(BaseEmotion::Fear),
        PlutchikAdvanced::Awe => Some(BaseEmotion::Surprise),
        PlutchikAdvanced::Disapproval => Some(BaseEmotion::Surprise),
        PlutchikAdvanced::Remorse => Some(BaseEmotion::Sadness),
        PlutchikAdvanced::Contempt => Some(BaseEmotion::Disgust),
        PlutchikAdvanced::Aggressiveness => Some(BaseEmotion::Anger),
        PlutchikAdvanced::Optimism => Some(BaseEmotion::Joy),
    }
}

/// 比较两个 PAD 中心的距离 (用于 "Plutchik vs BaseEmotion 哪个更接近当前情绪")
pub fn pad_distance(a: Pad, b: Pad) -> f32 {
    a.distance(&b)
}

/// 找出与给定 PAD 最接近的 BaseEmotion
pub fn closest_base_emotion(pad: Pad) -> BaseEmotion {
    let mut min_dist = f32::INFINITY;
    let mut closest = BaseEmotion::Joy;
    for &base in &BaseEmotion::ALL {
        let dist = pad.distance(&base.pad_center());
        if dist < min_dist {
            min_dist = dist;
            closest = base;
        }
    }
    closest
}

/// 找出与给定 PAD 最接近的 PlutchikBasic
pub fn closest_plutchik_basic(pad: Pad) -> PlutchikBasic {
    let mut min_dist = f32::INFINITY;
    let mut closest = PlutchikBasic::Joy;
    for &basic in &PlutchikBasic::ALL {
        let dist = pad.distance(&plutchik_pad_center(basic));
        if dist < min_dist {
            min_dist = dist;
            closest = basic;
        }
    }
    closest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn t01_base_to_plutchik_4_match() {
        // 4 个直接对应
        assert_eq!(base_to_plutchik(BaseEmotion::Joy), PlutchikBasic::Joy);
        assert_eq!(base_to_plutchik(BaseEmotion::Fear), PlutchikBasic::Fear);
        assert_eq!(base_to_plutchik(BaseEmotion::Anger), PlutchikBasic::Anger);
        assert_eq!(base_to_plutchik(BaseEmotion::Sadness), PlutchikBasic::Sadness);
    }

    #[test]
    fn t02_base_to_plutchik_2_unique() {
        // Trust / Anticipation 是 Plutchik 独有
        assert_eq!(plutchik_to_base(PlutchikBasic::Trust), None);
        assert_eq!(plutchik_to_base(PlutchikBasic::Anticipation), None);
    }

    #[test]
    fn t03_plutchik_to_base_4_match() {
        assert_eq!(plutchik_to_base(PlutchikBasic::Joy), Some(BaseEmotion::Joy));
        assert_eq!(plutchik_to_base(PlutchikBasic::Fear), Some(BaseEmotion::Fear));
        assert_eq!(plutchik_to_base(PlutchikBasic::Anger), Some(BaseEmotion::Anger));
        assert_eq!(plutchik_to_base(PlutchikBasic::Sadness), Some(BaseEmotion::Sadness));
    }

    #[test]
    fn t04_roundtrip_4() {
        // 4 个有对应的情绪应该 roundtrip
        for &base in &[BaseEmotion::Joy, BaseEmotion::Fear, BaseEmotion::Anger, BaseEmotion::Sadness] {
            let plutchik = base_to_plutchik(base);
            let back = plutchik_to_base(plutchik);
            assert_eq!(back, Some(base));
        }
    }

    #[test]
    fn t05_plutchik_advanced_to_base() {
        assert_eq!(plutchik_advanced_to_base(PlutchikAdvanced::Love), Some(BaseEmotion::Joy));
        assert_eq!(plutchik_advanced_to_base(PlutchikAdvanced::Aggressiveness), Some(BaseEmotion::Anger));
    }

    #[test]
    fn t06_pad_center_joy() {
        let pad = plutchik_pad_center(PlutchikBasic::Joy);
        assert!(approx_eq(pad.p, 0.6, 0.01));
    }

    #[test]
    fn t07_pad_distance_zero() {
        let pad = plutchik_pad_center(PlutchikBasic::Joy);
        assert_eq!(pad_distance(pad, pad), 0.0);
    }

    #[test]
    fn t08_closest_base_emotion_joy_pad() {
        // PAD = Joy's center -> 应该是 Joy
        let pad = BaseEmotion::Joy.pad_center();
        assert_eq!(closest_base_emotion(pad), BaseEmotion::Joy);
    }

    #[test]
    fn t09_closest_plutchik_joy_pad() {
        // PAD = Plutchik Joy -> 应该是 Joy
        let pad = plutchik_pad_center(PlutchikBasic::Joy);
        assert_eq!(closest_plutchik_basic(pad), PlutchikBasic::Joy);
    }

    #[test]
    fn t10_closest_plutchik_trust_pad() {
        // PAD = Plutchik Trust -> Trust 应该是最近的
        let pad = plutchik_pad_center(PlutchikBasic::Trust);
        assert_eq!(closest_plutchik_basic(pad), PlutchikBasic::Trust);
    }
}
