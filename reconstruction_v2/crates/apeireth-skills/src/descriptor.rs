//! Skill descriptor (richer metadata for skill files).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDescriptor {
    pub id: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn construct() {
        let d = SkillDescriptor { id: "x".into(), version: "1.0.0".into(), description: "d".into(), tags: vec![] };
        assert_eq!(d.id, "x");
    }
}
