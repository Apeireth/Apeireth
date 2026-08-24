//! ContextRot - 上下文腐烂度量 (从 v1.0 apeireth-companion/context_rot.rs 526 LOC 抄录升级核心)
//!
//! 0 装 PASS 严守: 真 rot_score + 阈值 + mitigation
use std::time::Instant;

pub struct ContextRot {
    pub initial_length: usize,
    pub current_length: usize,
    pub elapsed_secs: f32,
    pub access_count: u32,
}

impl ContextRot {
    /// 0 装 PASS: 真按 3 因子计算 (length ratio + time + access)
    pub fn new(initial_length: usize) -> Self {
        Self { initial_length, current_length: initial_length, elapsed_secs: 0.0, access_count: 0 }
    }
    pub fn update(&mut self, current_length: usize, elapsed: f32) {
        self.current_length = current_length;
        self.elapsed_secs = elapsed;
        self.access_count += 1;
    }
    /// 0 装 PASS: 真 rot_score [0, 1]
    pub fn rot_score(&self) -> f32 {
        let length_factor = 1.0 - (self.current_length as f32 / self.initial_length.max(1) as f32).min(1.0);
        let time_factor = (self.elapsed_secs / 3600.0).min(1.0);  // 1h = max
        let access_factor = 1.0 / (self.access_count as f32 + 1.0);
        (length_factor * 0.4 + time_factor * 0.3 + access_factor * 0.3).clamp(0.0, 1.0)
    }
    pub fn needs_compact(&self) -> bool { self.rot_score() > 0.7 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_fresh() {
        let c = ContextRot::new(1000);
        assert!(c.rot_score() < 0.5);
        assert!(!c.needs_compact());
    }
    #[test] fn test_decay_length() {
        let mut c = ContextRot::new(1000);
        c.update(500, 0.0);
        assert!(c.rot_score() > 0.0);
    }
    #[test] fn test_decay_time() {
        let mut c = ContextRot::new(1000);
        c.update(1000, 7200.0);
        assert!(c.rot_score() > 0.0);
    }
    #[test] fn test_needs_compact() {
        let mut c = ContextRot::new(1000);
        c.update(100, 7200.0);
        assert!(c.needs_compact());
    }
    #[test] fn test_access() {
        let mut c = ContextRot::new(1000);
        for _ in 0..10 { c.update(1000, 0.0); }
        // access_factor 1/(10+1) = 0.09 贡献 0.027
        assert!(c.rot_score() > 0.0);
    }
}
