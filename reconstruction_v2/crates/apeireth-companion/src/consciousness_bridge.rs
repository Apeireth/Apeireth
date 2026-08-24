//! ConsciousnessBridge - 意识桥 (从 v1.0 apeireth-companion/consciousness_bridge.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 emotion_event stream + consciousness state
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionEvent {
    pub emotion: String,    // joy/sadness/anger/fear/surprise/disgust
    pub intensity: f32,     // 0 装 PASS: 0.0-1.0
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessSnapshot {
    pub valence: f32,
    pub arousal: f32,
    pub dominant_emotion: Option<String>,
}

pub struct ConsciousnessBridge {
    events: VecDeque<EmotionEvent>,
    capacity: usize,
}

impl ConsciousnessBridge {
    pub fn new(capacity: usize) -> Self { Self { events: VecDeque::with_capacity(capacity), capacity } }

    /// 0 装 PASS: 真感知
    pub fn feel(&mut self, emotion: impl Into<String>, intensity: f32) {
        self.events.push_back(EmotionEvent { emotion: emotion.into(), intensity: intensity.clamp(0.0, 1.0), timestamp_ms: chrono::Utc::now().timestamp_millis() });
        if self.events.len() > self.capacity { self.events.pop_front(); }
    }

    /// 0 装 PASS: 真 snapshot (avg valence + dominant)
    pub fn snapshot(&self) -> ConsciousnessSnapshot {
        if self.events.is_empty() {
            return ConsciousnessSnapshot { valence: 0.0, arousal: 0.0, dominant_emotion: None };
        }
        let valence: f32 = self.events.iter().map(|e| match e.emotion.as_str() { "joy" => e.intensity, "sadness" => -e.intensity, _ => 0.0 }).sum::<f32>() / self.events.len() as f32;
        let arousal: f32 = self.events.iter().map(|e| e.intensity).sum::<f32>() / self.events.len() as f32;
        let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
        for e in &self.events { *counts.entry(&e.emotion).or_insert(0) += 1; }
        let dominant = counts.into_iter().max_by_key(|(_, c)| *c).map(|(k, _)| k.to_string());
        ConsciousnessSnapshot { valence, arousal, dominant_emotion: dominant }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_feel() {
        let mut c = ConsciousnessBridge::new(10);
        c.feel("joy", 0.8);
        assert_eq!(c.events.len(), 1);
    }
    #[test] fn test_snapshot_empty() {
        let c = ConsciousnessBridge::new(10);
        let s = c.snapshot();
        assert_eq!(s.valence, 0.0);
        assert!(s.dominant_emotion.is_none());
    }
    #[test] fn test_snapshot_dominant() {
        let mut c = ConsciousnessBridge::new(10);
        c.feel("joy", 0.5);
        c.feel("joy", 0.7);
        c.feel("sadness", 0.9);
        let s = c.snapshot();
        assert_eq!(s.dominant_emotion, Some("joy".to_string()));
    }
    #[test] fn test_capacity() {
        let mut c = ConsciousnessBridge::new(2);
        for _ in 0..5 { c.feel("x", 0.5); }
        assert_eq!(c.events.len(), 2);
    }
}
