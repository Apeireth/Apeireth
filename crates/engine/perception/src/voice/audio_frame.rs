//! PCM16 frame primitives recovered from donor `apeireth-voice`.
//!
//! Donor `VoiceSdk` / Porcupine facades returned `NotImplemented`. The load-bearing
//! pieces are the pvrecorder-style geometry (16 kHz / 16-bit / 512 samples),
//! duration math, empty / too-long guards, little-endian framing, and MiniMax
//! hex-audio decode. Plugin [`apeireth_plugin::perception_backend::AudioBuffer`]
//! is a byte bag without sample-rate schema; this module is the typed helper
//! sitting above that bag.
//!
//! Default-off library: nothing here talks to Whisper, MiniMax HTTP, or the
//! canonical turn loop.

use std::fmt;

use serde::{Deserialize, Serialize};

use apeireth_plugin::perception_backend::AudioBuffer;

/// pvrecorder / Porcupine sample rate (donor `VOICE_SAMPLE_RATE_HZ`).
pub const PCM16_SAMPLE_RATE_HZ: u32 = 16_000;

/// Single analysis frame (donor `VOICE_FRAME_LENGTH`).
pub const PCM16_FRAME_SAMPLES: usize = 512;

/// Single-session recording cap (donor `VOICE_MAX_AUDIO_SECONDS`).
pub const PCM16_MAX_AUDIO_SECONDS: u32 = 30;

/// Mono channel count used by the donor buffer helpers.
pub const PCM16_CHANNELS_MONO: u8 = 1;

/// Maximum duration in milliseconds implied by [`PCM16_MAX_AUDIO_SECONDS`].
#[allow(clippy::cast_lossless)]
pub const PCM16_MAX_DURATION_MS: u64 = PCM16_MAX_AUDIO_SECONDS as u64 * 1000;

/// PCM16 / hex-audio errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFrameError {
    /// No samples / no bytes.
    Empty,
    /// Duration exceeds [`PCM16_MAX_DURATION_MS`].
    TooLong { got_ms: u64, max_ms: u64 },
    /// Sample rate other than the donor hardcode.
    UnsupportedSampleRate { got: u32, expected: u32 },
    /// Channel count other than mono.
    UnsupportedChannels { got: u8 },
    /// PCM byte length is not a multiple of 2.
    OddByteLength { got: usize },
    /// MiniMax hex payload could not be decoded.
    HexDecode(String),
}

impl fmt::Display for AudioFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "audio buffer is empty"),
            Self::TooLong { got_ms, max_ms } => {
                write!(f, "audio too long: got {got_ms}ms, max {max_ms}ms")
            }
            Self::UnsupportedSampleRate { got, expected } => {
                write!(f, "unsupported sample rate: got {got}, expected {expected}")
            }
            Self::UnsupportedChannels { got } => {
                write!(f, "unsupported channel count: {got} (mono only)")
            }
            Self::OddByteLength { got } => {
                write!(f, "pcm16 byte length {got} is not a multiple of 2")
            }
            Self::HexDecode(msg) => write!(f, "hex audio decode error: {msg}"),
        }
    }
}

impl std::error::Error for AudioFrameError {}

/// Duration in milliseconds for `sample_count` PCM samples.
///
/// Donor formula: `samples.len() * 1000 / (sample_rate * channels)`, integer
/// division. Returns 0 when sample rate or channels is 0.
pub fn duration_ms(sample_count: usize, sample_rate_hz: u32, channels: u8) -> u64 {
    if sample_rate_hz == 0 || channels == 0 {
        return 0;
    }
    (sample_count as u64 * 1000) / (u64::from(sample_rate_hz) * u64::from(channels))
}

/// Normalized RMS energy in `[0.0, 1.0]` (`sqrt(mean(s²)) / 32768`).
///
/// Donor VAD named an Energy (RMS) algorithm but left `detect()` unimplemented.
/// This is the actual energy primitive the session / VAD helpers consume.
pub fn pcm16_rms(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples
        .iter()
        .map(|&s| {
            let x = f64::from(s);
            x * x
        })
        .sum();
    let rms = (sum_sq / samples.len() as f64).sqrt();
    (rms / 32768.0).clamp(0.0, 1.0) as f32
}

/// Little-endian PCM16 bytes → samples. Rejects odd lengths.
pub fn pcm16_from_le_bytes(bytes: &[u8]) -> Result<Vec<i16>, AudioFrameError> {
    if bytes.is_empty() {
        return Err(AudioFrameError::Empty);
    }
    if bytes.len() % 2 != 0 {
        return Err(AudioFrameError::OddByteLength { got: bytes.len() });
    }
    Ok(bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect())
}

/// Samples → little-endian PCM16 bytes.
pub fn pcm16_to_le_bytes(samples: &[i16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    out
}

/// MiniMax T2A hex string → audio bytes (whitespace ignored).
///
/// Recovered from `legacy/donor/apeireth-voice/src/minimax_live.rs`. The HTTP
/// client around it is discarded (v2 already owns MiniMax TTS request shaping).
pub fn hex_decode_audio(hex: &str) -> Result<Vec<u8>, AudioFrameError> {
    let cleaned: Vec<u8> = hex.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        return Err(AudioFrameError::HexDecode(format!(
            "hex length {} is odd",
            cleaned.len()
        )));
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let mut index = 0;
    while index < cleaned.len() {
        let hi = hex_nibble(cleaned[index])?;
        let lo = hex_nibble(cleaned[index + 1])?;
        out.push((hi << 4) | lo);
        index += 2;
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Result<u8, AudioFrameError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(AudioFrameError::HexDecode(format!(
            "invalid hex byte 0x{b:02x}"
        ))),
    }
}

/// One PCM16 analysis frame. Geometry defaults to the donor 16 kHz / 512-sample
/// model; a trailing partial frame may be shorter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pcm16Frame {
    /// 16-bit PCM samples.
    pub samples: Vec<i16>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count (mono = 1).
    pub channels: u8,
    /// Integer duration of this frame in milliseconds (may be 0 for short frames).
    pub duration_ms: u64,
}

impl Pcm16Frame {
    /// Build a frame from samples at the donor sample rate. Empty is rejected.
    pub fn from_samples(samples: Vec<i16>) -> Result<Self, AudioFrameError> {
        if samples.is_empty() {
            return Err(AudioFrameError::Empty);
        }
        Ok(Self::from_samples_at(
            samples,
            PCM16_SAMPLE_RATE_HZ,
            PCM16_CHANNELS_MONO,
        ))
    }

    /// Build a frame with explicit rate / channels. Does not reject empty.
    pub fn from_samples_at(samples: Vec<i16>, sample_rate: u32, channels: u8) -> Self {
        let duration_ms = duration_ms(samples.len(), sample_rate, channels);
        Self {
            samples,
            sample_rate,
            channels,
            duration_ms,
        }
    }

    /// Frame length in samples (donor field `frame_length`).
    pub fn frame_length(&self) -> u32 {
        self.samples.len() as u32
    }

    /// Reject sample rates other than [`PCM16_SAMPLE_RATE_HZ`].
    pub fn assert_sample_rate(&self) -> Result<(), AudioFrameError> {
        if self.sample_rate != PCM16_SAMPLE_RATE_HZ {
            return Err(AudioFrameError::UnsupportedSampleRate {
                got: self.sample_rate,
                expected: PCM16_SAMPLE_RATE_HZ,
            });
        }
        Ok(())
    }
}

/// Typed PCM16 buffer with duration / empty / cap guards.
///
/// Distinct from plugin [`AudioBuffer`] (opaque bytes). Convert with
/// [`Pcm16Buffer::into_plugin_buffer`] when handing bytes to Whisper.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pcm16Buffer {
    /// 16-bit PCM samples.
    pub samples: Vec<i16>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u8,
    /// Integer duration in milliseconds.
    pub duration_ms: u64,
}

impl Pcm16Buffer {
    /// Mono 16 kHz buffer. Rejects empty.
    pub fn from_samples(samples: Vec<i16>) -> Result<Self, AudioFrameError> {
        if samples.is_empty() {
            return Err(AudioFrameError::Empty);
        }
        let duration_ms = duration_ms(samples.len(), PCM16_SAMPLE_RATE_HZ, PCM16_CHANNELS_MONO);
        Ok(Self {
            samples,
            sample_rate: PCM16_SAMPLE_RATE_HZ,
            channels: PCM16_CHANNELS_MONO,
            duration_ms,
        })
    }

    /// Donor `assert_sample_rate_hardcode`.
    pub fn assert_sample_rate(&self) -> Result<(), AudioFrameError> {
        if self.sample_rate != PCM16_SAMPLE_RATE_HZ {
            return Err(AudioFrameError::UnsupportedSampleRate {
                got: self.sample_rate,
                expected: PCM16_SAMPLE_RATE_HZ,
            });
        }
        Ok(())
    }

    /// Donor `assert_duration_within_limit` (30 s).
    pub fn assert_duration_within_limit(&self) -> Result<(), AudioFrameError> {
        if self.duration_ms > PCM16_MAX_DURATION_MS {
            return Err(AudioFrameError::TooLong {
                got_ms: self.duration_ms,
                max_ms: PCM16_MAX_DURATION_MS,
            });
        }
        Ok(())
    }

    /// Empty + duration + sample-rate + mono guards.
    pub fn validate(&self) -> Result<(), AudioFrameError> {
        if self.samples.is_empty() {
            return Err(AudioFrameError::Empty);
        }
        if self.channels != PCM16_CHANNELS_MONO {
            return Err(AudioFrameError::UnsupportedChannels { got: self.channels });
        }
        self.assert_sample_rate()?;
        self.assert_duration_within_limit()
    }

    /// Split into 512-sample frames. A trailing partial frame is kept.
    pub fn split_frames(&self) -> Vec<Pcm16Frame> {
        split_pcm16_frames(&self.samples, self.sample_rate, self.channels)
    }

    /// Little-endian bytes for streaming / Whisper upload.
    pub fn to_le_bytes(&self) -> Vec<u8> {
        pcm16_to_le_bytes(&self.samples)
    }

    /// Plugin byte-bag with this buffer's duration. Does not encode a WAV header.
    pub fn into_plugin_buffer(&self) -> AudioBuffer {
        AudioBuffer {
            bytes: self.to_le_bytes(),
            duration_ms: self.duration_ms,
        }
    }
}

/// Split a PCM16 stream into donor-sized frames. Trailing partial frames are kept.
pub fn split_pcm16_frames(samples: &[i16], sample_rate: u32, channels: u8) -> Vec<Pcm16Frame> {
    if samples.is_empty() {
        return Vec::new();
    }
    samples
        .chunks(PCM16_FRAME_SAMPLES)
        .map(|chunk| Pcm16Frame::from_samples_at(chunk.to_vec(), sample_rate, channels))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_ms_one_second_mono_16k() {
        assert_eq!(duration_ms(16_000, PCM16_SAMPLE_RATE_HZ, 1), 1000);
    }

    #[test]
    fn duration_ms_zero_rate_is_zero() {
        assert_eq!(duration_ms(512, 0, 1), 0);
    }

    #[test]
    fn pcm16_buffer_rejects_empty() {
        assert_eq!(
            Pcm16Buffer::from_samples(vec![]),
            Err(AudioFrameError::Empty)
        );
    }

    #[test]
    fn pcm16_buffer_one_second_validates() {
        let buf = Pcm16Buffer::from_samples(vec![0i16; 16_000]).unwrap();
        assert_eq!(buf.duration_ms, 1000);
        assert!(buf.validate().is_ok());
    }

    #[test]
    fn pcm16_buffer_rejects_over_30s() {
        let mut buf = Pcm16Buffer::from_samples(vec![0i16; 16_000]).unwrap();
        buf.duration_ms = 31_000;
        assert_eq!(
            buf.assert_duration_within_limit(),
            Err(AudioFrameError::TooLong {
                got_ms: 31_000,
                max_ms: PCM16_MAX_DURATION_MS,
            })
        );
    }

    #[test]
    fn pcm16_buffer_rejects_wrong_sample_rate() {
        let mut buf = Pcm16Buffer::from_samples(vec![0i16; 512]).unwrap();
        buf.sample_rate = 8_000;
        assert!(matches!(
            buf.assert_sample_rate(),
            Err(AudioFrameError::UnsupportedSampleRate { got: 8_000, .. })
        ));
    }

    #[test]
    fn split_frames_keeps_partial_tail() {
        let samples = vec![1i16; PCM16_FRAME_SAMPLES + 3];
        let frames = split_pcm16_frames(&samples, PCM16_SAMPLE_RATE_HZ, 1);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].samples.len(), PCM16_FRAME_SAMPLES);
        assert_eq!(frames[1].samples.len(), 3);
    }

    #[test]
    fn split_empty_is_empty() {
        assert!(split_pcm16_frames(&[], PCM16_SAMPLE_RATE_HZ, 1).is_empty());
    }

    #[test]
    fn le_bytes_roundtrip() {
        let samples = vec![0i16, -1, 1, 32_767, -32_768];
        let bytes = pcm16_to_le_bytes(&samples);
        assert_eq!(pcm16_from_le_bytes(&bytes).unwrap(), samples);
    }

    #[test]
    fn le_bytes_reject_odd_and_empty() {
        assert!(matches!(
            pcm16_from_le_bytes(&[0x00]),
            Err(AudioFrameError::OddByteLength { got: 1 })
        ));
        assert_eq!(pcm16_from_le_bytes(&[]), Err(AudioFrameError::Empty));
    }

    #[test]
    fn rms_silence_is_zero() {
        assert_eq!(pcm16_rms(&[0; 512]), 0.0);
    }

    #[test]
    fn rms_full_scale_near_one() {
        let rms = pcm16_rms(&[i16::MAX; 64]);
        assert!(rms > 0.99 && rms <= 1.0, "got {rms}");
    }

    #[test]
    fn hex_decode_known_string() {
        assert_eq!(hex_decode_audio("494433").unwrap(), b"ID3");
    }

    #[test]
    fn hex_decode_rejects_odd_length() {
        assert!(matches!(
            hex_decode_audio("abc"),
            Err(AudioFrameError::HexDecode(_))
        ));
    }

    #[test]
    fn hex_decode_rejects_invalid_char() {
        assert!(matches!(
            hex_decode_audio("zz"),
            Err(AudioFrameError::HexDecode(_))
        ));
    }

    #[test]
    fn hex_decode_handles_whitespace() {
        assert_eq!(hex_decode_audio("49 44 33\n").unwrap(), b"ID3");
    }

    #[test]
    fn hex_decode_empty_ok() {
        assert!(hex_decode_audio("").unwrap().is_empty());
    }

    #[test]
    fn into_plugin_buffer_copies_duration() {
        let buf = Pcm16Buffer::from_samples(vec![1i16; 16_000]).unwrap();
        let plugin = buf.into_plugin_buffer();
        assert_eq!(plugin.duration_ms, 1000);
        assert_eq!(plugin.bytes.len(), 32_000);
    }
}
