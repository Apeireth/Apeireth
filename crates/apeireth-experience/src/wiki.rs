//! LLM Wiki — 从对话/事件提炼的"知识条目" (per stage2 §3 + Stage1 §13 候选).
//!
//! 借鉴 claude-mem 的 3 层渐进披露 (3-layer progressive disclosure) + apeireth-memory Note.
//!
//! WikiEntry 字段:
//! - id: 唯一 UUID
//! - title: 短标题
//! - content: 知识正文
//! - confidence: [0.0, 1.0] 置信度
//! - tags: 标签列表
//! - source_episode_ids: 来源 episode ids
//! - created_at: 创建时间
//! - last_updated: 最后更新时间
//! - promotion_count: 升迁计数 (per methodology crate)

#![deny(unsafe_code)]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WikiEntry {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub confidence: f64,
    pub tags: Vec<String>,
    pub source_episode_ids: Vec<String>,
    pub created_at: i64,
    pub last_updated: i64,
    pub promotion_count: u32,
}

impl WikiEntry {
    pub fn new(title: impl Into<String>, content: impl Into<String>, confidence: f64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            content: content.into(),
            confidence: confidence.clamp(0.0, 1.0),
            tags: Vec::new(),
            source_episode_ids: Vec::new(),
            created_at: now,
            last_updated: now,
            promotion_count: 0,
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source_episode_ids.push(source.into());
        self
    }

    pub fn promote(&mut self) {
        self.promotion_count += 1;
        self.last_updated = Utc::now().timestamp();
    }

    pub fn age_secs(&self) -> i64 {
        Utc::now().timestamp() - self.created_at
    }

    pub fn is_stale(&self, max_age_secs: i64) -> bool {
        self.age_secs() > max_age_secs
    }

    pub fn is_high_confidence(&self, threshold: f64) -> bool {
        self.confidence >= threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_entry_has_unique_id_and_initial_state() {
        let e1 = WikiEntry::new("a", "content", 0.8);
        let e2 = WikiEntry::new("a", "content", 0.8);
        assert_ne!(e1.id, e2.id);
        assert_eq!(e1.confidence, 0.8);
        assert_eq!(e1.promotion_count, 0);
        assert!(e1.tags.is_empty());
    }

    #[test]
    fn confidence_clamped_to_unit() {
        let e = WikiEntry::new("a", "b", 1.5);
        assert_eq!(e.confidence, 1.0);
        let e2 = WikiEntry::new("a", "b", -0.5);
        assert_eq!(e2.confidence, 0.0);
    }

    #[test]
    fn promote_increments_count() {
        let mut e = WikiEntry::new("a", "b", 0.5);
        e.promote();
        e.promote();
        assert_eq!(e.promotion_count, 2);
    }

    #[test]
    fn builder_pattern_with_tag_and_source() {
        let e = WikiEntry::new("a", "b", 0.5)
            .with_tag("test")
            .with_source("ep-1")
            .with_source("ep-2");
        assert_eq!(e.tags, vec!["test"]);
        assert_eq!(e.source_episode_ids, vec!["ep-1", "ep-2"]);
    }

    #[test]
    fn is_high_confidence_with_threshold() {
        let e = WikiEntry::new("a", "b", 0.9);
        assert!(e.is_high_confidence(0.85));
        assert!(!e.is_high_confidence(0.95));
    }
}
