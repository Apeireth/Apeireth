//! LifeForce - 生命力 (从 v1.0 apeireth-life-force 2,294 LOC 收敛)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LifeForce {
    pub energy: f64, pub vitality: f64, pub activity: f64,
}

impl LifeForce {
    pub const FULL: Self = Self { energy: 1.0, vitality: 1.0, activity: 1.0 };
    pub const DORMANT: Self = Self { energy: 0.1, vitality: 0.1, activity: 0.0 };

    pub fn mean(&self) -> f64 { (self.energy + self.vitality + self.activity) / 3.0 }

    pub fn decay(&self, dt_sec: f64) -> Self {
        Self {
            energy: (self.energy - 0.001 * dt_sec).max(0.0),
            vitality: (self.vitality - 0.0005 * dt_sec).max(0.0),
            activity: (self.activity - 0.01 * dt_sec).max(0.0),
        }
    }

    pub fn stimulate(&self, boost: f64) -> Self {
        Self {
            energy: (self.energy + boost).min(1.0),
            vitality: (self.vitality + boost * 0.5).min(1.0),
            activity: (self.activity + boost * 1.5).min(1.0),
        }
    }
}

impl Default for LifeForce { fn default() -> Self { Self::FULL } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_constants() {
        assert_eq!(LifeForce::FULL.energy, 1.0);
        assert_eq!(LifeForce::DORMANT.activity, 0.0);
    }
    #[test] fn test_decay_floored_at_zero() {
        let lf = LifeForce::FULL;
        let d = lf.decay(1_000_000.0);
        assert_eq!(d.energy, 0.0);
    }
    #[test] fn test_stimulate_capped() {
        let lf = LifeForce::DORMANT;
        assert_eq!(lf.stimulate(10.0).energy, 1.0);
    }
}
