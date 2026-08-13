//! # apeireth-voice
//!
//! Apeireth voice subsystem — wake word detection, audio capture, TTS, STT,
//! voiceprint, and the OpenAI Realtime API protocol schema.
//!
//! ## Three-layer architecture (R153 unified)
//!
//! | Layer | Module | Purpose |
//! |-------|--------|---------|
//! | STUB facade | `lib.rs` (this file) | Porcupine + pvrecorder-style API surface. Compile-time `STUB_MODE = true` guard returns `NotImplemented` for all 8 tools. Designed for downstream wiring once picovoice SDK is added. |
//! | Real HTTP client | `real.rs` | `VoiceRealImpl` — TTS / STT / wake-word / voiceprint over `reqwest` HTTP. Wiremock-tested. |
//! | Realtime protocol | `realtime.rs` (R153) | OpenAI Realtime API schema — 3-model dispatch (gpt-realtime / gpt-realtime-mini / gpt-4o-realtime), 128K context, ephemeral tokens, server VAD, function calling, multimodal image input. 0 引 external dep. |
//!
//! ## Borrowed upstream references (per O-5)
//!
//! - **Porcupine** (`@picovoice/porcupine`): offline on-device wake word detection.
//!   API surface in `lib.rs` mirrors Porcupine's keyword model.
//! - **pvrecorder** (`@picovoice/pvrecorder`): cross-platform audio stream (16kHz,
//!   16-bit PCM, 512 frames). API surface in `lib.rs` mirrors pvrecorder's
//!   frame model.
//! - **OpenAI Realtime API** (GA 2024-12, protocol v1): event schemas, session
//!   lifecycle, ephemeral tokens. Implementation in `realtime.rs`.
//!
//! Default wake word: **"apeireth"** (compile-time hardcode, brand-consistent).
//!
//! ## Modification guard (K-1)
//!
//! MUST-DO before any structural change to this crate's STUB facade:
//! 1. Audit STUB_MODE invariant (compile-time `STUB_MODE = true`).
//! 2. Confirm 9-tool whitelist (`TOOL_WHITELIST`) integrity.
//! 3. Confirm 5 WakeWordType enum exhaustiveness.
//! 4. Confirm 0-touch on the 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache).
//!
//! ## STUB mode guard
//!
//! `STUB_MODE = true` is a compile-time hardcode that makes the 8 Porcupine/
//! pvrecorder facade tools return `Err(VoiceError::NotImplemented)`. This is
//! the intentional design — the real implementation path is `VoiceRealImpl`
//! (HTTP) or `apeireth_voice::realtime::*` (OpenAI Realtime protocol). To
//! wire actual Porcupine / pvrecorder SDK calls, set `STUB_MODE = false` and
//! add `porcupine` + `pvrecorder` to `Cargo.toml`. This is gated by the
//! 3 immutable spines (Self-Disable / L0 HA / 13-key verdict cache).
//!
//! ## Tool whitelist (m3 hallucination defense)
//!
//! 9 compile-time hardcoded tool names prevent LLM models (notably `m3`) from
//! hallucinating tool calls. `validate_tool_call` rejects any tool not in
//! `TOOL_WHITELIST`.

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};
use uuid::Uuid;

// ============================================================================
// m3 hallucination 防御 #3 (per m3-hallucination-defense-2026-08-05.md §2.4 + §2.1)
// WHITELIST 编译期 hardcode 9 工具 (8 商业版 + 1 stub_status), validate_tool_call
// 在 dispatch 前 schema 校验. 防止 minimax m3 模型幻觉调用不存在的工具.
// ============================================================================

/// m3 hallucination defense: 9-tool whitelist, compile-time hardcode.
/// 8 Porcupine/pvrecorder facade tools + 1 stub_status (queried by callers
/// to check STUB_MODE state; kept for diagnostic visibility).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_voice_wake_word_detect",
    "apeireth_voice_record_audio",
    "apeireth_voice_transcribe",
    "apeireth_voice_synthesize",
    "apeireth_voice_list_keywords",
    "apeireth_voice_load_model",
    "apeireth_voice_unload_model",
    "apeireth_voice_audio_stream",
    "apeireth_voice_stub_status", // 额外 1: 查 STUB_MODE 状态
];

/// Compile-time guard: TOOL_WHITELIST length must be exactly 9.
pub const TOOL_WHITELIST_COUNT: usize = 9;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `VoiceError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), VoiceError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(VoiceError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §0.5 R20 阶段 6 flesh out 新增: `real` 模块 (真接 TTS / STT / 唤醒词 / 声纹 4 块)
// ============================================================================
//
// VoiceRealImpl is the explicit opt-in HTTP client for the 4-block voice API:
// TTS / STT / wake-word / voiceprint. It is fully decoupled from the STUB_MODE
// guard in §3 (Porcupine + pvrecorder facade). Call sites invoke
// `apeireth_voice::VoiceRealImpl::new(config, base_url, api_key)?` directly.
//
// Design:
// - TTS / STT / voiceprint run over real reqwest HTTP (POST multipart / JSON)
// - Wake-word remains STUB (default "apeireth" hardcode, Porcupine engine
//   deferred until porcupine/pvrecorder SDK integration)
// - Honest limitation list at the top of real.rs (per O-5)
//
// Invariants:
// - 0 touches STUB_MODE (still = true), 0 touches VoiceSdk 9 tools
// - 0 touches 24 LOCKED crate entry signatures (formally revoked R128; only
//   the 3 immutable spines remain — Self-Disable / L0 HA / 13-key verdict cache)
// - 0 touches workspace version (1.2.0) or existing 23 fixture + 23 tests
pub mod real;
pub mod minimax_live;

// ============================================================================
// §0.6 R153 新增: `realtime` 模块 (OpenAI Realtime API 协议支持)
// ============================================================================
//
// 跟本文件 §3 VoiceSdk 9 工具 (STUB 模式) + §0.5 real 4 块 (HTTP 真接) **并列**:
// `apeireth_voice::realtime` 是 OpenAI Realtime API 协议 schema + lifecycle + dispatch
// 表面, 不依赖 picovoice SDK, 不走 STUB 守门, 0 引外部 dep (reqwest + tokio 已齐).
//
// 设计:
// - 3-model dispatch: gpt-realtime / gpt-realtime-mini / gpt-4o-realtime
// - ephemeral token (POST /v1/realtime/sessions)
// - WebSocket event protocol: 10 server events + 8 client events + 4 conversation items
// - function calling / image input / server VAD / audio append
// - 128K context window + 1h session TTL (编译期 hardcode)
//
// 守门:
// - 0 改 STUB_MODE (仍 = true), 0 改 VoiceSdk 9 工具, 0 改 VoiceRealImpl 4 块
// - 0 改 24 LOCKED crate, 0 改 workspace version (1.2.0)
// - 0 触碰 3 不可变脊柱: Self-Disable / L0 HA / 13 键 verdict cache
pub mod realtime;

// 便捷 re-exports (调用方少打 crate 名)
// §0.6 R153: 加 realtime 顶层 re-export 块 (10 type alias + 4 常量)
pub use realtime::{
    EphemeralToken, EphemeralTokenRequest, RealtimeAudioFormat, RealtimeError,
    RealtimeModalities, RealtimeModality, RealtimeModel, RealtimeResult,
    RealtimeSessionConfig, RealtimeTool, RealtimeVoice, ServerEvent, ClientEvent,
    ConversationItem, TurnDetection, TurnDetectionKind,
    REALTIME_CONTEXT_WINDOW_TOKENS, REALTIME_DEFAULT_SESSION_TTL,
    REALTIME_DEFAULT_TOKEN_ENDPOINT, REALTIME_MAX_AUDIO_BUFFER_BYTES,
    REALTIME_MAX_IMAGE_BYTES, REALTIME_MAX_SESSION_TTL, REALTIME_MODEL_COUNT,
    REALTIME_SCHEMA_VERSION, SUPPORTED_REALTIME_MODELS, SUPPORTED_REALTIME_VOICES,
    encode_audio_append, encode_image_input,
};

// 便捷 re-exports (调用方少打 crate 名)
// §0.5 旧块 (real 模块) (调用方少打 crate 名)
pub use real::{
    AudioBuffer, AudioFormat, Lang, SUPPORTED_LANGS, SUPPORTED_VOICE_KINDS, VoiceApiResponse,
    VoiceKind, VoiceRealImpl, VoiceprintMatch, VoiceprintMatchResponse, VOICE_API_BASE_URL,
    VOICE_API_KEY_CACHE_TTL_SECONDS, VOICE_API_KEY_ENV, WakeWord,
};

// ============================================================================
// §1 文档头 + 编译期 hardcode (per R20 5 P0 crate 风格 + K-1 强校验)
// ============================================================================

/// voice SDK schema version (v0.9.21 商业版 1:1, 暂不写 version.workspace).
/// K-1 强校验 #1: 编译期 hardcode, 不写 `"1"` 字符串 elsewhere.
pub const VOICE_SCHEMA_VERSION: &str = "1";

/// 平台名 (K-1 强校验 #2: 编译期 hardcode `"apeireth"`, v0.9.21 1:1 翻译, 不写 "SpectrAI" 等装饰名).
pub const PLATFORM_NAME: &str = "apeireth";

/// **STUB 模式守门**: 编译期 hardcode `true`. R20 阶段 3 整合 #2 sub-agent
/// 改 `false` + 真接 picovoice (porcupine + pvrecorder) SDK.
/// 8 工具全部返 `Err(VoiceError::NotImplemented(api_name))`.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必为 true (R20 阶段 3 改 false 时同步改本断言).
/// K-1 强校验 #4: 守 stub 模式不漏防.
const _: () = assert!(STUB_MODE == true, "STUB_MODE must be true until R20 stage 3");

/// 查 STUB_MODE 状态 (m3 防御: 多 1 工具 `apeireth_voice_stub_status`).
/// R20 阶段 3 真接 picovoice 后, 整个 `STUB_MODE` 块 + `stub_status` 工具删.
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 默认采样率 (16kHz, per Porcupine 官方 + v0.9.21 商业版 pvrecorder).
pub const VOICE_SAMPLE_RATE_HZ: u32 = 16000;

/// 单帧长度 (512 frames, per Porcupine 官方 + v0.9.21 商业版 pvrecorder).
pub const VOICE_FRAME_LENGTH: u32 = 512;

/// 默认唤醒词 (K-1 强校验 #2 品牌一致: 编译期 hardcode `"apeireth"`, 不写 HeySpectrAI).
pub const VOICE_DEFAULT_KEYWORD: &str = "apeireth";

/// 单次录音最大秒数 (per v0.9.21 商业版估 30s 限制, 防恶意录音占内存).
pub const VOICE_MAX_AUDIO_SECONDS: u32 = 30;

// ============================================================================
// §2 核心类型 (VoiceConfig / WakeWordType / AudioFrame / VoiceError)
// ============================================================================

/// 唤醒词类型 (5 个枚举, K-1 强校验 #3: 编译期 hardcode, 不可运行时增删).
///
/// 字段对应 v0.9.21 商业版 Porcupine `keywords` 数组元素. 默认 = `Apeireth`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WakeWordType {
    /// **默认**: `"apeireth"` (品牌一致, K-1 强校验守门).
    #[default]
    Apeireth,
    /// `"computer"` (经典 sci-fi 致敬, v0.9.21 商业版估 1:1).
    Computer,
    /// `"jarvis"` (经典 sci-fi 致敬, v0.9.21 商业版估 1:1).
    Jarvis,
    /// `"hey apeireth"` (长句, v0.9.21 商业版估 1:1).
    HeyApeireth,
    /// 自定义唤醒词 (per R20 阶段 3 估补, 需用户上传 keyword 文件).
    Custom,
}

impl WakeWordType {
    /// 唤醒词字符串 (跟 v0.9.21 商业版 Porcupine 关键词库 1:1).
    pub fn as_str(&self) -> &'static str {
        match self {
            WakeWordType::Apeireth => "apeireth",
            WakeWordType::Computer => "computer",
            WakeWordType::Jarvis => "jarvis",
            WakeWordType::HeyApeireth => "hey apeireth",
            WakeWordType::Custom => "custom",
        }
    }
}

/// 编译期守门: SUPPORTED_WAKE_WORDS 5 项 (K-1 强校验 #3).
pub const SUPPORTED_WAKE_WORDS: &[WakeWordType] = &[
    WakeWordType::Apeireth,
    WakeWordType::Computer,
    WakeWordType::Jarvis,
    WakeWordType::HeyApeireth,
    WakeWordType::Custom,
];
const _: () = assert!(SUPPORTED_WAKE_WORDS.len() == 5);

/// Picovoice access_key (从 env `PICOVOICE_ACCESS_KEY` 读, R20 阶段 3 真接 SDK 时用).
/// STUB 模式不读, 但字段保留 1:1 翻译 v0.9.21 商业版 `accessKey` 配置.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    /// Picovoice access_key (env `PICOVOICE_ACCESS_KEY`, R20 阶段 3 真接).
    #[serde(default)]
    pub access_key: String,
    /// 关键词文件路径 (.ppn, per Porcupine 官方, R20 阶段 3 真接).
    #[serde(default = "default_keyword_path")]
    pub keyword_path: PathBuf,
    /// 模型文件路径 (.pv, per Porcupine 官方, R20 阶段 3 真接).
    #[serde(default = "default_model_path")]
    pub model_path: PathBuf,
    /// 默认唤醒词 (编译期 hardcode `Apeireth`).
    #[serde(default)]
    pub default_keyword: WakeWordType,
    /// 灵敏度 (0.0~1.0, per Porcupine 官方, 默认 0.5).
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,
    /// 单 session 最大录音秒数 (per `VOICE_MAX_AUDIO_SECONDS`).
    #[serde(default = "default_max_audio_seconds")]
    pub max_audio_seconds: u32,
}

fn default_keyword_path() -> PathBuf {
    PathBuf::from("./resources/porcupine/apeireth.ppn")
}
fn default_model_path() -> PathBuf {
    PathBuf::from("./resources/porcupine/porcupine_model.pv")
}
fn default_sensitivity() -> f32 {
    0.5
}
fn default_max_audio_seconds() -> u32 {
    VOICE_MAX_AUDIO_SECONDS
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            access_key: std::env::var("PICOVOICE_ACCESS_KEY").unwrap_or_default(),
            keyword_path: default_keyword_path(),
            model_path: default_model_path(),
            default_keyword: WakeWordType::Apeireth,
            sensitivity: default_sensitivity(),
            max_audio_seconds: default_max_audio_seconds(),
        }
    }
}

/// 音频帧 (单帧 = 512 samples, per Porcupine 官方).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioFrame {
    /// 16kHz / 16-bit PCM (per Porcupine 官方 + v0.9.21 商业版 pvrecorder).
    pub samples: Vec<i16>,
    /// 帧长度 (编译期 hardcode `VOICE_FRAME_LENGTH = 512`).
    pub frame_length: u32,
    /// 采样率 (编译期 hardcode `VOICE_SAMPLE_RATE_HZ = 16000`).
    pub sample_rate: u32,
    /// 帧时间戳 (SystemTime).
    pub timestamp: SystemTime,
}

impl AudioFrame {
    /// 创建新帧 (16kHz / 16-bit PCM, 512 frames 必填).
    pub fn new(samples: Vec<i16>) -> Self {
        Self {
            samples,
            frame_length: VOICE_FRAME_LENGTH,
            sample_rate: VOICE_SAMPLE_RATE_HZ,
            timestamp: SystemTime::now(),
        }
    }
}

/// 唤醒词检测结果.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeWordDetection {
    /// 命中的唤醒词 (e.g. `"apeireth"`).
    pub keyword: String,
    /// 唤醒词类型枚举.
    pub wake_word_type: WakeWordType,
    /// 检测时间戳.
    pub detected_at: SystemTime,
    /// 触发录音 session ID (R20 阶段 3 录音 pipeline 用).
    pub session_id: Option<Uuid>,
}

/// 录音 session 元信息.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSession {
    /// session ID (UUID v4).
    pub id: Uuid,
    /// 唤醒词 (触发该 session 的唤醒词).
    pub triggered_by: String,
    /// 录音开始时间.
    pub started_at: SystemTime,
    /// 录音最大时长 (per `VoiceConfig.max_audio_seconds`).
    pub max_duration: Duration,
    /// 录音状态 (4 状态机, R20 阶段 3 估补).
    pub status: RecordingStatus,
}

/// 录音状态 (4 状态机, R20 阶段 3 真接 pvrecorder 时用).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordingStatus {
    /// 唤醒词刚触发, 还没开始录.
    Pending,
    /// 正在录音 (pvrecorder 写 buffer).
    Recording,
    /// 用户主动停止 / 静音超时.
    Stopped,
    /// 失败 (pvrecorder 错误 / 超时).
    Failed,
}

/// Voice SDK 错误 (15 variant, per mcp-ssh 13 variant / plugin 12 variant 类比;
///
/// **R20 阶段 6 flesh out 扩展 5 variant** (Network/AuthFailed/ApiError/AudioTooLong/AudioEmpty)
/// 跟 lark `LarkError` 10 variant 1:1 模式, 用于 `real` 模块真接 4 块 HTTP 端点.
#[derive(Debug, Error)]
pub enum VoiceError {
    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),
    /// **STUB 模式**: API 未实现, R20 阶段 3 真接 picovoice SDK 后删.
    /// 8 工具全部返本 variant.
    #[error("STUB MODE: API not implemented: {0} (R20 stage 3 will wire picovoice SDK)")]
    NotImplemented(&'static str),
    /// 唤醒词未命中 (per v0.9.21 Porcupine 行为).
    #[error("no wake word detected")]
    NoWakeWord,
    /// 唤醒词模型加载失败 (R20 阶段 3 真接 Porcupine 时用).
    #[error("wake word model load failed: {0}")]
    ModelLoadFailed(String),
    /// 录音失败 (R20 阶段 3 真接 pvrecorder 时用).
    #[error("recording failed: {0}")]
    RecordingFailed(String),
    /// STT/TTS 服务调用失败 (R20 阶段 4 估补).
    #[error("STT/TTS service failed: {0}")]
    ServiceCallFailed(String),
    /// 配置无效 (e.g. access_key 空).
    #[error("invalid voice config: {0}")]
    InvalidConfig(String),
    /// 录音超时.
    #[error("voice session timeout ({0:?})")]
    Timeout(Duration),
    /// 音频格式不支持.
    #[error("unsupported audio format: {0}")]
    UnsupportedFormat(String),
    /// I/O 错误.
    #[error("voice I/O error: {0}")]
    Io(#[from] std::io::Error),

    // === R20 阶段 6 flesh out 新增 5 variant (per lark 1:1 模式, 后向兼容) ===
    /// 网络错误 (HTTP 请求 / 响应失败, per `reqwest::Error`).
    #[error("voice network error: {0}")]
    Network(String),
    /// 鉴权失败 (api_key 缺失 / 401 / 403, per 远端 voice API).
    #[error("voice auth failed: {0}")]
    AuthFailed(String),
    /// 远端 voice API 返回非零 code (per 远端 `code != 0`).
    #[error("voice api error: code={code}, msg={msg}")]
    ApiError { code: i32, msg: String },
    /// audio buffer 超过最大长度 (per `VOICE_MAX_AUDIO_SECONDS` 守门).
    #[error("voice audio too long: got {got}ms, max {max}ms")]
    AudioTooLong { got: u64, max: u64 },
    /// audio buffer 为空 (per STT / 声纹 输入校验).
    #[error("voice audio buffer is empty")]
    AudioEmpty,
}

pub type VoiceResult<T> = Result<T, VoiceError>;

// ============================================================================
// §3 Stub API 表面 (8 工具, 每个返 VoiceError::NotImplemented)
// ============================================================================

/// 唤醒词检测器 (Porcupine wrapper, R20 阶段 3 真接 SDK).
///
/// 字段对应 v0.9.21 商业版 Porcupine 初始化 (估 5 fields):
/// - `config` (per `VoiceConfig`)
/// - `keyword` (per `WakeWordType::Apeireth` 默认)
/// - `running` (per `AtomicBool`)
/// - `detection_count` (per `u64`)
/// - `model_loaded` (per `AtomicBool`)
#[derive(Debug)]
pub struct VoiceWake {
    config: VoiceConfig,
    keyword: WakeWordType,
    running: AtomicBool,
    detection_count: u64,
    model_loaded: AtomicBool,
}

impl VoiceWake {
    /// 创建新的唤醒词检测器 (STUB 模式不真接 Porcupine).
    pub fn new(config: VoiceConfig, keyword: WakeWordType) -> VoiceResult<Self> {
        if STUB_MODE {
            info!(
                target: "apeireth_voice",
                "VoiceWake::new STUB_MODE=true keyword={:?} (R20 stage 3 will wire porcupine)",
                keyword
            );
        } else {
            return Err(VoiceError::NotImplemented("VoiceWake::new"));
        }
        Ok(Self {
            config,
            keyword,
            running: AtomicBool::new(false),
            detection_count: 0,
            model_loaded: AtomicBool::new(false),
        })
    }

    /// 启动唤醒词检测 (STUB 返 NotImplemented, R20 阶段 3 真接 Porcupine.process()).
    pub async fn start(&mut self) -> VoiceResult<()> {
        if STUB_MODE {
            warn!(target: "apeireth_voice", "VoiceWake::start STUB_MODE returning NotImplemented");
            return Err(VoiceError::NotImplemented("apeireth_voice_wake_word_detect"));
        }
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// 停止唤醒词检测.
    pub async fn stop(&mut self) -> VoiceResult<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// 当前唤醒词 (per v0.9.21 商业版 `keyword` 字段).
    pub fn keyword(&self) -> WakeWordType {
        self.keyword
    }

    /// 总检测次数.
    pub fn detection_count(&self) -> u64 {
        self.detection_count
    }
}

/// 录音器 (pvrecorder wrapper, R20 阶段 3 真接 SDK).
///
/// 字段对应 v0.9.21 商业版 pvrecorder (估 4 fields):
/// - `config` (per `VoiceConfig`)
/// - `session` (per `Option<RecordingSession>`)
/// - `running` (per `AtomicBool`)
/// - `buffer` (per `Vec<AudioFrame>`)
#[derive(Debug)]
pub struct VoiceRecorder {
    config: VoiceConfig,
    session: Option<RecordingSession>,
    running: AtomicBool,
    buffer: Vec<AudioFrame>,
}

impl VoiceRecorder {
    /// 创建新的录音器 (STUB 模式不真接 pvrecorder).
    pub fn new(config: VoiceConfig) -> VoiceResult<Self> {
        if STUB_MODE {
            info!(target: "apeireth_voice", "VoiceRecorder::new STUB_MODE=true (R20 stage 3 will wire pvrecorder)");
        } else {
            return Err(VoiceError::NotImplemented("VoiceRecorder::new"));
        }
        Ok(Self {
            config,
            session: None,
            running: AtomicBool::new(false),
            buffer: Vec::new(),
        })
    }

    /// 启动录音 (STUB 返 NotImplemented, R20 阶段 3 真接 pvrecorder.start()).
    pub async fn start(&mut self, triggered_by: &str) -> VoiceResult<Uuid> {
        if STUB_MODE {
            warn!(target: "apeireth_voice", "VoiceRecorder::start STUB_MODE returning NotImplemented");
            return Err(VoiceError::NotImplemented("apeireth_voice_record_audio"));
        }
        let id = Uuid::new_v4();
        self.session = Some(RecordingSession {
            id,
            triggered_by: triggered_by.to_string(),
            started_at: SystemTime::now(),
            max_duration: Duration::from_secs(u64::from(self.config.max_audio_seconds)),
            status: RecordingStatus::Pending,
        });
        self.running.store(true, Ordering::SeqCst);
        Ok(id)
    }

    /// 停止录音.
    pub async fn stop(&mut self) -> VoiceResult<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// 当前 session ID.
    pub fn session_id(&self) -> Option<Uuid> {
        self.session.as_ref().map(|s| s.id)
    }

    /// 缓冲区帧数.
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

/// Voice SDK 顶层 facade (8 工具 dispatcher, STUB 模式返 NotImplemented).
///
/// 字段对应 v0.9.21 商业版 `voice` 模块 (估 3 fields):
/// - `config` (per `VoiceConfig`)
/// - `wake` (per `Option<VoiceWake>`)
/// - `recorder` (per `Option<VoiceRecorder>`)
#[derive(Debug)]
pub struct VoiceSdk {
    config: VoiceConfig,
    wake: Option<VoiceWake>,
    recorder: Option<VoiceRecorder>,
    sessions: HashMap<Uuid, RecordingSession>,
}

impl VoiceSdk {
    /// 创建新的 Voice SDK (STUB 模式 OK, R20 阶段 3 真接 picovoice).
    pub fn new(config: VoiceConfig) -> VoiceResult<Self> {
        info!(target: "apeireth_voice", "VoiceSdk::new STUB_MODE={} platform={} default_keyword={}", STUB_MODE, PLATFORM_NAME, VOICE_DEFAULT_KEYWORD);
        Ok(Self {
            config,
            wake: None,
            recorder: None,
            sessions: HashMap::new(),
        })
    }

    /// 工具 1: `apeireth_voice_wake_word_detect` (STUB 返 NotImplemented).
    pub async fn wake_word_detect(&mut self, _audio: &AudioFrame) -> VoiceResult<WakeWordDetection> {
        warn!(target: "apeireth_voice", "wake_word_detect STUB_MODE returning NotImplemented");
        Err(VoiceError::NotImplemented("apeireth_voice_wake_word_detect"))
    }

    /// 工具 2: `apeireth_voice_record_audio` (STUB 返 NotImplemented).
    pub async fn record_audio(&mut self, _duration_secs: u32) -> VoiceResult<Vec<i16>> {
        warn!(target: "apeireth_voice", "record_audio STUB_MODE returning NotImplemented");
        Err(VoiceError::NotImplemented("apeireth_voice_record_audio"))
    }

    /// 工具 3: `apeireth_voice_transcribe` STT (STUB 返 NotImplemented).
    pub async fn transcribe(&self, _audio: &[i16]) -> VoiceResult<String> {
        warn!(target: "apeireth_voice", "transcribe STUB_MODE returning NotImplemented");
        Err(VoiceError::NotImplemented("apeireth_voice_transcribe"))
    }

    /// 工具 4: `apeireth_voice_synthesize` TTS (STUB 返 NotImplemented).
    pub async fn synthesize(&self, _text: &str) -> VoiceResult<Vec<u8>> {
        warn!(target: "apeireth_voice", "synthesize STUB_MODE returning NotImplemented");
        Err(VoiceError::NotImplemented("apeireth_voice_synthesize"))
    }

    /// 工具 5: `apeireth_voice_list_keywords` (STUB 返 SUPPORTED_WAKE_WORDS 列表 — 不是 NotImplemented,
    /// 因为 5 唤醒词是编译期常量, 不依赖 picovoice SDK, 可直接返).
    pub fn list_keywords(&self) -> VoiceResult<Vec<WakeWordType>> {
        info!(target: "apeireth_voice", "list_keywords returning SUPPORTED_WAKE_WORDS (compile-time constant)");
        Ok(SUPPORTED_WAKE_WORDS.to_vec())
    }

    /// 工具 6: `apeireth_voice_load_model` (STUB 返 NotImplemented).
    pub async fn load_model(&mut self, _keyword: WakeWordType) -> VoiceResult<()> {
        warn!(target: "apeireth_voice", "load_model STUB_MODE returning NotImplemented");
        Err(VoiceError::NotImplemented("apeireth_voice_load_model"))
    }

    /// 工具 7: `apeireth_voice_unload_model` (STUB 返 NotImplemented).
    pub async fn unload_model(&mut self) -> VoiceResult<()> {
        warn!(target: "apeireth_voice", "unload_model STUB_MODE returning NotImplemented");
        Err(VoiceError::NotImplemented("apeireth_voice_unload_model"))
    }

    /// 工具 8: `apeireth_voice_audio_stream` (STUB 返 NotImplemented).
    pub async fn audio_stream(&self) -> VoiceResult<tokio::sync::mpsc::Receiver<AudioFrame>> {
        warn!(target: "apeireth_voice", "audio_stream STUB_MODE returning NotImplemented");
        Err(VoiceError::NotImplemented("apeireth_voice_audio_stream"))
    }

    /// 工具 9 (额外 1): `apeireth_voice_stub_status` (R20 阶段 3 后删, 查 STUB_MODE).
    pub fn stub_status(&self) -> VoiceResult<StubStatus> {
        Ok(StubStatus {
            stub_mode: STUB_MODE,
            platform: PLATFORM_NAME.to_string(),
            default_keyword: VOICE_DEFAULT_KEYWORD.to_string(),
            schema_version: VOICE_SCHEMA_VERSION.to_string(),
        })
    }

    /// 当前 config.
    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    /// 当前激活 session 数.
    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }
}

/// Stub 状态 (R20 阶段 3 后删, 仅供 `apeireth_voice_stub_status` 工具用).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubStatus {
    pub stub_mode: bool,
    pub platform: String,
    pub default_keyword: String,
    pub schema_version: String,
}

// ============================================================================
// §4 配置 + audio stream 占位
// ============================================================================

/// Audio stream 占位 (R20 阶段 3 真接 pvrecorder + R21 Tauri 估用).
///
/// STUB 模式: trait 表面 1:1 翻译 v0.9.21 商业版, 但实现由调用方注入.
/// R20 阶段 3 整合时换 pvrecorder 的 `read()` 异步流.
#[async_trait]
pub trait AudioStreamSource: Send + Sync {
    /// 读下一帧 (STUB 模式由 mock 实现返 dummy frame, R20 阶段 3 接 pvrecorder).
    async fn next_frame(&mut self) -> VoiceResult<AudioFrame>;
    /// 关流.
    async fn close(&mut self) -> VoiceResult<()>;
}

/// STUB 模式默认 audio stream (返空帧, R20 阶段 3 删).
#[derive(Debug, Default)]
pub struct StubAudioStream {
    closed: AtomicBool,
}

impl StubAudioStream {
    pub fn new() -> Self {
        Self {
            closed: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl AudioStreamSource for StubAudioStream {
    async fn next_frame(&mut self) -> VoiceResult<AudioFrame> {
        if STUB_MODE {
            return Err(VoiceError::NotImplemented("apeireth_voice_audio_stream"));
        }
        Ok(AudioFrame::new(vec![0i16; VOICE_FRAME_LENGTH as usize]))
    }

    async fn close(&mut self) -> VoiceResult<()> {
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }
}

// ============================================================================
// §5 占位扩展点 (R20 阶段 3 实现位置, 标 ⏳)
// ============================================================================

// ⏳ R20 阶段 3 续: 真接 picovoice (porcupine + pvrecorder) 时, 这里加:
//   - Porcupine builder wrapper (per `porcupine::PorcupineBuilder`)
//   - pvrecorder builder wrapper (per `pvrecorder::PvRecorderBuilder`)
//   - audio thread pool (per v0.9.21 商业版 4 worker thread)
//   - keyword model cache (per v0.9.21 商业版 5 唤醒词预加载)
//   - STT/TTS service 集成 (R20 阶段 4 估补, 走 apeireth-bus event)
//
// 当前 STUB 模式: 不引 picovoice 任何 crate, 编译期 hardcode 守门 STUB_MODE = true.

// ============================================================================
// §6 m3 防御 (TOOL_WHITELIST + validate_tool_call + stub 模式守门) — 在顶部
// ============================================================================

// 8 stub 工具 + 1 stub_status 9 工具白名单已在顶部 m3 防御块定义.
// 本节专门承载 stub 模式额外守门:

/// m3 防御: 守 8 stub 工具返 NotImplemented, 防止整合时有人"贴心"接 picovoice 但忘了改 STUB_MODE.
pub fn assert_stub_mode_or_panic(tool: &'static str) -> VoiceResult<()> {
    if !STUB_MODE {
        // 真接阶段 (R20 阶段 3 后) 这里应该返 `Ok(())`, 工具正常执行.
        // 当前 STUB 模式守门: 任何工具调用都返 NotImplemented.
        return Err(VoiceError::NotImplemented(tool));
    }
    Err(VoiceError::NotImplemented(tool))
}

// ============================================================================
// §7 测试 fixture (编译期 + stub 行为, R20 阶段 1 Fixture 5 + 2 额外)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture 1: 编译期 hardcode 守门
    #[test]
    fn voice_compile_time_constants_match_k1() {
        assert_eq!(VOICE_SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert!(STUB_MODE, "STUB_MODE must be true until R20 stage 3");
        assert_eq!(VOICE_SAMPLE_RATE_HZ, 16000);
        assert_eq!(VOICE_FRAME_LENGTH, 512);
        assert_eq!(VOICE_DEFAULT_KEYWORD, "apeireth");
        assert_eq!(VOICE_MAX_AUDIO_SECONDS, 30);
    }

    // Fixture 2: 5 WakeWordType 枚举守门
    #[test]
    fn voice_wake_word_type_has_5_variants() {
        assert_eq!(SUPPORTED_WAKE_WORDS.len(), 5);
        assert_eq!(WakeWordType::Apeireth.as_str(), "apeireth");
        assert_eq!(WakeWordType::Computer.as_str(), "computer");
        assert_eq!(WakeWordType::Jarvis.as_str(), "jarvis");
        assert_eq!(WakeWordType::HeyApeireth.as_str(), "hey apeireth");
        assert_eq!(WakeWordType::Custom.as_str(), "custom");
    }

    // Fixture 3: TOOL_WHITELIST 9 工具守门
    #[test]
    fn voice_tool_whitelist_has_9_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 9);
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_wake_word_detect"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_record_audio"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_transcribe"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_synthesize"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_list_keywords"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_load_model"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_unload_model"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_audio_stream"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_voice_stub_status"));
    }

    // Fixture 4: validate_tool_call 接受白名单, 拒绝非白名单
    #[test]
    fn voice_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_voice_wake_word_detect", &args).is_ok());
        assert!(validate_tool_call("apeireth_voice_stub_status", &args).is_ok());
    }

    #[test]
    fn voice_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let err = validate_tool_call("apeireth_voice_bogus_tool", &args).unwrap_err();
        assert!(matches!(err, VoiceError::ToolNotWhitelisted(_)));
    }

    // Fixture 5: is_stub_mode 返 true (K-1 强校验 #4 守门)
    #[test]
    fn voice_is_stub_mode_returns_true() {
        assert!(is_stub_mode());
        assert_eq!(is_stub_mode(), STUB_MODE);
    }

    // 额外 1: 8 stub 工具返 NotImplemented (体现 stub 模式)
    #[tokio::test]
    async fn voice_8_stub_tools_return_not_implemented() {
        let mut sdk = VoiceSdk::new(VoiceConfig::default()).unwrap();
        let frame = AudioFrame::new(vec![0i16; 512]);

        // 8 stub 工具必须全部返 VoiceError::NotImplemented
        assert!(matches!(sdk.wake_word_detect(&frame).await, Err(VoiceError::NotImplemented(_))));
        assert!(matches!(sdk.record_audio(5).await, Err(VoiceError::NotImplemented(_))));
        assert!(matches!(sdk.transcribe(&[0i16; 512]).await, Err(VoiceError::NotImplemented(_))));
        assert!(matches!(sdk.synthesize("hello").await, Err(VoiceError::NotImplemented(_))));
        // list_keywords 不返 NotImplemented (是编译期常量)
        assert!(sdk.list_keywords().is_ok());
        assert!(matches!(sdk.load_model(WakeWordType::Apeireth).await, Err(VoiceError::NotImplemented(_))));
        assert!(matches!(sdk.unload_model().await, Err(VoiceError::NotImplemented(_))));
        assert!(matches!(sdk.audio_stream().await, Err(VoiceError::NotImplemented(_))));
    }

    // 额外 2: 默认唤醒词 = "apeireth" (1:1 翻译品牌一致)
    #[test]
    fn voice_default_keyword_is_apeireth() {
        assert_eq!(VOICE_DEFAULT_KEYWORD, "apeireth");
        assert_eq!(VoiceConfig::default().default_keyword, WakeWordType::Apeireth);
        let sdk = VoiceSdk::new(VoiceConfig::default()).unwrap();
        let status = sdk.stub_status().unwrap();
        assert_eq!(status.default_keyword, "apeireth");
        assert_eq!(status.platform, "apeireth");
        assert!(status.stub_mode);
    }
}
