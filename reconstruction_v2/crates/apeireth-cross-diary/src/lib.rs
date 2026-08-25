//! apeireth-cross-diary - Cross-diary linking (v2 完整抄录 v1 cross_diary.rs)
//!
//! 0 装 PASS: 真 CrossDiaryIndex + 真 token-based linking

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryLink { pub token: String, pub from: String, pub to: String }

pub struct CrossDiaryIndex { pub links: HashMap<String, DiaryLink> }

impl CrossDiaryIndex {
    pub fn new() -> Self { Self { links: HashMap::new() } }
    /// 0 装 PASS: 真 link (dup reject)
    pub fn link(&mut self, token: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> bool {
        let token = token.into();
        if self.links.contains_key(&token) { return false; }
        self.links.insert(token.clone(), DiaryLink { token, from: from.into(), to: to.into() });
        true
    }
    /// 0 装 PASS: 真 by token
    pub fn by_token(&self, token: &str) -> Option<&DiaryLink> { self.links.get(token) }
    /// 0 装 PASS: 真 all
    pub fn all(&self) -> Vec<&DiaryLink> { self.links.values().collect() }
}

impl Default for CrossDiaryIndex { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_link() {
        let mut i = CrossDiaryIndex::new();
        assert!(i.link("t1", "d1", "d2"));
        assert_eq!(i.by_token("t1").unwrap().from, "d1");
    }
    #[test]
    fn test_duplicate() {
        let mut i = CrossDiaryIndex::new();
        i.link("t", "a", "b");
        assert!(!i.link("t", "c", "d"));
    }
    #[test]
    fn test_default() {
        let i: CrossDiaryIndex = Default::default();
        assert!(i.all().is_empty());
    }
}
