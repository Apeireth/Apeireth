//! Decay engine: Ebbinghaus-style forgetting curve.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct DecayConfig {
    /// Half-life in hours (default 24 = 1 day)
    pub half_life_hours: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self { half_life_hours: 24.0 }
    }
}

pub struct DecayEngine {
    config: DecayConfig,
}

impl DecayEngine {
    pub fn new() -> Self {
        Self { config: DecayConfig::default() }
    }
    pub fn with_config(config: DecayConfig) -> Self {
        Self { config }
    }

    /// Compute current strength (0.0 - 1.0) given last access time.
    /// strength(t) = 0.5 ^ (elapsed_hours / half_life)
    pub fn strength(&self, last_access: DateTime<Utc>) -> f32 {
        let elapsed_hours = (Utc::now() - last_access).num_seconds() as f64 / 3600.0;
        let strength = 0.5f64.powf(elapsed_hours / self.config.half_life_hours);
        strength as f32
    }

    /// Whether the item should be forgotten (strength below threshold).
    pub fn should_forget(&self, last_access: DateTime<Utc>, threshold: f32) -> bool {
        self.strength(last_access) < threshold
    }
}

impl Default for DecayEngine {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_item_strong() {
        let e = DecayEngine::new();
        let s = e.strength(Utc::now());
        assert!(s > 0.99);
    }

    #[test]
    fn old_item_weak() {
        let e = DecayEngine::with_config(DecayConfig { half_life_hours: 1.0 });
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
}