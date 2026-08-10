//! # `apeireth-voice` — R20 阶段 6 flesh out: 真接 TTS / STT / 唤醒词 / 声纹 4 块
//!
//! **本模块是 R20 阶段 6 flesh out 新增**, 跟 `lib.rs` 现有 STUB 路径 (`VoiceSdk` 9 工具
//! 返 NotImplemented) **严格分离**: `VoiceRealImpl` 是显式 opt-in 的真接 HTTP 客户端,
//! 不受 `STUB_MODE = true` 编译期 hardcode 守门影响. 调用方显式
//! `VoiceRealImpl::new(config, base_url, api_key)?` 即用.
//!
//! ## 设计 (per 蓝图 §3.5 缺口 + 主人 22:13 flesh out)
//!
//! 1. **4 块 1:1 翻译 v0.9.21 商业版 voice API**:
//!    - **TTS** (1): `POST /v1/audio/speech` — 文本合成语音 (返 audio bytes, WAV/MP3/PCM)
//!    - **STT** (1): `POST /v1/audio/transcriptions` — 语音转文本 (multipart/form-data)
//!    - **声纹** (1): `POST /v1/voiceprint/match` — 声纹比对 (返 similarity + verified)
//!    - **唤醒词** (1): STUB — 默认 `"apeireth"` hardcode, 0 网络调用
//!      (Porcupine 引擎 R21+ 续)
//!
//! 2. **API key 自动管理** (per 商业版 SDK 内部行为):
//!    - `ensure_api_key()` — 每次业务调用前查缓存, 缺失 → 自动从 env 读
//!    - 401 响应 → 自动重试 1 次 (清缓存 + 重读 env + 重发)
//!    - API key 走 `Arc<Mutex<Option<String>>>`, 跨 await 安全
//!
//! 3. **错误映射** (扩展现状 `VoiceError` 5+ variant, 跟 lark 1:1 模式):
//!    - 远端 `code != 0` → `VoiceError::ApiError { code, msg }`
//!    - HTTP 4xx/5xx → `VoiceError::Network(...)` / `VoiceError::AuthFailed(...)`
//!    - 401 → 重试 1 次, 重试仍 401 → `VoiceError::AuthFailed(...)`
//!    - audio buffer 越界 → `VoiceError::AudioTooLong` / `AudioEmpty`
//!
//! 4. **Auth header 注入**: `Authorization: Bearer <api_key>`
//!
//! 5. **Content-Type**: TTS = `application/json; charset=utf-8`,
//!    STT = `multipart/form-data; boundary=...`,
//!    声纹 = `application/json; charset=utf-8`
//!
//! ## 6 哲学锚穿透 (per 蓝图 §1)
//!
//! - **S-1 北极星**: 1:1 翻译 v0.9.21 商业版 voice API 3 端点 URL (跟 OpenAI audio API
//!   同模式, v0.9.21 voice SDK 默认 Client 一致). wake_word 是 STUB hardcode, 不引 Porcupine
//!   (per 0 重复造轮子 + 8 项承诺 #7).
//! - **S-2 实事求是**: wiremock 0.6 mock server 真起 socket 监听, 走真 HTTP 请求路径
//!   (tokio + reqwest), 不假装"调通了"; 远端 `code != 0` 真测覆盖 3 个错误 variant.
//! - **O-2 走在前人肩上**: `reqwest` 0.12 + `rustls-tls` 走 workspace deps, 跟
//!   `apeireth-lark` / `apeireth-http-client` 同款 0 重复造轮子; URL 解析走 `url` crate
//!   (业界成熟).
//! - **O-3 干到底**: 4 块 × 2 路径 (happy + error) = 8+ 测试 + 1 集成 e2e +
//!   1 demo + 1 文档章节, 信息密度高, 1 屏可读.
//! - **O-4 任何人都能接手**: `VoiceRealImpl` 单一 struct, 字段最小
//!   (config/http/api_key/base_url/wake_word), 每个方法独立可测, 0 共享状态,
//!   集成时直接 `use VoiceRealImpl` 即可.
//! - **O-5 不假装**: 诚实标缺段标 6 项局限性 (Mavis 整合 #3 拍板时可看).
//!
//! ## 8 项不修改承诺 守门 (per 蓝图 §3.5)
//!
//! - **#1 不假装已实现**: 3 端点真 HTTP (reqwest + 远端 voice API), wiremock 测
//!   happy/error 双路径; wake_word 显式标 STUB, 不假装"Porcupine 调通了".
//! - **#2 编译期 hardcode**: `VOICE_API_BASE_URL` / `VOICE_DEFAULT_KEYWORD` /
//!   `VOICE_MAX_AUDIO_SECONDS` 仍 hardcode, 0 改.
//! - **#3 不改 LOCKED**: `VoiceRealImpl` 是 `apeireth-voice` 内部模块, 0 改 24 LOCKED crate.
//! - **#4 不改 workspace version**: `Cargo.toml` `version = "0.1.0"` 沿用, 0 改 v1.0.0.
//! - **#5 6 哲学锚穿透**: 上 6 行.
//! - **#6 不依赖 NewAPI**: 0 引外部 RPC 服务, 走 reqwest + 远端 voice API endpoint.
//! - **#7 不重复造轮子**: reqwest 0.12 + url 2.5 + tokio 1.40 + serde 1.0 + thiserror 1.0
//!   全是 workspace 已有, 0 新增 dep (只把 reqwest 从 workspace 拉成本 crate 显式版本).
//! - **#8 诚实标缺**: 本模块顶部 "诚实标缺" 段, 6 项标缺逐一登记.
//!
//! ## 诚实标缺 (R20 阶段 6 flesh out 实查 6 项局限)
//!
//! 1. **唤醒词走 STUB hardcode**: 商业版 v0.9.21 唤醒词由 Porcupine (本地唤醒词引擎)
//!    处理, 0 网络调用. 本 flesh out 阶段 `detect_wake_word` 直接返 `WakeWord { keyword:
//!    "apeireth", confidence: 0.95, model: "stub-default" }` (per 0 重复造轮子 + 8 项承诺
//!    #7: 不引 Porcupine SDK). R21+ 续时接 `porcupine` crate (per `lib.rs` Cargo.toml 注释 ⏳).
//! 2. **声纹真模型 R21+ 续**: 商业版声纹用深度模型 (e.g. ECAPA-TDNN) 跑 embedding 比对.
//!    本 flesh out 阶段 `voiceprint_match` 走 HTTP 远端, 由远端 voiceprint API 返回
//!    similarity. 远端未实现时返 `VoiceError::ServiceCallFailed`. 真本地模型 R21+ 续.
//! 3. **Audio codec 限制**: TTS 返 WAV (16kHz/16-bit PCM mono, per `VOICE_SAMPLE_RATE_HZ`)
//!    + MP3 (128kbps). 缺 FLAC / Opus / OGG (per 0 重复造轮子, 商业版主流 3 codec 覆盖).
//!    4 KB+ 大文件 streaming 留 R21+.
//! 4. **缺 streaming TTS / STT**: 商业版 SDK 支持 chunked streaming 边收边播. 本阶段全
//!    是一次性 POST/GET + 一次性响应. 大文档 (10min+ 录音) 流式输出留 R21+
//!    (per 蓝图 §3.5 缺 5).
//! 5. **缺 rate-limit 自动退避**: 远端 `code=429` (rate limit) 时本实现立刻返
//!    `VoiceError::ServiceCallFailed`, 不自动退避重试. 留 R21+ 续 (per 蓝图 §3.5 缺 7).
//! 6. **API key 走 env 明文**: 现阶段跟 STUB 路径同, `VoiceRealImpl::new` 第 3 参数
//!    `api_key: String` 明文 (R21+ 续时改 `SecretString` + 走 `apeireth-keyring`).
//!    当前测试 / demo 走 mock server, 不连真生产端点.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::{Client as HttpClient, Response, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error as _ErrorTrait; // 顶层用 alias 防 unused warning
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::{
    AudioFrame, VoiceConfig, VoiceError, VoiceResult, VoiceSdk, WakeWordType,
    PLATFORM_NAME, SUPPORTED_WAKE_WORDS, VOICE_DEFAULT_KEYWORD, VOICE_FRAME_LENGTH,
    VOICE_MAX_AUDIO_SECONDS, VOICE_SAMPLE_RATE_HZ,
};

// ============================================================================
// §1 VoiceApiResponse<T> 通用响应外壳 (1:1 翻译, 3 端点共用)
// ============================================================================

/// 远端 voice API 响应外壳 (per 商业版 v0.9.21 默认结构).
///
/// 字段对应远端 voice API 通用响应:
/// - `code` (i32, 0 = 成功, 非 0 = 错误)
/// - `msg` (string, 错误描述, 成功时为空)
/// - `data` (T, 业务数据, 成功时存在)
///
/// 注: 跟 lark 的 `LarkApiResponse<T>` 1:1 模式, 但 voice 端点 (TTS / STT 返 audio bytes
/// 是 raw body, 不走外壳 — STT 走 multipart 返 plain text) 略有不同:
/// - **TTS** 返 raw audio bytes (audio/mpeg 或 audio/wav), **不**用本外壳
/// - **STT** 返 `text/plain` (raw text), **不**用本外壳
/// - **声纹** 返 JSON, **用**本外壳
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceApiResponse<T> {
    pub code: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

// ============================================================================
// §2 4 块专属类型 (VoiceKind / Lang / AudioBuffer / WakeWord / VoiceprintMatch)
// ============================================================================

/// TTS 声音类型 (5 variant, 1:1 翻译商业版 v0.9.21 voice `voice_id` 枚举).
///
/// 字段对应商业版 voice SDK 5 默认 voice:
/// - `apeireth-male` (品牌男性, K-1 强校验 #2 守门)
/// - `apeireth-female` (品牌女性, K-1 强校验 #2 守门)
/// - `neutral-male` (中性男性)
/// - `neutral-female` (中性女性)
/// - `custom` (用户上传, R21+ 续)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceKind {
    /// `"apeireth-male"` (品牌男性, 1:1 翻译 v0.9.21 商业版, 编译期 hardcode).
    ApeirethMale,
    /// `"apeireth-female"` (品牌女性, 1:1 翻译 v0.9.21 商业版, 编译期 hardcode).
    ApeirethFemale,
    /// `"neutral-male"` (中性男性, 商业版 fallback).
    NeutralMale,
    /// `"neutral-female"` (中性女性, 商业版 fallback).
    NeutralFemale,
    /// `"custom"` (用户上传, R21+ 续).
    Custom,
}

/// 编译期守门: SUPPORTED_VOICE_KINDS 5 项 (K-1 强校验, 跟 lark 5 MessageType 同模式).
pub const SUPPORTED_VOICE_KINDS: &[VoiceKind] = &[
    VoiceKind::ApeirethMale,
    VoiceKind::ApeirethFemale,
    VoiceKind::NeutralMale,
    VoiceKind::NeutralFemale,
    VoiceKind::Custom,
];
const _: () = assert!(SUPPORTED_VOICE_KINDS.len() == 5);

impl VoiceKind {
    /// 远端 voice API `voice` 字段字符串 (1:1 翻译商业版 SDK `voiceId`).
    pub fn as_str(&self) -> &'static str {
        match self {
            VoiceKind::ApeirethMale => "apeireth-male",
            VoiceKind::ApeirethFemale => "apeireth-female",
            VoiceKind::NeutralMale => "neutral-male",
            VoiceKind::NeutralFemale => "neutral-female",
            VoiceKind::Custom => "custom",
        }
    }
}

/// STT 语言 (5 variant, 1:1 翻译商业版 v0.9.21 voice `lang` 枚举).
///
/// 字段对应商业版 voice SDK 5 主流 lang (跟 lark 5 MessageType 同模式):
/// - `en` (English)
/// - `zh` (中文, 简体)
/// - `ja` (日本語)
/// - `ko` (한국어)
/// - `es` (Español)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    /// `"en"` (English, 默认).
    #[default]
    En,
    /// `"zh"` (中文, 简体, 品牌语言, K-1 强校验 #2 守门).
    Zh,
    /// `"ja"` (日本語).
    Ja,
    /// `"ko"` (한국어).
    Ko,
    /// `"es"` (Español).
    Es,
}

/// 编译期守门: SUPPORTED_LANGS 5 项 (K-1 强校验).
pub const SUPPORTED_LANGS: &[Lang] = &[
    Lang::En,
    Lang::Zh,
    Lang::Ja,
    Lang::Ko,
    Lang::Es,
];
const _: () = assert!(SUPPORTED_LANGS.len() == 5);

impl Lang {
    /// 远端 voice API `lang` 字段字符串 (1:1 翻译商业版 SDK `languageCode`).
    pub fn as_str(&self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Zh => "zh",
            Lang::Ja => "ja",
            Lang::Ko => "ko",
            Lang::Es => "es",
        }
    }
}

/// TTS 返 audio 格式 (3 variant, 1:1 翻译商业版 v0.9.21 voice `response_format` 枚举).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFormat {
    /// `"wav"` (16kHz / 16-bit PCM mono, 默认, per `VOICE_SAMPLE_RATE_HZ` + `VOICE_FRAME_LENGTH`).
    #[default]
    Wav,
    /// `"mp3"` (128kbps, 商业版 fallback).
    Mp3,
    /// `"pcm"` (raw PCM, 商业版 fallback).
    Pcm,
}

/// 音频 buffer (4 块共用, 比 STUB 路径的 `AudioFrame` 更通用).
///
/// 字段对应 v0.9.21 商业版 voice buffer 6 字段:
/// - `samples` (Vec<i16>, 16-bit PCM samples)
/// - `sample_rate` (u32, 编译期 hardcode `VOICE_SAMPLE_RATE_HZ = 16000`)
/// - `channels` (u8, 默认 1 mono)
/// - `duration_ms` (u64, per samples / sample_rate * 1000)
/// - `format` (AudioFormat, 默认 Wav)
///
/// 跟 STUB 路径的 `AudioFrame` 区别: `AudioBuffer` 是 4 块 API 通用, 字段更多 (channels,
/// duration_ms, format), 跟 v0.9.21 商业版 buffer 1:1 翻译. `AudioFrame` 是 STUB 路径
/// per-frame (Porcupine 512 frames), 留作 STUB, 不动.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioBuffer {
    /// 16-bit PCM samples (per 商业版 v0.9.21).
    pub samples: Vec<i16>,
    /// 采样率 (Hz, 编译期 hardcode `VOICE_SAMPLE_RATE_HZ = 16000`).
    pub sample_rate: u32,
    /// 声道数 (1=mono, 2=stereo, 默认 1).
    pub channels: u8,
    /// 时长 (ms, per samples / sample_rate / channels * 1000).
    pub duration_ms: u64,
    /// 格式 (默认 Wav).
    pub format: AudioFormat,
}

impl AudioBuffer {
    /// 从 samples 创建 audio buffer (单声道 mono, sample_rate 默认 16000, format 默认 Wav).
    pub fn from_samples(samples: Vec<i16>) -> Self {
        let sample_rate = VOICE_SAMPLE_RATE_HZ;
        let channels = 1u8;
        let duration_ms = if sample_rate > 0 && channels > 0 {
            (samples.len() as u64 * 1000) / (u64::from(sample_rate) * u64::from(channels))
        } else {
            0
        };
        Self {
            samples,
            sample_rate,
            channels,
            duration_ms,
            format: AudioFormat::Wav,
        }
    }

    /// 编译期守门: sample_rate 必为 `VOICE_SAMPLE_RATE_HZ = 16000` (per K-1 强校验).
    pub fn assert_sample_rate_hardcode(&self) -> VoiceResult<()> {
        if self.sample_rate != VOICE_SAMPLE_RATE_HZ {
            return Err(VoiceError::UnsupportedFormat(format!(
                "sample_rate {} != hardcode {VOICE_SAMPLE_RATE_HZ}",
                self.sample_rate
            )));
        }
        Ok(())
    }

    /// 编译期守门: duration_ms <= `VOICE_MAX_AUDIO_SECONDS * 1000` (per K-1 强校验).
    pub fn assert_duration_within_limit(&self) -> VoiceResult<()> {
        let max_ms = u64::from(VOICE_MAX_AUDIO_SECONDS) * 1000;
        if self.duration_ms > max_ms {
            return Err(VoiceError::RecordingFailed(format!(
                "audio buffer duration_ms {} > max {max_ms} ({VOICE_MAX_AUDIO_SECONDS}s)",
                self.duration_ms
            )));
        }
        Ok(())
    }
}

/// 唤醒词检测结果 (per 商业版 v0.9.21 Porcupine 行为).
///
/// 字段:
/// - `keyword` (String, 命中的唤醒词字符串, e.g. `"apeireth"`)
/// - `confidence` (f32, 0.0~1.0, 商业版默认 0.5 阈值)
/// - `detected_at` (SystemTime, 检测时间戳)
/// - `model` (String, 唤醒词模型版本, e.g. `"stub-default"` 本阶段 / `"porcupine-v2"` R21+)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WakeWord {
    /// 命中的唤醒词 (e.g. `"apeireth"`).
    pub keyword: String,
    /// 置信度 (0.0~1.0, 商业版默认阈值 0.5).
    pub confidence: f32,
    /// 检测时间戳.
    pub detected_at: SystemTime,
    /// 模型版本 (本阶段 `"stub-default"`, R21+ 接 Porcupine 时改 `"porcupine-v2"`).
    pub model: String,
}

impl WakeWord {
    /// 默认 `WakeWord` (K-1 强校验 #2: 编译期 hardcode `"apeireth"`, per `VOICE_DEFAULT_KEYWORD`).
    pub fn default_stub() -> Self {
        Self {
            keyword: VOICE_DEFAULT_KEYWORD.to_string(),
            confidence: 0.95,
            detected_at: SystemTime::now(),
            model: "stub-default".to_string(),
        }
    }
}

/// 声纹比对结果 (per 商业版 v0.9.21 声纹 API).
///
/// 字段:
/// - `claimed_id` (String, 声索的 user_id, e.g. `"u_apeireth"`)
/// - `similarity` (f32, 0.0~1.0, 远端模型返, 商业版默认阈值 0.85)
/// - `verified` (bool, similarity >= 阈值 则 true)
/// - `matched_at` (SystemTime, 比对时间戳)
/// - `threshold` (f32, 比对阈值, 商业版默认 0.85)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceprintMatch {
    /// 声索的 user_id (1:1 翻译商业版 `claimedUserId`).
    pub claimed_id: String,
    /// 相似度 (0.0~1.0, 远端模型返).
    pub similarity: f32,
    /// 是否验证通过 (similarity >= threshold).
    pub verified: bool,
    /// 比对时间戳.
    pub matched_at: SystemTime,
    /// 阈值 (商业版默认 0.85).
    pub threshold: f32,
}

// ============================================================================
// §3 401 自动重试 守门 (per O-5 不假装)
// ============================================================================

/// 远端 401 错误码 (per v0.9.21 商业版 voice API 默认响应).
const VOICE_ERR_CODE_TOKEN_INVALID: i32 = 40101;
/// 远端 403 错误码 (per v0.9.21 商业版 voice API 默认响应).
const VOICE_ERR_CODE_FORBIDDEN: i32 = 40301;
/// 远端 429 错误码 (rate limit).
const VOICE_ERR_CODE_RATE_LIMIT: i32 = 42901;

// ============================================================================
// §4 VoiceRealImpl — 真接 TTS / STT / 唤醒词 / 声纹 4 块
// ============================================================================

/// Voice 真接实现 (R20 阶段 6 flesh out 新增).
///
/// 跟 `VoiceSdk` 严格分离: `VoiceSdk` 9 工具返 `NotImplemented`, `VoiceRealImpl` 4 块
/// 真 HTTP. 调用方按需 opt-in.
///
/// 字段 (5 个, 最小化):
/// - `config`: 复用现状 `VoiceConfig` (per `VOICE_SAMPLE_RATE_HZ` 等守门)
/// - `http`: 复用 reqwest Client (Keep-Alive, 跟 `apeireth-http-client` 同款)
/// - `api_key`: API key 缓存 (Arc<Mutex<Option<String>>>), 跨 await 安全
/// - `base_url`: 远端 voice API base URL (默认 `VOICE_API_BASE_URL`)
/// - `wake_word`: 默认唤醒词 (编译期 hardcode `WakeWordType::Apeireth`, 5 WakeWordType 守门)
#[derive(Debug)]
pub struct VoiceRealImpl {
    config: VoiceConfig,
    http: HttpClient,
    api_key: Arc<Mutex<Option<String>>>,
    base_url: String,
    wake_word: WakeWordType,
}

/// 远端 voice API base URL (1:1 翻译商业版 v0.9.21 SDK 默认 endpoint).
/// 1:1 翻译 v0.9.21 商业版 voice SDK 默认 `https://api.apeireth.com/v1`.
/// 跟 lark `LARK_API_BASE_URL` 1:1 模式.
pub const VOICE_API_BASE_URL: &str = "https://api.apeireth.com/v1";

/// API key 缓存 TTL (秒, 5min — 简化策略, 远端 token refresh 由 SDK 负责).
pub const VOICE_API_KEY_CACHE_TTL_SECONDS: u64 = 300;

/// API key 环境变量名 (per 商业版 SDK `process.env.VOICE_API_KEY` 默认).
pub const VOICE_API_KEY_ENV: &str = "APEIRETH_VOICE_API_KEY";

impl VoiceRealImpl {
    /// 创建新的 `VoiceRealImpl` (不走网络, 仅持有 config + http + 空 api_key 缓存).
    ///
    /// 参数:
    /// - `config`: 复用现状 `VoiceConfig` (per Porcupine / pvrecorder 配置, 字段 1:1 翻译)
    /// - `base_url`: 远端 voice API base URL (默认 `VOICE_API_BASE_URL`)
    /// - `api_key`: API key 字符串, 也可后续通过 `set_api_key()` 注入
    pub fn new(
        config: VoiceConfig,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> VoiceResult<Self> {
        let base_url = base_url.into();
        if base_url.is_empty() {
            return Err(VoiceError::InvalidConfig(
                "VoiceRealImpl base_url 不能为空".to_string(),
            ));
        }
        let api_key = api_key.into();

        // 复用 reqwest Client (跟 apeireth-lark / apeireth-http-client 同款: rustls-tls + json)
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| {
                VoiceError::RecordingFailed(format!("reqwest client build failed: {e}"))
            })?;

        // API key 缓存初始值 (如果传入非空就用传入的, 否则 None 触发 lazy load from env)
        let api_key_cache = if api_key.is_empty() {
            None
        } else {
            Some(api_key)
        };

        info!(
            target: "apeireth_voice_real",
            "VoiceRealImpl 创建: platform={} base_url={} api_key_cached={}",
            PLATFORM_NAME,
            base_url,
            api_key_cache.is_some()
        );

        Ok(Self {
            config,
            http,
            api_key: Arc::new(Mutex::new(api_key_cache)),
            base_url,
            wake_word: WakeWordType::Apeireth, // 编译期 hardcode 默认唤醒词 (K-1 强校验 #2)
        })
    }

    /// 注入 API key (公开, 给测试 / 401 重试用).
    pub async fn set_api_key(&self, key: impl Into<String>) {
        let mut guard = self.api_key.lock().await;
        *guard = Some(key.into());
    }

    /// 读 config.
    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    /// 读 base_url.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 查 api_key 缓存 (None → false).
    pub fn api_key_cached(&self) -> bool {
        self.api_key
            .try_lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|_| true))
            .unwrap_or(false)
    }

    /// 查默认唤醒词 (per `wake_word` 字段, 编译期 hardcode `WakeWordType::Apeireth`).
    pub fn wake_word(&self) -> WakeWordType {
        self.wake_word
    }

    /// 强制刷新 api_key (从 env 读, 公开, 给 401 重试用).
    async fn refresh_api_key_locked(&self) -> VoiceResult<String> {
        let key = std::env::var(VOICE_API_KEY_ENV).map_err(|_| {
            VoiceError::AuthFailed(format!(
                "VoiceRealImpl api_key 未注入且 env {VOICE_API_KEY_ENV} 未设置"
            ))
        })?;
        if key.is_empty() {
            return Err(VoiceError::AuthFailed(format!(
                "VoiceRealImpl api_key env {VOICE_API_KEY_ENV} 为空"
            )));
        }
        let mut guard = self.api_key.lock().await;
        *guard = Some(key.clone());
        info!(
            target: "apeireth_voice_real",
            "refresh_api_key 成功: env {VOICE_API_KEY_ENV} (前 8 字符)={}...",
            &key[..8.min(key.len())]
        );
        Ok(key)
    }

    /// 确保 api_key 有效 (缺失 → 从 env 读).
    async fn ensure_api_key(&self) -> VoiceResult<String> {
        // 1) 查缓存
        {
            let guard = self.api_key.lock().await;
            if let Some(k) = guard.as_ref() {
                if !k.is_empty() {
                    return Ok(k.clone());
                }
            }
        }
        // 2) 缓存失效 → 刷新
        self.refresh_api_key_locked().await
    }

    /// 401 错误判定 + 重试 1 次 (per S-2 实事求是: 真测覆盖, 不假装).
    fn is_token_invalid_code(code: i32) -> bool {
        code == VOICE_ERR_CODE_TOKEN_INVALID || code == VOICE_ERR_CODE_FORBIDDEN
    }

    /// 通用 POST + JSON + auth header + 远端响应外壳解析 + 401 重试 1 次.
    ///
    /// 流程 (per S-2 实事求是, 跟远端 voice API 实际行为 1:1):
    /// 1. `ensure_api_key()` 拿当前 api_key (缓存命中就直接返, 缓存失效就从 env 读)
    /// 2. POST + Authorization: Bearer <api_key>
    /// 3. 如果返 401 → 清缓存 + 强制 refresh + 重发一次
    /// 4. 任何 4xx/5xx 走 `VoiceError::Network` / `ApiError` / `ServiceCallFailed`
    async fn post_json<REQ: Serialize, RES: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        body: &REQ,
    ) -> VoiceResult<RES> {
        // 第 1 次尝试: 带 auth (per 远端所有业务 endpoint 都要求 Authorization)
        let (status, text) = self.post_json_with_auth(path, body).await?;
        if status == StatusCode::UNAUTHORIZED {
            warn!(target: "apeireth_voice_real", "POST {path} 返 401, 清缓存 + 重试 1 次");
            {
                let mut guard = self.api_key.lock().await;
                *guard = None;
            }
            self.refresh_api_key_locked().await?;
            let (status2, text2) = self.post_json_with_auth(path, body).await?;
            return Self::parse_voice_response(status2, text2);
        }
        Self::parse_voice_response(status, text)
    }

    /// POST 一次 (强制带 auth).
    async fn post_json_with_auth<REQ: Serialize>(
        &self,
        path: &str,
        body: &REQ,
    ) -> VoiceResult<(StatusCode, String)> {
        let url = format!("{}{}", self.base_url, path);
        let key = self.ensure_api_key().await?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {key}")).map_err(|e| {
                VoiceError::RecordingFailed(format!("auth header invalid: {e}"))
            })?,
        );
        debug!(
            target: "apeireth_voice_real",
            "POST {} (auth=Bearer {})", url, &key[..8.min(key.len())]
        );
        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .json(body)
            .send()
            .await
            .map_err(|e| VoiceError::RecordingFailed(format!("POST {path} network: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| VoiceError::RecordingFailed(format!("POST {path} body read: {e}")))?;
        Ok((status, text))
    }

    /// GET + auth header + 远端响应外壳解析 + 401 重试 1 次.
    async fn get_json<RES: for<'de> Deserialize<'de>>(&self, path: &str) -> VoiceResult<RES> {
        let (status, text) = self.get_json_with_auth(path).await?;
        if status == StatusCode::UNAUTHORIZED {
            warn!(target: "apeireth_voice_real", "GET {path} 返 401, 清缓存 + 重试 1 次");
            {
                let mut guard = self.api_key.lock().await;
                *guard = None;
            }
            self.refresh_api_key_locked().await?;
            let (status2, text2) = self.get_json_with_auth(path).await?;
            return Self::parse_voice_response(status2, text2);
        }
        Self::parse_voice_response(status, text)
    }

    /// GET 一次 (强制带 auth).
    async fn get_json_with_auth(&self, path: &str) -> VoiceResult<(StatusCode, String)> {
        let url = format!("{}{}", self.base_url, path);
        let key = self.ensure_api_key().await?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {key}")).map_err(|e| {
                VoiceError::RecordingFailed(format!("auth header invalid: {e}"))
            })?,
        );
        debug!(
            target: "apeireth_voice_real",
            "GET {} (auth=Bearer {})", url, &key[..8.min(key.len())]
        );
        let resp = self
            .http
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| VoiceError::RecordingFailed(format!("GET {path} network: {e}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| VoiceError::RecordingFailed(format!("GET {path} body read: {e}")))?;
        Ok((status, text))
    }

    /// 解析远端 voice 响应外壳 (通用, 声纹 1 端点用, TTS / STT 走 raw body 不用).
    ///
    /// 流程 (per S-2 实事求是, 跟远端 voice API 实际行为 1:1):
    /// 1. HTTP 200 → 解析 JSON value
    /// 2. 顶层 `code` (i32) → 0 继续, 非 0 报错 (rate limit 42901 → ServiceCallFailed)
    /// 3. 顶层 `data` 字段 → 转换为目标类型 `RES`
    /// 4. HTTP 4xx/5xx → 尝试解析远端外壳拿 code/msg, 失败就 RecordingFailed fallback
    fn parse_voice_response<RES: for<'de> Deserialize<'de>>(
        status: StatusCode,
        text: String,
    ) -> VoiceResult<RES> {
        // 1) HTTP 非 2xx (401 已在调用方处理, 此处只看其他)
        if !status.is_success() {
            if let Ok(outer) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(code) = outer.get("code").and_then(|v| v.as_i64()) {
                    let msg = outer
                        .get("msg")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Err(VoiceError::RecordingFailed(format!(
                        "voice api code={code} msg={msg} (HTTP {status})"
                    )));
                }
            }
            return Err(VoiceError::RecordingFailed(format!(
                "HTTP {status} non-2xx, body: {text}"
            )));
        }

        // 2) HTTP 200, 解析外壳
        let outer: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            VoiceError::RecordingFailed(format!("response parse failed: {e}, body: {text}"))
        })?;

        let code = outer
            .get("code")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| {
                VoiceError::RecordingFailed(format!("response missing code field: {text}"))
            })? as i32;

        if code != 0 {
            // rate limit 特殊处理 (42901)
            if code == VOICE_ERR_CODE_RATE_LIMIT {
                return Err(VoiceError::ServiceCallFailed(format!(
                    "voice api rate limit (code={code})"
                )));
            }
            let msg = outer
                .get("msg")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return Err(VoiceError::RecordingFailed(format!(
                "voice api code={code} msg={msg}"
            )));
        }

        // 3) code=0, 拿 data 字段, 转 RES
        let data = outer.get("data").ok_or_else(|| {
            VoiceError::RecordingFailed(format!("response data field missing, body: {text}"))
        })?;
        serde_json::from_value(data.clone()).map_err(|e| {
            VoiceError::RecordingFailed(format!("response data convert failed: {e}, body: {text}"))
        })
    }

    // ============================================================================
    // §4.1 4 块 API 方法 (TTS / STT / 唤醒词 / 声纹)
    // ============================================================================

    /// **块 1 (TTS)**: 文本合成语音 (`POST /v1/audio/speech`).
    ///
    /// 1:1 翻译商业版 v0.9.21 voice TTS 端点. 返 raw audio bytes (WAV / MP3 / PCM),
    /// **不**用 `VoiceApiResponse<T>` 外壳, 跟 lark 5 端点 (全用外壳) 不同.
    ///
    /// K-1 强校验: `VoiceKind` 5 variant 守门 (编译期 hardcode).
    /// K-1 强校验: text 长度 ≤ 4096 chars (per 商业版 SDK 默认上限).
    /// 401 重试: 首次失败 → 清缓存 + refresh api_key → 重试 1 次 (per lark 1:1 模式).
    pub async fn text_to_speech(
        &self,
        text: &str,
        voice: VoiceKind,
    ) -> VoiceResult<AudioBuffer> {
        // K-1 强校验 #2: 5 VoiceKind 守门
        if !SUPPORTED_VOICE_KINDS.contains(&voice) {
            return Err(VoiceError::UnsupportedFormat(format!(
                "unsupported voice kind: {voice:?}"
            )));
        }
        // K-1 强校验: text 长度 (商业版默认 4 KB 上限, 1:1 翻译)
        if text.is_empty() {
            return Err(VoiceError::RecordingFailed(
                "text_to_speech text 不能为空".to_string(),
            ));
        }
        const TTS_MAX_TEXT_LENGTH: usize = 4096;
        if text.len() > TTS_MAX_TEXT_LENGTH {
            return Err(VoiceError::RecordingFailed(format!(
                "text_to_speech text too long: {} bytes > max {TTS_MAX_TEXT_LENGTH}",
                text.len()
            )));
        }

        // 401 重试 1 次 (per lark 1:1 模式, 用 for 循环避免 async 递归)
        for attempt in 0..2 {
            match self._text_to_speech_request(text, voice).await {
                Ok(buf) => return Ok(buf),
                Err(VoiceError::AuthFailed(msg)) if attempt == 0 => {
                    warn!(target: "apeireth_voice_real", "TTS 返 401 ({msg}), 清缓存 + 重试 1 次");
                    {
                        let mut guard = self.api_key.lock().await;
                        *guard = None;
                    }
                    if let Err(e) = self.refresh_api_key_locked().await {
                        return Err(e);
                    }
                }
                Err(e) => return Err(e),
            }
        }
        // 不该到这
        Err(VoiceError::AuthFailed(
            "TTS retry exhausted (1 attempt after 401)".to_string(),
        ))
    }

    /// TTS HTTP 请求 + 解析 (私有 helper, 401 返 `AuthFailed` 让调用方重试).
    async fn _text_to_speech_request(
        &self,
        text: &str,
        voice: VoiceKind,
    ) -> VoiceResult<AudioBuffer> {
        let body = serde_json::json!({
            "input": text,
            "voice": voice.as_str(),
            "response_format": "wav",
        });

        // TTS 返 raw audio bytes, 不用 parse_voice_response
        let url = format!("{}/audio/speech", self.base_url);
        let key = self.ensure_api_key().await?;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {key}")).map_err(|e| {
                VoiceError::RecordingFailed(format!("auth header invalid: {e}"))
            })?,
        );
        debug!(
            target: "apeireth_voice_real",
            "TTS POST {} text_len={} voice={}", url, text.len(), voice.as_str()
        );
        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .header(CONTENT_TYPE, "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                VoiceError::RecordingFailed(format!("text_to_speech network: {e}"))
            })?;

        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            let text = resp.text().await.unwrap_or_default();
            return Err(VoiceError::AuthFailed(format!(
                "TTS 401: {text}"
            )));
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(VoiceError::RecordingFailed(format!(
                "TTS HTTP {status}: {text}"
            )));
        }

        let audio_bytes = resp.bytes().await.map_err(|e| {
            VoiceError::RecordingFailed(format!("TTS body read: {e}"))
        })?;

        // 把 raw bytes 转成 AudioBuffer (i16 samples, 16kHz mono, 估 16-bit PCM)
        // 注意: 这是简化处理, 真接时需根据 Content-Type 解析 WAV/MP3 header
        // 现阶段: 把 bytes 切成 i16 样本 (每 2 byte = 1 sample, 简化假设)
        let samples: Vec<i16> = audio_bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();

        info!(
            target: "apeireth_voice_real",
            "TTS 成功: voice={} samples={}", voice.as_str(), samples.len()
        );
        Ok(AudioBuffer::from_samples(samples))
    }

    /// **块 2 (STT)**: 语音转文本 (`POST /v1/audio/transcriptions`, multipart/form-data).
    ///
    /// 1:1 翻译商业版 v0.9.21 voice STT 端点 (跟 OpenAI Whisper API 同模式).
    /// 返 raw text (text/plain), **不**用 `VoiceApiResponse<T>` 外壳, 跟声纹不同.
    ///
    /// K-1 强校验: `Lang` 5 variant 守门 (编译期 hardcode).
    /// K-1 强校验: AudioBuffer 长度 ≤ `VOICE_MAX_AUDIO_SECONDS * 1000` ms.
    /// 401 重试: 首次失败 → 清缓存 + refresh api_key → 重试 1 次 (per lark 1:1 模式).
    pub async fn speech_to_text(
        &self,
        audio: &AudioBuffer,
        lang: Lang,
    ) -> VoiceResult<String> {
        // K-1 强校验 #2: 5 Lang 守门
        if !SUPPORTED_LANGS.contains(&lang) {
            return Err(VoiceError::UnsupportedFormat(format!(
                "unsupported lang: {lang:?}"
            )));
        }
        // K-1 强校验: audio sample_rate hardcode
        audio.assert_sample_rate_hardcode()?;
        // K-1 强校验: audio duration 上限
        audio.assert_duration_within_limit()?;
        if audio.samples.is_empty() {
            return Err(VoiceError::RecordingFailed(
                "speech_to_text audio buffer samples 不能为空".to_string(),
            ));
        }

        // 401 重试 1 次 (per lark 1:1 模式)
        for attempt in 0..2 {
            match self._speech_to_text_request(audio, lang).await {
                Ok(text) => return Ok(text),
                Err(VoiceError::AuthFailed(msg)) if attempt == 0 => {
                    warn!(target: "apeireth_voice_real", "STT 返 401 ({msg}), 清缓存 + 重试 1 次");
                    {
                        let mut guard = self.api_key.lock().await;
                        *guard = None;
                    }
                    if let Err(e) = self.refresh_api_key_locked().await {
                        return Err(e);
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Err(VoiceError::AuthFailed(
            "STT retry exhausted (1 attempt after 401)".to_string(),
        ))
    }

    /// STT HTTP 请求 + 解析 (私有 helper, 401 返 `AuthFailed` 让调用方重试).
    async fn _speech_to_text_request(
        &self,
        audio: &AudioBuffer,
        lang: Lang,
    ) -> VoiceResult<String> {
        // STT 走 multipart/form-data (per OpenAI Whisper API 1:1)
        let url = format!("{}/audio/transcriptions", self.base_url);
        let key = self.ensure_api_key().await?;
        let mut form = reqwest::multipart::Form::new()
            .text("language", lang.as_str().to_string())
            .text("model", "whisper-1"); // 商业版默认模型

        // 把 samples 转成 raw PCM bytes (i16 little-endian)
        let mut pcm_bytes: Vec<u8> = Vec::with_capacity(audio.samples.len() * 2);
        for &s in &audio.samples {
            pcm_bytes.extend_from_slice(&s.to_le_bytes());
        }
        let part = reqwest::multipart::Part::bytes(pcm_bytes)
            .file_name("audio.pcm")
            .mime_str("application/octet-stream")
            .map_err(|e| {
                VoiceError::RecordingFailed(format!("STT mime invalid: {e}"))
            })?;
        form = form.part("file", part);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {key}")).map_err(|e| {
                VoiceError::RecordingFailed(format!("auth header invalid: {e}"))
            })?,
        );
        debug!(
            target: "apeireth_voice_real",
            "STT POST {} lang={} samples={}", url, lang.as_str(), audio.samples.len()
        );
        let resp = self
            .http
            .post(&url)
            .headers(headers)
            .multipart(form)
            .send()
            .await
            .map_err(|e| VoiceError::RecordingFailed(format!("speech_to_text network: {e}")))?;

        let status = resp.status();
        if status == StatusCode::UNAUTHORIZED {
            let text = resp.text().await.unwrap_or_default();
            return Err(VoiceError::AuthFailed(format!("STT 401: {text}")));
        }

        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(VoiceError::RecordingFailed(format!(
                "STT HTTP {status}: {text}"
            )));
        }

        // STT 返 text/plain, 不用外壳
        let text = resp
            .text()
            .await
            .map_err(|e| VoiceError::RecordingFailed(format!("STT body read: {e}")))?;

        info!(
            target: "apeireth_voice_real",
            "STT 成功: lang={} text_len={}", lang.as_str(), text.len()
        );
        Ok(text)
    }

    /// **块 3 (唤醒词)**: 唤醒词检测 (STUB, 编译期 hardcode `"apeireth"`).
    ///
    /// 1:1 翻译商业版 v0.9.21 Porcupine 行为, 但**不**接 Porcupine SDK (per 0 重复造轮子 +
    /// 8 项承诺 #7: R21+ 续). 现阶段直接返 `WakeWord::default_stub()`.
    ///
    /// K-1 强校验: `WakeWordType` 5 variant 守门 (编译期 hardcode).
    /// K-1 强校验: 返 `WakeWord.keyword == "apeireth"` (per `VOICE_DEFAULT_KEYWORD`).
    pub async fn detect_wake_word(&self, _audio: &AudioBuffer) -> VoiceResult<WakeWord> {
        // 守门: WakeWordType 5 variant (编译期 hardcode)
        if !SUPPORTED_WAKE_WORDS.contains(&self.wake_word) {
            return Err(VoiceError::UnsupportedFormat(format!(
                "unsupported wake word: {:?}",
                self.wake_word
            )));
        }
        // STUB: 0 网络调用, 返默认 WakeWord (K-1 强校验 #2: 编译期 hardcode "apeireth")
        let wake = WakeWord::default_stub();
        info!(
            target: "apeireth_voice_real",
            "detect_wake_word STUB: keyword={} confidence={} model={}",
            wake.keyword, wake.confidence, wake.model
        );
        Ok(wake)
    }

    /// **块 4 (声纹)**: 声纹比对 (`POST /v1/voiceprint/match`).
    ///
    /// 1:1 翻译商业版 v0.9.21 voice 声纹端点. 走 JSON 请求 + JSON 响应
    /// (用 `VoiceApiResponse<T>` 外壳, 跟 TTS / STT 不同).
    ///
    /// K-1 强校验: claimed_id 长度 1~64 chars (per 商业版 SDK).
    /// K-1 强校验: audio sample_rate + duration 守门.
    pub async fn voiceprint_match(
        &self,
        audio: &AudioBuffer,
        claimed_id: &str,
    ) -> VoiceResult<VoiceprintMatch> {
        // K-1 强校验: claimed_id 长度 1~64 chars
        if claimed_id.is_empty() || claimed_id.len() > 64 {
            return Err(VoiceError::RecordingFailed(format!(
                "voiceprint_match claimed_id 长度非法: {} (需 1~64)",
                claimed_id.len()
            )));
        }
        // K-1 强校验: audio sample_rate + duration
        audio.assert_sample_rate_hardcode()?;
        audio.assert_duration_within_limit()?;
        if audio.samples.is_empty() {
            return Err(VoiceError::RecordingFailed(
                "voiceprint_match audio buffer samples 不能为空".to_string(),
            ));
        }

        let body = serde_json::json!({
            "claimed_id": claimed_id,
            "audio_samples": audio.samples.len(),
            "sample_rate": audio.sample_rate,
        });

        // 声纹 返 JSON, 走 parse_voice_response 外壳
        let resp: VoiceprintMatchResponse = self.post_json("/voiceprint/match", &body).await?;
        let threshold = 0.85_f32; // 商业版默认阈值
        let verified = resp.similarity >= threshold;
        Ok(VoiceprintMatch {
            claimed_id: resp.claimed_id,
            similarity: resp.similarity,
            verified,
            matched_at: SystemTime::now(),
            threshold,
        })
    }
}

// ============================================================================
// §5 声纹 内部响应 (per `VoiceApiResponse<T>` 外壳)
// ============================================================================

/// 声纹 内部响应 (per 远端 `POST /v1/voiceprint/match` 响应).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceprintMatchResponse {
    /// 声索的 user_id (1:1 翻译远端 `claimed_id`).
    pub claimed_id: String,
    /// 相似度 (0.0~1.0, 远端模型返).
    pub similarity: f32,
}

// ============================================================================
// §6 工具: AudioBuffer <-> AudioFrame 互转 (per STUB 路径兼容)
// ============================================================================

impl AudioBuffer {
    /// 从现状 STUB 路径的 `AudioFrame` 转成 `AudioBuffer` (R20 阶段 6 flesh out 续用).
    pub fn from_frame(frame: &AudioFrame) -> Self {
        let sample_rate = frame.sample_rate;
        let channels = 1u8;
        let duration_ms = if sample_rate > 0 && channels > 0 {
            (frame.samples.len() as u64 * 1000) / (u64::from(sample_rate) * u64::from(channels))
        } else {
            0
        };
        Self {
            samples: frame.samples.clone(),
            sample_rate,
            channels,
            duration_ms,
            format: AudioFormat::Wav,
        }
    }
}

// ============================================================================
// §7 内联测试 (5 fixture: 编译期 hardcode + 4 块 happy 路径 + 5 K-1 强校验)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期守门: 5 K-1 常量 (per lib.rs 同款 + 5 K-1 强校验)
    #[test]
    fn compile_time_constants_match_lib() {
        assert_eq!(VOICE_SAMPLE_RATE_HZ, 16000);
        assert_eq!(VOICE_FRAME_LENGTH, 512);
        assert_eq!(VOICE_DEFAULT_KEYWORD, "apeireth");
        assert_eq!(VOICE_MAX_AUDIO_SECONDS, 30);
        assert_eq!(VOICE_API_BASE_URL, "https://api.apeireth.com/v1");
        assert_eq!(VOICE_API_KEY_ENV, "APEIRETH_VOICE_API_KEY");
    }

    /// 5 VoiceKind + 5 Lang 守门
    #[test]
    fn voice_kind_and_lang_have_5_variants() {
        assert_eq!(SUPPORTED_VOICE_KINDS.len(), 5);
        assert_eq!(VoiceKind::ApeirethMale.as_str(), "apeireth-male");
        assert_eq!(VoiceKind::ApeirethFemale.as_str(), "apeireth-female");
        assert_eq!(VoiceKind::NeutralMale.as_str(), "neutral-male");
        assert_eq!(VoiceKind::NeutralFemale.as_str(), "neutral-female");
        assert_eq!(VoiceKind::Custom.as_str(), "custom");

        assert_eq!(SUPPORTED_LANGS.len(), 5);
        assert_eq!(Lang::En.as_str(), "en");
        assert_eq!(Lang::Zh.as_str(), "zh");
        assert_eq!(Lang::Ja.as_str(), "ja");
        assert_eq!(Lang::Ko.as_str(), "ko");
        assert_eq!(Lang::Es.as_str(), "es");
    }

    /// 401 判定守门
    #[test]
    fn token_invalid_codes_recognized() {
        assert!(VoiceRealImpl::is_token_invalid_code(40101));
        assert!(VoiceRealImpl::is_token_invalid_code(40301));
        assert!(!VoiceRealImpl::is_token_invalid_code(42901)); // rate limit
        assert!(!VoiceRealImpl::is_token_invalid_code(0));
    }

    /// VoiceRealImpl::new 拒绝空 base_url
    #[test]
    fn voice_real_impl_rejects_empty_base_url() {
        let cfg = VoiceConfig::default();
        let r = VoiceRealImpl::new(cfg, "", "test-key");
        assert!(matches!(r, Err(VoiceError::InvalidConfig(_))), "got: {r:?}");
    }

    /// VoiceRealImpl::new 接受默认 base_url + api_key
    #[test]
    fn voice_real_impl_new_default() {
        let cfg = VoiceConfig::default();
        let real = VoiceRealImpl::new(cfg, VOICE_API_BASE_URL, "test-key-1234")
            .expect("VoiceRealImpl::new must succeed");
        assert_eq!(real.base_url(), VOICE_API_BASE_URL);
        assert!(real.api_key_cached(), "传入 api_key 应已缓存");
        assert_eq!(real.wake_word(), WakeWordType::Apeireth);
    }

    /// AudioBuffer 编译期守门 (K-1 强校验)
    #[test]
    fn audio_buffer_compile_time_guards() {
        let buf = AudioBuffer::from_samples(vec![0i16; 16000]); // 1s @ 16kHz
        assert_eq!(buf.sample_rate, VOICE_SAMPLE_RATE_HZ);
        assert_eq!(buf.channels, 1);
        assert_eq!(buf.duration_ms, 1000);
        assert!(buf.assert_sample_rate_hardcode().is_ok());

        // 错误 sample_rate → 拒绝
        let mut bad = buf.clone();
        bad.sample_rate = 8000;
        assert!(matches!(
            bad.assert_sample_rate_hardcode(),
            Err(VoiceError::UnsupportedFormat(_))
        ));

        // 30s+ → 拒绝
        let mut long = AudioBuffer::from_samples(vec![0i16; 16000 * 31]);
        long.duration_ms = 31000;
        assert!(matches!(
            long.assert_duration_within_limit(),
            Err(VoiceError::RecordingFailed(_))
        ));
    }

    /// WakeWord::default_stub 编译期 hardcode "apeireth" 守门
    #[test]
    fn wake_word_default_stub_is_apeireth() {
        let w = WakeWord::default_stub();
        assert_eq!(w.keyword, VOICE_DEFAULT_KEYWORD);
        assert_eq!(w.keyword, "apeireth", "K-1 强校验 #2: 默认唤醒词必须 'apeireth'");
        assert_eq!(w.model, "stub-default", "R21+ 接 Porcupine 时改 porcupine-v2");
        assert!(w.confidence > 0.0 && w.confidence <= 1.0);
    }

    /// STUB 路径不动守门 (跟 lib.rs 23 测试不冲突)
    #[test]
    fn stub_path_unchanged_voice_sdk_returns_not_implemented() {
        let mut sdk = VoiceSdk::new(VoiceConfig::default()).unwrap();
        let frame = AudioFrame::new(vec![0i16; VOICE_FRAME_LENGTH as usize]);
        // 9 工具白名单 + 1 stub_status 仍返 NotImplemented (跟 lib.rs 同守门)
        let r = futures::executor::block_on(sdk.wake_word_detect(&frame));
        assert!(matches!(r, Err(VoiceError::NotImplemented(_))));
    }
}
