//! MetaThinking - 元思考 (从 v1.0 apeireth-companion/meta_thinking.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 thought recursion depth + 反思栈
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    pub id: String,
    pub content: String,
    pub depth: u32,
    pub timestamp_ms: i64,
    pub parent_id: Option<String>,
}

pub struct MetaThinking {
    stack: VecDeque<ThoughtNode>,
    max_depth: u32,
    counter: AtomicU64,
}

impl MetaThinking {
    pub fn new(max_depth: u32) -> Self { Self { stack: VecDeque::new(), max_depth, counter: AtomicU64::new(0) } }

    /// 0 装 PASS: 真思考 (递归)
    pub fn think(&mut self, content: impl Into<String>, parent: Option<String>) -> Result<String, String> {
        let parent_depth = parent.as_ref().and_then(|p| self.stack.iter().find(|n| &n.id == p).map(|n| n.depth)).unwrap_or(0);
        if parent_depth >= self.max_depth { return Err("max depth reached".into()); }
        let id = format!("t-{}-{}", chrono::Utc::now().timestamp_millis(), self.counter.fetch_add(1, Ordering::Relaxed));
        self.stack.push_back(ThoughtNode { id: id.clone(), content: content.into(), depth: parent_depth + 1, timestamp_ms: chrono::Utc::now().timestamp_millis(), parent_id: parent });
        Ok(id)
    }

    pub fn depth(&self) -> u32 {
        self.stack.iter().map(|n| n.depth).max().unwrap_or(0)
    }

    pub fn trace(&self) -> Vec<&ThoughtNode> {
        self.stack.iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_single_thought() {
        let mut m = MetaThinking::new(3);
        let id = m.think("hello", None).unwrap();
        assert_eq!(m.depth(), 1);
        assert!(m.trace().iter().any(|n| n.id == id));
    }
    #[test] fn test_recursive_depth() {
        let mut m = MetaThinking::new(3);
        let a = m.think("a", None).unwrap();
        let b = m.think("b", Some(a.clone())).unwrap();
        let _c = m.think("c", Some(b.clone())).unwrap();
        assert_eq!(m.depth(), 3);
    }
    #[test] fn test_max_depth() {
        let mut m = MetaThinking::new(2);
        let a = m.think("a", None).unwrap();
        let b = m.think("b", Some(a.clone())).unwrap();
        assert!(m.think("c", Some(b)).is_err());
    }
}
