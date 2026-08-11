//! `SkillRecommender` — 14 Skill 关键词自动推荐 (R125-16 升级, recommender 层)
//!
//! # 借鉴 ID
//!
//! `R125-16-BORROW-obra/superpowers-2026-05-2026-08-10` (per 决策 #36 §1.1 + 决策 #51 §1.1
//! + 决策 #52)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! clone 状态: ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1)
//!
//! # 核心模式
//!
//! superpowers 公开 README §"How it works" + §"The Basic Workflow":
//!
//! > "It starts from the moment you fire up your coding agent. As soon as it sees that
//! >  you're building something, it *doesn't* just jump into trying to write code.
//! >  Instead, it steps back and asks you what you're really trying to do."
//!
//! > "**The agent checks for relevant skills before any task. Mandatory workflows,
//! >  not suggestions.**"
//!
//! 借鉴到 apeireth-central 形成 `SkillRecommender`:
//!
//! 1. **`new(registry)`** — 跟 R125-15e `SkillRegistry` 1:1 配合
//! 2. **`recommend(task_description, top_n)`** — 根据 task description 关键词匹配,
//!    返回 top N 个相关 skill (含匹配分数 + 排序)
//! 3. **`recommend_with_threshold(task_description, threshold)`** — 过滤低分匹配
//! 4. **`skill_keywords(skill_id)`** — 查 1 个 skill 的关键词列表
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (14 skill 关键词 mapping + 4 fn + 8 test, 0 跟 R125-15e / R125-18
//!   / R125-19 冲突)
//! - 借鉴字段: 14 skill 关键词 mapping 1:1 映射 superpowers 公开 SKILL.md
//!   (name/description/when_to_use) 4 段结构 关键词
//! - 0 假装"已借鉴" superpowers 私有 plugin 加载机制 (`.claude-plugin/` etc.)
//!
//! # 跟 R125-15e / R125-18 / R125-19 配合 (0 重复造轮子严守)
//!
//! - R125-15e 写 `Skill` trait + `SkillRegistry` + 14 Skill struct impl — R125-16 用
//!   `&SkillRegistry` 查 skill, 0 重写
//! - R125-18 写 5 mod (skill_execution / skill_prompt / skill_validation / skill_companion
//!   / skill_frontmatter) — R125-16 0 触碰
//! - R125-19 写 `apeireth-skills::skill_executor` (5 phase state machine) — R125-16 0 触碰
//!   (R125-19 在 apeireth-skills crate, R125-16 在 apeireth-central crate)
//! - R125-16 只加 1 NEW src 文件 (recommender 层) + 1 NEW test + 1 NEW example

#![deny(unsafe_code)]

use crate::skill_registry::SkillRegistry;
use crate::skill_trait::SkillId;

/// `SkillRecommender` — 14 Skill 关键词自动推荐, 跟 R125-15e `SkillRegistry` 1:1 配合.
///
/// 0 拥有 registry (跟 R125-15e 共享 14 Skill impl).
pub struct SkillRecommender<'a> {
    registry: &'a SkillRegistry,
}

impl<'a> SkillRecommender<'a> {
    /// 创建 1 个跟 R125-15e `SkillRegistry` 1:1 配合的 recommender.
    pub fn new(registry: &'a SkillRegistry) -> Self {
        Self { registry }
    }

    /// Registry reference (只读).
    pub fn registry(&self) -> &SkillRegistry {
        self.registry
    }

    /// 1 个 skill 的关键词列表 (编译期 hardcode, 1:1 映射 superpowers 公开 SKILL.md).
    ///
    /// 关键词来源: superpowers `skills/<name>/SKILL.md` 的 name + description + when_to_use
    /// 字段 (frontmatter 1:1) + body 中高频关键词. 0 装"已读私有", 仅用 14 SKILL.md 公开
    /// 内容 1:1 映射.
    pub fn skill_keywords(skill_id: SkillId) -> &'static [&'static str] {
        match skill_id {
            SkillId::Brainstorming => &[
                "brainstorm", "spec", "design", "idea", "explore", "alternative",
                "intent", "clarify", "validate",
            ],
            SkillId::TestDrivenDevelopment => &[
                "test", "tdd", "red", "green", "refactor", "failing", "first",
                "iron law", "no production",
            ],
            SkillId::SystematicDebugging => &[
                "debug", "bug", "fix", "root cause", "defense in depth", "regression",
                "systematic", "reproduce",
            ],
            SkillId::VerificationBeforeCompletion => &[
                "verify", "validate", "complete", "done", "test suite", "clippy",
                "cargo", "evidence",
            ],
            SkillId::WritingPlans => &[
                "plan", "writing", "task", "junior engineer", "15 min", "dependency",
                "implementation",
            ],
            SkillId::ExecutingPlans => &[
                "execute", "plan", "tdd", "verify", "task", "iterate", "checkpoint",
            ],
            SkillId::SubagentDrivenDevelopment => &[
                "subagent", "dispatch", "parallel", "iterate", "verify", "task",
                "fresh",
            ],
            SkillId::DispatchingParallelAgents => &[
                "parallel", "dispatch", "concurrent", "independent", "merge", "task",
            ],
            SkillId::RequestingCodeReview => &[
                "review", "code review", "submit", "feedback", "non-trivial", "diff",
                "human",
            ],
            SkillId::ReceivingCodeReview => &[
                "review", "feedback", "respond", "push back", "fix", "re-request",
            ],
            SkillId::UsingGitWorktrees => &[
                "worktree", "git", "branch", "parallel", "isolation", "subagent",
                "merge",
            ],
            SkillId::FinishingADevelopmentBranch => &[
                "finish", "merge", "pr", "pull request", "worktree", "discard",
                "complete",
            ],
            SkillId::WritingSkills => &[
                "writing skills", "create skill", "best practice", "test", "behavior",
            ],
            SkillId::UsingSuperpowers => &[
                "using superpowers", "intro", "meta", "skills", "system",
            ],
        }
    }

    /// 计算 1 个 skill 在 task description 中的匹配分数 (0-100).
    ///
    /// 算法: 匹配关键词数 / 总关键词数 * 100. 0 关键词匹配 = 0 分.
    /// 匹配是 case-insensitive 的, 关键词在 task description 中作为子串出现.
    pub fn score_skill(skill_id: SkillId, task_description: &str) -> u32 {
        let keywords = Self::skill_keywords(skill_id);
        if keywords.is_empty() {
            return 0;
        }
        let task_lower = task_description.to_lowercase();
        let matched = keywords
            .iter()
            .filter(|kw| task_lower.contains(&kw.to_lowercase()))
            .count();
        ((matched as f64 / keywords.len() as f64) * 100.0) as u32
    }

    /// 推荐 top N 个相关 skill, 按分数排序 (从高到低).
    ///
    /// `top_n = 0` 表示返回全部 (但仅返 >0 分的).
    pub fn recommend(
        &self,
        task_description: &str,
        top_n: usize,
    ) -> Vec<ScoredSkill> {
        let mut scored: Vec<ScoredSkill> = SkillId::ALL
            .iter()
            .map(|&id| {
                let score = Self::score_skill(id, task_description);
                let matched: Vec<&'static str> = Self::skill_keywords(id)
                    .iter()
                    .copied()
                    .filter(|kw| task_description.to_lowercase().contains(&kw.to_lowercase()))
                    .collect();
                ScoredSkill {
                    skill_id: id,
                    score,
                    matched_keywords: matched,
                }
            })
            .filter(|s| s.score > 0)
            .collect();
        // 排序: 分数从高到低
        scored.sort_by(|a, b| b.score.cmp(&a.score).then(a.skill_id.cmp(&b.skill_id)));
        if top_n == 0 {
            scored
        } else {
            scored.into_iter().take(top_n).collect()
        }
    }

    /// 推荐 ≥ threshold 分的 skill (0-100).
    pub fn recommend_with_threshold(
        &self,
        task_description: &str,
        threshold: u32,
    ) -> Vec<ScoredSkill> {
        self.recommend(task_description, 0)
            .into_iter()
            .filter(|s| s.score >= threshold)
            .collect()
    }

    /// 14 skill 全部关键词总数 (compile-time sanity check).
    pub fn total_keywords() -> usize {
        SkillId::ALL
            .iter()
            .map(|id| Self::skill_keywords(*id).len())
            .sum()
    }
}

/// `ScoredSkill` — 单个推荐结果 (skill_id + 匹配分数 + 匹配的关键词).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoredSkill {
    /// Skill id.
    pub skill_id: SkillId,
    /// 匹配分数 (0-100, 0 表示 0 匹配).
    pub score: u32,
    /// 匹配的关键词列表 (从 skill 的关键词列表中筛出 task 包含的).
    pub matched_keywords: Vec<&'static str>,
}

// ============================================================================
// 单元 tests (8 tests)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry() -> SkillRegistry {
        SkillRegistry::new()
    }

    #[test]
    fn recommend_tdd_skill_for_test_keywords() {
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        let recs = rec.recommend("I need to write a failing test first", 3);
        // TDD skill 应排第 1
        assert!(!recs.is_empty());
        assert_eq!(recs[0].skill_id, SkillId::TestDrivenDevelopment);
        assert!(recs[0].score > 0);
        // TDD 关键词 "test", "failing", "first" 都匹配
        assert!(recs[0].matched_keywords.contains(&"test"));
        assert!(recs[0].matched_keywords.contains(&"failing"));
        assert!(recs[0].matched_keywords.contains(&"first"));
    }

    #[test]
    fn recommend_brainstorming_for_spec_keywords() {
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        let recs = rec.recommend("Let me brainstorm the spec design alternatives", 3);
        assert!(!recs.is_empty());
        assert_eq!(recs[0].skill_id, SkillId::Brainstorming);
        assert!(recs[0].score > 0);
        assert!(recs[0].matched_keywords.contains(&"brainstorm"));
        assert!(recs[0].matched_keywords.contains(&"spec"));
    }

    #[test]
    fn recommend_empty_for_no_match() {
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        let recs = rec.recommend("xyzqwertynonsense", 3);
        // 0 匹配 = 空
        assert!(recs.is_empty());
    }

    #[test]
    fn recommend_top_n_limits_results() {
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        let recs = rec.recommend("test plan verify review", 2);
        // top 2 限制
        assert!(recs.len() <= 2);
        // 全部分数都 > 0
        for r in &recs {
            assert!(r.score > 0);
        }
    }

    #[test]
    fn recommend_sorted_by_score() {
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        let recs = rec.recommend("debug bug fix root cause", 0);
        // 全部分数从高到低
        for i in 1..recs.len() {
            assert!(recs[i - 1].score >= recs[i].score);
        }
    }

    #[test]
    fn recommend_case_insensitive() {
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        let recs1 = rec.recommend("I need to TEST this code", 3);
        let recs2 = rec.recommend("i need to test this code", 3);
        // case-insensitive 应该 1:1 匹配
        assert_eq!(recs1.len(), recs2.len());
        if !recs1.is_empty() {
            assert_eq!(recs1[0].skill_id, recs2[0].skill_id);
            assert_eq!(recs1[0].score, recs2[0].score);
        }
    }

    #[test]
    fn recommend_with_multiple_keywords_scores_higher() {
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        // TDD skill: "test" + "tdd" + "red" + "failing" + "first" = 5/9 = 55 分
        let recs1 = rec.recommend("test tdd", 1);
        // TDD skill: "test" + "tdd" + "red" + "failing" + "first" + "green" + "refactor" = 7/9 = 77 分
        let recs2 = rec.recommend("test tdd red failing first green refactor", 1);
        assert!(!recs1.is_empty() && !recs2.is_empty());
        assert!(recs2[0].score > recs1[0].score);
    }

    #[test]
    fn recommender_uses_skill_registry_1to1() {
        // R125-15e `SkillRegistry` 14 entry 严守, R125-16 `SkillRecommender` 1:1 配合
        let registry = make_registry();
        let rec = SkillRecommender::new(&registry);
        assert_eq!(rec.registry().count(), 14);
        // 14 skill 全部都有 keywords (1:1 配合)
        let total_kw = SkillRecommender::total_keywords();
        assert!(total_kw > 0);
        // 14 skill 各自 ≥ 1 关键词 (编译期 sanity check, 跟 R125-15e 14 entry 1:1)
        for id in SkillId::ALL {
            assert!(
                !SkillRecommender::skill_keywords(id).is_empty(),
                "{id:?} must have ≥1 keyword"
            );
        }
    }
}
