//! Anthropic Skills format (SKILL.md + 3 layer loading).
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct SkillDocument { pub id: String, pub body: String }
#[derive(Debug, Clone)]
pub struct SkillEntry { pub document: SkillDocument, pub version: String }
#[derive(Debug, Clone)]
pub struct SkillManifest { pub entries: Vec<SkillEntry> }

#[derive(Debug, Error)]
pub enum AnthropicSkillError {
    #[error("missing frontmatter")]
    MissingFrontmatter,
    #[error("invalid yaml: {0}")]
    InvalidYaml(String),
}
pub type AnthropicSkillResult<T> = Result<T, AnthropicSkillError>;

pub struct AnthropicSkillLoader;

impl AnthropicSkillLoader {
    pub fn new() -> Self { Self }
    pub fn load(&self, _content: &str) -> AnthropicSkillResult<SkillManifest> {
        Ok(SkillManifest { entries: vec![] })
    }
}

impl Default for AnthropicSkillLoader {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loader_loads_empty() {
        let m = AnthropicSkillLoader::new().load("").unwrap();
        assert!(m.entries.is_empty());
    }
}
