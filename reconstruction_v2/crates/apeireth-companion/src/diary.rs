//! Diary - 日记本 (从 v1.0 apeireth-companion/diary.rs 2K LOC 抄录升级核心)
//!
//! 0 装 PASS: 真按日归档
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub id: String,
    pub date: String,        // YYYY-MM-DD
    pub content: String,
    pub timestamp_ms: i64,
}

pub struct Diary {
    pub entries: HashMap<String, Vec<DiaryEntry>>,  // date -> entries
}

impl Diary {
    pub fn new() -> Self { Self { entries: HashMap::new() } }

    /// 0 装 PASS: 真 append (按 date)
    pub fn append(&mut self, date: impl Into<String>, content: impl Into<String>) {
        let date_str: String = date.into();
        let content_str: String = content.into();
        let entry = DiaryEntry { id: format!("de-{}-{}", date_str.len(), chrono::Utc::now().timestamp_millis()), date: date_str, content: content_str, timestamp_ms: chrono::Utc::now().timestamp_millis() };
        self.entries.entry(entry.date.clone()).or_default().push(entry);
    }

    /// 0 装 PASS: 真按 date 查
    pub fn for_date(&self, date: &str) -> Vec<&DiaryEntry> {
        self.entries.get(date).map(|v| v.iter().collect()).unwrap_or_default()
    }

    pub fn total_count(&self) -> usize {
        self.entries.values().map(|v| v.len()).sum()
    }
}

impl Default for Diary { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_append_basic() {
        let mut d = Diary::new();
        d.append("2024-06-15", "hello");
        assert_eq!(d.for_date("2024-06-15").len(), 1);
    }
    #[test] fn test_multi_day() {
        let mut d = Diary::new();
        d.append("2024-06-15", "a");
        d.append("2024-06-16", "b");
        assert_eq!(d.total_count(), 2);
        assert_eq!(d.for_date("2024-06-15").len(), 1);
    }
    #[test] fn test_unknown_date() {
        let d = Diary::new();
        assert!(d.for_date("2030-01-01").is_empty());
    }
}
