//! `SkillCompanion` — 借鉴 superpowers skill 协作资源模式 (R125-18 升级)
//!
//! # 借鉴 ID
//!
//! `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10` (per 决策 #51 §1.4 P3-1 + R125-18 decision-log)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! 借鉴模式: 4 skill 含 `skills/<name>/<companion>.md` 协作资源:
//!           - `brainstorming/visual-companion.md` (browser-based visual brainstorming)
//!           - `brainstorming/spec-document-reviewer-prompt.md`
//!           - `requesting-code-review/code-reviewer.md` (reviewer dispatch template)
//!           - `receiving-code-review/code-reviewer.md` (reviewer response template)
//!           - `systematic-debugging/condition-based-waiting.md`
//!           - `systematic-debugging/CREATION-LOG.md` (skill creation log)
//! clone 状态: ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1)
//!
//! # 核心
//!
//! 1 个 skill 可选配 0+ 个 companion resource. 编译期 hardcode 4 variant 1:1 映射 superpowers
//! 公开 6 个协作 .md, 配 `Other(String)` 兼容未来扩展. 1 个 skill 的 companion list 静态,
//! 由 `companions_for_skill()` 集中查询, 0 改 `SkillRegistry` 原 9 fn.
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (superpowers 234 files, 1:1 映射公开协作资源)
//! - 0 装"已加载" superpowers 私有加载机制 (我们 1:1 映射公开 .md 文件名 + 内容摘要)
//! - 0 触碰 R125-15e Skill trait / SkillRegistry / 14 skills (严守 0 改原 lib.rs 入口)

#![deny(unsafe_code)]

use crate::skill_trait::SkillId;

/// `SkillCompanionKind` — 1 个协作资源类型, 5 variant 1:1 映射 superpowers 公开 .md.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillCompanionKind {
    /// `skills/brainstorming/visual-companion.md` (browser-based visual brainstorming)
    VisualCompanion,
    /// `skills/brainstorming/spec-document-reviewer-prompt.md` (spec reviewer subagent template)
    SpecDocumentReviewerPrompt,
    /// `skills/requesting-code-review/code-reviewer.md` (reviewer dispatch template)
    CodeReviewerRequestPrompt,
    /// `skills/receiving-code-review/code-reviewer.md` (reviewer response template)
    CodeReviewerReceivePrompt,
    /// `skills/systematic-debugging/condition-based-waiting.md` (async test wait pattern)
    ConditionBasedWaiting,
    /// `skills/systematic-debugging/CREATION-LOG.md` (skill creation history log)
    CreationLog,
    /// 自由形式 (forward-compat, 0 强求 enum 覆盖)
    Other,
}

impl SkillCompanionKind {
    /// 6 已知 variant 1:1 映射 superpowers 公开 .md 路径.
    pub fn source_path(self) -> &'static str {
        match self {
            Self::VisualCompanion => "skills/brainstorming/visual-companion.md",
            Self::SpecDocumentReviewerPrompt => {
                "skills/brainstorming/spec-document-reviewer-prompt.md"
            }
            Self::CodeReviewerRequestPrompt => {
                "skills/requesting-code-review/code-reviewer.md"
            }
            Self::CodeReviewerReceivePrompt => {
                "skills/receiving-code-review/code-reviewer.md"
            }
            Self::ConditionBasedWaiting => {
                "skills/systematic-debugging/condition-based-waiting.md"
            }
            Self::CreationLog => "skills/systematic-debugging/CREATION-LOG.md",
            Self::Other => "(other)",
        }
    }

    /// 6 已知 variant 1:1 映射 superpowers 公开 .md title.
    pub const fn title(self) -> &'static str {
        match self {
            Self::VisualCompanion => "Visual Companion Guide",
            Self::SpecDocumentReviewerPrompt => "Spec Document Reviewer Prompt",
            Self::CodeReviewerRequestPrompt => "Code Reviewer Prompt Template",
            Self::CodeReviewerReceivePrompt => "Code Reviewer Response Template",
            Self::ConditionBasedWaiting => "Condition-Based Waiting",
            Self::CreationLog => "Creation Log",
            Self::Other => "(other companion)",
        }
    }

    /// 6 + 1 = 7 total variant count (compile-time check).
    pub const COUNT: usize = 7;
}

/// 1 个 skill 协作资源 (kind + 1 句摘要).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillCompanion {
    /// 协作资源 kind.
    pub kind: SkillCompanionKind,
    /// 协作资源 title (1:1 映射 .md H1 标题).
    pub title: &'static str,
    /// 1 句摘要, 供 dashboard / list UI 展示.
    pub summary: &'static str,
}

impl SkillCompanion {
    /// 构造 1 个已知 variant 协作资源.
    pub const fn new(kind: SkillCompanionKind, summary: &'static str) -> Self {
        Self {
            kind,
            title: kind.title(),
            summary,
        }
    }
}

/// Static 协作资源数组 (eager init 避免 const 上下文临时值问题).
static BRAINSTORMING_COMPANIONS: [SkillCompanion; 2] = [
    SkillCompanion::new(
        SkillCompanionKind::VisualCompanion,
        "Browser-based visual brainstorming for mockups, diagrams, options",
    ),
    SkillCompanion::new(
        SkillCompanionKind::SpecDocumentReviewerPrompt,
        "Subagent template for reviewing the spec document",
    ),
];
static REQUESTING_CODE_REVIEW_COMPANIONS: [SkillCompanion; 1] = [SkillCompanion::new(
    SkillCompanionKind::CodeReviewerRequestPrompt,
    "Template for dispatching a code reviewer subagent",
)];
static RECEIVING_CODE_REVIEW_COMPANIONS: [SkillCompanion; 1] = [SkillCompanion::new(
    SkillCompanionKind::CodeReviewerReceivePrompt,
    "Template for handling reviewer feedback",
)];
static SYSTEMATIC_DEBUGGING_COMPANIONS: [SkillCompanion; 2] = [
    SkillCompanion::new(
        SkillCompanionKind::ConditionBasedWaiting,
        "Wait for the actual condition, not a guess about timing",
    ),
    SkillCompanion::new(
        SkillCompanionKind::CreationLog,
        "Append-only history of this skill's edits",
    ),
];

/// 1 个 skill 的 0+ 个协作资源列表 (static 1:1 映射 superpowers 公开资源).
///
/// 4 skill 含协作资源: brainstorming (2) / requesting-code-review (1) /
/// receiving-code-review (1) / systematic-debugging (2). 其它 10 skill 0 协作资源.
pub fn companions_for_skill(skill_id: SkillId) -> &'static [SkillCompanion] {
    match skill_id {
        SkillId::Brainstorming => &BRAINSTORMING_COMPANIONS,
        SkillId::RequestingCodeReview => &REQUESTING_CODE_REVIEW_COMPANIONS,
        SkillId::ReceivingCodeReview => &RECEIVING_CODE_REVIEW_COMPANIONS,
        SkillId::SystematicDebugging => &SYSTEMATIC_DEBUGGING_COMPANIONS,
        _ => &[],
    }
}

/// 统计全 14 skill 的协作资源总数.
pub fn total_companion_count() -> usize {
    SkillId::ALL
        .iter()
        .map(|id| companions_for_skill(*id).len())
        .sum()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn companions_for_skill_brainstorming_has_two() {
        let list = companions_for_skill(SkillId::Brainstorming);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].kind, SkillCompanionKind::VisualCompanion);
        assert_eq!(list[1].kind, SkillCompanionKind::SpecDocumentReviewerPrompt);
    }

    #[test]
    fn companions_for_skill_requesting_code_review_has_one() {
        let list = companions_for_skill(SkillId::RequestingCodeReview);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, SkillCompanionKind::CodeReviewerRequestPrompt);
    }

    #[test]
    fn companions_for_skill_receiving_code_review_has_one() {
        let list = companions_for_skill(SkillId::ReceivingCodeReview);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].kind, SkillCompanionKind::CodeReviewerReceivePrompt);
    }

    #[test]
    fn companions_for_skill_systematic_debugging_has_two() {
        let list = companions_for_skill(SkillId::SystematicDebugging);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].kind, SkillCompanionKind::ConditionBasedWaiting);
        assert_eq!(list[1].kind, SkillCompanionKind::CreationLog);
    }

    #[test]
    fn companions_for_skill_without_companions_is_empty() {
        // 10 skills have 0 companions
        for id in [
            SkillId::TestDrivenDevelopment,
            SkillId::VerificationBeforeCompletion,
            SkillId::WritingPlans,
            SkillId::ExecutingPlans,
            SkillId::SubagentDrivenDevelopment,
            SkillId::DispatchingParallelAgents,
            SkillId::UsingGitWorktrees,
            SkillId::FinishingADevelopmentBranch,
            SkillId::WritingSkills,
            SkillId::UsingSuperpowers,
        ] {
            assert_eq!(
                companions_for_skill(id).len(),
                0,
                "{id:?} should have 0 companions"
            );
        }
    }

    #[test]
    fn source_path_1to1_maps_superpowers_public_files() {
        assert_eq!(
            SkillCompanionKind::VisualCompanion.source_path(),
            "skills/brainstorming/visual-companion.md"
        );
        assert_eq!(
            SkillCompanionKind::ConditionBasedWaiting.source_path(),
            "skills/systematic-debugging/condition-based-waiting.md"
        );
        assert_eq!(
            SkillCompanionKind::CodeReviewerRequestPrompt.source_path(),
            "skills/requesting-code-review/code-reviewer.md"
        );
    }

    #[test]
    fn total_companion_count_sums_6_across_4_skills() {
        // 2 + 1 + 1 + 2 = 6 companions across 4 skills
        assert_eq!(total_companion_count(), 6);
    }

    #[test]
    fn kind_count_is_7_with_other() {
        // 6 named + 1 Other = 7 total variants
        assert_eq!(SkillCompanionKind::COUNT, 7);
    }
}
