//! `SkillPrompt` — Skill prompt 渲染 (R125-18 升级)
//!
//! # 借鉴 ID
//!
//! `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10` (per 决策 #51 §1.4 P3-1 + R125-18 decision-log)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! 借鉴模式: `.opencode/plugins/superpowers.js` `getBootstrapContent` (lines 62-100) +
//!           `<EXTREMELY_IMPORTANT>` 包装 (line 89) +
//!           module-level cache `_bootstrapCache` (line 53) +
//!           tool mapping 段 (lines 76-87)
//! clone 状态: ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1)
//!
//! # 核心
//!
//! 1 个 skill 可被渲染为 1 个完整 agent prompt, 3 段拼接:
//! 1. **Header** — `<EXTREMELY_IMPORTANT>\nYou have superpowers. ...` (1:1 借鉴)
//! 2. **Body** — skill markdown body (frontmatter 1:1 stripped)
//! 3. **Footer** — 工具映射段 (apeireth 工具名 1:1 替代 superpowers OpenCode 工具名)
//!
//! 借鉴 superpowers `_bootstrapCache` 模式: 1 session 1 缓存, 0 重复 parse 14 SKILL.md.
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (1:1 映射 superpowers 公开 bootstrap + cache + tool mapping)
//! - 0 装"已注入" 私有 LLM system message (我们仅 render to String, 0 触碰 apeireth LLM gateway)
//! - 0 触碰 R125-15e Skill trait (仅消费 trait 方法)

#![deny(unsafe_code)]

use crate::skill_trait::{Skill, SkillId, SkillStep};
use std::fmt;

/// `<EXTREMELY_IMPORTANT>` marker (1:1 映射 superpowers `.opencode/plugins/superpowers.js` line 89).
pub const EXTREMELY_IMPORTANT_MARKER: &str = "<EXTREMELY_IMPORTANT>";

/// Bootstrap marker 标识 (1:1 映射 superpowers `.pi/extensions/superpowers.ts` line 7).
pub const BOOTSTRAP_MARKER: &str = "apeireth-skill-bootstrap";

/// Skill prompt 完整结构 (header + body + footer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillPrompt {
    /// Header 段 (`<EXTREMELY_IMPORTANT>\nYou have superpowers. ...`).
    pub header: String,
    /// Body 段 (skill markdown body, frontmatter 1:1 stripped).
    pub body: String,
    /// Footer 段 (工具映射).
    pub footer: String,
}

impl SkillPrompt {
    /// 1:1 映射 superpowers `getBootstrapContent` — 渲染 1 个 skill 的完整 prompt.
    ///
    /// # Args
    /// - `skill` — 1 个 skill trait obj
    /// - `tool_mapping` — 工具映射段 (调用者按 harness 适配, e.g. apeireth TUI / cli / api)
    pub fn render(skill: &dyn Skill, tool_mapping: &str) -> Self {
        let header = format!(
            "{marker}\n{bootstrap}\n\nYou have superpowers.\n\n**IMPORTANT: The {name} skill content is included below. It is ALREADY LOADED - you are currently following it. Do NOT use the skill tool to load \"{kebab}\" again - that would be redundant.**\n",
            marker = EXTREMELY_IMPORTANT_MARKER,
            bootstrap = BOOTSTRAP_MARKER,
            name = skill.name(),
            kebab = skill.id().kebab_name(),
        );
        let body = render_skill_body(skill);
        let footer = format!(
            "\n{tool_mapping}\n",
        );
        Self {
            header,
            body,
            footer,
        }
    }

    /// 完整 prompt 1 字符串拼接 (header + body + footer).
    pub fn to_full_string(&self) -> String {
        let mut s = String::with_capacity(
            self.header.len() + self.body.len() + self.footer.len() + 4,
        );
        s.push_str(&self.header);
        s.push_str("\n");
        s.push_str(&self.body);
        s.push_str(&self.footer);
        s.push_str(EXTREMELY_IMPORTANT_MARKER);
        s
    }
}

impl fmt::Display for SkillPrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_full_string())
    }
}

/// 渲染 skill body 段 (steps + frontmatter 1:1 stripped markdown 描述).
///
/// 借鉴 superpowers 公开 SKILL.md body, 但 0 装"已读私有" — 仅使用 Skill trait 公开 fn.
pub fn render_skill_body(skill: &dyn Skill) -> String {
    let mut s = String::new();
    s.push_str(&format!("# {}\n\n", skill.name()));
    s.push_str(&format!("> {}\n\n", skill.when_to_use()));
    s.push_str("## Steps\n\n");
    for step in skill.steps() {
        let marker = if step.is_tdd_red { "[RED] " } else { "" };
        s.push_str(&format!(
            "{}. {}{}\n",
            step.order, marker, step.description
        ));
    }
    s
}

/// Apeireth 工具映射段 (1:1 替代 superpowers OpenCode 工具映射).
///
/// 借鉴 superpowers `.opencode/plugins/superpowers.js` lines 76-87 模式, 适配 apeireth 工具集.
pub fn apeireth_tool_mapping() -> &'static str {
    r#"**Tool Mapping for Apeireth:**

When skills request actions, substitute Apeireth equivalents:
- Create or update todos → use `apeireth-tui` plan mode
- `Subagent (general-purpose):` → dispatch via Mavis root with sub-agent role
- Invoke a skill → use `apeireth-central::SkillRegistry::get(id)` 
- Read files → use `read` tool with absolute path
- Create, edit, or delete files → use `edit` / `write` tool
- Run shell commands → use `bash` (PowerShell on Windows)
- Search files → use `grep` / `glob` tool
- Fetch a URL → use `web_fetch` tool

Use `apeireth-central::SkillRegistry` to discover and load skills."#
}

/// 渲染 1 个 skill 的 steps 段 (简洁版, 0 含 frontmatter).
pub fn render_steps(skill: &dyn Skill) -> String {
    let steps = skill.steps();
    let mut s = String::new();
    for step in steps {
        let marker = if step.is_tdd_red { "🔴 " } else { "  " };
        s.push_str(&format!(
            "{}{}. {}\n",
            marker, step.order, step.description
        ));
    }
    s
}

/// 统计 1 个 skill prompt 的 TDD red 步骤数 (借鉴 superpowers TDD iron law).
pub fn count_tdd_red_steps(skill: &dyn Skill) -> usize {
    skill.steps().iter().filter(|s| s.is_tdd_red).count()
}

/// 1 个 skill prompt 缓存 (借鉴 superpowers `_bootstrapCache` 模式).
///
/// 每 session 1 缓存, `cache_get_or_init` 首次调用 render + 缓存, 后续 O(1) 命中.
#[derive(Debug, Default)]
pub struct SkillPromptCache {
    cached: std::sync::Mutex<
        std::collections::BTreeMap<SkillId, SkillPrompt>,
    >,
}

impl SkillPromptCache {
    /// 创建 1 个空缓存.
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取或渲染 (1 session 1 缓存, 借鉴 superpowers `_bootstrapCache`).
    pub fn get_or_render(
        &self,
        skill_id: SkillId,
        skill: &dyn Skill,
        tool_mapping: &str,
    ) -> SkillPrompt {
        let mut cache = self.cached.lock().expect("cache lock");
        cache
            .entry(skill_id)
            .or_insert_with(|| SkillPrompt::render(skill, tool_mapping))
            .clone()
    }

    /// 缓存大小 (0..=14).
    pub fn len(&self) -> usize {
        self.cached.lock().expect("cache lock").len()
    }

    /// 缓存是否为空.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 清空缓存.
    pub fn clear(&self) {
        self.cached.lock().expect("cache lock").clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill_trait::{BrainstormingSkill, TestDrivenDevelopmentSkill};

    #[test]
    fn prompt_contains_extremely_important_marker() {
        let skill = BrainstormingSkill;
        let prompt = SkillPrompt::render(&skill, "");
        assert!(prompt.header.contains(EXTREMELY_IMPORTANT_MARKER));
        assert!(prompt.header.contains(BOOTSTRAP_MARKER));
    }

    #[test]
    fn prompt_body_contains_skill_name_and_steps() {
        let skill = TestDrivenDevelopmentSkill;
        let prompt = SkillPrompt::render(&skill, "");
        assert!(prompt.body.contains("Test-Driven Development"));
        assert!(prompt.body.contains("1. "));
        assert!(prompt.body.contains("[RED]"));
    }

    #[test]
    fn prompt_full_string_wraps_with_marker() {
        let skill = BrainstormingSkill;
        let prompt = SkillPrompt::render(&skill, "");
        let full = prompt.to_full_string();
        assert!(full.starts_with(EXTREMELY_IMPORTANT_MARKER));
        assert!(full.ends_with(EXTREMELY_IMPORTANT_MARKER));
    }

    #[test]
    fn prompt_kebab_name_appears_in_header() {
        let skill = BrainstormingSkill;
        let prompt = SkillPrompt::render(&skill, "");
        assert!(prompt.header.contains("brainstorming"));
    }

    #[test]
    fn tool_mapping_contains_apeireth_equivalents() {
        let mapping = apeireth_tool_mapping();
        assert!(mapping.contains("Apeireth"));
        assert!(mapping.contains("SkillRegistry"));
        assert!(mapping.contains("apeireth-tui"));
    }

    #[test]
    fn render_steps_marks_tdd_red() {
        let skill = TestDrivenDevelopmentSkill;
        let rendered = render_steps(&skill);
        assert!(rendered.contains("🔴 1."));
        assert!(rendered.contains("2."));
    }

    #[test]
    fn count_tdd_red_steps_for_tdd_skill_is_one() {
        let skill = TestDrivenDevelopmentSkill;
        assert_eq!(count_tdd_red_steps(&skill), 1);
    }

    #[test]
    fn count_tdd_red_steps_for_meta_skill_is_zero() {
        let skill = crate::skill_trait::UsingSuperpowersSkill;
        assert_eq!(count_tdd_red_steps(&skill), 0);
    }

    #[test]
    fn prompt_cache_caches_first_render() {
        let cache = SkillPromptCache::new();
        assert!(cache.is_empty());
        let skill = BrainstormingSkill;
        let prompt = cache.get_or_render(SkillId::Brainstorming, &skill, "");
        assert!(prompt.header.contains(EXTREMELY_IMPORTANT_MARKER));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn prompt_cache_repeated_get_returns_same() {
        let cache = SkillPromptCache::new();
        let skill = BrainstormingSkill;
        let p1 = cache.get_or_render(SkillId::Brainstorming, &skill, "");
        let p2 = cache.get_or_render(SkillId::Brainstorming, &skill, "");
        assert_eq!(p1, p2);
        assert_eq!(cache.len(), 1, "cache should not duplicate on repeated get");
    }

    #[test]
    fn prompt_cache_clear_empties_cache() {
        let cache = SkillPromptCache::new();
        cache.get_or_render(SkillId::Brainstorming, &BrainstormingSkill, "");
        assert_eq!(cache.len(), 1);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn prompt_display_impl_equals_full_string() {
        let skill = BrainstormingSkill;
        let prompt = SkillPrompt::render(&skill, "tool-mapping");
        assert_eq!(format!("{prompt}"), prompt.to_full_string());
    }

    #[test]
    fn prompt_footer_contains_tool_mapping() {
        let skill = BrainstormingSkill;
        let prompt = SkillPrompt::render(&skill, "CUSTOM TOOL MAPPING");
        assert!(prompt.footer.contains("CUSTOM TOOL MAPPING"));
    }
}
