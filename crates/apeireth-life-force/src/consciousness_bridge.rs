//! bridge 2: consciousness -> life-force (R173 2026-08-14)
//!
//! 目标: apeireth-consciousness::PlutchikEmotion -> apeireth-life-force::LifeForce (持续力调整 + 反思触发建议).
//!
//! 情感不是孤立事件, 它会消耗或恢复生命力. 桥 2 把 Plutchik 情感翻译为 LifeForceAdjustment:
//! - endurance_delta: 持续力调整幅度, 范围 [-0.2, +0.2]
//! - should_trigger_reflection: 是否建议触发反思期
//! - reflection_reason: 触发原因 (用于反思日志)
//!
//! 不漂移:
//! - 0 改 apeireth-consciousness 任何已实装类型 (不派生 PartialOrd, 用本地 rank)
//! - 0 改 apeireth-life-force 任何已实装类型 (复用 validate_endurance, reflection_trigger)
//! - 0 副作用: translate 是纯函数; apply 只做范围校验 + (条件) 反思触发
//!
//! 当前状态: R173 最小可用落地 (P0 桥 2 of 7)

#![deny(unsafe_code)]

use apeireth_consciousness::plutchik::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};
use crate::{reflection_trigger, validate_endurance, LifeForce, LifeForceError, ReflectionTrigger};

// ============================================
// 1. 翻译结果 — LifeForceAdjustment
// ============================================

/// 生命力调整建议 — consciousness -> life-force 的翻译结果.
///
/// 字段:
/// - `endurance_delta`: 持续力调整幅度, 范围 [-0.2, +0.2] (per-emotion 带下)
/// - `should_trigger_reflection`: 是否建议触发反思期
/// - `reflection_reason`: 触发原因 (`None` = 不触发)
///
/// per 你you 哲学杂谈: 情感不是孤立事件, 它会消耗或恢复续航; 高强度负面情感
/// 应当触发反思期 (per M1 异常行为自动回流).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeForceAdjustment {
    /// 持续力调整幅度 (per-emotion, clamped [-0.2, +0.2]).
    pub endurance_delta: f64,
    /// 是否建议触发反思期.
    pub should_trigger_reflection: bool,
    /// 触发反思期的原因 (`None` = 不触发).
    pub reflection_reason: Option<&'static str>,
}

// ============================================
// 2. 内部辅助 — 强度权 + 基线 + 触发条件
// ============================================

/// 强度映射 (与桥 5 保持一致).
fn intensity_weight(intensity: PlutchikIntensity) -> f64 {
    match intensity {
        PlutchikIntensity::Mild => 0.25,
        PlutchikIntensity::Moderate => 0.5,
        PlutchikIntensity::Strong => 0.75,
        PlutchikIntensity::Extreme => 1.0,
    }
}

/// 强度等级 (0..3). 本地实现 — 不依赖 `PlutchikIntensity: PartialOrd`
/// (per 不漂移: 不改 consciousness crate 派生).
fn intensity_rank(i: PlutchikIntensity) -> u8 {
    match i {
        PlutchikIntensity::Mild => 0,
        PlutchikIntensity::Moderate => 1,
        PlutchikIntensity::Strong => 2,
        PlutchikIntensity::Extreme => 3,
    }
}

/// 情感基线 delta (per-emotion, 待强度相乘).
///
/// 设计:
/// - 正面情感 → 恢复续航 (正 delta)
/// - 负面情感 → 消耗续航 (负 delta)
/// - 高级情感 (Dyads) → 按主轴定基线 (与基础情感保持连续)
fn base_delta(e: &PlutchikEmotion) -> f64 {
    match e {
        // 正面 — 恢复续航
        PlutchikEmotion::Basic(PlutchikBasic::Joy, _) => 0.15,
        PlutchikEmotion::Basic(PlutchikBasic::Trust, _) => 0.10,
        PlutchikEmotion::Basic(PlutchikBasic::Anticipation, _) => 0.08,
        PlutchikEmotion::Basic(PlutchikBasic::Surprise, _) => 0.05,
        // 负面 — 消耗续航
        PlutchikEmotion::Basic(PlutchikBasic::Sadness, _) => -0.15,
        PlutchikEmotion::Basic(PlutchikBasic::Fear, _) => -0.12,
        PlutchikEmotion::Basic(PlutchikBasic::Anger, _) => -0.10,
        PlutchikEmotion::Basic(PlutchikBasic::Disgust, _) => -0.08,
        // 高级 — 复合情感, 按主轴定基线 (与基础情感保持连续)
        PlutchikEmotion::Advanced(PlutchikAdvanced::Optimism, _) => 0.15,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Love, _) => 0.15,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Awe, _) => -0.05,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Submission, _) => -0.05,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Disapproval, _) => -0.10,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Remorse, _) => -0.15,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Contempt, _) => -0.10,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Aggressiveness, _) => -0.10,
    }
}

/// 反思触发条件 — 哪些情感需要"停下来反思" (per M1 异常行为自动回流).
///
/// 不漂移: 只标记"建议触发", 实际触发由 `apply_plutchik_to_life_force` 完成.
fn should_trigger(e: &PlutchikEmotion) -> Option<&'static str> {
    match e {
        // Fear: 中度及以上触发 — 焦虑/恐惧需要停下反思
        PlutchikEmotion::Basic(PlutchikBasic::Fear, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Moderate) =>
        {
            Some("fear-moderate-or-above")
        }
        // Sadness: 强烈及以上触发 — 深度悲伤需要思考
        PlutchikEmotion::Basic(PlutchikBasic::Sadness, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Strong) =>
        {
            Some("sadness-strong-or-above")
        }
        // Anger: 强烈及以上触发 — 愤怒需要冷静
        PlutchikEmotion::Basic(PlutchikBasic::Anger, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Strong) =>
        {
            Some("anger-strong-or-above")
        }
        // Aggressiveness: 极端才触发 — 攻击性是边界状态
        PlutchikEmotion::Advanced(
            PlutchikAdvanced::Aggressiveness,
            PlutchikIntensity::Extreme,
        ) => Some("aggressiveness-extreme"),
        _ => None,
    }
}

// ============================================
// 3. 公共 API — translate (纯) + apply (mutating)
// ============================================

/// 纯翻译: PlutchikEmotion -> LifeForceAdjustment.
/// 0 副作用, 0 改源/目标. 纯函数.
pub fn plutchik_to_life_force_adjustment(e: &PlutchikEmotion) -> LifeForceAdjustment {
    let intensity = intensity_weight(e.intensity());
    let raw = base_delta(e) * intensity;
    // per-emotion 带下: 单次情感最多 ±0.2, 避免单次事件压垮续航
    let delta = raw.clamp(-0.2, 0.2);
    let reason = should_trigger(e);
    LifeForceAdjustment {
        endurance_delta: delta,
        should_trigger_reflection: reason.is_some(),
        reflection_reason: reason,
    }
}

/// 在 LifeForce 上应用 Plutchik 情感 (per 桥 2 入口).
///
/// 入口语义:
/// 1. 计算 endurance_delta, 校验后累加 (复用 `validate_endurance`)
/// 2. 若 `should_trigger_reflection`, 调用 `reflection_trigger` (M1 异常行为自动回流)
///
/// 错误传播: 任何步骤失败 (endurance 越界 / continuity_id 不匹配 / SGI 空) 直接返回.
pub fn apply_plutchik_to_life_force(
    life: &mut LifeForce,
    e: &PlutchikEmotion,
    now: i64,
) -> Result<LifeForceAdjustment, LifeForceError> {
    let adj = plutchik_to_life_force_adjustment(e);
    // 1. 应用 endurance delta (R177 fix: clamp 先于 validate, 避免边界值溢出→ Err)
    //    原始 bug (R176 Kani 发现): Joy 从 1.0 → 1.0375 返 Err, Fear 从 0.0 → -0.03 返 Err
    let raw_endurance = life.endurance + adj.endurance_delta;
    let clamped = raw_endurance.clamp(0.0, 1.0);
    life.endurance = validate_endurance(clamped)?;
    // 2. 若需要, 启动反思期 (M1 异常行为自动回流)
    if adj.should_trigger_reflection {
        let reason = adj.reflection_reason.unwrap_or("plutchik-emotion");
        reflection_trigger(
            life,
            ReflectionTrigger::AnomalyDetected(reason.to_string()),
            now,
        )?;
    }
    Ok(adj)
}

// ============================================
// 4. 单元测试 (8 个, 严守设计清单)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;

    // t01: joy strong -> endurance_delta > 0
    #[test]
    fn t01_joy_strong_yields_positive_delta() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(
            adj.endurance_delta > 0.0,
            "joy strong should yield positive delta, got {}",
            adj.endurance_delta
        );
        assert!(!adj.should_trigger_reflection);
    }

    // t02: sadness strong -> endurance_delta < 0
    #[test]
    fn t02_sadness_strong_yields_negative_delta() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Strong);
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(
            adj.endurance_delta < 0.0,
            "sadness strong should yield negative delta, got {}",
            adj.endurance_delta
        );
    }

    // t03: fear moderate -> should_trigger_reflection
    #[test]
    fn t03_fear_moderate_triggers_reflection() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Fear, PlutchikIntensity::Moderate);
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(adj.should_trigger_reflection);
        assert!(adj.reflection_reason.is_some());
    }

    // t04: sadness mild -> !should_trigger_reflection
    #[test]
    fn t04_sadness_mild_does_not_trigger_reflection() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Mild);
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(!adj.should_trigger_reflection);
        assert!(adj.reflection_reason.is_none());
    }

    // t05: endurance_delta clamped [-0.2, 0.2]
    #[test]
    fn t05_endurance_delta_clamped_to_band() {
        // 8 基础 × 4 强度 + 8 高级 × 4 强度 = 64 组合, 全部落在 [-0.2, 0.2]
        for intensity in PlutchikIntensity::ordered_levels() {
            for basic in PlutchikBasic::ALL {
                let e = PlutchikEmotion::basic(basic, intensity);
                let adj = plutchik_to_life_force_adjustment(&e);
                assert!(
                    adj.endurance_delta >= -0.2,
                    "basic {:?} {:?} delta {} below -0.2",
                    basic,
                    intensity,
                    adj.endurance_delta
                );
                assert!(
                    adj.endurance_delta <= 0.2,
                    "basic {:?} {:?} delta {} above 0.2",
                    basic,
                    intensity,
                    adj.endurance_delta
                );
            }
            for adv in PlutchikAdvanced::ALL {
                let e = PlutchikEmotion::advanced(adv, intensity);
                let adj = plutchik_to_life_force_adjustment(&e);
                assert!(
                    adj.endurance_delta >= -0.2,
                    "adv {:?} {:?} delta {} below -0.2",
                    adv,
                    intensity,
                    adj.endurance_delta
                );
                assert!(
                    adj.endurance_delta <= 0.2,
                    "adv {:?} {:?} delta {} above 0.2",
                    adv,
                    intensity,
                    adj.endurance_delta
                );
            }
        }
    }

    // t06: advanced optimism -> endurance_delta > 0
    #[test]
    fn t06_advanced_optimism_yields_positive_delta() {
        let e = PlutchikEmotion::advanced(PlutchikAdvanced::Optimism, PlutchikIntensity::Strong);
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(
            adj.endurance_delta > 0.0,
            "advanced optimism should yield positive delta, got {}",
            adj.endurance_delta
        );
    }

    // t07: advanced aggressiveness extreme -> should_trigger_reflection
    #[test]
    fn t07_advanced_aggressiveness_extreme_triggers_reflection() {
        let e = PlutchikEmotion::advanced(
            PlutchikAdvanced::Aggressiveness,
            PlutchikIntensity::Extreme,
        );
        let adj = plutchik_to_life_force_adjustment(&e);
        assert!(
            adj.should_trigger_reflection,
            "aggressiveness extreme should trigger reflection"
        );
        assert!(adj.endurance_delta < 0.0);
    }

    // t08: intensity scale (mild < extreme)
    #[test]
    fn t08_intensity_scales_endurance_delta() {
        let mild = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Mild);
        let extreme = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let m = plutchik_to_life_force_adjustment(&mild);
        let e = plutchik_to_life_force_adjustment(&extreme);
        assert!(
            e.endurance_delta > m.endurance_delta,
            "extreme ({}) should yield greater delta than mild ({})",
            e.endurance_delta,
            m.endurance_delta
        );
        assert!(m.endurance_delta > 0.0);
        assert!(e.endurance_delta > 0.0);
    }
}
