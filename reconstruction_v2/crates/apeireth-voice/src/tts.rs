use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TtsError {
    #[error("Network failure: {0}")]
    Network(String),
    #[error("Synthesis failed: {0}")]
    Synthesis(String),
    #[error("Audio encoding error: {0}")]
    Encoding(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TtsEngine {
    EdgeTts,
    Kokoro,
    Vits,
    Piper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsVoice {
    pub name: String,
    pub engine: TtsEngine,
    pub locale: String,
    pub gender: String,
}

impl Default for TtsVoice {
    fn default() -> Self {
        Self {
            name: "zh-CN-XiaoxiaoNeural".into(),
            engine: TtsEngine::EdgeTts,
            locale: "zh-CN".into(),
            gender: "Female".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    pub engine: TtsEngine,
    pub voice: TtsVoice,
    pub rate_percentage: i32,  // -50..+50%
    pub pitch_percentage: i32, // -50..+50%
    pub volume_percentage: i32,
    /// Reserved for output sample rate validation (16k/24k/48k).
    /// 当前 synthesize 走合成 PCM；真接 TTS API 后据此校验 PCM 帧。
    #[allow(dead_code)]
    pub sample_rate: u32,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            engine: TtsEngine::EdgeTts,
            voice: TtsVoice::default(),
            rate_percentage: 0,
            pitch_percentage: 0,
            volume_percentage: 100,
            sample_rate: 24000,
        }
    }
}

pub struct TtsClient {
    pub config: TtsConfig,
    /// Reserved for real TTS API HTTP calls (EdgeTTS / Kokoro / VITS / Piper).
    /// 当前 synthesize 走合成 PCM 占位，http_client 待接入真 API 时启用。
    #[allow(dead_code)]
    http_client: reqwest::Client,
}

impl TtsClient {
    pub fn new(config: TtsConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Generates SSML payload for streaming TTS synthesis
    pub fn build_ssml(&self, text: &str) -> String {
        format!(
            r#"<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="{}"><voice name="{}"><prosody rate="{}%" pitch="{}%">{}</prosody></voice></speak>"#,
            self.config.voice.locale,
            self.config.voice.name,
            self.config.rate_percentage,
            self.config.pitch_percentage,
            text
        )
    }

    /// Synthesizes speech into raw audio bytes
    ///
    /// 当前走合成 PCM 占位路径（正弦波）以测试 pipeline 不依赖真 TTS API；
    /// 接入真 EdgeTTS / Kokoro / VITS / Piper 后用 self.http_client 切。
    pub async fn synthesize(&self, text: &str) -> Result<bytes::Bytes, TtsError> {
        if text.trim().is_empty() {
            return Ok(bytes::Bytes::new());
        }

        // Synthetic PCM at self.config.sample_rate
        let sample_count = text.chars().count() * (self.config.sample_rate / 10) as usize;
        let mut pcm = Vec::with_capacity(sample_count * 2);
        for i in 0..sample_count {
            let sample = ((i as f32 * 0.05).sin() * 8000.0) as i16;
            pcm.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(bytes::Bytes::from(pcm))
    }
}
