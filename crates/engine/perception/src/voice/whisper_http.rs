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

/// 默认上传音频上限 (25 MiB, 对齐 OpenAI Whisper 单文件 25MB 上限的保守值).
const DEFAULT_MAX_UPLOAD_BYTES: usize = 25 * 1024 * 1024;

/// 默认响应体上限 (1 MiB — 转写 JSON 很小, 该上限对正常响应远超所需,
/// 同时杜绝 provider 返回超大 body 的无界内存分配).
const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;

/// 错误 body 预览的严格字节上限 (P1 硬化: 任意 provider 响应不得整段
/// 进入错误链路).
const MAX_ERROR_PREVIEW_BYTES: usize = 256;

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
    /// 上传音频最大字节数 (P1 硬化: 超限直接拒绝, 0 HTTP 请求).
    /// 默认 25 MiB, 对齐 OpenAI Whisper 单文件上限.
    pub max_upload_bytes: usize,
    /// 响应体最大字节数 (成功 JSON 与错误 body 同限; Content-Length 超限
    /// 提前失败, chunked 流式累计超限即失败). 默认 1 MiB.
    pub max_response_bytes: usize,
    /// 传输策略 (P1 硬化): 是否允许**非环回**地址的明文 HTTP. 默认 `false` —
    /// 强制 HTTPS; 环回地址 (localhost / 127.0.0.0/8 / [::1]) 的明文 HTTP
    /// 始终放行, 供测试与本地 provider 使用.
    pub allow_insecure_http: bool,
    /// 上传 MIME 类型 (默认 `"audio/wav"`). `AudioBuffer` 只携带原始容器
    /// 字节, 无格式元数据 — 上传非 WAV 容器 (mp3/ogg 等) 时必须显式配置.
    pub upload_mime_type: String,
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
            max_upload_bytes: DEFAULT_MAX_UPLOAD_BYTES,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            allow_insecure_http: false,
            upload_mime_type: "audio/wav".to_string(),
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
            max_upload_bytes: DEFAULT_MAX_UPLOAD_BYTES,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            allow_insecure_http: false,
            upload_mime_type: "audio/wav".to_string(),
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
        // P1 硬化: MIME 来自显式配置 (默认 "audio/wav"); AudioBuffer 不携带
        // 格式元数据, 上传非 WAV 容器必须配置 `upload_mime_type`.
        // 文件名扩展从 MIME subtype 推导 (常见别名归一), 不做媒体探测.
        let subtype = self
            .config
            .upload_mime_type
            .rsplit('/')
            .next()
            .unwrap_or("bin");
        let ext = match subtype {
            "mpeg" => "mp3",
            "x-wav" | "wave" => "wav",
            other => other,
        };
        let file_part = multipart::Part::bytes(audio.bytes)
            .file_name(format!("audio.{ext}"))
            .mime_str(&self.config.upload_mime_type)
            .map_err(|e| PerceptionBackendError::Audio(format!("failed to set MIME type: {e}")))?;

        let mut form = multipart::Form::new()
            .part("file", file_part)
            .text("model", self.config.model.clone());

        // 语言提示 (如果有)
        let language = lang.0.as_deref().unwrap_or(&self.config.default_language);
        form = form.text("language", language.to_string());

        Ok(form)
    }

    /// 传输策略校验 (P1 硬化): 默认强制 HTTPS; 环回明文 HTTP 仅用于测试与
    /// 本地 provider, 始终放行; 非环回明文 HTTP 必须显式
    /// `allow_insecure_http = true`. 错误文本不回显 base_url (URL 中可能
    /// 被配置进凭据类信息).
    fn validate_transport_policy(
        base_url: &str,
        allow_insecure_http: bool,
    ) -> Result<(), PerceptionBackendError> {
        let lower = base_url.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("https://") {
            if rest.is_empty() {
                return Err(PerceptionBackendError::BackendUnavailable(
                    "whisper base_url must include a host after 'https://'".to_string(),
                ));
            }
            return Ok(());
        }
        let Some(rest) = lower.strip_prefix("http://") else {
            return Err(PerceptionBackendError::BackendUnavailable(
                "whisper base_url must start with 'https://' (or 'http://' only for \
                 loopback / explicitly allowed insecure transport)"
                    .to_string(),
            ));
        };
        // 明文 HTTP: 提取 host (剥端口/userinfo/IPv6 括号), 仅环回或显式放行时允许.
        let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
        let no_userinfo = authority.rsplit('@').next().unwrap_or(authority);
        let host = if no_userinfo.starts_with('[') {
            // IPv6 字面量: "[::1]" 或 "[::1]:8080" → "::1"
            no_userinfo
                .trim_start_matches('[')
                .split(']')
                .next()
                .unwrap_or(no_userinfo)
        } else {
            // 普通主机: "host:port" → "host" (IPv4/域名不带括号)
            no_userinfo.split(':').next().unwrap_or(no_userinfo)
        };
        let is_loopback = host == "localhost"
            || host == "::1"
            || host
                .parse::<std::net::Ipv4Addr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false)
            || host
                .parse::<std::net::Ipv6Addr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false);
        if is_loopback || allow_insecure_http {
            return Ok(());
        }
        Err(PerceptionBackendError::BackendUnavailable(
            "insecure plaintext http is rejected for non-loopback hosts; use https:// \
             or set allow_insecure_http=true explicitly"
                .to_string(),
        ))
    }

    /// 有界读取响应体 (P1 硬化): 绝不对 provider 控制的 body 做无界分配.
    /// `Content-Length` 超限 → 提前失败 (零读取); chunked/未知长度 →
    /// 累计超限即失败并丢弃已读数据.
    async fn read_body_bounded(
        mut response: reqwest::Response,
        max_bytes: usize,
    ) -> Result<Vec<u8>, PerceptionBackendError> {
        if let Some(len) = response.content_length() {
            if len > max_bytes as u64 {
                return Err(PerceptionBackendError::Provider(format!(
                    "whisper response body too large: Content-Length {len} exceeds maximum {max_bytes}"
                )));
            }
        }
        let mut buf: Vec<u8> = Vec::with_capacity(4096);
        while let Some(chunk) = response.chunk().await.map_err(|e| {
            PerceptionBackendError::Network(format!("failed to read whisper response body: {e}"))
        })? {
            if buf.len() + chunk.len() > max_bytes {
                return Err(PerceptionBackendError::Provider(format!(
                    "whisper response body too large: exceeds maximum {max_bytes} bytes (stream truncated)"
                )));
            }
            buf.extend_from_slice(&chunk);
        }
        Ok(buf)
    }

    /// 错误 body 预览 (P1 硬化): 控制字符/换行折叠为空格, 严格字节上限;
    /// 防止任意长度 provider 响应或 HTML dump 进入错误文本.
    fn sanitize_preview(raw: &str) -> String {
        let mut out = String::new();
        for c in raw.chars().map(|c| if c.is_control() { ' ' } else { c }) {
            if out.len() + c.len_utf8() > MAX_ERROR_PREVIEW_BYTES {
                break;
            }
            out.push(c);
        }
        out
    }
}

#[async_trait]
impl VoiceBackend for WhisperHttpBackend {
    async fn transcribe(
        &self,
        audio: AudioBuffer,
        lang: LangHint,
    ) -> Result<Transcription, PerceptionBackendError> {
        // 1. 传输策略校验 (P1 硬化: 明文 HTTP 仅环回/显式放行) — 在任何
        //    凭证解析与网络行为之前.
        Self::validate_transport_policy(&self.config.base_url, self.config.allow_insecure_http)?;

        // 2. 上传体积上限 (P1 硬化: 超限直接拒绝, 零 HTTP 请求, 不静默截断).
        if audio.bytes.len() > self.config.max_upload_bytes {
            return Err(PerceptionBackendError::Audio(format!(
                "audio upload too large: {} bytes exceeds max_upload_bytes ({}); \
                 no HTTP request was sent",
                audio.bytes.len(),
                self.config.max_upload_bytes
            )));
        }

        // 3. 解析 API Key (0 装: 无 key → BackendUnavailable)
        let api_key = self.resolve_api_key()?;

        // 4. 记住 duration 用于返回
        let input_duration_ms = audio.duration_ms;

        // 5. 构建 multipart 请求体
        let form = self.build_multipart(audio, &lang)?;

        // 6. 发送 POST 请求
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

        // 7. 检查 HTTP 状态码
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
            // P1 硬化: 错误 body 有界读取 + 凭据回显清除 + 严格预览上限,
            // 完整 provider 响应绝不进入错误文本.
            let body_bytes = Self::read_body_bounded(response, self.config.max_response_bytes)
                .await
                .map_err(|e| {
                    PerceptionBackendError::Provider(format!(
                        "whisper API returned HTTP {status}; error body unreadable: {e}"
                    ))
                })?;
            let raw = String::from_utf8_lossy(&body_bytes).replace(&api_key, "[redacted]");
            let preview = Self::sanitize_preview(&raw);
            return Err(PerceptionBackendError::Provider(format!(
                "whisper API returned HTTP {status}; body preview: {preview}"
            )));
        }

        // 8. 有界读取并解析 JSON 响应 (P1 硬化: 无界 response.text() 已移除;
        //    解析失败时同样只携带有界预览)
        let body_bytes = Self::read_body_bounded(response, self.config.max_response_bytes).await?;
        let whisper: WhisperResponse = serde_json::from_slice(&body_bytes).map_err(|e| {
            let raw = String::from_utf8_lossy(&body_bytes).replace(&api_key, "[redacted]");
            let preview = Self::sanitize_preview(&raw);
            PerceptionBackendError::Provider(format!(
                "failed to parse Whisper response: {e}; body preview: {preview}"
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
            max_upload_bytes: DEFAULT_MAX_UPLOAD_BYTES,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            allow_insecure_http: false,
            upload_mime_type: "audio/wav".to_string(),
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
        // mock server must finish within 10s; if the client never connected
        // (e.g. policy rejection) fail on timeout instead of hanging forever.
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("mock server timed out - client never connected")
            .expect("server task");

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
            base_url: "http://127.0.0.1:1".to_string(), // 不存在的端口 (环回, 放行)
            model: "whisper-1".to_string(),
            credential_key: "provider.whisper.api_key".to_string(),
            default_language: "en".to_string(),
            timeout: Duration::from_secs(2),
            max_upload_bytes: DEFAULT_MAX_UPLOAD_BYTES,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            allow_insecure_http: false,
            upload_mime_type: "audio/wav".to_string(),
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

    // ============================================
    // P1 硬化: 上传/响应体积上限
    // ============================================

    fn test_config(
        base_url: String,
        max_upload_bytes: usize,
        max_response_bytes: usize,
    ) -> WhisperHttpConfig {
        WhisperHttpConfig {
            base_url,
            model: "whisper-1".to_string(),
            credential_key: "provider.whisper.api_key".to_string(),
            default_language: "en".to_string(),
            timeout: Duration::from_secs(5),
            max_upload_bytes,
            max_response_bytes,
            allow_insecure_http: false,
            upload_mime_type: "audio/wav".to_string(),
        }
    }

    /// mock server 响应写完后的有界吸收: 客户端 multipart body 可能尚未发完,
    /// 带未读数据关闭连接会触发 RST 使客户端 send 失败; 持续吸收直到客户端
    /// 关闭或超时, 保证客户端能收到响应.
    async fn drain_upload(stream: &mut tokio::net::TcpStream) {
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 65536];
        let _ = tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                match stream.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        })
        .await;
    }

    /// P1 硬化 (N): 超限上传在发出任何 HTTP 请求之前被拒绝 —
    /// listener 先绑定再关闭, 若代码真的发起连接会得到 Network 错误;
    /// 只有请求前检查才能产生 Audio 错误.
    #[tokio::test]
    async fn oversize_upload_rejected_before_any_http() {
        // 绑定后立即丢弃: 任何真实 HTTP 尝试都会 connection refused → Network
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind probe listener");
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let config = test_config(format!("http://127.0.0.1:{}", addr.port()), 10, 64);
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::new(config, creds);
        let audio = AudioBuffer {
            bytes: vec![0u8; 11], // > max_upload_bytes = 10
            duration_ms: 1000,
        };
        let err = backend
            .transcribe(audio, LangHint::auto())
            .await
            .expect_err("oversize upload must be rejected");
        match err {
            PerceptionBackendError::Audio(msg) => {
                assert!(msg.contains("too large"), "error must explain: {msg}");
                assert!(
                    msg.contains("no HTTP request was sent"),
                    "error must state zero-HTTP guarantee: {msg}"
                );
            }
            other => panic!("expected Audio (pre-HTTP rejection), got {other:?}"),
        }
    }

    /// P1 硬化 (O): 响应 Content-Length 超上限 → 提前失败 (Provider 错误).
    #[tokio::test]
    async fn oversize_success_body_with_content_length_is_rejected() {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock server");
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            // 声明超大 Content-Length, 实际只发一小段
            let head = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100000\r\n\r\n";
            stream.write_all(head.as_bytes()).await.expect("write");
            stream.write_all(b"{\"text\":\"hi\"}").await.expect("write");
            drain_upload(&mut stream).await;
        });

        let config = test_config(format!("http://127.0.0.1:{}", addr.port()), DEFAULT_MAX_UPLOAD_BYTES, 64);
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::new(config, creds);
        let audio = AudioBuffer {
            bytes: vec![0u8; 100],
            duration_ms: 1000,
        };
        let err = backend
            .transcribe(audio, LangHint::auto())
            .await
            .expect_err("oversized declared body must be rejected");
        match err {
            PerceptionBackendError::Provider(msg) => {
                assert!(msg.contains("too large"), "error must explain: {msg}");
                assert!(msg.contains("Content-Length"), "must name the mechanism: {msg}");
            }
            other => panic!("expected Provider (size bound), got {other:?}"),
        }
        // mock server must finish within 10s; if the client never connected
        // (e.g. policy rejection) fail on timeout instead of hanging forever.
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("mock server timed out - client never connected")
            .expect("server task");
    }

    /// P1 硬化 (O): chunked/未知长度响应累计超限 → 流式截断拒绝.
    #[tokio::test]
    async fn oversize_chunked_success_body_is_rejected() {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock server");
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 8192];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
            // chunked: 两个 50 字节 chunk, 总量 100 > max_response_bytes = 64
            let chunk = [b'x'; 50];
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                .await
                .expect("write head");
            stream.write_all(b"32\r\n").await.expect("write size");
            stream.write_all(&chunk).await.expect("write chunk");
            stream.write_all(b"\r\n").await.expect("write crlf");
            stream.write_all(b"32\r\n").await.expect("write size");
            stream.write_all(&chunk).await.expect("write chunk");
            stream.write_all(b"\r\n0\r\n\r\n").await.expect("write end");
            drain_upload(&mut stream).await;
        });

        let config = test_config(format!("http://127.0.0.1:{}", addr.port()), DEFAULT_MAX_UPLOAD_BYTES, 64);
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::new(config, creds);
        let audio = AudioBuffer {
            bytes: vec![0u8; 100],
            duration_ms: 1000,
        };
        let err = backend
            .transcribe(audio, LangHint::auto())
            .await
            .expect_err("oversized chunked body must be rejected");
        match err {
            PerceptionBackendError::Provider(msg) => {
                assert!(msg.contains("too large"), "error must explain: {msg}");
                assert!(msg.contains("truncated"), "must name stream truncation: {msg}");
            }
            other => panic!("expected Provider (size bound), got {other:?}"),
        }
        // mock server must finish within 10s; if the client never connected
        // (e.g. policy rejection) fail on timeout instead of hanging forever.
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("mock server timed out - client never connected")
            .expect("server task");
    }

    /// P1 硬化 (P+Q): 错误 body 只保留有界净化预览 — 完整远端 body
    /// (含超长尾部标记) 不得进入错误文本.
    #[tokio::test]
    async fn error_body_is_redacted_to_bounded_preview() {
        use tokio::io::AsyncWriteExt;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock server");
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = vec![0u8; 8192];
            let _ = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await;
            // 5KB 错误 body: 短前缀 + 大段填充 + 超出预览窗口的尾部标记
            let mut body = String::from("{\"error\":\"quota\"}");
            body.push_str(&"A".repeat(2000));
            body.push_str("END_OF_BODY_MARKER");
            let response = format!(
                "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
            drain_upload(&mut stream).await;
        });

        let config = test_config(format!("http://127.0.0.1:{}", addr.port()), DEFAULT_MAX_UPLOAD_BYTES, 65536);
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::new(config, creds);
        let audio = AudioBuffer {
            bytes: vec![0u8; 100],
            duration_ms: 1000,
        };
        let err = backend
            .transcribe(audio, LangHint::auto())
            .await
            .expect_err("non-2xx must fail");
        let err_text = err.to_string();
        match err {
            PerceptionBackendError::Provider(msg) => {
                assert!(msg.contains("HTTP 500"), "must carry status: {msg}");
                assert!(msg.contains("body preview"), "must be a bounded preview: {msg}");
            }
            other => panic!("expected Provider, got {other:?}"),
        }
        // 预览窗口 (256 字节) 之外的 body 尾部不得进入错误文本:
        // END_OF_BODY_MARKER 位于 ~2KB 处, 若出现说明预览无界
        assert!(
            !err_text.contains("END_OF_BODY_MARKER"),
            "provider body beyond the preview window must not leak into error: {err_text}"
        );
        assert!(
            err_text.len() < 2048,
            "error text must stay bounded, got {} bytes",
            err_text.len()
        );
        // mock server must finish within 10s; if the client never connected
        // (e.g. policy rejection) fail on timeout instead of hanging forever.
        tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("mock server timed out - client never connected")
            .expect("server task");
    }

    // ============================================
    // P1 硬化: 传输策略 (HTTPS 默认强制)
    // ============================================

    #[test]
    fn transport_policy_matrix() {
        // HTTPS 始终放行
        assert!(WhisperHttpBackend::validate_transport_policy("https://api.openai.com/v1", false).is_ok());
        assert!(WhisperHttpBackend::validate_transport_policy("HTTPS://API.OpenAI.com/v1", false).is_ok());
        assert!(WhisperHttpBackend::validate_transport_policy("https://api.example.com", true).is_ok());
        // 空 host 拒绝
        assert!(WhisperHttpBackend::validate_transport_policy("https://", false).is_err());
        // scheme 缺失 / 非法协议拒绝
        assert!(WhisperHttpBackend::validate_transport_policy("api.example.com", false).is_err());
        assert!(WhisperHttpBackend::validate_transport_policy("", false).is_err());
        assert!(WhisperHttpBackend::validate_transport_policy("ftp://api.example.com", false).is_err());
        // 环回明文 HTTP 默认放行 (测试 / 本地 provider) — 含带端口形式
        assert!(WhisperHttpBackend::validate_transport_policy("http://localhost:8080/v1", false).is_ok());
        assert!(WhisperHttpBackend::validate_transport_policy("http://127.0.0.1:9000/v1", false).is_ok());
        assert!(WhisperHttpBackend::validate_transport_policy("http://127.9.9.9/v1", false).is_ok());
        assert!(WhisperHttpBackend::validate_transport_policy("http://[::1]:8080/v1", false).is_ok());
        assert!(WhisperHttpBackend::validate_transport_policy("http://[::1]/v1", false).is_ok());
        // 非环回明文 HTTP 默认拒绝 (含带端口的私网/公网地址); 显式 allow_insecure_http=true 放行
        assert!(WhisperHttpBackend::validate_transport_policy("http://api.example.com/v1", false).is_err());
        assert!(WhisperHttpBackend::validate_transport_policy("http://api.example.com:8080/v1", false).is_err());
        assert!(WhisperHttpBackend::validate_transport_policy("http://192.168.1.5:9000/v1", false).is_err());
        assert!(WhisperHttpBackend::validate_transport_policy("http://203.0.113.5/v1", false).is_err());
        assert!(WhisperHttpBackend::validate_transport_policy("http://203.0.113.5/v1", true).is_ok());
    }

    /// P1 硬化 (R): 非环回明文 HTTP 在发起任何网络前被策略拒绝.
    #[tokio::test]
    async fn non_loopback_plain_http_rejected_by_default() {
        let config = WhisperHttpConfig {
            base_url: "http://203.0.113.5/v1".to_string(), // TEST-NET-3, 非环回
            model: "whisper-1".to_string(),
            credential_key: "provider.whisper.api_key".to_string(),
            default_language: "en".to_string(),
            timeout: Duration::from_secs(2),
            max_upload_bytes: DEFAULT_MAX_UPLOAD_BYTES,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            allow_insecure_http: false,
            upload_mime_type: "audio/wav".to_string(),
        };
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "sk-test-1234567890abcdef"),
        );
        let backend = WhisperHttpBackend::new(config, creds);
        let err = backend
            .transcribe(
                AudioBuffer {
                    bytes: vec![0u8; 100],
                    duration_ms: 1000,
                },
                LangHint::auto(),
            )
            .await
            .expect_err("plain http to non-loopback must be rejected");
        match err {
            PerceptionBackendError::BackendUnavailable(msg) => {
                assert!(msg.contains("insecure plaintext http"), "must explain policy: {msg}");
            }
            other => panic!("expected policy BackendUnavailable, got {other:?}"),
        }
    }

    // ============================================
    // P1 硬化: 凭据不进入 Debug / 错误文本
    // ============================================

    /// P1 硬化 (T): 凭据值不得出现在配置 Debug 输出 (配置只持逻辑名).
    #[test]
    fn credential_value_absent_from_config_debug() {
        let config = WhisperHttpConfig::openai();
        let dbg = format!("{config:?}");
        assert!(
            !dbg.contains("sk-"),
            "config Debug must not contain any key-like value: {dbg}"
        );
        assert!(
            dbg.contains("provider.whisper.api_key"),
            "config Debug carries the logical key name (not a secret)"
        );
    }

    /// P1 硬化 (T): key 过短的错误只报长度, 不回显 key 值.
    #[tokio::test]
    async fn short_key_error_does_not_echo_key_value() {
        let creds = Arc::new(
            StaticCredentials::new().with("provider.whisper.api_key", "XYZZY_PLUGH_ZZ"),
        );
        let backend = WhisperHttpBackend::openai(creds);
        let err = backend
            .transcribe(
                AudioBuffer {
                    bytes: vec![0u8; 100],
                    duration_ms: 1000,
                },
                LangHint::auto(),
            )
            .await
            .expect_err("short key must fail");
        let err_text = err.to_string();
        assert!(
            !err_text.contains("XYZZY_PLUGH_ZZ"),
            "error must not echo the key value: {err_text}"
        );
    }
}
