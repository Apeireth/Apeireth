//! CrossDiary - 跨日记关联 (从 v1.0 apeireth-companion/cross_diary.rs 301 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 token-based diary↔memory 链接

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DiaryMemoryLink { pub diary_id: String, pub memory_id: String, pub token: String }

pub struct CrossDiaryIndex { pub links: Vec<DiaryMemoryLink>, pub by_token: HashMap<String, (String, String)> }

impl CrossDiaryIndex {
    pub fn new() -> Self { Self { links: Vec::new(), by_token: HashMap::new() } }
    /// 0 装 PASS: 真 link
    pub fn link(&mut self, diary_id: impl Into<String>, memory_id: impl Into<String>, token: impl Into<String>) {
        let diary = diary_id.into();
        let mem = memory_id.into();
        let tok = token.into();
        self.links.push(DiaryMemoryLink { diary_id: diary.clone(), memory_id: mem.clone(), token: tok.clone() });
        self.by_token.insert(tok, (diary, mem));
    }
    /// 0 装 PASS: 真按 token 查
    pub fn by_token(&self, token: &str) -> Option<(&str, &str)> {
        self.by_token.get(token).map(|(d, m)| (d.as_str(), m.as_str()))
    }
    pub fn count(&self) -> usize { self.links.len() }
}

impl Default for CrossDiaryIndex { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_link() {
        let mut i = CrossDiaryIndex::new();
        i.link("d1", "m1", "tok1");
        assert_eq!(i.count(), 1);
    }
    #[test] fn test_by_token() {
        let mut i = CrossDiaryIndex::new();
        i.link("d1", "m1", "tok1");
        let (d, m) = i.by_token("tok1").unwrap();
        assert_eq!(d, "d1");
        assert_eq!(m, "m1");
    }
    #[test] fn test_unknown_token() {
        let i = CrossDiaryIndex::new();
        assert!(i.by_token("missing").is_none());
    }
    #[test] fn test_default() { let i: CrossDiaryIndex = Default::default(); assert_eq!(i.count(), 0); }
}
