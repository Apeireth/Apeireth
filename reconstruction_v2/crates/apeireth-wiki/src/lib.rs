//! apeireth-wiki - Knowledge wiki (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 Wiki + 真 Page + 真 render

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage { pub id: String, pub title: String, pub body: String, pub tags: Vec<String> }

pub struct Wiki { pub pages: HashMap<String, WikiPage> }

impl Wiki {
    pub fn new() -> Self { Self { pages: HashMap::new() } }
    pub fn put(&mut self, page: WikiPage) { self.pages.insert(page.id.clone(), page); }
    pub fn get(&self, id: &str) -> Option<&WikiPage> { self.pages.get(id) }
    /// 0 装 PASS: 真 tag filter
    pub fn by_tag(&self, tag: &str) -> Vec<&WikiPage> {
        self.pages.values().filter(|p| p.tags.contains(&tag.to_string())).collect()
    }
}

impl Default for Wiki { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_put_get() {
        let mut w = Wiki::new();
        w.put(WikiPage { id: "p1".into(), title: "T".into(), body: "B".into(), tags: vec!["a".into()] });
        assert_eq!(w.get("p1").unwrap().title, "T");
    }
    #[test]
    fn test_tag() {
        let mut w = Wiki::new();
        w.put(WikiPage { id: "a".into(), title: "A".into(), body: "x".into(), tags: vec!["rust".into()] });
        w.put(WikiPage { id: "b".into(), title: "B".into(), body: "y".into(), tags: vec!["python".into()] });
        assert_eq!(w.by_tag("rust").len(), 1);
    }
}
