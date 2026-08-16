//! bridge 3: consciousness -> motivation (R173 2026-08-14)
//!
//! 目标: apeireth-consciousness::PlutchikEmotion -> apeireth-motivation::MotivationDrive + SGI.
//!
//! 情感驱动动机: 正面情感提升内驱, 负面情感降低内驱并触发反思.
//! 桥 3 把 Plutchik 情感翻译为 MotivationAdjustment:
//! - internal_drive_delta: 内驱强度调整, 范围 [-0.2, +0.2]
//! - external_drive_delta: 外驱强度调整, 范围 [-0.2, +0.2]
//! - sgi_suggestion: 可选 SGI 建议 (只有特定情感触发)
//! - should_trigger_reflection: 是否建议触发反思
//!
//! 不漂移:
//! - 0 改 apeireth-consciousness 任何已实装类型 (本桥不依赖, 直接复用类型)
//! - 0 改 apeireth-motivation 任何已实装类型 (复用 InternalDrive, ExternalDrive, SGIStructured, SGIContent, SGIEntry, write_flow)
//! - 0 副作用: translate 是纯函数; apply_drive 是字段 mutating; 不在桥里直接 commit SGI (需要 E 层证据, 留给上游)
//!
//! 当前状态: R173 最小可用落地 (P0 桥 3 of 7)

#![deny(unsafe_code)]

use apeireth_consciousness::plutchik::{
    PlutchikAdvanced, PlutchikBasic, PlutchikEmotion, PlutchikIntensity,
};

use crate::{DriveKind, InternalDrive, SGIContent, SGIEntry, SGIStructured};

// ============================================
// 1. 翻译结果 — MotivationAdjustment
// ============================================

/// 动机调整建议 — consciousness -> motivation 的翻译结果.
///
/// 字段:
/// - `internal_drive_delta`: 内驱强度调整 (per-emotion, [-0.2, +0.2])
/// - `external_drive_delta`: 外驱强度调整 (per-emotion, [-0.2, +0.2])
/// - `sgi_suggestion`: 可选 SGI 建议 (None = 不推荐)
/// - `should_trigger_reflection`: 是否建议触发反思
/// - `reflection_reason`: 触发原因 (`None` = 不触发)
#[derive(Debug, Clone, PartialEq)]
pub struct MotivationAdjustment {
    /// 内驱强度调整 (per-emotion, clamped [-0.2, +0.2]).
    pub internal_drive_delta: f64,
    /// 外驱强度调整 (per-emotion, clamped [-0.2, +0.2]).
    pub external_drive_delta: f64,
    /// 可选 SGI 建议 (None = 不推荐).
    pub sgi_suggestion: Option<SGISuggestion>,
    /// 是否建议触发反思.
    pub should_trigger_reflection: bool,
    /// 触发反思的原因 (`None` = 不触发).
    pub reflection_reason: Option<&'static str>,
}

/// SGI 建议 — 桥 3 仅产出建议, 不直接 commit (commit 需要 E 层证据).
///
/// 设计: 桥 3 翻译建议, 上游 (action 层) 负责收集证据 + 调 write_flow 写入.
#[derive(Debug, Clone, PartialEq)]
pub struct SGISuggestion {
    /// C-SGI-7 ① 目标
    pub goal: String,
    /// C-SGI-7 ② 期限
    pub deadline: String,
    /// C-SGI-7 ③ 成功标准
    pub success_criteria: String,
    /// 驱动标签 (供反思期追溯)
    pub drive_label: String,
    /// 驱动强度 [0.0, 1.0]
    pub intensity: f64,
}

impl SGISuggestion {
    /// 转为 SGIContent::Structured (C-SGI-5 三选一之 ①).
    pub fn to_structured_content(&self) -> SGIContent {
        SGIContent::Structured(SGIStructured {
            goal: self.goal.clone(),
            deadline: self.deadline.clone(),
            success_criteria: self.success_criteria.clone(),
            extras: Default::default(),
            multimodal: None,
        })
    }
}

// ============================================
// 2. 内部辅助 — 强度权 + 基线 + 触发条件
// ============================================

/// 强度映射 (与桥 2/5 保持一致).
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

/// 内驱 (InternalDrive) 基线 delta.
fn internal_base_delta(e: &PlutchikEmotion) -> f64 {
    match e {
        // 正面 — 提升内驱
        PlutchikEmotion::Basic(PlutchikBasic::Joy, _) => 0.15,
        PlutchikEmotion::Basic(PlutchikBasic::Trust, _) => 0.10,
        PlutchikEmotion::Basic(PlutchikBasic::Anticipation, _) => 0.08,
        PlutchikEmotion::Basic(PlutchikBasic::Surprise, _) => 0.05,
        // 负面 — 降低内驱
        PlutchikEmotion::Basic(PlutchikBasic::Sadness, _) => -0.15,
        PlutchikEmotion::Basic(PlutchikBasic::Fear, _) => -0.12,
        PlutchikEmotion::Basic(PlutchikBasic::Anger, _) => -0.10,
        PlutchikEmotion::Basic(PlutchikBasic::Disgust, _) => -0.08,
        // 高级 — 复合情感, 按主轴定基线
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

/// 外驱 (ExternalDrive) 基线 delta.
///
/// 设计: 外驱主要由"对外界刺激的反应"驱动, 所以 Surprise/Anticipation 偏正,
/// Sadness/Disgust 偏负.
fn external_base_delta(e: &PlutchikEmotion) -> f64 {
    match e {
        // 期待/惊喜 — 强化对外部输入的关注
        PlutchikEmotion::Basic(PlutchikBasic::Anticipation, _) => 0.10,
        PlutchikEmotion::Basic(PlutchikBasic::Surprise, _) => 0.08,
        PlutchikEmotion::Basic(PlutchikBasic::Trust, _) => 0.05,
        // 厌恶/悲伤 — 防御姿态, 拒外部输入
        PlutchikEmotion::Basic(PlutchikBasic::Disgust, _) => -0.10,
        PlutchikEmotion::Basic(PlutchikBasic::Sadness, _) => -0.05,
        PlutchikEmotion::Basic(PlutchikBasic::Fear, _) => -0.05,
        // Joy/Anger 不直接动外驱 (它们是内在状态)
        PlutchikEmotion::Basic(PlutchikBasic::Joy, _) => 0.0,
        PlutchikEmotion::Basic(PlutchikBasic::Anger, _) => 0.0,
        // 高级情感
        PlutchikEmotion::Advanced(PlutchikAdvanced::Optimism, _) => 0.08,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Love, _) => 0.05,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Awe, _) => 0.05,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Submission, _) => 0.05,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Disapproval, _) => -0.05,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Remorse, _) => 0.0,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Contempt, _) => -0.05,
        PlutchikEmotion::Advanced(PlutchikAdvanced::Aggressiveness, _) => 0.05,
    }
}

/// 反思触发条件 — 与桥 2 保持一致 (per 蓝图中"情感走同一套反思决策").
fn should_trigger(e: &PlutchikEmotion) -> Option<&'static str> {
    match e {
        PlutchikEmotion::Basic(PlutchikBasic::Fear, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Moderate) =>
        {
            Some("fear-moderate-or-above")
        }
        PlutchikEmotion::Basic(PlutchikBasic::Sadness, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Strong) =>
        {
            Some("sadness-strong-or-above")
        }
        PlutchikEmotion::Basic(PlutchikBasic::Anger, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Strong) =>
        {
            Some("anger-strong-or-above")
        }
        PlutchikEmotion::Advanced(PlutchikAdvanced::Aggressiveness, PlutchikIntensity::Extreme) => {
            Some("aggressiveness-extreme")
        }
        _ => None,
    }
}

/// SGI 建议触发条件 — 哪些情感会产生 SGI 调整建议.
///
/// 设计: 强烈的"前瞻性"或"反思性"情感触发 SGI 建议.
fn sgi_suggestion_for(e: &PlutchikEmotion) -> Option<SGISuggestion> {
    match e {
        // Anticipation strong+ → 探索新目标
        PlutchikEmotion::Basic(PlutchikBasic::Anticipation, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Strong) =>
        {
            Some(SGISuggestion {
                goal: "explore-anticipated-direction".into(),
                deadline: "72h".into(),
                success_criteria: "scope-and-plan-emitted".into(),
                drive_label: "anticipation-strong".into(),
                intensity: intensity_weight(*i),
            })
        }
        // Optimism strong+ → 推进当前目标
        PlutchikEmotion::Advanced(PlutchikAdvanced::Optimism, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Strong) =>
        {
            Some(SGISuggestion {
                goal: "advance-current-via-optimism".into(),
                deadline: "168h".into(),
                success_criteria: "milestone-progress-logged".into(),
                drive_label: "optimism-strong".into(),
                intensity: intensity_weight(*i),
            })
        }
        // Sadness strong+ → 反思目标
        PlutchikEmotion::Basic(PlutchikBasic::Sadness, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Strong) =>
        {
            Some(SGISuggestion {
                goal: "reflect-on-sadness-trigger".into(),
                deadline: "during-reflection-period".into(),
                success_criteria: "cause-identified-and-logged".into(),
                drive_label: "sadness-strong".into(),
                intensity: intensity_weight(*i),
            })
        }
        // Fear moderate+ → 自保目标
        PlutchikEmotion::Basic(PlutchikBasic::Fear, i)
            if intensity_rank(*i) >= intensity_rank(PlutchikIntensity::Moderate) =>
        {
            Some(SGISuggestion {
                goal: "identify-fear-source-and-mitigate".into(),
                deadline: "during-reflection-period".into(),
                success_criteria: "risk-bounded".into(),
                drive_label: "fear-moderate".into(),
                intensity: intensity_weight(*i),
            })
        }
        _ => None,
    }
}

// ============================================
// 3. 公共 API — translate (纯) + apply (mutating)
// ============================================

/// 纯翻译: PlutchikEmotion -> MotivationAdjustment.
/// 0 副作用, 0 改源/目标. 纯函数.
pub fn plutchik_to_motivation_adjustment(e: &PlutchikEmotion) -> MotivationAdjustment {
    let intensity = intensity_weight(e.intensity());
    let raw_internal = internal_base_delta(e) * intensity;
    let raw_external = external_base_delta(e) * intensity;
    let internal_drive_delta = raw_internal.clamp(-0.2, 0.2);
    let external_drive_delta = raw_external.clamp(-0.2, 0.2);
    let sgi_suggestion = sgi_suggestion_for(e);
    let reflection_reason = should_trigger(e);
    MotivationAdjustment {
        internal_drive_delta,
        external_drive_delta,
        sgi_suggestion,
        should_trigger_reflection: reflection_reason.is_some(),
        reflection_reason,
    }
}

/// 在 InternalDrive 上应用 Plutchik 情感 (per 桥 3 入口).
///
/// 入口语义: 计算 delta, 校验后累加 (钳位 [0, 1]).
/// 不修改 `label` —— 桥 3 只调整强度, 不动身份.
pub fn apply_plutchik_to_internal_drive(drive: &mut InternalDrive, e: &PlutchikEmotion) -> f64 {
    let adj = plutchik_to_motivation_adjustment(e);
    let new_intensity = (drive.intensity + adj.internal_drive_delta).clamp(0.0, 1.0);
    drive.intensity = new_intensity;
    new_intensity
}

/// 在 ExternalDrive 上应用 Plutchik 情感.
pub fn apply_plutchik_to_external_drive(
    drive: &mut crate::ExternalDrive,
    e: &PlutchikEmotion,
) -> f64 {
    let adj = plutchik_to_motivation_adjustment(e);
    let new_intensity = (drive.intensity + adj.external_drive_delta).clamp(0.0, 1.0);
    drive.intensity = new_intensity;
    new_intensity
}

/// 把 SGI 建议转成 SGIEntry (待上游收集证据后调 write_flow).
///
/// **不漂移**: 桥 3 不直接 commit SGI, 仅产出 entry 骨架.
pub fn sgi_entry_from_suggestion(s: &SGISuggestion) -> SGIEntry {
    let stub_drive = InternalDrive::new(s.drive_label.clone(), s.intensity);
    SGIEntry::new(s.to_structured_content(), &stub_drive)
}

// ============================================
// 4. 单元测试 (8 个核心 + 4 个 apply 验证)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DriveKind;

    // t01: joy strong -> internal_drive_delta > 0
    #[test]
    fn t01_joy_strong_yields_positive_internal_delta() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(
            adj.internal_drive_delta > 0.0,
            "joy strong should yield positive internal_drive_delta, got {}",
            adj.internal_drive_delta
        );
        assert!(!adj.should_trigger_reflection);
    }

    // t02: sadness strong -> internal_drive_delta < 0
    #[test]
    fn t02_sadness_strong_yields_negative_internal_delta() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Strong);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(
            adj.internal_drive_delta < 0.0,
            "sadness strong should yield negative internal_drive_delta, got {}",
            adj.internal_drive_delta
        );
    }

    // t03: fear moderate -> should_trigger_reflection
    #[test]
    fn t03_fear_moderate_triggers_reflection() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Fear, PlutchikIntensity::Moderate);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(adj.should_trigger_reflection);
        assert!(adj.reflection_reason.is_some());
    }

    // t04: sadness mild -> !should_trigger_reflection
    #[test]
    fn t04_sadness_mild_does_not_trigger_reflection() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Mild);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(!adj.should_trigger_reflection);
        assert!(adj.reflection_reason.is_none());
    }

    // t05: deltas clamped [-0.2, 0.2]
    #[test]
    fn t05_deltas_clamped_to_band() {
        for intensity in PlutchikIntensity::ordered_levels() {
            for basic in PlutchikBasic::ALL {
                let e = PlutchikEmotion::basic(basic, intensity);
                let adj = plutchik_to_motivation_adjustment(&e);
                assert!(
                    adj.internal_drive_delta >= -0.2 && adj.internal_drive_delta <= 0.2,
                    "basic {:?} {:?} internal_delta {}",
                    basic,
                    intensity,
                    adj.internal_drive_delta
                );
                assert!(
                    adj.external_drive_delta >= -0.2 && adj.external_drive_delta <= 0.2,
                    "basic {:?} {:?} external_delta {}",
                    basic,
                    intensity,
                    adj.external_drive_delta
                );
            }
            for adv in PlutchikAdvanced::ALL {
                let e = PlutchikEmotion::advanced(adv, intensity);
                let adj = plutchik_to_motivation_adjustment(&e);
                assert!(
                    adj.internal_drive_delta >= -0.2 && adj.internal_drive_delta <= 0.2,
                    "adv {:?} {:?} internal_delta {}",
                    adv,
                    intensity,
                    adj.internal_drive_delta
                );
                assert!(
                    adj.external_drive_delta >= -0.2 && adj.external_drive_delta <= 0.2,
                    "adv {:?} {:?} external_delta {}",
                    adv,
                    intensity,
                    adj.external_drive_delta
                );
            }
        }
    }

    // t06: advanced optimism -> internal_drive_delta > 0
    #[test]
    fn t06_advanced_optimism_yields_positive_internal_delta() {
        let e = PlutchikEmotion::advanced(PlutchikAdvanced::Optimism, PlutchikIntensity::Strong);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(
            adj.internal_drive_delta > 0.0,
            "advanced optimism should yield positive internal_drive_delta, got {}",
            adj.internal_drive_delta
        );
    }

    // t07: advanced aggressiveness extreme -> should_trigger_reflection
    #[test]
    fn t07_advanced_aggressiveness_extreme_triggers_reflection() {
        let e =
            PlutchikEmotion::advanced(PlutchikAdvanced::Aggressiveness, PlutchikIntensity::Extreme);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(adj.should_trigger_reflection);
        assert!(adj.internal_drive_delta < 0.0);
    }

    // t08: intensity scale (mild < extreme)
    #[test]
    fn t08_intensity_scales_internal_drive_delta() {
        let mild = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Mild);
        let extreme = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let m = plutchik_to_motivation_adjustment(&mild);
        let e = plutchik_to_motivation_adjustment(&extreme);
        assert!(
            e.internal_drive_delta > m.internal_drive_delta,
            "extreme ({}) should yield greater internal_delta than mild ({})",
            e.internal_drive_delta,
            m.internal_drive_delta
        );
    }

    // t09: apply updates InternalDrive.intensity
    #[test]
    fn t09_apply_updates_internal_drive_intensity() {
        let mut drive = InternalDrive::new("self_goal", 0.5);
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Strong);
        let after = apply_plutchik_to_internal_drive(&mut drive, &e);
        assert!(after > 0.5, "joy strong should bump drive, got {}", after);
        assert!(after <= 1.0);
        assert_eq!(drive.label, "self_goal", "label should not change");
    }

    // t10: apply clamps drive to [0, 1]
    #[test]
    fn t10_apply_clamps_drive_to_unit_range() {
        let mut drive = InternalDrive::new("d", 0.95);
        let e = PlutchikEmotion::basic(PlutchikBasic::Joy, PlutchikIntensity::Extreme);
        let after = apply_plutchik_to_internal_drive(&mut drive, &e);
        assert!(after <= 1.0, "drive must clamp to 1.0, got {}", after);

        let mut drive2 = InternalDrive::new("d", 0.05);
        let e2 = PlutchikEmotion::basic(PlutchikBasic::Sadness, PlutchikIntensity::Extreme);
        let after2 = apply_plutchik_to_internal_drive(&mut drive2, &e2);
        assert!(after2 >= 0.0, "drive must clamp to 0.0, got {}", after2);
    }

    // t11: SGI suggestion for Anticipation strong -> SGIEntry built
    #[test]
    fn t11_sgi_suggestion_for_anticipation_emits_entry() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Anticipation, PlutchikIntensity::Strong);
        let adj = plutchik_to_motivation_adjustment(&e);
        let sgi = adj
            .sgi_suggestion
            .expect("anticipation strong should suggest SGI");
        assert!(sgi.goal.starts_with("explore-"));
        let entry = sgi_entry_from_suggestion(&sgi);
        assert_eq!(entry.drive_kind, DriveKind::Internal);
        assert!(matches!(entry.content, SGIContent::Structured(_)));
    }

    // t12: weak emotion does not trigger SGI suggestion
    #[test]
    fn t12_weak_emotion_no_sgi_suggestion() {
        let e = PlutchikEmotion::basic(PlutchikBasic::Anticipation, PlutchikIntensity::Mild);
        let adj = plutchik_to_motivation_adjustment(&e);
        assert!(
            adj.sgi_suggestion.is_none(),
            "mild anticipation should not trigger SGI"
        );
    }
}
