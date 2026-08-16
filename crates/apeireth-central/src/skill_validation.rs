//! `SkillValidation` — Skill 验证 (R125-18 升级)
//!
//! # 借鉴 ID
//!
//! `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10` (per 决策 #51 §1.4 P3-1 + R125-18 decision-log)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! 借鉴模式: superpowers v6.2.0 公开 skill 质量门:
//!           - TDD skill 必含至少 1 个 `[RED]` step
//!           - 每个 skill 至少 3 步 checklist
//!           - skill name / description 必填非空
//!           - step description 必填非空
//! clone 状态: ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1)
//!
//! # 核心
//!
//! 1 个 `validate_skill()` 返回 `SkillValidationReport` 含 0+ 个 error, 编译期 0 dep,
//! 仅使用 Skill trait 公开 fn. 0 装"已实施完整 superpowers schema 验证" — 我们仅做最小
//! 4 项检查 (name / description / steps 数 / TDD red 步骤).
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (1:1 映射 superpowers 公开质量门, 0 越界 8 硬墙)
//! - 0 装"已实施完整 superpowers schema 验证" (仅最小 4 项)
//! - 0 触碰 R125-15e Skill trait (仅消费 trait 方法)

#![deny(unsafe_code)]

use crate::skill_trait::SkillId;
use std::fmt;

/// 最小 step 数 (借鉴 superpowers 公开 skill 至少 3 步 checklist).
pub const MIN_STEP_COUNT: usize = 3;

/// Skill 验证错误 (1 项 = 1 错误).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillValidationError {
    /// skill name 必填非空.
    NameEmpty,
    /// skill description 必填非空.
    DescriptionEmpty,
    /// step 数低于 `MIN_STEP_COUNT`.
    TooFewSteps {
        /// 实际 step 数.
        count: usize,
        /// 最小要求.
        min: usize,
    },
    /// TDD skill 缺 RED 步骤 (借鉴 superpowers iron law).
    MissingTddRedStep,
    /// step description 必填非空 (按 step order 报告).
    StepDescriptionEmpty {
        /// 出错的 step 序号 (1-based).
        step_order: u8,
    },
}

impl fmt::Display for SkillValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameEmpty => write!(f, "skill name is empty"),
            Self::DescriptionEmpty => write!(f, "skill description is empty"),
            Self::TooFewSteps { count, min } => {
                write!(f, "skill has {count} steps, minimum is {min}")
            }
            Self::MissingTddRedStep => {
                write!(f, "TDD skill is missing RED step (iron law violation)")
            }
            Self::StepDescriptionEmpty { step_order } => {
                write!(f, "step {step_order} has empty description")
            }
        }
    }
}

impl std::error::Error for SkillValidationError {}

/// 1 个 skill 的完整验证报告.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillValidationReport {
    /// 验证的 skill id.
    pub skill_id: SkillId,
    /// skill 人类可读 name.
    pub skill_name: String,
    /// 0+ 个 error.
    pub errors: Vec<SkillValidationError>,
}

impl SkillValidationReport {
    /// 0 error = valid.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// error 数.
    pub fn n_errors(&self) -> usize {
        self.errors.len()
    }

    /// 1 段人类可读摘要.
    pub fn summary(&self) -> String {
        if self.is_valid() {
            format!("[{}] VALID", self.skill_name)
        } else {
            format!(
                "[{}] INVALID ({} error{}): {:?}",
                self.skill_name,
                self.n_errors(),
                if self.n_errors() == 1 { "" } else { "s" },
                self.errors
                    .iter()
                    .map(|e| format!("{e}"))
                    .collect::<Vec<_>>()
            )
        }
    }
}

/// 验证 1 个 skill (使用 Skill trait obj).
///
/// 借鉴 superpowers 公开质量门, 4 项检查:
/// 1. `name` 必填非空
/// 2. `description` 必填非空
/// 3. `steps` 数 >= `MIN_STEP_COUNT`
/// 4. TDD skill 必含至少 1 个 `is_tdd_red` step
/// 5. 每个 step `description` 必填非空
pub fn validate_skill(skill: &dyn crate::skill_trait::Skill) -> SkillValidationReport {
    let mut errors = Vec::new();

    // 1. name
    if skill.name().trim().is_empty() {
        errors.push(SkillValidationError::NameEmpty);
    }

    // 2. description
    if skill.when_to_use().trim().is_empty() {
        errors.push(SkillValidationError::DescriptionEmpty);
    }

    // 3. step count
    let steps = skill.steps();
    if steps.len() < MIN_STEP_COUNT {
        errors.push(SkillValidationError::TooFewSteps {
            count: steps.len(),
            min: MIN_STEP_COUNT,
        });
    }

    // 4. TDD red step
    if skill.tdd_required() && !steps.iter().any(|s| s.is_tdd_red) {
        errors.push(SkillValidationError::MissingTddRedStep);
    }

    // 5. step descriptions
    for step in steps {
        if step.description.trim().is_empty() {
            errors.push(SkillValidationError::StepDescriptionEmpty {
                step_order: step.order,
            });
        }
    }

    SkillValidationReport {
        skill_id: skill.id(),
        skill_name: skill.name().to_string(),
        errors,
    }
}

/// 批量验证: 跑过 1 个 `SkillRegistry` 全部 14 skill, 返回 14 reports.
pub fn validate_registry(
    registry: &crate::skill_registry::SkillRegistry,
) -> Vec<SkillValidationReport> {
    registry
        .all()
        .iter()
        .map(|s| validate_skill(s.as_ref()))
        .collect()
}

/// 统计 14 skill valid 比例 (0.0..=1.0).
pub fn registry_validity_ratio(reports: &[SkillValidationReport]) -> f64 {
    if reports.is_empty() {
        return 1.0;
    }
    let valid = reports.iter().filter(|r| r.is_valid()).count();
    valid as f64 / reports.len() as f64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_registry::SkillRegistry;
    use crate::skill_trait::{
        BrainstormingSkill, TestDrivenDevelopmentSkill, UsingSuperpowersSkill,
    };

    #[test]
    fn validate_tdd_skill_passes() {
        let skill = TestDrivenDevelopmentSkill;
        let report = validate_skill(&skill);
        assert!(
            report.is_valid(),
            "TDD should be valid: {:?}",
            report.errors
        );
        assert_eq!(report.skill_id, SkillId::TestDrivenDevelopment);
    }

    #[test]
    fn validate_brainstorming_skill_passes() {
        let skill = BrainstormingSkill;
        let report = validate_skill(&skill);
        assert!(
            report.is_valid(),
            "Brainstorming should be valid: {:?}",
            report.errors
        );
    }

    #[test]
    fn validate_meta_skill_using_superpowers_passes() {
        let skill = UsingSuperpowersSkill;
        let report = validate_skill(&skill);
        assert!(
            report.is_valid(),
            "UsingSuperpowers should be valid: {:?}",
            report.errors
        );
    }

    #[test]
    fn validate_registry_all_14_skills_valid() {
        let registry = SkillRegistry::new();
        let reports = validate_registry(&registry);
        assert_eq!(reports.len(), 14);
        for report in &reports {
            assert!(
                report.is_valid(),
                "{} should be valid: {:?}",
                report.skill_name,
                report.errors
            );
        }
    }

    #[test]
    fn validity_ratio_for_14_valid_skills_is_1() {
        let registry = SkillRegistry::new();
        let reports = validate_registry(&registry);
        let ratio = registry_validity_ratio(&reports);
        assert!((ratio - 1.0).abs() < 1e-9);
    }

    #[test]
    fn min_step_count_constant_is_3() {
        assert_eq!(MIN_STEP_COUNT, 3);
    }

    #[test]
    fn skill_validation_error_display_is_human_readable() {
        let e1 = SkillValidationError::NameEmpty;
        assert_eq!(format!("{e1}"), "skill name is empty");
        let e2 = SkillValidationError::TooFewSteps { count: 2, min: 3 };
        assert_eq!(format!("{e2}"), "skill has 2 steps, minimum is 3");
        let e3 = SkillValidationError::MissingTddRedStep;
        assert_eq!(
            format!("{e3}"),
            "TDD skill is missing RED step (iron law violation)"
        );
    }

    #[test]
    fn validation_report_n_errors_counts_correctly() {
        let skill = TestDrivenDevelopmentSkill;
        let report = validate_skill(&skill);
        assert_eq!(report.n_errors(), 0);
    }

    #[test]
    fn validation_report_summary_indicates_valid() {
        let skill = TestDrivenDevelopmentSkill;
        let report = validate_skill(&skill);
        let summary = report.summary();
        assert!(summary.contains("VALID"));
        assert!(summary.contains("Test-Driven Development"));
    }
}
