#[derive(Debug, Default)]
pub struct RhythmEstimator {
    hourly_histogram: [u32; 24],
}

impl RhythmEstimator {
    pub fn record_activity(&mut self, hour: usize) {
        if hour < 24 {
            self.hourly_histogram[hour] += 1;
        }
    }

    pub fn activity_probability(&self, hour: usize) -> f64 {
        if hour >= 24 { return 0.0; }
        let total: u32 = self.hourly_histogram.iter().sum();
        if total == 0 { return 0.0; }
        self.hourly_histogram[hour] as f64 / total as f64
    }
}

pub struct BorbelyModel {
    w1: f64,
    w2: f64,
    warmth: f64,
    silence_pressure: f64,
}

impl BorbelyModel {
    pub fn new(w1: f64, w2: f64) -> Self {
        Self { w1, w2, warmth: 0.5, silence_pressure: 0.0 }
    }

    pub fn update(&mut self, dt: f64, interacted: bool) {
        if interacted {
            self.silence_pressure = 0.0;
            self.warmth = (self.warmth + 0.1).min(1.0);
        } else {
            self.silence_pressure += dt * 0.01;
            self.warmth = (self.warmth - dt * 0.001).max(0.0);
        }
    }

    pub fn drive(&self) -> f64 {
        self.warmth * self.w1 + self.silence_pressure * self.w2
    }
}

pub enum InitiativeGate {
    UserQuiet,
    QuietHours,
    DailyLimit,
    LlmBudget,
    DepthLow,
    RhythmUnknown,
    RhythmVeto,
    DriveLow,
}

pub struct EmergenceLoop {
    pub rhythm: RhythmEstimator,
    pub drive_model: BorbelyModel,
    pub approach_weight: f64,
}

impl EmergenceLoop {
    pub fn new() -> Self {
        Self {
            rhythm: RhythmEstimator::default(),
            drive_model: BorbelyModel::new(0.6, 0.4),
            approach_weight: 0.5,
        }
    }

    pub fn evaluate_gates(&self, hour: usize, budget: f64) -> Result<(), InitiativeGate> {
        if budget < 0.1 { return Err(InitiativeGate::LlmBudget); }
        if self.drive_model.drive() < 0.3 { return Err(InitiativeGate::DriveLow); }
        if self.rhythm.activity_probability(hour) < 0.1 { return Err(InitiativeGate::RhythmVeto); }
        Ok(())
    }

    pub fn feedback_learning(&mut self, responded: bool) {
        if responded {
            self.approach_weight = (self.approach_weight + 0.1).min(1.0);
        } else {
            self.approach_weight = (self.approach_weight - 0.05).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_gates() {
        let mut loop_ = EmergenceLoop::new();
        loop_.rhythm.record_activity(10);
        loop_.drive_model.update(100.0, false);
        assert!(loop_.evaluate_gates(10, 1.0).is_ok());
    }
}
