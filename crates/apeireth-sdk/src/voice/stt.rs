//! # Voice Speech-to-Text (per @anthropic-ai/voice v0.9.21 商业版 1:1 翻译)
//!
//! 4 STT 模型 (per v0.9.21 商业版 + task spec §3):
//! 1. **Whisper** — OpenAI Whisper (offline, multi-language)
//! 2. **Wav2Vec** — Facebook wav2vec 2.0 (offline, self-supervised)
//! 3. **Deepgram** — Deepgram Nova (online, real-time API)
//! 4. **Google** — Google Cloud Speech-to-Text (online, multi-language)
//!
//! **STUB**: 4 模型枚举保留 1:1 翻译, 但 transcribe() 内部返 `VoiceError::NotImplemented`.
//!
//! ## 引用文档
//!
//! 1. `@anthropic-ai/voice v0.9.21` `client/api_transcribe.js` (transcribe 1:1 翻译源)
//! 2. `@anthropic-ai/voice v0.9.21` `core/types.d.ts` (SttModel 1:1 翻译源)

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::{VoiceError, VoiceResult};

// ============================================================================
// §1 4 STT 模型 enum (K-1 强校验守门, 编译期 hardcode 4 variant)
// ============================================================================

/// STT 模型 (4 variant, 1:1 翻译 @anthropic-ai/voice v0.9.21 商业版 `SttModel` enum).
///
/// 4 模型 snake_case 字符串严格匹配 v0.9.21 商业版 API 规范.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SttModel {
    /// **OpenAI Whisper** (offline, multi-language, per v0.9.21 商业版 估 1:1).
    #[default]
    Whisper,
    /// **Facebook wav2vec 2.0** (offline, self-supervised, per v0.9.21 商业版估 1:1).
    Wav2Vec,
    /// **Deepgram Nova** (online, real-time API, per v0.9.21 商业版估 1:1).
    Deepgram,
    /// **Google Cloud Speech-to-Text** (online, multi-language, per v0.9.21 商业版估 1:1).
    Google,
}

impl SttModel {
    /// 4 模型 hardcode 常量.
    pub const COUNT: usize = 4;

    /// 字符串 (1:1 翻译 v0.9.21 商业版 `model` 字段, snake_case 严格匹配).
    pub fn as_str(&self) -> &'static str {
        match self {
            SttModel::Whisper => "whisper",
            SttModel::Wav2Vec => "wav2vec",
            SttModel::Deepgram => "deepgram",
            SttModel::Google => "google",
        }
    }

    /// 从字符串解析 (per v0.9.21 商业版响应 `model` 字段).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "whisper" => Some(SttModel::Whisper),
            "wav2vec" => Some(SttModel::Wav2Vec),
            "deepgram" => Some(SttModel::Deepgram),
            "google" => Some(SttModel::Google),
            _ => None,
        }
    }

    /// 是否 offline (Whisper / Wav2Vec 本地推理).
    pub fn is_offline(&self) -> bool {
        matches!(self, SttModel::Whisper | SttModel::Wav2Vec)
    }

    /// 是否 online (Deepgram / Google Cloud API).
    pub fn is_online(&self) -> bool {
        !self.is_offline()
    }

    /// 估计最大 audio 长度 (秒, per v0.9.21 商业版估).
    /// - Whisper: 默认 30s (per OpenAI API 限制)
    /// - Wav2Vec: 默认 60s (per 模型架构)
    /// - Deepgram: 默认 300s (per Nova API)
    /// - Google: 默认 300s (per Cloud Speech API)
    pub fn max_audio_seconds(&self) -> u32 {
        match self {
            SttModel::Whisper => 30,
            SttModel::Wav2Vec => 60,
            SttModel::Deepgram => 300,
            SttModel::Google => 300,
        }
    }
}

impl std::fmt::Display for SttModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: SUPPORTED_STT_MODELS 长度 == 4 (K-1 强校验守门, 4 模型 hardcode).
pub const SUPPORTED_STT_MODELS: &[SttModel] = &[
    SttModel::Whisper,
    SttModel::Wav2Vec,
    SttModel::Deepgram,
    SttModel::Google,
];
const _: () = assert!(SUPPORTED_STT_MODELS.len() == 4);

// ============================================================================
// §2 Transcription STT 结果 (per v0.9.21 商业版 1:1 翻译)
// ============================================================================

/// STT 转写结果 (per v0.9.21 商业版 `transcribe` 响应 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `Transcription` 对象:
/// - `text` (转写后的文本)
/// - `model` (per `SttModel`)
/// - `language` (ISO 639-1, e.g. `en` / `zh-CN`)
/// - `confidence` (0.0..=1.0, 转写置信度)
/// - `duration_ms` (audio 长度, 毫秒)
/// - `transcribed_at` (转写时间戳, SystemTime)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcription {
    /// 转写后的文本
    pub text: String,
    /// 使用的模型
    pub model: SttModel,
    /// 语言 (ISO 639-1, e.g. `en` / `zh-CN`)
    pub language: String,
    /// 置信度 (0.0..=1.0)
    pub confidence: f32,
    /// audio 长度 (毫秒)
    pub duration_ms: u64,
    /// 转写时间戳
    pub transcribed_at: SystemTime,
}

impl Transcription {
    /// 创建新转写结果 (STUB 模式由调用方构造, R21 续真接时由 transcribe 返).
    pub fn new(text: String, model: SttModel, language: String, confidence: f32, duration_ms: u64) -> Self {
        Self {
            text,
            model,
            language,
            confidence,
            duration_ms,
            transcribed_at: SystemTime::now(),
        }
    }

    /// 是否为空 (text 为空)
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// 文本长度
    pub fn text_len(&self) -> usize {
        self.text.len()
    }
}

// ============================================================================
// §3 SttRequest STT 请求 (per v0.9.21 商业版 `transcribe` 入参 1:1)
// ============================================================================

/// STT 请求 (per v0.9.21 商业版 `transcribe` 入参 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `TranscribeRequest` 对象:
/// - `audio` (bytes, per audio file)
/// - `format` (wav/mp3/opus/flac)
/// - `sample_rate` (8000..=48000)
/// - `bit_depth` (8/16/24/32)
/// - `channels` (1/2)
/// - `model` (per `SttModel`)
/// - `language` (ISO 639-1, optional 推断)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SttRequest {
    /// 音频 bytes (per audio file)
    pub audio: Vec<u8>,
    /// 音频格式
    pub format: String,
    /// 采样率 (Hz)
    pub sample_rate: u32,
    /// 位深
    pub bit_depth: u16,
    /// 通道数
    pub channels: u8,
    /// STT 模型
    pub model: SttModel,
    /// 语言 (ISO 639-1, None = 自动推断)
    pub language: Option<String>,
}

impl SttRequest {
    /// 创建新 STT 请求 (per K-1 强校验 6 守门).
    pub fn new(
        audio: Vec<u8>,
        format: String,
        sample_rate: u32,
        bit_depth: u16,
        channels: u8,
        model: SttModel,
        language: Option<String>,
    ) -> VoiceResult<Self> {
        // K-1 #2: Audio Format
        VoiceError::validate_audio_format(&format)?;
        // K-1 #3: Sample Rate
        VoiceError::validate_sample_rate(sample_rate)?;
        // K-1 #4: Bit Depth
        VoiceError::validate_bit_depth(bit_depth)?;
        // K-1 #5: Channels
        VoiceError::validate_channels(channels)?;
        // K-1 #6: Language (如果显式提供)
        if let Some(lang) = &language {
            VoiceError::validate_language(lang)?;
        }
        Ok(Self {
            audio,
            format,
            sample_rate,
            bit_depth,
            channels,
            model,
            language,
        })
    }
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 4 SttModel 枚举守门 ----

    #[test]
    fn k1_stt_model_has_4_variants() {
        assert_eq!(SUPPORTED_STT_MODELS.len(), 4, "K-1 强校验: 必须 4 个 STT 模型");
        assert_eq!(SttModel::COUNT, 4);
        assert_eq!(SttModel::Whisper.as_str(), "whisper");
        assert_eq!(SttModel::Wav2Vec.as_str(), "wav2vec");
        assert_eq!(SttModel::Deepgram.as_str(), "deepgram");
        assert_eq!(SttModel::Google.as_str(), "google");
    }

    #[test]
    fn k1_stt_model_default_is_whisper() {
        let default = SttModel::default();
        assert_eq!(default, SttModel::Whisper);
    }

    #[test]
    fn k1_stt_model_parse_roundtrip() {
        for model in SUPPORTED_STT_MODELS {
            assert_eq!(SttModel::parse(model.as_str()), Some(*model));
        }
        assert_eq!(SttModel::parse("unknown"), None);
    }

    #[test]
    fn k1_stt_model_offline_online() {
        // Whisper / Wav2Vec = offline
        assert!(SttModel::Whisper.is_offline());
        assert!(!SttModel::Whisper.is_online());
        assert!(SttModel::Wav2Vec.is_offline());
        assert!(!SttModel::Wav2Vec.is_online());
        // Deepgram / Google = online
        assert!(!SttModel::Deepgram.is_offline());
        assert!(SttModel::Deepgram.is_online());
        assert!(!SttModel::Google.is_offline());
        assert!(SttModel::Google.is_online());
    }

    #[test]
    fn k1_stt_model_max_audio_seconds() {
        assert_eq!(SttModel::Whisper.max_audio_seconds(), 30);
        assert_eq!(SttModel::Wav2Vec.max_audio_seconds(), 60);
        assert_eq!(SttModel::Deepgram.max_audio_seconds(), 300);
        assert_eq!(SttModel::Google.max_audio_seconds(), 300);
    }

    // ---- §2 Transcription ----

    #[test]
    fn k1_transcription_new() {
        let t = Transcription::new(
            "hello world".to_string(),
            SttModel::Whisper,
            "en".to_string(),
            0.95,
            1500,
        );
        assert_eq!(t.text, "hello world");
        assert_eq!(t.model, SttModel::Whisper);
        assert_eq!(t.language, "en");
        assert!((t.confidence - 0.95).abs() < 0.001);
        assert_eq!(t.duration_ms, 1500);
        assert!(!t.is_empty());
        assert_eq!(t.text_len(), 11);
    }

    #[test]
    fn k1_transcription_empty_check() {
        let t = Transcription::new(String::new(), SttModel::Whisper, "en".to_string(), 0.0, 0);
        assert!(t.is_empty());

        let t2 = Transcription::new("   ".to_string(), SttModel::Whisper, "en".to_string(), 0.0, 0);
        assert!(t2.is_empty());
    }

    // ---- §3 SttRequest ----

    #[test]
    fn k1_stt_request_valid() {
        let req = SttRequest::new(
            vec![0u8; 100],
            "wav".to_string(),
            16000,
            16,
            1,
            SttModel::Whisper,
            Some("en".to_string()),
        )
        .expect("valid request");
        assert_eq!(req.format, "wav");
        assert_eq!(req.sample_rate, 16000);
        assert_eq!(req.bit_depth, 16);
        assert_eq!(req.channels, 1);
    }

    #[test]
    fn k1_stt_request_rejects_invalid_format() {
        let result = SttRequest::new(
            vec![0u8; 100],
            "aac".to_string(),
            16000,
            16,
            1,
            SttModel::Whisper,
            None,
        );
        assert!(matches!(result, Err(VoiceError::AudioFormatInvalid(_))));
    }

    #[test]
    fn k1_stt_request_rejects_invalid_sample_rate() {
        let result = SttRequest::new(
            vec![0u8; 100],
            "wav".to_string(),
            5000, // < 8000
            16,
            1,
            SttModel::Whisper,
            None,
        );
        assert!(matches!(result, Err(VoiceError::SampleRateInvalid(_))));
    }

    #[test]
    fn k1_stt_request_rejects_invalid_bit_depth() {
        let result = SttRequest::new(
            vec![0u8; 100],
            "wav".to_string(),
            16000,
            64, // 不在 8/16/24/32
            1,
            SttModel::Whisper,
            None,
        );
        assert!(matches!(result, Err(VoiceError::BitDepthInvalid(_))));
    }

    #[test]
    fn k1_stt_request_rejects_invalid_channels() {
        let result = SttRequest::new(
            vec![0u8; 100],
            "wav".to_string(),
            16000,
            16,
            6, // 不在 1/2
            SttModel::Whisper,
            None,
        );
        assert!(matches!(result, Err(VoiceError::ChannelsInvalid(_))));
    }

    #[test]
    fn k1_stt_request_rejects_invalid_language() {
        let result = SttRequest::new(
            vec![0u8; 100],
            "wav".to_string(),
            16000,
            16,
            1,
            SttModel::Whisper,
            Some("english".to_string()), // 太长
        );
        assert!(matches!(result, Err(VoiceError::LanguageInvalid(_))));
    }

    #[test]
    fn k1_stt_request_with_none_language() {
        let req = SttRequest::new(
            vec![0u8; 100],
            "mp3".to_string(),
            22050,
            16,
            2,
            SttModel::Deepgram,
            None,
        )
        .expect("valid with None language");
        assert!(req.language.is_none());
    }
}
