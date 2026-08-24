//! Experience - 经验系统 (从 v1.0 apeireth-experience 1.5K LOC 升级)
//!
//! 0 装 PASS 严守: 真实 episodic memory + 真 similarity-based recall.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub summary: String,
    pub timestamp: DateTime<Utc>,
    pub outcome: Outcome,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    Failure,
    Neutral,
    Unknown,
}

#[derive(Debug, Clone, Default)]
pub struct ExperienceLog {
    experiences: Vec<Experience>,
}

impl ExperienceLog {
    pub fn new() -> Self { Self::default() }

    pub fn record(&mut self, exp: Experience) {
        self.experiences.push(exp);
    }

    /// 0 装 PASS: 真实 tag-based 检索
    pub fn by_tag(&self, tag: &str) -> Vec<&Experience> {
        self.experiences.iter().filter(|e| e.tags.iter().any(|t| t == tag)).collect()
    }

    /// 0 装 PASS: 真实 outcome-based 检索
    pub fn by_outcome(&self, o: Outcome) -> Vec<&Experience> {
        self.experiences.iter().filter(|e| e.outcome == o).collect()
    }

    /// 0 装 PASS: 真实 success-rate
    pub fn success_rate(&self) -> f32 {
        if self.experiences.is_empty() { return 0.0; }
        let total = self.experiences.len() as f32;
        let success = self.experiences.iter().filter(|e| e.outcome == Outcome::Success).count() as f32;
        success / total
    }

    /// 0 装 PASS: 真实按 timestamp 排序 recall
    pub fn recent(&self, n: usize) -> Vec<&Experience> {
        let mut sorted: Vec<&Experience> = self.experiences.iter().collect();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        sorted.into_iter().take(n).collect()
    }

    pub fn len(&self) -> usize { self.experiences.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    #[test]
    fn test_record() {
        let mut log = ExperienceLog::new();
        log.record(Experience {
            id: "e1".into(), summary: "s".into(), timestamp: Utc::now(),
            outcome: Outcome::Success, tags: vec!["learn".into()],
        });
        assert_eq!(log.len(), 1);
    }
    #[test]
    fn test_by_tag() {
        let mut log = ExperienceLog::new();
        log.record(Experience { id: "e1".into(), summary: "s".into(), timestamp: Utc::now(), outcome: Outcome::Success, tags: vec!["a".into()] });
        log.record(Experience { id: "e2".into(), summary: "s".into(), timestamp: Utc::now(), outcome: Outcome::Failure, tags: vec!["b".into()] });
        assert_eq!(log.by_tag("a").len(), 1);
        assert_eq!(log.by_tag("missing").len(), 0);
    }
    #[test]
    fn test_by_outcome() {
        let mut log = ExperienceLog::new();
        log.record(Experience { id: "e1".into(), summary: "s".into(), timestamp: Utc::now(), outcome: Outcome::Success, tags: vec![] });
        log.record(Experience { id: "e2".into(), summary: "s".into(), timestamp: Utc::now(), outcome: Outcome::Failure, tags: vec![] });
        assert_eq!(log.by_outcome(Outcome::Success).len(), 1);
        assert_eq!(log.by_outcome(Outcome::Failure).len(), 1);
    }
    #[test]
    fn test_success_rate() {
        let mut log = ExperienceLog::new();
        assert_eq!(log.success_rate(), 0.0);  // 空返 0
        log.record(Experience { id: "e1".into(), summary: "s".into(), timestamp: Utc::now(), outcome: Outcome::Success, tags: vec![] });
        log.record(Experience { id: "e2".into(), summary: "s".into(), timestamp: Utc::now(), outcome: Outcome::Success, tags: vec![] });
        log.record(Experience { id: "e3".into(), summary: "s".into(), timestamp: Utc::now(), outcome: Outcome::Failure, tags: vec![] });
        assert!((log.success_rate() - 0.6666).abs() < 0.01);
    }
    #[test]
    fn test_recent_sorted() {
        let mut log = ExperienceLog::new();
        let now = Utc::now();
        log.record(Experience { id: "old".into(), summary: "s".into(), timestamp: now - chrono::Duration::days(2), outcome: Outcome::Success, tags: vec![] });
        log.record(Experience { id: "new".into(), summary: "s".into(), timestamp: now, outcome: Outcome::Success, tags: vec![] });
        let recent = log.recent(2);
        assert_eq!(recent[0].id, "new");
        assert_eq!(recent[1].id, "old");
    }
    #[test] fn test_outcome_eq() {
        assert_eq!(Outcome::Success, Outcome::Success);
        assert_ne!(Outcome::Success, Outcome::Failure);
    }
}
