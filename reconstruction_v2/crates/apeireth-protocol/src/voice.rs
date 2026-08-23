use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VadState {
    Silence,
    SpeechStarting,
    SpeechOngoing,
    SpeechEnded,
}

pub struct EnergyVad {
    energy_threshold: f32,
    speech_frame_count: usize,
    silence_frame_count: usize,
    state: VadState,
}

impl EnergyVad {
    pub fn new(energy_threshold: f32) -> Self {
        Self {
            energy_threshold,
            speech_frame_count: 0,
            silence_frame_count: 0,
            state: VadState::Silence,
        }
    }

    /// Computes Root Mean Square (RMS) energy from 16-bit PCM audio samples
    pub fn compute_rms(pcm_samples: &[i16]) -> f32 {
        if pcm_samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = pcm_samples.iter().map(|&s| {
            let normalized = s as f64 / 32768.0;
            normalized * normalized
        }).sum();
        (sum_sq / pcm_samples.len() as f64).sqrt() as f32
    }

    /// Processes a single audio frame (e.g. 20ms of 16kHz PCM) and returns current VadState
    pub fn process_frame(&mut self, pcm_samples: &[i16]) -> VadState {
        let rms = Self::compute_rms(pcm_samples);
        let is_voice = rms >= self.energy_threshold;

        match self.state {
            VadState::Silence => {
                if is_voice {
                    self.speech_frame_count += 1;
                    if self.speech_frame_count >= 2 {
                        self.state = VadState::SpeechStarting;
                    }
                } else {
                    self.speech_frame_count = 0;
                }
            }
            VadState::SpeechStarting => {
                if is_voice {
                    self.state = VadState::SpeechOngoing;
                    self.silence_frame_count = 0;
                } else {
                    self.state = VadState::Silence;
                    self.speech_frame_count = 0;
                }
            }
            VadState::SpeechOngoing => {
                if is_voice {
                    self.silence_frame_count = 0;
                } else {
                    self.silence_frame_count += 1;
                    if self.silence_frame_count >= 15 { // ~300ms silence
                        self.state = VadState::SpeechEnded;
                    }
                }
            }
            VadState::SpeechEnded => {
                self.state = VadState::Silence;
                self.speech_frame_count = 0;
                self.silence_frame_count = 0;
            }
        }

        self.state
    }
}

pub struct VoiceDuplexEngine {
    vad: EnergyVad,
    is_assistant_speaking: bool,
    accumulated_speech_samples: Vec<i16>,
}

impl VoiceDuplexEngine {
    pub fn new(energy_threshold: f32) -> Self {
        Self {
            vad: EnergyVad::new(energy_threshold),
            is_assistant_speaking: false,
            accumulated_speech_samples: Vec::new(),
        }
    }

    pub fn set_assistant_speaking(&mut self, speaking: bool) {
        self.is_assistant_speaking = speaking;
    }

    /// Feeds incoming user microphone audio chunk (16kHz 16-bit PCM).
    /// Returns (current_vad_state, is_barge_in_interrupted, optional_completed_speech_buffer)
    pub fn feed_audio_frame(&mut self, pcm_chunk: &[i16]) -> (VadState, bool, Option<Vec<i16>>) {
        let state = self.vad.process_frame(pcm_chunk);
        let mut barge_in = false;
        let mut completed_speech = None;

        if state == VadState::SpeechStarting || state == VadState::SpeechOngoing {
            self.accumulated_speech_samples.extend_from_slice(pcm_chunk);
            if self.is_assistant_speaking {
                barge_in = true;
                self.is_assistant_speaking = false;
            }
        } else if state == VadState::SpeechEnded {
            if !self.accumulated_speech_samples.is_empty() {
                completed_speech = Some(std::mem::take(&mut self.accumulated_speech_samples));
            }
        }

        (state, barge_in, completed_speech)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_vad_and_barge_in() {
        let mut engine = VoiceDuplexEngine::new(0.05);
        engine.set_assistant_speaking(true);

        // Frame of silence
        let silence_frame = vec![0i16; 320]; // 20ms at 16kHz
        let (state, barge_in, _) = engine.feed_audio_frame(&silence_frame);
        assert_eq!(state, VadState::Silence);
        assert!(!barge_in);

        // Frame of speech signal
        let speech_frame = vec![8000i16; 320];
        engine.feed_audio_frame(&speech_frame);
        let (state2, barge_in2, _) = engine.feed_audio_frame(&speech_frame);

        assert!(state2 == VadState::SpeechStarting || state2 == VadState::SpeechOngoing);
        assert!(barge_in2, "Speech while assistant speaking must trigger barge-in interrupt");
    }
}
