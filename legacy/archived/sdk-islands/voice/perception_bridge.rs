//! 感知后端契约对齐桥: 语音协议族 ↔ `perception_backend` trait 面.
//!
//! 对齐契约 (字段级钉死于测试):
//! - [`SttRequest`] → `AudioBuffer` (`bytes` 原样, `duration_ms` 按 PCM 字节率折算)
//! - `Option<String>` 语言 → `LangHint` (`None` = 自动推断)
//! - [`Transcription`] → `Transcription` (`model` 取枚举稳定字符串,
//!   `confidence` 按契约的 `Option<f32>` 形状携带)
//! - [`VoiceError`] → `PerceptionBackendError` (闭合词表逐类映射, 限流保留
//!   `retry_after_ms`)
//! - [`VoiceBackendAdapter`] 实现 `VoiceBackend`: runtime 以
//!   `Arc<dyn VoiceBackend>` 注入本族客户端, 契约面不感知协议细节。

use apeireth_plugin::perception_backend::{
    AudioBuffer, LangHint, PerceptionBackendError, Transcription as BackendTranscription,
    VoiceBackend,
};

use crate::error_taxonomy::ClassifyError;
use crate::voice::capture::duration_ms_for_bytes;
use crate::voice::config::AudioConfig;
use crate::voice::error::VoiceError;
use crate::voice::stt::{SttModel, SttRequest, Transcription};
use crate::voice::{VoiceClient, VoiceClientImpl};

/// 由音频参数估算 PCM 时长 (毫秒).
pub fn estimate_duration_ms(request: &SttRequest) -> u64 {
    duration_ms_for_bytes(
        request.audio.len(),
        request.sample_rate,
        request.bit_depth,
        request.channels,
    )
}

/// 语言提示映射 (`None` = 自动推断).
pub fn lang_hint_from(language: &Option<String>) -> LangHint {
    match language {
        Some(lang) => LangHint::new(lang.clone()),
        None => LangHint::auto(),
    }
}

/// 由模型枚举反查 (未知字符串 → 协议违例).
pub fn parse_model(model: &str) -> Result<SttModel, VoiceError> {
    SttModel::parse(model).ok_or_else(|| {
        VoiceError::Protocol(format!(
            "response carries unknown model identifier `{model}`"
        ))
    })
}

impl From<&SttRequest> for AudioBuffer {
    fn from(request: &SttRequest) -> Self {
        AudioBuffer {
            bytes: request.audio.clone(),
            duration_ms: estimate_duration_ms(request),
        }
    }
}

impl From<&Transcription> for BackendTranscription {
    fn from(t: &Transcription) -> Self {
        BackendTranscription {
            text: t.text.clone(),
            model: t.model.as_str().to_string(),
            language: t.language.clone(),
            // 契约面置信度是 Option 形状: 本族始终产出数值置信度
            confidence: Some(t.confidence),
            duration_ms: t.duration_ms,
        }
    }
}

impl From<VoiceError> for PerceptionBackendError {
    fn from(err: VoiceError) -> Self {
        match err.category() {
            crate::error_taxonomy::ErrorCategory::Network => {
                PerceptionBackendError::Network(err.to_string())
            }
            crate::error_taxonomy::ErrorCategory::RateLimited => {
                PerceptionBackendError::RateLimited {
                    retry_after_ms: err.retry_after_ms().unwrap_or(1_000),
                }
            }
            crate::error_taxonomy::ErrorCategory::Timeout
            | crate::error_taxonomy::ErrorCategory::Backpressure => {
                PerceptionBackendError::Stream(err.to_string())
            }
            crate::error_taxonomy::ErrorCategory::Validation
                if matches!(err, VoiceError::AudioFormatInvalid(_)) =>
            {
                PerceptionBackendError::Audio(err.to_string())
            }
            crate::error_taxonomy::ErrorCategory::Authentication => {
                PerceptionBackendError::BackendUnavailable(err.to_string())
            }
            _ => PerceptionBackendError::Provider(err.to_string()),
        }
    }
}

/// `VoiceBackend` 适配器: 把契约面调用翻译成本族协议调用.
#[derive(Debug, Clone)]
pub struct VoiceBackendAdapter {
    client: VoiceClientImpl,
}

impl VoiceBackendAdapter {
    /// 包装本族客户端.
    pub fn new(client: VoiceClientImpl) -> Self {
        Self { client }
    }

    /// 取回内部客户端.
    pub fn into_inner(self) -> VoiceClientImpl {
        self.client
    }

    /// 借用内部客户端.
    pub fn client(&self) -> &VoiceClientImpl {
        &self.client
    }

    /// 用客户端的采集配置 + 入参音频构造转写请求.
    pub fn build_request(
        &self,
        audio: AudioBuffer,
        lang: LangHint,
    ) -> Result<SttRequest, VoiceError> {
        let config: &AudioConfig = &self.client.config().audio;
        SttRequest::new(
            audio.bytes,
            config.format.clone(),
            config.sample_rate,
            config.bit_depth,
            config.channels,
            self.client.stt_model(),
            lang.0,
        )
    }
}

#[async_trait::async_trait]
impl VoiceBackend for VoiceBackendAdapter {
    async fn transcribe(
        &self,
        audio: AudioBuffer,
        lang: LangHint,
    ) -> Result<BackendTranscription, PerceptionBackendError> {
        let request = self.build_request(audio, lang)?;
        let transcription = self.client.transcribe(&request).await?;
        Ok((&transcription).into())
    }

    fn name(&self) -> &'static str {
        "apeireth-voice"
    }

    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        self.client.health_check().await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::mock::MockVoiceTransport;
    use crate::voice::tts::TtsModel;
    use crate::voice::vad::{VadAlgorithm, VadConfig, VadResult};
    use crate::voice::wake::{WakeWord, WakeWordCategory};
    use crate::voice::VoiceConfig;
    use std::sync::Arc;

    #[test]
    fn perception_contract_transcription_field_alignment_pinned() {
        // 解构钉死契约字段名: perception_backend::Transcription 字段增删会在此处编译失败
        let voice_t = Transcription {
            text: "hello world".to_string(),
            model: SttModel::default(),
            language: "en".to_string(),
            confidence: 0.875,
            duration_ms: 1_500,
            transcribed_at: std::time::SystemTime::now(),
        };
        let backend_t: BackendTranscription = (&voice_t).into();
        let BackendTranscription {
            text,
            model,
            language,
            confidence,
            duration_ms,
        } = backend_t;
        assert_eq!(text, "hello world");
        assert_eq!(model, voice_t.model.as_str());
        assert_eq!(language, "en");
        assert_eq!(confidence, Some(0.875), "契约面置信度为 Option 形状");
        assert_eq!(duration_ms, 1_500);
    }

    #[test]
    fn perception_contract_audio_buffer_field_alignment_pinned() {
        let request = SttRequest::new(
            vec![0u8; 32_000],
            "wav".to_string(),
            16_000,
            16,
            1,
            SttModel::default(),
            Some("en".to_string()),
        )
        .expect("valid request");
        let buffer: AudioBuffer = (&request).into();
        // 解构钉死契约字段名
        let AudioBuffer { bytes, duration_ms } = buffer;
        assert_eq!(bytes.len(), 32_000, "bytes 原样透传");
        assert_eq!(duration_ms, 1_000, "32000 bytes @ 16k/16bit/mono = 1s");
        assert_eq!(estimate_duration_ms(&request), 1_000);
    }

    #[test]
    fn perception_contract_lang_hint_alignment() {
        let auto = lang_hint_from(&None);
        assert_eq!(auto, LangHint::auto());
        let explicit = lang_hint_from(&Some("zh-CN".to_string()));
        assert_eq!(explicit, LangHint::new("zh-CN"));
        // 解构钉死契约形状 (LangHint 是单字段 newtype)
        let LangHint(inner) = explicit;
        assert_eq!(inner, Some("zh-CN".to_string()));
    }

    #[test]
    fn perception_contract_error_mapping_is_closed_and_lossless() {
        // 限流保留 retry_after_ms
        let mapped: PerceptionBackendError = VoiceError::RateLimited {
            retry_after_ms: 777,
        }
        .into();
        match mapped {
            PerceptionBackendError::RateLimited { retry_after_ms } => {
                assert_eq!(retry_after_ms, 777)
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
        // 网络 → Network
        let mapped: PerceptionBackendError = VoiceError::Network("reset".into()).into();
        assert!(matches!(mapped, PerceptionBackendError::Network(_)));
        // 超时 / 背压 → Stream
        let mapped: PerceptionBackendError = VoiceError::Timeout {
            operation: "transcribe",
        }
        .into();
        assert!(matches!(mapped, PerceptionBackendError::Stream(_)));
        let mapped: PerceptionBackendError = VoiceError::Backpressure("full".into()).into();
        assert!(matches!(mapped, PerceptionBackendError::Stream(_)));
        // 音频格式 → Audio
        let mapped: PerceptionBackendError = VoiceError::AudioFormatInvalid("aac".into()).into();
        assert!(matches!(mapped, PerceptionBackendError::Audio(_)));
        // 认证 → BackendUnavailable (换凭证前不可用)
        let mapped: PerceptionBackendError = VoiceError::ApiKeyMissing.into();
        assert!(matches!(
            mapped,
            PerceptionBackendError::BackendUnavailable(_)
        ));
        // 协议 / 其它 → Provider
        let mapped: PerceptionBackendError = VoiceError::Protocol("bad frame".into()).into();
        assert!(matches!(mapped, PerceptionBackendError::Provider(_)));
    }

    #[tokio::test]
    async fn backend_adapter_transcribe_returns_aligned_fields() {
        let mut client = VoiceClientImpl::new();
        client
            .set_api_key("sk-voice-abcdef1234567890xyz".to_string())
            .expect("valid key");
        let transport = Arc::new(MockVoiceTransport::new());
        transport.script_transcribe(Ok(crate::voice::mock::MockTranscribeResponse {
            text: "aligned".to_string(),
            model: SttModel::default().as_str().to_string(),
            language: "en".to_string(),
            confidence: Some(0.5),
            duration_ms: 42,
        }));
        client.set_transport(transport);

        let adapter = VoiceBackendAdapter::new(client);
        let backend: Arc<dyn VoiceBackend> = Arc::new(adapter);
        assert_eq!(backend.name(), "apeireth-voice");
        backend.ping().await.expect("ping");

        let audio = AudioBuffer {
            bytes: vec![0u8; 32_000],
            duration_ms: 1_000,
        };
        let out = backend
            .transcribe(audio, LangHint::new("en"))
            .await
            .expect("transcribe");
        assert_eq!(out.text, "aligned");
        assert_eq!(out.model, SttModel::default().as_str());
        assert_eq!(out.language, "en");
        assert_eq!(out.confidence, Some(0.5));
        assert_eq!(out.duration_ms, 42);
    }

    #[test]
    fn build_request_uses_client_capture_config() {
        let client = VoiceClientImpl::new();
        let adapter = VoiceBackendAdapter::new(client);
        let audio = AudioBuffer {
            bytes: vec![0u8; 100],
            duration_ms: 10,
        };
        let request = adapter
            .build_request(audio, LangHint::auto())
            .expect("request");
        assert_eq!(request.format, "wav");
        assert_eq!(request.sample_rate, 16_000);
        assert_eq!(request.bit_depth, 16);
        assert_eq!(request.channels, 1);
        assert_eq!(request.model, SttModel::default());
        assert!(request.language.is_none());
    }

    #[test]
    fn parse_model_rejects_unknown_identifier() {
        assert!(parse_model(SttModel::default().as_str()).is_ok());
        assert!(matches!(
            parse_model("bogus-model"),
            Err(VoiceError::Protocol(_))
        ));
    }

    // 保留类型引用, 防导入面漂移 (契约面枚举全量可见)
    #[test]
    fn contract_type_surface_stays_importable() {
        let _ = TtsModel::default();
        let _ = VadConfig::default();
        let _ = VadAlgorithm::default();
        let _ = VadResult::new(
            false,
            VadAlgorithm::default(),
            0.0,
            std::time::Duration::from_millis(0),
            std::time::Duration::from_millis(0),
        );
        let _ = WakeWord::default_apeireth();
        let _ = WakeWordCategory::default();
        let _ = VoiceConfig::default();
    }
}
