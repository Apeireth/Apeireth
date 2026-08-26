//! # Voice Config (per @anthropic-ai/voice v0.9.21 商业版 1:1 翻译)
//!
//! VoiceConfig 5 段 (per v0.9.21 商业版 + task spec §1):
//! 1. **wake** — 唤醒词配置 (per `WakeWord`, 默认 `Hardcoded("apeireth")`)
//! 2. **stt** — STT 模型配置 (per `SttModel`, 默认 `Whisper`)
//! 3. **tts** — TTS 模型配置 (per `TtsModel`, 默认 `ElevenLabs`)
//! 4. **vad** — VAD 配置 (per `VadConfig`, 默认 `Energy`)
//! 5. **audio** — 全局音频格式 (format/sample_rate/bit_depth/channels/language)
//!
//! **STUB**: 5 段配置字段保留 1:1 翻译, 但实际不真接 SDK.

use serde::{Deserialize, Serialize};

use crate::voice::error::{VoiceError, VoiceResult};
use crate::voice::stt::SttModel;
use crate::voice::tts::TtsModel;
use crate::voice::vad::{VadAlgorithm, VadConfig};
use crate::voice::wake::{WakeWord, WakeWordCategory, VOICE_DEFAULT_WAKE_WORD};

// ============================================================================
// §1 编译期 hardcode 常量
// ============================================================================

/// VoiceConfig 段数 (per task spec §1, 编译期 hardcode 5).
pub const VOICE_CONFIG_SECTION_COUNT: usize = 5;

/// 默认 audio 格式 (per v0.9.21 商业版估 wav).
pub const DEFAULT_AUDIO_FORMAT: &str = "wav";

/// 默认采样率 (16kHz, per Porcupine 官方 + v0.9.21 商业版估).
pub const DEFAULT_AUDIO_SAMPLE_RATE: u32 = 16_000;

/// 默认位深 (16-bit, per v0.9.21 商业版估).
pub const DEFAULT_AUDIO_BIT_DEPTH: u16 = 16;

/// 默认通道数 (单声道, per v0.9.21 商业版估).
pub const DEFAULT_AUDIO_CHANNELS: u8 = 1;

/// 默认语言 (英语, per v0.9.21 商业版估).
pub const DEFAULT_AUDIO_LANGUAGE: &str = "en";

// ============================================================================
// §2 AudioConfig 全局音频配置 (per v0.9.21 商业版 1:1 翻译)
// ============================================================================

/// 全局音频配置 (per v0.9.21 商业版 `audio_config` 字段 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 `AudioConfig` 对象:
/// - `format` (wav/mp3/opus/flac)
/// - `sample_rate` (8000..=48000)
/// - `bit_depth` (8/16/24/32)
/// - `channels` (1/2)
/// - `language` (ISO 639-1)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioConfig {
    /// 音频格式
    pub format: String,
    /// 采样率 (Hz)
    pub sample_rate: u32,
    /// 位深
    pub bit_depth: u16,
    /// 通道数
    pub channels: u8,
    /// 语言 (ISO 639-1)
    pub language: String,
}

impl AudioConfig {
    /// 默认配置 (wav, 16kHz, 16-bit, mono, en).
    pub fn default_wav() -> Self {
        Self {
            format: DEFAULT_AUDIO_FORMAT.to_string(),
            sample_rate: DEFAULT_AUDIO_SAMPLE_RATE,
            bit_depth: DEFAULT_AUDIO_BIT_DEPTH,
            channels: DEFAULT_AUDIO_CHANNELS,
            language: DEFAULT_AUDIO_LANGUAGE.to_string(),
        }
    }

    /// 自定义配置 (per K-1 强校验 5 守门).
    pub fn custom(
        format: String,
        sample_rate: u32,
        bit_depth: u16,
        channels: u8,
        language: String,
    ) -> VoiceResult<Self> {
        VoiceError::validate_audio_format(&format)?;
        VoiceError::validate_sample_rate(sample_rate)?;
        VoiceError::validate_bit_depth(bit_depth)?;
        VoiceError::validate_channels(channels)?;
        VoiceError::validate_language(&language)?;
        Ok(Self {
            format,
            sample_rate,
            bit_depth,
            channels,
            language,
        })
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self::default_wav()
    }
}

// ============================================================================
// §3 VoiceConfig 主配置 (5 段, 编译期 hardcode)
// ============================================================================

/// VoiceConfig 主配置 (5 段, per task spec §1).
///
/// 字段对应 v0.9.21 商业版 `VoiceConfig` 对象:
/// - `wake` (per `WakeWord`, 默认 `Hardcoded("apeireth")`)
/// - `stt` (per `SttModel`, 默认 `Whisper`)
/// - `tts` (per `TtsModel`, 默认 `ElevenLabs`)
/// - `vad` (per `VadConfig`, 默认 `Energy`)
/// - `audio` (per `AudioConfig`, 默认 wav/16kHz/16-bit/mono/en)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// 唤醒词配置
    pub wake: WakeWord,
    /// STT 模型
    pub stt: SttModel,
    /// TTS 模型
    pub tts: TtsModel,
    /// VAD 配置
    pub vad: VadConfig,
    /// 全局音频配置
    pub audio: AudioConfig,
}

impl VoiceConfig {
    /// 默认配置 (5 段全部默认).
    pub fn default_apeireth() -> Self {
        Self {
            wake: WakeWord::default_apeireth(),
            stt: SttModel::default(),
            tts: TtsModel::default(),
            vad: VadConfig::default(),
            audio: AudioConfig::default(),
        }
    }

    /// 验证配置 (per K-1 强校验, 在 set_* 时已校验, 这里只做交叉校验).
    pub fn validate(&self) -> VoiceResult<()> {
        // wake.keyword 必须非空
        if self.wake.keyword.trim().is_empty() {
            return Err(VoiceError::Other("wake.keyword is empty".to_string()));
        }
        // 交叉: audio.sample_rate 必须匹配 stt/tts 模型范围
        VoiceError::validate_sample_rate(self.audio.sample_rate)?;
        // 交叉: audio.language 必合法
        VoiceError::validate_language(&self.audio.language)?;
        Ok(())
    }

    /// 检查默认唤醒词是 `"apeireth"` (per R20 设计拍板).
    pub fn is_default_apeireth(&self) -> bool {
        self.wake.category == WakeWordCategory::Hardcoded
            && self
                .wake
                .keyword
                .eq_ignore_ascii_case(VOICE_DEFAULT_WAKE_WORD)
    }
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self::default_apeireth()
    }
}

// ============================================================================
// §4 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- §1 编译期 hardcode ----

    #[test]
    fn k1_voice_config_section_count_is_5() {
        assert_eq!(VOICE_CONFIG_SECTION_COUNT, 5, "K-1 强校验: 必须 5 段配置");
        assert_eq!(DEFAULT_AUDIO_FORMAT, "wav");
        assert_eq!(DEFAULT_AUDIO_SAMPLE_RATE, 16_000);
        assert_eq!(DEFAULT_AUDIO_BIT_DEPTH, 16);
        assert_eq!(DEFAULT_AUDIO_CHANNELS, 1);
        assert_eq!(DEFAULT_AUDIO_LANGUAGE, "en");
    }

    // ---- §2 AudioConfig ----

    #[test]
    fn k1_audio_config_default_wav() {
        let audio = AudioConfig::default_wav();
        assert_eq!(audio.format, "wav");
        assert_eq!(audio.sample_rate, 16_000);
        assert_eq!(audio.bit_depth, 16);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.language, "en");
    }

    #[test]
    fn k1_audio_config_custom_valid() {
        let audio = AudioConfig::custom("mp3".to_string(), 44100, 16, 2, "zh-CN".to_string())
            .expect("valid audio config");
        assert_eq!(audio.format, "mp3");
        assert_eq!(audio.sample_rate, 44100);
    }

    #[test]
    fn k1_audio_config_custom_rejects_invalid() {
        // 错 format
        let result = AudioConfig::custom("aac".to_string(), 44100, 16, 2, "en".to_string());
        assert!(matches!(result, Err(VoiceError::AudioFormatInvalid(_))));
        // 错 sample_rate
        let result = AudioConfig::custom("wav".to_string(), 5000, 16, 1, "en".to_string());
        assert!(matches!(result, Err(VoiceError::SampleRateInvalid(_))));
        // 错 bit_depth
        let result = AudioConfig::custom("wav".to_string(), 16000, 12, 1, "en".to_string());
        assert!(matches!(result, Err(VoiceError::BitDepthInvalid(_))));
        // 错 channels
        let result = AudioConfig::custom("wav".to_string(), 16000, 16, 6, "en".to_string());
        assert!(matches!(result, Err(VoiceError::ChannelsInvalid(_))));
        // 错 language
        let result = AudioConfig::custom("wav".to_string(), 16000, 16, 1, "english".to_string());
        assert!(matches!(result, Err(VoiceError::LanguageInvalid(_))));
    }

    // ---- §3 VoiceConfig ----

    #[test]
    fn k1_voice_config_default_apeireth() {
        let config = VoiceConfig::default_apeireth();
        // wake 段: Hardcoded "apeireth"
        assert_eq!(config.wake.category, WakeWordCategory::Hardcoded);
        assert_eq!(config.wake.keyword, "apeireth");
        // stt 段: Whisper
        assert_eq!(config.stt, SttModel::Whisper);
        // tts 段: ElevenLabs
        assert_eq!(config.tts, TtsModel::ElevenLabs);
        // vad 段: Energy
        assert_eq!(config.vad.algorithm, VadAlgorithm::Energy);
        // audio 段: wav/16kHz/16-bit/mono/en
        assert_eq!(config.audio.format, "wav");
        assert_eq!(config.audio.sample_rate, 16_000);
        assert_eq!(config.audio.bit_depth, 16);
        assert_eq!(config.audio.channels, 1);
        assert_eq!(config.audio.language, "en");
    }

    #[test]
    fn k1_voice_config_default_trait() {
        let config = VoiceConfig::default();
        assert_eq!(config, VoiceConfig::default_apeireth());
    }

    #[test]
    fn k1_voice_config_is_default_apeireth() {
        let config = VoiceConfig::default_apeireth();
        assert!(config.is_default_apeireth());

        // 改 wake.category 不再是 Hardcoded
        let mut config2 = config.clone();
        config2.wake.category = WakeWordCategory::Custom;
        assert!(!config2.is_default_apeireth());

        // 改 wake.keyword 不再是 "apeireth"
        let mut config3 = config.clone();
        config3.wake.keyword = "hey buddy".to_string();
        assert!(!config3.is_default_apeireth());
    }

    #[test]
    fn k1_voice_config_validate_ok() {
        let config = VoiceConfig::default_apeireth();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn k1_voice_config_validate_rejects_empty_wake() {
        let mut config = VoiceConfig::default_apeireth();
        config.wake.keyword = String::new();
        assert!(matches!(config.validate(), Err(VoiceError::Other(_))));
    }
}
