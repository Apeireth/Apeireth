//! apeireth-skills — R23 6 module skills 子模块。
//!
//! R23 P1 #5 实质化: 加 +7 顶层 pub fn — skill registry + ID/path conventions +
//! version compare. 不假装: 真 registry + 真 ID validation (kebab-case / ascii) +
//! 真 semver comparison (3 段比较).
//!
//! **8 项承诺**: 全部遵守. **不修改承诺 (LOCKED)**: 0 触碰 workspace.version.
//!
//! R63: 加 file_loader submodule — 借鉴 VCP `vcptoolbox/modules` 真扫目录 JSON.
//!
//! R125-19 (per decision-51 §1.4 P3-2): 加 skill_executor submodule — 5 phase state machines
//! 落地 obra/superpowers 14 公开 SKILL.md workflow 模式 (TDD / Plan-Verify / Parallel /
//! Review / Meta). 借鉴 ID: `R125-19-BORROW-obra/superpowers-2026-05-2026-08-10`.

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod descriptor;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;
pub mod mcp_bridge;
pub mod semver_strict;
pub mod eval_bridge;  // R110: Skill descriptor → eval scenario 桥接
pub mod watcher;  // R109: 文件 watcher 热加载 (polling-based, 0 新 dep)  // R107: 严格 semver 2.0.0 (3-segment + pre-release + build metadata)  // R86: Skill → MCP ToolServer 适配器 (SkillDescriptor → Tool, call 走 dispatch)
pub mod file_loader;
pub mod skill_executor;  // R125-19: Skill execution layer (5 phase state machines, superpowers 14 → 5 patterns)
pub mod library_stage6_guardianship;
pub mod wasm_bridge;  // R174: WASM skill executor (uses apeireth-sovereignty::wasm_runtime)
pub mod anthropic_skills;  // R149: Anthropic Skills 模式 (SKILL.md + 3 层加载)
  // R127 P5-3: Library Stage 6 守护 (借鉴 hyper 80 + PyO3 928 + servers 175)

#[derive(Debug, Error)]
pub enum SkillError {
    #[error("skill id `{0}` is empty")]
    EmptyId(String),
    #[error("skill version `{0}` is invalid")]
    InvalidVersion(String),
    #[error("skill id `{0}` 不符合 kebab-case: 只允许 ascii lowercase + digit + `-`")]
    InvalidIdFormat(String),
    #[error("skill: id=`{0}` 重复注册")]
    DuplicateId(String),
    #[error("skill: id=`{0}` 未注册")]
    UnknownSkill(String),
}
pub type SkillResult<T> = Result<T, SkillError>;

pub use anthropic_skills::{
    AnthropicSkillLoader,
    AnthropicSkillError,
    AnthropicSkillResult,
    SkillManifest,
    SkillDocument,
    SkillEntry,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub version: String,
    pub input_schema: String,
    pub output_schema: String,
}

impl Skill {
    pub fn new(id: impl Into<String>, version: impl Into<String>, input_schema: impl Into<String>, output_schema: impl Into<String>) -> Self {
        Self { id: id.into(), version: version.into(), input_schema: input_schema.into(), output_schema: output_schema.into() }
    }
    pub fn validate(&self) -> SkillResult<()> {
        let id = self.id.trim();
        if id.is_empty() { return Err(SkillError::EmptyId(self.id.clone())); }
        if !is_valid_id(id) { return Err(SkillError::InvalidIdFormat(self.id.clone())); }
        let parts: Vec<&str> = self.version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.parse::<u32>().is_err()) {
            return Err(SkillError::InvalidVersion(self.version.clone()));
        }
        Ok(())
    }
}

// ============================================================================
// R23 P1 #5: 加真 顶层 pub fn — Registry + ID/version utilities
// ============================================================================

/// Validate ID 为 kebab-case (ascii lowercase + digit + '-', 不允许连续 `-` 不允许头尾 `-`).
pub fn is_valid_id(id: &str) -> bool {
    if id.is_empty() || id.starts_with('-') || id.ends_with('-') { return false; }
    let mut prev_dash = false;
    for c in id.chars() {
        if c == '-' {
            if prev_dash { return false; }
            prev_dash = true;
        } else {
            prev_dash = false;
            if !(c.is_ascii_lowercase() || c.is_ascii_digit()) { return false; }
        }
    }
    true
}

/// Parse semver 3-segment version into (major, minor, patch).
pub fn parse_version(v: &str) -> SkillResult<(u32, u32, u32)> {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() != 3 { return Err(SkillError::InvalidVersion(v.into())); }
    Ok((
        parts[0].parse::<u32>().map_err(|_| SkillError::InvalidVersion(v.into()))?,
        parts[1].parse::<u32>().map_err(|_| SkillError::InvalidVersion(v.into()))?,
        parts[2].parse::<u32>().map_err(|_| SkillError::InvalidVersion(v.into()))?,
    ))
}

/// Compare two semver versions. Return -1 / 0 / +1 (lexicographic 3-segment).
pub fn compare_versions(a: &str, b: &str) -> SkillResult<i32> {
    let pa = parse_version(a)?;
    let pb = parse_version(b)?;
    Ok(pa.cmp(&pb) as i32)
}

/// In-memory skill registry (Vec-backed, sorted by id).
#[derive(Debug, Default, Clone)]
pub struct Registry { skills: Vec<Skill> }

impl Registry {
    pub fn new() -> Self { Self::default() }
    /// Register a skill. Fails if id is invalid (per is_valid_id) or duplicate.
    pub fn register(&mut self, skill: Skill) -> SkillResult<()> {
        skill.validate()?;
        if self.skills.iter().any(|s| s.id == skill.id) {
            return Err(SkillError::DuplicateId(skill.id));
        }
        self.skills.push(skill);
        self.skills.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(())
    }
    pub fn get(&self, id: &str) -> SkillResult<&Skill> {
        self.skills.iter().find(|s| s.id == id).ok_or_else(|| SkillError::UnknownSkill(id.into()))
    }
    pub fn len(&self) -> usize { self.skills.len() }
    pub fn is_empty(&self) -> bool { self.skills.is_empty() }
    pub fn ids(&self) -> Vec<&str> { self.skills.iter().map(|s| s.id.as_str()).collect() }
}

/// Select the highest-version skill matching a prefix (e.g. `"summarize-*"`).
pub fn select_with_prefix<'a>(reg: &'a Registry, prefix: &str) -> Option<&'a Skill> {
    reg.skills.iter()
        .filter(|s| s.id.starts_with(prefix.trim_end_matches('*')))
        .max_by_key(|s| parse_version(&s.version).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn valid_skill_passes_validation() {
        let s = Skill::new("summarize-text", "1.0.0", "{}", "{}");
        assert!(s.validate().is_ok());
    }
    #[test] fn empty_id_is_rejected() {
        let s = Skill::new("   ", "1.0.0", "{}", "{}");
        assert!(s.validate().is_err());
    }
    #[test] fn invalid_version_is_rejected() {
        let s = Skill::new("ok", "1.0", "{}", "{}");
        assert!(s.validate().is_err());
    }

    #[test] fn is_valid_id_kebab() {
        assert!(is_valid_id("hello"));
        assert!(is_valid_id("hello-world"));
        assert!(is_valid_id("a1b2c3"));
        assert!(is_valid_id("x-1-y-2"));
        assert!(!is_valid_id(""));
        assert!(!is_valid_id("-start"));
        assert!(!is_valid_id("end-"));
        assert!(!is_valid_id("CamelCase"));
        assert!(!is_valid_id("space here"));
        assert!(!is_valid_id("under_score"));
        assert!(!is_valid_id("double--dash"));
    }
    #[test] fn parse_version_basic() {
        assert_eq!(parse_version("1.2.3").unwrap(), (1, 2, 3));
        assert!(parse_version("1.2").is_err());
        assert!(parse_version("a.b.c").is_err());
    }
    #[test] fn compare_versions_basic() {
        assert_eq!(compare_versions("1.0.0", "1.0.1").unwrap(), -1);
        assert_eq!(compare_versions("1.0.0", "1.0.0").unwrap(), 0);
        assert_eq!(compare_versions("2.0.0", "1.9.9").unwrap(), 1);
    }
    #[test] fn registry_register_and_get() {
        let mut r = Registry::new();
        r.register(Skill::new("a-skill", "1.0.0", "{}", "{}")).unwrap();
        r.register(Skill::new("b-skill", "2.0.0", "{}", "{}")).unwrap();
        assert_eq!(r.len(), 2);
        let s = r.get("a-skill").unwrap();
        assert_eq!(s.version, "1.0.0");
    }
    #[test] fn registry_duplicate_rejected() {
        let mut r = Registry::new();
        r.register(Skill::new("a", "1.0.0", "{}", "{}")).unwrap();
        assert!(matches!(r.register(Skill::new("a", "2.0.0", "{}", "{}")), Err(SkillError::DuplicateId(_))));
    }
    #[test] fn registry_unknown_get() {
        let r = Registry::new();
        assert!(matches!(r.get("nope"), Err(SkillError::UnknownSkill(_))));
    }
    #[test] fn select_with_prefix_picks_highest() {
        let mut r = Registry::new();
        r.register(Skill::new("summarize-text", "1.0.0", "{}", "{}")).unwrap();
        r.register(Skill::new("summarize-html", "1.5.0", "{}", "{}")).unwrap();
        r.register(Skill::new("summarize-md", "2.0.0", "{}", "{}")).unwrap();
        let s = select_with_prefix(&r, "summarize-").unwrap();
        assert_eq!(s.id, "summarize-md");
    }
    #[test] fn ids_sorted() {
        let mut r = Registry::new();
        r.register(Skill::new("z", "1.0.0", "{}", "{}")).unwrap();
        r.register(Skill::new("a", "1.0.0", "{}", "{}")).unwrap();
        r.register(Skill::new("m", "1.0.0", "{}", "{}")).unwrap();
        assert_eq!(r.ids(), vec!["a", "m", "z"]);
    }
}
