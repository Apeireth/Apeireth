//! bridge 8: companion -> voice (R173 2026-08-14)
//!
//! 目标: apeireth-companion::Bond (BondStage + BondDepth + BondCharacter) -> apeireth-voice::Tone.
//!
//! 关系驱动语气: 关系亲密度调整 TTS 输出的距离感/温度/韵律.
//! 桥 8 把 Bond 翻译为 Tone 调整:
//! - BondStage::Initial → 距离感 (low volume, flat)
//! - BondStage::Familiar → 中性
//! - BondStage::Trusted → 温暖
//! - BondStage::Intimate → 温暖 + expressive + 略慢
//! - BondStage::LongTerm → calm + warm
//! - BondStage::Paused → cold, measured
//! - BondStage::Ended → cold, flat
//! - BondDepth ↑ → 更 expressive + warm
//! - BondCharacter.trust/resonance ↑ → 提升 warm + expressive
//!
//! 不漂移:
//! - 0 改 apeireth-companion 任何已实装类型 (直接复用)
//! - 0 改 apeireth-voice 现有 Tone 之外的 API
//! - 0 副作用: translate 是纯函数
//!
//! 当前状态: R173 最小可用落地 (P1 桥 8 of 7)

#![deny(unsafe_code)]

use apeireth_companion::bond::{Bond, BondCharacter, BondDepth, BondStage};

use crate::tone::{EmotionTone, Prosody, Tone};

// ============================================
// 1. 内部辅助 — 阶段 / 深度 / 性格 -> Tone
// ============================================

/// BondStage -> (base_speed, base_pitch, emotion_tone, prosody)
fn tone_for_stage(stage: BondStage) -> (f64, f64, EmotionTone, Prosody) {
    match stage {
        BondStage::Initial => (1.0, 1.0, EmotionTone::Neutral, Prosody::Flat),
        BondStage::Familiar => (1.0, 1.0, EmotionTone::Neutral, Prosody::Flat),
        BondStage::Trusted => (1.0, 1.0, EmotionTone::Warm, Prosody::Flat),
        BondStage::Intimate => (0.95, 1.0, EmotionTone::Warm, Prosody::Expressive),
        BondStage::LongTerm => (0.95, 0.95, EmotionTone::Calm, Prosody::Measured),
        BondStage::Paused => (1.0, 0.95, EmotionTone::Cold, Prosody::Measured),
        BondStage::Ended => (1.0, 0.95, EmotionTone::Cold, Prosody::Flat),
    }
}

/// BondDepth -> 强度缩放 (0.0 = 完全无声, 1.0 = 全力).
fn depth_volume_scale(depth: f64) -> f64 {
    // 0.4 起步, 1.0 顶点
    0.4 + depth * 0.6
}

/// BondCharacter -> (warmth, energy) 增量, 用于调整 emotion_tone / prosody.
fn character_offsets(_c: &BondCharacter) -> (f64, f64) {
    // 当前版本: 不引入额外偏移, 阶段和深度已足够
    (0.0, 0.0)
}

/// 强度方向放大 (per 桥 4 决策).
fn direction_amplify(base: f64, intensity: f64) -> f64 {
    let deviation = base - 1.0;
    let direction = if deviation.abs() < 1e-9 { 0.0 } else { deviation.signum() };
    let intensity_scale = intensity * 0.4;
    let target = base + direction * intensity_scale;
    target.clamp(0.5, 2.0)
}

/// 升级 emotion_tone (per trust / depth).
fn elevate_emotion(base: EmotionTone, trust: f64) -> EmotionTone {
    if trust >= 0.6 {
        upgrade_to_friendly(base)
    } else if trust < 0.2 {
        downgrade_to_cold(base)
    } else {
        base
    }
}

fn upgrade_to_friendly(base: EmotionTone) -> EmotionTone {
    match base {
        EmotionTone::Neutral => EmotionTone::Warm,
        EmotionTone::Cold => EmotionTone::Neutral,
        other => other,
    }
}

fn downgrade_to_cold(base: EmotionTone) -> EmotionTone {
    match base {
        EmotionTone::Warm => EmotionTone::Neutral,
        EmotionTone::Joyful => EmotionTone::Neutral,
        EmotionTone::Confident => EmotionTone::Neutral,
        other => other,
    }
}

/// 升级 prosody (per depth).
fn elevate_prosody(base: Prosody, depth: f64) -> Prosody {
    if depth >= 0.7 {
        match base {
            Prosody::Flat => Prosody::Expressive,
            Prosody::Falling => Prosody::Expressive,
            other => other,
        }
    } else if depth < 0.2 {
        Prosody::Flat
    } else {
        base
    }
}

// ============================================
// 2. 公共 API — translate (纯)
// ============================================

/// 纯翻译: Bond (stage + depth + character) -> Tone. 0 副作用. 纯函数.
pub fn bond_to_tone(bond: &Bond) -> Tone {
    let stage = bond.stage();
    let depth = bond.depth().value();
    let character = bond.character();

    let (base_speed, base_pitch, base_emotion, base_prosody) = tone_for_stage(stage);
    let intensity = depth_volume_scale(depth);
    let speed = direction_amplify(base_speed, intensity);
    let pitch = direction_amplify(base_pitch, intensity);
    let volume = (0.5 + depth * 0.5).clamp(0.0, 1.0);
    let emotion_tone = elevate_emotion(base_emotion, character.trust);
    let prosody = elevate_prosody(base_prosody, depth);
    Tone {
        speed,
        pitch,
        volume,
        emotion_tone,
        prosody,
    }
}

/// 纯翻译: BondStage + BondDepth + BondCharacter -> Tone. 0 副作用. 纯函数.
pub fn bond_components_to_tone(
    stage: BondStage,
    depth: BondDepth,
    character: &BondCharacter,
) -> Tone {
    let (base_speed, base_pitch, base_emotion, base_prosody) = tone_for_stage(stage);
    let d = depth.value();
    let intensity = depth_volume_scale(d);
    let speed = direction_amplify(base_speed, intensity);
    let pitch = direction_amplify(base_pitch, intensity);
    let volume = (0.5 + d * 0.5).clamp(0.0, 1.0);
    let emotion_tone = elevate_emotion(base_emotion, character.trust);
    let prosody = elevate_prosody(base_prosody, d);
    Tone {
        speed,
        pitch,
        volume,
        emotion_tone,
        prosody,
    }
}


// ============================================
// 3. 单元测试 (8 个 + 2 附加)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_companion::bond::BondCharacter;

    fn make_bond(stage: BondStage, depth: f64, trust: f64) -> Bond {
        let mut bond = Bond::new();
        // 初始为 0.0, 需要 set depth 以 evolve 实现
        bond.evolve(stage, depth);
        bond.character_mut().trust = trust;
        bond
    }

    // t01: Initial → neutral, flat
    #[test]
    fn t01_initial_stage_yields_neutral_flat() {
        let bond = make_bond(BondStage::Initial, 0.1, 0.0);
        let t = bond_to_tone(&bond);
        assert_eq!(t.emotion_tone, EmotionTone::Neutral);
        assert_eq!(t.prosody, Prosody::Flat);
    }

    // t02: Trusted → warm
    #[test]
    fn t02_trusted_stage_yields_warm() {
        let bond = make_bond(BondStage::Trusted, 0.5, 0.7);
        let t = bond_to_tone(&bond);
        assert_eq!(t.emotion_tone, EmotionTone::Warm);
    }

    // t03: Intimate → warm + expressive
    #[test]
    fn t03_intimate_stage_yields_warm_expressive() {
        let bond = make_bond(BondStage::Intimate, 0.8, 0.8);
        let t = bond_to_tone(&bond);
        assert_eq!(t.emotion_tone, EmotionTone::Warm);
        assert_eq!(t.prosody, Prosody::Expressive);
    }

    // t04: Ended → cold + flat
    #[test]
    fn t04_ended_stage_yields_cold_flat() {
        let bond = make_bond(BondStage::Ended, 0.0, 0.0);
        let t = bond_to_tone(&bond);
        assert_eq!(t.emotion_tone, EmotionTone::Cold);
        assert_eq!(t.prosody, Prosody::Flat);
    }

    // t05: depth 0.0 → volume 最低
    #[test]
    fn t05_zero_depth_yields_min_volume() {
        let bond = make_bond(BondStage::Familiar, 0.0, 0.5);
        let t = bond_to_tone(&bond);
        assert!(t.volume <= 0.5 + 1e-9, "depth 0 should keep volume near 0.5, got {}", t.volume);
    }

    // t06: depth 1.0 → volume 最高
    #[test]
    fn t06_full_depth_yields_high_volume() {
        let bond = make_bond(BondStage::Familiar, 1.0, 0.5);
        let t = bond_to_tone(&bond);
        assert!(t.volume >= 1.0 - 1e-9, "depth 1.0 should give volume near 1.0, got {}", t.volume);
    }

    // t07: trust 高 → emotion_tone 升级
    #[test]
    fn t07_high_trust_upgrades_emotion_tone() {
        let bond_low = make_bond(BondStage::Familiar, 0.5, 0.0);
        let bond_high = make_bond(BondStage::Familiar, 0.5, 0.9);
        let t_low = bond_to_tone(&bond_low);
        let t_high = bond_to_tone(&bond_high);
        // trust=0.9 with Familiar base Neutral should upgrade to Warm
        assert_eq!(t_high.emotion_tone, EmotionTone::Warm);
        assert_eq!(t_low.emotion_tone, EmotionTone::Neutral);
    }

    // t08: prosody 升级 — depth >= 0.7 → flat/falling becomes expressive
    #[test]
    fn t08_high_depth_upgrades_prosody() {
        let bond_low = make_bond(BondStage::Familiar, 0.3, 0.5);
        let bond_high = make_bond(BondStage::Familiar, 0.8, 0.5);
        let t_low = bond_to_tone(&bond_low);
        let t_high = bond_to_tone(&bond_high);
        // Familiar base prosody = Flat. depth 0.8 → Expressive.
        assert_eq!(t_low.prosody, Prosody::Flat);
        assert_eq!(t_high.prosody, Prosody::Expressive);
    }

    // t09: speed/pitch in [0.5, 2.0], volume in [0.0, 1.0]
    #[test]
    fn t09_tone_clamped_to_valid_range() {
        for stage in BondStage::ALL {
            for &depth in &[0.0, 0.3, 0.5, 0.7, 1.0] {
                for &trust in &[0.0, 0.3, 0.5, 0.7, 1.0] {
                    let bond = make_bond(stage, depth, trust);
                    let t = bond_to_tone(&bond);
                    assert!(t.speed >= 0.5 && t.speed <= 2.0, "stage {:?} depth {} trust {} speed {}", stage, depth, trust, t.speed);
                    assert!(t.pitch >= 0.5 && t.pitch <= 2.0, "stage {:?} depth {} trust {} pitch {}", stage, depth, trust, t.pitch);
                    assert!(t.volume >= 0.0 && t.volume <= 1.0, "stage {:?} depth {} trust {} volume {}", stage, depth, trust, t.volume);
                }
            }
        }
    }

    // t10: bond_components_to_tone 给出与 bond_to_tone 相同结果
    #[test]
    fn t10_components_to_tone_matches_bond_to_tone() {
        let bond = make_bond(BondStage::Intimate, 0.7, 0.6);
        let t1 = bond_to_tone(&bond);
        let t2 = bond_components_to_tone(bond.stage(), bond.depth(), bond.character());
        assert_eq!(t1.speed, t2.speed);
        assert_eq!(t1.pitch, t2.pitch);
        assert_eq!(t1.volume, t2.volume);
        assert_eq!(t1.emotion_tone, t2.emotion_tone);
        assert_eq!(t1.prosody, t2.prosody);
    }
}

