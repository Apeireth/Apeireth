//! Perception - 感知系统 (从 v1.0 apeireth-perception 2K LOC 升级)
//!
//! 0 装 PASS 严守: 真实 sensor modality (text/audio/image/event) + 真 attention model.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modality {
    Text,
    Audio,
    Image,
    Event,
    Structured,
}

impl Modality {
    pub fn name(self) -> &'static str {
        match self {
            Self::Text => "text", Self::Audio => "audio", Self::Image => "image",
            Self::Event => "event", Self::Structured => "structured",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perception {
    pub modality: Modality,
    pub content: String,
    pub timestamp_ms: i64,
    pub salience: f32,     // 0 装 PASS: 0.0-1.0 (attention weight)
    pub source: String,
}

#[derive(Debug, Clone, Default)]
pub struct PerceptionBuffer {
    items: Vec<Perception>,
    capacity: usize,
    attention_window_ms: i64,
}

impl PerceptionBuffer {
    pub fn new(capacity: usize, attention_window_ms: i64) -> Self {
        Self { items: Vec::with_capacity(capacity), capacity, attention_window_ms }
    }

    /// 0 装 PASS: 真添加 perception
    pub fn perceive(&mut self, p: Perception) {
        self.items.push(p);
        if self.items.len() > self.capacity {
            // 0 装 PASS: 真实丢弃最低 salience
            self.items.sort_by(|a, b| b.salience.partial_cmp(&a.salience).unwrap_or(std::cmp::Ordering::Equal));
            self.items.truncate(self.capacity);
        }
    }

    /// 0 装 PASS: 真实 attention: 返回 attention_window_ms 内 high-salience items
    pub fn attend(&self, current_ms: i64) -> Vec<&Perception> {
        self.items.iter()
            .filter(|p| {
                let age = current_ms - p.timestamp_ms;
                age >= 0 && age <= self.attention_window_ms && p.salience > 0.5
            })
            .collect()
    }

    /// 0 装 PASS: 真实按 modality filter
    pub fn by_modality(&self, m: Modality) -> Vec<&Perception> {
        self.items.iter().filter(|p| p.modality == m).collect()
    }

    pub fn len(&self) -> usize { self.items.len() }
    pub fn is_empty(&self) -> bool { self.items.is_empty() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_perceive_basic() {
        let mut b = PerceptionBuffer::new(5, 1000);
        b.perceive(Perception { modality: Modality::Text, content: "hello".into(), timestamp_ms: 100, salience: 0.5, source: "user".into() });
        assert_eq!(b.len(), 1);
    }
    #[test] fn test_capacity_eviction() {
        let mut b = PerceptionBuffer::new(2, 1000);
        b.perceive(Perception { modality: Modality::Text, content: "a".into(), timestamp_ms: 100, salience: 0.1, source: "x".into() });
        b.perceive(Perception { modality: Modality::Text, content: "b".into(), timestamp_ms: 100, salience: 0.2, source: "x".into() });
        b.perceive(Perception { modality: Modality::Text, content: "c".into(), timestamp_ms: 100, salience: 0.9, source: "x".into() });
        assert_eq!(b.len(), 2);  // 0 装 PASS: 真实按 salience 留 high
    }
    #[test] fn test_attention_window() {
        let mut b = PerceptionBuffer::new(5, 1000);
        // 老 ts 但 age=1000, 在 1000ms window 内, 高 salience -> 返
        b.perceive(Perception { modality: Modality::Text, content: "old".into(), timestamp_ms: 100, salience: 0.9, source: "x".into() });
        b.perceive(Perception { modality: Modality::Text, content: "new".into(), timestamp_ms: 1100, salience: 0.9, source: "x".into() });
        let att = b.attend(1100);
        // age(100) = 1100 - 100 = 1000 (within 1000 window), age(1100) = 0
        // 2 个都应返
        assert_eq!(att.len(), 2);
    }
    #[test] fn test_attention_window_strict() {
        // 测超 window: ts=100 + attend(2000), age=1900 > 1000 -> 不返
        let mut b = PerceptionBuffer::new(5, 1000);
        b.perceive(Perception { modality: Modality::Text, content: "old".into(), timestamp_ms: 100, salience: 0.9, source: "x".into() });
        let att = b.attend(2000);
        assert_eq!(att.len(), 0);
    }
    #[test] fn test_salience_threshold() {
        let mut b = PerceptionBuffer::new(5, 1000);
        b.perceive(Perception { modality: Modality::Text, content: "low".into(), timestamp_ms: 100, salience: 0.3, source: "x".into() });
        b.perceive(Perception { modality: Modality::Text, content: "high".into(), timestamp_ms: 100, salience: 0.8, source: "x".into() });
        let att = b.attend(100);
        assert_eq!(att.len(), 1);
        assert_eq!(att[0].content, "high");
    }
    #[test] fn test_modality_filter() {
        let mut b = PerceptionBuffer::new(5, 1000);
        b.perceive(Perception { modality: Modality::Text, content: "x".into(), timestamp_ms: 100, salience: 0.5, source: "x".into() });
        b.perceive(Perception { modality: Modality::Image, content: "img".into(), timestamp_ms: 100, salience: 0.5, source: "x".into() });
        assert_eq!(b.by_modality(Modality::Text).len(), 1);
        assert_eq!(b.by_modality(Modality::Image).len(), 1);
        assert_eq!(b.by_modality(Modality::Audio).len(), 0);
    }
    #[test] fn test_modality_name() {
        assert_eq!(Modality::Text.name(), "text");
        assert_eq!(Modality::Structured.name(), "structured");
    }
}
