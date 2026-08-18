//! bridge 6: life-force -> motivation (R173 2026-08-14)
//!
//! 目标: apeireth-life-force::LifeForce (endurance + SGI) -> apeireth-motivation::MotivationDrive.
//!
//! 续航驱动动机: 续航充足 → 驱动强度放大; 续航耗竭 → 驱动强度收缩 + 警告.
//! 反思期中 → 进一步收缩 (允许冷静思考).
//! SGI 字段: 续航充足 → 推进 SGI; 续航耗竭 → 建议休息 SGI.
//!
//! 翻译规则:
//! - endurance == 0.0 → multiplier = 0.3 (耗竭, 极限保守)
//! - endurance == 0.5 → multiplier = 0.9 (略低)
//! - endurance == 1.0 → multiplier = 1.5 (满续航, 全力推进)
//! - 反思期中 → multiplier *= 0.8 (允许冷静)
//! - 耗竭警告: endurance < 0.2
//!
//! 不漂移:
//! - 0 改 apeireth-life-force 任何已实装类型 (直接复用类型)
//! - 0 改 apeireth-motivation 任何已实装类型 (复用 InternalDrive.intensity / ExternalDrive.intensity)
//! - 0 副作用: translate 是纯函数; apply 仅做 multiply + clamp
//!
//! 当前状态: R173 最小可用落地 (P1 桥 6 of 7)

#![deny(unsafe_code)]

use apeireth_life_force::{
    LifeForce, ENDURANCE_EXHAUSTION_THRESHOLD, ENDURANCE_MAX, ENDURANCE_MIN,
};

use crate::{ExternalDrive, InternalDrive};

// ============================================
// 1. 翻译结果 — LifeForceMotivationAdjustment
// ============================================

/// 续航到动机的调整建议.
///
/// 字段:
/// - `drive_intensity_multiplier`: 驱动强度乘数 (clamp [0.3, 1.5])
/// - `exhaustion_warning`: 续航是否低于耗竭阈值 (< 0.2)
/// - `in_reflection`: 反思期是否激活
/// - `sgi_urgency`: SGI 节奏建议
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LifeForceMotivationAdjustment {
    /// 驱动强度乘数, clamped [0.3, 1.5].
    pub drive_intensity_multiplier: f64,
    /// 续航耗竭警告 (endurance < 0.2).
    pub exhaustion_warning: bool,
    /// 反思期激活.
    pub in_reflection: bool,
    /// SGI 节奏建议.
    pub sgi_urgency: SGIUrgency,
}

/// SGI 节奏建议.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SGIUrgency {
    /// 正常 — 推进当前 SGI
    Normal,
    /// 加速 — 全力推进 (高续航)
    Heightened,
    /// 放慢 — 建议缩小范围 (低续航)
    Reduced,
    /// 暂停 — 建议休息/反思 (耗竭)
    Suspended,
}

impl SGIUrgency {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Heightened => "heightened",
            Self::Reduced => "reduced",
            Self::Suspended => "suspended",
        }
    }
}

// ============================================
// 2. 内部辅助
// ============================================

/// endurance -> multiplier 线性映射.
///
/// 设计: multiplier = 0.3 + endurance * 1.2 ∈ [0.3, 1.5]
/// - endurance = 0.0 → 0.3 (极限保守)
/// - endurance = 0.5 → 0.9
/// - endurance = 1.0 → 1.5 (全力推进)
fn endurance_to_multiplier(endurance: f64) -> f64 {
    let raw = 0.3 + endurance * 1.2;
    raw.clamp(0.3, 1.5)
}

/// SGI 节奏建议: 基于续航 + 反思状态.
fn derive_sgi_urgency(endurance: f64, in_reflection: bool) -> SGIUrgency {
    if in_reflection {
        SGIUrgency::Reduced
    } else if endurance < ENDURANCE_EXHAUSTION_THRESHOLD {
        SGIUrgency::Suspended
    } else if endurance >= 0.8 {
        SGIUrgency::Heightened
    } else {
        SGIUrgency::Normal
    }
}

// ============================================
// 3. 公共 API — translate (纯) + apply (mutating)
// ============================================

/// 纯翻译: LifeForce -> LifeForceMotivationAdjustment.
/// 0 副作用, 0 改源/目标. 纯函数.
pub fn life_force_to_motivation_adjustment(
    life: &LifeForce,
    now: i64,
) -> LifeForceMotivationAdjustment {
    let endurance = life.endurance.clamp(ENDURANCE_MIN, ENDURANCE_MAX);
    let base_multiplier = endurance_to_multiplier(endurance);
    let in_reflection = life.is_in_reflection(now);
    // 反思期中: 进一步收缩 (允许冷静)
    let drive_intensity_multiplier = if in_reflection {
        base_multiplier * 0.8
    } else {
        base_multiplier
    }
    .clamp(0.3, 1.5);
    let exhaustion_warning = endurance < ENDURANCE_EXHAUSTION_THRESHOLD;
    let sgi_urgency = derive_sgi_urgency(endurance, in_reflection);
    LifeForceMotivationAdjustment {
        drive_intensity_multiplier,
        exhaustion_warning,
        in_reflection,
        sgi_urgency,
    }
}

/// 在 InternalDrive 上应用续航调整 (per 桥 6 入口).
///
/// 入口语义: intensity *= multiplier, clamp [0.0, 1.0].
/// 不修改 `label` —— 桥 6 只调整强度, 不动身份.
pub fn apply_life_force_to_internal_drive(
    drive: &mut InternalDrive,
    life: &LifeForce,
    now: i64,
) -> f64 {
    let adj = life_force_to_motivation_adjustment(life, now);
    let new_intensity = (drive.intensity * adj.drive_intensity_multiplier).clamp(0.0, 1.0);
    drive.intensity = new_intensity;
    new_intensity
}

/// 在 ExternalDrive 上应用续航调整.
pub fn apply_life_force_to_external_drive(
    drive: &mut ExternalDrive,
    life: &LifeForce,
    now: i64,
) -> f64 {
    let adj = life_force_to_motivation_adjustment(life, now);
    let new_intensity = (drive.intensity * adj.drive_intensity_multiplier).clamp(0.0, 1.0);
    drive.intensity = new_intensity;
    new_intensity
}

// ============================================
// 4. 单元测试 (8 个 + 2 附加)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::IdentityCard;
    use apeireth_life_force::SelfGrowthIndicator;

    fn make_identity() -> IdentityCard {
        IdentityCard {
            continuity_id: "did:apeireth:bridge6-test".to_string(),
            birth_time: 1_700_000_000,
            carriers: vec!["carrier-A".to_string()],
            migration_history: vec![],
        }
    }

    fn make_life(endurance: f64, in_reflection: bool, now: i64) -> LifeForce {
        let mut life = LifeForce::new(make_identity(), now);
        life.endurance = endurance;
        if in_reflection {
            life.sgi = SelfGrowthIndicator::new("assist", now);
            life.reflection = life.reflection.start(now);
        }
        life
    }

    // t01: 高续航 (1.0) → multiplier == 1.5 (full boost)
    #[test]
    fn t01_high_endurance_yields_max_multiplier() {
        let life = make_life(1.0, false, 1_700_000_000);
        let adj = life_force_to_motivation_adjustment(&life, 1_700_000_000);
        assert!(
            (adj.drive_intensity_multiplier - 1.5).abs() < 1e-9,
            "endurance=1.0 should give multiplier=1.5, got {}",
            adj.drive_intensity_multiplier
        );
        assert_eq!(adj.sgi_urgency, SGIUrgency::Heightened);
    }

    // t02: 低续航 (0.0) → multiplier == 0.3 (extreme conservative)
    #[test]
    fn t02_zero_endurance_yields_min_multiplier() {
        let life = make_life(0.0, false, 1_700_000_000);
        let adj = life_force_to_motivation_adjustment(&life, 1_700_000_000);
        assert!(
            (adj.drive_intensity_multiplier - 0.3).abs() < 1e-9,
            "endurance=0.0 should give multiplier=0.3, got {}",
            adj.drive_intensity_multiplier
        );
        assert!(adj.exhaustion_warning);
        assert_eq!(adj.sgi_urgency, SGIUrgency::Suspended);
    }

    // t03: 反思期中 → multiplier 进一步收缩
    #[test]
    fn t03_in_reflection_dampens_multiplier() {
        let life_n = make_life(1.0, false, 1_700_000_000);
        let life_r = make_life(1.0, true, 1_700_000_000);
        let adj_n = life_force_to_motivation_adjustment(&life_n, 1_700_000_000);
        let adj_r = life_force_to_motivation_adjustment(&life_r, 1_700_000_000);
        assert!(
            adj_r.drive_intensity_multiplier < adj_n.drive_intensity_multiplier,
            "reflection should dampen multiplier, got r={} n={}",
            adj_r.drive_intensity_multiplier,
            adj_n.drive_intensity_multiplier
        );
        assert!(adj_r.in_reflection);
        assert_eq!(adj_r.sgi_urgency, SGIUrgency::Reduced);
    }

    // t04: 中等续航 (0.5) → multiplier ≈ 0.9
    #[test]
    fn t04_mid_endurance_yields_near_neutral_multiplier() {
        let life = make_life(0.5, false, 1_700_000_000);
        let adj = life_force_to_motivation_adjustment(&life, 1_700_000_000);
        assert!(
            (adj.drive_intensity_multiplier - 0.9).abs() < 1e-9,
            "endurance=0.5 should give multiplier=0.9, got {}",
            adj.drive_intensity_multiplier
        );
        assert!(!adj.exhaustion_warning);
        assert_eq!(adj.sgi_urgency, SGIUrgency::Normal);
    }

    // t05: multiplier 始终 clamp 在 [0.3, 1.5]
    #[test]
    fn t05_multiplier_clamped_to_valid_range() {
        // 测试 0.0..1.0 全 11 个采样点
        for i in 0..=10 {
            let endurance = f64::from(i) / 10.0;
            let life = make_life(endurance, false, 1_700_000_000);
            let adj = life_force_to_motivation_adjustment(&life, 1_700_000_000);
            assert!(
                adj.drive_intensity_multiplier >= 0.3 - 1e-9,
                "endurance={} multiplier {} below 0.3",
                endurance,
                adj.drive_intensity_multiplier
            );
            assert!(
                adj.drive_intensity_multiplier <= 1.5 + 1e-9,
                "endurance={} multiplier {} above 1.5",
                endurance,
                adj.drive_intensity_multiplier
            );
        }
    }

    // t06: 耗竭警告触发阈值
    #[test]
    fn t06_exhaustion_warning_at_threshold() {
        let life_below = make_life(0.19, false, 1_700_000_000);
        let life_above = make_life(0.21, false, 1_700_000_000);
        assert!(life_force_to_motivation_adjustment(&life_below, 1_700_000_000).exhaustion_warning);
        assert!(
            !life_force_to_motivation_adjustment(&life_above, 1_700_000_000).exhaustion_warning
        );
    }

    // t07: 反思期 SGI 节奏 = Reduced
    #[test]
    fn t07_reflection_yields_reduced_sgi_urgency() {
        let life = make_life(1.0, true, 1_700_000_000);
        let adj = life_force_to_motivation_adjustment(&life, 1_700_000_000);
        assert_eq!(adj.sgi_urgency, SGIUrgency::Reduced);
    }

    // t08: endurance 与 multiplier 单调性 (高 → 高)
    #[test]
    fn t08_endurance_monotonic_with_multiplier() {
        let mut last = 0.0_f64;
        for i in 1..=10 {
            let endurance = f64::from(i) / 10.0;
            let life = make_life(endurance, false, 1_700_000_000);
            let adj = life_force_to_motivation_adjustment(&life, 1_700_000_000);
            assert!(
                adj.drive_intensity_multiplier > last,
                "endurance={} multiplier {} should exceed previous {}",
                endurance,
                adj.drive_intensity_multiplier,
                last
            );
            last = adj.drive_intensity_multiplier;
        }
    }

    // t09: apply 实际缩放 drive.intensity
    #[test]
    fn t09_apply_scales_internal_drive() {
        let mut drive = InternalDrive::new("self", 1.0);
        let life = make_life(0.0, false, 1_700_000_000); // multiplier = 0.3
        let after = apply_life_force_to_internal_drive(&mut drive, &life, 1_700_000_000);
        assert!(
            after < 1.0,
            "low endurance should reduce drive, got {}",
            after
        );
        assert!(after >= 0.0);
        assert_eq!(drive.label, "self", "label should not change");
    }

    // t10: apply 在高续航 + 反思期 → 收缩 (反思期 0.8 damp)
    #[test]
    fn t10_apply_reflection_dampens_drive() {
        let mut drive_with = InternalDrive::new("d", 0.5);
        let mut drive_without = InternalDrive::new("d", 0.5);
        let life = make_life(1.0, true, 1_700_000_000);
        let life_no = make_life(1.0, false, 1_700_000_000);
        let a = apply_life_force_to_internal_drive(&mut drive_with, &life, 1_700_000_000);
        let b = apply_life_force_to_internal_drive(&mut drive_without, &life_no, 1_700_000_000);
        assert!(a < b, "reflection should dampen, got r={} n={}", a, b);
    }
}
