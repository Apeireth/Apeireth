//! LiveKit 房间管理 (per livekit-client v0.9.21 1:1 翻译)
//!
//! 1:1 翻译 v0.9.21 商业版 `Room` class (per livekit-client/dist/src/room/Room.d.ts):
//! - `Room` (per RoomOptions)
//! - `RoomState` (5 状态机: Disconnected / Connecting / Connected / Reconnecting / Disconnected)
//! - `RoomEvent` (8 事件: 在 event.rs 详细定义)
//!
//! **5 状态机** (per v0.9.21 商业版 `ConnectionState` enum):
//!   1. `Disconnected` (初始 / 已断开)
//!   2. `Connecting` (正在连接 wss:// URL)
//!   3. `Connected` (已连接, 可 publish/subscribe tracks)
//!   4. `Reconnecting` (网络抖动自动重连)
//!   5. `Disconnected` (最终态, 需显式 `connect()` 重新连接)
//!
//! **当前 skeleton 不真接 livekit-server**, 状态机用 atomic bool 模拟, R21 续真接时换 WebSocket.

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::livekit::error::LiveKitError;

// ============================================================================
// §1 RoomState 5 状态机 (per v0.9.21 商业版 ConnectionState enum 1:1)
// ============================================================================

/// 房间状态 (5 状态机, K-1 强校验守门: 编译期 hardcode 5 个 variant).
///
/// 字段对应 v0.9.21 商业版 `ConnectionState` enum:
/// `disconnected` / `connecting` / `connected` / `reconnecting` (per livekit-client).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomState {
    /// **初始态 / 终态**: 房间已断开, 需 `connect()` 重新连接.
    #[default]
    Disconnected,
    /// **过渡态**: 正在连接 wss:// URL, 等待 LiveKit 服务端 ACK.
    Connecting,
    /// **目标态**: 已连接, 可 publish / subscribe tracks / 收发 data.
    Connected,
    /// **过渡态**: 网络抖动, 自动重连中 (per livekit-client 自动重连).
    Reconnecting,
    /// **别名**: 同 Disconnected, 区分中英文表述.
    DisconnectedAlt,
}

impl RoomState {
    /// 5 状态机 hardcode 常量.
    pub const COUNT: usize = 5;

    /// 状态字符串 (1:1 翻译 livekit-client v0.9.21 `ConnectionState` snake_case).
    pub fn as_str(&self) -> &'static str {
        match self {
            RoomState::Disconnected => "disconnected",
            RoomState::Connecting => "connecting",
            RoomState::Connected => "connected",
            RoomState::Reconnecting => "reconnecting",
            RoomState::DisconnectedAlt => "disconnected_alt",
        }
    }

    /// 从字符串解析 (per livekit-server signal JSON `state` 字段).
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "disconnected" => Some(RoomState::Disconnected),
            "connecting" => Some(RoomState::Connecting),
            "connected" => Some(RoomState::Connected),
            "reconnecting" => Some(RoomState::Reconnecting),
            "disconnected_alt" => Some(RoomState::DisconnectedAlt),
            _ => None,
        }
    }

    /// 是否为终态 (per `Disconnected` 守门).
    pub fn is_terminal(&self) -> bool {
        matches!(self, RoomState::Disconnected | RoomState::DisconnectedAlt)
    }

    /// 是否可 publish tracks (仅 `Connected` 状态允许).
    pub fn can_publish(&self) -> bool {
        matches!(self, RoomState::Connected)
    }

    /// 是否可 subscribe tracks (`Connected` + `Reconnecting` 都允许).
    pub fn can_subscribe(&self) -> bool {
        matches!(self, RoomState::Connected | RoomState::Reconnecting)
    }
}

impl fmt::Display for RoomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 编译期守门: SUPPORTED_ROOM_STATES 长度 == 5 (K-1 强校验 #5: 5 状态机 hardcode).
pub const SUPPORTED_ROOM_STATES: &[RoomState] = &[
    RoomState::Disconnected,
    RoomState::Connecting,
    RoomState::Connected,
    RoomState::Reconnecting,
    RoomState::DisconnectedAlt,
];
const _: () = assert!(SUPPORTED_ROOM_STATES.len() == 5);

// ============================================================================
// §2 RoomOptions (per v0.9.21 商业版 RoomOptions interface 1:1)
// ============================================================================

/// 房间配置 (per v0.9.21 商业版 `RoomOptions` interface 1:1).
///
/// 字段对应 livekit-client v0.9.21 RoomOptions:
/// - `adaptiveStream` (per AdaptiveStreamSettings)
/// - `dynacast` (per 启用 dynacast)
/// - `publishDefaults` (per TrackPublishDefaults)
/// - `autoSubscribe` (per 自动 subscribe remote tracks)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomOptions {
    /// 是否自动 subscribe 远端 track (默认 true, per livekit-client 默认).
    #[serde(default = "default_auto_subscribe")]
    pub auto_subscribe: bool,
    /// 是否启用 dynacast (动态分辨率, 默认 true).
    #[serde(default = "default_dynacast")]
    pub dynacast: bool,
    /// 是否启用 adaptive stream (默认 true, per livekit-client 默认).
    #[serde(default = "default_adaptive_stream")]
    pub adaptive_stream: bool,
    /// 连接超时 (秒, 默认 10s, per livekit-client 默认).
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
    /// 重连间隔 (秒, 默认 2s, per livekit-client 默认).
    #[serde(default = "default_reconnect_interval_secs")]
    pub reconnect_interval_secs: u64,
    /// 最大重连次数 (默认 5, per livekit-client 默认).
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
}

fn default_auto_subscribe() -> bool {
    true
}
fn default_dynacast() -> bool {
    true
}
fn default_adaptive_stream() -> bool {
    true
}
fn default_connect_timeout_secs() -> u64 {
    10
}
fn default_reconnect_interval_secs() -> u64 {
    2
}
fn default_max_reconnect_attempts() -> u32 {
    5
}

impl Default for RoomOptions {
    fn default() -> Self {
        Self {
            auto_subscribe: default_auto_subscribe(),
            dynacast: default_dynacast(),
            adaptive_stream: default_adaptive_stream(),
            connect_timeout_secs: default_connect_timeout_secs(),
            reconnect_interval_secs: default_reconnect_interval_secs(),
            max_reconnect_attempts: default_max_reconnect_attempts(),
        }
    }
}

// ============================================================================
// §3 Room 主结构 (5 状态机 atomic + 占位, R21 续真接 livekit-server)
// ============================================================================

/// LiveKit 房间 (per v0.9.21 商业版 `Room` class 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 Room (估 6 fields):
/// - `state` (per AtomicU8 模拟 5 状态机)
/// - `name` (per Room.name 字段)
/// - `sid` (per Room.sid, 服务端分配, R21 真接才有)
/// - `options` (per RoomOptions)
/// - `participants` (per HashMap<ParticipantSid, Participant>, R21 续)
/// - `local_participant` (per local Participant, R21 续)
#[derive(Debug)]
pub struct Room {
    /// 房间名 (per v0.9.21 商业版 `Room.name`)
    name: String,
    /// 房间 SID (服务端分配, R21 真接 livekit-server 后填, STUB 模式空)
    sid: Option<String>,
    /// 当前状态 (5 状态机, AtomicU8 模拟)
    state: AtomicU8,
    /// 房间配置
    options: RoomOptions,
    /// 上次状态变更时间戳 (秒, UNIX_EPOCH 起, R21 续接 livekit-server 时记)
    last_state_change_secs: u64,
}

// 手动 impl Clone (AtomicU8 没 impl Clone, 但 Room 可以通过 sid + name + options clone + state AtomicU8::new 重建)
impl Clone for Room {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            sid: self.sid.clone(),
            state: AtomicU8::new(self.state.load(Ordering::SeqCst)),
            options: self.options.clone(),
            last_state_change_secs: self.last_state_change_secs,
        }
    }
}

impl Room {
    /// 创建新房间 (STUB 模式不真接 livekit-server).
    ///
    /// **当前 skeleton 仅存 name + state = Disconnected**, R21 续真接 livekit-server.
    pub fn new(name: impl Into<String>, options: RoomOptions) -> Result<Self, LiveKitError> {
        let name_str = name.into();
        LiveKitError::validate_room_name(&name_str)?;
        Ok(Self {
            name: name_str,
            sid: None,
            state: AtomicU8::new(RoomState::Disconnected as u8),
            options,
            last_state_change_secs: 0,
        })
    }

    /// 房间名 (per v0.9.21 商业版 `Room.name`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 房间 SID (服务端分配, R21 续).
    pub fn sid(&self) -> Option<&str> {
        self.sid.as_deref()
    }

    /// 设置房间 SID (R21 续真接时由 `connect()` 调).
    pub fn set_sid(&mut self, sid: String) {
        self.sid = Some(sid);
    }

    /// 当前状态 (5 状态机, per v0.9.21 商业版 `Room.state`).
    pub fn state(&self) -> RoomState {
        // 安全的转换: 0..=4 范围内, 否则 fallback Disconnected
        let raw = self.state.load(Ordering::SeqCst);
        match raw {
            0 => RoomState::Disconnected,
            1 => RoomState::Connecting,
            2 => RoomState::Connected,
            3 => RoomState::Reconnecting,
            4 => RoomState::DisconnectedAlt,
            _ => RoomState::Disconnected,
        }
    }

    /// 设置状态 (per v0.9.21 商业版 `Room.setState` 内部, R21 续由 signal protocol 调).
    pub fn set_state(&mut self, new_state: RoomState) {
        self.state.store(new_state as u8, Ordering::SeqCst);
        self.last_state_change_secs = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// 是否已连接 (per `can_publish` 守门).
    pub fn is_connected(&self) -> bool {
        self.state() == RoomState::Connected
    }

    /// 房间配置.
    pub fn options(&self) -> &RoomOptions {
        &self.options
    }

    /// 上次状态变更时间戳 (秒, UNIX_EPOCH 起).
    pub fn last_state_change_secs(&self) -> u64 {
        self.last_state_change_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_state_5_variants() {
        assert_eq!(SUPPORTED_ROOM_STATES.len(), 5);
        assert_eq!(RoomState::COUNT, 5);
    }

    #[test]
    fn room_state_as_str() {
        assert_eq!(RoomState::Disconnected.as_str(), "disconnected");
        assert_eq!(RoomState::Connecting.as_str(), "connecting");
        assert_eq!(RoomState::Connected.as_str(), "connected");
        assert_eq!(RoomState::Reconnecting.as_str(), "reconnecting");
        assert_eq!(RoomState::DisconnectedAlt.as_str(), "disconnected_alt");
    }

    #[test]
    fn room_state_parse_roundtrip() {
        for state in SUPPORTED_ROOM_STATES {
            assert_eq!(RoomState::parse(state.as_str()), Some(*state));
        }
        assert_eq!(RoomState::parse("unknown"), None);
    }

    #[test]
    fn room_state_is_terminal() {
        assert!(RoomState::Disconnected.is_terminal());
        assert!(RoomState::DisconnectedAlt.is_terminal());
        assert!(!RoomState::Connecting.is_terminal());
        assert!(!RoomState::Connected.is_terminal());
        assert!(!RoomState::Reconnecting.is_terminal());
    }

    #[test]
    fn room_state_can_publish() {
        assert!(RoomState::Connected.can_publish());
        assert!(!RoomState::Disconnected.can_publish());
        assert!(!RoomState::Connecting.can_publish());
        assert!(!RoomState::Reconnecting.can_publish());
        assert!(!RoomState::DisconnectedAlt.can_publish());
    }

    #[test]
    fn room_state_can_subscribe() {
        assert!(RoomState::Connected.can_subscribe());
        assert!(RoomState::Reconnecting.can_subscribe());
        assert!(!RoomState::Disconnected.can_subscribe());
        assert!(!RoomState::Connecting.can_subscribe());
    }

    #[test]
    fn room_state_display() {
        assert_eq!(format!("{}", RoomState::Connected), "connected");
    }

    #[test]
    fn room_state_default() {
        let default_state: RoomState = Default::default();
        assert_eq!(default_state, RoomState::Disconnected);
    }

    #[test]
    fn room_options_default() {
        let opts = RoomOptions::default();
        assert!(opts.auto_subscribe);
        assert!(opts.dynacast);
        assert!(opts.adaptive_stream);
        assert_eq!(opts.connect_timeout_secs, 10);
        assert_eq!(opts.reconnect_interval_secs, 2);
        assert_eq!(opts.max_reconnect_attempts, 5);
    }

    #[test]
    fn room_creation_valid() {
        let room = Room::new("my-room-1", RoomOptions::default()).unwrap();
        assert_eq!(room.name(), "my-room-1");
        assert_eq!(room.state(), RoomState::Disconnected);
        assert!(!room.is_connected());
    }

    #[test]
    fn room_creation_invalid_name() {
        let result = Room::new("", RoomOptions::default());
        assert!(matches!(result, Err(LiveKitError::RoomNameEmpty)));
    }

    #[test]
    fn room_state_transition() {
        let mut room = Room::new("my-room-1", RoomOptions::default()).unwrap();
        assert_eq!(room.state(), RoomState::Disconnected);
        room.set_state(RoomState::Connecting);
        assert_eq!(room.state(), RoomState::Connecting);
        room.set_state(RoomState::Connected);
        assert_eq!(room.state(), RoomState::Connected);
        assert!(room.is_connected());
        room.set_state(RoomState::Reconnecting);
        assert_eq!(room.state(), RoomState::Reconnecting);
        room.set_state(RoomState::Disconnected);
        assert!(!room.is_connected());
    }

    #[test]
    fn room_sid_default_none() {
        let room = Room::new("my-room-1", RoomOptions::default()).unwrap();
        assert!(room.sid().is_none());
        assert_eq!(room.last_state_change_secs(), 0);
    }

    #[test]
    fn room_sid_set() {
        let mut room = Room::new("my-room-1", RoomOptions::default()).unwrap();
        room.set_sid("RM_xxxxxxxxxxxxx".to_string());
        assert_eq!(room.sid(), Some("RM_xxxxxxxxxxxxx"));
    }
}
