//! Streaming audio buffer framing recovered from canonical `apeireth-voice::realtime`.
//!
//! Engine realtime.rs is an OpenAI Realtime *schema* (event enums, gpt-realtime
//! dispatch, ephemeral tokens, function calling). Porting that as a second
//! session / provider path is forbidden. The media primitives that *are* real:
//!
//! - per-append size guards (15 MiB audio, 5 MiB image)
//! - empty-append rejection
//! - input-audio-buffer SM: Idle → Buffering → Committed / SpeechActive / Cleared
//! - turn-detection defaults (server VAD threshold 0.5, 500 ms silence, 300 ms pad)
//! - PCM16 / G.711 format tags used by the append path
//!
//! No WebSocket, no token mint, no model dispatch, no `response.create`.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::audio_frame::{pcm16_from_le_bytes, AudioFrameError, PCM16_FRAME_SAMPLES};

/// Maximum audio bytes per append (canonical `REALTIME_MAX_AUDIO_BUFFER_BYTES`).
pub const MAX_AUDIO_APPEND_BYTES: usize = 15 * 1024 * 1024;

/// Maximum image bytes for a multimodal attachment (canonical `REALTIME_MAX_IMAGE_BYTES`).
pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

/// Default VAD activation threshold.
pub const DEFAULT_TURN_THRESHOLD: f32 = 0.5;

/// Default silence duration that ends a turn (ms).
pub const DEFAULT_SILENCE_DURATION_MS: u32 = 500;

/// Default prefix padding included before speech start (ms).
pub const DEFAULT_PREFIX_PADDING_MS: u32 = 300;

/// Framing errors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamFrameError {
    /// Empty payload rejected (canonical used `AudioBufferTooLarge { got: 0 }` /
    /// `ImageTooLarge { got: 0 }` — same shape, clearer name).
    Empty,
    /// Audio append exceeded [`MAX_AUDIO_APPEND_BYTES`].
    AudioTooLarge { got: usize, max: usize },
    /// Image exceeded [`MAX_IMAGE_BYTES`].
    ImageTooLarge { got: usize, max: usize },
    /// Illegal buffer transition.
    IllegalTransition {
        from: InputBufferState,
        attempted: &'static str,
    },
    /// PCM decode failed.
    Audio(String),
}

impl fmt::Display for StreamFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty stream payload rejected"),
            Self::AudioTooLarge { got, max } => {
                write!(
                    f,
                    "audio buffer too large: got {got} bytes, max {max} bytes"
                )
            }
            Self::ImageTooLarge { got, max } => {
                write!(f, "image input too large: got {got} bytes, max {max} bytes")
            }
            Self::IllegalTransition { from, attempted } => {
                write!(
                    f,
                    "illegal input-buffer transition: {from:?} cannot {attempted}"
                )
            }
            Self::Audio(msg) => write!(f, "stream audio error: {msg}"),
        }
    }
}

impl std::error::Error for StreamFrameError {}

/// Audio format tag for a streaming append (canonical `RealtimeAudioFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamAudioFormat {
    /// PCM16, typically 16 kHz (perception) or 24 kHz (OpenAI realtime).
    Pcm16,
    /// G.711 µ-law, 8 kHz (telephony).
    G711Ulaw,
    /// G.711 A-law, 8 kHz (telephony EU).
    G711Alaw,
}

impl Default for StreamAudioFormat {
    fn default() -> Self {
        Self::Pcm16
    }
}

/// Turn-detection kind (canonical `TurnDetectionKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TurnDetectionKind {
    /// Server / local energy VAD.
    ServerVad,
    /// Manual commit via [`InputAudioBuffer::commit`].
    Disabled,
}

/// Server-side VAD configuration (canonical `TurnDetection` defaults).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnDetection {
    /// Detection kind.
    pub kind: TurnDetectionKind,
    /// Activation threshold in `[0.0, 1.0]`.
    pub threshold: f32,
    /// Silence duration to end a turn (ms).
    pub silence_duration_ms: u32,
    /// Prefix padding before speech start (ms).
    pub prefix_padding_ms: u32,
}

impl Default for TurnDetection {
    fn default() -> Self {
        Self {
            kind: TurnDetectionKind::ServerVad,
            threshold: DEFAULT_TURN_THRESHOLD,
            silence_duration_ms: DEFAULT_SILENCE_DURATION_MS,
            prefix_padding_ms: DEFAULT_PREFIX_PADDING_MS,
        }
    }
}

impl TurnDetection {
    /// Manual-turn config (`kind = Disabled`).
    pub fn disabled() -> Self {
        Self {
            kind: TurnDetectionKind::Disabled,
            threshold: 0.0,
            silence_duration_ms: 0,
            prefix_padding_ms: 0,
        }
    }

    /// Clamp threshold into `[0.0, 1.0]`.
    pub fn clamped(mut self) -> Self {
        self.threshold = self.threshold.clamp(0.0, 1.0);
        self
    }
}

/// Validate an audio append payload (size only; codec lives above this layer).
pub fn encode_audio_append(audio_bytes: &[u8]) -> Result<&[u8], StreamFrameError> {
    if audio_bytes.is_empty() {
        return Err(StreamFrameError::Empty);
    }
    if audio_bytes.len() > MAX_AUDIO_APPEND_BYTES {
        return Err(StreamFrameError::AudioTooLarge {
            got: audio_bytes.len(),
            max: MAX_AUDIO_APPEND_BYTES,
        });
    }
    Ok(audio_bytes)
}

/// Validate an image attachment (size only).
pub fn encode_image_input(image_bytes: &[u8]) -> Result<&[u8], StreamFrameError> {
    if image_bytes.is_empty() {
        return Err(StreamFrameError::Empty);
    }
    if image_bytes.len() > MAX_IMAGE_BYTES {
        return Err(StreamFrameError::ImageTooLarge {
            got: image_bytes.len(),
            max: MAX_IMAGE_BYTES,
        });
    }
    Ok(image_bytes)
}

/// Input-audio-buffer states. Engine events (`append` / `commit` / `clear` /
/// `speech_started` / `speech_stopped`) implied this machine but never ran it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputBufferState {
    /// Empty, waiting for the first append.
    Idle,
    /// Accumulating PCM.
    Buffering,
    /// VAD detected speech (optional overlay on buffering).
    SpeechActive,
    /// Caller committed the buffer (terminal until clear).
    Committed,
    /// Explicitly cleared (terminal until a new append, which re-enters Buffering).
    Cleared,
}

/// Streaming input buffer. Accumulates PCM16 little-endian bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct InputAudioBuffer {
    state: InputBufferState,
    bytes: Vec<u8>,
    format: StreamAudioFormat,
    turn: TurnDetection,
}

impl Default for InputAudioBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl InputAudioBuffer {
    /// Empty idle buffer, PCM16, server-VAD defaults.
    pub fn new() -> Self {
        Self {
            state: InputBufferState::Idle,
            bytes: Vec::new(),
            format: StreamAudioFormat::Pcm16,
            turn: TurnDetection::default(),
        }
    }

    /// Manual-commit buffer (`TurnDetectionKind::Disabled`).
    pub fn manual() -> Self {
        Self {
            state: InputBufferState::Idle,
            bytes: Vec::new(),
            format: StreamAudioFormat::Pcm16,
            turn: TurnDetection::disabled(),
        }
    }

    pub fn state(&self) -> InputBufferState {
        self.state
    }

    pub fn format(&self) -> StreamAudioFormat {
        self.format
    }

    pub fn turn_detection(&self) -> &TurnDetection {
        &self.turn
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Append audio bytes. Rejects empty / oversize. Legal from Idle, Buffering,
    /// SpeechActive, or Cleared (Cleared restarts accumulation).
    pub fn append(&mut self, audio_bytes: &[u8]) -> Result<InputBufferState, StreamFrameError> {
        encode_audio_append(audio_bytes)?;
        match self.state {
            InputBufferState::Idle | InputBufferState::Cleared => {
                self.bytes.clear();
                self.bytes.extend_from_slice(audio_bytes);
                self.state = InputBufferState::Buffering;
                Ok(self.state)
            }
            InputBufferState::Buffering | InputBufferState::SpeechActive => {
                let next = self.bytes.len().saturating_add(audio_bytes.len());
                if next > MAX_AUDIO_APPEND_BYTES {
                    return Err(StreamFrameError::AudioTooLarge {
                        got: next,
                        max: MAX_AUDIO_APPEND_BYTES,
                    });
                }
                self.bytes.extend_from_slice(audio_bytes);
                Ok(self.state)
            }
            InputBufferState::Committed => Err(StreamFrameError::IllegalTransition {
                from: self.state,
                attempted: "append",
            }),
        }
    }

    /// Mark speech started (`Buffering → SpeechActive`).
    pub fn speech_started(&mut self) -> Result<InputBufferState, StreamFrameError> {
        match self.state {
            InputBufferState::Buffering | InputBufferState::SpeechActive => {
                self.state = InputBufferState::SpeechActive;
                Ok(self.state)
            }
            other => Err(StreamFrameError::IllegalTransition {
                from: other,
                attempted: "speech_started",
            }),
        }
    }

    /// Mark speech stopped (`SpeechActive → Buffering`).
    pub fn speech_stopped(&mut self) -> Result<InputBufferState, StreamFrameError> {
        match self.state {
            InputBufferState::SpeechActive => {
                self.state = InputBufferState::Buffering;
                Ok(self.state)
            }
            other => Err(StreamFrameError::IllegalTransition {
                from: other,
                attempted: "speech_stopped",
            }),
        }
    }

    /// Commit the buffer. Returns owned PCM16 samples when format is PCM16.
    pub fn commit(&mut self) -> Result<Vec<i16>, StreamFrameError> {
        match self.state {
            InputBufferState::Buffering | InputBufferState::SpeechActive => {
                if self.bytes.is_empty() {
                    return Err(StreamFrameError::Empty);
                }
                let samples = match self.format {
                    StreamAudioFormat::Pcm16 => pcm16_from_le_bytes(&self.bytes)
                        .map_err(|err| StreamFrameError::Audio(err.to_string()))?,
                    StreamAudioFormat::G711Ulaw | StreamAudioFormat::G711Alaw => {
                        return Err(StreamFrameError::Audio(
                            "g.711 decode is not in this helper (codec lives above this layer)"
                                .into(),
                        ));
                    }
                };
                self.state = InputBufferState::Committed;
                Ok(samples)
            }
            other => Err(StreamFrameError::IllegalTransition {
                from: other,
                attempted: "commit",
            }),
        }
    }

    /// Clear the buffer from any non-terminal-locked state. Always succeeds
    /// except from nothing — Idle clear is a no-op that stays Idle.
    pub fn clear(&mut self) -> InputBufferState {
        self.bytes.clear();
        self.state = match self.state {
            InputBufferState::Idle => InputBufferState::Idle,
            _ => InputBufferState::Cleared,
        };
        self.state
    }

    /// How many canonical-sized PCM16 frames the current buffer would split into
    /// (partial tail counts as one).
    pub fn pending_frame_count(&self) -> usize {
        if self.bytes.is_empty() {
            return 0;
        }
        let samples = self.bytes.len() / 2;
        samples.div_ceil(PCM16_FRAME_SAMPLES)
    }
}

impl From<AudioFrameError> for StreamFrameError {
    fn from(err: AudioFrameError) -> Self {
        match err {
            AudioFrameError::Empty => Self::Empty,
            other => Self::Audio(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::audio_frame::pcm16_to_le_bytes;

    #[test]
    fn encode_audio_append_ok() {
        let audio = vec![0u8; 100];
        assert_eq!(encode_audio_append(&audio).unwrap().len(), 100);
    }

    #[test]
    fn encode_audio_append_empty_rejected() {
        assert_eq!(encode_audio_append(&[]), Err(StreamFrameError::Empty));
    }

    #[test]
    fn encode_audio_append_oversize_rejected() {
        let huge = vec![0u8; MAX_AUDIO_APPEND_BYTES + 1];
        assert!(matches!(
            encode_audio_append(&huge),
            Err(StreamFrameError::AudioTooLarge { got, max })
                if got == MAX_AUDIO_APPEND_BYTES + 1 && max == MAX_AUDIO_APPEND_BYTES
        ));
    }

    #[test]
    fn encode_image_empty_and_oversize() {
        assert_eq!(encode_image_input(&[]), Err(StreamFrameError::Empty));
        let huge = vec![0u8; MAX_IMAGE_BYTES + 1];
        assert!(matches!(
            encode_image_input(&huge),
            Err(StreamFrameError::ImageTooLarge { .. })
        ));
    }

    #[test]
    fn turn_detection_defaults() {
        let td = TurnDetection::default();
        assert_eq!(td.kind, TurnDetectionKind::ServerVad);
        assert!((td.threshold - 0.5).abs() < f32::EPSILON);
        assert_eq!(td.silence_duration_ms, 500);
        assert_eq!(td.prefix_padding_ms, 300);
    }

    #[test]
    fn append_commit_pcm16() {
        let samples = vec![1i16, 2, 3, 4];
        let bytes = pcm16_to_le_bytes(&samples);
        let mut buf = InputAudioBuffer::manual();
        assert_eq!(buf.state(), InputBufferState::Idle);
        buf.append(&bytes).unwrap();
        assert_eq!(buf.state(), InputBufferState::Buffering);
        let out = buf.commit().unwrap();
        assert_eq!(out, samples);
        assert_eq!(buf.state(), InputBufferState::Committed);
    }

    #[test]
    fn cannot_append_after_commit() {
        let bytes = pcm16_to_le_bytes(&[1i16; 8]);
        let mut buf = InputAudioBuffer::manual();
        buf.append(&bytes).unwrap();
        buf.commit().unwrap();
        assert!(matches!(
            buf.append(&bytes),
            Err(StreamFrameError::IllegalTransition {
                from: InputBufferState::Committed,
                ..
            })
        ));
    }

    #[test]
    fn speech_started_stopped() {
        let bytes = pcm16_to_le_bytes(&[1i16; 8]);
        let mut buf = InputAudioBuffer::new();
        buf.append(&bytes).unwrap();
        buf.speech_started().unwrap();
        assert_eq!(buf.state(), InputBufferState::SpeechActive);
        buf.speech_stopped().unwrap();
        assert_eq!(buf.state(), InputBufferState::Buffering);
    }

    #[test]
    fn speech_started_from_idle_rejected() {
        let mut buf = InputAudioBuffer::new();
        assert!(matches!(
            buf.speech_started(),
            Err(StreamFrameError::IllegalTransition {
                from: InputBufferState::Idle,
                ..
            })
        ));
    }

    #[test]
    fn clear_from_buffering() {
        let bytes = pcm16_to_le_bytes(&[1i16; 8]);
        let mut buf = InputAudioBuffer::new();
        buf.append(&bytes).unwrap();
        assert_eq!(buf.clear(), InputBufferState::Cleared);
        assert!(buf.is_empty());
        buf.append(&bytes).unwrap();
        assert_eq!(buf.state(), InputBufferState::Buffering);
    }

    #[test]
    fn clear_idle_stays_idle() {
        let mut buf = InputAudioBuffer::new();
        assert_eq!(buf.clear(), InputBufferState::Idle);
    }

    #[test]
    fn commit_empty_rejected() {
        let mut buf = InputAudioBuffer::new();
        // Force buffering with no bytes is not possible via append; commit from idle errors.
        assert!(matches!(
            buf.commit(),
            Err(StreamFrameError::IllegalTransition {
                from: InputBufferState::Idle,
                ..
            })
        ));
    }

    #[test]
    fn pending_frame_count_partial_tail() {
        let samples = vec![1i16; PCM16_FRAME_SAMPLES + 3];
        let bytes = pcm16_to_le_bytes(&samples);
        let mut buf = InputAudioBuffer::new();
        buf.append(&bytes).unwrap();
        assert_eq!(buf.pending_frame_count(), 2);
    }
}
