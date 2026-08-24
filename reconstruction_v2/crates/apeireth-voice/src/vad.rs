use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VadState {
    Silence,
    SpeechStarting,
    Speaking,
    SpeechEnding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    pub energy_threshold: f32,
    pub start_frames_required: usize,
    pub stop_frames_required: usize,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.05,
            start_frames_required: 3,
            stop_frames_required: 10,
        }
    }
}

pub struct VadDetector {
    config: VadConfig,
    state: VadState,
    consecutive_active: usize,
    consecutive_inactive: usize,
}

impl VadDetector {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            state: VadState::Silence,
            consecutive_active: 0,
            consecutive_inactive: 0,
        }
    }

    pub fn state(&self) -> VadState {
        self.state.clone()
    }

    /// Feeds an audio chunk RMS energy into VAD state machine
    pub fn process_energy(&mut self, rms: f32) -> (VadState, bool) {
        let is_active = rms >= self.config.energy_threshold;
        let mut triggered_interruption = false;

        if is_active {
            self.consecutive_active += 1;
            self.consecutive_inactive = 0;

            if self.consecutive_active >= self.config.start_frames_required {
                if self.state != VadState::Speaking {
                    self.state = VadState::Speaking;
                    triggered_interruption = true;
                }
            } else {

                self.state = VadState::SpeechStarting;
            }
        } else {
            self.consecutive_inactive += 1;
            self.consecutive_active = 0;

            if self.consecutive_inactive >= self.config.stop_frames_required {
                self.state = VadState::Silence;
            } else if self.state == VadState::Speaking {
                self.state = VadState::SpeechEnding;
            }
        }

        (self.state.clone(), triggered_interruption)
    }
}
