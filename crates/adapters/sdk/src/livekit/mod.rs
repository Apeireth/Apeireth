//! # 实时音视频信令客户端族 (协议客户端层真现实现)
//!
//! 本族实现 WebRTC 信令 / 房间 / 轨道发布订阅的**协议客户端层**:
//!
//! - [`signal`]: 信令帧定义 + 长度前缀 JSON 编解码 + 握手序列 + 数据分块重组
//! - [`session`]: 会话状态机 (5 状态全迁移, 含心跳 / 断线重连) + 媒体会话生命周期
//! - [`event`]: 房间事件 (8 变体) + 事件发射器
//! - [`room`] / [`participant`] / [`track`]: 房间 / 参与者 / 轨道领域模型
//! - [`auth`]: API Key / Secret 持有者 (脱敏) + 访问令牌 + K-1 常量
//! - [`error`]: 闭合错误词表 (网络 / 认证 / 协议 / 限流 / 超时 / 背压 / 状态)
//!
//! ## 架构边界
//!
//! 协议层**不直接碰网络**: 全部 IO 经 [`signal::SignalTransport`] 边界注入。
//! 生产实现注入 WebSocket 传输, 测试注入脚本化 mock —— 协议逻辑
//! (编解码 / 握手 / 心跳 / 重连 / 分块 / 背压 / 错误分类) 在两条路径上完全一致,
//! 测试零真实网络。
//!
//! 超时统一走 `apeireth_core::deadline` 熔合面 ([`clamp_timeout`] 过闸 +
//! [`Deadline`] 到期通知), 日志面统一走 [`crate::redact`] 脱敏 (令牌 / 密钥
//! 永不进错误文案与 Debug 输出)。
//!
//! ## 6 核心 API
//!
//! | # | API | 协议行为 |
//! |---:|---|---|
//! | 1 | `connect` | 握手 (Hello/Welcome/Join/JoinAccepted) + 超时熔合 |
//! | 2 | `disconnect` | 优雅离开 (Leave/LeaveAck), 终态 `DisconnectedAlt` |
//! | 3 | `publish_track` | `TrackPublish` → `TrackPublished` (服务端分配 SID) |
//! | 4 | `subscribe` | `TrackSubscribe` → `TrackSubscribed` 事件 |
//! | 5 | `set_camera_enabled` | Camera 轨道发布 / 撤下 (幂等) |
//! | 6 | `set_microphone_enabled` | Microphone 轨道发布 / 撤下 (幂等) |

#![warn(missing_docs)]
#![allow(clippy::all)]

// ============================================================================
// §0 模块声明 + 重新导出
// ============================================================================

pub mod auth;
pub mod error;
pub mod event;
pub mod participant;
pub mod room;
pub mod session;
pub mod signal;
pub mod track;

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
pub use crate::livekit::session::{
    HeartbeatPolicy, LocalTrackPhase, LocalTrackRecord, ReconnectPolicy, SessionEffect,
    SessionStats, SignalingSession, TrackSubscription,
};
pub use crate::livekit::signal::{SignalDecoder, SignalFrame, SignalTransport, PROTOCOL_VERSION};
pub use crate::livekit::track::{
    LocalTrack, RemoteTrack, Track, TrackDimensions, TrackKind, TrackSid, TrackSource,
    SUPPORTED_TRACK_KINDS, SUPPORTED_TRACK_SOURCES,
};

use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{debug, info, instrument, warn};

use crate::redact::SecretValue;

// ============================================================================
// §1 编译期常量
// ============================================================================

/// 信令 schema 版本 (跟 auth 模块一致).
pub use crate::livekit::auth::LIVEKIT_SCHEMA_VERSION as SCHEMA_VERSION;

/// 6 核心 API 数量常量.
pub const CORE_API_COUNT: usize = 6;

/// 5 RoomState 数量常量.
pub const ROOM_STATE_COUNT: usize = 5;

/// 8 RoomEvent 数量常量.
pub const ROOM_EVENT_COUNT: usize = 8;

/// 4 K-1 强校验数量常量.
pub const K1_STRONG_VALIDATION_COUNT: usize = 4;

/// 事件广播 channel 容量 (100 条).
pub const EVENT_CHANNEL_CAPACITY: usize = 100;

/// 连接 / 握手默认超时 (毫秒).
pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 10_000;

/// 连接 / 握手超时上限 (毫秒, 走 `apeireth_core::deadline::clamp_timeout` 过闸).
pub const MAX_CONNECT_TIMEOUT_MS: u64 = 120_000;

/// 协商帧缓冲上限 (溢出丢最旧, 防无消费者撑爆内存).
pub const MAX_NEGOTIATION_BUFFER: usize = 1024;

// ============================================================================
// §2 实现状态标志 + 工具白名单
// ============================================================================

/// 协议层实现状态: `false` = 协议逻辑已全量实现 (真现实现).
///
/// 传输实现仍经 [`SignalTransport`] 边界注入 —— 这是架构边界, 不是未实现面。
pub const STUB_MODE: bool = false;

/// 查询协议层实现状态 (兼容观测面).
pub fn is_stub_mode() -> bool {
    STUB_MODE
}

/// 工具白名单 (6 核心 API + 1 状态查询 = 7, 编译期 hardcode).
pub const TOOL_WHITELIST: &[&str] = &[
    "apeireth_livekit_connect",
    "apeireth_livekit_disconnect",
    "apeireth_livekit_publish_track",
    "apeireth_livekit_subscribe",
    "apeireth_livekit_set_camera_enabled",
    "apeireth_livekit_set_microphone_enabled",
    "apeireth_livekit_stub_status",
];

/// 白名单工具数.
pub const TOOL_WHITELIST_COUNT: usize = 7;
const _: () = assert!(TOOL_WHITELIST.len() == TOOL_WHITELIST_COUNT);

/// 校验工具调用是否在白名单内 (m3 防御).
pub fn validate_tool_call(tool: &str, _args: &serde_json::Value) -> Result<(), LiveKitError> {
    if !TOOL_WHITELIST.contains(&tool) {
        return Err(LiveKitError::ToolNotWhitelisted(tool.to_string()));
    }
    Ok(())
}

// ============================================================================
// §3 LiveKitClient trait (6 核心 API)
// ============================================================================

/// 实时音视频信令客户端 (6 核心 API).
#[async_trait]
pub trait LiveKitClient: Send + Sync {
    /// **API 1**: `connect` — 握手连接信令服务并加入房间.
    async fn connect(&self, url: &str, token: &str) -> Result<(), LiveKitError>;

    /// **API 2**: `disconnect` — 优雅离开房间.
    async fn disconnect(&self) -> Result<(), LiveKitError>;

    /// **API 3**: `publish_track` — 发布本地轨道.
    async fn publish_track(&self, track: &Track) -> Result<(), LiveKitError>;

    /// **API 4**: `subscribe` — 订阅远端轨道.
    async fn subscribe(&self, track_sid: &str) -> Result<(), LiveKitError>;

    /// **API 5**: `set_camera_enabled` — 启用 / 禁用摄像头 (幂等).
    async fn set_camera_enabled(&self, enabled: bool) -> Result<(), LiveKitError>;

    /// **API 6**: `set_microphone_enabled` — 启用 / 禁用麦克风 (幂等).
    async fn set_microphone_enabled(&self, enabled: bool) -> Result<(), LiveKitError>;
}

// ============================================================================
// §4 LiveKitClientImpl
// ============================================================================

/// 信令客户端实现: 持有凭证 + 配置 + 会话状态机 + 传输边界 + 事件发射器.
#[derive(Clone)]
pub struct LiveKitClientImpl {
    platform: String,
    api_key_holder: ApiKeyHolder,
    api_secret_holder: ApiSecretHolder,
    url: String,
    room_name: String,
    identity: String,
    room_options: RoomOptions,
    session: Arc<Mutex<Option<SignalingSession>>>,
    transport: Option<Arc<dyn SignalTransport>>,
    emitter: SharedEmitter,
    negotiation: Arc<Mutex<Vec<SignalFrame>>>,
}

impl std::fmt::Debug for LiveKitClientImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveKitClientImpl")
            .field("platform", &self.platform)
            .field("api_key_holder", &self.api_key_holder)
            .field("api_secret_holder", &self.api_secret_holder)
            .field("url", &self.url)
            .field("room_name", &self.room_name)
            .field("identity", &self.identity)
            .field("transport", &self.transport.is_some())
            .field("connected", &self.is_connected())
            .finish_non_exhaustive()
    }
}

impl LiveKitClientImpl {
    /// 创建客户端 (默认房间 `apeireth-room` / 身份 `apeireth-client`, 传输未注入).
    pub fn new() -> Self {
        info!(
            target: "apeireth_livekit",
            platform = PLATFORM_NAME,
            url = DEFAULT_LIVEKIT_URL,
            "signaling client created (protocol layer implemented; transport injected separately)"
        );
        Self {
            platform: PLATFORM_NAME.to_string(),
            api_key_holder: ApiKeyHolder::empty(),
            api_secret_holder: ApiSecretHolder::empty(),
            url: DEFAULT_LIVEKIT_URL.to_string(),
            room_name: "apeireth-room".to_string(),
            identity: "apeireth-client".to_string(),
            room_options: RoomOptions::default(),
            session: Arc::new(Mutex::new(None)),
            transport: None,
            emitter: Arc::new(EventEmitter::new(EVENT_CHANNEL_CAPACITY)),
            negotiation: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 平台名.
    pub fn platform(&self) -> &str {
        &self.platform
    }

    /// 当前信令 URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// 设置信令 URL (K-1 #4 守门).
    pub fn set_url(&mut self, url: String) -> Result<(), LiveKitError> {
        LiveKitError::validate_url(&url)?;
        self.url = url;
        Ok(())
    }

    /// 配置加入房间的房间名 / 身份 (K-1 #3 守门).
    pub fn configure_join(
        &mut self,
        room_name: String,
        identity: String,
    ) -> Result<(), LiveKitError> {
        LiveKitError::validate_room_name(&room_name)?;
        if identity.trim().is_empty() {
            return Err(LiveKitError::InvalidArgument(
                "identity is empty".to_string(),
            ));
        }
        self.room_name = room_name;
        self.identity = identity;
        Ok(())
    }

    /// 房间名.
    pub fn room_name(&self) -> &str {
        &self.room_name
    }

    /// 加入身份.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// 房间配置.
    pub fn room_options(&self) -> &RoomOptions {
        &self.room_options
    }

    /// 设置房间配置 (含重连次数 / 间隔 / 连接超时).
    pub fn set_room_options(&mut self, options: RoomOptions) {
        self.room_options = options;
    }

    /// 注入信令传输 (生产 WebSocket / 测试 mock).
    pub fn set_transport(&mut self, transport: Arc<dyn SignalTransport>) {
        self.transport = Some(transport);
    }

    /// 是否已注入传输.
    pub fn has_transport(&self) -> bool {
        self.transport.is_some()
    }

    /// 房间快照 (未连接为 `None`).
    pub fn room(&self) -> Option<Room> {
        self.session
            .lock()
            .expect("session lock")
            .as_ref()
            .map(|s| s.to_room().expect("room snapshot"))
    }

    /// 当前房间状态 (未连接为 `None`).
    pub fn room_state(&self) -> Option<RoomState> {
        self.session
            .lock()
            .expect("session lock")
            .as_ref()
            .map(|s| s.state())
    }

    /// 是否已连接.
    pub fn is_connected(&self) -> bool {
        self.room_state() == Some(RoomState::Connected)
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

    /// 设置 API key (K-1 #1 守门).
    pub fn set_api_key(&mut self, api_key: String) -> Result<(), LiveKitError> {
        self.api_key_holder.set(api_key)
    }

    /// 设置 API secret (K-1 #2 守门).
    pub fn set_api_secret(&mut self, api_secret: String) -> Result<(), LiveKitError> {
        self.api_secret_holder.set(api_secret)
    }

    /// 健康检查 (本地校验, 零网络).
    pub async fn health_check(&self) -> Result<(), LiveKitError> {
        LiveKitError::validate_url(&self.url)?;
        debug!(target: "apeireth_livekit", url = %self.url, "health_check: local checks ok");
        Ok(())
    }

    /// 5 RoomState 列表.
    pub fn list_room_states() -> &'static [RoomState] {
        SUPPORTED_ROOM_STATES
    }

    /// 8 RoomEvent 列表.
    pub fn list_events() -> &'static [&'static str] {
        SUPPORTED_ROOM_EVENTS
    }

    /// 6 核心 API 名列表.
    pub fn list_apis() -> &'static [&'static str] {
        &TOOL_WHITELIST[..CORE_API_COUNT]
    }

    /// 状态上报 (含协议层实现标志 / 传输注入状态 / 连接状态).
    pub fn stub_status(&self) -> StubStatus {
        StubStatus {
            stub_mode: STUB_MODE,
            platform: self.platform.clone(),
            url: self.url.clone(),
            schema_version: LIVEKIT_SCHEMA_VERSION.to_string(),
            api_key_set: self.api_key_holder.is_set(),
            api_secret_set: self.api_secret_holder.is_set(),
            connected: self.is_connected(),
            room_state: self.room_state(),
            transport_configured: self.has_transport(),
        }
    }

    /// 取出缓冲的 SDP / ICE 协商帧 (WebRTC 会话层消费).
    pub fn drain_negotiation_frames(&self) -> Vec<SignalFrame> {
        std::mem::take(&mut *self.negotiation.lock().expect("negotiation lock"))
    }

    /// 处理传输失联 (进入重连状态机).
    pub fn handle_transport_closed(&self) -> Result<Vec<RoomEvent>, LiveKitError> {
        let mut guard = self.session.lock().expect("session lock");
        let session = guard
            .as_mut()
            .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
        let effects = session.on_transport_closed()?;
        drop(guard);
        Ok(self.apply_local_effects(effects))
    }

    /// 重连定时器到点: 重新握手 (退避次数由状态机管理).
    pub async fn reconnect(&self) -> Result<(), LiveKitError> {
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.on_reconnect_timer()?
        };
        self.apply_effects(&transport, effects).await?;
        self.pump_until_settled(&transport).await
    }

    /// 处理一帧入站信令 (驱动层泵送点): 返回本次产生的房间事件.
    pub async fn pump_once(&self) -> Result<Vec<RoomEvent>, LiveKitError> {
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let frame = transport.recv().await?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.on_frame(frame)?
        };
        Ok(self.apply_local_effects(effects))
    }

    /// 心跳节拍 (驱动层定时器转发).
    pub async fn on_heartbeat_tick(&self) -> Result<Vec<RoomEvent>, LiveKitError> {
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.on_heartbeat_tick()?
        };
        let mut events = Vec::new();
        for effect in effects {
            match effect {
                SessionEffect::Send(frame) => transport.send(frame).await?,
                SessionEffect::Emit(event) => {
                    let _ = self.emitter.emit(event.clone());
                    events.push(event);
                }
                other => self.buffer_non_io_effect(other),
            }
        }
        Ok(events)
    }

    // ---------- 内部: 效果落地 + 握手泵送 ----------

    fn buffer_non_io_effect(&self, effect: SessionEffect) {
        match effect {
            SessionEffect::Negotiation(frame) => {
                let mut buf = self.negotiation.lock().expect("negotiation lock");
                if buf.len() >= MAX_NEGOTIATION_BUFFER {
                    buf.remove(0);
                    warn!(target: "apeireth_livekit", "negotiation buffer full, dropping oldest");
                }
                buf.push(frame);
            }
            SessionEffect::LocalTrackPublished { track_sid } => {
                debug!(target: "apeireth_livekit", track_sid = %track_sid, "local track published");
            }
            SessionEffect::LocalTrackUnpublished { track_sid } => {
                debug!(target: "apeireth_livekit", track_sid = %track_sid, "local track unpublished");
            }
            SessionEffect::Send(_) | SessionEffect::Emit(_) => {}
        }
    }

    fn apply_local_effects(&self, effects: Vec<SessionEffect>) -> Vec<RoomEvent> {
        let mut events = Vec::new();
        for effect in effects {
            match effect {
                SessionEffect::Emit(event) => {
                    let _ = self.emitter.emit(event.clone());
                    events.push(event);
                }
                other => self.buffer_non_io_effect(other),
            }
        }
        events
    }

    async fn apply_effects(
        &self,
        transport: &Arc<dyn SignalTransport>,
        effects: Vec<SessionEffect>,
    ) -> Result<Vec<RoomEvent>, LiveKitError> {
        let mut events = Vec::new();
        for effect in effects {
            match effect {
                SessionEffect::Send(frame) => transport.send(frame).await?,
                SessionEffect::Emit(event) => {
                    let _ = self.emitter.emit(event.clone());
                    events.push(event);
                }
                other => self.buffer_non_io_effect(other),
            }
        }
        Ok(events)
    }

    async fn pump_until_settled(
        &self,
        transport: &Arc<dyn SignalTransport>,
    ) -> Result<(), LiveKitError> {
        let timeout_ms = apeireth_core::deadline::clamp_timeout(
            Some(self.room_options.connect_timeout_secs * 1000),
            DEFAULT_CONNECT_TIMEOUT_MS,
            MAX_CONNECT_TIMEOUT_MS,
        )
        .map_err(|e| LiveKitError::from_deadline(e, "handshake"))?;
        let (_deadline, mut notice) =
            apeireth_core::deadline::Deadline::after(std::time::Duration::from_millis(timeout_ms))
                .map_err(|e| LiveKitError::from_deadline(e, "handshake"))?;

        loop {
            let state = self
                .room_state()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            match state {
                RoomState::Connected => return Ok(()),
                RoomState::Disconnected | RoomState::DisconnectedAlt => {
                    return Err(LiveKitError::ConnectionFailed(
                        "handshake terminated before connected".to_string(),
                    ))
                }
                _ => {}
            }
            tokio::select! {
                _ = notice.notified() => {
                    return Err(LiveKitError::Timeout { operation: "handshake" });
                }
                frame = transport.recv() => {
                    let frame = frame?;
                    let effects = {
                        let mut guard = self.session.lock().expect("session lock");
                        let session = guard.as_mut().expect("session checked above");
                        session.on_frame(frame)?
                    };
                    self.apply_effects(transport, effects).await?;
                }
            }
        }
    }
}

impl Default for LiveKitClientImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// 状态上报 (观测面).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StubStatus {
    /// 协议层实现标志 (恒 `false`: 已全量实现)
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
    /// 当前房间状态
    pub room_state: Option<RoomState>,
    /// 是否已注入信令传输
    pub transport_configured: bool,
}

// ============================================================================
// §5 6 核心 API 实现 (协议真现实现, IO 经传输边界)
// ============================================================================

#[async_trait]
impl LiveKitClient for LiveKitClientImpl {
    #[instrument(skip(self, token), fields(url = %url))]
    async fn connect(&self, url: &str, token: &str) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_connect";
        validate_tool_call(tool_name, &serde_json::json!({ "url": url }))?;
        // K-1 #1/#2: 凭证已配置
        if !self.api_key_holder.is_set() {
            return Err(LiveKitError::ApiKeyMissing);
        }
        if !self.api_secret_holder.is_set() {
            return Err(LiveKitError::ApiSecretMissing);
        }
        // K-1 #4: URL
        LiveKitError::validate_url(url)?;
        // 传输边界: 未注入不假装成功
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        if self.room_state().is_some_and(|s| !s.is_terminal()) {
            return Err(LiveKitError::State(
                "connect called while a session is still active".to_string(),
            ));
        }

        let mut session = SignalingSession::new(
            &self.room_name,
            &self.identity,
            SecretValue::new(token),
            self.room_options.clone(),
        )?;
        let effects = session.begin_connect()?;
        {
            let mut guard = self.session.lock().expect("session lock");
            *guard = Some(session);
        }
        self.apply_effects(&transport, effects).await?;
        self.pump_until_settled(&transport).await
    }

    #[instrument(skip(self))]
    async fn disconnect(&self) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_disconnect";
        validate_tool_call(tool_name, &serde_json::json!({}))?;
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.disconnect()?
        };
        self.apply_effects(&transport, effects).await?;
        Ok(())
    }

    #[instrument(skip(self, track), fields(track_kind = ?track.kind(), track_source = ?track.source()))]
    async fn publish_track(&self, track: &Track) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_publish_track";
        validate_tool_call(
            tool_name,
            &serde_json::json!({ "track_kind": track.kind() }),
        )?;
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.publish_track(track)?
        };
        self.apply_effects(&transport, effects).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(track_sid = %track_sid))]
    async fn subscribe(&self, track_sid: &str) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_subscribe";
        validate_tool_call(tool_name, &serde_json::json!({ "track_sid": track_sid }))?;
        if track_sid.is_empty() {
            return Err(LiveKitError::TrackNotFound("empty track_sid".to_string()));
        }
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.subscribe(track_sid)?
        };
        self.apply_effects(&transport, effects).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(enabled = %enabled))]
    async fn set_camera_enabled(&self, enabled: bool) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_set_camera_enabled";
        validate_tool_call(tool_name, &serde_json::json!({ "enabled": enabled }))?;
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.set_camera_enabled(enabled)?
        };
        self.apply_effects(&transport, effects).await?;
        Ok(())
    }

    #[instrument(skip(self), fields(enabled = %enabled))]
    async fn set_microphone_enabled(&self, enabled: bool) -> Result<(), LiveKitError> {
        let tool_name = "apeireth_livekit_set_microphone_enabled";
        validate_tool_call(tool_name, &serde_json::json!({ "enabled": enabled }))?;
        let transport = self
            .transport
            .clone()
            .ok_or(LiveKitError::TransportUnavailable)?;
        let effects = {
            let mut guard = self.session.lock().expect("session lock");
            let session = guard
                .as_mut()
                .ok_or_else(|| LiveKitError::RoomDisconnected("no active session".to_string()))?;
            session.set_microphone_enabled(enabled)?
        };
        self.apply_effects(&transport, effects).await?;
        Ok(())
    }
}

// ============================================================================
// §6 事件流工具
// ============================================================================

/// 事件订阅便利方法.
pub fn event_subscribe(client: &LiveKitClientImpl) -> broadcast::Receiver<RoomEvent> {
    client.emitter().subscribe()
}

/// 事件发射便利方法 (驱动层 / 测试用).
pub fn event_publish(
    client: &LiveKitClientImpl,
    event: RoomEvent,
) -> Result<usize, broadcast::error::SendError<RoomEvent>> {
    client.emitter().emit(event)
}

/// 事件流 (`Stream<Item = RoomEvent>`): 基于广播订阅的实时流.
///
/// 落后太多 (Lagged) 的订阅者跳到最旧未覆盖事件, 不中断流。
pub fn livekit_event_stream(
    client: &LiveKitClientImpl,
) -> Pin<Box<dyn Stream<Item = RoomEvent> + Send>> {
    let mut rx = client.emitter().subscribe();
    let stream = futures::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => return Some((event, rx)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Box::pin(stream)
}

// ============================================================================
// §7 测试 (mock 边界: 脚本化传输 + 6 核心 API 全流程)
// ============================================================================

#[cfg(test)]
pub(crate) mod mock {
    //! 脚本化传输 mock: 协议层测试的唯一 IO 边界 (零真实网络).

    use super::*;
    use std::collections::VecDeque;

    use async_trait::async_trait;

    use crate::livekit::error::LiveKitError;
    use crate::livekit::signal::{SignalFrame, SignalTransport};

    /// 脚本化信令传输.
    #[derive(Debug, Default)]
    pub struct MockSignalTransport {
        inbound: Mutex<VecDeque<SignalFrame>>,
        sent: Mutex<Vec<SignalFrame>>,
        fail_recv: Mutex<Option<LiveKitError>>,
        stall_when_empty: bool,
    }

    impl MockSignalTransport {
        /// 创建 (空脚本, 脚本耗尽时返传输错误).
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

        /// 压入脚本响应帧.
        pub fn script(&self, frames: Vec<SignalFrame>) {
            let mut q = self.inbound.lock().expect("mock lock");
            q.extend(frames);
        }

        /// 注入 recv 失败.
        pub fn fail_next_recv(&self, err: LiveKitError) {
            *self.fail_recv.lock().expect("mock lock") = Some(err);
        }

        /// 已发送帧 (按序).
        pub fn sent(&self) -> Vec<SignalFrame> {
            self.sent.lock().expect("mock lock").clone()
        }
    }

    #[async_trait]
    impl SignalTransport for MockSignalTransport {
        async fn send(&self, frame: SignalFrame) -> Result<(), LiveKitError> {
            self.sent.lock().expect("mock lock").push(frame);
            Ok(())
        }

        async fn recv(&self) -> Result<SignalFrame, LiveKitError> {
            if let Some(err) = self.fail_recv.lock().expect("mock lock").take() {
                return Err(err);
            }
            if let Some(frame) = self.inbound.lock().expect("mock lock").pop_front() {
                return Ok(frame);
            }
            if self.stall_when_empty {
                futures::future::pending::<()>().await;
            }
            Err(LiveKitError::Network(
                "mock: scripted frames exhausted".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mock::MockSignalTransport;
    use super::*;
    use crate::error_taxonomy::{ClassifyError, ErrorCategory};

    fn ready_client() -> (LiveKitClientImpl, Arc<MockSignalTransport>) {
        let mut client = LiveKitClientImpl::new();
        client
            .set_api_key("API12345678".to_string())
            .expect("valid api key");
        client
            .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid api secret");
        client
            .configure_join("room-1".to_string(), "user-1".to_string())
            .expect("valid join");
        let transport = Arc::new(MockSignalTransport::new());
        client.set_transport(transport.clone());
        (client, transport)
    }

    fn scripted_handshake() -> Vec<SignalFrame> {
        vec![
            SignalFrame::Welcome {
                protocol_version: PROTOCOL_VERSION,
                session_id: "sess-1".to_string(),
                heartbeat_interval_ms: 5_000,
            },
            SignalFrame::JoinAccepted {
                room_sid: "RM_1".to_string(),
                participant_sid: "PA_SELF".to_string(),
                participants: vec![],
            },
        ]
    }

    #[tokio::test]
    async fn connect_runs_handshake_and_reports_connected() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());

        let mut rx = client.emitter().subscribe();
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        assert!(client.is_connected());
        assert_eq!(client.room_state(), Some(RoomState::Connected));

        // 发出 Hello + Join (顺序)
        let sent = transport.sent();
        assert!(matches!(&sent[0], SignalFrame::Hello { .. }));
        assert!(matches!(&sent[1], SignalFrame::Join { .. }));

        // 状态迁移事件必须发到事件面
        let mut states = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            if let RoomEvent::ConnectionStateChanged { previous, current } = ev {
                states.push((previous, current));
            }
        }
        assert_eq!(
            states,
            vec![
                (RoomState::Disconnected, RoomState::Connecting),
                (RoomState::Connecting, RoomState::Connected),
            ]
        );
    }

    #[tokio::test]
    async fn connect_never_leaks_token_into_logs_or_wire_debug() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-SECRET-VALUE")
            .await
            .expect("connect");

        let sent = transport.sent();
        let dbg = format!("{sent:?}");
        assert!(
            !dbg.contains("join-token-SECRET-VALUE"),
            "发送帧 Debug 不得回显令牌: {dbg}"
        );
        assert!(dbg.contains("[redacted]"));
    }

    #[tokio::test]
    async fn connect_guards_credentials_and_url() {
        let mut client = LiveKitClientImpl::new();
        // 缺 API key
        assert!(matches!(
            client.connect("wss://signal.example.com", "t").await,
            Err(LiveKitError::ApiKeyMissing)
        ));
        client
            .set_api_key("API12345678".to_string())
            .expect("valid");
        // 缺 API secret
        assert!(matches!(
            client.connect("wss://signal.example.com", "t").await,
            Err(LiveKitError::ApiSecretMissing)
        ));
        client
            .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        // URL 不合法
        assert!(matches!(
            client.connect("http://signal.example.com", "t").await,
            Err(LiveKitError::InvalidUrl(_))
        ));
    }

    #[tokio::test]
    async fn connect_without_transport_is_explicit_error() {
        let mut client = LiveKitClientImpl::new();
        client
            .set_api_key("API12345678".to_string())
            .expect("valid");
        client
            .set_api_secret("abcdef1234567890abcdef1234567890".to_string())
            .expect("valid");
        let err = client
            .connect("wss://signal.example.com", "t")
            .await
            .expect_err("no transport must fail");
        assert_eq!(err, LiveKitError::TransportUnavailable);
        assert!(!err.is_retryable());
    }

    #[tokio::test]
    async fn connect_join_rejection_classifies_authentication() {
        let (client, transport) = ready_client();
        transport.script(vec![
            SignalFrame::Welcome {
                protocol_version: PROTOCOL_VERSION,
                session_id: "sess-1".to_string(),
                heartbeat_interval_ms: 5_000,
            },
            SignalFrame::JoinRejected {
                reason: "invalid_token".to_string(),
                message: "token rejected".to_string(),
            },
        ]);
        let err = client
            .connect("wss://signal.example.com", "bad-token")
            .await
            .expect_err("must reject");
        assert_eq!(err.category(), ErrorCategory::Authentication);
    }

    #[tokio::test]
    async fn connect_timeout_uses_deadline_fusion() {
        let mut options = RoomOptions::default();
        options.connect_timeout_secs = 1;
        let (mut client, _transport) = ready_client();
        client.set_room_options(options);
        // 无脚本 + 永久挂起: 只能靠超时熔合退出
        let stalling = Arc::new(MockSignalTransport::stalling());
        client.set_transport(stalling.clone());

        let err = client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect_err("must time out");
        assert!(matches!(
            err,
            LiveKitError::Timeout {
                operation: "handshake"
            }
        ));
        assert_eq!(err.category(), ErrorCategory::Timeout);
    }

    #[tokio::test]
    async fn connect_transport_failure_is_network_class() {
        let (client, transport) = ready_client();
        transport.fail_next_recv(LiveKitError::Network("connection reset".to_string()));
        let err = client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect_err("must fail");
        assert_eq!(err.category(), ErrorCategory::Network);
        assert!(err.is_retryable());
    }

    #[tokio::test]
    async fn disconnect_sends_leave_and_reports_graceful_terminal() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        client.disconnect().await.expect("disconnect");
        assert_eq!(client.room_state(), Some(RoomState::DisconnectedAlt));
        let sent = transport.sent();
        assert!(matches!(sent.last(), Some(SignalFrame::Leave)));

        // 未连接再 disconnect → 状态错误
        let err = client.disconnect().await.expect_err("must fail");
        assert_eq!(err.category(), ErrorCategory::State);
    }

    #[tokio::test]
    async fn publish_track_and_ack_via_pump() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        let track = Track::new(TrackKind::Video, TrackSource::Camera);
        client.publish_track(&track).await.expect("publish");
        let sent = transport.sent();
        assert!(matches!(
            sent.last(),
            Some(SignalFrame::TrackPublish {
                kind: TrackKind::Video,
                source: TrackSource::Camera,
                ..
            })
        ));

        // 服务端回执经 pump_once 落地
        transport.script(vec![SignalFrame::TrackPublished {
            track_sid: "TR_1".to_string(),
            kind: TrackKind::Video,
            source: TrackSource::Camera,
        }]);
        let events = client.pump_once().await.expect("pump");
        assert!(events.is_empty(), "本地发布不产房间事件");
        assert!(client.drain_negotiation_frames().is_empty());
    }

    #[tokio::test]
    async fn publish_track_requires_connection() {
        let (client, _transport) = ready_client();
        let track = Track::new(TrackKind::Audio, TrackSource::Microphone);
        let err = client.publish_track(&track).await.expect_err("must fail");
        assert_eq!(err.category(), ErrorCategory::State);
    }

    #[tokio::test]
    async fn subscribe_roundtrip_emits_track_subscribed_event() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        client.subscribe("TR_9").await.expect("subscribe");
        assert!(matches!(
            transport.sent().last(),
            Some(SignalFrame::TrackSubscribe { track_sid }) if track_sid == "TR_9"
        ));

        transport.script(vec![SignalFrame::TrackSubscribed {
            track_sid: "TR_9".to_string(),
            participant_sid: "PA_2".to_string(),
            source: TrackSource::ScreenShare,
        }]);
        let events = client.pump_once().await.expect("pump");
        assert!(matches!(
            &events[0],
            RoomEvent::TrackSubscribed { track_sid, .. } if track_sid == "TR_9"
        ));

        // 空 SID → TrackNotFound
        let err = client.subscribe("").await.expect_err("must fail");
        assert!(matches!(err, LiveKitError::TrackNotFound(_)));
    }

    #[tokio::test]
    async fn device_toggles_go_through_track_lifecycle() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        client.set_camera_enabled(true).await.expect("enable cam");
        client
            .set_microphone_enabled(true)
            .await
            .expect("enable mic");
        let sent = transport.sent();
        let publishes: Vec<_> = sent
            .iter()
            .filter(|f| matches!(f, SignalFrame::TrackPublish { .. }))
            .collect();
        assert_eq!(publishes.len(), 2);

        // 回执后禁用 → 发撤下帧
        transport.script(vec![
            SignalFrame::TrackPublished {
                track_sid: "TR_CAM".to_string(),
                kind: TrackKind::Video,
                source: TrackSource::Camera,
            },
            SignalFrame::TrackPublished {
                track_sid: "TR_MIC".to_string(),
                kind: TrackKind::Audio,
                source: TrackSource::Microphone,
            },
        ]);
        client.pump_once().await.expect("pump cam");
        client.pump_once().await.expect("pump mic");

        client.set_camera_enabled(false).await.expect("disable cam");
        assert!(matches!(
            transport.sent().last(),
            Some(SignalFrame::TrackUnpublish { track_sid }) if track_sid == "TR_CAM"
        ));
    }

    #[tokio::test]
    async fn full_reconnect_flow_reaches_connected_and_emits_reconnected() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        // 传输失联 → Reconnecting
        let events = client.handle_transport_closed().expect("close");
        assert_eq!(client.room_state(), Some(RoomState::Reconnecting));
        assert!(matches!(
            &events[0],
            RoomEvent::ConnectionStateChanged {
                previous: RoomState::Connected,
                current: RoomState::Reconnecting,
            }
        ));

        // 重连定时器 → 重新握手成功
        transport.script(scripted_handshake());
        let mut rx = client.emitter().subscribe();
        client.reconnect().await.expect("reconnect");
        assert!(client.is_connected());

        let mut saw_reconnected = false;
        while let Ok(ev) = rx.try_recv() {
            if matches!(ev, RoomEvent::Reconnected { .. }) {
                saw_reconnected = true;
            }
        }
        assert!(saw_reconnected, "重连成功必须发 Reconnected 事件");
    }

    #[tokio::test]
    async fn negotiation_frames_are_buffered_for_webrtc_layer() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        transport.script(vec![
            SignalFrame::Offer {
                sdp: "v=0".to_string(),
            },
            SignalFrame::IceCandidate {
                candidate: "candidate:1".to_string(),
                sdp_mid: Some("0".to_string()),
                sdp_mline_index: Some(0),
            },
        ]);
        client.pump_once().await.expect("pump offer");
        client.pump_once().await.expect("pump ice");
        let frames = client.drain_negotiation_frames();
        assert_eq!(frames.len(), 2);
        assert!(matches!(frames[0], SignalFrame::Offer { .. }));
        assert!(matches!(frames[1], SignalFrame::IceCandidate { .. }));
    }

    #[tokio::test]
    async fn event_stream_delivers_room_events() {
        let (client, _transport) = ready_client();
        let mut stream = livekit_event_stream(&client);
        event_publish(
            &client,
            RoomEvent::Reconnected {
                disconnected_at: None,
            },
        )
        .expect("publish");
        let got = futures::StreamExt::next(&mut stream)
            .await
            .expect("stream must yield");
        assert_eq!(
            got,
            RoomEvent::Reconnected {
                disconnected_at: None
            }
        );
    }

    #[tokio::test]
    async fn server_error_frames_classify_into_closed_vocabulary() {
        let (client, transport) = ready_client();
        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");

        transport.script(vec![SignalFrame::Error {
            code: "rate_limited".to_string(),
            message: "slow down".to_string(),
            fatal: true,
            retry_after_ms: Some(3_000),
        }]);
        let err = client.pump_once().await.expect_err("must error");
        assert_eq!(err.category(), ErrorCategory::RateLimited);
        assert_eq!(err.retry_after_ms(), Some(3_000));
    }

    #[test]
    fn constants_and_whitelist_stay_pinned() {
        assert_eq!(SCHEMA_VERSION, "1");
        assert_eq!(PLATFORM_NAME, "apeireth");
        assert_eq!(PROVIDER_NAME, "livekit");
        assert!(!is_stub_mode());
        assert!(!STUB_MODE);
        assert_eq!(TOOL_WHITELIST.len(), TOOL_WHITELIST_COUNT);
        assert_eq!(LiveKitClientImpl::list_apis().len(), CORE_API_COUNT);
        assert_eq!(
            LiveKitClientImpl::list_room_states().len(),
            ROOM_STATE_COUNT
        );
        assert_eq!(LiveKitClientImpl::list_events().len(), ROOM_EVENT_COUNT);
    }

    #[test]
    fn validate_tool_call_accepts_whitelist_and_rejects_unknown() {
        let args = serde_json::json!({});
        assert!(validate_tool_call("apeireth_livekit_connect", &args).is_ok());
        assert!(validate_tool_call("apeireth_livekit_stub_status", &args).is_ok());
        let err = validate_tool_call("apeireth_livekit_bogus", &args).unwrap_err();
        assert!(matches!(err, LiveKitError::ToolNotWhitelisted(_)));
    }

    #[tokio::test]
    async fn status_reports_real_state_including_transport() {
        let (client, transport) = ready_client();
        let status = client.stub_status();
        assert!(!status.stub_mode);
        assert!(status.transport_configured);
        assert!(!status.connected);

        transport.script(scripted_handshake());
        client
            .connect("wss://signal.example.com", "join-token-123")
            .await
            .expect("connect");
        let status = client.stub_status();
        assert!(status.connected);
        assert_eq!(status.room_state, Some(RoomState::Connected));
    }

    #[tokio::test]
    async fn health_check_validates_url_locally() {
        let (mut client, _transport) = ready_client();
        assert!(client.health_check().await.is_ok());
        client.url = "http://bad".to_string();
        assert!(client.health_check().await.is_err());
    }

    #[tokio::test]
    async fn session_constructor_rejects_bad_join_config() {
        let (mut client, _transport) = ready_client();
        assert!(matches!(
            client.configure_join("bad room".to_string(), "u".to_string()),
            Err(LiveKitError::RoomNameInvalid(_))
        ));
        assert!(matches!(
            client.configure_join("room-1".to_string(), " ".to_string()),
            Err(LiveKitError::InvalidArgument(_))
        ));
    }
}
