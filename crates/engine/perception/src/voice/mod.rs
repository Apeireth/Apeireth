//! Voice perception backend implementations (engine layer).
//!
//! **架构**: trait 在 `apeireth-plugin::perception_backend` (foundation),
//! impl 在本模块 (engine). 单向依赖: perception → plugin.
//!
//! **当前实现**:
//! - `WhisperHttpBackend`: 真实 HTTP 调用 OpenAI/MiniMax Whisper-compatible
//!   `/v1/audio/transcriptions` 端点. 凭证走 `CredentialResolver`.
//!
//! **Salvage (agent 09)**: PCM framing, recording / listen-speak session
//! helpers, streaming input-buffer SM, Energy VAD. All default-off library
//! helpers — they do not own the main loop, a transcript, or final response.
//!
//! **O-6 三阶审查**:
//! 1. 总体: RC-7 Perception 真 modality, 与 `ProviderCapability` HTTP 模式一致
//! 2. 系统: engine 层持 HTTP 客户端, foundation 层只持 trait 契约
//! 3. 架构: runtime 通过 `Arc<dyn VoiceBackend>` 注入, 多 backend 可选

pub mod audio_frame;
pub mod audio_session;
pub mod emotion_voice;
pub mod energy_vad;
pub mod minimax_tts;
pub mod stream_frame;
pub mod whisper_http;

pub use audio_frame::{
    duration_ms, hex_decode_audio, pcm16_from_le_bytes, pcm16_rms, pcm16_to_le_bytes,
    split_pcm16_frames, AudioFrameError, Pcm16Buffer, Pcm16Frame, PCM16_CHANNELS_MONO,
    PCM16_FRAME_SAMPLES, PCM16_MAX_AUDIO_SECONDS, PCM16_MAX_DURATION_MS, PCM16_SAMPLE_RATE_HZ,
};
pub use audio_session::{
    NoopSpeechInput, NoopSpeechOutput, RecordingError, RecordingSession, RecordingStatus,
    SpeechInput, SpeechOutput, VoiceSession, VoiceTurn,
};
pub use emotion_voice::{
    AcousticParameters, EmotionCategory, EmotionVoiceSynthesizer, PadEmotion,
};
pub use energy_vad::{detect_energy, EnergyVadConfig, EnergyVadResult, EnergyVadStream, VadError};
pub use minimax_tts::{
    AudioChunk, EmotionToneModulation, MiniMaxLiveTtsClient, MiniMaxTtsRequest, TtsError,
};
pub use stream_frame::{
    encode_audio_append, encode_image_input, InputAudioBuffer, InputBufferState, StreamAudioFormat,
    StreamFrameError, TurnDetection, TurnDetectionKind, DEFAULT_PREFIX_PADDING_MS,
    DEFAULT_SILENCE_DURATION_MS, DEFAULT_TURN_THRESHOLD, MAX_AUDIO_APPEND_BYTES, MAX_IMAGE_BYTES,
};
pub use whisper_http::{WhisperHttpBackend, WhisperHttpConfig};
