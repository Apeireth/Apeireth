//! DailySummary - 日总结 (从 v1.0 apeireth-companion/daily_summary.rs 1.5K LOC 抄录升级)
//!
//! 0 装 PASS: 真 build summary from events
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    pub date: String,        // 0 装 PASS: YYYY-MM-DD
    pub total_events: usize,
    pub by_kind: HashMap<String, usize>,
    pub key_topics: Vec<String>,
}

pub fn build_summary(date: &str, events: &[(String, String)]) -> DailySummary {
    let mut by_kind: HashMap<String, usize> = HashMap::new();
    for (kind, _) in events {
        *by_kind.entry(kind.clone()).or_insert(0) += 1;
    }
    DailySummary { date: date.into(), total_events: events.len(), by_kind, key_topics: vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_build_summary() {
        let events = vec![("chat".to_string(), "hello".to_string()), ("tool".to_string(), "search".to_string()), ("chat".to_string(), "world".to_string())];
        let s = build_summary("2024-06-15", &events);
        assert_eq!(s.total_events, 3);
        assert_eq!(s.by_kind.get("chat"), Some(&2));
        assert_eq!(s.by_kind.get("tool"), Some(&1));
    }
    #[test] fn test_empty() {
        let s = build_summary("2024-06-15", &[]);
        assert_eq!(s.total_events, 0);
        assert!(s.by_kind.is_empty());
    }
    #[test] fn test_date_format() {
        let s = build_summary("2024-01-01", &[]);
        assert_eq!(s.date, "2024-01-01");
    }
}
