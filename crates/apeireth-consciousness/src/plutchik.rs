//! R218 Plutchik 8 情绪 (R187 cognition 调研 + R192 推荐).
//!
//! **来源**: R187 调研提到 Plutchik 情感轮 (8 基础 + 8 高级 = 16) 是经典情感理论.
//! Robert Plutchik 1980 提出, 与 Ekman 6 情绪并列.
//!
//! **设计**:
//! - Plutchik 8 基础情绪 enum (Joy / Trust / Fear / Surprise / Sadness / Disgust / Anger / Anticipation)
//! - Plutchik 8 高级情绪 enum (Love / Submission / Awe / Disapproval / Remorse / Contempt / Aggressiveness / Optimism)
//! - 强度 (Intensity) 枚举: Mild / Moderate / Strong / Extreme
//! - WheelPos 8 位置: 8 维情绪轮位置 (借鉴 R187 PAD 转换)
//!
//! **0 触碰**: 本模块是 additive, emotion.rs 现有 6 基础情绪 API 0 改.
//!
//! **不假装** (O-5):
//! - 经典 Plutchik 模型, 学术有据 (1980)
//! - 不假装 LLM 真的能识别所有 16 情绪, 仅提供 trait

#![allow(missing_docs)] // R218: 0 触碰现有 API 文档

use serde::{Deserialize, Serialize};

/// Plutchik 8 基础情绪 (经典 Plutchik 情感轮).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlutchikBasic {
    /// 喜 (Joy)
    Joy,
    /// 信任 (Trust)
    Trust,
    /// 恐惧 (Fear)
    Fear,
    /// 惊讶 (Surprise)
    Surprise,
    /// 悲伤 (Sadness)
    Sadness,
    /// 厌恶 (Disgust)
    Disgust,
    /// 愤怒 (Anger)
    Anger,
    /// 期待 (Anticipation)
    Anticipation,
}

impl PlutchikBasic {
    /// 8 维编译期 hardcode
    pub const COUNT: usize = 8;
    pub const ALL: [PlutchikBasic; 8] = [
        Self::Joy, Self::Trust, Self::Fear, Self::Surprise,
        Self::Sadness, Self::Disgust, Self::Anger, Self::Anticipation,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Joy => "joy",
            Self::Trust => "trust",
            Self::Fear => "fear",
            Self::Surprise => "surprise",
            Self::Sadness => "sadness",
            Self::Disgust => "disgust",
            Self::Anger => "anger",
            Self::Anticipation => "anticipation",
        }
    }

    /// 8 维情感轮位置 (按 R187 调研, 顺时针 0..7)
    pub const fn wheel_position(&self) -> u8 {
        match self {
            Self::Joy => 0,
            Self::Trust => 1,
            Self::Fear => 2,
            Self::Surprise => 3,
            Self::Sadness => 4,
            Self::Disgust => 5,
            Self::Anger => 6,
            Self::Anticipation => 7,
        }
    }
}

/// Plutchik 8 高级情绪 (Dyads, 8 基础情绪两两相邻的复合).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlutchikAdvanced {
    /// 爱 (Joy + Trust)
    Love,
    /// 顺从 (Trust + Fear)
    Submission,
    /// 敬畏 (Fear + Surprise)
    Awe,
    /// 不赞同 (Surprise + Sadness)
    Disapproval,
    /// 懊悔 (Sadness + Disgust)
    Remorse,
    /// 轻蔑 (Disgust + Anger)
    Contempt,
    /// 攻击性 (Anger + Anticipation)
    Aggressiveness,
    /// 乐观 (Anticipation + Joy)
    Optimism,
}

impl PlutchikAdvanced {
    pub const COUNT: usize = 8;
    pub const ALL: [PlutchikAdvanced; 8] = [
        Self::Love, Self::Submission, Self::Awe, Self::Disapproval,
        Self::Remorse, Self::Contempt, Self::Aggressiveness, Self::Optimism,
    ];

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Love => "love",
            Self::Submission => "submission",
            Self::Awe => "awe",
            Self::Disapproval => "disapproval",
            Self::Remorse => "remorse",
            Self::Contempt => "contempt",
            Self::Aggressiveness => "aggressiveness",
            Self::Optimism => "optimism",
        }
    }

    /// 从两个基础情绪推导高级情绪 (按 Plutchik 规则)
    /// 相邻 (差 1) 或 wrap (差 7) 才返回高级情绪
    pub fn from_pair(a: PlutchikBasic, b: PlutchikBasic) -> Option<Self> {
        if a == b { return None; }  // 同情绪
        let pos_a = a.wheel_position() as i8;
        let pos_b = b.wheel_position() as i8;
        let diff = (pos_a - pos_b).abs();
        // Plutchik 情感轮是循环的: 0-7 差 1 (相邻) 或 7 (wrap: 7->0)
        if diff != 1 && diff != 7 { return None; }
        // wrap 情况: Anticipation(7) + Joy(0) = Optimism
        if diff == 7 { return Some(Self::Optimism); }
        let start = pos_a.min(pos_b) as u8;
        match start {
            0 => Some(Self::Love),       // Joy(0) + Trust(1)
            1 => Some(Self::Submission), // Trust(1) + Fear(2)
            2 => Some(Self::Awe),        // Fear(2) + Surprise(3)
            3 => Some(Self::Disapproval),// Surprise(3) + Sadness(4)
            4 => Some(Self::Remorse),    // Sadness(4) + Disgust(5)
            5 => Some(Self::Contempt),   // Disgust(5) + Anger(6)
            6 => Some(Self::Aggressiveness), // Anger(6) + Anticipation(7)
            _ => None,
        }
    }
}

/// Plutchik 强度 (4 档: Mild / Moderate / Strong / Extreme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlutchikIntensity {
    Mild,
    Moderate,
    Strong,
    Extreme,
}

impl PlutchikIntensity {
    pub const fn ordered_levels() -> [PlutchikIntensity; 4] {
        [Self::Mild, Self::Moderate, Self::Strong, Self::Extreme]
    }
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Mild => "mild",
            Self::Moderate => "moderate",
            Self::Strong => "strong",
            Self::Extreme => "extreme",
        }
    }
}

/// Plutchik 情绪实例 (基础 + 强度, 或高级 + 强度).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlutchikEmotion {
    Basic(PlutchikBasic, PlutchikIntensity),
    Advanced(PlutchikAdvanced, PlutchikIntensity),
}

impl PlutchikEmotion {
    pub const fn intensity(&self) -> PlutchikIntensity {
        match self {
            Self::Basic(_, i) => *i,
            Self::Advanced(_, i) => *i,
        }
    }
    pub fn basic(emotion: PlutchikBasic, intensity: PlutchikIntensity) -> Self {
        Self::Basic(emotion, intensity)
    }
    pub fn advanced(emotion: PlutchikAdvanced, intensity: PlutchikIntensity) -> Self {
        Self::Advanced(emotion, intensity)
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::Basic(e, _) => e.as_str(),
            Self::Advanced(e, _) => e.as_str(),
        }
    }
}

// 编译期守门: 8 基础 + 8 高级
const _: () = assert!(PlutchikBasic::COUNT == 8);
const _: () = assert!(PlutchikAdvanced::COUNT == 8);
const _: () = assert!(PlutchikBasic::ALL.len() == 8);
const _: () = assert!(PlutchikAdvanced::ALL.len() == 8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t01_basic_count() {
        assert_eq!(PlutchikBasic::COUNT, 8);
        assert_eq!(PlutchikBasic::ALL.len(), 8);
    }

    #[test]
    fn t02_advanced_count() {
        assert_eq!(PlutchikAdvanced::COUNT, 8);
    }

    #[test]
    fn t03_basic_as_str() {
        assert_eq!(PlutchikBasic::Joy.as_str(), "joy");
        assert_eq!(PlutchikBasic::Anticipation.as_str(), "anticipation");
    }

    #[test]
    fn t04_wheel_positions() {
        assert_eq!(PlutchikBasic::Joy.wheel_position(), 0);
        assert_eq!(PlutchikBasic::Anticipation.wheel_position(), 7);
    }

    #[test]
    fn t05_advanced_from_pair_adjacent() {
        // Joy + Trust = Love
        assert_eq!(PlutchikAdvanced::from_pair(PlutchikBasic::Joy, PlutchikBasic::Trust), Some(PlutchikAdvanced::Love));
        // Anger + Anticipation = Aggressiveness
        assert_eq!(PlutchikAdvanced::from_pair(PlutchikBasic::Anger, PlutchikBasic::Anticipation), Some(PlutchikAdvanced::Aggressiveness));
    }

    #[test]
    fn t06_advanced_from_pair_not_adjacent() {
        // Joy + Sadness (差 4) 不相邻
        assert_eq!(PlutchikAdvanced::from_pair(PlutchikBasic::Joy, PlutchikBasic::Sadness), None);
    }

    #[test]
    fn t07_advanced_from_same_emotion() {
        assert_eq!(PlutchikAdvanced::from_pair(PlutchikBasic::Joy, PlutchikBasic::Joy), None);
    }

    #[test]
    fn t08_advanced_from_opposite() {
        // Joy (0) + Sadness (4) 对位
        assert_eq!(PlutchikAdvanced::from_pair(PlutchikBasic::Joy, PlutchikBasic::Sadness), None);
        // Fear (2) + Anger (6) 对位
        assert_eq!(PlutchikAdvanced::from_pair(PlutchikBasic::Fear, PlutchikBasic::Anger), None);
    }

    #[test]
    fn t09_intensity_as_str() {
        assert_eq!(PlutchikIntensity::Mild.as_str(), "mild");
        assert_eq!(PlutchikIntensity::Extreme.as_str(), "extreme");
    }

    #[test]
    fn t10_plutchik_emotion_basic() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        assert_eq!(e.name(), "joy");
    }

    #[test]
    fn t11_plutchik_emotion_advanced() {
        let e = PlutchikEmotion::advanced(PlutchikAdvanced::Love, PlutchikIntensity::Moderate);
        assert_eq!(e.name(), "love");
    }

    #[test]
    fn t12_all_advanced_pairs() {
        // 全部 8 个相邻对都应有高级情绪
        let basics = PlutchikBasic::ALL;
        for window in basics.windows(2) {
            let pair = PlutchikAdvanced::from_pair(window[0], window[1]);
            assert!(pair.is_some(), "pair {:?} + {:?} should produce advanced emotion", window[0], window[1]);
        }
        // 最后一对 (Anticipation -> Joy, wrap)
        let wrap = PlutchikAdvanced::from_pair(PlutchikBasic::Anticipation, PlutchikBasic::Joy);
        assert_eq!(wrap, Some(PlutchikAdvanced::Optimism));
    }
}
