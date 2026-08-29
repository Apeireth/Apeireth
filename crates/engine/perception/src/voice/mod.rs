//! Voice perception backend implementations (engine layer).
//!
//! **架构**: trait 在 `apeireth-plugin::perception_backend` (foundation),
//! impl 在本模块 (engine). 单向依赖: perception → plugin.
//!
//! **当前实现**:
//! - `WhisperHttpBackend`: 真实 HTTP 调用 OpenAI/MiniMax Whisper-compatible
//!   `/v1/audio/transcriptions` 端点. 凭证走 `CredentialResolver`.
//!
//! **O-6 三阶审查**:
//! 1. 总体: RC-7 Perception 真 modality, 与 `ProviderCapability` HTTP 模式一致
//! 2. 系统: engine 层持 HTTP 客户端, foundation 层只持 trait 契约
//! 3. 架构: runtime 通过 `Arc<dyn VoiceBackend>` 注入, 多 backend 可选

pub mod emotion_voice;
pub mod whisper_http;

pub use emotion_voice::{
    AcousticParameters, EmotionCategory, EmotionVoiceSynthesizer, PadEmotion,
};
pub use whisper_http::{WhisperHttpBackend, WhisperHttpConfig};
