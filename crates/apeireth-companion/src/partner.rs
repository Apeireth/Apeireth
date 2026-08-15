//! 伙伴 —— 用户作为伙伴 (Per stage1 2026-08-14 清晰版: 用户在关系里, 是 AI 的伙伴)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::bond::{Bond, BondStage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartnerId(pub Uuid);

impl PartnerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PartnerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PartnerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "partner:{}", self.0)
    }
}

/// 用户偏好 —— 用户希望 AI 怎么对待
///
/// 这些是用户**声明**的界限, 不是 AI 自己猜的. 配合 sovereignty 形成双层约束.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartnerPreferences {
    /// 称呼偏好 ("你" / "您" / 名字)
    pub address: Option<String>,
    /// 表达风格 (哲学密度 / 简洁 / 详细 / 调侃 / 严肃)
    pub style: Option<String>,
    /// 关心的话题 (有中心, AI 可围绕)
    pub topics: Vec<String>,
    /// 雷区 (避开的话题)
    pub avoid: Vec<String>,
    /// 自由备注
    pub notes: HashMap<String, String>,
    /// 隐私边界 (per opencode-vibeguard 模式)
    pub privacy: PrivacyBoundary,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PrivacyBoundary {
    /// 是否允许出站 LLM 调用的敏感字符串替换
    pub allow_outbound_substitution: bool,
    /// 敏感字符串列表 (用户自己声明)
    pub sensitive_strings: Vec<String>,
}

/// 伙伴 —— 用户作为 AI 的伙伴
///
/// 这是"用户在关系里" 的工程化承载. 不是 AI 的子民, 是 AI 的伙伴.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Partner {
    id: PartnerId,
    display_name: String,
    preferences: PartnerPreferences,
    bond: Bond,
    created_at: DateTime<Utc>,
    last_seen: DateTime<Utc>,
}

impl Partner {
    pub fn new(id: PartnerId, display_name: String, preferences: PartnerPreferences) -> Self {
        let now = Utc::now();
        Self {
            id,
            display_name,
            preferences,
            bond: Bond::new(),
            created_at: now,
            last_seen: now,
        }
    }

    pub fn id(&self) -> PartnerId { self.id }
    pub fn display_name(&self) -> &str { &self.display_name }
    pub fn preferences(&self) -> &PartnerPreferences { &self.preferences }
    pub fn bond(&self) -> &Bond { &self.bond }
    pub fn bond_mut(&mut self) -> &mut Bond { &mut self.bond }
    pub fn created_at(&self) -> DateTime<Utc> { self.created_at }
    pub fn last_seen(&self) -> DateTime<Utc> { self.last_seen }

    pub fn touch(&mut self) {
        self.last_seen = Utc::now();
    }

    pub fn update_preferences(&mut self, prefs: PartnerPreferences) {
        self.preferences = prefs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_partner_has_empty_bond() {
        let p = Partner::new(PartnerId::new(), "alice".into(), PartnerPreferences::default());
        assert_eq!(p.bond().stage(), BondStage::Initial);
    }

    #[test]
    fn touch_updates_last_seen() {
        let mut p = Partner::new(PartnerId::new(), "alice".into(), PartnerPreferences::default());
        let before = p.last_seen();
        std::thread::sleep(std::time::Duration::from_millis(2));
        p.touch();
        assert!(p.last_seen() > before);
    }
}
