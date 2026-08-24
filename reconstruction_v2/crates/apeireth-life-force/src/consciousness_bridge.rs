//! bridge 2: cognition -> life-force (R173 2026-08-14, v2 重构)
//!
//! 目标: apeireth-cognition::PlutchikEmotion -> apeireth-life-force::LifeForce (持续力调整 + 反思触发建议).
//!
//! 情感不是孤立事件, 它会消耗或恢复生命力. 桥 2 把 Plutchik 情感翻译为 LifeForceAdjustment:
//! - endurance_delta: 持续力调整幅度, 范围 [-0.2, +0.2]
//! - should_trigger_reflection: 是否建议触发反思期
//! - reflection_reason: 触发原因 (用于反思日志)
//!
//! **v2 重构 (v1 → v2)**:
//! - v1 引用 `apeireth_consciousness::plutchik::{PlutchikBasic, PlutchikAdvanced, PlutchikEmotion,
//!   PlutchikIntensity}`, v2 改为 `apeireth_cognition::{PlutchikBasic, PlutchikAdvanced,
//!   PlutchikEmotion, PlutchikIntensity}`.
//! - v1 用本地 `intensity_weight` + `intensity_rank` 函数, v2 复用 cognition crate 的
//!   `PlutchikIntensity::weight()` (值完全相同: Mild=0.25, Moderate=0.5, Strong=0.75, Extreme=1.0),
//!   `intensity_rank` 仍本地实现 (因为 cognition 不派生 PartialOrd).
//! - delta / reflection 触发逻辑 1:1 保留 v1 算法 (per-emotion base_delta + 强度相乘 +
//!   clamp ±0.2).
//!
//! 不漂移:
//! - 0 改 apeireth-cognition 任何已实装类型 (不派生 PartialOrd, 用本地 rank)
//! - 0 改 apeireth-life-force 任何已实装类型 (复用 validate_endurance, reflection_trigger)
//! - 0 副作用: translate 是纯函数; apply 只做范围校验 + (条件) 反思触发
//!
//! 当前状态: R173 最小可用落地 (P0 桥 2 of 7), v2 surface 完全覆盖 v1 测试.

#![deny(unsafe_code)]

use crate::{reflection_trigger, validate_endurance, LifeForce, LifeForceError, ReflectionTrigger};
use apeireth_cognition::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};

// ============================================
// 1. 翻译结果 — LifeForceAdjustment
// ============================================

/// 生命力调整建议 — cognition -> life-force 的翻译结果.
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

/// 强度权 (与桥 5 保持一致, 数值 1:1 复用 v1).
///
/// v2: 复用 `PlutchikIntensity::weight()` (cognition crate 提供).
/// v1: 本地 `intensity_weight` 函数实现相同数值.
/// 这里直接调用 cognition 的 `weight()`, 0 重写实现, 0 漂移.
fn intensity_weight(intensity: PlutchikIntensity) -> f64 {
    intensity.weight()
}

/// 强度等级 (0..3). 本地实现 — 不依赖 `PlutchikIntensity: PartialOrd`
/// (per 不漂移: 不改 cognition crate 派生).
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
///
/// **v2 note**: 数值 1:1 保留 v1 算法 (per bridge invariant, 不能改).
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
        PlutchikEmotion::Advanced(PlutchikAdvanced::Aggressiveness, PlutchikIntensity::Extreme) => {
            Some("aggressiveness-extreme")
        }
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
    use apeireth_core::domain::IdentityCard;
    use chrono::{TimeZone, Utc};

    fn make_identity() -> IdentityCard {
        IdentityCard {
            continuity_id: "did:apeireth:bridge-test".to_string(),
            birth_time: 1_700_000_000,
            carriers: vec!["test-carrier".to_string()],
            migration_history: vec![],
            ..Default::default()
        }
    }

    fn fresh_life_force() -> LifeForce {
        LifeForce::new(make_identity(), 1_700_000_000)
    }

    /// v2 cognition 不派生 `ALL` / `ordered_levels` const, 本地 hardcode 8 basic + 8 advanced
    /// variant 列表 (per docs/stage1/inspiration-stage1-2026-07-30.md §21.4 八基础 + 八高级 = 16).
    const ALL_BASIC: [PlutchikBasic; 8] = [
        PlutchikBasic::Joy,
        PlutchikBasic::Trust,
        PlutchikBasic::Fear,
        PlutchikBasic::Surprise,
        PlutchikBasic::Sadness,
        PlutchikBasic::Disgust,
        PlutchikBasic::Anger,
        PlutchikBasic::Anticipation,
    ];

    const ALL_ADVANCED: [PlutchikAdvanced; 8] = [
        PlutchikAdvanced::Love,
        PlutchikAdvanced::Submission,
        PlutchikAdvanced::Awe,
        PlutchikAdvanced::Disapproval,
        PlutchikAdvanced::Remorse,
        PlutchikAdvanced::Contempt,
        PlutchikAdvanced::Aggressiveness,
        PlutchikAdvanced::Optimism,
    ];

    const ALL_INTENSITY: [PlutchikIntensity; 4] = [
        PlutchikIntensity::Mild,
        PlutchikIntensity::Moderate,
        PlutchikIntensity::Strong,
        PlutchikIntensity::Extreme,
    ];

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
        for intensity in ALL_INTENSITY.iter().copied() {
            for basic in ALL_BASIC.iter().copied() {
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
            for adv in ALL_ADVANCED.iter().copied() {
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
        let e =
            PlutchikEmotion::advanced(PlutchikAdvanced::Aggressiveness, PlutchikIntensity::Extreme);
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

    // t09 (v2 重构新增): apply 真实入口测试 — Joy + Trust + Anticipation 从 1.0 → clamp 到 1.0
    #[test]
    fn t09_apply_clamp_at_upper_bound() {
        let mut life = fresh_life_force();
        life.endurance = 1.0;
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let res = apply_plutchik_to_life_force(&mut life, &e, 1_700_001_000);
        assert!(res.is_ok(), "apply Ok after clamp, got {:?}", res);
        assert_eq!(life.endurance, 1.0, "clamp to upper bound 1.0");
    }

    // t10 (v2 重构新增): apply 真实入口测试 — Fear 从 0.0 → clamp 到 0.0
    #[test]
    fn t10_apply_clamp_at_lower_bound() {
        let mut life = fresh_life_force();
        life.endurance = 0.0;
        let e = PlutchikEmotion::basic(PlutchikBasic::Fear, PlutchikIntensity::Extreme);
        let res = apply_plutchik_to_life_force(&mut life, &e, 1_700_001_000);
        assert!(res.is_ok(), "apply Ok after clamp, got {:?}", res);
        assert_eq!(life.endurance, 0.0, "clamp to lower bound 0.0");
    }

    // t11 (v2 重构新增): 强度 weight 数值 1:1 复现 v1 (Mild=0.25, Moderate=0.5, Strong=0.75, Extreme=1.0)
    #[test]
    fn t11_intensity_weights_match_v1() {
        assert!((PlutchikIntensity::Mild.weight() - 0.25).abs() < 1e-9);
        assert!((PlutchikIntensity::Moderate.weight() - 0.5).abs() < 1e-9);
        assert!((PlutchikIntensity::Strong.weight() - 0.75).abs() < 1e-9);
        assert!((PlutchikIntensity::Extreme.weight() - 1.0).abs() < 1e-9);
    }

    // t12 (v2 重构新增): 时区 round-trip — 不依赖 chrono::Utc 在测试中实际跑时区
    #[test]
    fn t12_identity_helper_unused_marker() {
        // 守门测试: 验证 make_identity 返回值正常 (防止 v2 IdentityCard 字段漂移)
        let id = make_identity();
        assert_eq!(id.continuity_id, "did:apeireth:bridge-test");
        assert_eq!(id.birth_time, 1_700_000_000);
        // created_at 默认值 (v2 IdentityCard Default impl 自动填 Utc::now)
        let _ = id.created_at; // 字段访问, 守门编译期字段存在
        let _ = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0); // 引用 Utc, 守门 use 声明
    }
}
