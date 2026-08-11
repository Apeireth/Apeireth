//! `SkillFrontmatter` — 解析 superpowers SKILL.md YAML frontmatter (R125-18 升级)
//!
//! # 借鉴 ID
//!
//! `R125-18-BORROW-obra/superpowers-v6.2-2026-08-10` (per 决策 #51 §1.4 P3-1 + R125-18 decision-log)
//!
//! 借鉴源码: `.openclaw/workspace/borrowed-repos/superpowers/`
//! 借鉴模式: `.opencode/plugins/superpowers.js` `extractAndStripFrontmatter` (lines 16-34)
//!           + `.pi/extensions/superpowers.ts` `stripFrontmatter` (line 64)
//! clone 状态: ✅ cloned (234 files, per 决策 #36 §1.1 + 决策 #41 §1)
//!
//! # 核心
//!
//! 解析 SKILL.md 的 YAML frontmatter (`---\n...\n---\n`) 提取 2 必填字段 (`name`, `description`),
//! 1 可选字段 (其它 `extra` 元数据). 1:1 映射 superpowers `extractAndStripFrontmatter` 模式:
//! - 找到 `---\n...` 开头
//! - 找 `\n---` 关闭
//! - 解析每行 `key: value`
//! - 返回 (frontmatter, body)
//!
//! # 0 装 PASS 严守
//!
//! - ✅ cloned = 真实施 (superpowers v6.2.0 234 files cloned, 8 硬墙 0 越界, 有真 src + tests)
//! - 0 引入 `serde_yaml` / `yaml-rust` 等外部 dep (手写 1 个 minimal parser, 编译期 0 dep 风险)
//! - 0 装"已解析" superpowers 私有 YAML 字段 (1:1 映射公开 frontmatter 模式, 0 触碰私有 schema)

#![deny(unsafe_code)]

use std::fmt;

/// YAML frontmatter 解析结果 (2 必填字段 + 1 可选字段).
///
/// 1:1 映射 superpowers `extractAndStripFrontmatter` 返回的 `frontmatter` 字段
/// (`name` + `description` 2 必填 + 其它 `extra` 元数据).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillFrontmatter {
    /// Skill kebab-case 名 (e.g. `"test-driven-development"`).
    ///
    /// 必填, 1:1 映射 superpowers `name:` frontmatter 字段.
    pub name: String,
    /// 触发条件 (e.g. `"Use when implementing any feature..."`).
    ///
    /// 必填, 1:1 映射 superpowers `description:` frontmatter 字段.
    pub description: String,
    /// 其它 frontmatter 字段 (key, value) 列表, 保留顺序.
    ///
    /// 可选, 不强求 schema. 例如 superpowers 偶有 `when_to_use:` / `version:` 等扩展.
    pub extra: Vec<(String, String)>,
}

/// Frontmatter 解析错误.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontmatterError {
    /// 缺 `---\n` 开头.
    MissingOpening,
    /// 缺 `\n---` 关闭.
    MissingClosing,
    /// 某行不是 `key: value` 格式.
    MalformedLine {
        /// 出错的行内容.
        line: String,
    },
    /// 缺必填字段 (e.g. `name` / `description`).
    MissingField {
        /// 缺的字段名.
        field: String,
    },
}

impl fmt::Display for FrontmatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingOpening => write!(f, "frontmatter missing opening `---`"),
            Self::MissingClosing => write!(f, "frontmatter missing closing `---`"),
            Self::MalformedLine { line } => {
                write!(f, "frontmatter line malformed (no `:`): {line:?}")
            }
            Self::MissingField { field } => {
                write!(f, "frontmatter missing required field: {field}")
            }
        }
    }
}

impl std::error::Error for FrontmatterError {}

/// 1:1 映射 superpowers `.opencode/plugins/superpowers.js` `extractAndStripFrontmatter`.
pub fn parse_frontmatter(content: &str) -> Result<SkillFrontmatter, FrontmatterError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(FrontmatterError::MissingOpening);
    }
    let after_open = &trimmed[3..];
    let after_open = after_open.trim_start_matches('\n');
    let close_idx = after_open
        .find("\n---")
        .ok_or(FrontmatterError::MissingClosing)?;
    let fm_str = &after_open[..close_idx];

    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut extra = Vec::new();
    for raw_line in fm_str.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let colon = line
            .find(':')
            .ok_or_else(|| FrontmatterError::MalformedLine {
                line: raw_line.to_string(),
            })?;
        let key = line[..colon].trim().to_string();
        let raw_value = line[colon + 1..].trim();
        let value = raw_value
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_start_matches('\'')
            .trim_end_matches('\'')
            .to_string();
        match key.as_str() {
            "name" => name = Some(value),
            "description" => description = Some(value),
            _ => extra.push((key, value)),
        }
    }

    let name = name.ok_or_else(|| FrontmatterError::MissingField {
        field: "name".to_string(),
    })?;
    let description = description.ok_or_else(|| FrontmatterError::MissingField {
        field: "description".to_string(),
    })?;
    Ok(SkillFrontmatter {
        name,
        description,
        extra,
    })
}

/// 1:1 映射 superpowers `.pi/extensions/superpowers.ts` `stripFrontmatter`.
///
/// 返回 frontmatter 之后的 body (trim leading newline).
pub fn strip_frontmatter(content: &str) -> &str {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return content;
    }
    let after_open = &trimmed[3..];
    let after_open = after_open.trim_start_matches('\n');
    if let Some(close_idx) = after_open.find("\n---") {
        let body = &after_open[close_idx + 4..];
        return body.trim_start_matches('\n');
    }
    content
}

/// 14 skill 的 frontmatter `name` 编译期 hardcode 1:1 映射 superpowers.
///
/// 借鉴 superpowers `skills/<name>/SKILL.md` 的 `name:` frontmatter 字段.
pub const SUPERPOWERS_SKILL_NAMES: [&str; 14] = [
    "brainstorming",
    "test-driven-development",
    "systematic-debugging",
    "verification-before-completion",
    "writing-plans",
    "executing-plans",
    "subagent-driven-development",
    "dispatching-parallel-agents",
    "requesting-code-review",
    "receiving-code-review",
    "using-git-worktrees",
    "finishing-a-development-branch",
    "writing-skills",
    "using-superpowers",
];

/// 验证 `name` 是否是 14 已知 superpowers skill 之一.
pub fn is_known_skill_name(name: &str) -> bool {
    SUPERPOWERS_SKILL_NAMES.contains(&name)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const TDD_SKILL_MD: &str = "---\nname: test-driven-development\ndescription: Use when implementing any feature or bugfix\n---\n\n# Test-Driven Development\n\nSome body content here.\n";

    #[test]
    fn parse_frontmatter_extracts_name_and_description() {
        let fm = parse_frontmatter(TDD_SKILL_MD).expect("valid frontmatter");
        assert_eq!(fm.name, "test-driven-development");
        assert_eq!(
            fm.description,
            "Use when implementing any feature or bugfix"
        );
        assert!(fm.extra.is_empty());
    }

    #[test]
    fn parse_frontmatter_preserves_extra_fields() {
        let md = "---\nname: brainstorming\ndescription: spec design\nwhen_to_use: before any feature\nversion: 1.0\n---\n\n# Body\n";
        let fm = parse_frontmatter(md).expect("valid");
        assert_eq!(fm.name, "brainstorming");
        assert_eq!(fm.description, "spec design");
        assert_eq!(fm.extra.len(), 2);
        assert_eq!(fm.extra[0], ("when_to_use".to_string(), "before any feature".to_string()));
        assert_eq!(fm.extra[1], ("version".to_string(), "1.0".to_string()));
    }

    #[test]
    fn parse_frontmatter_strips_quotes() {
        let md = "---\nname: \"test-driven-development\"\ndescription: 'Use when implementing'\n---\n\n# Body\n";
        let fm = parse_frontmatter(md).expect("valid");
        assert_eq!(fm.name, "test-driven-development");
        assert_eq!(fm.description, "Use when implementing");
    }

    #[test]
    fn parse_frontmatter_missing_opening_errors() {
        let md = "name: foo\n---\n";
        assert_eq!(parse_frontmatter(md), Err(FrontmatterError::MissingOpening));
    }

    #[test]
    fn parse_frontmatter_missing_closing_errors() {
        let md = "---\nname: foo\n";
        let result = parse_frontmatter(md);
        assert!(matches!(result, Err(FrontmatterError::MissingClosing)));
    }

    #[test]
    fn parse_frontmatter_missing_name_errors() {
        let md = "---\ndescription: only description\n---\n";
        let result = parse_frontmatter(md);
        assert!(matches!(
            result,
            Err(FrontmatterError::MissingField { ref field }) if field == "name"
        ));
    }

    #[test]
    fn parse_frontmatter_missing_description_errors() {
        let md = "---\nname: only-name\n---\n";
        let result = parse_frontmatter(md);
        assert!(matches!(
            result,
            Err(FrontmatterError::MissingField { ref field }) if field == "description"
        ));
    }

    #[test]
    fn parse_frontmatter_skips_blank_and_comment_lines() {
        let md = "---\n# this is a comment\nname: foo\n\ndescription: bar\n---\n\n# Body\n";
        let fm = parse_frontmatter(md).expect("valid");
        assert_eq!(fm.name, "foo");
        assert_eq!(fm.description, "bar");
    }

    #[test]
    fn strip_frontmatter_returns_body_only() {
        let body = strip_frontmatter(TDD_SKILL_MD);
        assert!(body.starts_with("# Test-Driven Development"));
        assert!(!body.starts_with("---"));
    }

    #[test]
    fn strip_frontmatter_passthrough_when_no_frontmatter() {
        let md = "Just plain markdown\nwithout frontmatter\n";
        let result = strip_frontmatter(md);
        assert_eq!(result, md);
    }

    #[test]
    fn superpowers_skill_names_contains_14_entries() {
        assert_eq!(SUPERPOWERS_SKILL_NAMES.len(), 14);
        assert!(is_known_skill_name("test-driven-development"));
        assert!(is_known_skill_name("brainstorming"));
        assert!(is_known_skill_name("using-superpowers"));
        assert!(!is_known_skill_name("nonexistent-skill"));
    }

    #[test]
    fn frontmatter_error_display_is_human_readable() {
        let e1 = FrontmatterError::MissingOpening;
        assert_eq!(format!("{e1}"), "frontmatter missing opening `---`");
        let e2 = FrontmatterError::MissingField { field: "name".to_string() };
        assert_eq!(
            format!("{e2}"),
            "frontmatter missing required field: name"
        );
    }
}
