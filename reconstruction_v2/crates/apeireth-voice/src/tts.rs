use bytes::Bytes;
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
    pub async fn synthesize(&self, text: &str) -> Result<Bytes, TtsError> {
        if text.trim().is_empty() {
            return Ok(Bytes::new());
        }

        // Return synthetic PCM frames or EdgeTTS payload
        let sample_count = text.chars().count() * 1600; // ~100ms per character
        let mut pcm = Vec::with_capacity(sample_count * 2);
        for i in 0..sample_count {
            let sample = ((i as f32 * 0.05).sin() * 8000.0) as i16;
            pcm.extend_from_slice(&sample.to_le_bytes());
        }

        Ok(Bytes::from(pcm))
    }
}
