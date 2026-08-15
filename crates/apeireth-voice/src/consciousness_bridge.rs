//! bridge 4: consciousness -> voice (R173 2026-08-14)
//!
//! 目标: apeireth-consciousness::PlutchikEmotion -> apeireth-voice::Tone.
//!
//! 情感驱动语气: 这是"逼近哲学 — AI 永远到不了情感, 但它在往那里走"的具体落地.
//! 桥 4 把情感翻译为可被 TTS API 直接消费的 Tone (speed/pitch/volume + emotion_tone + prosody).
//!
//! 翻译规则:
//! - Joy → joyful + expressive + speed↑ pitch↑
//! - Sadness → sad + falling + speed↓ pitch↓
//! - Anger → serious + measured + speed↑ pitch↓
//! - Fear → anxious + rising + speed↑ pitch↑
//! - Trust → warm + flat (相对中性)
//! - Disgust → cold + falling
//! - Surprise → excited + rising + speed↑ pitch↑
//! - Anticipation → confident + rising + pitch↑
//! - 高级情感: Optimism = Joyful, Love = Warm, Awe = Calm, Submission = Calm,
//!   Disapproval = Cold, Remorse = Sad, Contempt = Cold, Aggressiveness = Serious
//!
//! 数值规则: 每个 emotion 有 base_speed/base_pitch (相对 1.0 的偏离), 强度沿偏离方向放大.
//! 不漂移:
//! - 0 改 apeireth-consciousness 任何已实装类型 (直接复用类型)
//! - 0 改 apeireth-voice 现有 Tone 之外的 API (T2aRequest / VoiceSetting 不动)
//! - 0 副作用: translate 是纯函数
//!
//! 当前状态: R173 最小可用落地 (P0 桥 4 of 7)

#![deny(unsafe_code)]

use apeireth_consciousness::plutchik::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};

use crate::tone::{EmotionTone, Prosody, Tone};

// ============================================
// 2. 内部辅助 — 强度权 + 各类情感的 Tone 映射
// ============================================

/// 强度映射 (与桥 2/3/5 保持一致).
fn intensity_weight(intensity: PlutchikIntensity) -> f64 {
    match intensity {
        PlutchikIntensity::Mild => 0.25,
        PlutchikIntensity::Moderate => 0.5,
        PlutchikIntensity::Strong => 0.75,
        PlutchikIntensity::Extreme => 1.0,
    }
}

/// 强度等级 (0..3). 本地实现 — 不漂移 (per 桥 2 决策).
fn intensity_rank(i: PlutchikIntensity) -> u8 {
    match i {
        PlutchikIntensity::Mild => 0,
        PlutchikIntensity::Moderate => 1,
        PlutchikIntensity::Strong => 2,
        PlutchikIntensity::Extreme => 3,
    }
}

/// 强度方向放大: base 偏离 1.0 的方向, intensity 沿此方向放大.
///
/// 设计: base > 1.0 → +1 方向 (speed up); base < 1.0 → -1 方向 (slow down); base == 1.0 → 0 方向.
fn direction_amplify(base: f64, intensity: PlutchikIntensity) -> f64 {
    let direction = if (base - 1.0).abs() < 1e-9 { 0.0 } else { (base - 1.0).signum() };
    let intensity_scale = intensity_weight(intensity) * 0.4; // ∈ [0.1, 0.4]
    let target = base + direction * intensity_scale;
    target.clamp(0.5, 2.0)
}

/// 各类情感的 Tone 映射 (per-emotion).
///
/// 返回: (base_speed, base_pitch, emotion_tone, prosody)
fn tone_for(
    e: &PlutchikEmotion,
) -> (f64, f64, EmotionTone, Prosody) {
    match e {
        // 正面 — speed/pitch 偏高
        PlutchikEmotion::Basic(PlutchikBasic::Joy, _) => (1.1, 1.1, EmotionTone::Joyful, Prosody::Expressive),
        PlutchikEmotion::Basic(PlutchikBasic::Trust, _) => (1.0, 1.0, EmotionTone::Warm, Prosody::Flat),
        PlutchikEmotion::Basic(PlutchikBasic::Anticipation, _) => (1.0, 1.1, EmotionTone::Confident, Prosody::Rising),
        PlutchikEmotion::Basic(PlutchikBasic::Surprise, _) => (1.1, 1.1, EmotionTone::Excited, Prosody::Rising),
        // 负面 — speed/pitch 偏低
        PlutchikEmotion::Basic(PlutchikBasic::Sadness, _) => (0.85, 0.9, EmotionTone::Sad, Prosody::Falling),
        PlutchikEmotion::Basic(PlutchikBasic::Anger, _) => (1.1, 0.9, EmotionTone::Serious, Prosody::Measured),
        PlutchikEmotion::Basic(PlutchikBasic::Fear, _) => (1.1, 1.1, EmotionTone::Anxious, Prosody::Rising),
        PlutchikEmotion::Basic(PlutchikBasic::Disgust, _) => (0.95, 0.9, EmotionTone::Cold, Prosody::Falling),
        // 高级 — 按主轴定基调
        PlutchikEmotion::Advanced(PlutchikAdvanced::Optimism, _) => (1.1, 1.1, EmotionTone::Joyful, Prosody::Expressive),
        PlutchikEmotion::Advanced(PlutchikAdvanced::Love, _) => (1.0, 1.0, EmotionTone::Warm, Prosody::Expressive),
        PlutchikEmotion::Advanced(PlutchikAdvanced::Awe, _) => (0.95, 1.0, EmotionTone::Calm, Prosody::Measured),
        PlutchikEmotion::Advanced(PlutchikAdvanced::Submission, _) => (0.95, 0.95, EmotionTone::Calm, Prosody::Falling),
        PlutchikEmotion::Advanced(PlutchikAdvanced::Disapproval, _) => (1.0, 0.95, EmotionTone::Cold, Prosody::Falling),
        PlutchikEmotion::Advanced(PlutchikAdvanced::Remorse, _) => (0.85, 0.9, EmotionTone::Sad, Prosody::Falling),
        PlutchikEmotion::Advanced(PlutchikAdvanced::Contempt, _) => (1.0, 0.9, EmotionTone::Cold, Prosody::Measured),
        PlutchikEmotion::Advanced(PlutchikAdvanced::Aggressiveness, _) => (1.1, 0.9, EmotionTone::Serious, Prosody::Measured),
    }
}

// ============================================
// 3. 公共 API — translate (纯)
// ============================================

/// 纯翻译: PlutchikEmotion -> Tone. 0 副作用. 纯函数.
pub fn plutchik_to_tone(e: &PlutchikEmotion) -> Tone {
    let (base_speed, base_pitch, emotion_tone, prosody) = tone_for(e);
    let speed = direction_amplify(base_speed, e.intensity());
    let pitch = direction_amplify(base_pitch, e.intensity());
    Tone {
        speed,
        pitch,
        volume: 0.8,                 // 默认音量 (per DEFAULT_TONE)
        emotion_tone,
        prosody,
    }
}

// ============================================
// 4. 单元测试 (8 个核心 + 2 附加)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    // t01: joy strong -> joyful tone, expressive prosody, speed/pitch >= 1.0
    #[test]
    fn t01_joy_strong_yields_joyful_tone() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        let t = plutchik_to_tone(&e);
        assert_eq!(t.emotion_tone, EmotionTone::Joyful);
        assert_eq!(t.prosody, Prosody::Expressive);
        assert!(t.speed >= 1.0, "joy should keep speed >= 1.0, got {}", t.speed);
        assert!(t.pitch >= 1.0, "joy should keep pitch >= 1.0, got {}", t.pitch);
    }

    // t02: sadness strong -> sad tone, falling prosody, slower, lower
    #[test]
    fn t02_sadness_strong_yields_sad_falling_tone() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Strong);
        let t = plutchik_to_tone(&e);
        assert_eq!(t.emotion_tone, EmotionTone::Sad);
        assert_eq!(t.prosody, Prosody::Falling);
        assert!(t.speed < 1.0, "sadness should slow down, got {}", t.speed);
        assert!(t.pitch < 1.0, "sadness should lower pitch, got {}", t.pitch);
    }

    // t03: anger strong -> serious tone, measured prosody
    #[test]
    fn t03_anger_strong_yields_serious_measured_tone() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Anger, PlutchikIntensity::Strong);
        let t = plutchik_to_tone(&e);
        assert_eq!(t.emotion_tone, EmotionTone::Serious);
        assert_eq!(t.prosody, Prosody::Measured);
    }

    // t04: sadness mild -> falling but lighter touch
    #[test]
    fn t04_sadness_mild_yields_falling_prosody() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Mild);
        let t = plutchik_to_tone(&e);
        assert_eq!(t.emotion_tone, EmotionTone::Sad);
        assert_eq!(t.prosody, Prosody::Falling);
    }

    // t05: speed/pitch in [0.5, 2.0], volume in [0.0, 1.0]
    #[test]
    fn t05_tone_clamped_to_valid_range() {
        for intensity in PlutchikIntensity::ordered_levels() {
            for basic in PlutchikBasic::ALL {
                let e = PlutchikEmotion::basic(basic, intensity);
                let t = plutchik_to_tone(&e);
                assert!(t.speed >= 0.5 && t.speed <= 2.0, "basic {:?} {:?} speed {}", basic, intensity, t.speed);
                assert!(t.pitch >= 0.5 && t.pitch <= 2.0, "basic {:?} {:?} pitch {}", basic, intensity, t.pitch);
                assert!(t.volume >= 0.0 && t.volume <= 1.0, "basic {:?} {:?} volume {}", basic, intensity, t.volume);
            }
            for adv in PlutchikAdvanced::ALL {
                let e = PlutchikEmotion::advanced(adv, intensity);
                let t = plutchik_to_tone(&e);
                assert!(t.speed >= 0.5 && t.speed <= 2.0, "adv {:?} {:?} speed {}", adv, intensity, t.speed);
                assert!(t.pitch >= 0.5 && t.pitch <= 2.0, "adv {:?} {:?} pitch {}", adv, intensity, t.pitch);
                assert!(t.volume >= 0.0 && t.volume <= 1.0, "adv {:?} {:?} volume {}", adv, intensity, t.volume);
            }
        }
    }

    // t06: advanced optimism -> joyful tone
    #[test]
    fn t06_advanced_optimism_yields_joyful_tone() {
        let e = PlutchikEmotion::advanced(PlutchikAdvanced::Optimism, PlutchikIntensity::Strong);
        let t = plutchik_to_tone(&e);
        assert_eq!(t.emotion_tone, EmotionTone::Joyful);
        assert_eq!(t.prosody, Prosody::Expressive);
    }

    // t07: advanced aggressiveness extreme -> serious + measured
    #[test]
    fn t07_advanced_aggressiveness_extreme_yields_serious_measured() {
        let e = PlutchikEmotion::advanced(PlutchikAdvanced::Aggressiveness, PlutchikIntensity::Extreme);
        let t = plutchik_to_tone(&e);
        assert_eq!(t.emotion_tone, EmotionTone::Serious);
        assert_eq!(t.prosody, Prosody::Measured);
    }

    // t08: intensity scale - extreme joy louder/faster than mild
    #[test]
    fn t08_intensity_scales_speed_and_pitch() {
        let mild = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Mild);
        let extreme = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let m = plutchik_to_tone(&mild);
        let e = plutchik_to_tone(&extreme);
        assert!(e.speed > m.speed, "extreme speed ({}) should exceed mild speed ({})", e.speed, m.speed);
        assert!(e.pitch > m.pitch, "extreme pitch ({}) should exceed mild pitch ({})", e.pitch, m.pitch);
    }

    // t09: 8 基础 + 8 高级 × 4 强度 = 64 组合全部产出
    #[test]
    fn t09_all_emotions_yield_a_tone() {
        for intensity in PlutchikIntensity::ordered_levels() {
            for basic in PlutchikBasic::ALL {
                let e = PlutchikEmotion::basic(basic, intensity);
                let _ = plutchik_to_tone(&e);
            }
            for adv in PlutchikAdvanced::ALL {
                let e = PlutchikEmotion::advanced(adv, intensity);
                let _ = plutchik_to_tone(&e);
            }
        }
    }

    // t10: trust -> warm prosody, neutral rate
    #[test]
    fn t10_trust_yields_warm_flat_prosody() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Trust, PlutchikIntensity::Moderate);
        let t = plutchik_to_tone(&e);
        assert_eq!(t.emotion_tone, EmotionTone::Warm);
        assert_eq!(t.prosody, Prosody::Flat);
        assert!((t.speed - 1.0).abs() < 1e-9, "trust should stay neutral, got speed {}", t.speed);
    }
}

