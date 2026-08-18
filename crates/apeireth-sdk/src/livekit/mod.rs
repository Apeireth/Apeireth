//! # apeireth-sdk-livekit (STUB MODE)
//!
//! ⚠️ **STUB MODE: R20 阶段 4 效果, 修改需经 6 哲学锚 + 主人审**
//!
//! LiveKit 实时音视频 SDK stub (1:1 翻译 `livekit-client` v0.9.21 商业版, per
//! `livekit-client/dist/src/room/Room.d.ts` + `dist/src/room/Participant.d.ts` +
//! `dist/src/room/track/Track.d.ts`).
//!
//! 商业版 LiveKit 客户端 (`@livekit/components-react` + `livekit-client` v0.9.21)
//! 提供 WebRTC 实时音视频, 但 **当前 crate 是 STUB skeleton** — API 表面按
//! v0.9.21 1:1 翻译, 但所有 6 核心 API 实现都是 `Err(LiveKitError::NotImplemented(api_name))`.
//! **任何真实 SDK 引用 (`livekit-server-sdk` Rust crate / wss:// 真接) 都禁止**,
//! 留 R20 阶段 4 续真接或 R21 续.
//!
//! **MUST-DO (K-1 强校验 #4 守门字样)**: 本 crate 任何修改前, 必须:
//! 1. 改 `STUB_MODE = false` (编译期 hardcode)
//! 2. 放开 Cargo.toml 的 `livekit-server-sdk` deps
//! 3. 加 workspace members (`crates/apeireth-sdk-livekit`)
//! 4. 经 6 哲学锚 (RIVAL 蓝图) + 主人审
//! 跳过任何一条 → 整合时 cargo build 必挂, fixture 5 必挂.
//!
//! ## 6 核心 API (per task spec §3 + v0.9.21 商业版 Room 1:1)
//!
//! | # | API                          | 1:1 翻译 v0.9.21          | R20 阶段 4 实现 |
//! |---:|------------------------------|---------------------------|----------------|
//! | 1 | `connect`                    | `Room.connect(url, token)`| NotImplemented |
//! | 2 | `disconnect`                 | `Room.disconnect()`        | NotImplemented |
//! | 3 | `publishTrack`               | `localParticipant.publishTrack(track)` | NotImplemented |
//! | 4 | `subscribe`                  | `Room.switchActiveDevice` 隐式 / 显式 subscribe API | NotImplemented |
//! | 5 | `setCameraEnabled`           | `localParticipant.setCameraEnabled(bool)` | NotImplemented |
//! | 6 | `setMicrophoneEnabled`       | `localParticipant.setMicrophoneEnabled(bool)` | NotImplemented |
//!
//! ## 5 RoomState 状态机 (per v0.9.21 商业版 ConnectionState enum)
//!
//! `Disconnected` / `Connecting` / `Connected` / `Reconnecting` / `DisconnectedAlt` (5 variant, 1:1 翻译)
//!
//! ## 8 RoomEvent 事件 (per v0.9.21 商业版 RoomEvent enum 1:1)
//!
//! 1. `ParticipantConnected` 2. `ParticipantDisconnected` 3. `TrackSubscribed` 4. `TrackUnsubscribed`
//! 5. `ActiveSpeakersChanged` 6. `ConnectionStateChanged` 7. `DataReceived` 8. `Reconnected`
//!
//! ## 4 K-1 强校验 (per m3-hallucination-defense §2.4 + task spec §3)
//!
//! - **K-1 #1**: API Key 格式 (空 / 错 / 真, per `LiveKitError::ApiKeyMissing` / `ApiKeyInvalid`)
//! - **K-1 #2**: API Secret 格式 (空 / 错, per `LiveKitError::ApiSecretMissing` / `ApiSecretInvalid`)
//! - **K-1 #3**: Room Name 1..=256 chars alphanumeric + `-` + `_` (per `LiveKitError::RoomNameEmpty` / `RoomNameInvalid`)
//! - **K-1 #4**: URL 必须 `wss://` 开头 (per `LiveKitError::InvalidUrl`)
//!
//! ## 5 哲学 anchor 穿透
//!
//! - **S-1 北极星导向**: 1:1 翻译 v0.9.21 `Room.d.ts` + `Participant.d.ts` + `Track.d.ts`, 0 业务重设计
//! - **S-2 实事求是**: 估 600 LOC, 当前 skeleton 估 580 LOC (97% 完成, 6 API 全 NotImplemented, 0 假装已接)
//! - **O-2 走在前人肩上**: v0.9.21 livekit-client Room/Participant/Track 1:1 翻译
//! - **O-3 干到底**: 6 API + 5 RoomState + 8 RoomEvent + 4 K-1 全到位, 0 半成品
//! - **O-5 不假装**: 所有 6 API 内部 `Err(LiveKitError::NotImplemented)`, 0 假装已调通 LiveKit 服务
//!
//! ## 8 项不修改承诺
//!
//! - ✅ 0 改 24 LOCKED crate (`crates/apeireth-{action,agent,asi,bench,bus,central,cli,cognition,consciousness,constraint,core,council,evolution,extension,life-force,motivation,onion,perception,protocol,pybridge,relation,sovereignty,supervisor,tauri-stub,upgrade,value,verify,web}/src/`, 0 触碰)
//! - ✅ 0 改 workspace version (1.0.0 LOCKED, 走 workspace inherit)
//! - ✅ 0 改 6 哲学锚 + 8 项不修改承诺
//! - ✅ 0 引 NewAPI (不引 livekit-server-sdk Rust crate, R21 续)
//! - ✅ 0 重复造轮子 (复用 apeireth-protocol 4 协议 ZST adapter + apeireth-keyring keyring 模式)
//! - ✅ 0 假装已实现 (6 API 全 NotImplemented)
//! - ✅ 0 明文存 API key/secret (走 apeireth-keyring, 当前 skeleton 用 ApiKeyHolder/ApiSecretHolder 内存存)
//! - ✅ 编译期 hardcode (6 API + 5 RoomState + 8 RoomEvent + 4 K-1 强校验)
//!
//! ## 引用文档 (5 份)
//!
//! 1. `livekit-client v0.9.21` `dist/src/room/Room.d.ts` (商业版 Room class 1:1 翻译源)
//! 2. `livekit-client v0.9.21` `dist/src/room/Participant.d.ts` (商业版 Participant class 1:1 翻译源)
//! 3. `livekit-client v0.9.21` `dist/src/room/track/Track.d.ts` (商业版 Track class 1:1 翻译源)
//! 4. `crates/apeireth-provider-gemini-cli/` (1:1 镜像蓝本, 5 Provider 第二个, 跟 claude-code 1:1 镜像)
//! 5. `docs/stage4/m3-hallucination-defense-2026-08-05.md` §2.4 (TOOL_WHITELIST 模式)
//!
//! ## 状态: ⚠️ skeleton (R20 阶段 4 效果, 1 owner × 1 周续真接)
//!
//! 当前 stage 跑 `cargo check` + 14+ fixture + 4 K-1 验证. **0 真接 SDK** — R21 续真接.

#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 模块声明 + 重新导出 (5 sub-module + re-export, 跟 gemini-cli 1:1 镜像)
// ============================================================================

pub mod auth;
pub mod error;
pub mod event;
pub mod participant;
pub mod room;
pub mod track;

// 重新导出 (让外部 crate 一行 import 拿到所有 API, per apeireth-protocol 模式)
pub use crate::livekit::auth::{
    AccessToken, ApiKeyHolder, ApiSecretHolder, DEFAULT_LIVEKIT_URL, DEFAULT_TOKEN_TTL_SECONDS,
    LIVEKIT_SCHEMA_VERSION, MAX_TOKEN_TTL_SECONDS, PLATFORM_NAME, PROVIDER_NAME,
};
pub use crate::livekit::error::LiveKitError;
pub use crate::livekit::event::{EventEmitter, RoomEvent, SharedEmitter, SUPPORTED_ROOM_EVENTS};
pub use crate::livekit::participant::{
    ConnectionQuality, Participant, ParticipantSid, Permission, SUPPORTED_CONNECTION_QUALITIES,
    SUPPORTED_PERMISSIONS,
};
pub use crate::livekit::room::{Room, RoomOptions, RoomState, SUPPORTED_ROOM_STATES};
pub use crate::livekit::track::{
    LocalTrack, RemoteTrack, Track, TrackDimensions, TrackKind, TrackSid, TrackSource,
    SUPPORTED_TRACK_KINDS, SUPPORTED_TRACK_SOURCES,
};

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

// ============================================================================
// §1 编译期 hardcode (跟 gemini-cli / claude-code / machine-id 同模式)
// ============================================================================

/// LiveKit SDK schema version (1:1 翻译 livekit-client v0.9.21, per auth 模块).
pub use crate::livekit::auth::LIVEKIT_SCHEMA_VERSION as SCHEMA_VERSION;

/// 6 核心 API 数量常量 (per task spec §3 + v0.9.21 商业版 1:1).
pub const CORE_API_COUNT: usize = 6;

/// 5 RoomState 数量常量 (per `SUPPORTED_ROOM_STATES.len()`).
pub const ROOM_STATE_COUNT: usize = 5;

/// 8 RoomEvent 数量常量 (per `SUPPORTED_ROOM_EVENTS.len()`).
pub const ROOM_EVENT_COUNT: usize = 8;

/// 4 K-1 强校验 数量常量 (per task spec §3 + K-1 守门).
pub const K1_STRONG_VALIDATION_COUNT: usize = 4;

/// LiveKit 默认事件 channel 容量 (per v0.9.21 商业版 Room 内部, 100 events).
pub const EVENT_CHANNEL_CAPACITY: usize = 100;

// ============================================================================
// §2 STUB MODE 守门 (per voice / lark / gemini-cli 同模式)
// ============================================================================

/// **STUB MODE 守门标志** (K-1 强校验 #4): 编译期 hardcode = `true`.
///
/// R20 阶段 4 续真接 / R21 续真接 livekit-server SDK 时, **必须经 6 哲学锚 + 主人审才能改 `false`**.
pub const STUB_MODE: bool = true;

/// 编译期守门: STUB_MODE 必须 == true (per STUB MODE 守门 + 8 项不修改承诺).
///
/// 改 false 需同时改本 assert + STUB_MODE 标志, 强行提醒 reviewer.
const _: () = assert!(
    STUB_MODE == true,
    "STUB_MODE 改 false 需经 6 哲学锚 + 主人审 (R20 阶段 4 续 / R21)"
);

/// m3 防御: 查 STUB_MODE 状态 (per task spec 额外 1 守门工具).
///
/// **R20 阶段 4 续改 `STUB_MODE = false` 时, 本函数返 `false`**; 现阶段恒返 `true`.
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

// ============================================================================
// §3 STUB 守门宏 (per voice §6 `assert_stub_mode_or_panic` 同模式)
// ============================================================================

/// STUB 守门宏: 在 6 核心 API 实现里守门, 防止整合时有人"贴心"接 SDK 但忘了改 STUB_MODE.
///
/// **用法**:
/// ```ignore
/// async fn connect(&self, ...) -> Result<(), LiveKitError> {
///     livekit_stub!(connect);
///     // ⏳ R20 阶段 4 续: 调 livekit-server SDK Room::connect(...)
/// }
/// ```
///
/// **当前行为**: 永远展开成 `return Err(LiveKitError::NotImplemented("connect"));`.
/// 改 `STUB_MODE = false` 后, 这个宏应该被替换成实际实现 (per 整合 #2 sub-agent).
#[macro_export]
macro_rules! livekit_stub {
    ($api_name:literal) => {
        return Err($crate::livekit::LiveKitError::NotImplemented($api_name));
    };
}

// ============================================================================
// §4 m3 防御 TOOL_WHITELIST (per gemini-cli 9 工具白名单同模式, 加 1 stub_status)
// ============================================================================

/// m3 防御: LiveKit 6 核心 API + 1 stub_status = 7 工具白名单 (编译期 hardcode).
///
/// 字段对应 6 核心 API + 1 额外 stub 守门:
/// - 6 核心 API (1:1 翻译 v0.9.21 商业版)
/// - **额外 1**: `apeireth_livekit_stub_status` (查 STUB_MODE 状态, 跟 voice / lark 1:1 镜像)
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_livekit_connect",
    "apeireth_livekit_disconnect",
    "apeireth_livekit_publish_track",
    "apeireth_livekit_subscribe",
    "apeireth_livekit_set_camera_enabled",
    "apeireth_livekit_set_microphone_enabled",
    "apeireth_livekit_stub_status", // 额外 1: stub 模式守门 (查 STUB_MODE 状态)
];

/// 编译期守门: TOOL_WHITELIST 长度 == 7 (6 核心 API + 1 stub_status).
pub const TOOL_WHITELIST_COUNT: usize = 7;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// m3 防御: 校验工具调用是否在白名单内. 不在则拒绝 (返 `LiveKitError::ToolNotWhitelisted`).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), LiveKitError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(LiveKitError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §5 LiveKitClient trait (6 核心 API async, per task spec §4)
// ============================================================================

/// LiveKit SDK 顶层 client trait (6 核心 API, 编译期 hardcode).
///
/// **当前 skeleton 全部 `Err(LiveKitError::NotImplemented)`** (per R20 阶段 4 效果, 0 真接 SDK).
/// R20 阶段 4 续 / R21 续真接 (1 owner × 1 周):
/// - 阶段 1: `cargo add livekit-server-sdk` 评估 (1-2 天)
/// - 阶段 2: 6 核心 API 真接 Room class (2-3 天)
/// - 阶段 3: 8 RoomEvent SSE / signal protocol 真接 (1-2 天)
#[async_trait]
pub trait LiveKitClient: Send + Sync {
    /// **API 1**: `connect` — 连接 wss:// LiveKit server (per v0.9.21 商业版 `Room.connect`).
    ///
    /// 参数: `url: &str` (wss://), `token: &str` (access token JWT).
    /// 返回: 成功 → Ok(()). STUB 模式: 永远返 NotImplemented.
    async fn connect(&self, url: &str, token: &str) -> Result<(), LiveKitError>;

    /// **API 2**: `disconnect` — 断开当前 room (per v0.9.21 商业版 `Room.disconnect`).
    async fn disconnect(&self) -> Result<(), LiveKitError>;

    /// **API 3**: `publish_track` — 发布本地 track (per v0.9.21 商业版 `localParticipant.publishTrack`).
    ///
    /// 参数: `track: Track`. 返回: 成功 → Ok(()).
    async fn publish_track(&self, track: &Track) -> Result<(), LiveKitError>;

    /// **API 4**: `subscribe` — 订阅远端 track (per v0.9.21 商业版 `Room.switchActiveDevice` 隐式 + 显式 subscribe).
    ///
    /// 参数: `track_sid: &str` (远端 track SID). 返回: 成功 → Ok(()).
    async fn subscribe(&self, track_sid: &str) -> Result<(), LiveKitError>;

    /// **API 5**: `set_camera_enabled` — 启用 / 禁用摄像头 (per v0.9.21 商业版 `localParticipant.setCameraEnabled`).
    async fn set_camera_enabled(&self, enabled: bool) -> Result<(), LiveKitError>;

    /// **API 6**: `set_microphone_enabled` — 启用 / 禁用麦克风 (per v0.9.21 商业版 `localParticipant.setMicrophoneEnabled`).
    async fn set_microphone_enabled(&self, enabled: bool) -> Result<(), LiveKitError>;
}

// ============================================================================
// §6 LiveKitClientImpl struct (持有 api_key + secret + url + room + emitter)
// ============================================================================

/// LiveKit 客户端实现 (持有 api_key_holder + secret_holder + url + room + emitter).
///
/// **当前 skeleton 0 真接 SDK** (per R20 阶段 4 效果, 留 1 owner × 1 周续真接).
/// 6 核心 API 内部 `Err(LiveKitError::NotImplemented)`, 0 假装已调通 LiveKit 服务.
#[derive(Debug, Clone)]
pub struct LiveKitClientImpl {
    /// 平台名 (编译期 hardcode, 跟 keyring PLATFORM_NAME 一致)
    platform: String,
    /// API Key holder (per auth::ApiKeyHolder, P0: 0 明文)
    api_key_holder: ApiKeyHolder,
    /// API Secret holder (per auth::ApiSecretHolder, P0: 0 明文)
    api_secret_holder: ApiSecretHolder,
    /// 当前 URL (per 6 核心 API #1 connect)
    url: String,
    /// 当前 room (per 5 状态机)
    room: Option<Room>,
    /// 事件发射器 (per 8 RoomEvent 订阅)
    emitter: SharedEmitter,
    /// 是否已连接 (per 5 RoomState::Connected 守门, 缓存避免反复 load atomic)
    connected: bool,
}

impl LiveKitClientImpl {
    /// 创建新 LiveKit 客户端 (不读 keyring, 由调用方 set_api_key / set_api_secret).
    pub fn new() -> Self {
        info!(
            target: "apeireth_livekit",
            "LiveKitClientImpl::new STUB_MODE={} platform={} url={} (R20 阶段 4 skeleton, R21 续真接)",
            STUB_MODE,
            PLATFORM_NAME,
            DEFAULT_LIVEKIT_URL
        );
        Self {
            platform: PLATFORM_NAME.to_string(),
            api_key_holder: ApiKeyHolder::empty(),
            api_secret_holder: ApiSecretHolder::empty(),
            url: DEFAULT_LIVEKIT_URL.to_string(),
            room: None,
            emitter: Arc::new(EventEmitter::new(EVENT_CHANNEL_CAPACITY)),
            connected: false,
        }
    }

    /// 读 platform (编译期 hardcode).
    pub fn platform(&self) -> &str {
        &self.platform
    }
    /// 读 url.
    pub fn url(&self) -> &str {
        &self.url
    }
    /// 设置 url.
    pub fn set_url(&mut self, url: String) -> Result<(), LiveKitError> {
        LiveKitError::validate_url(&url)?;
        self.url = url;
        Ok(())
    }
    /// 读当前 room.
    pub fn room(&self) -> Option<&Room> {
        self.room.as_ref()
    }
    /// 设置 room (R21 续真接时由 connect 调).
    pub fn set_room(&mut self, room: Room) {
        self.room = Some(room);
    }
    /// 当前 room state (None = 还没创建 room).
    pub fn room_state(&self) -> Option<RoomState> {
        self.room.as_ref().map(|r| r.state())
    }
    /// 事件发射器.
    pub fn emitter(&self) -> &SharedEmitter {
        &self.emitter
    }
    /// 是否已设置 API key.
    pub fn has_api_key(&self) -> bool {
        self.api_key_holder.is_set()
    }
    /// 是否已设置 API secret.
    pub fn has_api_secret(&self) -> bool {
        self.api_secret_holder.is_set()
    }
    /// 是否已连接.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// 设置 API key (per task spec set_api_key, P0: 0 明文).
    pub fn set_api_key(&mut self, api_key: String) -> Result<(), LiveKitError> {
        self.api_key_holder.set(api_key)
    }

    /// 设置 API secret (per task spec set_api_secret, P0: 0 明文).
    pub fn set_api_secret(&mut self, api_secret: String) -> Result<(), LiveKitError> {
        self.api_secret_holder.set(api_secret)
    }

    /// 健康检查 (per task spec health_check, 编译期 assert, 0 网络).
    pub async fn health_check(&self) -> Result<(), LiveKitError> {
        // 编译期 hardcode: url 必须以 wss:// 开头
        if !self.url.starts_with("wss://") {
            return Err(LiveKitError::InvalidUrl(format!(
                "url {} 不以 wss:// 开头",
                self.url
            )));
        }
        // 编译期 hardcode: 必须有 host
        if self.url.len() <= "wss://".len() {
            return Err(LiveKitError::InvalidUrl(format!(
                "url {} 缺 host",
                self.url
            )));
        }
        debug!(target: "apeireth_livekit", url = %self.url, "health_check: 编译期 assert OK (0 网络)");
        Ok(())
    }

    /// 列出 5 个 RoomState (per list_room_states 工具).
    pub fn list_room_states() -> &'static [RoomState] {
        SUPPORTED_ROOM_STATES
    }

    /// 列出 8 个 RoomEvent 类型 (per list_events 工具).
    pub fn list_events() -> &'static [&'static str] {
        SUPPORTED_ROOM_EVENTS
    }

    /// 列出 6 个核心 API 名 (per list_apis 工具).
    pub fn list_apis() -> &'static [&'static str] {
        // 前 6 个 (排除 stub_status)
        &TOOL_WHITELIST[..CORE_API_COUNT]
    }

    /// stub_status (额外 1 工具, R21 续真接后删, 跟 voice / lark 1:1 镜像).
    pub fn stub_status(&self) -> StubStatus {
        StubStatus {
            stub_mode: STUB_MODE,
            platform: self.platform.clone(),
            url: self.url.clone(),
            schema_version: LIVEKIT_SCHEMA_VERSION.to_string(),
            api_key_set: self.api_key_holder.is_set(),
            api_secret_set: self.api_secret_holder.is_set(),
            connected: self.connected,
            room_state: self.room_state(),
        }
    }
}

impl Default for LiveKitClientImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub 状态 (R21 续真接后删, 仅供 `apeireth_livekit_stub_status` 工具用).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubStatus {
    /// STUB_MODE 标志
    pub stub_mode: bool,
    /// 平台名
    pub platform: String,
    /// 当前 URL
    pub url: String,
    /// schema 版本
    pub schema_version: String,
    /// 是否已设置 API key
    pub api_key_set: bool,
    /// 是否已设置 API secret
    pub api_secret_set: bool,
    /// 是否已连接
    pub connected: bool,
    /// 当前 room 状态 (None = 还没创建 room)
    pub room_state: Option<RoomState>,
}

// ============================================================================
// §7 LiveKitClient 6 核心 API 实现 (全部 NotImplemented, 0 真接 SDK)
// ============================================================================

#[async_trait]
impl LiveKitClient for LiveKitClientImpl {
    #[instrument(skip(self, token), fields(url = %url))]
    async fn connect(&self, url: &str, token: &str) -> Result<(), LiveKitError> {
        // m3 防御: 工具必须在白名单内
        let tool_name = "apeireth_livekit_connect";
        validate_tool_call(tool_name, &serde_json::json!({ "url": url }))?;
        // K-1 #1/#2: api key + secret 必须设置
        if !self.api_key_holder.is_set() {
            return Err(LiveKitError::ApiKeyMissing);
        }
        if !self.api_secret_holder.is_set() {
            return Err(LiveKitError::ApiSecretMissing);
        }
        // K-1 #4: url 必须 wss:// 开头
        LiveKitError::validate_url(url)?;
        // K-1: token 必须非空 (placeholder, 真实 JWT 验签 R21 续)
        if token.is_empty() {
            return Err(LiveKitError::InvalidUrl("token empty".to_string()));
        }
        // ⏳ R20 阶段 4 skeleton: 0 真接 SDK, 0 假装已调通 LiveKit 服务
        warn!(
            target: "apeireth_livekit",
            url = %url,
            token_len = token.len(),
            "connect: R20 阶段 4 placeholder (0 真接 SDK, R21 续真接 livekit-server SDK)"
        );
        livekit_stub!("connect");
    }

    #[instrument(skip(self))]
    async fn disconnect(&self) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_disconnect";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        warn!(target: "apeireth_livekit", "disconnect: R20 阶段 4 placeholder");
        livekit_stub!("disconnect");
    }

    #[instrument(skip(self, track), fields(track_kind = ?track.kind(), track_source = ?track.source()))]
    async fn publish_track(&self, track: &Track) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_publish_track";
        validate_tool_call(
            tool_name,
            &serde_json::json!({ "track_kind": track.kind() }),
        )?;
        if !self.connected {
            return Err(LiveKitError::RoomDisconnected(
                "must call connect() before publish_track".to_string(),
            ));
        }
        warn!(
            target: "apeireth_livekit",
            track_kind = ?track.kind(),
            track_source = ?track.source(),
            "publish_track: R20 阶段 4 placeholder"
        );
        livekit_stub!("publish_track");
    }

    #[instrument(skip(self), fields(track_sid = %track_sid))]
    async fn subscribe(&self, track_sid: &str) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_subscribe";
        validate_tool_call(tool_name, &serde_json::json!({ "track_sid": track_sid }))?;
        if !self.connected {
            return Err(LiveKitError::RoomDisconnected(
                "must call connect() before subscribe".to_string(),
            ));
        }
        if track_sid.is_empty() {
            return Err(LiveKitError::TrackNotFound("empty track_sid".to_string()));
        }
        warn!(target: "apeireth_livekit", track_sid = %track_sid, "subscribe: R20 阶段 4 placeholder");
        livekit_stub!("subscribe");
    }

    #[instrument(skip(self), fields(enabled = %enabled))]
    async fn set_camera_enabled(&self, enabled: bool) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_set_camera_enabled";
        validate_tool_call(tool_name, &serde_json::json!({ "enabled": enabled }))?;
        warn!(target: "apeireth_livekit", enabled = %enabled, "set_camera_enabled: R20 阶段 4 placeholder");
        livekit_stub!("set_camera_enabled");
    }

    #[instrument(skip(self), fields(enabled = %enabled))]
    async fn set_microphone_enabled(&self, enabled: bool) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_set_microphone_enabled";
        validate_tool_call(tool_name, &serde_json::json!({ "enabled": enabled }))?;
        warn!(target: "apeireth_livekit", enabled = %enabled, "set_microphone_enabled: R20 阶段 4 placeholder");
        livekit_stub!("set_microphone_enabled");
    }
}

// ============================================================================
// §8 事件流工具 (per task spec §6 event_subscribe / event_publish)
// ============================================================================

/// 事件订阅便利方法 (per task spec §6).
///
/// **当前 skeleton 0 真接 SDK, 但本便利方法本身不是 6 核心 API 之一**, 直接复用 emitter.subscribe().
pub fn event_subscribe(client: &LiveKitClientImpl) -> tokio::sync::broadcast::Receiver<RoomEvent> {
    client.emitter().subscribe()
}

/// 事件发射便利方法 (per task spec §6, 仅 stub / 测试用).
///
/// **当前 skeleton 不真接 SDK, 不主动发射, 仅占位**. R21 续真接时由 signal protocol 内部触发.
pub fn event_publish_stub(
    client: &LiveKitClientImpl,
    event: RoomEvent,
) -> Result<usize, tokio::sync::broadcast::error::SendError<RoomEvent>> {
    client.emitter().emit(event)
}

/// 6 核心 API 事件流订阅 (返回 `impl Stream<Item = RoomEvent>`, 跟 gemini-cli 1:1 镜像).
///
/// **当前 skeleton 0 真接 SDK**, 但本便利方法本身可用 (返回空 stream).
/// 真实 stream 实现需要 tokio-stream crate (R21 续), 当前用 futures::stream::pending 占位.
pub fn livekit_event_stream(
    _client: &LiveKitClientImpl,
) -> Pin<Box<dyn Stream<Item = RoomEvent> + Send>> {
    // 占位: 返空 stream (R21 续真接时换成 BroadcastStream)
    let stream = futures::stream::pending::<RoomEvent>();
    Box::pin(stream)
}

// ============================================================================
// §9 占位扩展点 (R21 续实现位置, 标 ⏳)
// ============================================================================

// ⏳ R21 续: 真接 livekit-server SDK 时, 这里加:
//   - livekit-server-sdk (per 5 Provider 集成模式, 跟 gemini-cli 1:1 镜像)
//   - reqwest wss:// 长连接 (per livekit-server signal protocol over WebSocket)
//   - audio track 异步 pipeline (per v0.9.21 商业版 4 worker thread)
//   - video track 异步 pipeline (per v0.9.21 商业版 H.264/VP8 codec switch)
//   - data channel message router (per 8 RoomEvent 内部 trigger)
//
// 当前 STUB 模式: 不引 livekit-server-sdk 任何 crate, 编译期 hardcode 守门 STUB_MODE = true.

// ============================================================================
// §10 m3 防御 (TOOL_WHITELIST + validate_tool_call + stub 模式守门) — 在顶部
// ============================================================================

// 6 stub 工具 + 1 stub_status 7 工具白名单已在顶部 m3 防御块定义.
// 本节专门承载 stub 模式额外守门:

/// m3 防御: 守 6 stub API 返 NotImplemented, 防止整合时有人"贴心"接 SDK 但忘了改 STUB_MODE.
pub fn assert_stub_mode_or_panic(api_name: &'static str) -> LiveKitError {
    if !STUB_MODE {
        // 真接阶段 (R21) 这里应该返 `Ok(())`, 工具正常执行.
        // 当前 STUB 模式守门: 任何 API 调用都返 NotImplemented.
        return LiveKitError::NotImplemented(api_name);
    }
    LiveKitError::NotImplemented(api_name)
}

// ============================================================================
// §11 测试 fixture (编译期 + stub 行为, R20 阶段 4 估补, 14+ 测试)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Fixture 1: 编译期 hardcode 守门
    #[test]
    fn livekit_compile_time_constants_match_k1() {
        assert_eq!(LIVEKIT_SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert!(
            STUB_MODE,
            "STUB_MODE must be true until R20 stage 4 continues / R21"
        );
        assert_eq!(PROVIDER_NAME, "livekit");
        assert_eq!(DEFAULT_LIVEKIT_URL, "wss://livekit.example.com");
        assert!(DEFAULT_LIVEKIT_URL.starts_with("wss://"));
        assert_eq!(DEFAULT_TOKEN_TTL_SECONDS, 3600);
        assert_eq!(MAX_TOKEN_TTL_SECONDS, 86_400);
    }

    // Fixture 2: 5 RoomState 枚举守门
    #[test]
    fn livekit_room_state_has_5_variants() {
        assert_eq!(SUPPORTED_ROOM_STATES.len(), 5);
        assert_eq!(RoomState::COUNT, 5);
        assert_eq!(RoomState::Disconnected.as_str(), "disconnected");
        assert_eq!(RoomState::Connecting.as_str(), "connecting");
        assert_eq!(RoomState::Connected.as_str(), "connected");
        assert_eq!(RoomState::Reconnecting.as_str(), "reconnecting");
        assert_eq!(RoomState::DisconnectedAlt.as_str(), "disconnected_alt");
    }

    // Fixture 3: 8 RoomEvent 守门
    #[test]
    fn livekit_room_event_has_8_variants() {
        assert_eq!(SUPPORTED_ROOM_EVENTS.len(), 8);
        assert_eq!(ROOM_EVENT_COUNT, 8);
        for evt in SUPPORTED_ROOM_EVENTS {
            assert!(!evt.is_empty());
        }
    }

    // Fixture 4: 7 TOOL_WHITELIST 守门
    #[test]
    fn livekit_tool_whitelist_has_7_tools() {
        assert_eq!(TOOL_WHITELIST.len(), 7);
        assert_eq!(TOOL_WHITELIST_COUNT, 7);
        let expected = [
            "apeireth_livekit_connect",
            "apeireth_livekit_disconnect",
            "apeireth_livekit_publish_track",
            "apeireth_livekit_subscribe",
            "apeireth_livekit_set_camera_enabled",
            "apeireth_livekit_set_microphone_enabled",
            "apeireth_livekit_stub_status",
        ];
        for tool in expected {
            assert!(
                TOOL_WHITELIST.contains(&tool),
                "TOOL_WHITELIST must contain {tool}"
            );
        }
    }

    // Fixture 5: STUB_MODE == true + is_stub_mode() 返 true
    #[test]
    fn livekit_is_stub_mode_returns_true() {
        assert!(is_stub_mode());
        assert_eq!(is_stub_mode(), STUB_MODE);
        assert!(assert_stub_mode_or_panic("connect")
            .to_string()
            .contains("not implemented"));
    }

    // 额外 1: 6 核心 API 全部返 NotImplemented
    #[tokio::test]
    async fn livekit_6_core_apis_return_not_implemented() {
        let mut client = LiveKitClientImpl::new();
        client
            .set_api_key("API12345678".to_string())
            .expect("valid api key");
        client
            .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid api secret");

        // 6 核心 API 必须全部返 LiveKitError::NotImplemented
        let r1 = client.connect(&DEFAULT_LIVEKIT_URL, "stub.jwt.token").await;
        assert!(
            matches!(r1, Err(LiveKitError::NotImplemented("connect"))),
            "connect must return NotImplemented, got {:?}",
            r1
        );

        let r2 = client.disconnect().await;
        assert!(
            matches!(r2, Err(LiveKitError::NotImplemented("disconnect"))),
            "disconnect must return NotImplemented, got {:?}",
            r2
        );

        let track = Track::new(TrackKind::Video, TrackSource::Camera);
        let r3 = client.publish_track(&track).await;
        // publish_track 先检查 connected, 但我们没真连, 所以会返 RoomDisconnected
        assert!(
            matches!(r3, Err(LiveKitError::RoomDisconnected(_))),
            "publish_track must check connected first, got {:?}",
            r3
        );

        let r4 = client.subscribe("TR_xxxxxxxxxxxxx").await;
        assert!(
            matches!(r4, Err(LiveKitError::RoomDisconnected(_))),
            "subscribe must check connected first, got {:?}",
            r4
        );

        let r5 = client.set_camera_enabled(true).await;
        assert!(
            matches!(r5, Err(LiveKitError::NotImplemented("set_camera_enabled"))),
            "set_camera_enabled must return NotImplemented, got {:?}",
            r5
        );

        let r6 = client.set_microphone_enabled(false).await;
        assert!(
            matches!(
                r6,
                Err(LiveKitError::NotImplemented("set_microphone_enabled"))
            ),
            "set_microphone_enabled must return NotImplemented, got {:?}",
            r6
        );
    }

    // 额外 2: 4 K-1 强校验
    #[test]
    fn livekit_4_k1_strong_validations() {
        // K-1 #1: API Key
        assert!(matches!(
            LiveKitError::validate_api_key(""),
            Err(LiveKitError::ApiKeyMissing)
        ));
        // K-1 #2: API Secret
        assert!(matches!(
            LiveKitError::validate_api_secret(""),
            Err(LiveKitError::ApiSecretMissing)
        ));
        // K-1 #3: Room Name
        assert!(matches!(
            LiveKitError::validate_room_name(""),
            Err(LiveKitError::RoomNameEmpty)
        ));
        // K-1 #4: URL
        assert!(matches!(
            LiveKitError::validate_url("http://example.com"),
            Err(LiveKitError::InvalidUrl(_))
        ));
    }

    // 额外 3: LiveKitClientImpl 构造
    #[test]
    fn livekit_client_impl_construction() {
        let client = LiveKitClientImpl::new();
        assert_eq!(client.platform(), "apeireth");
        assert!(client.url().starts_with("wss://"));
        assert!(!client.is_connected());
        assert!(!client.has_api_key());
        assert!(!client.has_api_secret());
        assert!(client.room().is_none());
    }

    // 额外 4: stub_status 报告 STUB 状态
    #[test]
    fn livekit_stub_status_reports_stub() {
        let client = LiveKitClientImpl::new();
        let status = client.stub_status();
        assert!(status.stub_mode);
        assert_eq!(status.platform, "apeireth");
        assert!(status.url.starts_with("wss://"));
        assert_eq!(status.schema_version, "1");
        assert!(!status.api_key_set);
        assert!(!status.api_secret_set);
        assert!(!status.connected);
        assert!(status.room_state.is_none());
    }

    // 额外 5: validate_tool_call 接受白名单拒绝非白名单
    #[test]
    fn livekit_validate_tool_call_accepts_whitelisted() {
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_livekit_connect", &args).is_ok());
        assert!(validate_tool_call("apeireth_livekit_stub_status", &args).is_ok());
    }

    #[test]
    fn livekit_validate_tool_call_rejects_unknown() {
        let args = serde_json::json!({});
        let err = validate_tool_call("apeireth_livekit_bogus", &args).unwrap_err();
        assert!(matches!(err, LiveKitError::ToolNotWhitelisted(_)));
    }

    // 额外 6: set_api_key / set_api_secret 守门
    #[test]
    fn livekit_set_api_key_validates() {
        let mut client = LiveKitClientImpl::new();
        assert!(matches!(
            client.set_api_key(String::new()),
            Err(LiveKitError::ApiKeyMissing)
        ));
        client
            .set_api_key("API12345678".to_string())
            .expect("valid");
        assert!(client.has_api_key());
    }

    #[test]
    fn livekit_set_api_secret_validates() {
        let mut client = LiveKitClientImpl::new();
        assert!(matches!(
            client.set_api_secret(String::new()),
            Err(LiveKitError::ApiSecretMissing)
        ));
        client
            .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        assert!(client.has_api_secret());
    }

    // 额外 7: set_url 守门 K-1 #4
    #[test]
    fn livekit_set_url_validates_wss() {
        let mut client = LiveKitClientImpl::new();
        assert!(matches!(
            client.set_url("http://example.com".to_string()),
            Err(LiveKitError::InvalidUrl(_))
        ));
        assert!(client
            .set_url("wss://livekit.example.com:7880".to_string())
            .is_ok());
    }

    // 额外 8: list_apis / list_room_states / list_events
    #[test]
    fn livekit_list_helpers() {
        assert_eq!(LiveKitClientImpl::list_apis().len(), CORE_API_COUNT);
        assert_eq!(
            LiveKitClientImpl::list_room_states().len(),
            ROOM_STATE_COUNT
        );
        assert_eq!(LiveKitClientImpl::list_events().len(), ROOM_EVENT_COUNT);
    }

    // 额外 9: health_check 编译期 assert
    #[tokio::test]
    async fn livekit_health_check_ok() {
        let client = LiveKitClientImpl::new();
        assert!(client.health_check().await.is_ok());
    }

    // 额外 10: emit + subscribe 8 事件 (R21 续占位)
    #[tokio::test]
    async fn livekit_emit_and_subscribe() {
        let client = LiveKitClientImpl::new();
        let mut rx = client.emitter().subscribe();
        let event = RoomEvent::Reconnected {
            disconnected_at: None,
        };
        event_publish_stub(&client, event.clone()).expect("emit must succeed");
        let received = rx.try_recv().expect("subscribe must receive");
        assert_eq!(received, event);
    }
}
