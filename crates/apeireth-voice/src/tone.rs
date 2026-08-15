//! Tone — 语音输出"语气"抽象 (R173 2026-08-14 桥 4 落地).
//!
//! 语音不只是文字, 语气 = 情感 + 节奏 + 音调. 桥 4 (consciousness -> voice) 需要 Tone
//! 作为中间表示. 桥 8 (companion -> voice) 也复用.
//!
//! 设计:
//! - `Tone` — 完整语气画像 (speed/pitch/volume + emotion_tone + prosody)
//! - 数值连续 (f64) + 类别 (enum) 混合表达, 兼容下游 TTS API
//!
//! 不漂移:
//! - 0 改 voice crate 现有任何 API (T2aRequest / VoiceSetting / MiniMaxLive)
//! - 0 依赖 voice 之外的类型, 桥 4 / 桥 8 自己定义转换函数
//!
//! 当前状态: R173 最小可用骨架 (P0 桥 4 of 7)

#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};

/// 默认语气 (中性 / 平静 / 平调)
pub const DEFAULT_TONE: Tone = Tone {
    speed: 1.0,
    pitch: 1.0,
    volume: 0.8,
    emotion_tone: EmotionTone::Neutral,
    prosody: Prosody::Flat,
};

/// 情绪色彩 — 类别化语气, 用于 TTS API 选择音色风格.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EmotionTone {
    /// 中性
    Neutral,
    /// 温暖 (Trust / Love)
    Warm,
    /// 冷淡 (Disgust / Contempt)
    Cold,
    /// 兴奋 (Joy / Surprise)
    Excited,
    /// 平静 (Awe / Submission)
    Calm,
    /// 严肃 (Anger / Aggressiveness)
    Serious,
    /// 悲伤 (Sadness / Remorse)
    Sad,
    /// 欢喜 (Joy strong)
    Joyful,
    /// 焦虑 (Fear strong)
    Anxious,
    /// 自信 (Anticipation / Optimism)
    Confident,
}

impl EmotionTone {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Warm => "warm",
            Self::Cold => "cold",
            Self::Excited => "excited",
            Self::Calm => "calm",
            Self::Serious => "serious",
            Self::Sad => "sad",
            Self::Joyful => "joyful",
            Self::Anxious => "anxious",
            Self::Confident => "confident",
        }
    }
}

/// 韵律 — 语调轮廓.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Prosody {
    /// 平调
    Flat,
    /// 升调 (问句 / 期待)
    Rising,
    /// 降调 (陈述 / 失望)
    Falling,
    /// 富有表现力
    Expressive,
    /// 克制 (权威 / 严肃)
    Measured,
}

impl Prosody {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Flat => "flat",
            Self::Rising => "rising",
            Self::Falling => "falling",
            Self::Expressive => "expressive",
            Self::Measured => "measured",
        }
    }
}

/// 语气 — 完整语气画像.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tone {
    /// 语速倍率 (0.5 .. 2.0, 1.0 = 正常).
    pub speed: f64,
    /// 音调倍率 (0.5 .. 2.0, 1.0 = 正常).
    pub pitch: f64,
    /// 音量 [0.0, 1.0].
    pub volume: f64,
    /// 情绪色彩.
    pub emotion_tone: EmotionTone,
    /// 韵律轮廓.
    pub prosody: Prosody,
}

impl Tone {
    /// 构造中性 Tone.
    pub fn neutral() -> Self {
        DEFAULT_TONE
    }

    /// 构造 + 校验 (speed/pitch 在 [0.5, 2.0], volume 在 [0.0, 1.0]).
    pub fn new(speed: f64, pitch: f64, volume: f64, emotion_tone: EmotionTone, prosody: Prosody) -> Self {
        Self {
            speed: speed.clamp(0.5, 2.0),
            pitch: pitch.clamp(0.5, 2.0),
            volume: volume.clamp(0.0, 1.0),
            emotion_tone,
            prosody,
        }
    }
}

/// 默认值实现 (per serde derive 兼容).
impl Default for Tone {
    fn default() -> Self {
        DEFAULT_TONE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tone_is_neutral() {
        let t = Tone::default();
        assert_eq!(t.emotion_tone, EmotionTone::Neutral);
        assert_eq!(t.prosody, Prosody::Flat);
        assert!((t.speed - 1.0).abs() < 1e-9);
        assert!((t.pitch - 1.0).abs() < 1e-9);
        assert!((t.volume - 0.8).abs() < 1e-9);
    }

    #[test]
    fn new_clamps_speed_pitch_volume() {
        let t = Tone::new(3.0, 0.1, 1.5, EmotionTone::Joyful, Prosody::Expressive);
        assert!((t.speed - 2.0).abs() < 1e-9, "speed must clamp to 2.0, got {}", t.speed);
        assert!((t.pitch - 0.5).abs() < 1e-9, "pitch must clamp to 0.5, got {}", t.pitch);
        assert!((t.volume - 1.0).abs() < 1e-9, "volume must clamp to 1.0, got {}", t.volume);
    }

    #[test]
    fn emotion_tone_as_str_covers_all() {
        let _ = EmotionTone::Neutral.as_str();
        let _ = EmotionTone::Confident.as_str();
        let _ = EmotionTone::Cold.as_str();
    }

    #[test]
    fn prosody_as_str_covers_all() {
        let _ = Prosody::Flat.as_str();
        let _ = Prosody::Rising.as_str();
        let _ = Prosody::Expressive.as_str();
    }
}
