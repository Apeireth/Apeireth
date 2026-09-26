//! Energy (RMS) voice-activity detector.
//!
//! Archived `apeireth-sdk-voice` / v2 `sdk/voice/vad.rs` document three
//! algorithms (Energy / Silence / WebRtc) and a `VadConfig` validator, but
//! `detect()` is `NotImplemented`. This module implements the **Energy**
//! algorithm those configs describe, using [`super::audio_frame::pcm16_rms`]:
//!
//! - frame RMS vs `energy_threshold`
//! - `min_speech_duration_ms` (drop short bursts)
//! - `silence_threshold_ms` (end of utterance)
//! - frame sizes 10 / 20 / 30 ms (WebRTC VAD geometry)
//!
//! WebRtc (Chromium) is **not** ported — it would need a new C/FFI dep.
//! Silence-only mode is the Energy detector with threshold 0 plus the
//! silence-duration rule.
//!
//! Default-off library helper. Not a session owner.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::audio_frame::{duration_ms, pcm16_rms, PCM16_CHANNELS_MONO, PCM16_SAMPLE_RATE_HZ};

/// VAD configuration errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VadError {
    /// `energy_threshold` outside `[0.0, 1.0]`.
    EnergyThresholdOutOfRange,
    /// `silence_threshold_ms` > 10_000.
    SilenceThresholdOutOfRange,
    /// `min_speech_duration_ms` > 10_000.
    MinSpeechOutOfRange,
    /// Frame size not in `{10, 20, 30}` ms.
    InvalidFrameSize { got: u32 },
    /// Empty PCM.
    EmptyAudio,
}

impl fmt::Display for VadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnergyThresholdOutOfRange => {
                write!(f, "energy_threshold out of range [0.0, 1.0]")
            }
            Self::SilenceThresholdOutOfRange => {
                write!(f, "silence_threshold_ms out of range [0, 10000]")
            }
            Self::MinSpeechOutOfRange => {
                write!(f, "min_speech_duration_ms out of range [0, 10000]")
            }
            Self::InvalidFrameSize { got } => {
                write!(f, "frame_size_ms {got} invalid, expected 10/20/30")
            }
            Self::EmptyAudio => write!(f, "empty audio for VAD"),
        }
    }
}

impl std::error::Error for VadError {}

/// Energy VAD configuration (donor `VadConfig` Energy defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyVadConfig {
    /// RMS threshold in `[0.0, 1.0]`. Donor Energy default = 0.05.
    pub energy_threshold: f32,
    /// Trailing silence that ends an utterance (ms). 0 = no hangover.
    pub silence_threshold_ms: u32,
    /// Minimum speech length (ms); shorter bursts are noise.
    pub min_speech_duration_ms: u32,
    /// Analysis frame (ms). Must be 10, 20, or 30.
    pub frame_size_ms: u32,
}

impl EnergyVadConfig {
    /// Donor `VadConfig::default_energy`.
    pub fn default_energy() -> Self {
        Self {
            energy_threshold: 0.05,
            silence_threshold_ms: 0,
            min_speech_duration_ms: 100,
            frame_size_ms: 20,
        }
    }

    /// Donor `VadConfig::default_silence` (threshold 0, 500 ms hangover).
    pub fn default_silence() -> Self {
        Self {
            energy_threshold: 0.0,
            silence_threshold_ms: 500,
            min_speech_duration_ms: 100,
            frame_size_ms: 20,
        }
    }

    /// Validated constructor (donor `VadConfig::custom` Energy path).
    pub fn custom(
        energy_threshold: f32,
        silence_threshold_ms: u32,
        min_speech_duration_ms: u32,
        frame_size_ms: u32,
    ) -> Result<Self, VadError> {
        if !(0.0..=1.0).contains(&energy_threshold) {
            return Err(VadError::EnergyThresholdOutOfRange);
        }
        if silence_threshold_ms > 10_000 {
            return Err(VadError::SilenceThresholdOutOfRange);
        }
        if min_speech_duration_ms > 10_000 {
            return Err(VadError::MinSpeechOutOfRange);
        }
        if !matches!(frame_size_ms, 10 | 20 | 30) {
            return Err(VadError::InvalidFrameSize { got: frame_size_ms });
        }
        Ok(Self {
            energy_threshold,
            silence_threshold_ms,
            min_speech_duration_ms,
            frame_size_ms,
        })
    }

    /// Samples per analysis frame at 16 kHz mono.
    pub fn frame_samples(&self) -> usize {
        (u64::from(PCM16_SAMPLE_RATE_HZ) * u64::from(self.frame_size_ms) / 1000) as usize
    }
}

impl Default for EnergyVadConfig {
    fn default() -> Self {
        Self::default_energy()
    }
}

/// One Energy-VAD result (donor `VadResult` fields that the Energy path can fill).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnergyVadResult {
    /// True when a speech segment meeting `min_speech_duration_ms` was found.
    pub is_speech: bool,
    /// Peak RMS among speech frames (0 when none).
    pub confidence: f32,
    /// Speech duration.
    pub speech_duration: Duration,
    /// Silence duration (frames below threshold).
    pub silence_duration: Duration,
}

impl EnergyVadResult {
    /// Speech / (speech + silence). 0 when both are zero.
    pub fn speech_ratio(&self) -> f32 {
        let total = self.speech_duration.as_millis() + self.silence_duration.as_millis();
        if total == 0 {
            return 0.0;
        }
        self.speech_duration.as_millis() as f32 / total as f32
    }
}

/// Run Energy VAD over a whole PCM16 buffer (offline).
pub fn detect_energy(
    samples: &[i16],
    config: &EnergyVadConfig,
) -> Result<EnergyVadResult, VadError> {
    if samples.is_empty() {
        return Err(VadError::EmptyAudio);
    }
    let frame_len = config.frame_samples().max(1);
    let mut speech_ms: u64 = 0;
    let mut silence_ms: u64 = 0;
    let mut peak: f32 = 0.0;
    let mut current_speech_ms: u64 = 0;
    let mut accepted_speech_ms: u64 = 0;

    for chunk in samples.chunks(frame_len) {
        let frame_ms = duration_ms(chunk.len(), PCM16_SAMPLE_RATE_HZ, PCM16_CHANNELS_MONO);
        let rms = pcm16_rms(chunk);
        let loud = if config.energy_threshold == 0.0 {
            rms > 0.0
        } else {
            rms >= config.energy_threshold
        };
        if loud {
            speech_ms += frame_ms;
            current_speech_ms += frame_ms;
            if rms > peak {
                peak = rms;
            }
        } else {
            silence_ms += frame_ms;
            if current_speech_ms >= u64::from(config.min_speech_duration_ms) {
                accepted_speech_ms += current_speech_ms;
            }
            current_speech_ms = 0;
        }
    }
    if current_speech_ms >= u64::from(config.min_speech_duration_ms) {
        accepted_speech_ms += current_speech_ms;
    }

    // Hangover: if trailing silence is shorter than the threshold, do not
    // treat the utterance as ended — still speech if we already accepted some.
    let hangover_ok = config.silence_threshold_ms == 0
        || silence_ms == 0
        || current_speech_ms > 0
        || accepted_speech_ms > 0;

    let is_speech = accepted_speech_ms > 0 && hangover_ok;
    Ok(EnergyVadResult {
        is_speech,
        confidence: if is_speech { peak } else { 0.0 },
        speech_duration: Duration::from_millis(speech_ms),
        silence_duration: Duration::from_millis(silence_ms),
    })
}

/// Streaming Energy VAD. Feed frames; query [`EnergyVadStream::is_speech`].
#[derive(Debug, Clone)]
pub struct EnergyVadStream {
    config: EnergyVadConfig,
    speech_ms: u64,
    silence_ms: u64,
    in_speech: bool,
}

impl EnergyVadStream {
    pub fn new(config: EnergyVadConfig) -> Self {
        Self {
            config,
            speech_ms: 0,
            silence_ms: 0,
            in_speech: false,
        }
    }

    /// Push one PCM16 frame (any length; duration computed from sample count).
    pub fn push_frame(&mut self, samples: &[i16]) -> bool {
        if samples.is_empty() {
            return self.in_speech;
        }
        let frame_ms = duration_ms(samples.len(), PCM16_SAMPLE_RATE_HZ, PCM16_CHANNELS_MONO);
        let rms = pcm16_rms(samples);
        let loud = if self.config.energy_threshold == 0.0 {
            rms > 0.0
        } else {
            rms >= self.config.energy_threshold
        };
        if loud {
            self.speech_ms += frame_ms;
            self.silence_ms = 0;
            if self.speech_ms >= u64::from(self.config.min_speech_duration_ms) {
                self.in_speech = true;
            }
        } else {
            self.silence_ms += frame_ms;
            if self.in_speech && self.silence_ms >= u64::from(self.config.silence_threshold_ms) {
                self.in_speech = false;
                self.speech_ms = 0;
            }
        }
        self.in_speech
    }

    pub fn is_speech(&self) -> bool {
        self.in_speech
    }

    pub fn reset(&mut self) {
        self.speech_ms = 0;
        self.silence_ms = 0;
        self.in_speech = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amplitude: i16, n: usize) -> Vec<i16> {
        vec![amplitude; n]
    }

    #[test]
    fn default_energy_config() {
        let cfg = EnergyVadConfig::default_energy();
        assert!((cfg.energy_threshold - 0.05).abs() < 0.001);
        assert_eq!(cfg.frame_size_ms, 20);
        assert_eq!(cfg.frame_samples(), 320); // 16k * 20ms / 1000
    }

    #[test]
    fn custom_rejects_bad_threshold_and_frame() {
        assert_eq!(
            EnergyVadConfig::custom(1.5, 0, 100, 20),
            Err(VadError::EnergyThresholdOutOfRange)
        );
        assert_eq!(
            EnergyVadConfig::custom(0.1, 0, 100, 50),
            Err(VadError::InvalidFrameSize { got: 50 })
        );
        assert_eq!(
            EnergyVadConfig::custom(0.1, 20_000, 100, 20),
            Err(VadError::SilenceThresholdOutOfRange)
        );
    }

    #[test]
    fn silence_is_not_speech() {
        let cfg = EnergyVadConfig::default_energy();
        let result = detect_energy(&[0i16; 3200], &cfg).unwrap();
        assert!(!result.is_speech);
        assert_eq!(result.speech_ratio(), 0.0);
    }

    #[test]
    fn loud_tone_is_speech() {
        let cfg = EnergyVadConfig::default_energy();
        // 200 ms of full-scale > min_speech 100 ms
        let samples = tone(i16::MAX, 3200);
        let result = detect_energy(&samples, &cfg).unwrap();
        assert!(result.is_speech);
        assert!(result.confidence > 0.9);
        assert!(result.speech_duration.as_millis() >= 100);
    }

    #[test]
    fn short_burst_below_min_speech_rejected() {
        let cfg = EnergyVadConfig::custom(0.05, 0, 100, 20).unwrap();
        // one 20 ms frame of tone, rest silence — below 100 ms min
        let mut samples = tone(i16::MAX, 320);
        samples.extend_from_slice(&[0i16; 3200]);
        let result = detect_energy(&samples, &cfg).unwrap();
        assert!(!result.is_speech);
    }

    #[test]
    fn empty_audio_errors() {
        assert_eq!(
            detect_energy(&[], &EnergyVadConfig::default()),
            Err(VadError::EmptyAudio)
        );
    }

    #[test]
    fn speech_ratio_three_to_one() {
        let result = EnergyVadResult {
            is_speech: true,
            confidence: 0.95,
            speech_duration: Duration::from_millis(3000),
            silence_duration: Duration::from_millis(1000),
        };
        assert!((result.speech_ratio() - 0.75).abs() < 0.001);
    }

    #[test]
    fn stream_enters_and_leaves_speech() {
        let cfg = EnergyVadConfig::custom(0.05, 40, 20, 20).unwrap();
        let mut stream = EnergyVadStream::new(cfg);
        let loud = tone(i16::MAX, 320);
        let quiet = vec![0i16; 320];
        // 20 ms loud frame meets min_speech_duration_ms = 20.
        assert!(stream.push_frame(&loud));
        assert!(stream.is_speech());
        stream.push_frame(&quiet);
        // 20 ms silence < 40 ms hangover — still speech
        assert!(stream.is_speech());
        stream.push_frame(&quiet);
        // 40 ms silence >= hangover
        assert!(!stream.is_speech());
    }
}
