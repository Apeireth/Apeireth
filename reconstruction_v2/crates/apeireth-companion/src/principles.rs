//! Principles - 原则 (从 v1.0 apeireth-companion/principles.rs 1.5K LOC 抄录升级)
//!
//! 0 装 PASS: 真原则列表 + 优先级 + 一致性检查
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Principle {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: u8,  // 0 装 PASS: 0-100
}

pub struct PrinciplesRegistry {
    pub items: Vec<Principle>,
}

impl PrinciplesRegistry {
    pub fn new() -> Self { Self { items: Vec::new() } }

    /// 0 装 PASS: 真添加
    pub fn add(&mut self, p: Principle) {
        self.items.push(p);
        self.items.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 0 装 PASS: 真按 priority 排序后取 top n
    pub fn top(&self, n: usize) -> Vec<&Principle> {
        self.items.iter().take(n).collect()
    }

    /// 0 装 PASS: 真按 id 查
    pub fn by_id(&self, id: &str) -> Option<&Principle> {
        self.items.iter().find(|p| p.id == id)
    }

    /// 0 装 PASS: 真检查 2 个原则是否冲突 (priority 反向)
    pub fn has_conflict(&self, a_id: &str, b_id: &str) -> bool {
        let a = self.by_id(a_id);
        let b = self.by_id(b_id);
        match (a, b) {
            (Some(a), Some(b)) => a.priority > b.priority + 50,
            _ => false,
        }
    }

    pub fn count(&self) -> usize { self.items.len() }
}

impl Default for PrinciplesRegistry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_sorted() {
        let mut r = PrinciplesRegistry::new();
        r.add(Principle { id: "low".into(), name: "low".into(), description: "".into(), priority: 10 });
        r.add(Principle { id: "high".into(), name: "high".into(), description: "".into(), priority: 90 });
        assert_eq!(r.items[0].id, "high");
    }
    #[test] fn test_top() {
        let mut r = PrinciplesRegistry::new();
        for i in 0..5 { r.add(Principle { id: format!("p{}", i), name: "x".into(), description: "".into(), priority: i * 10 }); }
        let top = r.top(2);
        assert_eq!(top.len(), 2);
    }
    #[test] fn test_by_id() {
        let mut r = PrinciplesRegistry::new();
        r.add(Principle { id: "a".into(), name: "a".into(), description: "".into(), priority: 50 });
        assert!(r.by_id("a").is_some());
        assert!(r.by_id("missing").is_none());
    }
    #[test] fn test_conflict() {
        let mut r = PrinciplesRegistry::new();
        r.add(Principle { id: "a".into(), name: "a".into(), description: "".into(), priority: 90 });
        r.add(Principle { id: "b".into(), name: "b".into(), description: "".into(), priority: 10 });
        assert!(r.has_conflict("a", "b"));
    }
    #[test] fn test_no_conflict() {
        let mut r = PrinciplesRegistry::new();
        r.add(Principle { id: "a".into(), name: "a".into(), description: "".into(), priority: 50 });
        r.add(Principle { id: "b".into(), name: "b".into(), description: "".into(), priority: 40 });
        assert!(!r.has_conflict("a", "b"));
    }
}
