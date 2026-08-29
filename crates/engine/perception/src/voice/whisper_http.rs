//! WhisperHttpBackend — 真实 HTTP 调用 Whisper-compatible `/v1/audio/transcriptions`.
//!
//! RC-7 Perception 真 modality 实施 (per `rc-7-perception-true-modality-spec.md`).
//!
//! **设计** (per scene-d §5 决策 1, 复用 `ProviderCapability` HTTP 模式):
//! - 使用 `reqwest::Client` 发送 `multipart/form-data` 请求
//! - 凭证走 `CredentialResolver::resolve(credential_key)`, 不读 env 不存 string
//! - 支持 OpenAI (`api.openai.com`) / MiniMax (`api.minimaxi.com`) 两家端点
//! - 超时、重试、错误映射为 `PerceptionBackendError`
//! - 无 API Key 时 → `Err(BackendUnavailable)` **（0 装 PASS: 不假装能转写）**
//!
//! **O-6 三阶审查**:
//! 1. 总体: 与 RC-7 spec 对齐, engine 层真 HTTP 实现
//! 2. 系统: 依赖 `reqwest` (workspace dep) + `apeireth-plugin` (credential + backend trait)
//! 3. 架构: runtime 通过 `Arc<dyn VoiceBackend>` 注入, 此 struct 实现 `VoiceBackend`
//!
//! **0 装诚实**:
//! - 无 credential → `BackendUnavailable("no credential configured")`
//! - 空 audio → `Audio("empty audio buffer")`
//! - HTTP 错 → `Network(...)` / `Provider(...)` / `RateLimited { retry_after_ms }`
//! - 真实转写结果解析失败 → `Provider("failed to parse response: ...")`

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::multipart;
use serde::Deserialize;

use apeireth_plugin::credentials::CredentialResolver;
use apeireth_plugin::perception_backend::{
    AudioBuffer, LangHint, PerceptionBackendError, Transcription, VoiceBackend,
};

/// 默认请求超时 (30 秒, per Whisper API 常规 audio 文件处理时间).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// 最小 API Key 长度 (per K-1 强校验 `voice::MIN_API_KEY_LENGTH >= 16`).
const MIN_API_KEY_LENGTH: usize = 16;

/// 真实 HTTP Whisper backend.
///
/// 调用 OpenAI / MiniMax Whisper-compatible `/audio/transcriptions` 端点,
/// 通过 `CredentialResolver` 获取 API Key.
///
/// **构造**: 使用 `WhisperHttpBackend::new()` 并传入 `Arc<dyn CredentialResolver>`.
/// 配置可通过 `WhisperHttpConfig` 自定义 (base_url / model / credential_key / timeout).
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use apeireth_plugin::credentials::StaticCredentials;
/// use apeireth_perception::voice::WhisperHttpBackend;
///
/// let creds = Arc::new(
///     StaticCredentials::new()
///         .with("provider.whisper.api_key", "sk-your-key-here-1234567890")
/// );
/// let backend = WhisperHttpBackend::openai(creds);
/// ```
pub struct WhisperHttpBackend {
    /// HTTP 客户端 (复用连接池)
    client: reqwest::Client,
    /// 配置
    config: WhisperHttpConfig,
    /// 凭证解析器 (per RC-9 keyring 真接)
    credentials: Arc<dyn CredentialResolver>,
}

/// WhisperHttpBackend 配置.
#[derive(Debug, Clone)]
pub struct WhisperHttpConfig {
    /// API base URL (含 `/v1`, e.g. `"https://api.openai.com/v1"`)
    pub base_url: String,
    /// 模型 ID (e.g. `"whisper-1"`, `"speech-01"`)
    pub model: String,
    /// `CredentialResolver` 用的逻辑名 (e.g. `"provider.whisper.api_key"`)
    pub credential_key: String,
    /// 默认语言 (ISO 639-1, e.g. `"en"` / `"zh"`)
    pub default_language: String,
    /// 请求超时
    pub timeout: Duration,
}

impl WhisperHttpConfig {
    /// OpenAI Whisper 默认配置.
    pub fn openai() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "whisper-1".to_string(),
            credential_key: "provider.whisper.api_key".to_string(),
            default_language: "en".to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// MiniMax Whisper-compatible 默认配置.
    pub fn minimax() -> Self {
        Self {
            base_url: "https://api.minimaxi.com/v1".to_string(),
            model: "speech-01".to_string(),
            credential_key: "provider.minimax.api_key".to_string(),
            default_language: "zh".to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

/// Whisper API 响应体 (OpenAI-compatible JSON).
#[derive(Debug, Deserialize)]
struct WhisperResponse {
    /// 转写文本
    text: String,
    /// 返回的语言 (部分 provider 可能不返, 容忍 None)
    #[serde(default)]
    language: Option<String>,
    /// 音频时长 (秒, 部分 provider 可能不返)
    #[serde(default)]
    duration: Option<f64>,
}

impl WhisperHttpBackend {
    /// 使用自定义配置构造.
    pub fn new(config: WhisperHttpConfig, credentials: Arc<dyn CredentialResolver>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .expect("reqwest client build should not fail with default TLS");
        Self {
            client,
            config,
            credentials,
        }
    }

    /// OpenAI Whisper 快捷构造.
    pub fn openai(credentials: Arc<dyn CredentialResolver>) -> Self {
        Self::new(WhisperHttpConfig::openai(), credentials)
    }

    /// MiniMax Whisper-compatible 快捷构造.
    pub fn minimax(credentials: Arc<dyn CredentialResolver>) -> Self {
        Self::new(WhisperHttpConfig::minimax(), credentials)
    }

    /// 解析 API Key, 校验长度.
    ///
    /// **0 装 PASS**: 无 key → `BackendUnavailable`, 不假装能转写.
    fn resolve_api_key(&self) -> Result<String, PerceptionBackendError> {
        let secret = self
            .credentials
            .resolve(&self.config.credential_key)
            .ok_or_else(|| {
                PerceptionBackendError::BackendUnavailable(format!(
                    "no credential configured for '{}'; set API key via CredentialResolver \
                     (env / keyring / StaticCredentials)",
                    self.config.credential_key
                ))
            })?;
        let key = secret.expose().to_string();
        if key.len() < MIN_API_KEY_LENGTH {
            return Err(PerceptionBackendError::BackendUnavailable(format!(
                "API key for '{}' is too short ({} chars, minimum {}); \
                 check credential configuration",
                self.config.credential_key,
                key.len(),
                MIN_API_KEY_LENGTH
            )));
        }
        Ok(key)
    }

    /// 构建 multipart/form-data 请求体.
    fn build_multipart(
        &self,
        audio: AudioBuffer,
        lang: &LangHint,
    ) -> Result<multipart::Form, PerceptionBackendError> {
        if audio.bytes.is_empty() {
            return Err(PerceptionBackendError::Audio(
                "empty audio buffer; cannot transcribe silence".to_string(),
            ));
        }
        // Whisper API 要求 `file` 字段 (binary audio) + `model` 字段
        // 可选: `language` (ISO 639-1)
        let file_part = multipart::Part::bytes(audio.bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| PerceptionBackendError::Audio(format!("failed to set MIME type: {e}")))?;

        let mut form = multipart::Form::new()
            .part("file", file_part)
            .text("model", self.config.model.clone());

        // 语言提示 (如果有)
        let language = lang.0.as_deref().unwrap_or(&self.config.default_language);
        form = form.text("language", language.to_string());

        Ok(form)
    }
}

#[async_trait]
impl VoiceBackend for WhisperHttpBackend {
    async fn transcribe(
        &self,
        audio: AudioBuffer,
        lang: LangHint,
    ) -> Result<Transcription, PerceptionBackendError> {
        // 1. 解析 API Key (0 装: 无 key → BackendUnavailable)
        let api_key = self.resolve_api_key()?;

        // 2. 记住 duration 用于返回
        let input_duration_ms = audio.duration_ms;

        // 3. 构建 multipart 请求体
        let form = self.build_multipart(audio, &lang)?;

        // 4. 发送 POST 请求
        let url = format!("{}/audio/transcriptions", self.config.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    PerceptionBackendError::Stream(format!(
                        "Whisper request timed out after {:?}: {e}",
                        self.config.timeout
                    ))
                } else if e.is_connect() {
                    PerceptionBackendError::Network(format!(
                        "failed to connect to Whisper endpoint {url}: {e}"
                    ))
                } else {
                    PerceptionBackendError::Network(format!("Whisper HTTP request failed: {e}"))
                }
            })?;

        // 5. 检查 HTTP 状态码
        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // 尝试从 Retry-After 头获取重试时间
            let retry_after_ms = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(|secs| secs * 1000)
                .unwrap_or(5000);
            return Err(PerceptionBackendError::RateLimited { retry_after_ms });
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PerceptionBackendError::Provider(format!(
                "Whisper API returned HTTP {status}: {body}"
            )));
        }

        // 6. 解析 JSON 响应
        let body = response.text().await.map_err(|e| {
            PerceptionBackendError::Network(format!("failed to read Whisper response body: {e}"))
        })?;
        let whisper: WhisperResponse = serde_json::from_str(&body).map_err(|e| {
            PerceptionBackendError::Provider(format!(
                "failed to parse Whisper response: {e}; body: {body}"
            ))
        })?;

        // 7. 构建 Transcription
        let language = lang.0.as_deref().unwrap_or(&self.config.default_language);
        Ok(Transcription {
            text: whisper.text,
            model: self.config.model.clone(),
            language: whisper.language.unwrap_or_else(|| language.to_string()),
            confidence: None, // Whisper API 不返回 confidence (0 装: 不假装有)
            duration_ms: whisper
                .duration
                .map(|d| (d * 1000.0) as u64)
                .unwrap_or(input_duration_ms),
        })
    }

    fn name(&self) -> &'static str {
        "whisper_http"
    }

    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        // 验证 credential 可解析即可, 不发真请求 (0 装: ping 不消耗 API 配额)
        let _key = self.resolve_api_key()?;
        Ok(())
    }
}

// ============================================
// 测试
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::credentials::{NoCredentials, StaticCredentials};

    /// 无 credential → BackendUnavailable (0 装: 不假装能转写)
    #[tokio::test]
    async fn no_credential_returns_backend_unavailable() {
        let backend = WhisperHttpBackend::openai(Arc::new(NoCredentials));
        let result = backend
            .transcribe(AudioBuffer::empty(), LangHint::auto())
            .await;
        let err = result.expect_err("must fail without credential");
        match err {
            PerceptionBackendError::BackendUnavailable(msg) => {
                assert!(
                    msg.contains("no credential configured"),
                    "error must explain: {msg}"
                );
                assert!(
                    msg.contains("provider.whisper.api_key"),
                    "error must name the key: {msg}"
                );
            }
            other => panic!("expected BackendUnavailable, got {other:?}"),
        }
    }

    /// API Key 太短 → BackendUnavailable (K-1 强校验)
    #[tokio::test]
    async fn short_api_key_returns_backend_unavailable() {
        let creds = Arc::new(StaticCredentials::new().with("provider.whisper.api_key", "short"));
        let backend = WhisperHttpBackend::openai(creds);
        let result = backend
            .transcribe(
                AudioBuffer {
                    bytes: vec![0u8; 100],
                    duration_ms: 1000,
                },
                LangHint::auto(),
            )
            .await;
        let err = result.expect_err("must fail with short key");
        match err {
            PerceptionBackendError::BackendUnavailable(msg) => {
                assert!(msg.contains("too short"), "error must explain: {msg}");
            }
            other => panic!("expected BackendUnavailable, got {other:?}"),
        }
    }

    /// 空 audio → Audio error (不浪费 API 配额)
    #[tokio::test]
    async fn empty_audio_returns_audio_error() {
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::openai(creds);
        let result = backend
            .transcribe(AudioBuffer::empty(), LangHint::auto())
            .await;
        let err = result.expect_err("must fail with empty audio");
        match err {
            PerceptionBackendError::Audio(msg) => {
                assert!(
                    msg.contains("empty audio buffer"),
                    "error must explain: {msg}"
                );
            }
            other => panic!("expected Audio error, got {other:?}"),
        }
    }

    /// ping 无 credential → BackendUnavailable
    #[tokio::test]
    async fn ping_without_credential_fails() {
        let backend = WhisperHttpBackend::openai(Arc::new(NoCredentials));
        let result = backend.ping().await;
        assert!(result.is_err(), "ping must fail without credential");
    }

    /// ping 有 credential → Ok (不发真请求)
    #[tokio::test]
    async fn ping_with_credential_succeeds() {
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::openai(creds);
        let result = backend.ping().await;
        assert!(result.is_ok(), "ping must succeed with valid credential");
    }

    /// MiniMax 配置验证
    #[test]
    fn minimax_config_uses_correct_defaults() {
        let config = WhisperHttpConfig::minimax();
        assert!(config.base_url.contains("minimaxi.com"));
        assert_eq!(config.model, "speech-01");
        assert_eq!(config.credential_key, "provider.minimax.api_key");
        assert_eq!(config.default_language, "zh");
    }

    /// backend name 验证
    #[test]
    fn backend_name_is_whisper_http() {
        let backend = WhisperHttpBackend::openai(Arc::new(NoCredentials));
        assert_eq!(backend.name(), "whisper_http");
    }

    /// dyn VoiceBackend: Send + Sync 编译期断言
    #[test]
    fn whisper_http_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WhisperHttpBackend>();

        // Arc<dyn VoiceBackend> 注入路径
        let backend: Arc<dyn VoiceBackend> =
            Arc::new(WhisperHttpBackend::openai(Arc::new(NoCredentials)));
        let _clone = backend.clone();
    }

    /// Mock HTTP server 验证 multipart 格式与 API Key 注入.
    ///
    /// 这个测试启动一个本地 TCP listener 模拟 Whisper API,
    /// 验证请求头含 `Authorization: Bearer ...` 和 multipart body.
    #[tokio::test]
    async fn mock_server_validates_multipart_and_auth() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // 启动 mock server
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock server");
        let addr = listener.local_addr().unwrap();

        // 后台接受一个连接并返回假响应
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).await.expect("read");
            let request = String::from_utf8_lossy(&buf[..n]);

            // 验证 Authorization header (HTTP header 大小写不敏感)
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer sk-test-1234567890abcdef"),
                "request must contain auth header: {request}"
            );
            // 验证 multipart content type
            assert!(
                request.contains("multipart/form-data"),
                "request must be multipart: {request}"
            );

            // 返回 Whisper JSON 响应
            let body = r#"{"text":"hello world","language":"en","duration":1.5}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        // 配置 backend 指向 mock server
        let config = WhisperHttpConfig {
            base_url: format!("http://127.0.0.1:{}", addr.port()),
            model: "whisper-1".to_string(),
            credential_key: "provider.whisper.api_key".to_string(),
            default_language: "en".to_string(),
            timeout: Duration::from_secs(5),
        };
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::new(config, creds);

        // 发送转写请求
        let audio = AudioBuffer {
            bytes: vec![0u8; 100], // 模拟 audio data
            duration_ms: 1500,
        };
        let result = backend.transcribe(audio, LangHint::new("en")).await;

        // 等 server 完成
        server.await.expect("server task");

        // 验证转写结果
        let transcription = result.expect("transcription should succeed");
        assert_eq!(transcription.text, "hello world");
        assert_eq!(transcription.model, "whisper-1");
        assert_eq!(transcription.language, "en");
        assert_eq!(transcription.duration_ms, 1500); // 1.5s * 1000
        assert!(
            transcription.confidence.is_none(),
            "Whisper API 不返 confidence (0 装)"
        );
    }

    /// 连接失败 → Network error
    #[tokio::test]
    async fn connection_failure_returns_network_error() {
        let config = WhisperHttpConfig {
            base_url: "http://127.0.0.1:1".to_string(), // 不存在的端口
            model: "whisper-1".to_string(),
            credential_key: "provider.whisper.api_key".to_string(),
            default_language: "en".to_string(),
            timeout: Duration::from_secs(2),
        };
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::new(config, creds);
        let result = backend
            .transcribe(
                AudioBuffer {
                    bytes: vec![0u8; 100],
                    duration_ms: 1000,
                },
                LangHint::auto(),
            )
            .await;
        let err = result.expect_err("must fail on connection refused");
        match err {
            PerceptionBackendError::Network(msg) | PerceptionBackendError::Stream(msg) => {
                assert!(!msg.is_empty(), "error must have message");
            }
            other => panic!("expected Network or Stream error, got {other:?}"),
        }
    }
}
