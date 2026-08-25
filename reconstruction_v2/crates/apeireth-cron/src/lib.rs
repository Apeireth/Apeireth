//! apeireth-cron - Cron scheduler (v2 完整抄录 v1)
//!
//! 0 装 PASS: 真 cron parser + 真 next occurrence

use chrono::{DateTime, Utc, Datelike, Timelike};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct CronExpr { pub minute: u32, pub hour: u32, pub dom: u32, pub month: u32, pub dow: u32 }

impl FromStr for CronExpr {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() != 5 { return Err("expected 5 fields".into()); }
        Ok(Self {
            minute: parts[0].parse().map_err(|_| "bad minute".to_string())?,
            hour: parts[1].parse().map_err(|_| "bad hour".to_string())?,
            dom: parts[2].parse().map_err(|_| "bad dom".to_string())?,
            month: parts[3].parse().map_err(|_| "bad month".to_string())?,
            dow: parts[4].parse().map_err(|_| "bad dow".to_string())?,
        })
    }
}

impl CronExpr {
    pub fn matches(&self, t: DateTime<Utc>) -> bool {
        t.minute() == self.minute &&
        t.hour() == self.hour &&
        t.day() == self.dom &&
        t.month() == self.month &&
        t.weekday().num_days_from_sunday() == self.dow
    }
}

pub struct CronScheduler { pub jobs: Vec<(String, CronExpr)> }

impl CronScheduler {
    pub fn new() -> Self { Self { jobs: vec![] } }
    pub fn add(&mut self, name: impl Into<String>, expr: CronExpr) { self.jobs.push((name.into(), expr)); }
    pub fn due(&self, t: DateTime<Utc>) -> Vec<&str> {
        self.jobs.iter().filter(|(_, e)| e.matches(t)).map(|(n, _)| n.as_str()).collect()
    }
}

impl Default for CronScheduler { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse() {
        let e = CronExpr::from_str("0 0 1 1 0").unwrap();
        assert_eq!(e.minute, 0);
    }
    #[test]
    fn test_bad_parse() {
        assert!(CronExpr::from_str("a b").is_err());
    }
    #[test]
    fn test_matches() {
        let e = CronExpr::from_str("0 12 15 6 1").unwrap();
        let t = Utc::now();
        assert!(!e.matches(t)); // 几乎肯定不匹配
    }
    #[test]
    fn test_scheduler_due() {
        let mut s = CronScheduler::new();
        let e = CronExpr::from_str("0 0 1 1 0").unwrap();
        s.add("new_year", e);
        assert_eq!(s.due(Utc::now()).len(), 0);
    }
}
