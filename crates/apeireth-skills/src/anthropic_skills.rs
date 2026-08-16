//! Anthropic Skills 模式适配层 (R149).
//!
//! 借鉴 https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills
//!
//! **目录结构** (Anthropic 规范):
//! ```text
//! skills/
//!   my-skill/
//!     SKILL.md          # 必填: YAML frontmatter + Markdown body
//!     scripts/          # 可选: 可执行脚本
//!     references/       # 可选: 引用文档 (按需加载)
//!     assets/           # 可选: 静态资源
//! ```
//!
//! **3 层加载** (节省 token):
//! 1. YAML metadata (always loaded) - name/description/allowed-tools
//! 2. SKILL.md body (on-demand) - 触发条件命中后加载
//! 3. references/resources (lazy) - 实际执行时才加载
//!
//! 不假装 (O-5): 真解析 YAML frontmatter, 真扫描目录, 真 3 层 lazy load.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnthropicSkillError {
    #[error("path not found: {0}")]
    PathNotFound(String),
    #[error("not a directory: {0}")]
    NotDirectory(String),
    #[error("missing SKILL.md in {0}")]
    MissingSkillMd(String),
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("invalid skill name: {0}")]
    InvalidName(String),
}

pub type AnthropicSkillResult<T> = Result<T, AnthropicSkillError>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub triggers: Vec<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SkillManifest {
    pub fn validate(&self) -> AnthropicSkillResult<()> {
        if self.name.is_empty() {
            return Err(AnthropicSkillError::InvalidName("empty name".into()));
        }
        if !self
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(AnthropicSkillError::InvalidName(self.name.clone()));
        }
        if self.name.starts_with('-') || self.name.ends_with('-') {
            return Err(AnthropicSkillError::InvalidName(self.name.clone()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDocument {
    pub manifest: SkillManifest,
    pub body_md: String,
    pub references: Vec<PathBuf>,
    pub scripts: Vec<PathBuf>,
    pub assets: Vec<PathBuf>,
}

impl SkillDocument {
    pub fn size_bytes(&self) -> usize {
        self.body_md.len()
            + self
                .references
                .iter()
                .map(|p| p.metadata().map(|m| m.len() as usize).unwrap_or(0))
                .sum::<usize>()
            + self
                .scripts
                .iter()
                .map(|p| p.metadata().map(|m| m.len() as usize).unwrap_or(0))
                .sum::<usize>()
            + self
                .assets
                .iter()
                .map(|p| p.metadata().map(|m| m.len() as usize).unwrap_or(0))
                .sum::<usize>()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub manifest: SkillManifest,
    pub skill_md_path: PathBuf,
}

pub struct AnthropicSkillLoader {
    root: PathBuf,
    cache: RwLock<HashMap<String, SkillDocument>>,
}

impl AnthropicSkillLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn scan_entries(&self) -> AnthropicSkillResult<Vec<SkillEntry>> {
        if !self.root.exists() {
            return Err(AnthropicSkillError::PathNotFound(
                self.root.display().to_string(),
            ));
        }
        if !self.root.is_dir() {
            return Err(AnthropicSkillError::NotDirectory(
                self.root.display().to_string(),
            ));
        }
        let mut entries = Vec::new();
        for dir in std::fs::read_dir(&self.root)
            .map_err(|e| AnthropicSkillError::PathNotFound(e.to_string()))?
        {
            let dir = dir.map_err(|e| AnthropicSkillError::PathNotFound(e.to_string()))?;
            let path = dir.path();
            if !path.is_dir() {
                continue;
            }
            let skill_md = path.join("SKILL.md");
            if !skill_md.exists() {
                continue;
            }
            let raw = std::fs::read_to_string(&skill_md)
                .map_err(|e| AnthropicSkillError::InvalidFrontmatter(e.to_string()))?;
            let manifest = parse_frontmatter(&raw)?;
            manifest.validate()?;
            entries.push(SkillEntry {
                manifest,
                skill_md_path: skill_md,
            });
        }
        Ok(entries)
    }

    pub fn load_full(&self, name: &str) -> AnthropicSkillResult<SkillDocument> {
        if let Some(doc) = self.cache.read().get(name).cloned() {
            return Ok(doc);
        }
        let skill_dir = self.root.join(name);
        if !skill_dir.is_dir() {
            return Err(AnthropicSkillError::PathNotFound(
                skill_dir.display().to_string(),
            ));
        }
        let skill_md = skill_dir.join("SKILL.md");
        if !skill_md.exists() {
            return Err(AnthropicSkillError::MissingSkillMd(
                skill_dir.display().to_string(),
            ));
        }
        let raw = std::fs::read_to_string(&skill_md)
            .map_err(|e| AnthropicSkillError::InvalidFrontmatter(e.to_string()))?;
        let (manifest, body) = parse_frontmatter_and_body(&raw)?;
        manifest.validate()?;
        let references = list_dir_files(&skill_dir.join("references"));
        let scripts = list_dir_files(&skill_dir.join("scripts"));
        let assets = list_dir_files(&skill_dir.join("assets"));
        let doc = SkillDocument {
            manifest,
            body_md: body,
            references,
            scripts,
            assets,
        };
        self.cache.write().insert(name.to_string(), doc.clone());
        Ok(doc)
    }

    pub fn invalidate(&self, name: &str) -> bool {
        self.cache.write().remove(name).is_some()
    }

    pub fn cache_size(&self) -> usize {
        self.cache.read().len()
    }
}

fn parse_frontmatter(raw: &str) -> AnthropicSkillResult<SkillManifest> {
    let (manifest, _body) = parse_frontmatter_and_body(raw)?;
    Ok(manifest)
}

fn parse_frontmatter_and_body(raw: &str) -> AnthropicSkillResult<(SkillManifest, String)> {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return Err(AnthropicSkillError::InvalidFrontmatter(
            "missing opening ---".into(),
        ));
    }
    let after_open = &trimmed[3..];
    let close_idx = after_open
        .find("\n---")
        .ok_or_else(|| AnthropicSkillError::InvalidFrontmatter("missing closing ---".into()))?;
    let yaml = &after_open[..close_idx];
    let body_start = close_idx + 4;
    let body = after_open[body_start..]
        .trim_start_matches(|c: char| c == '\n' || c == ' ')
        .to_string();
    let manifest: SkillManifest = serde_yaml_parse(yaml)?;
    Ok((manifest, body))
}

/// 极简 YAML 解析 (kebab-case skill manifest 子集, 不引 serde_yaml 350KB 依赖)
fn serde_yaml_parse(yaml: &str) -> AnthropicSkillResult<SkillManifest> {
    let mut m = SkillManifest::default();
    for line in yaml.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if let Some(colon_idx) = l.find(':') {
            let key = l[..colon_idx].trim().to_string();
            let value = l[colon_idx + 1..].trim();
            if value.starts_with('[') && value.ends_with(']') {
                let inner = &value[1..value.len() - 1];
                let items: Vec<String> = inner
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .collect();
                apply_list(&mut m, &key, items);
            } else if value.starts_with('{') && value.ends_with('}') {
                let inner = &value[1..value.len() - 1];
                for pair in inner.split(',') {
                    if let Some(eq) = pair.find('=') {
                        let k = pair[..eq].trim().trim_matches('"').to_string();
                        let v = pair[eq + 1..].trim().trim_matches('"').to_string();
                        m.metadata.insert(k, v);
                    }
                }
            } else {
                match key.as_str() {
                    "name" => m.name = value.trim_matches('"').to_string(),
                    "description" => m.description = value.trim_matches('"').to_string(),
                    _ => {}
                }
            }
        }
    }
    Ok(m)
}

fn apply_list(m: &mut SkillManifest, key: &str, items: Vec<String>) {
    match key {
        "allowed_tools" => m.allowed_tools = items,
        "triggers" => m.triggers = items,
        _ => {}
    }
}

fn list_dir_files(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_file() {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_skill(dir: &Path, name: &str, yaml: &str, body: &str) {
        let skill_dir = dir.join(name);
        std::fs::create_dir_all(&skill_dir).unwrap();
        let mut f = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "{}", yaml).unwrap();
        writeln!(f, "---").unwrap();
        writeln!(f, "{}", body).unwrap();
    }

    #[test]
    fn parse_simple_frontmatter() {
        let raw = "---\nname: test-skill\ndescription: A test skill\n---\n# Body";
        let m = parse_frontmatter(raw).unwrap();
        assert_eq!(m.name, "test-skill");
        assert_eq!(m.description, "A test skill");
    }

    #[test]
    fn parse_with_lists() {
        let raw = "---\nname: x\ndescription: y\nallowed_tools: [tool-a, tool-b]\ntriggers: [when-x]\n---\nbody";
        let m = parse_frontmatter(raw).unwrap();
        assert_eq!(m.name, "x");
        assert_eq!(m.allowed_tools, vec!["tool-a", "tool-b"]);
        assert_eq!(m.triggers, vec!["when-x"]);
    }

    #[test]
    fn parse_metadata_inline() {
        let raw = "---\nname: x\ndescription: y\nmetadata: {author = z, version = 1.0}\n---\n";
        let m = parse_frontmatter(raw).unwrap();
        assert_eq!(m.metadata.get("author").unwrap(), "z");
        assert_eq!(m.metadata.get("version").unwrap(), "1.0");
    }

    #[test]
    fn invalid_no_frontmatter() {
        let raw = "# Just markdown\nNo frontmatter";
        assert!(parse_frontmatter(raw).is_err());
    }

    #[test]
    fn invalid_no_close() {
        let raw = "---\nname: x\nno close";
        assert!(parse_frontmatter(raw).is_err());
    }

    #[test]
    fn manifest_validate_rejects_bad_name() {
        let mut m = SkillManifest::default();
        m.name = "Bad_Name".into();
        assert!(m.validate().is_err());
        m.name = "-start".into();
        assert!(m.validate().is_err());
        m.name = "end-".into();
        assert!(m.validate().is_err());
        m.name = "".into();
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifest_validate_accepts_kebab() {
        let m = SkillManifest {
            name: "good-skill-1".into(),
            description: "ok".into(),
            ..Default::default()
        };
        assert!(m.validate().is_ok());
    }

    #[test]
    fn loader_scan_and_load() {
        let tmp = std::env::temp_dir().join("apeireth-skills-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        write_skill(
            &tmp,
            "skill-a",
            "name: skill-a\ndescription: First",
            "body of skill a",
        );
        write_skill(
            &tmp,
            "skill-b",
            "name: skill-b\ndescription: Second",
            "body of skill b",
        );

        let loader = AnthropicSkillLoader::new(&tmp);
        let entries = loader.scan_entries().unwrap();
        assert_eq!(entries.len(), 2);
        let names: Vec<&str> = entries.iter().map(|e| e.manifest.name.as_str()).collect();
        assert!(names.contains(&"skill-a"));
        assert!(names.contains(&"skill-b"));

        let full = loader.load_full("skill-a").unwrap();
        assert_eq!(full.manifest.name, "skill-a");
        assert!(full.body_md.contains("body of skill a"));

        assert_eq!(loader.cache_size(), 1);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn loader_load_missing_skill_errors() {
        let tmp = std::env::temp_dir().join("apeireth-skills-test-empty");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let loader = AnthropicSkillLoader::new(&tmp);
        let r = loader.load_full("nonexistent");
        assert!(matches!(r, Err(AnthropicSkillError::PathNotFound(_))));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn loader_invalidate_clears_cache() {
        let tmp = std::env::temp_dir().join("apeireth-skills-test-inv");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        write_skill(&tmp, "x", "name: x\ndescription: y", "z");
        let loader = AnthropicSkillLoader::new(&tmp);
        loader.load_full("x").unwrap();
        assert_eq!(loader.cache_size(), 1);
        assert!(loader.invalidate("x"));
        assert_eq!(loader.cache_size(), 0);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
