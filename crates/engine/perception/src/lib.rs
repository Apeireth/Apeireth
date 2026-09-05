//! P-arch (2026-08-27): A3 Perception 0 装接口 (O-6 重构批次 Refactor-3).
//!
//! **O-6 重构**: trait 抽象层搬到 `apeireth-plugin` (foundation), impl 留本 crate (engine).
//! 单向依赖: perception → plugin. 5 modality 0 装实现 (Text 真实现 + 3 NotImplemented
//! + Command) 留在本 crate; 完整 5 modality 真实现留 v2.1 路线 (per `v2-unabsorbed-features.md` §A3).
//!
//! **v1 compat**: `apeireth_perception::*` 仍可访问 (re-export), 5 个内部测试 0 破坏.
//!
//! **Salvage (agent 08)**: this crate owns recovered perception *algorithms*
//! (normalization, capture metadata, screen salience, observation capture).
//! It is **not** an `AgentModule` and does **not** own final response.
//! [`owner::PerceptionOwner`] is default-off and unwired.

pub mod capture;
pub mod normalize;
pub mod observe;
pub mod owner;
pub mod screen;
pub mod vision;
pub mod voice;

// Trait 在 plugin (P-arch 2026-08-27 O-6 重构); 这里 re-export 保持 v1 兼容路径
pub use apeireth_plugin::perception::{
    Attention, PerceptionChannel, PerceptionError, PerceptionEvent, PerceptionInput,
    PerceptionModality, TactileInput, TextInput, ThresholdAttention, TopKAttention, VisionInput,
    VoiceInput,
};
pub use capture::{capture_metadata, CaptureMetadata};
pub use normalize::{
    command_observation, default_attention_threshold, default_top_k, pipeline_events,
    tactile_observation, text_observation, top_k_events, validate_event, vision_observation,
    voice_observation, SignalSource,
};
pub use observe::{ObservationCandidate, ObservationOutcome, ObservationQueue};
pub use owner::PerceptionOwner;
pub use screen::{NoopScreenSource, ScreenEvent, ScreenEventKind, ScreenPerception};
pub use vision::{NoopVisionBackend, XcapVisionBackend, XcapVisionConfig};
pub use voice::{
    detect_energy, encode_audio_append, encode_image_input, hex_decode_audio, pcm16_rms,
    split_pcm16_frames, AudioFrameError, EnergyVadConfig, EnergyVadResult, EnergyVadStream,
    InputAudioBuffer, InputBufferState, NoopSpeechInput, NoopSpeechOutput, Pcm16Buffer, Pcm16Frame,
    RecordingError, RecordingSession, RecordingStatus, SpeechInput, SpeechOutput,
    StreamAudioFormat, StreamFrameError, TurnDetection, TurnDetectionKind, VadError, VoiceSession,
    VoiceTurn, WhisperHttpBackend, WhisperHttpConfig, PCM16_FRAME_SAMPLES, PCM16_SAMPLE_RATE_HZ,
};

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_core::kernel::SessionId;

    /// v1 compat: `apeireth_perception::TextInput` 仍可访问 (re-export)
    #[test]
    fn re_export_text_input_works() {
        let input = TextInput::new(SessionId::new(), "test".into());
        let event = input.next_event().unwrap().expect("first event");
        assert_eq!(event.source, PerceptionModality::Text);
        assert_eq!(event.payload["text"], "test");
    }

    #[test]
    fn default_owner_is_off() {
        let owner = PerceptionOwner::default();
        assert!(!owner.is_enabled());
    }
}
