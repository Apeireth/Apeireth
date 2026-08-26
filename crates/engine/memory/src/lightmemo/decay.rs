//! Decay engine: Ebbinghaus-style forgetting curve.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use apeireth_core::clock::{Clock, SystemClock};
use chrono::{DateTime, Utc};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DecayConfig {
    /// Half-life in hours (default 24 = 1 day)
    pub half_life_hours: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            half_life_hours: 24.0,
        }
    }
}

pub struct DecayEngine {
    config: DecayConfig,
    /// 可注入时钟: 默认系统时钟; 模拟/测试注入 VirtualClock 快进 (0 真等待).
    clock: Arc<dyn Clock>,
}

impl DecayEngine {
    pub fn new() -> Self {
        Self {
            config: DecayConfig::default(),
            clock: Arc::new(SystemClock),
        }
    }
    pub fn with_config(config: DecayConfig) -> Self {
        Self {
            config,
            clock: Arc::new(SystemClock),
        }
    }
    /// 注入时钟版 (虚拟时间模拟/测试).
    pub fn with_config_and_clock(config: DecayConfig, clock: Arc<dyn Clock>) -> Self {
        Self { config, clock }
    }

    /// Compute current strength (0.0 - 1.0) given last access time.
    /// strength(t) = 0.5 ^ (elapsed_hours / half_life)
    pub fn strength(&self, last_access: DateTime<Utc>) -> f32 {
        let elapsed_hours = (self.clock.now() - last_access).num_seconds() as f64 / 3600.0;
        let strength = 0.5f64.powf(elapsed_hours / self.config.half_life_hours);
        strength as f32
    }

    /// Whether the item should be forgotten (strength below threshold).
    pub fn should_forget(&self, last_access: DateTime<Utc>, threshold: f32) -> bool {
        self.strength(last_access) < threshold
    }
}

impl Default for DecayEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::clock::VirtualClock;
    use chrono::TimeZone;

    fn vclock() -> VirtualClock {
        VirtualClock::new(Utc.with_ymd_and_hms(2026, 8, 16, 6, 0, 0).single().unwrap())
    }

    #[test]
    fn fresh_item_strong() {
        let e = DecayEngine::new();
        let s = e.strength(Utc::now());
        assert!(s > 0.99);
    }

    #[test]
    fn old_item_weak() {
        let e = DecayEngine::with_config(DecayConfig {
            half_life_hours: 1.0,
        });
        let old = Utc::now() - chrono::Duration::hours(10);
        let s = e.strength(old);
        assert!(s < 0.01);
    }

    #[test]
    fn half_life_24h() {
        let e = DecayEngine::new();
        let yesterday = Utc::now() - chrono::Duration::hours(24);
        let s = e.strength(yesterday);
        assert!((s - 0.5).abs() < 0.05);
    }

    #[test]
    fn should_forget_threshold() {
        let e = DecayEngine::new();
        let very_old = Utc::now() - chrono::Duration::days(30);
        assert!(e.should_forget(very_old, 0.5));
    }

    #[test]
    fn should_not_forget_fresh() {
        let e = DecayEngine::new();
        let fresh = Utc::now();
        assert!(!e.should_forget(fresh, 0.5));
    }

    #[test]
    fn virtual_clock_half_life_without_waiting() {
        // 虚拟时钟: 0 真等待验证 24h 半衰期
        let vc = vclock();
        let e = DecayEngine::with_config_and_clock(DecayConfig::default(), Arc::new(vc.clone()));
        let last = vc.current();
        assert!((e.strength(last) - 1.0).abs() < 1e-6, "刚访问强度应接近 1");
        vc.advance(chrono::Duration::hours(24));
        let s = e.strength(last);
        assert!((s - 0.5).abs() < 0.01, "24h 半衰期应 ≈0.5, 实际 {s}");
        // 快进 30 天 → 应被遗忘
        vc.advance(chrono::Duration::days(29));
        assert!(e.should_forget(last, 0.5), "30 天后应低于遗忘阈值");
    }
}
