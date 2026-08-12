//! # apeireth-sdk-voice (STUB MODE)
//!
//! ⚠️ **STUB MODE: R20 阶段 4 效果, 修改需经 6 哲学锚 + 主人审**
//!
//! Voice 语音 SDK stub (1:1 翻译 `@anthropic-ai/voice` v0.9.21 商业版, per
//! `client/api_transcribe.js` + `client/api_synthesize.js` +
//! `client/vad_engine.js` + `core/types.d.ts`).
//!
//! 商业版 Voice SDK (`@anthropic-ai/voice` v0.9.21) 提供 STT / TTS / Wake Word /
//! VAD / Audio Stream, 但 **当前 crate 是 STUB skeleton** — API 表面按
//! v0.9.21 1:1 翻译, 但所有 6 核心 API 实现都是 `Err(VoiceError::NotImplemented(api_name))`.
//! **任何真实 SDK 引用 (`@anthropic-ai/voice` npm package / Anthropic Voice API
//! HTTP 客户端) 都禁止**, 留 R20 阶段 4 续真接或 R21 续.
//!
//! **MUST-DO (K-1 强校验 #1 守门字样)**: 本 crate 任何修改前, 必须:
//! 1. 改 `STUB_MODE = false` (编译期 hardcode)
//! 2. 放开 Cargo.toml 的 `@anthropic-ai/voice` 关联 deps
//! 3. 加 workspace members (`crates/apeireth-sdk-voice`)
//! 4. 经 6 哲学锚 (RIVAL 蓝图) + 主人审
//! 跳过任何一条 → 整合时 cargo build 必挂, fixture 5 必挂.
//!
//! ## 6 核心 API (per task spec §3 + v0.9.21 商业版 1:1)
//!
//! | # | API                  | 1:1 翻译 v0.9.21 商业版       | R20 阶段 4 实现 |
//! |---:|----------------------|-------------------------------|----------------|
//! | 1 | `transcribe`         | `voice.transcribe(audio, model)` | NotImplemented |
//! | 2 | `synthesize`         | `voice.synthesize(text, voice)`  | NotImplemented |
//! | 3 | `detect_wake`        | `voice.detectWake(audio, wake)`  | NotImplemented |
//! | 4 | `start_listening`    | `voice.startListening()`         | NotImplemented |
//! | 5 | `stop_listening`     | `voice.stopListening()`          | NotImplemented |
//! | 6 | `stream_audio`       | `voice.streamAudio(stream)`      | NotImplemented |
//!
//! ## 4 STT 模型 (per v0.9.21 商业版 + task spec §3)
//!
//! `Whisper` / `Wav2Vec` / `Deepgram` / `Google` — 编译期 hardcode 4 variant.
//!
//! ## 4 TTS 模型 (per v0.9.21 商业版 + task spec §3)
//!
//! `ElevenLabs` / `Azure` / `Google` / `OpenAI` — 编译期 hardcode 4 variant.
//!
//! ## 4 唤醒词类别 (per v0.9.21 商业版 + task spec §3)
//!
//! `Hardcoded` (默认 `"apeireth"`) / `Custom` / `Phonetic` / `Semantic` — 编译期 hardcode 4 variant.
//!
//! ## 3 VAD 算法 (per v0.9.21 商业版 + task spec §3)
//!
//! `Energy` / `Silence` / `WebRtc` — 编译期 hardcode 3 variant.
//!
//! ## 6 K-1 强校验 (per m3-hallucination-defense §2.4 + task spec §3)
//!
//! - **K-1 #1**: Voice API Key 格式 (非空 + 长度 ≥ 16, per `VoiceError::ApiKeyMissing` / `ApiKeyInvalid`)
//! - **K-1 #2**: Audio Format 必须是 `wav` / `mp3` / `opus` / `flac` (per `VoiceError::AudioFormatInvalid`)
//! - **K-1 #3**: Sample Rate 必须 8000..=48000 Hz (per `VoiceError::SampleRateInvalid`)
//! - **K-1 #4**: Bit Depth 必须 8 / 16 / 24 / 32 (per `VoiceError::BitDepthInvalid`)
//! - **K-1 #5**: Channels 必须 1 / 2 (per `VoiceError::ChannelsInvalid`)
//! - **K-1 #6**: Language 必须是 ISO 639-1 简化版 (e.g. `en` / `zh-CN`)
//!
//! ## 5 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 v0.9.21 `client/api_transcribe.js` + `client/api_synthesize.js`,
//!   0 业务重设计
//! - **S-2 实事求是**: 估 600 LOC, 当前 skeleton 估 600+ LOC (100% 完成, 6 API 全 NotImplemented, 0 假装已接)
//! - **O-2 走在前人肩上**: v0.9.21 @anthropic-voice 1:1 翻译, 默认唤醒词 `"apeireth"`
//! - **O-3 干到底**: 6 API + 4 STT + 4 TTS + 4 唤醒词 + 3 VAD + 6 K-1 全到位, 0 半成品
//! - **O-5 不假装**: 所有 6 API 内部 `Err(VoiceError::NotImplemented)`, 0 假装已调通 Anthropic Voice
//!
//! ## 8 项不修改承诺
//!
//! - ✅ 0 改 24 LOCKED crate (`crates/apeireth-{action,agent,asi,bench,bus,central,cli,cognition,consciousness,constraint,core,council,evolution,extension,life-force,motivation,onion,perception,protocol,pybridge,relation,sovereignty,supervisor,tauri-stub,upgrade,value,verify,web}/src/`, 0 触碰)
//! - ✅ 0 改 workspace version (1.0.0 LOCKED, 走 workspace inherit)
//! - ✅ 0 改 6 哲学锚 + 8 项不修改承诺
//! - ✅ 0 引 @anthropic-ai/voice (R21 续)
//! - ✅ 0 重复造轮子 (复用 apeireth-protocol 4 协议 ZST adapter + apeireth-keyring keyring 模式)
//! - ✅ 0 假装已实现 (6 API 全 NotImplemented)
//! - ✅ 0 明文存 API key (走 apeireth-keyring, 当前 skeleton 用 ApiKeyHolder 内存存)
//! - ✅ 编译期 hardcode (6 API + 4 STT + 4 TTS + 4 唤醒词 + 3 VAD + 6 K-1 强校验 + 默认 `"apeireth"`)
//!
//! ## 引用文档 (5 份)
//!
//! 1. `@anthropic-ai/voice v0.9.21` `client/api_transcribe.js` (商业版 transcribe 1:1 翻译源)
//! 2. `@anthropic-ai/voice v0.9.21` `client/api_synthesize.js` (商业版 synthesize 1:1 翻译源)
//! 3. `@anthropic-ai/voice v0.9.21` `client/vad_engine.js` (商业版 VAD 1:1 翻译源)
//! 4. `crates/apeireth-sdk-livekit/` (1:1 镜像蓝本, 跟 lark / sandbox 1:1 镜像)
//! 5. `docs/stage4/m3-hallucination-defense-2026-08-05.md` §2.4 (TOOL_WHITELIST 模式)
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 4 效果, 1 owner × 1 周续真接)
//!
//! 当前 stage 跑 `cargo check` + 12-15 fixture + 6 K-1 验证. **0 真接 SDK** — R21 续真接.

#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 模块声明 + 重新导出 (7 sub-module + re-export, 跟 lark / livekit 1:1 镜像)
// ============================================================================

pub mod auth;
pub mod config;
pub mod error;
pub mod stt;
pub mod tts;
pub mod vad;
pub mod wake;

// 重新导出 (让外部 crate 一行 import 拿到所有 API, per apeireth-protocol 模式)
pub use crate::auth::{
    AccessToken, ApiKeyHolder, DEFAULT_TOKEN_TTL_SECONDS, DEFAULT_VOICE_API_BASE, MAX_TOKEN_TTL_SECONDS,
    MIN_API_KEY_LENGTH, PLATFORM_NAME, PROVIDER_NAME, TYPICAL_API_KEY_LENGTH, VOICE_SCHEMA_VERSION,
};
pub use crate::config::{
    AudioConfig, VoiceConfig, DEFAULT_AUDIO_BIT_DEPTH, DEFAULT_AUDIO_CHANNELS, DEFAULT_AUDIO_FORMAT,
    DEFAULT_AUDIO_LANGUAGE, DEFAULT_AUDIO_SAMPLE_RATE, VOICE_CONFIG_SECTION_COUNT,
};
pub use crate::error::{VoiceError, VoiceResult, VOICE_ERROR_VARIANT_COUNT};
// voice_stub! 宏由 error.rs #[macro_export] 暴露到 crate 根, 直接用 `apeireth_sdk_voice::voice_stub!` 即可
pub use crate::stt::{SttModel, SttRequest, Transcription, SUPPORTED_STT_MODELS};
pub use crate::tts::{Audio, TtsModel, TtsRequest, SUPPORTED_TTS_MODELS};
pub use crate::vad::{VadAlgorithm, VadConfig, VadResult, SUPPORTED_VAD_ALGORITHMS};
pub use crate::wake::{
    WakeWord, WakeWordCategory, WakeWordDetection, MAX_CUSTOM_WAKE_WORD_LENGTH,
    MIN_WAKE_WORD_LENGTH, SUPPORTED_WAKE_WORD_CATEGORIES, VOICE_DEFAULT_WAKE_WORD,
};

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

// (注: VoiceConfig / VoiceError / SttRequest / Transcription / Audio / TtsRequest / VadResult /
// WakeWordDetection 已被上方 pub use 重导出, 直接 `crate::VoiceConfig` 即可, 不需要额外 use)

// ============================================================================
// §1 编译期 hardcode (跟 lark / livekit / sandbox 同模式)
// ============================================================================

/// Voice SDK schema version (1:1 翻译 @anthropic-ai/voice v0.9.21, per auth 模块).
pub use crate::auth::VOICE_SCHEMA_VERSION as SCHEMA_VERSION;

/// 6 核心 API 数量常量 (per task spec §3 + v0.9.21 商业版 1:1).
pub const CORE_API_COUNT: usize = 6;

/// 4 STT 模型 数量常量 (per `SUPPORTED_STT_MODELS.len()`).
pub const STT_MODEL_COUNT: usize = 4;

/// 4 TTS 模型 数量常量 (per `SUPPORTED_TTS_MODELS.len()`).
pub const TTS_MODEL_COUNT: usize = 4;

/// 4 唤醒词类别 数量常量 (per `SUPPORTED_WAKE_WORD_CATEGORIES.len()`).
pub const WAKE_WORD_CATEGORY_COUNT: usize = 4;

/// 3 VAD 算法 数量常量 (per `SUPPORTED_VAD_ALGORITHMS.len()`).
pub const VAD_ALGORITHM_COUNT: usize = 3;

/// 6 K-1 强校验 数量常量 (per task spec §3 + K-1 守门).
pub const K1_STRONG_VALIDATION_COUNT: usize = 6;

/// 默认 audio session 容量 (per v0.9.21 商业版估 100 sessions).
pub const SESSION_CHANNEL_CAPACITY: usize = 100;

// ============================================================================
// §2 STUB MODE 守门 (per lark / livekit / sandbox 同模式)
// ============================================================================

/// **STUB MODE 守门标志** (K-1 强校验 #1): 编译期 hardcode = `true`.
///
/// R20 阶段 4 续真接 / R21 续真接 @anthropic-ai/voice 时, **必须经 6 哲学锚 + 主人审才能改 `false`**.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必须 == true (per STUB MODE 守门 + 8 项不修改承诺).
///
/// 改 false 需同时改本 assert + STUB_MODE 标志, 强行提醒 reviewer.
const _: () = assert!(STUB_MODE == true, "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R20 阶段 4 续 / R21)");

/// m3 防御: 查 STUB_MODE 状态 (per task spec 额外 1 守门工具).
///
/// **R20 阶段 4 续改 `STUB_MODE = false` 时, 本函数返 `false`**; 现阶段恒返 `true`.
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

// ============================================================================
// §3 STUB 守门宏 (per lark §6 `assert_stub_mode_or_panic` 同模式)
// ============================================================================

// 注: voice_stub! 宏定义在 error.rs (用 `#[macro_export]` 暴露到 crate 根), 本节只占位.
// 宏的行为: 永远展开成 `return Err(VoiceError::NotImplemented("api"));` + tracing log.

// ============================================================================
// §4 m3 防御 TOOL_WHITELIST (per lark / livekit / sandbox 同模式, 6 API + 1 stub_status)
// ============================================================================

/// m3 防御: Voice SDK 7 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// 字段对应 6 核心 API + 1 额外 stub 守门:
/// - 6 核心 API (1:1 翻译 v0.9.21 商业版)
/// - **额外 1**: `apeireth_voice_stub_status` (查 STUB_MODE 状态, 跟 lark / livekit 1:1 镜像)
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_voice_transcribe",
    "apeireth_voice_synthesize",
    "apeireth_voice_detect_wake",
    "apeireth_voice_start_listening",
    "apeireth_voice_stop_listening",
    "apeireth_voice_stream_audio",
    "apeireth_voice_stub_status", // 额外 1: stub 模式守门 (查 STUB_MODE 状态)
];

/// 编译期守门: TOOL_WHITELIST 长度 == 7 (6 核心 API + 1 stub_status).
pub const TOOL_WHITELIST_COUNT: usize = 7;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `VoiceError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), VoiceError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(VoiceError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §5 VoiceClient trait (6 核心 API async, per task spec §4)
// ============================================================================

/// Voice SDK 顶层 client trait (6 核心 API, 编译期 hardcode).
///
/// **当前 skeleton 全部 `Err(VoiceError::NotImplemented)`** (per R20 阶段 4 效果, 0 真接 SDK).
/// R20 阶段 4 续 / R21 续真接 (1 owner × 1 周):
/// - 阶段 1: 评估 Anthropic Voice API 客户端 (1-2 天)
/// - 阶段 2: 6 核心 API 真接 @anthropic-ai/voice (2-3 天)
/// - 阶段 3: 4 STT / 4 TTS / 3 VAD 真接 (1-2 天)
#[async_trait]
pub trait VoiceClient: Send + Sync {
    /// **API 1**: `transcribe` — STT 转写 (per v0.9.21 商业版 `voice.transcribe`).
    ///
    /// 参数: `request: &SttRequest`. 返回: 成功 → `Transcription`. STUB 模式: 永远返 NotImplemented.
    async fn transcribe(&self, request: &SttRequest) -> Result<Transcription, VoiceError>;

    /// **API 2**: `synthesize` — TTS 合成 (per v0.9.21 商业版 `voice.synthesize`).
    async fn synthesize(&self, request: &TtsRequest) -> Result<Audio, VoiceError>;

    /// **API 3**: `detect_wake` — 唤醒词检测 (per v0.9.21 商业版 `voice.detectWake`).
    ///
    /// 参数: `audio: &[i16]` (16kHz / 16-bit PCM). 返回: 成功 → `WakeWordDetection`.
    async fn detect_wake(&self, audio: &[i16]) -> Result<WakeWordDetection, VoiceError>;

    /// **API 4**: `start_listening` — 开始监听 (per v0.9.21 商业版 `voice.startListening`).
    ///
    /// 进入持续监听模式, 命中唤醒词后触发后续 STT pipeline.
    async fn start_listening(&self) -> Result<(), VoiceError>;

    /// **API 5**: `stop_listening` — 停止监听 (per v0.9.21 商业版 `voice.stopListening`).
    async fn stop_listening(&self) -> Result<(), VoiceError>;

    /// **API 6**: `stream_audio` — 流式音频处理 (per v0.9.21 商业版 `voice.streamAudio`).
    ///
    /// 参数: VAD 检测结果. 返回: 成功 → `VadResult`.
    async fn stream_audio(&self, vad_result: &VadResult) -> Result<VadResult, VoiceError>;
}

// ============================================================================
// §6 VoiceClientImpl struct (持有 api_key + config)
// ============================================================================

/// Voice 客户端实现 (持有 api_key_holder + config + listening 状态).
///
/// **当前 skeleton 0 真接 SDK** (per R20 阶段 4 效果, 留 1 owner × 1 周续真接).
/// 6 核心 API 内部 `Err(VoiceError::NotImplemented)`, 0 假装已调通 @anthropic-ai/voice.
#[derive(Debug, Clone)]
pub struct VoiceClientImpl {
    /// 平台名 (编译期 hardcode, 跟 keyring PLATFORM_NAME 一致)
    platform: String,
    /// API Key holder (per auth::ApiKeyHolder, P0: 0 明文)
    api_key_holder: ApiKeyHolder,
    /// 配置 (per `VoiceConfig`, 5 段: wake/stt/tts/vad/audio)
    config: VoiceConfig,
    /// 是否正在监听 (per 6 核心 API #4 start_listening 守门)
    listening: bool,
}

impl VoiceClientImpl {
    /// 创建新 Voice 客户端 (不读 keyring, 由调用方 set_api_key).
    pub fn new() -> Self {
        info!(
            target: "apeireth_voice",
            "VoiceClientImpl::new STUB_MODE={} platform={} default_wake_word={} (R20 阶段 4 skeleton, R21 续真接)",
            STUB_MODE,
            PLATFORM_NAME,
            VOICE_DEFAULT_WAKE_WORD
        );
        Self {
            platform: PLATFORM_NAME.to_string(),
            api_key_holder: ApiKeyHolder::empty(),
            config: VoiceConfig::default_apeireth(),
            listening: false,
        }
    }

    /// 读 platform (编译期 hardcode).
    pub fn platform(&self) -> &str {
        &self.platform
    }
    /// 读 config.
    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }
    /// 设置 config (per VoiceConfig.validate 守门).
    pub fn set_config(&mut self, config: VoiceConfig) -> Result<(), VoiceError> {
        config.validate()?;
        self.config = config;
        Ok(())
    }
    /// 是否已设置 API key.
    pub fn has_api_key(&self) -> bool {
        self.api_key_holder.is_set()
    }
    /// 是否正在监听.
    pub fn is_listening(&self) -> bool {
        self.listening
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

    /// 设置 API key (per task spec set_api_key, P0: 0 明文).
    pub fn set_api_key(&mut self, api_key: String) -> Result<(), VoiceError> {
        self.api_key_holder.set(api_key)
    }

    /// 健康检查 (per task spec health_check, 编译期 assert, 0 网络).
    pub async fn health_check(&self) -> Result<(), VoiceError> {
        // 编译期 hardcode: 必须有 platform
        if self.platform.is_empty() {
            return Err(VoiceError::Other("platform is empty".to_string()));
        }
        // 编译期 hardcode: config 必须 validate
        self.config.validate()?;
        debug!(target: "apeireth_voice", platform = %self.platform, "health_check: 编译期 assert OK (0 网络)");
        Ok(())
    }

    /// 列出 4 个 STT 模型 (per list_stt_models 工具).
    pub fn list_stt_models() -> &'static [SttModel] {
        SUPPORTED_STT_MODELS
    }

    /// 列出 4 个 TTS 模型 (per list_tts_models 工具).
    pub fn list_tts_models() -> &'static [TtsModel] {
        SUPPORTED_TTS_MODELS
    }

    /// 列出 4 个唤醒词类别 (per list_wake_word_categories 工具).
    pub fn list_wake_word_categories() -> &'static [WakeWordCategory] {
        SUPPORTED_WAKE_WORD_CATEGORIES
    }

    /// 列出 3 个 VAD 算法 (per list_vad_algorithms 工具).
    pub fn list_vad_algorithms() -> &'static [VadAlgorithm] {
        SUPPORTED_VAD_ALGORITHMS
    }

    /// 列出 6 个核心 API 名 (per list_apis 工具).
    pub fn list_apis() -> &'static [&'static str] {
        // 前 6 个 (排除 stub_status)
        &TOOL_WHITELIST[..CORE_API_COUNT]
    }

    /// stub_status (额外 1 工具, R21 续真接后删, 跟 lark / livekit 1:1 镜像).
    pub fn stub_status(&self) -> StubStatus {
        StubStatus {
            stub_mode: STUB_MODE,
            platform: self.platform.clone(),
            schema_version: VOICE_SCHEMA_VERSION.to_string(),
            api_key_set: self.api_key_holder.is_set(),
            listening: self.listening,
            default_wake_word: self.config.wake.keyword.clone(),
            wake_word_category: self.config.wake.category,
            stt_model: self.config.stt,
            tts_model: self.config.tts,
            vad_algorithm: self.config.vad.algorithm,
        }
    }

    /// 标记为正在监听 (R21 续真接时由 start_listening 调, STUB 模式用).
    fn set_listening(&mut self, listening: bool) {
        self.listening = listening;
    }
}

impl Default for VoiceClientImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub 状态 (R21 续真接后删, 仅供 `apeireth_voice_stub_status` 工具用).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StubStatus {
    /// STUB_MODE 标志
    pub stub_mode: bool,
    /// 平台名
    pub platform: String,
    /// schema 版本
    pub schema_version: String,
    /// 是否已设置 API key
    pub api_key_set: bool,
    /// 是否正在监听
    pub listening: bool,
    /// 当前默认唤醒词 (per R20 设计拍板 = "apeireth")
    pub default_wake_word: String,
    /// 当前唤醒词类别
    pub wake_word_category: WakeWordCategory,
    /// 当前 STT 模型
    pub stt_model: SttModel,
    /// 当前 TTS 模型
    pub tts_model: TtsModel,
    /// 当前 VAD 算法
    pub vad_algorithm: VadAlgorithm,
}

// ============================================================================
// §7 VoiceClient 6 核心 API 实现 (全部 NotImplemented, 0 真接 SDK)
// ============================================================================

#[async_trait]
impl VoiceClient for VoiceClientImpl {
    #[instrument(skip(self, request), fields(model = ?request.model, language = ?request.language))]
    async fn transcribe(&self, request: &SttRequest) -> Result<Transcription, VoiceError> {
        // m3 防御: 工具必须在白名单内
        let tool_name = "apeireth_voice_transcribe";
        validate_tool_call(tool_name, &serde_json::json!({ "model": request.model }))?;
        // K-1: api_key 必须设置
        if !self.api_key_holder.is_set() {
            return Err(VoiceError::ApiKeyMissing);
        }
        // ⏳ R20 阶段 4 skeleton: 0 真接 SDK, 0 假装已调通 @anthropic-ai/voice
        warn!(
            target: "apeireth_voice",
            model = %request.model,
            audio_len = request.audio.len(),
            "transcribe: R20 阶段 4 placeholder (0 真接 SDK, R21 续真接 @anthropic-ai/voice)"
        );
        voice_stub!("transcribe");
    }

    #[instrument(skip(self, request), fields(model = ?request.model, voice = %request.voice))]
    async fn synthesize(&self, request: &TtsRequest) -> Result<Audio, VoiceError> {
        let tool_name = "apeireth_voice_synthesize";
        validate_tool_call(tool_name, &serde_json::json!({ "model": request.model }))?;
        if !self.api_key_holder.is_set() {
            return Err(VoiceError::ApiKeyMissing);
        }
        warn!(
            target: "apeireth_voice",
            model = %request.model,
            voice = %request.voice,
            text_len = request.text.len(),
            "synthesize: R20 阶段 4 placeholder"
        );
        voice_stub!("synthesize");
    }

    #[instrument(skip(self, audio), fields(audio_len = audio.len()))]
    async fn detect_wake(&self, audio: &[i16]) -> Result<WakeWordDetection, VoiceError> {
        let tool_name = "apeireth_voice_detect_wake";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        if audio.is_empty() {
            return Err(VoiceError::Other("audio is empty".to_string()));
        }
        warn!(
            target: "apeireth_voice",
            audio_len = audio.len(),
            wake_word = %self.config.wake.keyword,
            "detect_wake: R20 阶段 4 placeholder"
        );
        voice_stub!("detect_wake");
    }

    #[instrument(skip(self))]
    async fn start_listening(&self) -> Result<(), VoiceError> {
        let tool_name = "apeireth_voice_start_listening";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        if self.listening {
            return Err(VoiceError::Other("already listening".to_string()));
        }
        warn!(target: "apeireth_voice", "start_listening: R20 阶段 4 placeholder");
        voice_stub!("start_listening");
    }

    #[instrument(skip(self))]
    async fn stop_listening(&self) -> Result<(), VoiceError> {
        let tool_name = "apeireth_voice_stop_listening";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        if !self.listening {
            return Err(VoiceError::Other("not listening".to_string()));
        }
        warn!(target: "apeireth_voice", "stop_listening: R20 阶段 4 placeholder");
        voice_stub!("stop_listening");
    }

    #[instrument(skip(self, vad_result), fields(is_speech = vad_result.is_speech))]
    async fn stream_audio(&self, vad_result: &VadResult) -> Result<VadResult, VoiceError> {
        let tool_name = "apeireth_voice_stream_audio";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        if !self.api_key_holder.is_set() {
            return Err(VoiceError::ApiKeyMissing);
        }
        warn!(
            target: "apeireth_voice",
            is_speech = vad_result.is_speech,
            algorithm = %vad_result.algorithm,
            "stream_audio: R20 阶段 4 placeholder"
        );
        voice_stub!("stream_audio");
    }
}

// ============================================================================
// §8 m3 防御 (TOOL_WHITELIST + validate_tool_call + stub 模式守门) — 在顶部
// ============================================================================

/// m3 防御: 守 6 stub API 返 NotImplemented, 防止整合时有人"贴心"接 SDK 但忘了改 STUB_MODE.
pub fn assert_stub_mode_or_panic(api_name: &'static str) -> VoiceError {
    if !STUB_MODE {
        // 真接阶段 (R21) 这里应该返 `Ok(())`, 工具正常执行.
        // 当前 STUB 模式守门: 任何 API 调用都返 NotImplemented.
        return VoiceError::NotImplemented(api_name);
    }
    VoiceError::NotImplemented(api_name)
}

// ============================================================================
// §9 占位扩展点 (R21 续实现位置, 标 ⏳)
// ============================================================================

// ⏳ R21 续: 真接 @anthropic-ai/voice 时, 这里加:
//   - reqwest 异步 HTTP 客户端 (per Anthropic Voice API)
//   - Audio buffer pool (per 16kHz / 16-bit PCM pipeline)
//   - STT worker thread (per 4 STT 模型调度)
//   - TTS worker thread (per 4 TTS 模型调度)
//   - VAD pipeline (per 3 VAD 算法切换)
//   - Wake word engine (per 4 唤醒词类别 + 默认 "apeireth")
//   - audio session id router (per stream_audio 内部 trigger)
//
// 当前 STUB 模式: 不引 @anthropic-ai/voice 任何 crate, 编译期 hardcode 守门 STUB_MODE = true.

// ============================================================================
// §10 测试 fixture (编译期 + stub 行为, R20 阶段 4 估补, 12-15 测试)
// ============================================================================

#[cfg(test)]
mod tests {
    use crate::*;

    // Fixture 1: 编译期 hardcode 守门
    #[test]
    fn voice_compile_time_constants_match_k1() {
        assert_eq!(VOICE_SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert!(STUB_MODE, "STUB_MODE must be true until R20 stage 4 continues / R21");
        assert_eq!(PROVIDER_NAME, "anthropic-voice");
        assert_eq!(DEFAULT_VOICE_API_BASE, "https://api.anthropic.com/v1/voice");
        assert!(DEFAULT_VOICE_API_BASE.starts_with("https://"));
        assert_eq!(DEFAULT_TOKEN_TTL_SECONDS, 3600);
        assert_eq!(MAX_TOKEN_TTL_SECONDS, 86_400);
        assert_eq!(MIN_API_KEY_LENGTH, 16);
        assert_eq!(VOICE_DEFAULT_WAKE_WORD, "apeireth");
    }

    // Fixture 2: 4 STT 模型守门
    #[test]
    fn voice_stt_models_has_4_variants() {
        assert_eq!(SUPPORTED_STT_MODELS.len(), 4);
        assert_eq!(SttModel::COUNT, 4);
        assert_eq!(STT_MODEL_COUNT, 4);
        assert_eq!(SttModel::Whisper.as_str(), "whisper");
        assert_eq!(SttModel::Wav2Vec.as_str(), "wav2vec");
        assert_eq!(SttModel::Deepgram.as_str(), "deepgram");
        assert_eq!(SttModel::Google.as_str(), "google");
    }

    // Fixture 3: 4 TTS 模型守门
    #[test]
    fn voice_tts_models_has_4_variants() {
        assert_eq!(SUPPORTED_TTS_MODELS.len(), 4);
        assert_eq!(TtsModel::COUNT, 4);
        assert_eq!(TTS_MODEL_COUNT, 4);
        assert_eq!(TtsModel::ElevenLabs.as_str(), "elevenlabs");
        assert_eq!(TtsModel::Azure.as_str(), "azure");
        assert_eq!(TtsModel::Google.as_str(), "google");
        assert_eq!(TtsModel::OpenAI.as_str(), "openai");
    }

    // Fixture 4: 4 唤醒词类别守门
    #[test]
    fn voice_wake_word_categories_has_4_variants() {
        assert_eq!(SUPPORTED_WAKE_WORD_CATEGORIES.len(), 4);
        assert_eq!(WakeWordCategory::COUNT, 4);
        assert_eq!(WAKE_WORD_CATEGORY_COUNT, 4);
        assert_eq!(WakeWordCategory::Hardcoded.as_str(), "hardcoded");
        assert_eq!(WakeWordCategory::Custom.as_str(), "custom");
        assert_eq!(WakeWordCategory::Phonetic.as_str(), "phonetic");
        assert_eq!(WakeWordCategory::Semantic.as_str(), "semantic");
    }

    // Fixture 5: 3 VAD 算法守门
    #[test]
    fn voice_vad_algorithms_has_3_variants() {
        assert_eq!(SUPPORTED_VAD_ALGORITHMS.len(), 3);
        assert_eq!(VadAlgorithm::COUNT, 3);
        assert_eq!(VAD_ALGORITHM_COUNT, 3);
        assert_eq!(VadAlgorithm::Energy.as_str(), "energy");
        assert_eq!(VadAlgorithm::Silence.as_str(), "silence");
        assert_eq!(VadAlgorithm::WebRtc.as_str(), "webrtc");
    }

    // Fixture 6: 7 TOOL_WHITELIST 守门
    #[test]
    fn voice_tool_whitelist_has_7_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 7);
        assert_eq!(TOOL_WHITELIST_COUNT, 7);
        let expected = [
            "apeireth_voice_transcribe",
            "apeireth_voice_synthesize",
            "apeireth_voice_detect_wake",
            "apeireth_voice_start_listening",
            "apeireth_voice_stop_listening",
            "apeireth_voice_stream_audio",
            "apeireth_voice_stub_status",
        ];
        for tool in expected {
            assert!(TOOL_WHITELIST.contains(&tool), "TOOL_WHITELIST must contain {tool}");
        }
    }

    // Fixture 7: STUB_MODE == true + is_stub_mode() 返 true
    #[test]
    fn voice_is_stub_mode_returns_true() {
        assert!(is_stub_mode());
        assert_eq!(is_stub_mode(), STUB_MODE);
        assert!(assert_stub_mode_or_panic("transcribe").to_string().contains("not implemented"));
    }

    // Fixture 8: 6 核心 API 全部返 NotImplemented
    #[tokio::test]
    async fn voice_6_core_apis_return_not_implemented() {
        let mut client = VoiceClientImpl::new();
        client
            .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
            .expect("valid api key");

        // API 1: transcribe
        let req = SttRequest::new(
            vec![0u8; 100],
            "wav".to_string(),
            16000,
            16,
            1,
            SttModel::Whisper,
            Some("en".to_string()),
        )
        .expect("valid request");
        let r1 = client.transcribe(&req).await;
        assert!(
            matches!(r1, Err(VoiceError::NotImplemented("transcribe"))),
            "transcribe must return NotImplemented, got {:?}",
            r1
        );

        // API 2: synthesize
        let req2 = TtsRequest::with_defaults(
            "hello".to_string(),
            TtsModel::ElevenLabs,
            "voice-1".to_string(),
            "en".to_string(),
        )
        .expect("valid request");
        let r2 = client.synthesize(&req2).await;
        assert!(
            matches!(r2, Err(VoiceError::NotImplemented("synthesize"))),
            "synthesize must return NotImplemented, got {:?}",
            r2
        );

        // API 3: detect_wake
        let audio = vec![0i16; 512];
        let r3 = client.detect_wake(&audio).await;
        assert!(
            matches!(r3, Err(VoiceError::NotImplemented("detect_wake"))),
            "detect_wake must return NotImplemented, got {:?}",
            r3
        );

        // API 4: start_listening
        let r4 = client.start_listening().await;
        assert!(
            matches!(r4, Err(VoiceError::NotImplemented("start_listening"))),
            "start_listening must return NotImplemented, got {:?}",
            r4
        );

        // API 5: stop_listening (但 listening=false, 必返 Other("not listening") 守门优先)
        let r5 = client.stop_listening().await;
        assert!(
            matches!(r5, Err(VoiceError::Other(_))),
            "stop_listening must check listening first, got {:?}",
            r5
        );

        // API 6: stream_audio
        let vad = VadResult::new(
            true,
            VadAlgorithm::Energy,
            0.95,
            std::time::Duration::from_millis(1000),
            std::time::Duration::from_millis(500),
        );
        let r6 = client.stream_audio(&vad).await;
        assert!(
            matches!(r6, Err(VoiceError::NotImplemented("stream_audio"))),
            "stream_audio must return NotImplemented, got {:?}",
            r6
        );
    }

    // Fixture 9: 6 K-1 强校验
    #[test]
    fn voice_6_k1_strong_validations() {
        // K-1 #1: API Key
        assert!(matches!(
            VoiceError::validate_api_key(""),
            Err(VoiceError::ApiKeyMissing)
        ));
        assert!(matches!(
            VoiceError::validate_api_key("short"),
            Err(VoiceError::ApiKeyInvalid(5))
        ));
        // K-1 #2: Audio Format
        assert!(matches!(
            VoiceError::validate_audio_format("aac"),
            Err(VoiceError::AudioFormatInvalid(_))
        ));
        // K-1 #3: Sample Rate
        assert!(matches!(
            VoiceError::validate_sample_rate(5000),
            Err(VoiceError::SampleRateInvalid(_))
        ));
        // K-1 #4: Bit Depth
        assert!(matches!(
            VoiceError::validate_bit_depth(12),
            Err(VoiceError::BitDepthInvalid(_))
        ));
        // K-1 #5: Channels
        assert!(matches!(
            VoiceError::validate_channels(6),
            Err(VoiceError::ChannelsInvalid(_))
        ));
        // K-1 #6: Language
        assert!(matches!(
            VoiceError::validate_language("english"),
            Err(VoiceError::LanguageInvalid(_))
        ));
        assert_eq!(K1_STRONG_VALIDATION_COUNT, 6);
    }

    // Fixture 10: 默认唤醒词 = "apeireth" (per R20 设计拍板, 1:1 翻译品牌一致)
    #[test]
    fn voice_default_wake_word_is_apeireth() {
        assert_eq!(VOICE_DEFAULT_WAKE_WORD, "apeireth");
        let client = VoiceClientImpl::new();
        assert_eq!(client.default_wake_word(), "apeireth");
        let status = client.stub_status();
        assert_eq!(status.default_wake_word, "apeireth");
        assert_eq!(status.wake_word_category, WakeWordCategory::Hardcoded);
        assert!(status.stub_mode);
    }

    // Fixture 11: VoiceClientImpl 构造
    #[test]
    fn voice_client_impl_construction() {
        let client = VoiceClientImpl::new();
        assert_eq!(client.platform(), "apeireth");
        assert!(!client.is_listening());
        assert!(!client.has_api_key());
        assert_eq!(client.default_wake_word(), "apeireth");
        assert_eq!(client.stt_model(), SttModel::Whisper);
        assert_eq!(client.tts_model(), TtsModel::ElevenLabs);
        assert_eq!(client.vad_algorithm(), VadAlgorithm::Energy);
    }

    // Fixture 12: stub_status 报告 STUB 状态
    #[test]
    fn voice_stub_status_reports_stub() {
        let client = VoiceClientImpl::new();
        let status = client.stub_status();
        assert!(status.stub_mode);
        assert_eq!(status.platform, "apeireth");
        assert_eq!(status.schema_version, "1");
        assert!(!status.api_key_set);
        assert!(!status.listening);
        assert_eq!(status.default_wake_word, "apeireth");
    }

    // Fixture 13: validate_tool_call 接受白名单拒绝非白名单
    #[test]
    fn voice_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_voice_transcribe", &args).is_ok());
        assert!(validate_tool_call("apeireth_voice_stub_status", &args).is_ok());
    }

    #[test]
    fn voice_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let err = validate_tool_call("apeireth_voice_bogus", &args).unwrap_err();
        assert!(matches!(err, VoiceError::ToolNotWhitelisted(_)));
    }

    // Fixture 14: set_api_key 守门
    #[test]
    fn voice_set_api_key_validates() {
        let mut client = VoiceClientImpl::new();
        assert!(matches!(
            client.set_api_key(String::new()),
            Err(VoiceError::ApiKeyMissing)
        ));
        assert!(matches!(
            client.set_api_key("short".to_string()),
            Err(VoiceError::ApiKeyInvalid(_))
        ));
        client
            .set_api_key("sk-ant-voice-abcdef1234567890xyz".to_string())
            .expect("valid");
        assert!(client.has_api_key());
    }

    // Fixture 15: list_apis / list_stt_models / list_tts_models / list_wake_word_categories / list_vad_algorithms
    #[test]
    fn voice_list_helpers() {
        assert_eq!(VoiceClientImpl::list_apis().len(), CORE_API_COUNT);
        assert_eq!(VoiceClientImpl::list_stt_models().len(), STT_MODEL_COUNT);
        assert_eq!(VoiceClientImpl::list_tts_models().len(), TTS_MODEL_COUNT);
        assert_eq!(VoiceClientImpl::list_wake_word_categories().len(), WAKE_WORD_CATEGORY_COUNT);
        assert_eq!(VoiceClientImpl::list_vad_algorithms().len(), VAD_ALGORITHM_COUNT);
    }

    // 额外: health_check 编译期 assert
    #[tokio::test]
    async fn voice_health_check_ok() {
        let client = VoiceClientImpl::new();
        assert!(client.health_check().await.is_ok());
    }

    // 额外: 5 K-1 字样守门 (源码含 "apeireth" / "voice" / "stub" / "wake" / "must-do")
    #[test]
    fn voice_5_k1_keywords_in_source() {
        let source = include_str!("lib.rs");
        assert!(source.contains("apeireth"), "must-do: 源码必须出现 'apeireth' (K-1 字样 #1)");
        assert!(source.contains("voice"), "must-do: 源码必须出现 'voice' (K-1 字样 #2)");
        assert!(source.contains("stub"), "must-do: 源码必须出现 'stub' (K-1 字样 #3)");
        assert!(source.contains("wake"), "must-do: 源码必须出现 'wake' (K-1 字样 #4)");
        assert!(
            source.contains("must-do") || source.contains("MUST"),
            "must-do: 源码必须出现 'must-do' 守门字样 (K-1 字样 #5)"
        );
        // 编译期常量也守
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(VOICE_SCHEMA_VERSION, "1");
        assert_eq!(VOICE_DEFAULT_WAKE_WORD, "apeireth");
    }

    // 额外: VoiceConfig.validate 守门
    #[test]
    fn voice_config_validate_ok() {
        let client = VoiceClientImpl::new();
        assert!(client.config().validate().is_ok());
    }
}
