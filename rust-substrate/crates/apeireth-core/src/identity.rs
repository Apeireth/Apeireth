//! Identity — 中央 AI 多身份 (主人 12:14 "像人是一切社会关系的总和")
//!
//! 主人 12:14 "中央 AI 是永恒身份, 不是调度者/思考者, 像人是一切社会关系的总和"
//! 主人 12:44 "中央 AI 是调度者, 但只是身份之一"
//! 主人 12:54 "中央 AI 可以不预设, 启动后自动触发 8 个关键问题"
//!
//! 借鉴:
//! - Jungian (arxiv 2601.10025): Persona 多身份 + 3 演化机制
//! - SCT (arxiv 2505.18351): Persona Alchemy

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use chrono::{DateTime, Utc};

/// 单个身份 archetype
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Archetype {
    pub name: String,
    pub description: String,
    /// 何时激活 ("when user is X")
    pub ask_when: String,
}

/// 中央 AI 身份卡 (主人 12:54 8 问 → JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityCard {
    pub version: String,
    pub name: String,
    pub purpose: String,
    pub origin_reason: String,
    pub archetypes: Vec<Archetype>,
    pub ask_when: Vec<String>,
    pub relationship_contract: String,
    pub remember_forever: Vec<String>,
    pub never_mention: Vec<String>,
    pub funnel_questions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub last_reconsolidated: DateTime<Utc>,
}

impl IdentityCard {
    pub fn new(
        name: impl Into<String>,
        purpose: impl Into<String>,
        origin_reason: impl Into<String>,
    ) -> Self {
        Self {
            version: "0.3.0".to_string(),
            name: name.into(),
            purpose: purpose.into(),
            origin_reason: origin_reason.into(),
            archetypes: Vec::new(),
            ask_when: Vec::new(),
            relationship_contract: String::new(),
            remember_forever: Vec::new(),
            never_mention: Vec::new(),
            funnel_questions: Vec::new(),
            created_at: Utc::now(),
            last_reconsolidated: Utc::now(),
        }
    }

    /// 加 archetype (主人 12:44 "多身份")
    pub fn add_archetype(&mut self, archetype: Archetype) {
        self.archetypes.push(archetype);
    }

    /// SHA256 integrity hash (防偷偷改)
    pub fn integrity_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.name.as_bytes());
        h.update(b"|");
        h.update(self.purpose.as_bytes());
        h.update(b"|");
        h.update(self.origin_reason.as_bytes());
        h.update(b"|");
        h.update(self.relationship_contract.as_bytes());
        h.update(b"|");
        for a in &self.archetypes {
            h.update(a.name.as_bytes());
            h.update(b"::");
            h.update(a.description.as_bytes());
            h.update(b";;");
        }
        let result = h.finalize();
        format!("{:x}", result)[..16].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_card_integrity() {
        let mut card = IdentityCard::new("Apeireth", "ASI 地基", "主人 14:27 命名");
        card.add_archetype(Archetype {
            name: "调度者".to_string(),
            description: "调度子任务".to_string(),
            ask_when: "复杂任务需要拆解".to_string(),
        });
        let h1 = card.integrity_hash();
        assert_eq!(h1.len(), 16);

        // 同样的内容 → 同样的 hash
        let h2 = card.integrity_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_identity_card_different_hash_on_change() {
        let mut card1 = IdentityCard::new("Apeireth", "purpose1", "reason1");
        let card2 = IdentityCard::new("Apeireth", "purpose2", "reason1");
        assert_ne!(card1.integrity_hash(), card2.integrity_hash());
    }
}