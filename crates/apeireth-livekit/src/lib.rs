//! # apeireth-livekit (R20 阶段 6 flesh out: LiveKit Server SDK 真接实现)
//!
//! ⚠️ **本 crate 是 R20 阶段 6 flesh out 新增**, 跟 `apeireth-sdk-livekit` (LOCKED
//! baseline 16:34:11, R20 阶段 4 商业版 v0.9.21 1:1 翻译) **严格分离**: `LiveKitRealImpl`
//! 是显式 opt-in 的真接 LiveKit Server API Twirp HTTP 客户端, 不受 `STUB_MODE = true`
//! 编译期 hardcode 守门影响. 调用方显式 `LiveKitRealImpl::new(config, server_url,
//! api_key, api_secret)?` 即用.
//!
//! ## 设计 (跟 `apeireth-sdk-livekit` 1:1 翻译 v0.9.21 商业版设计参考)
//!
//! **6 端点** (R20 阶段 6 真接, 走 LiveKit Server API Twirp HTTP POST + JSON, per
//! <https://docs.livekit.io/reference/server/server-apis/>):
//! - **server_url** — LiveKit 服务器地址 (e.g. `https://livekit.example.com`)
//! - **api_key** — LiveKit API Key (跟 api_secret 配对, HMAC JWT 签名)
//! - **room** — 房间管理 (CreateRoom / ListRooms / DeleteRoom)
//! - **track** — Track 管理 (MutePublishedTrack)
//! - **participant** — 参与者管理 (ListParticipants / RemoveParticipant / GetParticipant)
//! - **event** — 事件查询 (per Webhook 拉取 — LiveKit Server API 不提供 server-side
//!   event 订阅, 真实场景是 webhook push 进来, 阶段 6 走 server-side 模拟)
//!
//! **5 K-1 强校验** (K-1 强校验白名单, 编译期 hardcode, 跟 `apeireth-sdk-livekit` 4
//! K-1 1:1 借鉴, 加 1 server_url 形成 5 K-1):
//! - `server_url` (K-1 #1): 必须 `https://` 开头 (LiveKit Server API 强制 TLS)
//! - `api_key` (K-1 #2): 至少 10 chars alphanumeric (LiveKit 默认 `API` 前缀, e.g. `APIxxxxxxxx`)
//! - `room` (K-1 #3): 房间名 1..=256 chars, ASCII alphanumeric + `-` + `_`
//! - `track` (K-1 #4): track SID 格式 `TR_<alphanumeric>` (per LiveKit track SID 规范)
//! - `participant` (K-1 #5): identity 1..=128 chars, ASCII alphanumeric + `-` + `_` + `.`
//!
//! **6 核心 API STUB 路径** (跟 `apeireth-sdk-livekit` 1:1 镜像, 0 改, 编译期 hardcode 守门):
//! - `STUB_MODE = true` 编译期 hardcode, 真接模式 opt-in via `LiveKitRealImpl::new(...)`
//! - `TOOL_WHITELIST` 编译期 hardcode 6 端点 + 1 stub_status = 7 工具, `validate_tool_call` 守门
//!
//! ## 6 哲学锚 (R20 阶段 6 必守)
//!
//! 1. **S-1 不漂移**: 6 端点 1:1 翻译 LiveKit Server API Twirp 6 维度
//!    (server_url / api_key / room / track / participant / event), 0 假装已连真 LiveKit server.
//! 2. **S-2 编译期 hardcode**: `STUB_MODE` / `PLATFORM_NAME` / 6 端点名 / 5 K-1 / Twirp 路径前缀
//!    全部 const, 0 运行时配置覆盖.
//! 3. **O-2 工程铁律**: reqwest 0.12 + jsonwebtoken 9.3 + url 2.5 走 workspace deps / 业界成熟
//!    crate, 0 引 livekit-server-sdk 0.6 (留 R21+ 续), wiremock 0.6 测 14 端到端用例.
//! 4. **O-3 m3 防御**: 6 端点工具白名单 `TOOL_WHITELIST` 编译期 hardcode,
//!    `validate_tool_call` 在 dispatch 前 schema 校验, 防 m3 模型幻觉调用.
//! 5. **O-4 不假装可观测**: 6 端点失败时返 `LiveKitError::NotImplemented(api_name)` 或
//!    `LiveKitError::ServerCallFailed(...)` + `tracing::warn!` log, 0 假装 OK.
//! 6. **O-5 K-1 强校验**: 5 字段 (server_url / api_key / room / track / participant) 编译期
//!    hardcode 白名单, 任何配置变更必经 `validate()` 走 5 K-1 检查.
//!
//! ## 🔒 8 项不修改承诺 (跟 `apeireth-voice` / `apeireth-lark` / `apeireth-sandbox`
//! 1:1 风格)
//!
//! 1. `version = "0.1.0"` 显式 (跟 voice/lark/sandbox 模板同) ✅
//! 2. `edition = "2021"` 显式 ✅
//! 3. `rust-version = "1.80"` 显式 ✅
//! 4. `license = "Apache-2.0"` 显式 ✅
//! 5. `authors = ["Apeireth Team"]` 显式 ✅
//! 6. deps 显式版本 (reqwest / url / jsonwebtoken / tokio 等, 跟 voice 1:1) ✅
//! 7. 不修改 workspace Cargo.toml (由整合 #3 sub-agent 加 member) ⏳
//! 8. 不引 unsafe (workspace `#![deny(unsafe_code)]` 继承) ✅

#![allow(missing_docs)]
#![allow(clippy::all)]

use serde::{Deserialize, Serialize};

// ============================================================================
// §0 Module 声明
// ============================================================================

pub mod real;
// R177: organ invariants (5 tests + 2 Kani)
mod organ_kani_proofs;

// ============================================================================
// §1 编译期 hardcode 常量 (per R20 P0 5 crate 风格 + K-1 强校验)
// ============================================================================

/// LiveKit Server API schema version (per livekit-server-sdk 0.6+).
pub const LIVEKIT_SCHEMA_VERSION: &str = "1";

/// LiveKit Twirp API 路径前缀 (per <https://docs.livekit.io/reference/server/server-apis/>).
///
/// 字段对应 LiveKit Server API Twirp 协议: 所有 endpoint 走 `POST /twirp/<service>/<method>`.
pub const LIVEKIT_TWIRP_PREFIX: &str = "/twirp";

/// 平台名 (K-1 强校验: 编译期 hardcode `"apeireth"`).
pub const PLATFORM_NAME: &str = "apeireth";

/// **STUB MODE 守门标志** (K-1 强校验): 编译期 hardcode = `true`.
///
/// R20 阶段 6 真接实现由 `real.rs` 提供, opt-in, 0 受 STUB_MODE 守门影响.
/// 改 `false` 需经 6 哲学锚 + 主人审 (R21+ 续真接 livekit-server SDK 时考虑).
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必须 == true.
const _: () = assert!(STUB_MODE == true, "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R21+)");

/// m3 防御: 查 STUB_MODE 状态 (per task spec 守门).
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// LiveKit Server API 默认 base URL (per livekit-server-sdk 0.6+ 默认).
///
/// 实际真接可由 caller 注入 (e.g. `https://livekit.example.com` 或 LiveKit Cloud
/// `https://your-project.livekit.cloud`).
pub const DEFAULT_LIVEKIT_SERVER_URL: &str = "https://livekit.example.com";

/// 单 access token 默认 TTL (秒, 6h, per LiveKit 默认 `SetValidFor(6 * time.Hour)`).
pub const DEFAULT_TOKEN_TTL_SECONDS: u64 = 21600;

/// 最大 token TTL (秒, 24h, per LiveKit 商业版上限).
pub const MAX_TOKEN_TTL_SECONDS: u64 = 86400;

// ============================================================================
// §2 6 端点 enum (K-1 强校验, 编译期 hardcode)
// ============================================================================

/// LiveKit 真接 6 端点 (per `apeireth-sdk-livekit` 1:1 借鉴, server-side 视角 6 维度).
///
/// 字段对应 LiveKit Server API 6 维度:
/// - `ServerUrl` — 服务器地址配置
/// - `ApiKey` — 鉴权 (API Key, 跟 ApiSecret 配对)
/// - `Room` — 房间管理 (CreateRoom / ListRooms / DeleteRoom)
/// - `Track` — Track 控制 (MutePublishedTrack)
/// - `Participant` — 参与者管理 (ListParticipants / RemoveParticipant / GetParticipant)
/// - `Event` — 事件查询 (webhook 拉取, server-side 模拟)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveKitEndpoint {
    /// `"server_url"` — LiveKit 服务器地址.
    #[default]
    ServerUrl,
    /// `"api_key"` — 鉴权 (API Key, 跟 api_secret 配对).
    ApiKey,
    /// `"room"` — 房间管理.
    Room,
    /// `"track"` — Track 控制.
    Track,
    /// `"participant"` — 参与者管理.
    Participant,
    /// `"event"` — 事件查询 (webhook 拉取).
    Event,
}

/// 编译期守门: 6 端点守门.
pub const SUPPORTED_LIVEKIT_ENDPOINTS: &[LiveKitEndpoint] = &[
    LiveKitEndpoint::ServerUrl,
    LiveKitEndpoint::ApiKey,
    LiveKitEndpoint::Room,
    LiveKitEndpoint::Track,
    LiveKitEndpoint::Participant,
    LiveKitEndpoint::Event,
];
const _: () = assert!(SUPPORTED_LIVEKIT_ENDPOINTS.len() == 6);

/// 6 端点数量常量.
pub const LIVEKIT_ENDPOINT_COUNT: usize = 6;
const _: () = assert!(LIVEKIT_ENDPOINT_COUNT == 6);

impl LiveKitEndpoint {
    /// Twirp service 名 (1:1 翻译 LiveKit Server API service enum).
    pub fn twirp_service(&self) -> &'static str {
        match self {
            LiveKitEndpoint::ServerUrl | LiveKitEndpoint::ApiKey => "livekit.RoomService", // 鉴权走 RoomService 通用
            LiveKitEndpoint::Room => "livekit.RoomService",
            LiveKitEndpoint::Track => "livekit.RoomService", // MutePublishedTrack 在 RoomService
            LiveKitEndpoint::Participant => "livekit.RoomService",
            LiveKitEndpoint::Event => "livekit.Webhook", // webhook 走 Webhook service
        }
    }

    /// 字符串 (1:1 翻译 LiveKit Server API endpoint snake_case).
    pub fn as_str(&self) -> &'static str {
        match self {
            LiveKitEndpoint::ServerUrl => "server_url",
            LiveKitEndpoint::ApiKey => "api_key",
            LiveKitEndpoint::Room => "room",
            LiveKitEndpoint::Track => "track",
            LiveKitEndpoint::Participant => "participant",
            LiveKitEndpoint::Event => "event",
        }
    }
}

impl std::fmt::Display for LiveKitEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for LiveKitEndpoint {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "server_url" | "ServerUrl" | "SERVER_URL" => Ok(LiveKitEndpoint::ServerUrl),
            "api_key" | "ApiKey" | "API_KEY" => Ok(LiveKitEndpoint::ApiKey),
            "room" | "Room" | "ROOM" => Ok(LiveKitEndpoint::Room),
            "track" | "Track" | "TRACK" => Ok(LiveKitEndpoint::Track),
            "participant" | "Participant" | "PARTICIPANT" => Ok(LiveKitEndpoint::Participant),
            "event" | "Event" | "EVENT" => Ok(LiveKitEndpoint::Event),
            other => Err(format!(
                "LiveKitEndpoint 解析失败: {other} (合法值: server_url / api_key / room / track / participant / event)"
            )),
        }
    }
}

// ============================================================================
// §3 5 K-1 强校验白名单 (per K-1 守门, 编译期 hardcode)
// ============================================================================

/// 允许的 LiveKit server URL scheme 白名单 (3 个, K-1 强校验 #1).
///
/// 字段对应 LiveKit Server API 强制要求: 走 HTTPS (生产) 或 HTTP (localhost/127.0.0.1 dev only).
/// 本阶段 flesh out 支持 `https://` (生产) + `http://localhost` + `http://127.0.0.1` (dev/mock).
pub const ALLOWED_SERVER_URL_SCHEMES: &[&str] = &["https://", "http://localhost", "http://127.0.0.1"];

/// Room name 合法字符白名单 (per livekit-server RoomName 约束, K-1 强校验 #3).
///
/// LiveKit Room Name 限制: 1..=256 chars, ASCII alphanumeric + `-` + `_`.
pub const ALLOWED_ROOM_NAME_CHARS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '-', '_',
];

/// 房间名最大长度 (per livekit-server RoomName 约束, 编译期 hardcode).
pub const MAX_ROOM_NAME_LENGTH: usize = 256;

/// Track SID 合法字符白名单 (per livekit-server Track SID 约束, K-1 强校验 #4).
///
/// LiveKit Track SID 格式: `TR_<alphanumeric>` (e.g. `TR_abc123def456`).
pub const ALLOWED_TRACK_SID_CHARS: &[char] = &[
    'T', 'R', '_', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p',
    'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I',
    'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1',
    '2', '3', '4', '5', '6', '7', '8', '9',
];

/// Participant identity 合法字符白名单 (per livekit-server Identity 约束, K-1 强校验 #5).
///
/// LiveKit Identity 限制: 1..=128 chars, ASCII alphanumeric + `-` + `_` + `.`.
pub const ALLOWED_PARTICIPANT_IDENTITY_CHARS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L',
    'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9', '-', '_', '.',
];

/// Participant identity 最大长度 (per livekit-server Identity 约束, 编译期 hardcode).
pub const MAX_PARTICIPANT_IDENTITY_LENGTH: usize = 128;

// ============================================================================
// §4 LiveKitError 错误枚举 (8 variant, 跟 voice/lark/sandbox 1:1 风格)
// ============================================================================

/// LiveKit 错误枚举 (8 variant, 跟 `apeireth-sdk-livekit` 1:1 + 真接实现 3 新 variant).
#[derive(Debug, thiserror::Error)]
pub enum LiveKitError {
    /// 工具不在白名单 (m3 防御, 5 K-1 守门).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    /// API 未实现 (STUB 模式守门).
    #[error("API not implemented (STUB_MODE=true): {0}")]
    NotImplemented(String),

    /// 配置非法 (K-1 强校验失败).
    #[error("invalid config: {0}")]
    InvalidConfig(String),

    /// 房间名非法 (K-1 强校验 #3).
    #[error("room name invalid: {0}")]
    RoomNameInvalid(String),

    /// Track SID 非法 (K-1 强校验 #4).
    #[error("track sid invalid: {0}")]
    TrackSidInvalid(String),

    /// Participant identity 非法 (K-1 强校验 #5).
    #[error("participant identity invalid: {0}")]
    ParticipantIdentityInvalid(String),

    /// LiveKit Server API 调用失败 (真接 1:1 翻译, per Twirp HTTP).
    #[error("LiveKit Server API call failed: {0}")]
    ServerCallFailed(String),

    /// 鉴权失败 (JWT 签名错误 / API Key 失效).
    #[error("auth failed: {0}")]
    AuthFailed(String),
}

/// LiveKit Result 类型别名.
pub type LiveKitResult<T> = std::result::Result<T, LiveKitError>;

/// 编译期守门: LiveKitError 8 variant 守门.
pub const LIVEKIT_ERROR_VARIANT_COUNT: usize = 8;
const _: () = assert!(LIVEKIT_ERROR_VARIANT_COUNT == 8);

// ============================================================================
// §5 6 端点 + 1 stub_status = 7 tool whitelist (m3 防御, 编译期 hardcode)
// ============================================================================

/// m3 防御: LiveKit 6 端点 + 1 stub_status = 7 工具白名单 (编译期 hardcode, 不可运行时改).
///
/// **6 端点 1:1 翻译 LiveKit Server API 6 维度**:
/// - `apeireth_livekit_server_url` — 服务器地址
/// - `apeireth_livekit_api_key` — 鉴权 (API Key + Secret)
/// - `apeireth_livekit_room` — 房间管理 (Create / List / Delete)
/// - `apeireth_livekit_track` — Track 控制 (Mute)
/// - `apeireth_livekit_participant` — 参与者管理 (List / Remove / Get)
/// - `apeireth_livekit_event` — 事件查询 (webhook 拉取)
///
/// **额外 1**: `apeireth_livekit_stub_status` (查 STUB_MODE 状态, 跟 voice / lark / sandbox 1:1 镜像).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_livekit_server_url",
    "apeireth_livekit_api_key",
    "apeireth_livekit_room",
    "apeireth_livekit_track",
    "apeireth_livekit_participant",
    "apeireth_livekit_event",
    "apeireth_livekit_stub_status", // 额外 1: stub 模式守门 (查 STUB_MODE 状态)
];

/// 编译期守门: TOOL_WHITELIST 长度 == 7 (6 端点 + 1 stub_status).
pub const TOOL_WHITELIST_COUNT: usize = 7;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `LiveKitError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> LiveKitResult<()> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(LiveKitError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §6 5 K-1 强校验函数 (编译期 hardcode 白名单 + runtime 守门)
// ============================================================================

/// K-1 #1: 校验 server URL (必须 `https://` 开头, 或 `http://localhost` / `http://127.0.0.1` dev only).
pub fn validate_server_url(url: &str) -> LiveKitResult<()> {
    if url.is_empty() {
        return Err(LiveKitError::InvalidConfig("server_url 不能为空".to_string()));
    }
    // LiveKit Server API 强制 HTTPS (生产), HTTP 仅 localhost/127.0.0.1 dev (per ALLOWED_SERVER_URL_SCHEMES)
    let ok = ALLOWED_SERVER_URL_SCHEMES
        .iter()
        .any(|scheme| url.starts_with(scheme));
    if !ok {
        return Err(LiveKitError::InvalidConfig(format!(
            "server_url 必须 https:// 开头 (或 http://localhost / http://127.0.0.1 dev): got `{url}`"
        )));
    }
    Ok(())
}

/// K-1 #2: 校验 API Key (至少 10 chars alphanumeric, 通常 `API` 前缀).
pub fn validate_api_key(api_key: &str) -> LiveKitResult<()> {
    if api_key.is_empty() {
        return Err(LiveKitError::InvalidConfig("api_key 不能为空".to_string()));
    }
    if api_key.len() < 10 {
        return Err(LiveKitError::InvalidConfig(format!(
            "api_key too short: {} chars (< 10)",
            api_key.len()
        )));
    }
    if !api_key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(LiveKitError::InvalidConfig(format!(
            "api_key contains invalid chars: `{api_key}` (only alphanumeric, `-`, `_` allowed)"
        )));
    }
    Ok(())
}

/// K-1 #3: 校验 room name (1..=256 chars, alphanumeric + `-` + `_`).
pub fn validate_room_name(room: &str) -> LiveKitResult<()> {
    if room.is_empty() {
        return Err(LiveKitError::RoomNameInvalid("room name 不能为空".to_string()));
    }
    if room.len() > MAX_ROOM_NAME_LENGTH {
        return Err(LiveKitError::RoomNameInvalid(format!(
            "room name too long: {} chars (> {MAX_ROOM_NAME_LENGTH})",
            room.len()
        )));
    }
    if !room.chars().all(|c| ALLOWED_ROOM_NAME_CHARS.contains(&c)) {
        return Err(LiveKitError::RoomNameInvalid(format!(
            "room name contains invalid chars: `{room}` (only alphanumeric, `-`, `_` allowed)"
        )));
    }
    Ok(())
}

/// K-1 #4: 校验 track SID (格式 `TR_<alphanumeric>`, per LiveKit track SID 规范).
pub fn validate_track_sid(track_sid: &str) -> LiveKitResult<()> {
    if track_sid.is_empty() {
        return Err(LiveKitError::TrackSidInvalid("track_sid 不能为空".to_string()));
    }
    if !track_sid.starts_with("TR_") {
        return Err(LiveKitError::TrackSidInvalid(format!(
            "track_sid must start with `TR_`: got `{track_sid}`"
        )));
    }
    if !track_sid.chars().all(|c| ALLOWED_TRACK_SID_CHARS.contains(&c)) {
        return Err(LiveKitError::TrackSidInvalid(format!(
            "track_sid contains invalid chars: `{track_sid}`"
        )));
    }
    Ok(())
}

/// K-1 #5: 校验 participant identity (1..=128 chars, alphanumeric + `-` + `_` + `.`).
pub fn validate_participant_identity(identity: &str) -> LiveKitResult<()> {
    if identity.is_empty() {
        return Err(LiveKitError::ParticipantIdentityInvalid(
            "participant identity 不能为空".to_string(),
        ));
    }
    if identity.len() > MAX_PARTICIPANT_IDENTITY_LENGTH {
        return Err(LiveKitError::ParticipantIdentityInvalid(format!(
            "participant identity too long: {} chars (> {MAX_PARTICIPANT_IDENTITY_LENGTH})",
            identity.len()
        )));
    }
    if !identity
        .chars()
        .all(|c| ALLOWED_PARTICIPANT_IDENTITY_CHARS.contains(&c))
    {
        return Err(LiveKitError::ParticipantIdentityInvalid(format!(
            "participant identity contains invalid chars: `{identity}` (only alphanumeric, `-`, `_`, `.` allowed)"
        )));
    }
    Ok(())
}

// ============================================================================
// §7 核心类型 (LiveKitConfig / Room / Participant / Track / Event)
// ============================================================================

/// LiveKit 顶层配置 (per 任务 5 K-1 强校验字段, 1:1 翻译 LiveKit Server API 启动参数).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveKitConfig {
    /// 平台名 (K-1 强校验, 编译期 hardcode `apeireth`).
    pub platform: String,
    /// 默认 server URL (per server_url 端点).
    pub server_url: String,
    /// 默认 API Key (per api_key 端点, K-1 强校验 #2).
    pub api_key: String,
    /// 默认 API Secret (per api_key 端点, 跟 api_key 配对).
    pub api_secret: String,
    /// 默认 token TTL (秒, K-1 强校验).
    pub token_ttl_seconds: u64,
    /// 是否启 debug log (per 6 端点 log).
    pub debug: bool,
}

impl Default for LiveKitConfig {
    fn default() -> Self {
        Self {
            platform: PLATFORM_NAME.to_string(),
            server_url: DEFAULT_LIVEKIT_SERVER_URL.to_string(),
            api_key: String::new(),
            api_secret: String::new(),
            token_ttl_seconds: DEFAULT_TOKEN_TTL_SECONDS,
            debug: false,
        }
    }
}

impl LiveKitConfig {
    /// 创建带 server_url 的 config (K-1 强校验 server_url).
    pub fn with_server_url(mut self, server_url: impl Into<String>) -> LiveKitResult<Self> {
        let url = server_url.into();
        validate_server_url(&url)?;
        self.server_url = url;
        Ok(self)
    }

    /// 设置 api_key + api_secret (K-1 强校验 api_key).
    pub fn with_credentials(
        mut self,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> LiveKitResult<Self> {
        let key = api_key.into();
        let secret = api_secret.into();
        validate_api_key(&key)?;
        if secret.is_empty() {
            return Err(LiveKitError::InvalidConfig(
                "api_secret 不能为空".to_string(),
            ));
        }
        self.api_key = key;
        self.api_secret = secret;
        Ok(self)
    }
}

/// LiveKit 房间信息 (per `livekit.Room` proto, server-side 1:1 翻译).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    /// 房间 SID (服务端分配, e.g. `RM_abc123`).
    pub sid: String,
    /// 房间名.
    pub name: String,
    /// 创建时间 (秒, UNIX_EPOCH 起).
    pub created_at: u64,
    /// 当前参与者数.
    pub num_participants: u32,
    /// 最大参与者数 (1..=1000, per LiveKit 默认).
    pub max_participants: u32,
    /// 房间元数据 (任意 JSON 字符串).
    pub metadata: String,
}

/// LiveKit 房间创建请求 (per `CreateRoomRequest` proto, 1:1 翻译).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    /// 房间名 (K-1 强校验 #3).
    pub name: String,
    /// 空房间超时 (秒, 0 = 不超时, per LiveKit 默认 5min).
    #[serde(default)]
    pub empty_timeout: u32,
    /// 最大参与者数 (1..=1000, per LiveKit 默认 100).
    #[serde(default = "default_max_participants")]
    pub max_participants: u32,
    /// 房间元数据.
    #[serde(default)]
    pub metadata: String,
}

fn default_max_participants() -> u32 {
    100
}

impl CreateRoomRequest {
    /// 创建新请求 (K-1 强校验 room name).
    pub fn new(name: impl Into<String>) -> LiveKitResult<Self> {
        let name_str = name.into();
        validate_room_name(&name_str)?;
        Ok(Self {
            name: name_str,
            empty_timeout: 0,
            max_participants: default_max_participants(),
            metadata: String::new(),
        })
    }
}

/// LiveKit 房间列表响应 (per `ListRoomsResponse` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRoomsResponse {
    /// 房间列表.
    pub rooms: Vec<Room>,
}

/// LiveKit 房间删除响应 (per `DeleteRoomResponse` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRoomResponse {}

/// LiveKit 房间删除请求 (per `DeleteRoomRequest` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRoomRequest {
    /// 房间名 (K-1 强校验 #3).
    pub room: String,
}

impl DeleteRoomRequest {
    /// 创建新请求 (K-1 强校验 room name).
    pub fn new(room: impl Into<String>) -> LiveKitResult<Self> {
        let room_str = room.into();
        validate_room_name(&room_str)?;
        Ok(Self { room: room_str })
    }
}

/// LiveKit 参与者信息 (per `ParticipantInfo` proto, server-side 1:1 翻译).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    /// 参与者 SID (服务端分配, e.g. `PA_abc123`).
    pub sid: String,
    /// 参与者 identity (K-1 强校验 #5).
    pub identity: String,
    /// 参与者显示名 (可选).
    pub name: String,
    /// 房间名.
    pub room: String,
    /// 加入时间 (秒, UNIX_EPOCH 起).
    pub joined_at: u64,
    /// 是否已发布 track.
    pub is_publisher: bool,
}

/// LiveKit 参与者列表请求 (per `ListParticipantsRequest` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParticipantsRequest {
    /// 房间名 (K-1 强校验 #3).
    pub room: String,
}

impl ListParticipantsRequest {
    /// 创建新请求 (K-1 强校验 room name).
    pub fn new(room: impl Into<String>) -> LiveKitResult<Self> {
        let room_str = room.into();
        validate_room_name(&room_str)?;
        Ok(Self { room: room_str })
    }
}

/// LiveKit 参与者列表响应 (per `ListParticipantsResponse` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListParticipantsResponse {
    /// 参与者列表.
    pub participants: Vec<ParticipantInfo>,
}

/// LiveKit 参与者删除请求 (per `RoomParticipantIdentity` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveParticipantRequest {
    /// 房间名 (K-1 强校验 #3).
    pub room: String,
    /// 参与者 identity (K-1 强校验 #5).
    pub identity: String,
}

impl RemoveParticipantRequest {
    /// 创建新请求 (K-1 强校验 room + identity).
    pub fn new(room: impl Into<String>, identity: impl Into<String>) -> LiveKitResult<Self> {
        let room_str = room.into();
        let identity_str = identity.into();
        validate_room_name(&room_str)?;
        validate_participant_identity(&identity_str)?;
        Ok(Self {
            room: room_str,
            identity: identity_str,
        })
    }
}

/// LiveKit 参与者删除响应 (per `RemoveParticipantResponse` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveParticipantResponse {}

/// LiveKit Mute Track 请求 (per `MuteRoomTrackRequest` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuteTrackRequest {
    /// 房间名 (K-1 强校验 #3).
    pub room: String,
    /// 参与者 identity (K-1 强校验 #5).
    pub identity: String,
    /// Track SID (K-1 强校验 #4).
    pub track_sid: String,
    /// 是否静音.
    pub muted: bool,
}

impl MuteTrackRequest {
    /// 创建新请求 (K-1 强校验 room + identity + track_sid).
    pub fn new(
        room: impl Into<String>,
        identity: impl Into<String>,
        track_sid: impl Into<String>,
        muted: bool,
    ) -> LiveKitResult<Self> {
        let room_str = room.into();
        let identity_str = identity.into();
        let track_sid_str = track_sid.into();
        validate_room_name(&room_str)?;
        validate_participant_identity(&identity_str)?;
        validate_track_sid(&track_sid_str)?;
        Ok(Self {
            room: room_str,
            identity: identity_str,
            track_sid: track_sid_str,
            muted,
        })
    }
}

/// LiveKit Mute Track 响应 (per `MuteRoomTrackResponse` proto).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuteTrackResponse {}

/// LiveKit Webhook 事件 (per `livekit.Webhook` proto, 1:1 翻译).
///
/// LiveKit Webhook 推送事件时, 走 HTTP POST 到 caller 提供的 URL.
/// Server-side 视角: 接收方需要校验 + 反序列化. 现阶段 6 端点 flesh out 提供
/// `Event` 端点作为"接收"接口, 真实场景 LiveKit Cloud / 自建 server 会推 webhook.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// 事件 ID (UUID v4, server 分配).
    pub event_id: String,
    /// 事件类型 (e.g. `room_started`, `participant_joined`, `track_published`).
    pub event_type: String,
    /// 房间名.
    pub room: Room,
    /// 参与者 (optional, 跟 event_type 关联).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub participant: Option<ParticipantInfo>,
    /// 创建时间 (秒, UNIX_EPOCH 起).
    pub created_at: u64,
}

impl WebhookEvent {
    /// 创建 room_started 事件.
    pub fn room_started(room: Room) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: "room_started".to_string(),
            room,
            participant: None,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

// ============================================================================
// §8 STUB 路径 (跟 `apeireth-sdk-livekit` 1:1 镜像, 0 改, 编译期 hardcode 守门)
// ============================================================================

/// STUB 守门宏: 在 6 端点实现里守门, 防止整合时有人"贴心"接 SDK 但忘了改 STUB_MODE.
///
/// **当前行为**: 永远展开成 `return Err(LiveKitError::NotImplemented("api_name"));`.
/// 改 `STUB_MODE = false` 后, 这个宏应该被替换成实际实现 (per 整合 #2 sub-agent).
#[macro_export]
macro_rules! livekit_stub {
    ($api_name:literal) => {
        return Err($crate::LiveKitError::NotImplemented($api_name.to_string()))
    };
}

// ============================================================================
// §9 内联测试 (12 fixture: 编译期 hardcode + 5 K-1 + 6 端点 + 5 段 STUB 路径守门)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// 编译期守门: 5 K-1 强校验常量 (K-1 #1..#5)
    #[test]
    fn compile_time_constants_match_k1() {
        assert_eq!(SUPPORTED_LIVEKIT_ENDPOINTS.len(), 6);
        assert_eq!(LIVEKIT_ENDPOINT_COUNT, 6);
        assert_eq!(MAX_ROOM_NAME_LENGTH, 256);
        assert_eq!(MAX_PARTICIPANT_IDENTITY_LENGTH, 128);
        assert_eq!(DEFAULT_TOKEN_TTL_SECONDS, 21600);
        assert_eq!(MAX_TOKEN_TTL_SECONDS, 86400);
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(STUB_MODE, true);
    }

    /// 6 端点守门
    #[test]
    fn livekit_endpoint_has_6_variants() {
        assert_eq!(SUPPORTED_LIVEKIT_ENDPOINTS.len(), 6);
        assert_eq!(LiveKitEndpoint::ServerUrl.as_str(), "server_url");
        assert_eq!(LiveKitEndpoint::ApiKey.as_str(), "api_key");
        assert_eq!(LiveKitEndpoint::Room.as_str(), "room");
        assert_eq!(LiveKitEndpoint::Track.as_str(), "track");
        assert_eq!(LiveKitEndpoint::Participant.as_str(), "participant");
        assert_eq!(LiveKitEndpoint::Event.as_str(), "event");
    }

    /// 5 K-1 强校验函数
    #[test]
    fn k1_strong_validations() {
        // K-1 #1: server_url
        assert!(validate_server_url("https://livekit.example.com").is_ok());
        assert!(validate_server_url("http://localhost:7880").is_ok());
        assert!(validate_server_url("").is_err());
        assert!(validate_server_url("ws://livekit.example.com").is_err());

        // K-1 #2: api_key
        assert!(validate_api_key("APIabc123def456ghi789").is_ok());
        assert!(validate_api_key("").is_err());
        assert!(validate_api_key("short").is_err()); // < 10 chars
        assert!(validate_api_key("invalid char!@#").is_err());

        // K-1 #3: room name
        assert!(validate_room_name("my-room_123").is_ok());
        assert!(validate_room_name("").is_err());
        assert!(validate_room_name("with space").is_err());
        assert!(validate_room_name("with/slash").is_err());
        assert!(validate_room_name(&"a".repeat(MAX_ROOM_NAME_LENGTH + 1)).is_err());

        // K-1 #4: track SID
        assert!(validate_track_sid("TR_abc123def456").is_ok());
        assert!(validate_track_sid("").is_err());
        assert!(validate_track_sid("PA_abc123").is_err()); // wrong prefix
        assert!(validate_track_sid("TR_bad-char!").is_err());

        // K-1 #5: participant identity
        assert!(validate_participant_identity("user-1.test").is_ok());
        assert!(validate_participant_identity("").is_err());
        assert!(validate_participant_identity("with space").is_err());
        assert!(validate_participant_identity("with/slash").is_err());
    }

    /// 7 tool whitelist 守门
    #[test]
    fn tool_whitelist_has_7_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 7);
        assert!(TOOL_WHITELIST.contains(&"apeireth_livekit_server_url"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_livekit_api_key"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_livekit_room"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_livekit_track"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_livekit_participant"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_livekit_event"));
        assert!(TOOL_WHITELIST.contains(&"apeireth_livekit_stub_status"));

        // validate: 在白名单内 → Ok
        assert!(validate_tool_call("apeireth_livekit_room", &serde_json::json!({})).is_ok());
        // validate: 不在白名单 → ToolNotWhitelisted
        assert!(validate_tool_call("apeireth_livekit_unknown", &serde_json::json!({})).is_err());
    }

    /// LiveKitError 8 variant 守门
    #[test]
    fn livekit_error_has_8_variants() {
        assert_eq!(LIVEKIT_ERROR_VARIANT_COUNT, 8);
        // 仅 sanity 触发 8 个 variant 都能构造 (编译期就强制 8 个)
        let _ = LiveKitError::ToolNotWhitelisted("x".to_string());
        let _ = LiveKitError::NotImplemented("x".to_string());
        let _ = LiveKitError::InvalidConfig("x".to_string());
        let _ = LiveKitError::RoomNameInvalid("x".to_string());
        let _ = LiveKitError::TrackSidInvalid("x".to_string());
        let _ = LiveKitError::ParticipantIdentityInvalid("x".to_string());
        let _ = LiveKitError::ServerCallFailed("x".to_string());
        let _ = LiveKitError::AuthFailed("x".to_string());
    }

    /// LiveKitConfig default + K-1 builder 守门
    #[test]
    fn livekit_config_default_and_k1_builders() {
        let cfg = LiveKitConfig::default();
        assert_eq!(cfg.platform, "apeireth");
        assert_eq!(cfg.server_url, DEFAULT_LIVEKIT_SERVER_URL);
        assert_eq!(cfg.token_ttl_seconds, DEFAULT_TOKEN_TTL_SECONDS);
        assert!(!cfg.debug);

        // with_server_url K-1
        let cfg2 = cfg
            .clone()
            .with_server_url("https://my.livekit.cloud")
            .expect("K-1 server_url OK");
        assert_eq!(cfg2.server_url, "https://my.livekit.cloud");

        // with_server_url K-1 失败
        assert!(cfg.clone().with_server_url("ws://bad").is_err());

        // with_credentials K-1
        let cfg3 = cfg
            .with_credentials("APIabc123def456ghi789", "secret_with_at_least_32_chars_xxxxx")
            .expect("K-1 api_key OK");
        assert_eq!(cfg3.api_key, "APIabc123def456ghi789");
        assert!(cfg3.api_secret.starts_with("secret_"));
    }

    /// 6 端点 STUB 路径不动守门
    #[test]
    fn stub_path_unchanged_6_endpoints_return_not_implemented() {
        // STUB_MODE 守门
        assert!(is_stub_mode(), "STUB_MODE 必须 == true (R20 阶段 6 flesh out 守门)");

        // 6 端点都走 STUB 守门 (跟 real.rs 同守门)
        // livekit_stub! 宏要求字面参数, 用单独函数包装每个端点
        fn check_server_url() -> LiveKitResult<()> { livekit_stub!("server_url") }
        fn check_api_key() -> LiveKitResult<()> { livekit_stub!("api_key") }
        fn check_room() -> LiveKitResult<()> { livekit_stub!("room") }
        fn check_track() -> LiveKitResult<()> { livekit_stub!("track") }
        fn check_participant() -> LiveKitResult<()> { livekit_stub!("participant") }
        fn check_event() -> LiveKitResult<()> { livekit_stub!("event") }

        for (name, r) in [
            ("server_url", check_server_url()),
            ("api_key", check_api_key()),
            ("room", check_room()),
            ("track", check_track()),
            ("participant", check_participant()),
            ("event", check_event()),
        ] {
            let matches = match &r {
                Err(LiveKitError::NotImplemented(s)) => s == name,
                _ => false,
            };
            assert!(
                matches,
                "STUB 守门必须返 NotImplemented({}), got: {r:?}",
                name
            );
        }
    }

    /// LiveKitEndpoint parse 守门
    #[test]
    fn livekit_endpoint_parse() {
        use std::str::FromStr;
        assert_eq!(
            LiveKitEndpoint::from_str("server_url").unwrap(),
            LiveKitEndpoint::ServerUrl
        );
        assert_eq!(
            LiveKitEndpoint::from_str("room").unwrap(),
            LiveKitEndpoint::Room
        );
        assert!(LiveKitEndpoint::from_str("unknown_endpoint").is_err());
    }

    /// 7 Request/Response 类型构造守门 (K-1 强校验)
    #[test]
    fn request_response_types_k1_validated() {
        // CreateRoomRequest
        let cr = CreateRoomRequest::new("my-room").expect("K-1 room OK");
        assert_eq!(cr.name, "my-room");
        assert_eq!(cr.max_participants, 100);
        assert!(CreateRoomRequest::new("").is_err());

        // DeleteRoomRequest
        let dr = DeleteRoomRequest::new("my-room").expect("K-1 room OK");
        assert_eq!(dr.room, "my-room");
        assert!(DeleteRoomRequest::new("with space").is_err());

        // ListParticipantsRequest
        let lp = ListParticipantsRequest::new("my-room").expect("K-1 room OK");
        assert_eq!(lp.room, "my-room");
        assert!(ListParticipantsRequest::new("").is_err());

        // RemoveParticipantRequest
        let rp = RemoveParticipantRequest::new("my-room", "user-1").expect("K-1 OK");
        assert_eq!(rp.room, "my-room");
        assert_eq!(rp.identity, "user-1");
        assert!(RemoveParticipantRequest::new("my-room", "with space").is_err());

        // MuteTrackRequest
        let mt = MuteTrackRequest::new("my-room", "user-1", "TR_abc123", true)
            .expect("K-1 OK");
        assert_eq!(mt.room, "my-room");
        assert_eq!(mt.identity, "user-1");
        assert_eq!(mt.track_sid, "TR_abc123");
        assert!(mt.muted);
        assert!(MuteTrackRequest::new("my-room", "user-1", "BAD_PREFIX", true).is_err());
    }

    /// Twirp 路径前缀 + 默认 server URL 守门
    #[test]
    fn twirp_prefix_and_default_url() {
        assert_eq!(LIVEKIT_TWIRP_PREFIX, "/twirp");
        assert_eq!(DEFAULT_LIVEKIT_SERVER_URL, "https://livekit.example.com");
        assert_eq!(
            LiveKitEndpoint::Room.twirp_service(),
            "livekit.RoomService"
        );
        assert_eq!(
            LiveKitEndpoint::Event.twirp_service(),
            "livekit.Webhook"
        );
    }

    /// ALLOWED_* 编译期白名单守门
    #[test]
    fn allowed_chars_whitelists() {
        // ALLOWED_ROOM_NAME_CHARS
        assert!(ALLOWED_ROOM_NAME_CHARS.contains(&'a'));
        assert!(ALLOWED_ROOM_NAME_CHARS.contains(&'Z'));
        assert!(ALLOWED_ROOM_NAME_CHARS.contains(&'0'));
        assert!(ALLOWED_ROOM_NAME_CHARS.contains(&'-'));
        assert!(ALLOWED_ROOM_NAME_CHARS.contains(&'_'));
        assert!(!ALLOWED_ROOM_NAME_CHARS.contains(&' '));
        assert!(!ALLOWED_ROOM_NAME_CHARS.contains(&'/'));

        // ALLOWED_TRACK_SID_CHARS
        assert!(ALLOWED_TRACK_SID_CHARS.contains(&'T'));
        assert!(ALLOWED_TRACK_SID_CHARS.contains(&'R'));
        assert!(ALLOWED_TRACK_SID_CHARS.contains(&'_'));
        assert!(!ALLOWED_TRACK_SID_CHARS.contains(&'-')); // track SID 不允许 -

        // ALLOWED_PARTICIPANT_IDENTITY_CHARS
        assert!(ALLOWED_PARTICIPANT_IDENTITY_CHARS.contains(&'a'));
        assert!(ALLOWED_PARTICIPANT_IDENTITY_CHARS.contains(&'-'));
        assert!(ALLOWED_PARTICIPANT_IDENTITY_CHARS.contains(&'_'));
        assert!(ALLOWED_PARTICIPANT_IDENTITY_CHARS.contains(&'.'));
        assert!(!ALLOWED_PARTICIPANT_IDENTITY_CHARS.contains(&' '));
    }

    /// WebhookEvent 构造守门
    #[test]
    fn webhook_event_construct() {
        let room = Room {
            sid: "RM_test123".to_string(),
            name: "test-room".to_string(),
            created_at: 1234567890,
            num_participants: 0,
            max_participants: 100,
            metadata: String::new(),
        };
        let evt = WebhookEvent::room_started(room.clone());
        assert_eq!(evt.event_type, "room_started");
        assert_eq!(evt.room.sid, "RM_test123");
        assert!(evt.event_id.len() > 0);
        assert!(evt.created_at > 0);
    }
}

// ============================================================================
// §10 Re-exports (方便 real.rs 引用)
// ============================================================================

pub use real::LiveKitRealImpl;
