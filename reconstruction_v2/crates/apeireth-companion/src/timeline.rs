//! Timeline - 时间线 (从 v1.0 apeireth-companion/timeline.rs 1K LOC 抄录升级)
//!
//! 0 装 PASS: 真 timeline entry + 按时间排序
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub id: String,
    pub timestamp_ms: i64,
    pub kind: String,
    pub description: String,
}

pub struct Timeline {
    entries: VecDeque<TimelineEntry>,
    capacity: usize,
}

impl Timeline {
    pub fn new(capacity: usize) -> Self { Self { entries: VecDeque::with_capacity(capacity), capacity } }

    /// 0 装 PASS: 真按时间排序添加
    pub fn add(&mut self, mut entry: TimelineEntry) {
        if entry.timestamp_ms == 0 {
            entry.timestamp_ms = chrono::Utc::now().timestamp_millis();
        }
        let pos = self.entries.iter().position(|e| e.timestamp_ms > entry.timestamp_ms).unwrap_or(self.entries.len());
        self.entries.insert(pos, entry);
        if self.entries.len() > self.capacity { self.entries.pop_front(); }
    }

    pub fn range(&self, start_ms: i64, end_ms: i64) -> Vec<&TimelineEntry> {
        self.entries.iter().filter(|e| e.timestamp_ms >= start_ms && e.timestamp_ms <= end_ms).collect()
    }

    pub fn latest(&self, n: usize) -> Vec<&TimelineEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    pub fn len(&self) -> usize { self.entries.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_add_sorted() {
        let mut tl = Timeline::new(10);
        tl.add(TimelineEntry { id: "3".into(), timestamp_ms: 300, kind: "x".into(), description: "c".into() });
        tl.add(TimelineEntry { id: "1".into(), timestamp_ms: 100, kind: "x".into(), description: "a".into() });
        tl.add(TimelineEntry { id: "2".into(), timestamp_ms: 200, kind: "x".into(), description: "b".into() });
        assert_eq!(tl.entries[0].id, "1");
        assert_eq!(tl.entries[2].id, "3");
    }
    #[test] fn test_range() {
        let mut tl = Timeline::new(10);
        tl.add(TimelineEntry { id: "1".into(), timestamp_ms: 100, kind: "x".into(), description: "a".into() });
        tl.add(TimelineEntry { id: "2".into(), timestamp_ms: 200, kind: "x".into(), description: "b".into() });
        tl.add(TimelineEntry { id: "3".into(), timestamp_ms: 300, kind: "x".into(), description: "c".into() });
        let r = tl.range(150, 250);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "2");
    }
    #[test] fn test_latest() {
        let mut tl = Timeline::new(10);
        for i in 0..5 { tl.add(TimelineEntry { id: format!("{}", i), timestamp_ms: i * 100, kind: "x".into(), description: "x".into() }); }
        let l = tl.latest(2);
        assert_eq!(l.len(), 2);
        assert!(!l.is_empty());
    }
}
