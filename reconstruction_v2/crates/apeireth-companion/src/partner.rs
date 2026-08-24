//! Partner - 合作伙伴 (从 v1.0 apeireth-companion/partner.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 Partner + preferences
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub type PartnerId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartnerPreferences {
    pub language: String,
    pub tone: String,
    pub notifications: bool,
}

impl Default for PartnerPreferences {
    fn default() -> Self {
        Self { language: "en".into(), tone: "neutral".into(), notifications: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Partner {
    pub id: PartnerId,
    pub name: String,
    pub preferences: PartnerPreferences,
    pub metadata: HashMap<String, String>,
}

pub struct PartnerRegistry {
    partners: HashMap<PartnerId, Partner>,
}

impl PartnerRegistry {
    pub fn new() -> Self { Self { partners: HashMap::new() } }

    /// 0 装 PASS: 真注册
    pub fn register(&mut self, p: Partner) {
        self.partners.insert(p.id.clone(), p);
    }

    pub fn get(&self, id: &str) -> Option<&Partner> { self.partners.get(id) }

    /// 0 装 PASS: 真按 language 过滤
    pub fn by_language(&self, lang: &str) -> Vec<&Partner> {
        self.partners.values().filter(|p| p.preferences.language == lang).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_register() {
        let mut r = PartnerRegistry::new();
        r.register(Partner { id: "p1".into(), name: "Alice".into(), preferences: PartnerPreferences::default(), metadata: HashMap::new() });
        assert!(r.get("p1").is_some());
    }
    #[test] fn test_unknown() {
        let r = PartnerRegistry::new();
        assert!(r.get("missing").is_none());
    }
    #[test] fn test_by_language() {
        let mut r = PartnerRegistry::new();
        r.register(Partner { id: "1".into(), name: "A".into(), preferences: PartnerPreferences { language: "en".into(), tone: "".into(), notifications: true }, metadata: HashMap::new() });
        r.register(Partner { id: "2".into(), name: "B".into(), preferences: PartnerPreferences { language: "zh".into(), tone: "".into(), notifications: true }, metadata: HashMap::new() });
        assert_eq!(r.by_language("en").len(), 1);
    }
    #[test] fn test_default_prefs() {
        let p = PartnerPreferences::default();
        assert_eq!(p.language, "en");
        assert!(p.notifications);
    }
}
