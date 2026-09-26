//! 信令会话状态机 + 媒体会话生命周期 (加入 / 离开 / 轨道发布订阅).
//!
//! 纯状态机: 所有迁移由 [`SessionInput`] 驱动、以 [`SessionEffect`] 输出,
//! 不做任何 IO —— IO 由驱动层 (客户端) 经 [`crate::livekit::signal::SignalTransport`]
//! 边界完成。这让"状态机全迁移 (含断线重连)"可以在零真实网络下逐迁移钉死。
//!
//! ## 5 状态 (复用 [`RoomState`])
//!
//! | 状态 | 含义 |
//! |---|---|
//! | `Disconnected` | 未连接 / 已终止, 需显式重连 |
//! | `Connecting` | 握手进行中 (Hello/Welcome/Join 序列) |
//! | `Connected` | 已入会, 可发布订阅轨道 / 收发数据 |
//! | `Reconnecting` | 传输失联或心跳超限, 自动重连中 |
//! | `DisconnectedAlt` | 主动离开 (优雅关闭) 的终止态 |
//!
//! ## 迁移表 (全部覆盖于测试)
//!
//! | 从 | 触发 | 到 | 附带效应 |
//! |---|---|---|---|
//! | Disconnected / DisconnectedAlt | `begin_connect` | Connecting | Send Hello+Join |
//! | Connecting | `Welcome` | Connecting | 校验版本, 记 session/心跳参数 |
//! | Connecting | `JoinAccepted` | Connected | Emit 状态迁移 + 已有参与者事件 |
//! | Connecting | `JoinRejected` | Disconnected | 返分类错误 (认证/限流/协议) |
//! | Connecting | `on_transport_closed` | Disconnected | Emit 状态迁移 |
//! | Connecting | `disconnect` | DisconnectedAlt | Emit 状态迁移 |
//! | Connected | `on_heartbeat_tick` | Connected | Send Heartbeat; 连续无响应 → Reconnecting |
//! | Connected | `HeartbeatAck` | Connected | 清零心跳丢失计数 |
//! | Connected | `on_transport_closed` | Reconnecting | Emit 状态迁移, 记断开时刻 |
//! | Connected / Reconnecting | `disconnect` | DisconnectedAlt | Send Leave + Emit |
//! | Connected | fatal `Error` 帧 | Disconnected | 返分类错误 |
//! | Reconnecting | `on_reconnect_timer` | Reconnecting | Send Hello+Join (退避计数++) |
//! | Reconnecting | `JoinAccepted` | Connected | Emit `Reconnected` + 状态迁移 |
//! | Reconnecting | 重连次数耗尽 | Disconnected | Emit 状态迁移 |
//!
//! 非法输入 (未连接先发布 / 未知轨道订阅回执 / 重复订阅等) 一律
//! [`LiveKitError::Protocol`] 或 [`LiveKitError::State`], 零静默吞掉。

use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::livekit::error::LiveKitError;
use crate::livekit::event::RoomEvent;
use crate::livekit::participant::{Participant, ParticipantSid};
use crate::livekit::room::{Room, RoomOptions, RoomState};
use crate::livekit::signal::{
    chunk_data_message, DataMessageAssembler, ParticipantInfo, SignalFrame,
    DEFAULT_HEARTBEAT_INTERVAL_MS, MAX_HEARTBEAT_MISSED, PROTOCOL_VERSION,
};
use crate::livekit::track::{Track, TrackKind, TrackSid, TrackSource};
use crate::redact::SecretValue;

// ============================================================================
// §1 心跳 / 重连策略
// ============================================================================

/// 心跳策略 (间隔 / 超时 / 最大连续丢失).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeartbeatPolicy {
    /// 心跳间隔.
    pub interval: Duration,
    /// 单次心跳响应超时.
    pub timeout: Duration,
    /// 连续丢失上限 (达到即进入重连).
    pub max_missed: u32,
}

impl Default for HeartbeatPolicy {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(DEFAULT_HEARTBEAT_INTERVAL_MS),
            timeout: Duration::from_millis(crate::livekit::signal::DEFAULT_HEARTBEAT_TIMEOUT_MS),
            max_missed: MAX_HEARTBEAT_MISSED,
        }
    }
}

/// 重连策略 (基数退避 + 次数上限).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    /// 退避基数.
    pub base_interval: Duration,
    /// 最大重连次数.
    pub max_attempts: u32,
    /// 退避上限.
    pub max_delay: Duration,
}

impl ReconnectPolicy {
    /// 从房间配置推导.
    pub fn from_options(options: &RoomOptions) -> Self {
        Self {
            base_interval: Duration::from_secs(options.reconnect_interval_secs.max(1)),
            max_attempts: options.max_reconnect_attempts.max(1),
            max_delay: Duration::from_secs(30),
        }
    }

    /// 第 `attempt` 次重连的退避时长 (指数退避, 封顶 `max_delay`).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let exp = attempt.saturating_sub(1).min(16);
        let scaled = self.base_interval.saturating_mul(1u32 << exp);
        scaled.min(self.max_delay)
    }
}

// ============================================================================
// §2 状态机输出
// ============================================================================

/// 状态机输出: 驱动层据此发帧 / 发事件 / 通知 WebRTC 层.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionEffect {
    /// 需要发送一帧信令.
    Send(SignalFrame),
    /// 需要向事件订阅面发布房间事件.
    Emit(RoomEvent),
    /// SDP / ICE 协商帧透传给 WebRTC 会话层 (信令层不解析 SDP).
    Negotiation(SignalFrame),
    /// 本地轨道发布完成 (服务端已分配 SID).
    LocalTrackPublished {
        /// 轨道 SID.
        track_sid: TrackSid,
    },
    /// 本地轨道撤下完成.
    LocalTrackUnpublished {
        /// 轨道 SID.
        track_sid: TrackSid,
    },
}

/// 会话统计快照 (观测面, 测试 / 监控共用).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStats {
    /// 当前状态.
    pub state: RoomState,
    /// 已发送信令帧数.
    pub frames_sent: u64,
    /// 已接收信令帧数.
    pub frames_received: u64,
    /// 心跳已发次数.
    pub heartbeats_sent: u64,
    /// 当前连续心跳丢失数.
    pub heartbeat_missed: u32,
    /// 当前重连尝试次数.
    pub reconnect_attempts: u32,
    /// 本地已发布 / 发布中轨道数.
    pub local_tracks: usize,
    /// 远端订阅数.
    pub remote_subscriptions: usize,
}

// ============================================================================
// §3 会话本体
// ============================================================================

/// 本地轨道记录 (发布中 / 已发布 / 撤下中).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTrackRecord {
    /// 轨道 SID (服务端分配前为空).
    pub track_sid: Option<TrackSid>,
    /// 轨道类型.
    pub kind: TrackKind,
    /// 轨道来源.
    pub source: TrackSource,
    /// 显示名.
    pub name: Option<String>,
    /// 生命周期相位.
    pub phase: LocalTrackPhase,
}

/// 本地轨道生命周期相位.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTrackPhase {
    /// 已发 `TrackPublish`, 等 `TrackPublished`.
    Publishing,
    /// 已发布.
    Published,
    /// 已发 `TrackUnpublish`, 等 `TrackUnpublished`.
    Unpublishing,
}

/// 远端订阅记录.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackSubscription {
    /// 轨道 SID.
    pub track_sid: TrackSid,
    /// 轨道所属参与者 (订阅回执后可知).
    pub participant_sid: Option<ParticipantSid>,
    /// 轨道来源 (订阅回执后可知).
    pub source: Option<TrackSource>,
    /// 是否已完成订阅.
    pub subscribed: bool,
}

/// 信令会话状态机 (纯逻辑, 零 IO).
#[derive(Debug)]
pub struct SignalingSession {
    room_name: String,
    identity: String,
    token: SecretValue,
    options: RoomOptions,
    heartbeat_policy: HeartbeatPolicy,
    reconnect_policy: ReconnectPolicy,
    state: RoomState,
    session_id: Option<String>,
    room_sid: Option<String>,
    local_sid: Option<ParticipantSid>,
    participants: BTreeMap<ParticipantSid, Participant>,
    local_tracks: Vec<LocalTrackRecord>,
    remote_tracks: BTreeMap<TrackSid, TrackSubscription>,
    data_assembler: DataMessageAssembler,
    data_meta: HashMap<String, (ParticipantSid, bool)>,
    heartbeat_missed: u32,
    awaiting_heartbeat_ack: bool,
    reconnect_attempts: u32,
    disconnected_at: Option<SystemTime>,
    frames_sent: u64,
    frames_received: u64,
    heartbeats_sent: u64,
}

impl SignalingSession {
    /// 创建会话 (初始态 `Disconnected`).
    pub fn new(
        room_name: impl Into<String>,
        identity: impl Into<String>,
        token: SecretValue,
        options: RoomOptions,
    ) -> Result<Self, LiveKitError> {
        let room_name = room_name.into();
        LiveKitError::validate_room_name(&room_name)?;
        let identity = identity.into();
        if identity.is_empty() {
            return Err(LiveKitError::InvalidArgument(
                "identity is empty".to_string(),
            ));
        }
        if token.is_empty() {
            return Err(LiveKitError::Authentication(
                "join token is empty".to_string(),
            ));
        }
        let heartbeat_policy = HeartbeatPolicy::default();
        let reconnect_policy = ReconnectPolicy::from_options(&options);
        Ok(Self {
            room_name,
            identity,
            token,
            options,
            heartbeat_policy,
            reconnect_policy,
            state: RoomState::Disconnected,
            session_id: None,
            room_sid: None,
            local_sid: None,
            participants: BTreeMap::new(),
            local_tracks: Vec::new(),
            remote_tracks: BTreeMap::new(),
            data_assembler: DataMessageAssembler::new(),
            data_meta: HashMap::new(),
            heartbeat_missed: 0,
            awaiting_heartbeat_ack: false,
            reconnect_attempts: 0,
            disconnected_at: None,
            frames_sent: 0,
            frames_received: 0,
            heartbeats_sent: 0,
        })
    }

    // ---------- 观测面 ----------

    /// 当前状态.
    pub fn state(&self) -> RoomState {
        self.state
    }

    /// 房间名.
    pub fn room_name(&self) -> &str {
        &self.room_name
    }

    /// 服务端会话 ID.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// 房间 SID.
    pub fn room_sid(&self) -> Option<&str> {
        self.room_sid.as_deref()
    }

    /// 本端参与者 SID.
    pub fn local_sid(&self) -> Option<&str> {
        self.local_sid.as_deref()
    }

    /// 当前参与者表.
    pub fn participants(&self) -> impl Iterator<Item = &Participant> {
        self.participants.values()
    }

    /// 本地轨道记录.
    pub fn local_tracks(&self) -> &[LocalTrackRecord] {
        &self.local_tracks
    }

    /// 远端订阅表.
    pub fn remote_tracks(&self) -> impl Iterator<Item = &TrackSubscription> {
        self.remote_tracks.values()
    }

    /// 当前重连尝试次数.
    pub fn reconnect_attempts(&self) -> u32 {
        self.reconnect_attempts
    }

    /// 当前连续心跳丢失数.
    pub fn heartbeat_missed(&self) -> u32 {
        self.heartbeat_missed
    }

    /// 下次重连退避时长 (按当前尝试次数).
    pub fn next_reconnect_delay(&self) -> Duration {
        self.reconnect_policy
            .delay_for_attempt(self.reconnect_attempts.saturating_add(1))
    }

    /// 心跳策略 (含握手协商结果).
    pub fn heartbeat_policy(&self) -> HeartbeatPolicy {
        self.heartbeat_policy
    }

    /// 统计快照.
    pub fn stats(&self) -> SessionStats {
        SessionStats {
            state: self.state,
            frames_sent: self.frames_sent,
            frames_received: self.frames_received,
            heartbeats_sent: self.heartbeats_sent,
            heartbeat_missed: self.heartbeat_missed,
            reconnect_attempts: self.reconnect_attempts,
            local_tracks: self.local_tracks.len(),
            remote_subscriptions: self.remote_tracks.len(),
        }
    }

    /// 房间快照 (给 `LiveKitClientImpl::room()` 观测面).
    pub fn to_room(&self) -> Result<Room, LiveKitError> {
        let mut room = Room::new(&self.room_name, self.options.clone())?;
        if let Some(sid) = &self.room_sid {
            room.set_sid(sid.clone());
        }
        room.set_state(self.state);
        Ok(room)
    }

    // ---------- 迁移面 ----------

    /// 发起连接 (握手第一步): `Disconnected` / `DisconnectedAlt` → `Connecting`.
    pub fn begin_connect(&mut self) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(
            self.state,
            RoomState::Disconnected | RoomState::DisconnectedAlt
        ) {
            return Err(LiveKitError::State(format!(
                "connect not allowed from state {}",
                self.state
            )));
        }
        self.reconnect_attempts = 0;
        self.disconnected_at = None;
        let previous = self.state;
        self.state = RoomState::Connecting;
        self.frames_sent += 2;
        Ok(vec![
            SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                previous,
                current: RoomState::Connecting,
            }),
            SessionEffect::Send(self.hello_frame()),
            SessionEffect::Send(self.join_frame()),
        ])
    }

    /// 传输层失联 (读写失败 / 对端关闭).
    pub fn on_transport_closed(&mut self) -> Result<Vec<SessionEffect>, LiveKitError> {
        match self.state {
            RoomState::Connecting => {
                let previous = self.state;
                self.state = RoomState::Disconnected;
                Ok(vec![SessionEffect::Emit(
                    RoomEvent::ConnectionStateChanged {
                        previous,
                        current: RoomState::Disconnected,
                    },
                )])
            }
            RoomState::Connected | RoomState::Reconnecting => {
                let previous = self.state;
                self.state = RoomState::Reconnecting;
                self.disconnected_at = Some(SystemTime::now());
                self.heartbeat_missed = 0;
                self.awaiting_heartbeat_ack = false;
                Ok(vec![SessionEffect::Emit(
                    RoomEvent::ConnectionStateChanged {
                        previous,
                        current: RoomState::Reconnecting,
                    },
                )])
            }
            other => Err(LiveKitError::State(format!(
                "transport closed while state {other}"
            ))),
        }
    }

    /// 心跳节拍 (由驱动层定时器驱动): 发心跳或判定失联.
    pub fn on_heartbeat_tick(&mut self) -> Result<Vec<SessionEffect>, LiveKitError> {
        if self.state != RoomState::Connected {
            // 心跳只在已连接态工作 (重连期间暂停)
            return Ok(Vec::new());
        }
        if self.awaiting_heartbeat_ack {
            self.heartbeat_missed += 1;
        }
        if self.heartbeat_missed >= self.heartbeat_policy.max_missed {
            // 心跳超限 → 走失联路径
            return self.on_transport_closed();
        }
        self.awaiting_heartbeat_ack = true;
        self.heartbeats_sent += 1;
        let frame = SignalFrame::Heartbeat {
            timestamp_ms: unix_ms(),
        };
        self.frames_sent += 1;
        Ok(vec![SessionEffect::Send(frame)])
    }

    /// 重连定时器到点: 重新握手或判定次数耗尽.
    pub fn on_reconnect_timer(&mut self) -> Result<Vec<SessionEffect>, LiveKitError> {
        if self.state != RoomState::Reconnecting {
            return Err(LiveKitError::State(format!(
                "reconnect timer fired while state {}",
                self.state
            )));
        }
        self.reconnect_attempts += 1;
        if self.reconnect_attempts > self.reconnect_policy.max_attempts {
            let previous = self.state;
            self.state = RoomState::Disconnected;
            return Ok(vec![SessionEffect::Emit(
                RoomEvent::ConnectionStateChanged {
                    previous,
                    current: RoomState::Disconnected,
                },
            )]);
        }
        self.frames_sent += 2;
        Ok(vec![
            SessionEffect::Send(self.hello_frame()),
            SessionEffect::Send(self.join_frame()),
        ])
    }

    /// 主动离开: `Connecting` / `Connected` / `Reconnecting` → `DisconnectedAlt`.
    pub fn disconnect(&mut self) -> Result<Vec<SessionEffect>, LiveKitError> {
        match self.state {
            RoomState::Disconnected | RoomState::DisconnectedAlt => Err(LiveKitError::State(
                "disconnect called while already disconnected".to_string(),
            )),
            state => {
                let previous = state;
                self.state = RoomState::DisconnectedAlt;
                self.clear_session();
                let mut effects = Vec::new();
                if previous == RoomState::Connected || previous == RoomState::Reconnecting {
                    self.frames_sent += 1;
                    effects.push(SessionEffect::Send(SignalFrame::Leave));
                }
                effects.push(SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                    previous,
                    current: RoomState::DisconnectedAlt,
                }));
                Ok(effects)
            }
        }
    }

    /// 发布本地轨道 (仅 `Connected`).
    pub fn publish_track(&mut self, track: &Track) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !self.state.can_publish() {
            return Err(LiveKitError::RoomDisconnected(format!(
                "publish_track requires connected state, got {}",
                self.state
            )));
        }
        if self
            .local_tracks
            .iter()
            .any(|r| r.source == track.source() && r.phase != LocalTrackPhase::Unpublishing)
        {
            return Err(LiveKitError::State(format!(
                "track source {} already publishing",
                track.source()
            )));
        }
        self.local_tracks.push(LocalTrackRecord {
            track_sid: None,
            kind: track.kind(),
            source: track.source(),
            name: track.name().map(|s| s.to_string()),
            phase: LocalTrackPhase::Publishing,
        });
        self.frames_sent += 1;
        Ok(vec![SessionEffect::Send(SignalFrame::TrackPublish {
            kind: track.kind(),
            source: track.source(),
            name: track.name().map(|s| s.to_string()),
        })])
    }

    /// 撤下本地轨道 (仅 `Connected`; 等 `TrackUnpublished` 回执后销账).
    pub fn unpublish_track(&mut self, track_sid: &str) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !self.state.can_publish() {
            return Err(LiveKitError::RoomDisconnected(format!(
                "unpublish_track requires connected state, got {}",
                self.state
            )));
        }
        let record = self
            .local_tracks
            .iter_mut()
            .find(|r| r.track_sid.as_deref() == Some(track_sid))
            .ok_or_else(|| LiveKitError::TrackNotFound(track_sid.to_string()))?;
        if record.phase == LocalTrackPhase::Unpublishing {
            return Err(LiveKitError::State(format!(
                "track {track_sid} already unpublishing"
            )));
        }
        record.phase = LocalTrackPhase::Unpublishing;
        self.frames_sent += 1;
        Ok(vec![SessionEffect::Send(SignalFrame::TrackUnpublish {
            track_sid: track_sid.to_string(),
        })])
    }

    /// 订阅远端轨道 (`Connected` / `Reconnecting`).
    pub fn subscribe(&mut self, track_sid: &str) -> Result<Vec<SessionEffect>, LiveKitError> {
        if track_sid.is_empty() {
            return Err(LiveKitError::TrackNotFound("empty track_sid".to_string()));
        }
        if !self.state.can_subscribe() {
            return Err(LiveKitError::RoomDisconnected(format!(
                "subscribe requires connected state, got {}",
                self.state
            )));
        }
        if self.remote_tracks.contains_key(track_sid) {
            return Err(LiveKitError::State(format!(
                "track {track_sid} already subscribed or subscribing"
            )));
        }
        self.remote_tracks.insert(
            track_sid.to_string(),
            TrackSubscription {
                track_sid: track_sid.to_string(),
                participant_sid: None,
                source: None,
                subscribed: false,
            },
        );
        self.frames_sent += 1;
        Ok(vec![SessionEffect::Send(SignalFrame::TrackSubscribe {
            track_sid: track_sid.to_string(),
        })])
    }

    /// 取消订阅远端轨道.
    pub fn unsubscribe(&mut self, track_sid: &str) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !self.state.can_subscribe() {
            return Err(LiveKitError::RoomDisconnected(format!(
                "unsubscribe requires connected state, got {}",
                self.state
            )));
        }
        if !self.remote_tracks.contains_key(track_sid) {
            return Err(LiveKitError::TrackNotFound(track_sid.to_string()));
        }
        self.frames_sent += 1;
        Ok(vec![SessionEffect::Send(SignalFrame::TrackUnsubscribe {
            track_sid: track_sid.to_string(),
        })])
    }

    /// 启用 / 禁用摄像头 (映射到 Camera 轨道的发布 / 撤下, 幂等).
    pub fn set_camera_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        self.set_source_enabled(TrackSource::Camera, TrackKind::Video, enabled)
    }

    /// 启用 / 禁用麦克风 (映射到 Microphone 轨道的发布 / 撤下, 幂等).
    pub fn set_microphone_enabled(
        &mut self,
        enabled: bool,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        self.set_source_enabled(TrackSource::Microphone, TrackKind::Audio, enabled)
    }

    fn set_source_enabled(
        &mut self,
        source: TrackSource,
        kind: TrackKind,
        enabled: bool,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !self.state.can_publish() {
            return Err(LiveKitError::RoomDisconnected(format!(
                "set device enabled requires connected state, got {}",
                self.state
            )));
        }
        let existing = self
            .local_tracks
            .iter()
            .find(|r| r.source == source && r.phase != LocalTrackPhase::Unpublishing);
        let existing_sid = existing.and_then(|r| r.track_sid.clone());
        let publishing_in_flight = existing
            .map(|r| r.phase == LocalTrackPhase::Publishing)
            .unwrap_or(false);
        match (enabled, existing_sid, publishing_in_flight) {
            // 幂等: 已启用 / 已禁用
            (true, Some(_), _) => Ok(Vec::new()),
            (false, None, false) => Ok(Vec::new()),
            // 发布回执未到, 还没有 SID 可撤
            (false, None, true) => Err(LiveKitError::State(format!(
                "track source {source} publish still in flight"
            ))),
            (true, None, _) => {
                let track = Track::new(kind, source);
                self.publish_track(&track)
            }
            (false, Some(sid), _) => self.unpublish_track(&sid),
        }
    }

    /// 发送一条数据消息 (自动分块; 受 [`MAX_DATA_CHUNKS`] 上限约束).
    pub fn send_data_message(
        &mut self,
        message_id: &str,
        payload: &[u8],
        reliable: bool,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !self.state.can_publish() {
            return Err(LiveKitError::RoomDisconnected(format!(
                "send_data_message requires connected state, got {}",
                self.state
            )));
        }
        let frames = chunk_data_message(message_id, payload, reliable)?;
        self.frames_sent += frames.len() as u64;
        Ok(frames.into_iter().map(SessionEffect::Send).collect())
    }

    /// 接收一帧信令: 全部生命周期迁移入口.
    pub fn on_frame(&mut self, frame: SignalFrame) -> Result<Vec<SessionEffect>, LiveKitError> {
        self.frames_received += 1;
        match frame {
            SignalFrame::Welcome {
                protocol_version,
                session_id,
                heartbeat_interval_ms,
            } => self.on_welcome(protocol_version, session_id, heartbeat_interval_ms),
            SignalFrame::JoinAccepted {
                room_sid,
                participant_sid,
                participants,
            } => self.on_join_accepted(room_sid, participant_sid, participants),
            SignalFrame::JoinRejected { reason, message } => {
                self.on_join_rejected(&reason, &message)
            }
            SignalFrame::Offer { .. }
            | SignalFrame::Answer { .. }
            | SignalFrame::IceCandidate { .. } => self.on_negotiation(frame),
            SignalFrame::TrackPublished {
                track_sid,
                kind,
                source,
            } => self.on_track_published(track_sid, kind, source),
            SignalFrame::TrackUnpublished { track_sid } => self.on_track_unpublished(track_sid),
            SignalFrame::TrackSubscribed {
                track_sid,
                participant_sid,
                source,
            } => self.on_track_subscribed(track_sid, participant_sid, source),
            SignalFrame::TrackUnsubscribed {
                track_sid,
                participant_sid,
            } => self.on_track_unsubscribed(track_sid, participant_sid),
            SignalFrame::ParticipantJoined { participant } => {
                self.on_participant_joined(participant)
            }
            SignalFrame::ParticipantLeft { participant_sid } => {
                self.on_participant_left(participant_sid)
            }
            SignalFrame::ActiveSpeakers { speakers } => self.on_active_speakers(speakers),
            SignalFrame::DataReceived {
                message_id,
                chunk_index,
                chunk_count,
                participant_sid,
                payload,
                reliable,
            } => self.on_data_received(
                message_id,
                chunk_index,
                chunk_count,
                participant_sid,
                payload,
                reliable,
            ),
            SignalFrame::HeartbeatAck { .. } => self.on_heartbeat_ack(),
            SignalFrame::LeaveAck => self.on_leave_ack(),
            SignalFrame::Error {
                code,
                message,
                fatal,
                retry_after_ms,
            } => self.on_error_frame(&code, &message, fatal, retry_after_ms),
            // 客户端方向帧不允许出现在接收队列 (协议违例)
            other => Err(LiveKitError::Protocol(format!(
                "unexpected inbound frame `{}`",
                other.type_str()
            ))),
        }
    }

    // ---------- 帧处理私有面 ----------

    fn hello_frame(&self) -> SignalFrame {
        SignalFrame::Hello {
            protocol_version: PROTOCOL_VERSION,
            client: crate::livekit::signal::CLIENT_NAME.to_string(),
        }
    }

    fn join_frame(&self) -> SignalFrame {
        SignalFrame::Join {
            room: self.room_name.clone(),
            identity: self.identity.clone(),
            token: self.token.clone(),
        }
    }

    fn transition(&mut self, to: RoomState) -> SessionEffect {
        let previous = self.state;
        self.state = to;
        SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
            previous,
            current: to,
        })
    }

    fn clear_session(&mut self) {
        self.session_id = None;
        self.room_sid = None;
        self.local_sid = None;
        self.participants.clear();
        self.local_tracks.clear();
        self.remote_tracks.clear();
        self.data_meta.clear();
        self.data_assembler = DataMessageAssembler::new();
        self.heartbeat_missed = 0;
        self.awaiting_heartbeat_ack = false;
    }

    fn on_welcome(
        &mut self,
        protocol_version: u32,
        session_id: String,
        heartbeat_interval_ms: u64,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connecting | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "welcome in state {}",
                self.state
            )));
        }
        if protocol_version != PROTOCOL_VERSION {
            self.state = RoomState::Disconnected;
            return Err(LiveKitError::Protocol(format!(
                "protocol version mismatch: server {protocol_version}, client {PROTOCOL_VERSION}"
            )));
        }
        self.session_id = Some(session_id);
        // 服务端建议心跳间隔钳到 1s..=60s, 防恶意 / 误配
        let clamped = heartbeat_interval_ms.clamp(1_000, 60_000);
        self.heartbeat_policy.interval = Duration::from_millis(clamped);
        Ok(Vec::new())
    }

    fn on_join_accepted(
        &mut self,
        room_sid: String,
        participant_sid: ParticipantSid,
        participants: Vec<ParticipantInfo>,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connecting | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "join_accepted in state {}",
                self.state
            )));
        }
        let reconnected = self.state == RoomState::Reconnecting;
        self.room_sid = Some(room_sid);
        self.local_sid = Some(participant_sid);
        if reconnected {
            // 重连后服务端给的是权威快照: 清掉失联前的参与者 / 订阅残影
            self.participants.clear();
            self.remote_tracks.clear();
            self.data_meta.clear();
            self.data_assembler = DataMessageAssembler::new();
        }
        let mut effects = Vec::new();
        for info in participants {
            let sid = info.sid.clone();
            let participant = info.into_participant()?;
            if self
                .participants
                .insert(sid.clone(), participant.clone())
                .is_some()
            {
                return Err(LiveKitError::Protocol(format!(
                    "duplicate participant {sid} in join_accepted"
                )));
            }
            effects.push(SessionEffect::Emit(RoomEvent::ParticipantConnected {
                participant,
            }));
        }
        if reconnected {
            self.reconnect_attempts = 0;
            effects.push(self.transition(RoomState::Connected));
            effects.push(SessionEffect::Emit(RoomEvent::Reconnected {
                disconnected_at: self.disconnected_at.take(),
            }));
        } else {
            effects.push(self.transition(RoomState::Connected));
        }
        Ok(effects)
    }

    fn on_join_rejected(
        &mut self,
        reason: &str,
        message: &str,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connecting | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "join_rejected in state {}",
                self.state
            )));
        }
        let previous = self.state;
        self.state = RoomState::Disconnected;
        self.clear_session();
        let _ = previous;
        Err(classify_server_rejection(reason, message))
    }

    fn on_negotiation(&mut self, frame: SignalFrame) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connected | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "negotiation frame in state {}",
                self.state
            )));
        }
        Ok(vec![SessionEffect::Negotiation(frame)])
    }

    fn on_track_published(
        &mut self,
        track_sid: TrackSid,
        kind: TrackKind,
        source: TrackSource,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if self.state != RoomState::Connected {
            return Err(LiveKitError::Protocol(format!(
                "track_published in state {}",
                self.state
            )));
        }
        let record = self
            .local_tracks
            .iter_mut()
            .find(|r| r.phase == LocalTrackPhase::Publishing && r.source == source)
            .ok_or_else(|| {
                LiveKitError::Protocol(format!(
                    "track_published for source {source} without pending publish"
                ))
            })?;
        if record.kind != kind {
            return Err(LiveKitError::Protocol(format!(
                "track_published kind mismatch: expected {:?}, got {kind:?}",
                record.kind
            )));
        }
        record.track_sid = Some(track_sid.clone());
        record.phase = LocalTrackPhase::Published;
        Ok(vec![SessionEffect::LocalTrackPublished { track_sid }])
    }

    fn on_track_unpublished(
        &mut self,
        track_sid: TrackSid,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if self.state != RoomState::Connected {
            return Err(LiveKitError::Protocol(format!(
                "track_unpublished in state {}",
                self.state
            )));
        }
        let before = self.local_tracks.len();
        self.local_tracks
            .retain(|r| r.track_sid.as_deref() != Some(track_sid.as_str()));
        if self.local_tracks.len() == before {
            return Err(LiveKitError::Protocol(format!(
                "track_unpublished for unknown track {track_sid}"
            )));
        }
        Ok(vec![SessionEffect::LocalTrackUnpublished { track_sid }])
    }

    fn on_track_subscribed(
        &mut self,
        track_sid: TrackSid,
        participant_sid: ParticipantSid,
        source: TrackSource,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connected | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "track_subscribed in state {}",
                self.state
            )));
        }
        let record = self.remote_tracks.get_mut(&track_sid).ok_or_else(|| {
            LiveKitError::Protocol(format!(
                "track_subscribed for unsubscribed track {track_sid}"
            ))
        })?;
        if record.subscribed {
            return Err(LiveKitError::Protocol(format!(
                "track {track_sid} subscribed twice"
            )));
        }
        record.subscribed = true;
        record.participant_sid = Some(participant_sid.clone());
        record.source = Some(source);
        Ok(vec![SessionEffect::Emit(RoomEvent::TrackSubscribed {
            track_sid,
            participant_sid,
            source,
        })])
    }

    fn on_track_unsubscribed(
        &mut self,
        track_sid: TrackSid,
        participant_sid: ParticipantSid,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connected | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "track_unsubscribed in state {}",
                self.state
            )));
        }
        if self.remote_tracks.remove(&track_sid).is_none() {
            return Err(LiveKitError::Protocol(format!(
                "track_unsubscribed for unknown track {track_sid}"
            )));
        }
        Ok(vec![SessionEffect::Emit(RoomEvent::TrackUnsubscribed {
            track_sid,
            participant_sid,
        })])
    }

    fn on_participant_joined(
        &mut self,
        info: ParticipantInfo,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connected | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "participant_joined in state {}",
                self.state
            )));
        }
        let sid = info.sid.clone();
        let participant = info.into_participant()?;
        if self
            .participants
            .insert(sid.clone(), participant.clone())
            .is_some()
        {
            return Err(LiveKitError::Protocol(format!(
                "duplicate participant {sid}"
            )));
        }
        Ok(vec![SessionEffect::Emit(RoomEvent::ParticipantConnected {
            participant,
        })])
    }

    fn on_participant_left(
        &mut self,
        participant_sid: ParticipantSid,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connected | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "participant_left in state {}",
                self.state
            )));
        }
        if self.participants.remove(&participant_sid).is_none() {
            return Err(LiveKitError::Protocol(format!(
                "participant_left for unknown participant {participant_sid}"
            )));
        }
        Ok(vec![SessionEffect::Emit(
            RoomEvent::ParticipantDisconnected { participant_sid },
        )])
    }

    fn on_active_speakers(
        &mut self,
        speakers: Vec<ParticipantSid>,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if self.state != RoomState::Connected {
            return Err(LiveKitError::Protocol(format!(
                "active_speakers in state {}",
                self.state
            )));
        }
        Ok(vec![SessionEffect::Emit(
            RoomEvent::ActiveSpeakersChanged { speakers },
        )])
    }

    fn on_data_received(
        &mut self,
        message_id: String,
        chunk_index: u32,
        chunk_count: u32,
        participant_sid: ParticipantSid,
        payload: Vec<u8>,
        reliable: bool,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connected | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "data_received in state {}",
                self.state
            )));
        }
        self.data_meta
            .entry(message_id.clone())
            .or_insert((participant_sid.clone(), reliable));
        let assembled =
            self.data_assembler
                .push_chunk(&message_id, chunk_index, chunk_count, payload)?;
        match assembled {
            Some(payload) => {
                let (_, reliable_flag) = self
                    .data_meta
                    .remove(&message_id)
                    .expect("meta inserted before assembly");
                Ok(vec![SessionEffect::Emit(RoomEvent::DataReceived {
                    participant_sid,
                    payload,
                    reliable: reliable_flag,
                })])
            }
            None => Ok(Vec::new()),
        }
    }

    fn on_heartbeat_ack(&mut self) -> Result<Vec<SessionEffect>, LiveKitError> {
        if !matches!(self.state, RoomState::Connected | RoomState::Reconnecting) {
            return Err(LiveKitError::Protocol(format!(
                "heartbeat_ack in state {}",
                self.state
            )));
        }
        self.awaiting_heartbeat_ack = false;
        self.heartbeat_missed = 0;
        Ok(Vec::new())
    }

    fn on_leave_ack(&mut self) -> Result<Vec<SessionEffect>, LiveKitError> {
        match self.state {
            RoomState::DisconnectedAlt | RoomState::Disconnected => Ok(Vec::new()),
            _ => {
                self.clear_session();
                Ok(vec![self.transition(RoomState::DisconnectedAlt)])
            }
        }
    }

    fn on_error_frame(
        &mut self,
        code: &str,
        message: &str,
        fatal: bool,
        retry_after_ms: Option<u64>,
    ) -> Result<Vec<SessionEffect>, LiveKitError> {
        if fatal {
            self.state = RoomState::Disconnected;
            self.clear_session();
        }
        Err(classify_server_error(code, message, retry_after_ms))
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 服务端错误帧 → 闭合词表分类.
pub fn classify_server_error(
    code: &str,
    message: &str,
    retry_after_ms: Option<u64>,
) -> LiveKitError {
    match code {
        "rate_limited" => LiveKitError::RateLimited {
            retry_after_ms: retry_after_ms.unwrap_or(1_000),
        },
        "unauthorized" | "forbidden" | "auth" => LiveKitError::Authentication(message.to_string()),
        "protocol" => LiveKitError::Protocol(message.to_string()),
        "unavailable" | "network" => LiveKitError::Network(message.to_string()),
        "timeout" => LiveKitError::Timeout {
            operation: "server_operation",
        },
        "backpressure" => LiveKitError::Backpressure(message.to_string()),
        _ => LiveKitError::Internal(format!("server error code `{code}`: {message}")),
    }
}

/// 加入被拒原因 → 闭合词表分类.
pub fn classify_server_rejection(reason: &str, message: &str) -> LiveKitError {
    match reason {
        "unauthorized" | "invalid_token" | "token_expired" => {
            LiveKitError::Authentication(message.to_string())
        }
        "rate_limited" => LiveKitError::RateLimited {
            retry_after_ms: 1_000,
        },
        "room_not_found" | "room_full" | "rejected" => LiveKitError::State(message.to_string()),
        "protocol" => LiveKitError::Protocol(message.to_string()),
        _ => LiveKitError::Internal(format!("join rejected: {reason}: {message}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::livekit::signal::MAX_DATA_CHUNK_BYTES;
    use crate::livekit::track::TrackSource;

    fn session() -> SignalingSession {
        SignalingSession::new(
            "room-1",
            "user-1",
            SecretValue::new("join-token-value-1"),
            RoomOptions::default(),
        )
        .expect("valid session")
    }

    fn welcome() -> SignalFrame {
        SignalFrame::Welcome {
            protocol_version: PROTOCOL_VERSION,
            session_id: "sess-1".to_string(),
            heartbeat_interval_ms: 5_000,
        }
    }

    fn join_accepted() -> SignalFrame {
        SignalFrame::JoinAccepted {
            room_sid: "RM_1".to_string(),
            participant_sid: "PA_SELF".to_string(),
            participants: vec![ParticipantInfo {
                sid: "PA_2".to_string(),
                identity: "user-2".to_string(),
                name: None,
                metadata: None,
            }],
        }
    }

    fn connect_to_active(s: &mut SignalingSession) {
        s.begin_connect().expect("begin");
        s.on_frame(welcome()).expect("welcome");
        s.on_frame(join_accepted()).expect("join accepted");
        assert_eq!(s.state(), RoomState::Connected);
    }

    // ---------- 握手 ----------

    #[test]
    fn connect_emits_hello_then_join_and_enters_connecting() {
        let mut s = session();
        assert_eq!(s.state(), RoomState::Disconnected);
        let effects = s.begin_connect().expect("begin");
        assert_eq!(s.state(), RoomState::Connecting);
        assert_eq!(effects.len(), 3);
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                previous: RoomState::Disconnected,
                current: RoomState::Connecting,
            })
        ));
        assert!(matches!(
            &effects[1],
            SessionEffect::Send(SignalFrame::Hello { .. })
        ));
        match &effects[2] {
            SessionEffect::Send(SignalFrame::Join {
                room,
                identity,
                token,
            }) => {
                assert_eq!(room, "room-1");
                assert_eq!(identity, "user-1");
                // 令牌不进 Debug 日志
                assert!(!format!("{token:?}").contains("join-token-value-1"));
            }
            other => panic!("expected join frame, got {other:?}"),
        }
    }

    #[test]
    fn connect_rejected_from_mid_states() {
        let mut s = session();
        s.begin_connect().expect("begin");
        assert!(
            matches!(s.begin_connect(), Err(LiveKitError::State(_))),
            "connecting 中不允许二次 connect"
        );
    }

    #[test]
    fn welcome_validates_protocol_version() {
        let mut s = session();
        s.begin_connect().expect("begin");
        let bad = SignalFrame::Welcome {
            protocol_version: PROTOCOL_VERSION + 1,
            session_id: "sess-1".to_string(),
            heartbeat_interval_ms: 5_000,
        };
        assert!(matches!(s.on_frame(bad), Err(LiveKitError::Protocol(_))));
    }

    #[test]
    fn welcome_negotiates_heartbeat_interval_clamped() {
        let mut s = session();
        s.begin_connect().expect("begin");
        s.on_frame(SignalFrame::Welcome {
            protocol_version: PROTOCOL_VERSION,
            session_id: "sess-1".to_string(),
            heartbeat_interval_ms: 999_999,
        })
        .expect("welcome");
        assert_eq!(s.heartbeat_policy().interval, Duration::from_secs(60));
    }

    #[test]
    fn join_accepted_transitions_to_connected_and_reports_members() {
        let mut s = session();
        s.begin_connect().expect("begin");
        s.on_frame(welcome()).expect("welcome");
        let effects = s.on_frame(join_accepted()).expect("accepted");
        assert_eq!(s.state(), RoomState::Connected);
        assert_eq!(s.room_sid(), Some("RM_1"));
        assert_eq!(s.local_sid(), Some("PA_SELF"));
        assert_eq!(s.participants().count(), 1);
        // 效果: 已有参与者事件 + 状态迁移事件
        assert!(effects.iter().any(|e| matches!(
            e,
            SessionEffect::Emit(RoomEvent::ParticipantConnected { .. })
        )));
        assert!(effects.iter().any(|e| matches!(
            e,
            SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                previous: RoomState::Connecting,
                current: RoomState::Connected,
            })
        )));
    }

    #[test]
    fn join_rejected_classifies_and_returns_to_disconnected() {
        let mut s = session();
        s.begin_connect().expect("begin");
        s.on_frame(welcome()).expect("welcome");
        let err = s
            .on_frame(SignalFrame::JoinRejected {
                reason: "unauthorized".to_string(),
                message: "token rejected".to_string(),
            })
            .expect_err("must reject");
        assert!(matches!(err, LiveKitError::Authentication(_)));
        assert_eq!(s.state(), RoomState::Disconnected);

        let mut s2 = session();
        s2.begin_connect().expect("begin");
        let err2 = s2
            .on_frame(SignalFrame::JoinRejected {
                reason: "rate_limited".to_string(),
                message: "too many".to_string(),
            })
            .expect_err("must reject");
        assert!(matches!(err2, LiveKitError::RateLimited { .. }));
    }

    // ---------- 全迁移: 断线重连 ----------

    #[test]
    fn transport_closed_paths_cover_all_states() {
        // Disconnected: 非法
        let mut s = session();
        assert!(matches!(
            s.on_transport_closed(),
            Err(LiveKitError::State(_))
        ));

        // Connecting → Disconnected
        let mut s = session();
        s.begin_connect().expect("begin");
        let effects = s.on_transport_closed().expect("close");
        assert_eq!(s.state(), RoomState::Disconnected);
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                previous: RoomState::Connecting,
                current: RoomState::Disconnected,
            })
        ));

        // Connected → Reconnecting (记断开时刻)
        let mut s = session();
        connect_to_active(&mut s);
        let effects = s.on_transport_closed().expect("close");
        assert_eq!(s.state(), RoomState::Reconnecting);
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                previous: RoomState::Connected,
                current: RoomState::Reconnecting,
            })
        ));

        // Reconnecting → Reconnecting (幂等, 状态保持)
        let effects = s.on_transport_closed().expect("close again");
        assert_eq!(s.state(), RoomState::Reconnecting);
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                previous: RoomState::Reconnecting,
                current: RoomState::Reconnecting,
            })
        ));
    }

    #[test]
    fn reconnect_timer_resends_handshake_with_backoff() {
        let mut s = session();
        connect_to_active(&mut s);
        s.on_transport_closed().expect("close");
        let delay_first = s.next_reconnect_delay();
        let effects = s.on_reconnect_timer().expect("timer");
        assert_eq!(s.reconnect_attempts(), 1);
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::Hello { .. })
        ));
        assert!(matches!(
            &effects[1],
            SessionEffect::Send(SignalFrame::Join { .. })
        ));
        // 指数退避: 第二次 >= 第一次
        let delay_second = s.next_reconnect_delay();
        assert!(delay_second >= delay_first);

        // 重连成功: Reconnecting → Connected + Reconnected 事件
        let effects = s.on_frame(welcome()).expect("welcome");
        assert!(effects.is_empty());
        let effects = s.on_frame(join_accepted()).expect("accepted");
        assert_eq!(s.state(), RoomState::Connected);
        assert!(effects
            .iter()
            .any(|e| matches!(e, SessionEffect::Emit(RoomEvent::Reconnected { .. }))));
        assert_eq!(s.reconnect_attempts(), 0, "重连成功清零尝试计数");
    }

    #[test]
    fn reconnect_exhaustion_goes_to_disconnected() {
        let mut options = RoomOptions::default();
        options.max_reconnect_attempts = 2;
        let mut s = SignalingSession::new(
            "room-1",
            "user-1",
            SecretValue::new("join-token-value-1"),
            options,
        )
        .expect("valid session");
        connect_to_active(&mut s);
        s.on_transport_closed().expect("close");

        // 第 1、2 次: 重新握手
        s.on_reconnect_timer().expect("attempt 1");
        s.on_reconnect_timer().expect("attempt 2");
        // 第 3 次: 超限 → Disconnected
        let effects = s.on_reconnect_timer().expect("attempt 3");
        assert_eq!(s.state(), RoomState::Disconnected);
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ConnectionStateChanged {
                previous: RoomState::Reconnecting,
                current: RoomState::Disconnected,
            })
        ));
    }

    #[test]
    fn reconnect_timer_rejected_outside_reconnecting() {
        let mut s = session();
        assert!(matches!(
            s.on_reconnect_timer(),
            Err(LiveKitError::State(_))
        ));
    }

    #[test]
    fn disconnect_covers_all_states() {
        // Disconnected: 非法
        let mut s = session();
        assert!(matches!(s.disconnect(), Err(LiveKitError::State(_))));

        // Connecting → DisconnectedAlt (不发 Leave, 握手未建立)
        let mut s = session();
        s.begin_connect().expect("begin");
        let effects = s.disconnect().expect("disconnect");
        assert_eq!(s.state(), RoomState::DisconnectedAlt);
        assert!(effects.iter().all(|e| !matches!(e, SessionEffect::Send(_))));

        // Connected → DisconnectedAlt + Leave
        let mut s = session();
        connect_to_active(&mut s);
        let effects = s.disconnect().expect("disconnect");
        assert_eq!(s.state(), RoomState::DisconnectedAlt);
        assert!(matches!(
            effects[0],
            SessionEffect::Send(SignalFrame::Leave)
        ));

        // Reconnecting → DisconnectedAlt + Leave
        let mut s = session();
        connect_to_active(&mut s);
        s.on_transport_closed().expect("close");
        let effects = s.disconnect().expect("disconnect");
        assert_eq!(s.state(), RoomState::DisconnectedAlt);
        assert!(matches!(
            effects[0],
            SessionEffect::Send(SignalFrame::Leave)
        ));

        // DisconnectedAlt: 二次 disconnect 非法
        assert!(matches!(s.disconnect(), Err(LiveKitError::State(_))));
    }

    #[test]
    fn leave_ack_is_idempotent_after_graceful_disconnect() {
        let mut s = session();
        connect_to_active(&mut s);
        s.disconnect().expect("disconnect");
        let effects = s.on_frame(SignalFrame::LeaveAck).expect("ack");
        assert!(effects.is_empty(), "优雅关闭后 LeaveAck 幂等");
        assert_eq!(s.state(), RoomState::DisconnectedAlt);
    }

    // ---------- 心跳 ----------

    #[test]
    fn heartbeat_tick_sends_then_declares_loss_after_max_missed() {
        let mut s = session();
        connect_to_active(&mut s);

        // 第一拍: 发心跳
        let effects = s.on_heartbeat_tick().expect("tick");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::Heartbeat { .. })
        ));
        assert_eq!(s.heartbeat_missed(), 0);

        // 第二拍 (无 Ack): 记 1 次丢失, 仍发心跳
        let effects = s.on_heartbeat_tick().expect("tick");
        assert_eq!(s.heartbeat_missed(), 1);
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::Heartbeat { .. })
        ));

        // Ack 清零
        s.on_frame(SignalFrame::HeartbeatAck { timestamp_ms: 1 })
            .expect("ack");
        assert_eq!(s.heartbeat_missed(), 0);

        // 连续丢到上限 → Reconnecting (需要 max_missed+1 拍: 首拍只发不计)
        for _ in 0..=MAX_HEARTBEAT_MISSED {
            s.on_heartbeat_tick().expect("tick");
        }
        assert_eq!(s.state(), RoomState::Reconnecting, "心跳超限必须进重连");
    }

    #[test]
    fn heartbeat_tick_is_noop_outside_connected() {
        let mut s = session();
        let effects = s.on_heartbeat_tick().expect("tick");
        assert!(effects.is_empty(), "未连接不发心跳");
    }

    #[test]
    fn heartbeat_ack_outside_session_is_protocol_error() {
        let mut s = session();
        assert!(matches!(
            s.on_frame(SignalFrame::HeartbeatAck { timestamp_ms: 1 }),
            Err(LiveKitError::Protocol(_))
        ));
    }

    // ---------- 媒体生命周期 ----------

    #[test]
    fn publish_track_lifecycle_via_server_ack() {
        let mut s = session();
        connect_to_active(&mut s);
        let track = Track::new(TrackKind::Video, TrackSource::Camera);
        let effects = s.publish_track(&track).expect("publish");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackPublish {
                kind: TrackKind::Video,
                source: TrackSource::Camera,
                ..
            })
        ));
        assert_eq!(s.local_tracks().len(), 1);

        // 服务端分配 SID
        let effects = s
            .on_frame(SignalFrame::TrackPublished {
                track_sid: "TR_1".to_string(),
                kind: TrackKind::Video,
                source: TrackSource::Camera,
            })
            .expect("published");
        assert_eq!(
            effects,
            vec![SessionEffect::LocalTrackPublished {
                track_sid: "TR_1".to_string()
            }]
        );
        assert_eq!(s.local_tracks()[0].track_sid.as_deref(), Some("TR_1"));
        assert_eq!(s.local_tracks()[0].phase, LocalTrackPhase::Published);

        // 重复发布同源 → State 错误
        let track2 = Track::new(TrackKind::Video, TrackSource::Camera);
        assert!(matches!(
            s.publish_track(&track2),
            Err(LiveKitError::State(_))
        ));

        // 撤下: 等回执销账
        let effects = s.unpublish_track("TR_1").expect("unpublish");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackUnpublish { .. })
        ));
        let effects = s
            .on_frame(SignalFrame::TrackUnpublished {
                track_sid: "TR_1".to_string(),
            })
            .expect("unpublished");
        assert_eq!(
            effects,
            vec![SessionEffect::LocalTrackUnpublished {
                track_sid: "TR_1".to_string()
            }]
        );
        assert!(s.local_tracks().is_empty());

        // 未知轨道回执 → Protocol
        assert!(matches!(
            s.on_frame(SignalFrame::TrackUnpublished {
                track_sid: "TR_GHOST".to_string(),
            }),
            Err(LiveKitError::Protocol(_))
        ));
    }

    #[test]
    fn publish_track_requires_connected_state() {
        let mut s = session();
        let track = Track::new(TrackKind::Audio, TrackSource::Microphone);
        assert!(matches!(
            s.publish_track(&track),
            Err(LiveKitError::RoomDisconnected(_))
        ));
    }

    #[test]
    fn subscribe_lifecycle_and_violations() {
        let mut s = session();
        connect_to_active(&mut s);
        let effects = s.subscribe("TR_9").expect("subscribe");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackSubscribe { .. })
        ));

        // 重复订阅 → State
        assert!(matches!(s.subscribe("TR_9"), Err(LiveKitError::State(_))));

        // 订阅回执
        let effects = s
            .on_frame(SignalFrame::TrackSubscribed {
                track_sid: "TR_9".to_string(),
                participant_sid: "PA_2".to_string(),
                source: TrackSource::ScreenShare,
            })
            .expect("subscribed");
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::TrackSubscribed { .. })
        ));
        // 二次回执 → Protocol
        assert!(matches!(
            s.on_frame(SignalFrame::TrackSubscribed {
                track_sid: "TR_9".to_string(),
                participant_sid: "PA_2".to_string(),
                source: TrackSource::ScreenShare,
            }),
            Err(LiveKitError::Protocol(_))
        ));

        // 未订阅轨道的回执 → Protocol
        assert!(matches!(
            s.on_frame(SignalFrame::TrackSubscribed {
                track_sid: "TR_OTHER".to_string(),
                participant_sid: "PA_2".to_string(),
                source: TrackSource::ScreenShare,
            }),
            Err(LiveKitError::Protocol(_))
        ));

        // 取消订阅
        let effects = s.unsubscribe("TR_9").expect("unsubscribe");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackUnsubscribe { .. })
        ));
        let effects = s
            .on_frame(SignalFrame::TrackUnsubscribed {
                track_sid: "TR_9".to_string(),
                participant_sid: "PA_2".to_string(),
            })
            .expect("unsubscribed");
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::TrackUnsubscribed { .. })
        ));
        // 未知轨道取消 → TrackNotFound
        assert!(matches!(
            s.unsubscribe("TR_GHOST"),
            Err(LiveKitError::TrackNotFound(_))
        ));
    }

    #[test]
    fn subscribe_empty_sid_is_track_not_found() {
        let mut s = session();
        connect_to_active(&mut s);
        assert!(matches!(
            s.subscribe(""),
            Err(LiveKitError::TrackNotFound(_))
        ));
    }

    #[test]
    fn camera_and_microphone_toggle_is_idempotent() {
        let mut s = session();
        connect_to_active(&mut s);

        // 启用摄像头 → 发布
        let effects = s.set_camera_enabled(true).expect("enable");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackPublish {
                kind: TrackKind::Video,
                source: TrackSource::Camera,
                ..
            })
        ));
        s.on_frame(SignalFrame::TrackPublished {
            track_sid: "TR_CAM".to_string(),
            kind: TrackKind::Video,
            source: TrackSource::Camera,
        })
        .expect("published");

        // 再次启用 → 幂等空效应
        let effects = s.set_camera_enabled(true).expect("enable again");
        assert!(effects.is_empty());

        // 麦克风启用 (发布在途)
        let effects = s.set_microphone_enabled(true).expect("enable mic");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackPublish {
                kind: TrackKind::Audio,
                source: TrackSource::Microphone,
                ..
            })
        ));
        // 发布回执未到就禁用 → State 错误 (还没有 SID 可撤)
        assert!(matches!(
            s.set_microphone_enabled(false),
            Err(LiveKitError::State(_))
        ));
        s.on_frame(SignalFrame::TrackPublished {
            track_sid: "TR_MIC".to_string(),
            kind: TrackKind::Audio,
            source: TrackSource::Microphone,
        })
        .expect("published");

        // 禁用麦克风 → 撤下
        let effects = s.set_microphone_enabled(false).expect("disable mic");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackUnpublish { track_sid })
                if track_sid == "TR_MIC"
        ));
        s.on_frame(SignalFrame::TrackUnpublished {
            track_sid: "TR_MIC".to_string(),
        })
        .expect("unpublished");
        // 已禁用再禁用 → 幂等空效应
        let effects = s.set_microphone_enabled(false).expect("disable again");
        assert!(effects.is_empty(), "已禁用再禁用必须幂等");

        // 禁用摄像头 → 撤下
        let effects = s.set_camera_enabled(false).expect("disable cam");
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::TrackUnpublish { track_sid })
                if track_sid == "TR_CAM"
        ));
    }

    #[test]
    fn device_toggle_requires_connected_state() {
        let mut s = session();
        assert!(matches!(
            s.set_camera_enabled(true),
            Err(LiveKitError::RoomDisconnected(_))
        ));
        assert!(matches!(
            s.set_microphone_enabled(true),
            Err(LiveKitError::RoomDisconnected(_))
        ));
    }

    // ---------- 参与者 / 说话者 / 数据 ----------

    #[test]
    fn participant_lifecycle_and_violations() {
        let mut s = session();
        connect_to_active(&mut s);
        // 已有 PA_2
        let info = ParticipantInfo {
            sid: "PA_3".to_string(),
            identity: "user-3".to_string(),
            name: None,
            metadata: None,
        };
        let effects = s.on_participant_joined(info.clone()).expect("join");
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ParticipantConnected { .. })
        ));
        // 重复加入 → Protocol
        assert!(matches!(
            s.on_participant_joined(info),
            Err(LiveKitError::Protocol(_))
        ));
        // 离开
        let effects = s.on_participant_left("PA_3".to_string()).expect("left");
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ParticipantDisconnected { participant_sid })
                if participant_sid == "PA_3"
        ));
        // 未知离开 → Protocol
        assert!(matches!(
            s.on_participant_left("PA_GHOST".to_string()),
            Err(LiveKitError::Protocol(_))
        ));
    }

    #[test]
    fn active_speakers_event_only_in_connected() {
        let mut s = session();
        assert!(matches!(
            s.on_frame(SignalFrame::ActiveSpeakers {
                speakers: vec!["PA_1".into()]
            }),
            Err(LiveKitError::Protocol(_))
        ));
        connect_to_active(&mut s);
        let effects = s
            .on_frame(SignalFrame::ActiveSpeakers {
                speakers: vec!["PA_2".into()],
            })
            .expect("speakers");
        assert!(matches!(
            &effects[0],
            SessionEffect::Emit(RoomEvent::ActiveSpeakersChanged { .. })
        ));
    }

    #[test]
    fn data_message_chunks_reassemble_into_single_event() {
        let mut s = session();
        connect_to_active(&mut s);
        // 上行: 3 分块
        let effects = s
            .send_data_message("msg-1", &[1u8; MAX_DATA_CHUNK_BYTES * 2 + 1], true)
            .expect("send");
        assert_eq!(effects.len(), 3);
        assert!(matches!(
            &effects[0],
            SessionEffect::Send(SignalFrame::DataSend { chunk_index: 0, .. })
        ));

        // 下行: 2 分块 → 收齐才发事件
        let effects = s
            .on_frame(SignalFrame::DataReceived {
                message_id: "in-1".to_string(),
                chunk_index: 0,
                chunk_count: 2,
                participant_sid: "PA_2".to_string(),
                payload: vec![1, 2],
                reliable: true,
            })
            .expect("chunk 0");
        assert!(effects.is_empty(), "未收齐不产事件");
        let effects = s
            .on_frame(SignalFrame::DataReceived {
                message_id: "in-1".to_string(),
                chunk_index: 1,
                chunk_count: 2,
                participant_sid: "PA_2".to_string(),
                payload: vec![3, 4],
                reliable: true,
            })
            .expect("chunk 1");
        assert_eq!(
            effects,
            vec![SessionEffect::Emit(RoomEvent::DataReceived {
                participant_sid: "PA_2".to_string(),
                payload: vec![1, 2, 3, 4],
                reliable: true,
            })]
        );
    }

    #[test]
    fn negotiation_frames_pass_through_only_when_connected() {
        let mut s = session();
        assert!(matches!(
            s.on_frame(SignalFrame::Offer {
                sdp: "v=0".to_string()
            }),
            Err(LiveKitError::Protocol(_))
        ));
        connect_to_active(&mut s);
        let effects = s
            .on_frame(SignalFrame::IceCandidate {
                candidate: "candidate:1".to_string(),
                sdp_mid: Some("0".to_string()),
                sdp_mline_index: Some(0),
            })
            .expect("ice");
        assert!(matches!(
            &effects[0],
            SessionEffect::Negotiation(SignalFrame::IceCandidate { .. })
        ));
    }

    #[test]
    fn inbound_client_direction_frames_are_protocol_errors() {
        let mut s = session();
        connect_to_active(&mut s);
        assert!(matches!(
            s.on_frame(SignalFrame::Leave),
            Err(LiveKitError::Protocol(_))
        ));
        assert!(matches!(
            s.on_frame(SignalFrame::Hello {
                protocol_version: PROTOCOL_VERSION,
                client: "x".to_string()
            }),
            Err(LiveKitError::Protocol(_))
        ));
    }

    #[test]
    fn fatal_error_frame_terminates_and_classifies() {
        let mut s = session();
        connect_to_active(&mut s);
        let err = s
            .on_frame(SignalFrame::Error {
                code: "rate_limited".to_string(),
                message: "slow down".to_string(),
                fatal: true,
                retry_after_ms: Some(2_500),
            })
            .expect_err("must error");
        match err {
            LiveKitError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 2_500),
            other => panic!("expected rate limited, got {other:?}"),
        }
        assert_eq!(s.state(), RoomState::Disconnected, "fatal 必须终止会话");
    }

    #[test]
    fn non_fatal_error_frame_keeps_state() {
        let mut s = session();
        connect_to_active(&mut s);
        let err = s
            .on_frame(SignalFrame::Error {
                code: "protocol".to_string(),
                message: "bad frame".to_string(),
                fatal: false,
                retry_after_ms: None,
            })
            .expect_err("must error");
        assert!(matches!(err, LiveKitError::Protocol(_)));
        assert_eq!(s.state(), RoomState::Connected, "非致命不终止会话");
    }

    #[test]
    fn error_frame_code_classification_closed() {
        assert!(matches!(
            classify_server_error("rate_limited", "m", Some(10)),
            LiveKitError::RateLimited { retry_after_ms: 10 }
        ));
        assert!(matches!(
            classify_server_error("unauthorized", "m", None),
            LiveKitError::Authentication(_)
        ));
        assert!(matches!(
            classify_server_error("protocol", "m", None),
            LiveKitError::Protocol(_)
        ));
        assert!(matches!(
            classify_server_error("network", "m", None),
            LiveKitError::Network(_)
        ));
        assert!(matches!(
            classify_server_error("timeout", "m", None),
            LiveKitError::Timeout { .. }
        ));
        assert!(matches!(
            classify_server_error("backpressure", "m", None),
            LiveKitError::Backpressure(_)
        ));
        assert!(matches!(
            classify_server_error("bogus", "m", None),
            LiveKitError::Internal(_)
        ));

        assert!(matches!(
            classify_server_rejection("invalid_token", "m"),
            LiveKitError::Authentication(_)
        ));
        assert!(matches!(
            classify_server_rejection("rate_limited", "m"),
            LiveKitError::RateLimited { .. }
        ));
        assert!(matches!(
            classify_server_rejection("room_full", "m"),
            LiveKitError::State(_)
        ));
        assert!(matches!(
            classify_server_rejection("bogus", "m"),
            LiveKitError::Internal(_)
        ));
    }

    #[test]
    fn to_room_snapshot_matches_session() {
        let mut s = session();
        connect_to_active(&mut s);
        let room = s.to_room().expect("snapshot");
        assert_eq!(room.name(), "room-1");
        assert_eq!(room.state(), RoomState::Connected);
    }

    #[test]
    fn stats_snapshot_tracks_counters() {
        let mut s = session();
        connect_to_active(&mut s);
        s.on_heartbeat_tick().expect("tick");
        let stats = s.stats();
        assert_eq!(stats.state, RoomState::Connected);
        assert_eq!(stats.heartbeats_sent, 1);
        assert_eq!(stats.frames_sent, 3, "hello + join + 心跳 = 3 帧");
    }

    #[test]
    fn session_constructor_validates_inputs() {
        assert!(matches!(
            SignalingSession::new("", "u", SecretValue::new("t"), RoomOptions::default()),
            Err(LiveKitError::RoomNameEmpty)
        ));
        assert!(matches!(
            SignalingSession::new("r", "", SecretValue::new("t"), RoomOptions::default()),
            Err(LiveKitError::InvalidArgument(_))
        ));
        assert!(matches!(
            SignalingSession::new("r", "u", SecretValue::new(""), RoomOptions::default()),
            Err(LiveKitError::Authentication(_))
        ));
    }
}
