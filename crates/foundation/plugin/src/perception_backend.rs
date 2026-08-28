//! P-arch (2026-08-28): Perception backend adapter trait.
//!
//! RC-7 真 modality 推进 (per `docs/04-internal/v2.0.0-rc-roadmap.md` §1.1).
//! 让 V/T/T (Voice/Vision/Tactile) 真 backend 可插拔.
//! 子代理 D handoff #5 (Cognitive module 不变量) 续.
//!
//! **设计原则** (per scene-d §5 决策 1, 复用 `LlmFactory` 同模式):
//! - trait 在 foundation (`apeireth-plugin`), impl 在 engine (`crates/engine/perception`).
//! - **多 backend 可选**: Whisper / MiniMax / 本地 whisper.cpp 等,
//!   runtime 通过 `Arc<dyn VoiceBackend>` 注入, 按配置或场景选.
//! - **0 装 PASS**: `WhisperBackend` 骨架不接真 HTTP, `transcribe()` 返
//!   `Err(PerceptionBackendError::BackendUnavailable)` 显式标注.
//! - **凭证不直接读 env**, 走 `CredentialResolver` (per RC-9 keyring 真接,
//!   `credentials::NoCredentials` / `StaticCredentials` 已就位).
//! - **HTTP 抽象**: `WhisperBackend` 不直接依赖 `reqwest` —
//!   `apeireth-plugin` 是 capability 契约 crate, 不持 HTTP client.
//!   runtime 在 engine 层装配 HTTP 后端 (复用 `ProviderCapability` 模式).
//!
//! **位置**: 与 `LlmFactory` (RC-5) / `MemoryBackend` (RC-1) / `CredentialResolver` 同位,
//! 都是 capability 抽象. 4 件 capability 在 foundation 集中.
//!
//! **3 阶审查** (O-6 锚 9, commit message 必写明):
//! 1. 总体: 与 RC-7 (Voice/Vision/Tactile 真 backend) + scene-d §5 决策 1 (多 backend 可选) 对齐
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致)
//! 3. 架构: runtime 拿 `Arc<dyn VoiceBackend>`, 3 模 trait 抽象统一入口语义
//!
//! **async-trait**: 用 `async_trait::async_trait` 宏 (per `llm_factory.rs` 同模式).
//! `LlmFactory` 用 `Box<dyn LlmInstance>` 返 (因为 LlmInstance 是 owned + Send),
//! `VoiceBackend` 用 `Arc<dyn VoiceBackend>` 注入 (因为 runtime 跨 turn 复用).
//!
//! **0 装 PASS**:
//! - `WhisperBackend::transcribe` 返 `Err(BackendUnavailable("Whisper API not wired..."))`.
//! - rc 阶段真生产路径: engine 层 `WhisperHttpBackend` 调 `/audio/transcriptions`,
//!   key 走 `CredentialResolver`, 复用 `ProviderCapability` HTTP 客户端.
//! - 不假装"已调通 Whisper API": 当前 alpha 没麦克风硬件 + MiniMax Coding Plan
//!   audio transcription 兼容性未确认.
//!
//! **v1 compat**: trait 是新增, 0 破现有 consumer (`VoiceInput` 仅扩字段, 默认 None
//! 仍返 `NotImplemented`, 现有测试 0 破).

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ============================================
// 公用类型 (AudioBuffer / LangHint / Transcription / ScreenshotBytes / TactileState)
// ============================================

/// 语言提示 (ISO 639-1 简化, per `apeireth-sdk-voice::AudioConfig::language`).
///
/// `None` = backend 自动推断 (Whisper 支持 detect_language, per OpenAI Whisper API).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LangHint(pub Option<String>);

impl LangHint {
    /// 自动推断
    pub fn auto() -> Self {
        Self(None)
    }
    /// 指定语言 (e.g. `LangHint::new("en")` / `LangHint::new("zh-CN")`)
    pub fn new(lang: impl Into<String>) -> Self {
        Self(Some(lang.into()))
    }
}

/// 音频缓冲 (per v1 `voice_session::AudioChunk` 简化版, 不绑具体采样率).
///
/// **设计**: `bytes` 是 raw PCM / WAV / MP3 (backend 自己解析), 不强 schema
/// —— backend 选择语义 (per scene-d §5 决策 1: 不同 backend 接受不同输入).
///
/// **0 装**: 当前不含 `format` / `sample_rate` 字段, 是**最小公约数**.
/// 真生产时如果 backend 要更细 schema, 加 wrapper struct 即可 (向后兼容).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioBuffer {
    /// 音频 bytes
    pub bytes: Vec<u8>,
    /// 估算长度 (毫秒; 0 = 未知)
    pub duration_ms: u64,
}

impl AudioBuffer {
    /// 空 buffer (用于 backend unavailable 测试)
    pub fn empty() -> Self {
        Self {
            bytes: Vec::new(),
            duration_ms: 0,
        }
    }
}

/// 转写结果 (per `apeireth-sdk-voice::stt::Transcription` 字段对齐,
/// 但 SttModel 字段不绑死 — backend 自由命名).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transcription {
    /// 转写后的文本
    pub text: String,
    /// backend 报告的模型名 (e.g. "whisper-1", "MiniMax/speech-01")
    pub model: String,
    /// 语言 (ISO 639-1, backend 报告; 可能与 `LangHint` 不同)
    pub language: String,
    /// 置信度 (0.0..=1.0; backend 不支持时填 None)
    pub confidence: Option<f32>,
    /// audio 长度 (毫秒)
    pub duration_ms: u64,
}

/// 截屏字节 (PNG/JPEG, 不解)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenshotBytes {
    /// 图像 bytes
    pub bytes: Vec<u8>,
    /// 格式 (per image header, e.g. "png" / "jpeg")
    pub format: String,
    /// 截屏时间戳 (epoch millis)
    pub captured_at_ms: i64,
}

/// 触觉状态 (0 装: v2.0 估不实现, v2.x 决定 sensor schema)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TactileState {
    /// 触觉强度 (0.0..=1.0)
    pub intensity: f32,
    /// 接触点 (x, y) 归一化坐标
    pub point: Option<(f32, f32)>,
    /// sensor 名字
    pub sensor: String,
}

// ============================================
// 统一错误
// ============================================

/// Perception backend 错误 (per `LlmError` 同 shape, per scene-d §5 决策 1
/// 多 backend 错误通道统一).
#[derive(Debug)]
pub enum PerceptionBackendError {
    /// backend 不可用 (Whisper 没启 / MiniMax API key 缺 / screen capture 失败)
    BackendUnavailable(String),
    /// 网络/HTTP 错
    Network(String),
    /// Rate limit (transient, per `LlmError::RateLimited`)
    RateLimited {
        retry_after_ms: u64,
    },
    /// Provider 返回错误 (4xx/5xx)
    Provider(String),
    /// 流中断 / 超时
    Stream(String),
    /// 音频格式错 (与 `apeireth-sdk-voice::VoiceError::AudioFormatInvalid` 对齐)
    Audio(String),
}

impl std::fmt::Display for PerceptionBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BackendUnavailable(m) => {
                write!(f, "perception backend unavailable: {m}")
            }
            Self::Network(m) => write!(f, "perception backend network error: {m}"),
            Self::RateLimited { retry_after_ms } => write!(
                f,
                "perception backend rate limited, retry after {retry_after_ms}ms"
            ),
            Self::Provider(m) => write!(f, "perception backend provider error: {m}"),
            Self::Stream(m) => write!(f, "perception backend stream error: {m}"),
            Self::Audio(m) => write!(f, "perception backend audio error: {m}"),
        }
    }
}

impl std::error::Error for PerceptionBackendError {}

// ============================================
// Backend traits (3 modality × 1 backend each)
// ============================================

/// Voice backend (STT).
///
/// **多 backend 可选** (Whisper / MiniMax / 本地 whisper.cpp), runtime 通过
/// `Arc<dyn VoiceBackend>` 注入, per scene-d §5 决策 1.
#[async_trait]
pub trait VoiceBackend: Send + Sync {
    /// 转语音 → text
    async fn transcribe(
        &self,
        audio: AudioBuffer,
        lang: LangHint,
    ) -> Result<Transcription, PerceptionBackendError>;

    /// backend 名字 (用于配置 + 监控, e.g. "whisper" / "MiniMax" / "local_whisper")
    fn name(&self) -> &'static str;

    /// 健康检查 (启动时调, per `LlmFactory` 同模式)
    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        Ok(())
    }
}

/// Vision backend (screen capture).
#[async_trait]
pub trait VisionBackend: Send + Sync {
    /// 截屏 → bytes
    async fn capture(&self) -> Result<ScreenshotBytes, PerceptionBackendError>;

    /// backend 名字
    fn name(&self) -> &'static str;

    /// 健康检查
    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        Ok(())
    }
}

/// Tactile backend (物理触觉 sensor).
///
/// **0 装**: v2.0 估不实现, v2.x 决定 sensor schema. trait 已就位, engine
/// 层真 sensor 接入时填 impl.
#[async_trait]
pub trait TactileBackend: Send + Sync {
    /// 读触觉状态
    async fn read(&self) -> Result<TactileState, PerceptionBackendError>;

    /// backend 名字
    fn name(&self) -> &'static str;

    /// 健康检查
    async fn ping(&self) -> Result<(), PerceptionBackendError> {
        Ok(())
    }
}

// ============================================
// WhisperBackend 骨架 (0 装: 不接真 HTTP)
// ============================================

/// Whisper backend (OpenAI-compatible `/audio/transcriptions` endpoint, 0 装骨架).
///
/// **0 装诚实**:
/// - 当前 `transcribe()` 返 `Err(BackendUnavailable)` 显式标注
/// - 不持 `reqwest::Client` (plugin crate 不依赖 HTTP lib, per
///   `crates/foundation/protocol/Cargo.toml` 注释 "library code must not
///   construct a reqwest")
/// - 凭证走 `credential_key` 名字 + `CredentialResolver`, 不读 env 不存 string
/// - rc 阶段真生产路径: engine 层 `WhisperHttpBackend` 调 OpenAI / MiniMax
///   endpoint, key 通过 `CredentialResolver::resolve(credential_key)` 拿
///
/// **endpoint 配置**:
/// - `base_url`: `"https://api.openai.com/v1"` (默认) / `"https://api.minimaxi.com/v1"`
/// - `model`: `"whisper-1"` (OpenAI 默认) / `"speech-01"` (MiniMax)
/// - `credential_key`: `"provider.whisper.api_key"` (OpenAI) /
///   `"provider.minimax.api_key"` (MiniMax)
pub struct WhisperBackend {
    /// API base URL (含 `/v1`)
    pub base_url: String,
    /// 模型 ID (e.g. `"whisper-1"`, `"speech-01"`)
    pub model: String,
    /// `CredentialResolver` 用的逻辑名 (per `CredentialResolver` 文档:
    /// "Names are logical, not locations")
    pub credential_key: String,
    /// 默认语言 (per `apeireth-sdk-voice::AudioConfig::language` 默认 `"en"`)
    pub default_language: String,
}

impl WhisperBackend {
    /// 默认 OpenAI Whisper 构造 (per K-1 强校验 `voice::MIN_API_KEY_LENGTH >= 16`,
    /// key 解析时校验, 不在构造时填)
    pub fn openai_default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "whisper-1".to_string(),
            credential_key: "provider.whisper.api_key".to_string(),
            default_language: "en".to_string(),
        }
    }

    /// MiniMax 构造 (兼容 `/v1/audio/transcriptions` 端点)
    pub fn minimax_default() -> Self {
        Self {
            base_url: "https://api.minimaxi.com/v1".to_string(),
            model: "speech-01".to_string(),
            credential_key: "provider.minimax.api_key".to_string(),
            default_language: "zh-CN".to_string(),
        }
    }

    /// 自定义构造 (per K-1 强校验守门, runtime 装配时调)
    pub fn custom(
        base_url: impl Into<String>,
        model: impl Into<String>,
        credential_key: impl Into<String>,
        default_language: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            credential_key: credential_key.into(),
            default_language: default_language.into(),
        }
    }
}

#[async_trait]
impl VoiceBackend for WhisperBackend {
    async fn transcribe(
        &self,
        _audio: AudioBuffer,
        _lang: LangHint,
    ) -> Result<Transcription, PerceptionBackendError> {
        // 0 装 PASS: 不接真 HTTP, 不假装"已调通 Whisper API". rc 阶段 engine
        // 层 `WhisperHttpBackend` 接管, 调 multipart/form-data POST
        // {base_url}/audio/transcriptions, Authorization: Bearer {secret}.
        Err(PerceptionBackendError::BackendUnavailable(format!(
            "Whisper backend not wired (model={}, base_url={}); RC-7 follow-up: engine layer \
             WhisperHttpBackend required; credential '{}' must resolve via CredentialResolver",
            self.model, self.base_url, self.credential_key
        )))
    }

    fn name(&self) -> &'static str {
        "whisper"
    }
}

// ============================================
// NoopVoiceBackend (0 装显式占位, 与 NoopLlmFactory 同模式)
// ============================================

/// 0 装 PASS: NoopVoiceBackend
/// 不调真 backend, 返 `BackendUnavailable`. 测试用 + alpha 0 装路径.
pub struct NoopVoiceBackend;

#[async_trait]
impl VoiceBackend for NoopVoiceBackend {
    async fn transcribe(
        &self,
        _audio: AudioBuffer,
        _lang: LangHint,
    ) -> Result<Transcription, PerceptionBackendError> {
        Err(PerceptionBackendError::BackendUnavailable(
            "NoopVoiceBackend (0 装 PASS; RC-7 follow-up 真接)".to_string(),
        ))
    }

    fn name(&self) -> &'static str {
        "noop"
    }
}

// ============================================
// 测试 (3 测, per RC-7 验收)
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::{CredentialResolver, NoCredentials, StaticCredentials};

    /// RC-7: VoiceInput 加 backend field 后, None backend 仍返 NotImplemented
    /// (0 装诚实保留); Some(backend) 返 BackendNotWired 提示装配路径.
    /// 兼容现有测试 (NotImplemented msg 仍含 "0 装" + "v2.1" substring).
    #[tokio::test]
    async fn voice_input_backend_unwired_returns_backend_not_wired_error() {
        use crate::perception::{PerceptionInput, PerceptionModality, VoiceInput};

        // 1. None backend → NotImplemented (现有契约保留)
        let sid = apeireth_core::kernel::SessionId::new();
        let v_none = VoiceInput {
            session_id: sid,
            backend: None,
        };
        let err_none = v_none
            .next_event()
            .expect_err("None backend must return NotImplemented");
        let s_none = err_none.to_string();
        assert!(s_none.contains("0 装"), "0 装 must be in msg: {s_none}");
        assert!(s_none.contains("v2.1"), "v2.1 path must be in msg: {s_none}");

        // 2. Some(noop backend) → BackendNotWired (新契约, 提示 runtime 装配)
        let v_some: VoiceInput = VoiceInput {
            session_id: sid,
            backend: Some(Arc::new(NoopVoiceBackend) as Arc<dyn VoiceBackend>),
        };
        let err_some = v_some
            .next_event()
            .expect_err("Some(backend) without poll() wiring must error");
        let s_some = err_some.to_string();
        assert!(
            s_some.contains("BackendNotWired") || s_some.contains("backend"),
            "BackendNotWired msg expected: {s_some}"
        );
        assert_eq!(v_some.modality(), PerceptionModality::Voice);
    }

    /// RC-7: WhisperBackend::transcribe 在没接 HTTP 时返 BackendUnavailable.
    /// 凭证走 CredentialResolver, NoCredentials 永返 None (模拟 keyring 空).
    /// 当前 skeleton 无论有没有 key 都返 BackendUnavailable (因为 HTTP 未接).
    #[tokio::test]
    async fn whisper_backend_unavailable_when_no_key() {
        let backend = WhisperBackend::openai_default();

        // NoCredentials 模拟 keyring 空 (per RC-9 keyring 真接前)
        let resolver = NoCredentials;
        assert!(
            resolver.resolve(&backend.credential_key).is_none(),
            "NoCredentials must yield None for any name"
        );

        // 当前 skeleton 不查 resolver (HTTP 未接), 但返 BackendUnavailable 显式标
        let audio = AudioBuffer::empty();
        let lang = LangHint::auto();
        let result = backend.transcribe(audio, lang).await;
        let err = result.expect_err("Whisper skeleton must error, never silently succeed");
        match err {
            PerceptionBackendError::BackendUnavailable(msg) => {
                assert!(msg.contains("Whisper backend not wired"));
                assert!(msg.contains("provider.whisper.api_key"));
                assert!(msg.contains("RC-7"));
            }
            other => panic!("expected BackendUnavailable, got {other:?}"),
        }

        // StaticCredentials 模拟有 key (per RC-9 后续 keyring 真接路径),
        // 当前 skeleton 同样返 BackendUnavailable (因为 HTTP 未接), 不假装"有 key 就通".
        let resolver_with_key = StaticCredentials::new()
            .with("provider.whisper.api_key", "sk-test-1234567890abcdef");
        let _ = resolver_with_key.resolve(&backend.credential_key); // 确认 resolver 能拿到
        let result2 = backend
            .transcribe(AudioBuffer::empty(), LangHint::auto())
            .await;
        assert!(result2.is_err(), "Whisper skeleton must error even with key (HTTP unwired)");
    }

    /// RC-7: `dyn VoiceBackend` 必须 Send + Sync, runtime 跨 turn 复用要满足.
    /// 编译期断言 (fn signature 要求); 不依赖运行时 assert.
    #[test]
    fn voice_backend_trait_send_sync_works() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}

        // trait object 形态: 编译期要求 dyn VoiceBackend: Send + Sync
        assert_send_sync::<dyn VoiceBackend>();

        // Arc<dyn VoiceBackend> 注入 runtime: 同样要 Send + Sync
        let backend: Arc<dyn VoiceBackend> = Arc::new(WhisperBackend::openai_default());
        let _round_trip: Arc<dyn VoiceBackend> = backend.clone();
        assert_eq!(backend.name(), "whisper");

        // NoopVoiceBackend 同形态
        let noop: Arc<dyn VoiceBackend> = Arc::new(NoopVoiceBackend);
        assert_eq!(noop.name(), "noop");
    }
}