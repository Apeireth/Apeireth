//! P-arch (2026-08-27): A3 Perception 0 装接口 (O-6 重构批次 Refactor-3).
//!
//! **O-6 重构**: trait 抽象层搬到 `apeireth-plugin` (foundation), impl 留本 crate (engine).
//! 单向依赖: perception → plugin. 5 modality 0 装实现 (Text 真实现 + 3 NotImplemented
//! + Command) 留在本 crate; 完整 5 modality 真实现留 v2.1 路线 (per `v2-unabsorbed-features.md` §A3).
//!
//! **v1 compat**: `apeireth_perception::*` 仍可访问 (re-export), 5 个内部测试 0 破坏.

pub mod vision;
pub mod voice;

// Trait 在 plugin (P-arch 2026-08-27 O-6 重构); 这里 re-export 保持 v1 兼容路径
pub use apeireth_plugin::perception::{
    Attention, PerceptionChannel, PerceptionError, PerceptionEvent, PerceptionInput,
    PerceptionModality, TactileInput, TextInput, ThresholdAttention, TopKAttention, VisionInput,
    VoiceInput,
};
pub use vision::{NoopVisionBackend, XcapVisionBackend, XcapVisionConfig};
pub use voice::{
    detect_energy, encode_audio_append, encode_image_input, hex_decode_audio, pcm16_rms,
    split_pcm16_frames, AudioFrameError, EnergyVadConfig, EnergyVadResult, EnergyVadStream,
    InputAudioBuffer, InputBufferState, NoopSpeechInput, NoopSpeechOutput, Pcm16Buffer,
    Pcm16Frame, RecordingError, RecordingSession, RecordingStatus, SpeechInput, SpeechOutput,
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
}
