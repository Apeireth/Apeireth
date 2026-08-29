//! MiniMax 高保真 TTS (Text-to-Speech) 客户端适配器.
//!
//! 支持 `speech-2.6-hd` 128kbps 32kHz 音频流生成与基于 PAD 情感空间的情绪声学语气调制.

use std::fmt;
use serde::{Deserialize, Serialize};

/// TTS 错误类型.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum TtsError {
    MissingApiKey,
    EmptyText,
    AudioCodecError(String),
    UpstreamError(u16, String),
}

impl fmt::Display for TtsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingApiKey => write!(f, "API Key 缺失或无效"),
            Self::EmptyText => write!(f, "请求文本为空"),
            Self::AudioCodecError(msg) => write!(f, "音频流编码/解码错误: {}", msg),
            Self::UpstreamError(code, msg) => write!(f, "上游服务返回错误: HTTP {} - {}", code, msg),
        }
    }
}

impl std::error::Error for TtsError {}

/// 情绪音色调制参数.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmotionToneModulation {
    /// 语速 (0.5 ~ 2.0, 默认 1.0)
    pub speed: f32,
    /// 音调 (-12 ~ +12 半音, 默认 0.0)
    pub pitch: f32,
    /// 情感倾向 (Neutral, Happy, Gentle, Excited, Melancholy, Serious)
    pub emotional_style: String,
    /// 情感强度 (0.0 ~ 1.0)
    pub intensity: f32,
}

impl Default for EmotionToneModulation {
    fn default() -> Self {
        Self {
            speed: 1.0,
            pitch: 0.0,
            emotional_style: "Neutral".to_string(),
            intensity: 0.5,
        }
    }
}

/// MiniMax TTS 请求配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiniMaxTtsRequest {
    pub text: String,
    pub voice_id: String,
    pub audio_sample_rate: u32,
    pub bitrate: u32,
    pub format: String,
    pub tone: EmotionToneModulation,
}

/// 生成的音频数据包.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioChunk {
    pub sample_rate: u32,
    pub format: String,
    pub audio_bytes_len: usize,
    pub is_final: bool,
}

/// MiniMax LIVE TTS 客户端.
#[derive(Debug, Clone)]
pub struct MiniMaxLiveTtsClient {
    pub api_key: Option<String>,
    pub group_id: Option<String>,
    pub default_voice_id: String,
}

impl MiniMaxLiveTtsClient {
    pub fn new(api_key: Option<String>, group_id: Option<String>) -> Self {
        Self {
            api_key,
            group_id,
            default_voice_id: "female-tianmei".to_string(),
        }
    }

    /// 根据三维 PAD 情感值自动推导音色语气调制参数.
    pub fn derive_tone_from_pad(valence: f32, arousal: f32, dominance: f32) -> EmotionToneModulation {
        let (style, speed, pitch) = if valence > 0.3 && arousal > 0.2 {
            ("Happy", 1.05 + arousal * 0.1, dominance * 2.0)
        } else if valence > 0.2 && arousal <= 0.2 {
            ("Gentle", 0.95, -1.0)
        } else if valence < -0.2 && arousal > 0.3 {
            ("Serious", 1.08, 1.5)
        } else if valence < -0.2 && arousal <= 0.3 {
            ("Melancholy", 0.88, -2.0)
        } else {
            ("Neutral", 1.0, 0.0)
        };

        EmotionToneModulation {
            speed,
            pitch,
            emotional_style: style.to_string(),
            intensity: (arousal.abs() + valence.abs()) / 2.0,
        }
    }

    /// 构建受控的 TTS 请求负载.
    pub fn build_request(&self, text: &str, tone: EmotionToneModulation) -> Result<MiniMaxTtsRequest, TtsError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TtsError::EmptyText);
        }

        Ok(MiniMaxTtsRequest {
            text: trimmed.to_string(),
            voice_id: self.default_voice_id.clone(),
            audio_sample_rate: 32000,
            bitrate: 128000,
            format: "mp3".to_string(),
            tone,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_tone_from_pad() {
        let happy_tone = MiniMaxLiveTtsClient::derive_tone_from_pad(0.6, 0.5, 0.2);
        assert_eq!(happy_tone.emotional_style, "Happy");
        assert!(happy_tone.speed > 1.0);

        let gentle_tone = MiniMaxLiveTtsClient::derive_tone_from_pad(0.4, 0.1, 0.0);
        assert_eq!(gentle_tone.emotional_style, "Gentle");
        assert!(gentle_tone.speed < 1.0);

        let sad_tone = MiniMaxLiveTtsClient::derive_tone_from_pad(-0.5, 0.1, -0.2);
        assert_eq!(sad_tone.emotional_style, "Melancholy");
    }

    #[test]
    fn test_build_request() {
        let client = MiniMaxLiveTtsClient::new(Some("test_key".to_string()), Some("group_123".to_string()));
        let tone = EmotionToneModulation::default();
        let req = client.build_request("你好，欢迎来到 Apeireth 世界！", tone).unwrap();
        assert_eq!(req.audio_sample_rate, 32000);
        assert_eq!(req.format, "mp3");
        assert_eq!(req.voice_id, "female-tianmei");

        assert_eq!(client.build_request("   ", EmotionToneModulation::default()).unwrap_err(), TtsError::EmptyText);
    }
}
