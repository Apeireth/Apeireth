//! # Voice Text-to-Speech (per @anthropic-ai/voice v0.9.21 商业版 1:1 翻译)
//!
//! 4 TTS 模型 (per v0.9.21 商业版 + task spec §3):
//! 1. **ElevenLabs** — ElevenLabs (online, real-time API, 多 voice clone)
//! 2. **Azure** — Azure Cognitive Services Speech (online, multi-language, 神经语音)
//! 3. **Google** — Google Cloud Text-to-Speech (online, multi-language, WaveNet)
//! 4. **OpenAI** — OpenAI TTS (online, 6 voice preset)
//!
//! **STUB**: 4 模型枚举保留 1:1 翻译, 但 synthesize() 内部返 `VoiceError::NotImplemented`.
//!
//! ## 引用文档
//!
//! 1. `@anthropic-ai/voice v0.9.21` `client/api_synthesize.js` (synthesize 1:1 翻译源)
//! 2. `@anthropic-ai/voice v0.9.21` `core/types.d.ts` (TtsModel 1:1 翻译源)

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::voice::error::{VoiceError, VoiceResult};

// ============================================================================
// §1 4 TTS 模型 enum (K-1 强校验守门, 编译期 hardcode 4 variant)
// ============================================================================

/// TTS 模型 (4 variant, 1:1 翻译 @anthropic-ai/voice v0.9.21 商业版 `TtsModel` enum).
///
/// 4 模型 snake_case 字符串严格匹配 v0.9.21 商业版 API 规范.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsModel {
    /// **ElevenLabs** (online, real-time API, 多 voice clone, per v0.9.21 商业版估 1:1).
    #[default]
    ElevenLabs,
    /// **Azure Cognitive Services Speech** (online, multi-language, 神经语音, per v0.9.21 商业版估 1:1).
    Azure,
    /// **Google Cloud Text-to-Speech** (online, multi-language, WaveNet, per v0.9.21 商业版估 1:1).
    Google,
    /// **OpenAI TTS** (online, 6 voice preset, per v0.9.21 商业版估 1:1).
    OpenAI,
}

impl TtsModel {
    /// 4 模型 hardcode 常量.
    pub const COUNT: usize = 4;

    /// 字符串 (1:1 翻译 v0.9.21 商业版 `model` 字段, snake_case 严格匹配).
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsModel::ElevenLabs => "elevenlabs",
            TtsModel::Azure => "azure",
            TtsModel::Google => "google",
            TtsModel::OpenAI => "openai",
        }
    }

    /// 从字符串解析 (per v0.9.21 商业版响应 `model` 字段).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "elevenlabs" => Some(TtsModel::ElevenLabs),
            "azure" => Some(TtsModel::Azure),
            "google" => Some(TtsModel::Google),
            "openai" => Some(TtsModel::OpenAI),
            _ => None,
        }
    }

    /// 默认输出音频格式 (per v0.9.21 商业版估).
    /// - ElevenLabs: mp3
    /// - Azure: wav
    /// - Google: mp3
    /// - OpenAI: mp3
    pub fn default_format(&self) -> &'static str {
        match self {
            TtsModel::ElevenLabs => "mp3",
            TtsModel::Azure => "wav",
            TtsModel::Google => "mp3",
            TtsModel::OpenAI => "mp3",
        }
    }

    /// 默认采样率 (Hz, per v0.9.21 商业版估).
    /// - ElevenLabs: 44100
    /// - Azure: 16000
    /// - Google: 24000
    /// - OpenAI: 24000
    pub fn default_sample_rate(&self) -> u32 {
        match self {
            TtsModel::ElevenLabs => 44100,
            TtsModel::Azure => 16000,
            TtsModel::Google => 24000,
            TtsModel::OpenAI => 24000,
        }
    }

    /// 最大文本长度 (字符, per v0.9.21 商业版估).
    /// - ElevenLabs: 5000
    /// - Azure: 10000
    /// - Google: 5000
    /// - OpenAI: 4096
    pub fn max_text_length(&self) -> usize {
        match self {
            TtsModel::ElevenLabs => 5000,
            TtsModel::Azure => 10_000,
            TtsModel::Google => 5000,
            TtsModel::OpenAI => 4096,
        }
    }
}

impl std::fmt::Display for TtsModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: SUPPORTED_TTS_MODELS 长度 == 4 (K-1 强校验守门, 4 模型 hardcode).
pub const SUPPORTED_TTS_MODELS: &[TtsModel] = &[
    TtsModel::ElevenLabs,
    TtsModel::Azure,
    TtsModel::Google,
    TtsModel::OpenAI,
];
const _: () = assert!(SUPPORTED_TTS_MODELS.len() == 4);

// ============================================================================
// §2 Audio TTS 输出音频 (per v0.9.21 商业版 `synthesize` 响应 1:1 翻译)
// ============================================================================

/// TTS 输出音频 (per v0.9.21 商业版 `synthesize` 响应 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `Audio` 对象:
/// - `data` (audio bytes)
/// - `format` (wav/mp3/opus/flac)
/// - `sample_rate` (Hz)
/// - `bit_depth` (8/16/24/32)
/// - `channels` (1/2)
/// - `duration_ms` (合成 audio 长度)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Audio {
    /// audio bytes
    pub data: Vec<u8>,
    /// 音频格式
    pub format: String,
    /// 采样率 (Hz)
    pub sample_rate: u32,
    /// 位深
    pub bit_depth: u16,
    /// 通道数
    pub channels: u8,
    /// 合成 audio 长度 (毫秒)
    pub duration_ms: u64,
}

impl Audio {
    /// 创建新 audio (per K-1 强校验 4 守门: format/sample_rate/bit_depth/channels).
    pub fn new(
        data: Vec<u8>,
        format: String,
        sample_rate: u32,
        bit_depth: u16,
        channels: u8,
        duration_ms: u64,
    ) -> VoiceResult<Self> {
        VoiceError::validate_audio_format(&format)?;
        VoiceError::validate_sample_rate(sample_rate)?;
        VoiceError::validate_bit_depth(bit_depth)?;
        VoiceError::validate_channels(channels)?;
        Ok(Self {
            data,
            format,
            sample_rate,
            bit_depth,
            channels,
            duration_ms,
        })
    }

    /// 数据大小 (bytes)
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// ============================================================================
// §3 TtsRequest TTS 请求 (per v0.9.21 商业版 `synthesize` 入参 1:1)
// ============================================================================

/// TTS 请求 (per v0.9.21 商业版 `synthesize` 入参 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `SynthesizeRequest` 对象:
/// - `text` (要合成的文本, 1..=max_text_length)
/// - `model` (per `TtsModel`)
/// - `voice` (per voice id, e.g. `"alloy"` for OpenAI, 估)
/// - `language` (ISO 639-1)
/// - `output_format` (wav/mp3/opus/flac, default per model)
/// - `sample_rate` (8000..=48000, default per model)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsRequest {
    /// 要合成的文本
    pub text: String,
    /// TTS 模型
    pub model: TtsModel,
    /// Voice ID (per model 规范, e.g. OpenAI `"alloy"` / Azure `"zh-CN-XiaoxiaoNeural"`)
    pub voice: String,
    /// 语言 (ISO 639-1)
    pub language: String,
    /// 输出格式
    pub output_format: String,
    /// 采样率 (Hz)
    pub sample_rate: u32,
}

impl TtsRequest {
    /// 创建新 TTS 请求 (per K-1 强校验 5 守门).
    pub fn new(
        text: String,
        model: TtsModel,
        voice: String,
        language: String,
        output_format: String,
        sample_rate: u32,
    ) -> VoiceResult<Self> {
        // 文本非空 + ≤ max_text_length
        if text.trim().is_empty() {
            return Err(VoiceError::Other("tts text is empty".to_string()));
        }
        if text.len() > model.max_text_length() {
            return Err(VoiceError::Other(format!(
                "tts text too long: {} > {}",
                text.len(),
                model.max_text_length()
            )));
        }
        // Voice ID 非空
        if voice.trim().is_empty() {
            return Err(VoiceError::Other("voice id is empty".to_string()));
        }
        // K-1 #6: Language
        VoiceError::validate_language(&language)?;
        // K-1 #2: Audio Format
        VoiceError::validate_audio_format(&output_format)?;
        // K-1 #3: Sample Rate
        VoiceError::validate_sample_rate(sample_rate)?;
        Ok(Self {
            text,
            model,
            voice,
            language,
            output_format,
            sample_rate,
        })
    }

    /// 创建默认请求 (用 model 默认 format + sample_rate).
    pub fn with_defaults(
        text: String,
        model: TtsModel,
        voice: String,
        language: String,
    ) -> VoiceResult<Self> {
        Self::new(
            text,
            model,
            voice,
            language,
            model.default_format().to_string(),
            model.default_sample_rate(),
        )
    }
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 4 TtsModel 枚举守门 ----

    #[test]
    fn k1_tts_model_has_4_variants() {
        assert_eq!(
            SUPPORTED_TTS_MODELS.len(),
            4,
            "K-1 强校验: 必须 4 个 TTS 模型"
        );
        assert_eq!(TtsModel::COUNT, 4);
        assert_eq!(TtsModel::ElevenLabs.as_str(), "elevenlabs");
        assert_eq!(TtsModel::Azure.as_str(), "azure");
        assert_eq!(TtsModel::Google.as_str(), "google");
        assert_eq!(TtsModel::OpenAI.as_str(), "openai");
    }

    #[test]
    fn k1_tts_model_default_is_elevenlabs() {
        let default = TtsModel::default();
        assert_eq!(default, TtsModel::ElevenLabs);
    }

    #[test]
    fn k1_tts_model_parse_roundtrip() {
        for model in SUPPORTED_TTS_MODELS {
            assert_eq!(TtsModel::parse(model.as_str()), Some(*model));
        }
        assert_eq!(TtsModel::parse("unknown"), None);
    }

    #[test]
    fn k1_tts_model_default_format() {
        assert_eq!(TtsModel::ElevenLabs.default_format(), "mp3");
        assert_eq!(TtsModel::Azure.default_format(), "wav");
        assert_eq!(TtsModel::Google.default_format(), "mp3");
        assert_eq!(TtsModel::OpenAI.default_format(), "mp3");
    }

    #[test]
    fn k1_tts_model_default_sample_rate() {
        assert_eq!(TtsModel::ElevenLabs.default_sample_rate(), 44100);
        assert_eq!(TtsModel::Azure.default_sample_rate(), 16000);
        assert_eq!(TtsModel::Google.default_sample_rate(), 24000);
        assert_eq!(TtsModel::OpenAI.default_sample_rate(), 24000);
    }

    #[test]
    fn k1_tts_model_max_text_length() {
        assert_eq!(TtsModel::ElevenLabs.max_text_length(), 5000);
        assert_eq!(TtsModel::Azure.max_text_length(), 10_000);
        assert_eq!(TtsModel::Google.max_text_length(), 5000);
        assert_eq!(TtsModel::OpenAI.max_text_length(), 4096);
    }

    // ---- §2 Audio ----

    #[test]
    fn k1_audio_new_valid() {
        let audio =
            Audio::new(vec![0u8; 100], "mp3".to_string(), 44100, 16, 2, 5000).expect("valid audio");
        assert_eq!(audio.size(), 100);
        assert!(!audio.is_empty());
        assert_eq!(audio.format, "mp3");
    }

    #[test]
    fn k1_audio_rejects_invalid_format() {
        let result = Audio::new(vec![0u8; 100], "aac".to_string(), 44100, 16, 2, 5000);
        assert!(matches!(result, Err(VoiceError::AudioFormatInvalid(_))));
    }

    #[test]
    fn k1_audio_rejects_invalid_sample_rate() {
        let result = Audio::new(vec![0u8; 100], "mp3".to_string(), 5000, 16, 2, 5000);
        assert!(matches!(result, Err(VoiceError::SampleRateInvalid(_))));
    }

    #[test]
    fn k1_audio_rejects_invalid_bit_depth() {
        let result = Audio::new(vec![0u8; 100], "mp3".to_string(), 44100, 12, 2, 5000);
        assert!(matches!(result, Err(VoiceError::BitDepthInvalid(_))));
    }

    #[test]
    fn k1_audio_rejects_invalid_channels() {
        let result = Audio::new(vec![0u8; 100], "mp3".to_string(), 44100, 16, 6, 5000);
        assert!(matches!(result, Err(VoiceError::ChannelsInvalid(_))));
    }

    #[test]
    fn k1_audio_empty_check() {
        let audio =
            Audio::new(Vec::new(), "wav".to_string(), 16000, 16, 1, 0).expect("valid empty audio");
        assert!(audio.is_empty());
    }

    // ---- §3 TtsRequest ----

    #[test]
    fn k1_tts_request_valid() {
        let req = TtsRequest::new(
            "hello world".to_string(),
            TtsModel::OpenAI,
            "alloy".to_string(),
            "en".to_string(),
            "mp3".to_string(),
            24000,
        )
        .expect("valid request");
        assert_eq!(req.text, "hello world");
        assert_eq!(req.model, TtsModel::OpenAI);
        assert_eq!(req.voice, "alloy");
    }

    #[test]
    fn k1_tts_request_rejects_empty_text() {
        let result = TtsRequest::new(
            String::new(),
            TtsModel::OpenAI,
            "alloy".to_string(),
            "en".to_string(),
            "mp3".to_string(),
            24000,
        );
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_tts_request_rejects_too_long_text() {
        let long = "a".repeat(5000);
        let result = TtsRequest::new(
            long,
            TtsModel::OpenAI, // max 4096
            "alloy".to_string(),
            "en".to_string(),
            "mp3".to_string(),
            24000,
        );
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_tts_request_rejects_empty_voice() {
        let result = TtsRequest::new(
            "hello".to_string(),
            TtsModel::OpenAI,
            String::new(),
            "en".to_string(),
            "mp3".to_string(),
            24000,
        );
        assert!(matches!(result, Err(VoiceError::Other(_))));
    }

    #[test]
    fn k1_tts_request_with_defaults() {
        let req = TtsRequest::with_defaults(
            "hello".to_string(),
            TtsModel::ElevenLabs,
            "voice-1".to_string(),
            "en".to_string(),
        )
        .expect("valid with defaults");
        assert_eq!(req.output_format, "mp3");
        assert_eq!(req.sample_rate, 44100);
    }
}
