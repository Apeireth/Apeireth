//! # 语音流协议客户端族 (采集配置 / 流式转写 / 流式合成 真现实现)
//!
//! 本族实现语音**协议客户端层**:
//!
//! - [`capture`]: 采集配置推导 (字节率 / 帧对齐) + 有界帧队列背压 + 生命周期
//! - [`stream`]: 流式转写 / 合成分块协议 + 信用窗口背压 + 组装器
//! - [`transport`]: 一元转写 / 合成请求响应帧 + 传输边界
//! - [`wake`]: 唤醒模板包络匹配引擎 (纯信号处理)
//! - [`vad`]: 流式 VAD 折叠状态机 (语音段门限 + 静音挂留)
//! - [`stt`] / [`tts`] / [`config`] / [`auth`]: 领域模型 + K-1 校验面
//! - [`error`]: 闭合错误词表 (网络 / 认证 / 协议 / 限流 / 超时 / 背压 / 状态)
//! - [`perception_bridge`]: 与感知后端契约 (`perception_backend`) 的字段对齐桥
//!
//! ## 架构边界
//!
//! 协议层**不直接碰网络**: 一元 / 流式 IO 全部经 [`transport::VoiceTransport`]
//! 边界注入 (生产注入 HTTP / 流式传输, 测试注入脚本化 mock, 零真实网络)。
//! 本地信号处理面 (唤醒 / VAD / 采集) 不需要凭证, 一元转写 / 合成需要凭证。
//!
//! 超时统一走 `apeireth_core::deadline` 熔合面; 日志面统一走 [`crate::redact`]
//! 脱敏 (凭证 / 令牌永不进错误文案与 Debug 输出)。
//!
//! ## 6 核心 API
//!
//! | # | API | 协议行为 |
//! |---:|---|---|
//! | 1 | `transcribe` | 一元转写 RPC (请求校验 + 超时熔合 + 响应严格解析) |
//! | 2 | `synthesize` | 一元合成 RPC (同上, 输出过 K-1 守门) |
//! | 3 | `detect_wake` | 唤醒模板包络匹配 (需先登记模板, 零假装) |
//! | 4 | `start_listening` | 采集会话 `Idle → Listening` |
//! | 5 | `stop_listening` | 采集会话 `Listening → Idle` (统计经 `stats()` 观测) |
//! | 6 | `stream_audio` | 流式 VAD 折叠 (门限 + 挂留 + EMA 平滑) |

#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 模块声明 + 重新导出
// ============================================================================

pub mod auth;
pub mod capture;
pub mod config;
pub mod error;
pub mod perception_bridge;
pub mod stream;
pub mod stt;
pub mod transport;
pub mod tts;
pub mod vad;
pub mod wake;

pub use crate::voice::auth::{
    AccessToken, ApiKeyHolder, DEFAULT_TOKEN_TTL_SECONDS, DEFAULT_VOICE_API_BASE,
    MAX_TOKEN_TTL_SECONDS, MIN_API_KEY_LENGTH, PLATFORM_NAME, PROVIDER_NAME,
    TYPICAL_API_KEY_LENGTH, VOICE_SCHEMA_VERSION,
};
pub use crate::voice::capture::{
    bytes_per_frame, bytes_per_second, duration_ms_for_bytes, frame_count_for_duration,
    CaptureSession, CaptureState, CaptureStats, DEFAULT_CAPTURE_QUEUE_FRAMES,
};
pub use crate::voice::config::{
    AudioConfig, VoiceConfig, DEFAULT_AUDIO_BIT_DEPTH, DEFAULT_AUDIO_CHANNELS,
    DEFAULT_AUDIO_FORMAT, DEFAULT_AUDIO_LANGUAGE, DEFAULT_AUDIO_SAMPLE_RATE,
    VOICE_CONFIG_SECTION_COUNT,
};
pub use crate::voice::error::{VoiceError, VoiceResult, VOICE_ERROR_VARIANT_COUNT};
pub use crate::voice::stream::{
    StreamFrame, StreamKind, StreamReceiver, StreamSender, SynthesisAssembler, TranscriptAssembler,
    DEFAULT_WINDOW_CHUNKS, MAX_CHUNK_BYTES,
};
pub use crate::voice::stt::{SttModel, SttRequest, Transcription, SUPPORTED_STT_MODELS};
pub use crate::voice::transport::{next_request_id, VoiceFrame, VoiceTransport, VOICE_FRAME_COUNT};
pub use crate::voice::tts::{Audio, TtsModel, TtsRequest, SUPPORTED_TTS_MODELS};
pub use crate::voice::vad::{
    VadAlgorithm, VadConfig, VadResult, VadStreamState, SUPPORTED_VAD_ALGORITHMS,
};
pub use crate::voice::wake::{
    envelope_similarity, extract_envelope, WakeDetector, WakeTemplate, WakeWord, WakeWordCategory,
    WakeWordDetection, DEFAULT_WAKE_THRESHOLD, MAX_CUSTOM_WAKE_WORD_LENGTH, MIN_WAKE_WORD_LENGTH,
    SUPPORTED_WAKE_WORD_CATEGORIES, VOICE_DEFAULT_WAKE_WORD,
};

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use crate::voice::capture::duration_ms_for_bytes as duration_ms_for_pcm;
use crate::voice::transport::{classify_error_frame, next_request_id as next_rpc_id};

// ============================================================================
// §1 编译期常量
// ============================================================================

/// 语音 schema 版本.
pub use crate::voice::auth::VOICE_SCHEMA_VERSION as SCHEMA_VERSION;

/// 6 核心 API 数量常量.
pub const CORE_API_COUNT: usize = 6;

/// 4 STT 模型数量常量.
pub const STT_MODEL_COUNT: usize = 4;

/// 4 TTS 模型数量常量.
pub const TTS_MODEL_COUNT: usize = 4;

/// 4 唤醒词类别数量常量.
pub const WAKE_WORD_CATEGORY_COUNT: usize = 4;

/// 3 VAD 算法数量常量.
pub const VAD_ALGORITHM_COUNT: usize = 3;

/// 6 K-1 强校验数量常量.
pub const K1_STRONG_VALIDATION_COUNT: usize = 6;

/// 采集会话默认队列容量 (帧).
pub const SESSION_CHANNEL_CAPACITY: usize = 100;

/// 一元 RPC 默认超时 (毫秒, 走 `apeireth_core::deadline::clamp_timeout` 过闸).
pub const DEFAULT_OP_TIMEOUT_MS: u64 = 30_000;

/// 一元 RPC 超时上限 (毫秒).
pub const MAX_OP_TIMEOUT_MS: u64 = 120_000;

// ============================================================================
// §2 实现状态标志 + 工具白名单
// ============================================================================

/// 协议层实现状态: `false` = 协议逻辑已全量实现 (真现实现).
///
/// 传输实现仍经 [`VoiceTransport`] 边界注入 —— 这是架构边界, 不是未实现面。
pub const STUB_MODE: bool = false;

/// 查询协议层实现状态 (兼容观测面).
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 工具白名单 (6 核心 API + 1 状态查询 = 7, 编译期 hardcode).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_voice_transcribe",
    "apeireth_voice_synthesize",
    "apeireth_voice_detect_wake",
    "apeireth_voice_start_listening",
    "apeireth_voice_stop_listening",
    "apeireth_voice_stream_audio",
    "apeireth_voice_stub_status",
];

/// 白名单工具数.
pub const TOOL_WHITELIST_COUNT: usize = 7;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// 校验工具调用是否在白名单内 (m3 防御).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), VoiceError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(VoiceError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §3 VoiceClient trait (6 核心 API)
// ============================================================================

/// 语音流协议客户端 (6 核心 API).
#[async_trait]
pub trait VoiceClient: Send + Sync {
    /// **API 1**: `transcribe` — 一元转写.
    async fn transcribe(&self, request: &SttRequest) -> Result<Transcription, VoiceError>;

    /// **API 2**: `synthesize` — 一元合成.
    async fn synthesize(&self, request: &TtsRequest) -> Result<Audio, VoiceError>;

    /// **API 3**: `detect_wake` — 唤醒模板包络匹配 (需先登记模板).
    async fn detect_wake(&self, audio: &[i16]) -> Result<WakeWordDetection, VoiceError>;

    /// **API 4**: `start_listening` — 开始采集.
    async fn start_listening(&self) -> Result<(), VoiceError>;

    /// **API 5**: `stop_listening` — 停止采集 (统计经采集会话 `stats()` 观测).
    async fn stop_listening(&self) -> Result<(), VoiceError>;

    /// **API 6**: `stream_audio` — 流式 VAD 折叠.
    async fn stream_audio(&self, vad_result: &VadResult) -> Result<VadResult, VoiceError>;
}

// ============================================================================
// §4 VoiceClientImpl
// ============================================================================

/// 语音客户端实现: 凭证 + 配置 + 采集会话 + 唤醒检测器 + VAD 折叠 + 传输边界.
#[derive(Clone)]
pub struct VoiceClientImpl {
    platform: String,
    api_key_holder: ApiKeyHolder,
    config: VoiceConfig,
    op_timeout_ms: u64,
    capture: Arc<Mutex<CaptureSession>>,
    wake_detector: Arc<Mutex<WakeDetector>>,
    vad_stream: Arc<Mutex<VadStreamState>>,
    transport: Option<Arc<dyn VoiceTransport>>,
}

impl std::fmt::Debug for VoiceClientImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceClientImpl")
            .field("platform", &self.platform)
            .field("api_key_holder", &self.api_key_holder)
            .field("config", &self.config)
            .field("listening", &self.is_listening())
            .field("transport", &self.transport.is_some())
            .finish_non_exhaustive()
    }
}

impl VoiceClientImpl {
    /// 创建客户端 (默认配置, 传输未注入).
    pub fn new() -> Self {
        let config = VoiceConfig::default();
        info!(
            target: "apeireth_voice",
            platform = PLATFORM_NAME,
            wake_word = %config.wake.keyword,
            "voice client created (protocol layer implemented; transport injected separately)"
        );
        let capture = CaptureSession::from_voice_config(&config)
            .expect("default capture config must be valid");
        let vad_stream = VadStreamState::new(&config.vad);
        Self {
            platform: PLATFORM_NAME.to_string(),
            api_key_holder: ApiKeyHolder::empty(),
            config,
            op_timeout_ms: DEFAULT_OP_TIMEOUT_MS,
            capture: Arc::new(Mutex::new(capture)),
            wake_detector: Arc::new(Mutex::new(
                WakeDetector::new(DEFAULT_WAKE_THRESHOLD)
                    .expect("default wake threshold must be valid"),
            )),
            vad_stream: Arc::new(Mutex::new(vad_stream)),
            transport: None,
        }
    }

    /// 设置一元 RPC 超时 (毫秒, 经 `apeireth_core::deadline::clamp_timeout` 过闸).
    pub fn set_op_timeout_ms(&mut self, timeout_ms: u64) -> Result<(), VoiceError> {
        apeireth_core::deadline::clamp_timeout(
            Some(timeout_ms),
            DEFAULT_OP_TIMEOUT_MS,
            MAX_OP_TIMEOUT_MS,
        )
        .map_err(|e| VoiceError::from_deadline(e, "set_op_timeout"))?;
        self.op_timeout_ms = timeout_ms;
        Ok(())
    }

    /// 平台名.
    pub fn platform(&self) -> &str {
        &self.platform
    }

    /// 当前配置.
    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    /// 替换配置 (K-1 全量校验; 采集会话与 VAD 折叠随配置重建).
    pub fn set_config(&mut self, config: VoiceConfig) -> Result<(), VoiceError> {
        config.validate()?;
        let capture = CaptureSession::from_voice_config(&config)?;
        let vad_stream = VadStreamState::new(&config.vad);
        self.config = config;
        *self.capture.lock().expect("capture lock") = capture;
        *self.vad_stream.lock().expect("vad lock") = vad_stream;
        Ok(())
    }

    /// 是否已设置 API key.
    pub fn has_api_key(&self) -> bool {
        self.api_key_holder.is_set()
    }

    /// 设置 API key (K-1 #1 守门).
    pub fn set_api_key(&mut self, api_key: String) -> Result<(), VoiceError> {
        self.api_key_holder.set(api_key)
    }

    /// 是否正在采集.
    pub fn is_listening(&self) -> bool {
        self.capture.lock().expect("capture lock").state() == CaptureState::Listening
    }

    /// 当前默认唤醒词.
    pub fn default_wake_word(&self) -> &str {
        &self.config.wake.keyword
    }

    /// 当前 STT 模型.
    pub fn stt_model(&self) -> SttModel {
        self.config.stt
    }

    /// 当前 TTS 模型.
    pub fn tts_model(&self) -> TtsModel {
        self.config.tts
    }

    /// 当前 VAD 算法.
    pub fn vad_algorithm(&self) -> VadAlgorithm {
        self.config.vad.algorithm
    }

    /// 注入语音传输 (生产 HTTP / 流式传输, 测试 mock).
    pub fn set_transport(&mut self, transport: Arc<dyn VoiceTransport>) {
        self.transport = Some(transport);
    }

    /// 是否已注入传输.
    pub fn has_transport(&self) -> bool {
        self.transport.is_some()
    }

    /// 借用采集会话锁 (驱动层 / 测试观测用).
    pub fn capture_session(&self) -> Arc<Mutex<CaptureSession>> {
        self.capture.clone()
    }

    /// 登记唤醒模板 (以真实发音 PCM; 见 [`WakeDetector::enroll`]).
    pub fn enroll_wake_template(&self, samples: &[i16]) -> Result<(), VoiceError> {
        self.wake_detector
            .lock()
            .expect("wake lock")
            .enroll(&self.config.wake, samples)
    }

    /// 健康检查 (本地校验, 零网络).
    pub async fn health_check(&self) -> Result<(), VoiceError> {
        if self.platform.is_empty() {
            return Err(VoiceError::InvalidArgument("platform is empty".to_string()));
        }
        self.config.validate()?;
        debug!(target: "apeireth_voice", platform = %self.platform, "health_check: local checks ok");
        Ok(())
    }

    /// 4 STT 模型列表.
    pub fn list_stt_models() -> &'static [SttModel] {
        SUPPORTED_STT_MODELS
    }

    /// 4 TTS 模型列表.
    pub fn list_tts_models() -> &'static [TtsModel] {
        SUPPORTED_TTS_MODELS
    }

    /// 4 唤醒词类别列表.
    pub fn list_wake_word_categories() -> &'static [WakeWordCategory] {
        SUPPORTED_WAKE_WORD_CATEGORIES
    }

    /// 3 VAD 算法列表.
    pub fn list_vad_algorithms() -> &'static [VadAlgorithm] {
        SUPPORTED_VAD_ALGORITHMS
    }

    /// 6 核心 API 名列表.
    pub fn list_apis() -> &'static [&'static str] {
        &TOOL_WHITELIST[..CORE_API_COUNT]
    }

    /// 状态上报 (含协议层实现标志 / 传输注入状态 / 采集状态).
    pub fn stub_status(&self) -> StubStatus {
        StubStatus {
            stub_mode: STUB_MODE,
            platform: self.platform.clone(),
            schema_version: VOICE_SCHEMA_VERSION.to_string(),
            api_key_set: self.api_key_holder.is_set(),
            listening: self.is_listening(),
            default_wake_word: self.config.wake.keyword.clone(),
            wake_word_category: self.config.wake.category,
            stt_model: self.config.stt,
            tts_model: self.config.tts,
            vad_algorithm: self.config.vad.algorithm,
            transport_configured: self.has_transport(),
        }
    }

    // ---------- 内部: 一元 RPC 驱动 ----------

    async fn exchange_under_deadline(
        &self,
        request: VoiceFrame,
        operation: &'static str,
    ) -> Result<VoiceFrame, VoiceError> {
        let transport = self
            .transport
            .clone()
            .ok_or(VoiceError::TransportUnavailable)?;
        let timeout_ms = apeireth_core::deadline::clamp_timeout(
            Some(self.op_timeout_ms),
            DEFAULT_OP_TIMEOUT_MS,
            MAX_OP_TIMEOUT_MS,
        )
        .map_err(|e| VoiceError::from_deadline(e, operation))?;
        let (_deadline, mut notice) =
            apeireth_core::deadline::Deadline::after(std::time::Duration::from_millis(timeout_ms))
                .map_err(|e| VoiceError::from_deadline(e, operation))?;
        tokio::select! {
            _ = notice.notified() => Err(VoiceError::Timeout { operation }),
            response = transport.exchange(request) => response,
        }
    }
}

impl Default for VoiceClientImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// 状态上报 (观测面).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StubStatus {
    /// 协议层实现标志 (恒 `false`: 已全量实现)
    pub stub_mode: bool,
    /// 平台名
    pub platform: String,
    /// schema 版本
    pub schema_version: String,
    /// 是否已设置 API key
    pub api_key_set: bool,
    /// 是否正在采集
    pub listening: bool,
    /// 当前默认唤醒词
    pub default_wake_word: String,
    /// 当前唤醒词类别
    pub wake_word_category: WakeWordCategory,
    /// 当前 STT 模型
    pub stt_model: SttModel,
    /// 当前 TTS 模型
    pub tts_model: TtsModel,
    /// 当前 VAD 算法
    pub vad_algorithm: VadAlgorithm,
    /// 是否已注入语音传输
    pub transport_configured: bool,
}

// ============================================================================
// §5 6 核心 API 实现 (协议真现实现, IO 经传输边界)
// ============================================================================

#[async_trait]
impl VoiceClient for VoiceClientImpl {
    #[instrument(skip(self, request), fields(model = ?request.model, language = ?request.language))]
    async fn transcribe(&self, request: &SttRequest) -> Result<Transcription, VoiceError> {
        let tool_name = "apeireth_voice_transcribe";
        validate_tool_call(tool_name, &serde_json::json!({ "model": request.model }))?;
        if !self.api_key_holder.is_set() {
            return Err(VoiceError::ApiKeyMissing);
        }
        // 请求面 K-1 复检 (防手工绕过构造校验)
        VoiceError::validate_audio_format(&request.format)?;
        VoiceError::validate_sample_rate(request.sample_rate)?;
        VoiceError::validate_bit_depth(request.bit_depth)?;
        VoiceError::validate_channels(request.channels)?;
        if let Some(lang) = &request.language {
            VoiceError::validate_language(lang)?;
        }
        if request.audio.is_empty() {
            return Err(VoiceError::InvalidArgument("audio is empty".to_string()));
        }
        // 模型时长上限守门 (按 PCM 字节率折算)
        let duration_ms = duration_ms_for_pcm(
            request.audio.len(),
            request.sample_rate,
            request.bit_depth,
            request.channels,
        );
        let limit_ms = u64::from(request.model.max_audio_seconds()) * 1_000;
        if duration_ms > limit_ms {
            return Err(VoiceError::InvalidArgument(format!(
                "audio {duration_ms}ms exceeds model limit {limit_ms}ms"
            )));
        }

        let request_id = next_rpc_id();
        let wire = VoiceFrame::TranscribeRequest {
            request_id,
            model: request.model.as_str().to_string(),
            format: request.format.clone(),
            sample_rate: request.sample_rate,
            bit_depth: request.bit_depth,
            channels: request.channels,
            language: request.language.clone(),
            audio: request.audio.clone(),
        };
        let response = self.exchange_under_deadline(wire, "transcribe").await?;
        match response {
            VoiceFrame::TranscribeResponse {
                request_id: rid,
                text,
                model,
                language,
                confidence,
                duration_ms,
            } => {
                if rid != request_id {
                    return Err(VoiceError::Protocol(format!(
                        "transcribe response correlation mismatch: sent {request_id}, got {rid}"
                    )));
                }
                let model = perception_bridge::parse_model(&model)?;
                VoiceError::validate_language(&language).map_err(|e| {
                    VoiceError::Protocol(format!("response carries invalid language: {e}"))
                })?;
                let confidence = confidence.unwrap_or(0.0);
                if !(0.0..=1.0).contains(&confidence) {
                    return Err(VoiceError::Protocol(format!(
                        "response confidence {confidence} out of range [0.0, 1.0]"
                    )));
                }
                Ok(Transcription::new(
                    text,
                    model,
                    language,
                    confidence,
                    duration_ms,
                ))
            }
            VoiceFrame::Error {
                code,
                message,
                retry_after_ms,
                ..
            } => Err(classify_error_frame(&code, &message, retry_after_ms)),
            other => Err(VoiceError::Protocol(format!(
                "unexpected response frame `{}` for transcribe",
                other.type_str()
            ))),
        }
    }

    #[instrument(skip(self, request), fields(model = ?request.model, voice = %request.voice))]
    async fn synthesize(&self, request: &TtsRequest) -> Result<Audio, VoiceError> {
        let tool_name = "apeireth_voice_synthesize";
        validate_tool_call(tool_name, &serde_json::json!({ "model": request.model }))?;
        if !self.api_key_holder.is_set() {
            return Err(VoiceError::ApiKeyMissing);
        }
        // 请求面复检
        if request.text.trim().is_empty() {
            return Err(VoiceError::InvalidArgument("tts text is empty".to_string()));
        }
        if request.text.len() > request.model.max_text_length() {
            return Err(VoiceError::InvalidArgument(format!(
                "tts text too long: {} > {}",
                request.text.len(),
                request.model.max_text_length()
            )));
        }

        let request_id = next_rpc_id();
        let wire = VoiceFrame::SynthesisRequest {
            request_id,
            model: request.model.as_str().to_string(),
            voice: request.voice.clone(),
            language: request.language.clone(),
            output_format: request.output_format.clone(),
            sample_rate: request.sample_rate,
            text: request.text.clone(),
        };
        let response = self.exchange_under_deadline(wire, "synthesize").await?;
        match response {
            VoiceFrame::SynthesisResponse {
                request_id: rid,
                data,
                format,
                sample_rate,
                bit_depth,
                channels,
                duration_ms,
            } => {
                if rid != request_id {
                    return Err(VoiceError::Protocol(format!(
                        "synthesis response correlation mismatch: sent {request_id}, got {rid}"
                    )));
                }
                Audio::new(data, format, sample_rate, bit_depth, channels, duration_ms).map_err(
                    |e| VoiceError::Protocol(format!("invalid synthesis response parameters: {e}")),
                )
            }
            VoiceFrame::Error {
                code,
                message,
                retry_after_ms,
                ..
            } => Err(classify_error_frame(&code, &message, retry_after_ms)),
            other => Err(VoiceError::Protocol(format!(
                "unexpected response frame `{}` for synthesize",
                other.type_str()
            ))),
        }
    }

    #[instrument(skip(self, audio), fields(audio_len = audio.len()))]
    async fn detect_wake(&self, audio: &[i16]) -> Result<WakeWordDetection, VoiceError> {
        let tool_name = "apeireth_voice_detect_wake";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        self.wake_detector
            .lock()
            .expect("wake lock")
            .detect(&self.config.wake, audio)
    }

    #[instrument(skip(self))]
    async fn start_listening(&self) -> Result<(), VoiceError> {
        let tool_name = "apeireth_voice_start_listening";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        self.config.validate()?;
        self.capture.lock().expect("capture lock").start()
    }

    #[instrument(skip(self))]
    async fn stop_listening(&self) -> Result<(), VoiceError> {
        let tool_name = "apeireth_voice_stop_listening";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        let stats = self.capture.lock().expect("capture lock").stop()?;
        debug!(
            target: "apeireth_voice",
            frames_pushed = stats.frames_pushed,
            bytes_pushed = stats.bytes_pushed,
            "capture session stopped"
        );
        Ok(())
    }

    #[instrument(skip(self, vad_result), fields(is_speech = vad_result.is_speech))]
    async fn stream_audio(&self, vad_result: &VadResult) -> Result<VadResult, VoiceError> {
        let tool_name = "apeireth_voice_stream_audio";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        Ok(self.vad_stream.lock().expect("vad lock").fold(vad_result))
    }
}

// ============================================================================
// §6 测试 (mock 边界: 脚本化传输 + 6 核心 API 全流程)
// ============================================================================

#[cfg(test)]
pub(crate) mod mock {
    //! 脚本化语音传输 mock: 协议层测试的唯一 IO 边界 (零真实网络).

    use super::*;
    use std::collections::VecDeque;

    use crate::voice::stream::StreamFrame;
    use crate::voice::transport::{VoiceFrame, VoiceTransport};

    /// 脚本化转写响应.
    #[derive(Debug, Clone)]
    pub struct MockTranscribeResponse {
        /// 转写文本.
        pub text: String,
        /// 模型标识.
        pub model: String,
        /// 语言.
        pub language: String,
        /// 置信度.
        pub confidence: Option<f32>,
        /// 时长 (毫秒).
        pub duration_ms: u64,
    }

    /// 脚本化合成响应.
    #[derive(Debug, Clone)]
    pub struct MockSynthesisResponse {
        /// 音频字节.
        pub data: Vec<u8>,
        /// 格式.
        pub format: String,
        /// 采样率.
        pub sample_rate: u32,
        /// 位深.
        pub bit_depth: u16,
        /// 通道数.
        pub channels: u8,
        /// 时长 (毫秒).
        pub duration_ms: u64,
    }

    /// 脚本化语音传输.
    #[derive(Debug, Default)]
    pub struct MockVoiceTransport {
        transcribe_script: Mutex<VecDeque<Result<MockTranscribeResponse, VoiceError>>>,
        synthesis_script: Mutex<VecDeque<Result<MockSynthesisResponse, VoiceError>>>,
        stream_inbound: Mutex<VecDeque<StreamFrame>>,
        stream_sent: Mutex<Vec<StreamFrame>>,
        requests: Mutex<Vec<VoiceFrame>>,
        fail_exchange: Mutex<Option<VoiceError>>,
        stall_when_empty: bool,
    }

    impl MockVoiceTransport {
        /// 创建 (空脚本, 耗尽时返传输错误).
        pub fn new() -> Self {
            Self::default()
        }

        /// 创建 (空脚本且耗尽后永久挂起, 用于超时路径).
        pub fn stalling() -> Self {
            Self {
                stall_when_empty: true,
                ..Self::default()
            }
        }

        /// 压入一元转写脚本 (按序消费).
        pub fn script_transcribe(&self, response: Result<MockTranscribeResponse, VoiceError>) {
            self.transcribe_script
                .lock()
                .expect("mock lock")
                .push_back(response);
        }

        /// 压入一元合成脚本 (按序消费).
        pub fn script_synthesis(&self, response: Result<MockSynthesisResponse, VoiceError>) {
            self.synthesis_script
                .lock()
                .expect("mock lock")
                .push_back(response);
        }

        /// 压入流式下行脚本.
        pub fn script_stream(&self, frames: Vec<StreamFrame>) {
            self.stream_inbound
                .lock()
                .expect("mock lock")
                .extend(frames);
        }

        /// 注入下一次一元调用失败.
        pub fn fail_next_exchange(&self, err: VoiceError) {
            *self.fail_exchange.lock().expect("mock lock") = Some(err);
        }

        /// 已收一元请求 (按序).
        pub fn requests(&self) -> Vec<VoiceFrame> {
            self.requests.lock().expect("mock lock").clone()
        }

        /// 已发流式帧 (按序).
        pub fn sent_stream(&self) -> Vec<StreamFrame> {
            self.stream_sent.lock().expect("mock lock").clone()
        }
    }

    #[async_trait]
    impl VoiceTransport for MockVoiceTransport {
        async fn exchange(&self, request: VoiceFrame) -> Result<VoiceFrame, VoiceError> {
            self.requests
                .lock()
                .expect("mock lock")
                .push(request.clone());
            if let Some(err) = self.fail_exchange.lock().expect("mock lock").take() {
                return Err(err);
            }
            match request {
                VoiceFrame::TranscribeRequest { request_id, .. } => {
                    let next = self
                        .transcribe_script
                        .lock()
                        .expect("mock lock")
                        .pop_front();
                    match next {
                        Some(Ok(r)) => Ok(VoiceFrame::TranscribeResponse {
                            request_id,
                            text: r.text,
                            model: r.model,
                            language: r.language,
                            confidence: r.confidence,
                            duration_ms: r.duration_ms,
                        }),
                        Some(Err(e)) => Err(e),
                        None => self.exhausted_unary().await,
                    }
                }
                VoiceFrame::SynthesisRequest { request_id, .. } => {
                    let next = self.synthesis_script.lock().expect("mock lock").pop_front();
                    match next {
                        Some(Ok(r)) => Ok(VoiceFrame::SynthesisResponse {
                            request_id,
                            data: r.data,
                            format: r.format,
                            sample_rate: r.sample_rate,
                            bit_depth: r.bit_depth,
                            channels: r.channels,
                            duration_ms: r.duration_ms,
                        }),
                        Some(Err(e)) => Err(e),
                        None => self.exhausted_unary().await,
                    }
                }
                other => Err(VoiceError::Protocol(format!(
                    "mock: unexpected request frame `{}`",
                    other.type_str()
                ))),
            }
        }

        async fn send_stream(&self, frame: StreamFrame) -> Result<(), VoiceError> {
            self.stream_sent.lock().expect("mock lock").push(frame);
            Ok(())
        }

        async fn recv_stream(&self) -> Result<StreamFrame, VoiceError> {
            if let Some(frame) = self.stream_inbound.lock().expect("mock lock").pop_front() {
                return Ok(frame);
            }
            self.exhausted().await
        }
    }

    impl MockVoiceTransport {
        async fn exhausted(&self) -> Result<StreamFrame, VoiceError> {
            if self.stall_when_empty {
                futures::future::pending::<()>().await;
            }
            Err(VoiceError::Network(
                "mock: scripted frames exhausted".to_string(),
            ))
        }

        async fn exhausted_unary(&self) -> Result<VoiceFrame, VoiceError> {
            if self.stall_when_empty {
                futures::future::pending::<()>().await;
            }
            Err(VoiceError::Network(
                "mock: scripted frames exhausted".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::{MockSynthesisResponse, MockTranscribeResponse, MockVoiceTransport};
    use super::*;
    use crate::error_taxonomy::{ClassifyError, ErrorCategory};

    fn ready_client() -> (VoiceClientImpl, Arc<MockVoiceTransport>) {
        let mut client = VoiceClientImpl::new();
        client
            .set_api_key("sk-voice-abcdef1234567890xyz".to_string())
            .expect("valid api key");
        let transport = Arc::new(MockVoiceTransport::new());
        client.set_transport(transport.clone());
        (client, transport)
    }

    fn wav_request() -> SttRequest {
        SttRequest::new(
            vec![0u8; 32_000],
            "wav".to_string(),
            16_000,
            16,
            1,
            SttModel::default(),
            Some("en".to_string()),
        )
        .expect("valid request")
    }

    #[tokio::test]
    async fn transcribe_roundtrip_with_strict_response_parsing() {
        let (client, transport) = ready_client();
        transport.script_transcribe(Ok(MockTranscribeResponse {
            text: "hello".to_string(),
            model: SttModel::default().as_str().to_string(),
            language: "en".to_string(),
            confidence: Some(0.9),
            duration_ms: 1_000,
        }));

        let out = client.transcribe(&wav_request()).await.expect("transcribe");
        assert_eq!(out.text, "hello");
        assert_eq!(out.model, SttModel::default());
        assert_eq!(out.language, "en");
        assert!((out.confidence - 0.9).abs() < 1e-6);
        assert_eq!(out.duration_ms, 1_000);

        // 请求帧形状
        let requests = transport.requests();
        assert!(matches!(
            &requests[0],
            VoiceFrame::TranscribeRequest { format, sample_rate, channels, .. }
                if format == "wav" && *sample_rate == 16_000 && *channels == 1
        ));
    }

    #[tokio::test]
    async fn transcribe_requires_credential_and_transport() {
        let mut client = VoiceClientImpl::new();
        // 无凭证
        assert!(matches!(
            client.transcribe(&wav_request()).await,
            Err(VoiceError::ApiKeyMissing)
        ));
        client
            .set_api_key("sk-voice-abcdef1234567890xyz".to_string())
            .expect("valid");
        // 无传输
        let err = client
            .transcribe(&wav_request())
            .await
            .expect_err("must fail");
        assert_eq!(err, VoiceError::TransportUnavailable);
        assert!(!err.is_retryable());
    }

    #[tokio::test]
    async fn transcribe_revalidates_request_and_model_limits() {
        let (client, _transport) = ready_client();
        // 空音频
        let empty = SttRequest::new(
            vec![],
            "wav".to_string(),
            16_000,
            16,
            1,
            SttModel::default(),
            None,
        )
        .expect("valid request");
        assert!(matches!(
            client.transcribe(&empty).await,
            Err(VoiceError::InvalidArgument(_))
        ));
        // 超模型时长上限 (默认模型 30s; 31s 音频)
        let long = SttRequest::new(
            vec![0u8; 32_000 * 31],
            "wav".to_string(),
            16_000,
            16,
            1,
            SttModel::default(),
            None,
        )
        .expect("valid request");
        assert!(matches!(
            client.transcribe(&long).await,
            Err(VoiceError::InvalidArgument(_))
        ));
    }

    #[tokio::test]
    async fn transcribe_rejects_malformed_responses() {
        let (client, transport) = ready_client();
        // 未知模型标识 → Protocol
        transport.script_transcribe(Ok(MockTranscribeResponse {
            text: "x".to_string(),
            model: "bogus-model".to_string(),
            language: "en".to_string(),
            confidence: None,
            duration_ms: 0,
        }));
        assert!(matches!(
            client.transcribe(&wav_request()).await,
            Err(VoiceError::Protocol(_))
        ));

        // 越界置信度 → Protocol
        transport.script_transcribe(Ok(MockTranscribeResponse {
            text: "x".to_string(),
            model: SttModel::default().as_str().to_string(),
            language: "en".to_string(),
            confidence: Some(1.5),
            duration_ms: 0,
        }));
        assert!(matches!(
            client.transcribe(&wav_request()).await,
            Err(VoiceError::Protocol(_))
        ));

        // 非法语言 → Protocol
        transport.script_transcribe(Ok(MockTranscribeResponse {
            text: "x".to_string(),
            model: SttModel::default().as_str().to_string(),
            language: "english".to_string(),
            confidence: None,
            duration_ms: 0,
        }));
        assert!(matches!(
            client.transcribe(&wav_request()).await,
            Err(VoiceError::Protocol(_))
        ));
    }

    #[tokio::test]
    async fn transcribe_error_frames_classify_into_closed_vocabulary() {
        let (client, transport) = ready_client();
        transport.script_transcribe(Err(VoiceError::RateLimited {
            retry_after_ms: 2_500,
        }));
        let err = client
            .transcribe(&wav_request())
            .await
            .expect_err("must fail");
        assert_eq!(err.category(), ErrorCategory::RateLimited);
        assert_eq!(err.retry_after_ms(), Some(2_500));

        transport.script_transcribe(Err(VoiceError::Network("reset".into())));
        let err = client
            .transcribe(&wav_request())
            .await
            .expect_err("must fail");
        assert_eq!(err.category(), ErrorCategory::Network);
        assert!(err.is_retryable());
    }

    #[tokio::test]
    async fn transcribe_timeout_uses_deadline_fusion() {
        let mut client = VoiceClientImpl::new();
        client
            .set_api_key("sk-voice-abcdef1234567890xyz".to_string())
            .expect("valid");
        client.set_op_timeout_ms(50).expect("valid timeout");
        // 无脚本 + 永久挂起: 只能靠超时熔合退出
        client.set_transport(Arc::new(MockVoiceTransport::stalling()));
        let err = client
            .transcribe(&wav_request())
            .await
            .expect_err("must time out");
        assert!(matches!(
            err,
            VoiceError::Timeout {
                operation: "transcribe"
            }
        ));
        assert_eq!(err.category(), ErrorCategory::Timeout);

        // 超时取值过闸: 0 / 超上限被拒
        assert!(matches!(
            client.set_op_timeout_ms(0),
            Err(VoiceError::InvalidArgument(_))
        ));
        assert!(matches!(
            client.set_op_timeout_ms(MAX_OP_TIMEOUT_MS + 1),
            Err(VoiceError::InvalidArgument(_))
        ));
    }

    #[tokio::test]
    async fn synthesize_roundtrip_validates_output_parameters() {
        let (client, transport) = ready_client();
        transport.script_synthesis(Ok(MockSynthesisResponse {
            data: vec![1, 2, 3],
            format: "mp3".to_string(),
            sample_rate: 24_000,
            bit_depth: 16,
            channels: 1,
            duration_ms: 500,
        }));
        let request = TtsRequest::with_defaults(
            "hello".to_string(),
            TtsModel::default(),
            "voice-1".to_string(),
            "en".to_string(),
        )
        .expect("valid request");
        let audio = client.synthesize(&request).await.expect("synthesize");
        assert_eq!(audio.data, vec![1, 2, 3]);
        assert_eq!(audio.format, "mp3");
        assert_eq!(audio.duration_ms, 500);

        // 输出参数非法 → Protocol (服务端违约, 不算入参错)
        transport.script_synthesis(Ok(MockSynthesisResponse {
            data: vec![1],
            format: "aac".to_string(),
            sample_rate: 24_000,
            bit_depth: 16,
            channels: 1,
            duration_ms: 100,
        }));
        assert!(matches!(
            client.synthesize(&request).await,
            Err(VoiceError::Protocol(_))
        ));
    }

    #[tokio::test]
    async fn synthesize_guards_request_and_credentials() {
        let empty = TtsRequest::with_defaults(
            "   ".to_string(),
            TtsModel::default(),
            "voice-1".to_string(),
            "en".to_string(),
        );
        // TtsRequest 构造已拒空文本
        assert!(empty.is_err());

        let mut no_key = VoiceClientImpl::new();
        let request = TtsRequest::with_defaults(
            "hello".to_string(),
            TtsModel::default(),
            "voice-1".to_string(),
            "en".to_string(),
        )
        .expect("valid");
        assert!(matches!(
            no_key.synthesize(&request).await,
            Err(VoiceError::ApiKeyMissing)
        ));
    }

    #[tokio::test]
    async fn detect_wake_requires_enrolled_template() {
        let client = VoiceClientImpl::new();
        let wake = client.detect_wake(&[0i16; 320]).await;
        assert!(matches!(wake, Err(VoiceError::State(_))));
    }

    #[tokio::test]
    async fn detect_wake_scores_against_enrolled_template() {
        let client = VoiceClientImpl::new();
        // 合成发音音频: 5 段包络
        let mut audio = Vec::new();
        for amp in [0.2f32, 0.6, 1.0, 0.7, 0.3] {
            for i in 0..64 {
                let v = (amp * 20_000.0) as i16;
                audio.push(if i % 2 == 0 { v } else { -v });
            }
        }
        client.enroll_wake_template(&audio).expect("enroll");
        let det = client.detect_wake(&audio).await.expect("detect");
        assert_eq!(det.keyword, VOICE_DEFAULT_WAKE_WORD);
        assert!(
            det.confidence > 0.99,
            "same audio scores high: {}",
            det.confidence
        );
        assert!(det.is_wake(client.config().wake.sensitivity));

        // 静音 → 低分
        let det = client.detect_wake(&[0i16; 320]).await.expect("detect");
        assert!(det.confidence < 0.5);
    }

    #[tokio::test]
    async fn listening_lifecycle_and_backpressure() {
        let (client, _transport) = ready_client();
        assert!(!client.is_listening());
        // 未开始先停 → State
        assert!(matches!(
            client.stop_listening().await,
            Err(VoiceError::State(_))
        ));

        client.start_listening().await.expect("start");
        assert!(client.is_listening());
        // 二次开始 → State
        assert!(matches!(
            client.start_listening().await,
            Err(VoiceError::State(_))
        ));

        // 采集帧入队 + 背压
        {
            let capture = client.capture_session();
            let mut guard = capture.lock().expect("capture lock");
            let frame_bytes = guard.expected_frame_bytes();
            let capacity = guard.stats().capacity_frames;
            for _ in 0..capacity {
                guard.push_frame(vec![0u8; frame_bytes]).expect("frame");
            }
            let err = guard
                .push_frame(vec![0u8; frame_bytes])
                .expect_err("must backpressure");
            assert_eq!(err.category(), ErrorCategory::Backpressure);
        }

        client.stop_listening().await.expect("stop");
        assert!(!client.is_listening());
    }

    #[tokio::test]
    async fn stream_audio_folds_vad_observations() {
        let (client, _transport) = ready_client();
        let obs = |is_speech: bool, ms: u64, conf: f32| VadResult {
            is_speech,
            algorithm: VadAlgorithm::Energy,
            confidence: conf,
            speech_duration: std::time::Duration::from_millis(if is_speech { ms } else { 0 }),
            silence_duration: std::time::Duration::from_millis(if is_speech { 0 } else { ms }),
        };
        // 短促噪声被门限吃掉
        let out = client
            .stream_audio(&obs(true, 30, 0.5))
            .await
            .expect("fold");
        assert!(!out.is_speech);
        // 累计过门限 → 语音段
        client
            .stream_audio(&obs(true, 30, 0.5))
            .await
            .expect("fold");
        let out = client
            .stream_audio(&obs(true, 60, 0.5))
            .await
            .expect("fold");
        assert!(out.is_speech);
        assert_eq!(out.speech_duration, std::time::Duration::from_millis(120));
    }

    #[tokio::test]
    async fn server_error_frames_surface_closed_vocabulary() {
        let (client, transport) = ready_client();
        transport.script_synthesis(Err(VoiceError::TokenExpired));
        let request = TtsRequest::with_defaults(
            "hello".to_string(),
            TtsModel::default(),
            "voice-1".to_string(),
            "en".to_string(),
        )
        .expect("valid");
        let err = client.synthesize(&request).await.expect_err("must fail");
        assert_eq!(err.category(), ErrorCategory::Authentication);
        assert!(!err.is_retryable());
    }

    #[tokio::test]
    async fn set_config_rebuilds_capture_and_vad() {
        let (mut client, _transport) = ready_client();
        let mut config = VoiceConfig::default();
        config.audio.sample_rate = 48_000;
        config.vad.min_speech_duration_ms = 200;
        client.set_config(config).expect("set config");
        assert_eq!(client.config().audio.sample_rate, 48_000);
        let capture = client.capture_session();
        let guard = capture.lock().expect("capture lock");
        assert_eq!(guard.config().sample_rate, 48_000);

        // 非法配置 → K-1 拒绝
        let mut bad = VoiceConfig::default();
        bad.audio.sample_rate = 10;
        assert!(matches!(
            client.set_config(bad),
            Err(VoiceError::SampleRateInvalid(_))
        ));
    }

    #[test]
    fn constants_and_whitelist_stay_pinned() {
        assert_eq!(SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert!(!is_stub_mode());
        assert!(!STUB_MODE);
        assert_eq!(TOOL_WHITELIST.len(), TOOL_WHITELIST_COUNT);
        assert_eq!(VoiceClientImpl::list_apis().len(), CORE_API_COUNT);
        assert_eq!(VoiceClientImpl::list_stt_models().len(), STT_MODEL_COUNT);
        assert_eq!(VoiceClientImpl::list_tts_models().len(), TTS_MODEL_COUNT);
        assert_eq!(
            VoiceClientImpl::list_wake_word_categories().len(),
            WAKE_WORD_CATEGORY_COUNT
        );
        assert_eq!(
            VoiceClientImpl::list_vad_algorithms().len(),
            VAD_ALGORITHM_COUNT
        );
        assert_eq!(K1_STRONG_VALIDATION_COUNT, 6);
    }

    #[test]
    fn validate_tool_call_accepts_whitelist_and_rejects_unknown() {
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_voice_transcribe", &args).is_ok());
        assert!(validate_tool_call("apeireth_voice_stub_status", &args).is_ok());
        let err = validate_tool_call("apeireth_voice_bogus", &args).unwrap_err();
        assert!(matches!(err, VoiceError::ToolNotWhitelisted(_)));
    }

    #[tokio::test]
    async fn status_reports_real_state_including_transport() {
        let (client, transport) = ready_client();
        let status = client.stub_status();
        assert!(!status.stub_mode);
        assert!(status.transport_configured);
        assert!(!status.listening);
        assert_eq!(status.default_wake_word, VOICE_DEFAULT_WAKE_WORD);

        client.start_listening().await.expect("start");
        let status = client.stub_status();
        assert!(status.listening);
        let _ = transport;
    }

    #[tokio::test]
    async fn health_check_validates_config_locally() {
        let (client, _transport) = ready_client();
        assert!(client.health_check().await.is_ok());
    }
}
