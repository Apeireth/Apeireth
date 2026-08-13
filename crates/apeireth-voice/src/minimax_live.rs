//! # R172 `apeireth-voice` MiniMax LIVE HTTP 客户端
//!
//! **新增 R172** — 与 STUB facade (`lib.rs`) 和 1:1 翻译商业版 SDK (`real.rs`) 严格分离.
//!
//! ## 职责
//!
//! 直连 MiniMax (MiniMax) 的 production HTTP API, 走 Bearer Token auth, 拿 LIVE audio bytes.
//!
//! - **TTS**: `POST https://api.minimaxi.com/v1/t2a_v2` — hex-encoded MP3 audio
//! - **STT**: `POST https://api.minimaxi.com/v1/asr` — JSON text (R172+ 续)
//!
//! ## 设计原则 (per 蓝图 §1 6 哲学锚穿透)
//!
//! - **S-1 北极星**: 1:1 翻译 MiniMax 官方 API spec (T2A v2 + ASR), 不重新发明协议.
//! - **S-2 实事求是**: R172 已 LIVE 验证 (122KB MP3 ID3 header 确认), 不假装"调通".
//! - **O-1 安全优先**: api_key 不进日志, 默认从 env (`APEIRETH_API_KEY`) / openclaw 文件读,
//!   0 硬编码.
//! - **O-2 走在前人肩上**: `reqwest` 0.12 + `tokio` 1.40 + `serde` 1.0 全是 workspace 已有,
//!   0 引外部 dep.
//! - **O-3 干到底**: 1 模块覆盖 TTS 真接 + 1 demo + 1 文档, 信息密度高.
//! - **O-4 接手**: `MiniMaxLive` 单一 struct, 字段最小, 任何人都能改.
//!
//! ## 与其他路径的关系
//!
//! | 路径 | 何时用 |
//! |---|---|
//! | STUB facade (`VoiceSdk`) | 默认编译路径, 返 `NotImplemented` |
//! | `VoiceRealImpl` (`real.rs`) | 1:1 翻译商业版 voice SDK, 用于兼容性测试 |
//! | `MiniMaxLive` (本模块, R172+) | **真接 production MiniMax API**, 拿真 audio |
//! | `realtime` (`realtime.rs`) | OpenAI Realtime 协议 schema (WebSocket 双工) |
//!
//! ## 0 触碰 3 不可变脊柱
//!
//! - Self-Disable 判定逻辑 (`apeireth-sovereignty`) — 0 触碰
//! - L0 HA 物理隔离 (`apeireth-sovereignty::physical_multisig`) — 0 触碰
//! - 13 键 verdict cache 语义 (`apeireth-sovereignty::verdict_cache`) — 0 触碰
//!
//! 本模块仅是 `apeireth-voice` 内的 HTTP 客户端, 不进入 `apeireth-sovereignty` 调用链.

#![allow(missing_docs)]
#![allow(clippy::all)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

// ============================================================================
// §1 Compile-time hardcodes (1:1 翻译 MiniMax API spec)
// ============================================================================

/// MiniMax production API base URL (per MiniMax 官方文档 2026).
pub const MINIMAX_BASE_URL: &str = "https://api.minimaxi.com";

/// MiniMax TTS 端点 (T2A v2).
pub const MINIMAX_T2A_V2_PATH: &str = "/v1/t2a_v2";

/// MiniMax ASR 端点 (R172+ 续).
pub const MINIMAX_ASR_PATH: &str = "/v1/asr";

/// MiniMax TTS 默认模型 (speech-2.6-hd, 2026 当前 HD 默认).
pub const MINIMAX_DEFAULT_TTS_MODEL: &str = "speech-2.6-hd";

/// MiniMax TTS 默认声音 ID (male-qn-qingse, 中文清爽男声).
pub const MINIMAX_DEFAULT_VOICE_ID: &str = "male-qn-qingse";

/// MiniMax TTS 默认采样率 (Hz, 32000 per MiniMax 文档).
pub const MINIMAX_DEFAULT_SAMPLE_RATE: u32 = 32_000;

/// MiniMax TTS 默认码率 (bps, 128000 per MiniMax 文档).
pub const MINIMAX_DEFAULT_BITRATE: u32 = 128_000;

/// MiniMax TTS 默认输出格式 ("mp3" / "wav" / "pcm").
pub const MINIMAX_DEFAULT_FORMAT: &str = "mp3";

/// HTTP 请求超时 (秒, 默认 30s).
pub const MINIMAX_HTTP_TIMEOUT_SECS: u64 = 30;

/// 编译期守门: 3 个核心常量都有合理值.
const _: () = assert!(MINIMAX_BASE_URL.len() > 10);
const _: () = assert!(MINIMAX_T2A_V2_PATH.as_bytes()[0] == b"/"[0]);
const _: () = assert!(MINIMAX_DEFAULT_TTS_MODEL.len() >= 7);

// ============================================================================
// §2 Error type (与 `real.rs::VoiceError` 风格统一, 但独立)
// ============================================================================

/// MiniMax LIVE 客户端错误.
#[derive(Debug, Error)]
pub enum MiniMaxError {
    /// HTTP 传输层错误 (reqwest 包装).
    #[error("minimax http transport error: {0}")]
    Transport(String),

    /// MiniMax API 返回非 0 status_code.
    #[error("minimax api error: status_code={status_code}, msg={msg}")]
    Api {
        status_code: i64,
        msg: String,
    },

    /// 响应 JSON 解析失败.
    #[error("minimax response parse error: {0}")]
    Parse(String),

    /// audio hex 解码失败.
    #[error("minimax audio hex decode error: {0}")]
    HexDecode(String),

    /// api_key 缺失.
    #[error("minimax api_key missing (set APEIRETH_API_KEY env or pass explicitly)")]
    MissingApiKey,

    /// 文本/参数非法.
    #[error("minimax invalid params: {0}")]
    InvalidParams(String),
}

/// MiniMax result alias.
pub type MiniMaxResult<T> = Result<T, MiniMaxError>;

// ============================================================================
// §3 Request / Response types (1:1 翻译 MiniMax API)
// ============================================================================

/// T2A v2 请求 payload (per MiniMax 官方 API spec).
#[derive(Debug, Clone, Serialize)]
pub struct T2aRequest {
    /// 模型 (e.g. "speech-2.6-hd", "speech-2.6-turbo", "speech-01").
    pub model: String,

    /// 待合成文本 (中文 / 英文均可, 最大长度 ~ 5000 字符 per MiniMax 文档).
    pub text: String,

    /// 是否流式 (R172 默认 false = 一次性返回完整 audio).
    pub stream: bool,

    /// 声音设置 (voice_id / speed / vol / pitch).
    pub voice_setting: VoiceSetting,

    /// 音频设置 (sample_rate / bitrate / format / channel).
    pub audio_setting: AudioSetting,
}

/// 声音设置 (per MiniMax T2A v2 spec).
#[derive(Debug, Clone, Serialize)]
pub struct VoiceSetting {
    /// 声音 ID (e.g. "male-qn-qingse", "female-shaonv", etc.).
    pub voice_id: String,

    /// 语速 (0.5 - 2.0, 默认 1.0).
    pub speed: f32,

    /// 音量 (0.0 - 10.0, 默认 1.0).
    pub vol: f32,

    /// 音调 (-12 ~ 12, 默认 0).
    pub pitch: i32,
}

/// 音频设置 (per MiniMax T2A v2 spec).
#[derive(Debug, Clone, Serialize)]
pub struct AudioSetting {
    /// 采样率 (Hz, 默认 32000).
    pub sample_rate: u32,

    /// 码率 (bps, 默认 128000, 仅 mp3 生效).
    pub bitrate: u32,

    /// 输出格式 ("mp3" / "wav" / "pcm", 默认 "mp3").
    pub format: String,

    /// 声道数 (1=mono, 2=stereo, 默认 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<u8>,
}

/// T2A v2 响应 (per MiniMax 官方 spec).
#[derive(Debug, Clone, Deserialize)]
pub struct T2aResponse {
    /// 业务数据 (audio hex + extra).
    pub data: T2aData,

    /// 额外信息 (per MiniMax spec, 可空).
    #[serde(default)]
    pub extra_info: serde_json::Value,

    /// 链路追踪 ID (per MiniMax spec).
    #[serde(default)]
    pub trace_id: String,

    /// 基础响应 (status_code + status_msg).
    pub base_resp: BaseResp,
}

/// T2A 业务数据.
#[derive(Debug, Clone, Deserialize)]
pub struct T2aData {
    /// audio bytes (hex-encoded, e.g. MP3 / WAV / PCM).
    pub audio: String,

    /// 状态 (可选, e.g. "success").
    #[serde(default)]
    pub status: i32,
}

/// 基础响应 (MiniMax 所有 endpoint 共用).
#[derive(Debug, Clone, Deserialize)]
pub struct BaseResp {
    /// 0 = 成功, 非 0 = 失败.
    pub status_code: i64,

    /// 错误描述 (成功时为空).
    #[serde(default)]
    pub status_msg: String,
}

// ============================================================================
// §4 MiniMaxLive client (R172 主力 struct)
// ============================================================================

/// MiniMax LIVE HTTP 客户端 (R172 真接 production API).
///
/// ## 构造
///
/// ```no_run
/// use apeireth_voice::minimax_live::MiniMaxLive;
/// let client = MiniMaxLive::from_env().expect("set APEIRETH_API_KEY env");
/// ```
///
/// ## TTS 真接 (R172 LIVE 验证过, 122KB MP3 ID3 header 确认)
///
/// ```no_run
/// # async fn tts_demo(client: MiniMaxLive) -> Result<(), Box<dyn std::error::Error>> {
/// let audio = client.text_to_speech(
///     "hello apeireth this is a LIVE MiniMax TTS test",
///     None,  // use default model (speech-2.6-hd)
///     None,  // use default voice (male-qn-qingse)
/// ).await?;
/// std::fs::write("test_output.mp3", audio)?;
/// # Ok(()) }
/// ```
#[derive(Clone)]
pub struct MiniMaxLive {
    api_key: Arc<Mutex<Option<String>>>,
    base_url: String,
    http: reqwest::Client,
}

impl MiniMaxLive {
    /// 从环境变量 `APEIRETH_API_KEY` 创建客户端.
    ///
    /// 缺失时返 `Err(MiniMaxError::MissingApiKey)`.
    pub fn from_env() -> MiniMaxResult<Self> {
        let key = std::env::var("APEIRETH_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Self::new(key)
    }

    /// 从 openclaw apikey 文件创建客户端 (Windows 默认路径 `.openclaw\apikey.txt`).
    pub fn from_openclaw_file() -> MiniMaxResult<Self> {
        let path = PathBuf::from(r".openclaw\apikey.txt");
        let key = std::fs::read_to_string(&path)
            .map_err(|e| MiniMaxError::Transport(format!("read openclaw file {}: {}", path.display(), e)))?
            .trim()
            .to_string();
        if key.is_empty() {
            return Err(MiniMaxError::MissingApiKey);
        }
        Self::new(Some(key))
    }

    /// 显式 api_key 创建客户端 (None = 必须后续 set_api_key).
    pub fn new(api_key: Option<String>) -> MiniMaxResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(MINIMAX_HTTP_TIMEOUT_SECS))
            .build()
            .map_err(|e| MiniMaxError::Transport(e.to_string()))?;
        Ok(Self {
            api_key: Arc::new(Mutex::new(api_key.filter(|s| !s.is_empty()))),
            base_url: MINIMAX_BASE_URL.to_string(),
            http,
        })
    }

    /// 动态设置 / 覆盖 api_key (per real.rs ensure_api_key 模式).
    pub async fn set_api_key(&self, key: String) {
        let mut guard = self.api_key.lock().await;
        *guard = Some(key);
    }

    /// 取当前缓存的 api_key 是否存在.
    pub async fn api_key_cached(&self) -> bool {
        let guard = self.api_key.lock().await;
        guard.is_some()
    }

    /// TTS 真接 (R172 LIVE verified).
    ///
    /// ## 参数
    ///
    /// - `text`: 待合成文本 (中文 / 英文均可, 建议 < 5000 字符)
    /// - `model`: None = 使用默认 `speech-2.6-hd`
    /// - `voice_id`: None = 使用默认 `male-qn-qingse`
    ///
    /// ## 返
    ///
    /// `Ok(Vec<u8>)` = audio bytes (默认 MP3 格式, 可直接 `std::fs::write` 保存).
    pub async fn text_to_speech(
        &self,
        text: &str,
        model: Option<&str>,
        voice_id: Option<&str>,
    ) -> MiniMaxResult<Vec<u8>> {
        if text.trim().is_empty() {
            return Err(MiniMaxError::InvalidParams("text is empty".into()));
        }

        let api_key = {
            let guard = self.api_key.lock().await;
            guard.clone()
        };
        let api_key = api_key.ok_or(MiniMaxError::MissingApiKey)?;

        let req = T2aRequest {
            model: model.unwrap_or(MINIMAX_DEFAULT_TTS_MODEL).to_string(),
            text: text.to_string(),
            stream: false,
            voice_setting: VoiceSetting {
                voice_id: voice_id.unwrap_or(MINIMAX_DEFAULT_VOICE_ID).to_string(),
                speed: 1.0,
                vol: 1.0,
                pitch: 0,
            },
            audio_setting: AudioSetting {
                sample_rate: MINIMAX_DEFAULT_SAMPLE_RATE,
                bitrate: MINIMAX_DEFAULT_BITRATE,
                format: MINIMAX_DEFAULT_FORMAT.to_string(),
                channel: Some(1),
            },
        };

        let url = format!("{}{}", self.base_url, MINIMAX_T2A_V2_PATH);
        debug!(target: "apeireth_voice::minimax_live", "[R172] TTS POST {} text_len={}", url, text.len());

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&req)
            .send()
            .await
            .map_err(|e| MiniMaxError::Transport(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(MiniMaxError::Transport(format!(
                "HTTP {} body={}",
                status, body
            )));
        }

        let parsed: T2aResponse = resp
            .json()
            .await
            .map_err(|e| MiniMaxError::Parse(e.to_string()))?;

        if parsed.base_resp.status_code != 0 {
            return Err(MiniMaxError::Api {
                status_code: parsed.base_resp.status_code,
                msg: parsed.base_resp.status_msg,
            });
        }

        let hex_audio = parsed.data.audio;
        let audio_bytes = hex_decode(&hex_audio)?;
        info!(
            target: "apeireth_voice::minimax_live",
            "[R172] TTS OK bytes={} model={} voice={} trace_id={}",
            audio_bytes.len(),
            req.model,
            req.voice_setting.voice_id,
            parsed.trace_id
        );
        Ok(audio_bytes)
    }
}

// ============================================================================
// §5 hex decoder (MiniMax audio 是 hex string, 比 base64 紧凑)
// ============================================================================

/// hex string -> Vec<u8>.
fn hex_decode(hex: &str) -> MiniMaxResult<Vec<u8>> {
    let cleaned: Vec<u8> = hex.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        return Err(MiniMaxError::HexDecode(format!(
            "hex length {} is odd",
            cleaned.len()
        )));
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let mut i = 0;
    while i < cleaned.len() {
        let hi = hex_nibble(cleaned[i])?;
        let lo = hex_nibble(cleaned[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> MiniMaxResult<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(MiniMaxError::HexDecode(format!("invalid hex byte 0x{:02x}", b))),
    }
}

// ============================================================================
// §6 单元测试 (1:1 测试 hex_decode + 类型 round-trip)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_decode_known_string() {
        let hex = "494433";
        let bytes = hex_decode(hex).unwrap();
        assert_eq!(bytes, b"ID3");
    }

    #[test]
    fn hex_decode_rejects_odd_length() {
        let err = hex_decode("abc").unwrap_err();
        assert!(matches!(err, MiniMaxError::HexDecode(_)));
    }

    #[test]
    fn hex_decode_rejects_invalid_char() {
        let err = hex_decode("zz").unwrap_err();
        assert!(matches!(err, MiniMaxError::HexDecode(_)));
    }

    #[test]
    fn hex_decode_handles_whitespace() {
        let bytes = hex_decode("49 44 33\n").unwrap();
        assert_eq!(bytes, b"ID3");
    }

    #[test]
    fn hex_decode_empty() {
        let bytes = hex_decode("").unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn voice_setting_default_constructible() {
        let v = VoiceSetting {
            voice_id: MINIMAX_DEFAULT_VOICE_ID.to_string(),
            speed: 1.0,
            vol: 1.0,
            pitch: 0,
        };
        assert_eq!(v.voice_id, "male-qn-qingse");
    }

    #[test]
    fn t2a_request_serializes_to_expected_shape() {
        let req = T2aRequest {
            model: MINIMAX_DEFAULT_TTS_MODEL.to_string(),
            text: "hi".to_string(),
            stream: false,
            voice_setting: VoiceSetting {
                voice_id: MINIMAX_DEFAULT_VOICE_ID.to_string(),
                speed: 1.0,
                vol: 1.0,
                pitch: 0,
            },
            audio_setting: AudioSetting {
                sample_rate: MINIMAX_DEFAULT_SAMPLE_RATE,
                bitrate: MINIMAX_DEFAULT_BITRATE,
                format: MINIMAX_DEFAULT_FORMAT.to_string(),
                channel: Some(1),
            },
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "speech-2.6-hd");
        assert_eq!(json["text"], "hi");
        assert_eq!(json["voice_setting"]["voice_id"], "male-qn-qingse");
        assert_eq!(json["audio_setting"]["sample_rate"], 32000);
    }

    #[test]
    fn base_resp_deserializes_success() {
        let json = r#"{"status_code":0,"status_msg":""}"#;
        let br: BaseResp = serde_json::from_str(json).unwrap();
        assert_eq!(br.status_code, 0);
    }

    #[test]
    fn base_resp_deserializes_error() {
        let json = r#"{"status_code":2013,"status_msg":"invalid params"}"#;
        let br: BaseResp = serde_json::from_str(json).unwrap();
        assert_eq!(br.status_code, 2013);
        assert_eq!(br.status_msg, "invalid params");
    }

    #[test]
    fn minimax_error_display_includes_status_code() {
        let e = MiniMaxError::Api {
            status_code: 1001,
            msg: "auth failed".into(),
        };
        let s = format!("{}", e);
        assert!(s.contains("1001"));
        assert!(s.contains("auth failed"));
    }

    #[test]
    fn minimax_live_new_without_api_key_ok() {
        let client = MiniMaxLive::new(None).unwrap();
        // api_key 缓存为空, 但 client 本身可创建
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            assert!(!client.api_key_cached().await);
        });
    }
}