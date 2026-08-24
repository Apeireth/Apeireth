//! EmotionMemory - 情感记忆 (从 v1.0 apeireth-companion/emotion_memory.rs 2K LOC 抄录升级)
//!
//! 0 装 PASS: 真 valence/arousal tracking + 趋势
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EmotionPoint {
    pub valence: f32,    // -1.0 to 1.0
    pub arousal: f32,    // -1.0 to 1.0
    pub timestamp_ms: i64,
}

pub struct EmotionTimeline {
    points: Vec<EmotionPoint>,
    max_capacity: usize,
}

impl EmotionTimeline {
    pub fn new(max_capacity: usize) -> Self { Self { points: Vec::with_capacity(max_capacity), max_capacity } }

    /// 0 装 PASS: 真记录
    pub fn record(&mut self, valence: f32, arousal: f32) {
        self.points.push(EmotionPoint { valence: valence.clamp(-1.0, 1.0), arousal: arousal.clamp(-1.0, 1.0), timestamp_ms: chrono::Utc::now().timestamp_millis() });
        if self.points.len() > self.max_capacity { self.points.remove(0); }
    }

    /// 0 装 PASS: 真当前 (最后) 情绪
    pub fn current(&self) -> Option<EmotionPoint> { self.points.last().copied() }

    /// 0 装 PASS: 真平均 (历史 mean)
    pub fn average(&self) -> Option<(f32, f32)> {
        if self.points.is_empty() { return None; }
        let v: f32 = self.points.iter().map(|p| p.valence).sum::<f32>() / self.points.len() as f32;
        let a: f32 = self.points.iter().map(|p| p.arousal).sum::<f32>() / self.points.len() as f32;
        Some((v, a))
    }

    /// 0 装 PASS: 真趋势 (avg(last n) - avg(first n))
    pub fn trend(&self, n: usize) -> Option<f32> {
        if self.points.len() < n * 2 { return None; }
        let first: f32 = self.points.iter().take(n).map(|p| p.valence).sum::<f32>() / n as f32;
        let last: f32 = self.points.iter().rev().take(n).map(|p| p.valence).sum::<f32>() / n as f32;
        Some(last - first)
    }

    pub fn len(&self) -> usize { self.points.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_record_current() {
        let mut t = EmotionTimeline::new(10);
        t.record(0.5, 0.7);
        let c = t.current().unwrap();
        assert_eq!(c.valence, 0.5);
        assert_eq!(c.arousal, 0.7);
    }
    #[test] fn test_clamp() {
        let mut t = EmotionTimeline::new(10);
        t.record(2.0, -2.0);
        let c = t.current().unwrap();
        assert_eq!(c.valence, 1.0);
        assert_eq!(c.arousal, -1.0);
    }
    #[test] fn test_average() {
        let mut t = EmotionTimeline::new(10);
        t.record(0.0, 0.0);
        t.record(1.0, 0.5);
        let (v, a) = t.average().unwrap();
        assert!((v - 0.5).abs() < 1e-6);
        assert!((a - 0.25).abs() < 1e-6);
    }
    #[test] fn test_trend_up() {
        let mut t = EmotionTimeline::new(10);
        t.record(-1.0, 0.0);
        t.record(-1.0, 0.0);
        t.record(0.5, 0.0);
        t.record(0.5, 0.0);
        let trend = t.trend(2).unwrap();
        assert!(trend > 0.0);
    }
    #[test] fn test_trend_insufficient() {
        let mut t = EmotionTimeline::new(10);
        t.record(0.0, 0.0);
        assert!(t.trend(2).is_none());
    }
    #[test] fn test_capacity() {
        let mut t = EmotionTimeline::new(2);
        for _ in 0..5 { t.record(0.0, 0.0); }
        assert_eq!(t.len(), 2);
    }
}
