//! Plutchik 情感分类 (8 基础 + 8 高级 + 4 强度等级).
//!
//! apeireth-life-force 桥 2 调用本模块的 types. Stub 保最小可编译表面.

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 8 基础 Plutchik 情感 (per Plutchik 1980 wheel of emotions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlutchikBasic {
    /// 喜悦
    Joy,
    /// 信任
    Trust,
    /// 恐惧
    Fear,
    /// 惊讶
    Surprise,
    /// 悲伤
    Sadness,
    /// 厌恶
    Disgust,
    /// 愤怒
    Anger,
    /// 期待
    Anticipation,
}

impl PlutchikBasic {
    /// 8 基础情感全部 (常量数组, 顺序固定).
    pub const ALL: [PlutchikBasic; 8] = [
        PlutchikBasic::Joy,
        PlutchikBasic::Trust,
        PlutchikBasic::Fear,
        PlutchikBasic::Surprise,
        PlutchikBasic::Sadness,
        PlutchikBasic::Disgust,
        PlutchikBasic::Anger,
        PlutchikBasic::Anticipation,
    ];
}

/// 8 高级 Dyad 情感 (per Plutchik primary + adjacent basic dyads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlutchikAdvanced {
    /// 乐观 (Joy + Anticipation)
    Optimism,
    /// 爱 (Joy + Trust)
    Love,
    /// 敬畏 (Fear + Surprise)
    Awe,
    /// 顺从 (Trust + Fear)
    Submission,
    /// 失望 (Surprise + Sadness)
    Disapproval,
    /// 悔恨 (Sadness + Disgust)
    Remorse,
    /// 轻蔑 (Disgust + Anger)
    Contempt,
    /// 攻击性 (Anger + Anticipation)
    Aggressiveness,
}

impl PlutchikAdvanced {
    /// 8 高级 Dyad 全部 (常量数组, 顺序固定).
    pub const ALL: [PlutchikAdvanced; 8] = [
        PlutchikAdvanced::Optimism,
        PlutchikAdvanced::Love,
        PlutchikAdvanced::Awe,
        PlutchikAdvanced::Submission,
        PlutchikAdvanced::Disapproval,
        PlutchikAdvanced::Remorse,
        PlutchikAdvanced::Contempt,
        PlutchikAdvanced::Aggressiveness,
    ];
}

/// 4 强度等级 (Mild / Moderate / Strong / Extreme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PlutchikIntensity {
    /// Mild
    Mild,
    /// Moderate
    Moderate,
    /// Strong
    Strong,
    /// Extreme
    Extreme,
}

impl PlutchikIntensity {
    /// 4 强度等级有序数组 (Mild → Extreme).
    pub const ORDERED: [PlutchikIntensity; 4] = [
        PlutchikIntensity::Mild,
        PlutchikIntensity::Moderate,
        PlutchikIntensity::Strong,
        PlutchikIntensity::Extreme,
    ];

    /// 4 强度等级 (别名 — 与 ORDERED 等价, 适配 ordered_levels() 调用方).
    pub fn ordered_levels() -> [PlutchikIntensity; 4] {
        Self::ORDERED
    }
}

/// Plutchik 情感 = 基础或高级 + 强度.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlutchikEmotion {
    /// 基础情感
    Basic(PlutchikBasic, PlutchikIntensity),
    /// 高级情感 (Dyad)
    Advanced(PlutchikAdvanced, PlutchikIntensity),
}

impl PlutchikEmotion {
    /// 构造基础情感.
    pub fn basic(basic: PlutchikBasic, intensity: PlutchikIntensity) -> Self {
        PlutchikEmotion::Basic(basic, intensity)
    }

    /// 构造高级情感.
    pub fn advanced(advanced: PlutchikAdvanced, intensity: PlutchikIntensity) -> Self {
        PlutchikEmotion::Advanced(advanced, intensity)
    }

    /// 取强度.
    pub fn intensity(&self) -> PlutchikIntensity {
        match self {
            PlutchikEmotion::Basic(_, i) => *i,
            PlutchikEmotion::Advanced(_, i) => *i,
        }
    }
}
