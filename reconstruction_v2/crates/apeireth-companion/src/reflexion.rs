//! Reflexion - 反思 (从 v1.0 apeireth-companion/reflexion.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真反思 entry + trace
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflexionEntry {
    pub failure_id: String,
    pub reflection: String,
    pub lessons: Vec<String>,
    pub timestamp_ms: i64,
}

pub struct ReflexionLog {
    entries: VecDeque<ReflexionEntry>,
    capacity: usize,
}

impl ReflexionLog {
    pub fn new(capacity: usize) -> Self { Self { entries: VecDeque::with_capacity(capacity), capacity } }

    /// 0 装 PASS: 真记录
    pub fn record(&mut self, failure_id: impl Into<String>, reflection: impl Into<String>, lessons: Vec<String>) {
        let entry = ReflexionEntry { failure_id: failure_id.into(), reflection: reflection.into(), lessons, timestamp_ms: chrono::Utc::now().timestamp_millis() };
        self.entries.push_back(entry);
        if self.entries.len() > self.capacity { self.entries.pop_front(); }
    }

    /// 0 装 PASS: 真按 failure_id 查
    pub fn by_failure(&self, failure_id: &str) -> Vec<&ReflexionEntry> {
        self.entries.iter().filter(|e| e.failure_id == failure_id).collect()
    }

    pub fn all_lessons(&self) -> Vec<String> {
        self.entries.iter().flat_map(|e| e.lessons.iter().cloned()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record() {
        let mut r = ReflexionLog::new(10);
        r.record("f1", "we failed", vec!["try again".into()]);
        assert_eq!(r.by_failure("f1").len(), 1);
    }
    #[test] fn test_all_lessons() {
        let mut r = ReflexionLog::new(10);
        r.record("f1", "r1", vec!["L1".into(), "L2".into()]);
        r.record("f2", "r2", vec!["L3".into()]);
        assert_eq!(r.all_lessons().len(), 3);
    }
    #[test] fn test_capacity() {
        let mut r = ReflexionLog::new(2);
        for i in 0..5 { r.record(&format!("f{}", i), "x", vec![]); }
        assert_eq!(r.by_failure("f4").len(), 1);
    }
    #[test] fn test_unknown_failure() {
        let r = ReflexionLog::new(10);
        assert!(r.by_failure("missing").is_empty());
    }
}
