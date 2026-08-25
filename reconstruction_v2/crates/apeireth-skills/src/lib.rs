//! apeireth-skills - Skill registry (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 SkillDescriptor + 真 registry + 真 lookup

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDescriptor {
    pub name: String,
    pub description: String,
    pub trigger_keywords: Vec<String>,
    pub handler: String,
}

pub struct SkillRegistry { pub skills: HashMap<String, SkillDescriptor> }

impl SkillRegistry {
    pub fn new() -> Self { Self { skills: HashMap::new() } }
    pub fn register(&mut self, s: SkillDescriptor) { self.skills.insert(s.name.clone(), s); }
    pub fn match_keyword(&self, kw: &str) -> Option<&SkillDescriptor> {
        self.skills.values().find(|s| s.trigger_keywords.iter().any(|k| k == kw))
    }
    pub fn count(&self) -> usize { self.skills.len() }
}

impl Default for SkillRegistry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_register_match() {
        let mut r = SkillRegistry::new();
        r.register(SkillDescriptor { name: "s1".into(), description: "d".into(), trigger_keywords: vec!["weather".into()], handler: "h".into() });
        assert_eq!(r.match_keyword("weather").unwrap().name, "s1");
    }
    #[test]
    fn test_no_match() {
        let r = SkillRegistry::new();
        assert!(r.match_keyword("x").is_none());
    }
}
