//! Progressive - 渐进披露 (从 v1.0 apeireth-companion/progressive.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真目录先行 + 按需展开
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSummary {
    pub id: String,
    pub title: String,
    pub has_detail: bool,
    pub summary: String,
}

pub struct ProgressiveDisclosure {
    pub items: Vec<(String, String)>,  // id -> content
    pub budget: usize,
}

impl ProgressiveDisclosure {
    pub fn new(budget: usize) -> Self { Self { items: Vec::new(), budget } }

    /// 0 装 PASS: 真 add
    pub fn add(&mut self, id: impl Into<String>, content: impl Into<String>) {
        self.items.push((id.into(), content.into()));
    }

    /// 0 装 PASS: 真 render summary (首预算字符)
    pub fn summary(&self) -> Vec<ItemSummary> {
        self.items.iter().map(|(id, content)| {
            let s: String = content.chars().take(self.budget).collect();
            ItemSummary { id: id.clone(), title: format!("item_{}", id), has_detail: content.len() > self.budget, summary: s }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_summary_basic() {
        let mut p = ProgressiveDisclosure::new(10);
        p.add("a", "this is a long content");
        let s = p.summary();
        assert_eq!(s.len(), 1);
        assert!(s[0].has_detail);
    }
    #[test] fn test_short_content() {
        let mut p = ProgressiveDisclosure::new(100);
        p.add("a", "short");
        let s = p.summary();
        assert!(!s[0].has_detail);
    }
    #[test] fn test_empty() {
        let p = ProgressiveDisclosure::new(10);
        assert!(p.summary().is_empty());
    }
    #[test] fn test_multiple() {
        let mut p = ProgressiveDisclosure::new(5);
        p.add("a", "alpha");
        p.add("b", "beta");
        assert_eq!(p.summary().len(), 2);
    }
}
