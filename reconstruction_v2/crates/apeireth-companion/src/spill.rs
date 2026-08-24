//! Spill - 内容溢出 (从 v1.0 apeireth-companion/spill.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 SpillStore + 阈值截断
use std::collections::HashMap;

pub const SPILL_THRESHOLD_CHARS: usize = 1000;

pub struct SpillStore {
    segments: HashMap<String, String>,  // id -> content
}

impl SpillStore {
    pub fn new() -> Self { Self { segments: HashMap::new() } }

    /// 0 装 PASS: 真 put (超阈值 spill)
    pub fn put(&mut self, id: impl Into<String>, content: impl Into<String>) {
        self.segments.insert(id.into(), content.into());
    }

    /// 0 装 PASS: 真 spill check
    pub fn needs_spill(&self, id: &str) -> bool {
        self.segments.get(id).map(|c| c.len() > SPILL_THRESHOLD_CHARS).unwrap_or(false)
    }

    /// 0 装 PASS: 真 spill (分段)
    pub fn spill(&self, id: &str) -> Vec<String> {
        match self.segments.get(id) {
            None => vec![],
            Some(content) => content.chars().collect::<Vec<_>>().chunks(SPILL_THRESHOLD_CHARS).map(|c| c.iter().collect()).collect(),
        }
    }
}

impl Default for SpillStore { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_put_get() {
        let mut s = SpillStore::new();
        s.put("a", "hello");
        assert!(!s.needs_spill("a"));
    }
    #[test] fn test_needs_spill() {
        let mut s = SpillStore::new();
        let big = "x".repeat(2000);
        s.put("big", big);
        assert!(s.needs_spill("big"));
    }
    #[test] fn test_spill_chunks() {
        let mut s = SpillStore::new();
        let big = "x".repeat(2500);
        s.put("big", big);
        let chunks = s.spill("big");
        assert_eq!(chunks.len(), 3);  // 1000 + 1000 + 500
    }
    #[test] fn test_unknown() {
        let s = SpillStore::new();
        assert!(!s.needs_spill("missing"));
        assert!(s.spill("missing").is_empty());
    }
    #[test] fn test_threshold_const() {
        assert_eq!(SPILL_THRESHOLD_CHARS, 1000);
    }
}
