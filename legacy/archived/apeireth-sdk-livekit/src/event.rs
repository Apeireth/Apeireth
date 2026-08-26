//! LiveKit 房间事件订阅 (per livekit-client v0.9.21 1:1 翻译)
//!
//! 1:1 翻译 v0.9.21 商业版 `RoomEvent` enum (per livekit-client/dist/src/room/RoomEvent.d.ts):
//! - 8 事件: ParticipantConnected / ParticipantDisconnected / TrackSubscribed / TrackUnsubscribed
//!   / ActiveSpeakersChanged / ConnectionStateChanged / DataReceived / Reconnected
//!
//! **当前 skeleton 不真接 livekit-server**, EventEmitter 占位, R21 续真接时用 tokio::sync::broadcast.

use std::sync::Arc;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::participant::{Participant, ParticipantSid};
use crate::room::RoomState;
use crate::track::{TrackSid, TrackSource};

// ============================================================================
// §1 RoomEvent 8 事件 (per v0.9.21 商业版 RoomEvent enum 1:1)
// ============================================================================

/// 房间事件 (8 事件, K-1 强校验守门: 编译期 hardcode 8 个 variant).
///
/// 字段对应 v0.9.21 商业版 `RoomEvent` enum:
/// 1. `ParticipantConnected`
/// 2. `ParticipantDisconnected`
/// 3. `TrackSubscribed`
/// 4. `TrackUnsubscribed`
/// 5. `ActiveSpeakersChanged`
/// 6. `ConnectionStateChanged`
/// 7. `DataReceived`
/// 8. `Reconnected`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RoomEvent {
    /// **事件 1**: 新参与者加入房间 (per `RoomEvent.ParticipantConnected`).
    ParticipantConnected {
        /// 新加入的参与者
        participant: Participant,
    },
    /// **事件 2**: 参与者离开房间 (per `RoomEvent.ParticipantDisconnected`).
    ParticipantDisconnected {
        /// 离开的参与者 SID
        participant_sid: ParticipantSid,
    },
    /// **事件 3**: 已订阅远端 track (per `RoomEvent.TrackSubscribed`).
    TrackSubscribed {
        /// 远端轨道 SID
        track_sid: TrackSid,
        /// 远端参与者 SID
        participant_sid: ParticipantSid,
        /// 轨道来源
        source: TrackSource,
    },
    /// **事件 4**: 取消订阅远端 track (per `RoomEvent.TrackUnsubscribed`).
    TrackUnsubscribed {
        /// 远端轨道 SID
        track_sid: TrackSid,
        /// 远端参与者 SID
        participant_sid: ParticipantSid,
    },
    /// **事件 5**: 当前说话者变化 (per `RoomEvent.ActiveSpeakersChanged`).
    ///
    /// 携带当前所有"在说话"的参与者 SID 列表.
    ActiveSpeakersChanged {
        /// 当前说话者 SID 列表
        speakers: Vec<ParticipantSid>,
    },
    /// **事件 6**: 连接状态变化 (per `RoomEvent.ConnectionStateChanged`).
    ConnectionStateChanged {
        /// 旧状态
        previous: RoomState,
        /// 新状态
        current: RoomState,
    },
    /// **事件 7**: 收到 data message (per `RoomEvent.DataReceived`).
    ///
    /// LiveKit data channel 携带任意字节 (UTF-8 字符串 / 二进制 protobuf 等).
    DataReceived {
        /// 发送者 SID
        participant_sid: ParticipantSid,
        /// data payload (字节)
        payload: Vec<u8>,
        /// 是否可靠 (reliable vs lossy, per LiveKit data channel 选项)
        reliable: bool,
    },
    /// **事件 8**: 重连成功 (per `RoomEvent.Reconnected`).
    ///
    /// 仅在 `RoomState::Reconnecting` 后恢复到 `Connected` 时触发.
    Reconnected {
        /// 重连前断开时间戳 (per `SystemTime`, 给监控用)
        disconnected_at: Option<SystemTime>,
    },
}

impl RoomEvent {
    /// 8 事件 hardcode 常量.
    pub const COUNT: usize = 8;
    /// 事件类型字符串 (1:1 翻译 livekit-client v0.9.21).
    pub fn type_str(&self) -> &'static str {
        match self {
            RoomEvent::ParticipantConnected { .. } => "participant_connected",
            RoomEvent::ParticipantDisconnected { .. } => "participant_disconnected",
            RoomEvent::TrackSubscribed { .. } => "track_subscribed",
            RoomEvent::TrackUnsubscribed { .. } => "track_unsubscribed",
            RoomEvent::ActiveSpeakersChanged { .. } => "active_speakers_changed",
            RoomEvent::ConnectionStateChanged { .. } => "connection_state_changed",
            RoomEvent::DataReceived { .. } => "data_received",
            RoomEvent::Reconnected { .. } => "reconnected",
        }
    }
    /// 是否为参与者事件 (ParticipantConnected / ParticipantDisconnected).
    pub fn is_participant_event(&self) -> bool {
        matches!(
            self,
            RoomEvent::ParticipantConnected { .. } | RoomEvent::ParticipantDisconnected { .. }
        )
    }
    /// 是否为 track 事件 (TrackSubscribed / TrackUnsubscribed).
    pub fn is_track_event(&self) -> bool {
        matches!(
            self,
            RoomEvent::TrackSubscribed { .. } | RoomEvent::TrackUnsubscribed { .. }
        )
    }
    /// 是否为状态事件 (ConnectionStateChanged / Reconnected).
    pub fn is_state_event(&self) -> bool {
        matches!(
            self,
            RoomEvent::ConnectionStateChanged { .. } | RoomEvent::Reconnected { .. }
        )
    }
}

/// 8 RoomEvent hardcode 常量 (K-1 强校验守门: 编译期保证 8 个事件).
pub const SUPPORTED_ROOM_EVENTS: &[&str] = &[
    "participant_connected",
    "participant_disconnected",
    "track_subscribed",
    "track_unsubscribed",
    "active_speakers_changed",
    "connection_state_changed",
    "data_received",
    "reconnected",
];
const _: () = assert!(SUPPORTED_ROOM_EVENTS.len() == 8);

// ============================================================================
// §2 EventEmitter (per v0.9.21 商业版 Room.on / Room.off 1:1 翻译)
// ============================================================================

/// 事件发射器 (per v0.9.21 商业版 `EventEmitter` 1:1 翻译, 用 tokio::broadcast 实现).
///
/// 字段对应 v0.9.21 商业版 Room 内部 emitter:
/// - `tx` (per tokio::broadcast::Sender<RoomEvent>, 8 事件共享一个 channel)
/// - `capacity` (per 100 条 buffer, R21 续可配)
#[derive(Debug, Clone)]
pub struct EventEmitter {
    /// 事件广播 channel sender (per v0.9.21 商业版 `RoomEventEmitter`)
    tx: broadcast::Sender<RoomEvent>,
}

impl EventEmitter {
    /// 创建新事件发射器 (capacity = 100, per LiveKit 默认).
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// 订阅事件 (返 `broadcast::Receiver<RoomEvent>`, 跟 LiveKit `Room.on` 1:1).
    pub fn subscribe(&self) -> broadcast::Receiver<RoomEvent> {
        self.tx.subscribe()
    }

    /// 发射事件 (per v0.9.21 商业版 `_emit` 内部, R21 续由 signal protocol 调).
    ///
    /// **STUB 模式**: 当前仅由测试代码调, 真实场景 R21 续真接 livekit-server 后由 SDK 内部触发.
    pub fn emit(&self, event: RoomEvent) -> Result<usize, broadcast::error::SendError<RoomEvent>> {
        self.tx.send(event)
    }

    /// 当前订阅者数 (per `ReceiverCount`, 给测试 / 监控用).
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new(100)
    }
}

// ============================================================================
// §3 共享 Arc<EventEmitter> 别名 (per lib.rs 共享用)
// ============================================================================

/// Arc 包装的事件发射器 (per `Arc<EventEmitter>`, 跨 6 核心 API 共享).
pub type SharedEmitter = Arc<EventEmitter>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LiveKitError;
    use crate::track::TrackKind;

    fn make_participant() -> Participant {
        Participant::new("user-1").unwrap()
    }

    #[test]
    fn room_event_8_variants_type_str() {
        assert_eq!(RoomEvent::COUNT, 8);
        assert_eq!(SUPPORTED_ROOM_EVENTS.len(), 8);

        let p = make_participant();
        assert_eq!(
            RoomEvent::ParticipantConnected { participant: p.clone() }.type_str(),
            "participant_connected"
        );
        assert_eq!(
            RoomEvent::ParticipantDisconnected { participant_sid: "PA_1".to_string() }.type_str(),
            "participant_disconnected"
        );
        assert_eq!(
            RoomEvent::TrackSubscribed {
                track_sid: "TR_1".to_string(),
                participant_sid: "PA_1".to_string(),
                source: TrackSource::Camera,
            }
            .type_str(),
            "track_subscribed"
        );
        assert_eq!(
            RoomEvent::TrackUnsubscribed {
                track_sid: "TR_1".to_string(),
                participant_sid: "PA_1".to_string(),
            }
            .type_str(),
            "track_unsubscribed"
        );
        assert_eq!(
            RoomEvent::ActiveSpeakersChanged { speakers: vec!["PA_1".to_string()] }.type_str(),
            "active_speakers_changed"
        );
        assert_eq!(
            RoomEvent::ConnectionStateChanged {
                previous: RoomState::Disconnected,
                current: RoomState::Connected,
            }
            .type_str(),
            "connection_state_changed"
        );
        assert_eq!(
            RoomEvent::DataReceived {
                participant_sid: "PA_1".to_string(),
                payload: vec![1, 2, 3],
                reliable: true,
            }
            .type_str(),
            "data_received"
        );
        assert_eq!(
            RoomEvent::Reconnected { disconnected_at: None }.type_str(),
            "reconnected"
        );
    }

    #[test]
    fn room_event_is_participant_event() {
        let p = make_participant();
        assert!(RoomEvent::ParticipantConnected { participant: p.clone() }.is_participant_event());
        assert!(RoomEvent::ParticipantDisconnected { participant_sid: "PA_1".to_string() }.is_participant_event());
        assert!(!RoomEvent::Reconnected { disconnected_at: None }.is_participant_event());
    }

    #[test]
    fn room_event_is_track_event() {
        assert!(RoomEvent::TrackSubscribed {
            track_sid: "TR_1".to_string(),
            participant_sid: "PA_1".to_string(),
            source: TrackSource::Camera,
        }
        .is_track_event());
        assert!(RoomEvent::TrackUnsubscribed {
            track_sid: "TR_1".to_string(),
            participant_sid: "PA_1".to_string(),
        }
        .is_track_event());
        assert!(!RoomEvent::Reconnected { disconnected_at: None }.is_track_event());
    }

    #[test]
    fn room_event_is_state_event() {
        assert!(RoomEvent::ConnectionStateChanged {
            previous: RoomState::Disconnected,
            current: RoomState::Connected,
        }
        .is_state_event());
        assert!(RoomEvent::Reconnected { disconnected_at: None }.is_state_event());
        assert!(!RoomEvent::ParticipantConnected { participant: make_participant() }.is_state_event());
    }

    #[test]
    fn event_emitter_creation() {
        let emitter = EventEmitter::new(100);
        assert_eq!(emitter.receiver_count(), 0);
    }

    #[test]
    fn event_emitter_subscribe_and_emit() {
        let emitter = EventEmitter::new(100);
        let mut rx = emitter.subscribe();
        assert_eq!(emitter.receiver_count(), 1);

        let event = RoomEvent::Reconnected { disconnected_at: None };
        let sent_count = emitter.emit(event.clone()).expect("emit must succeed");
        assert_eq!(sent_count, 1);

        let received = rx.try_recv().expect("receive must succeed");
        assert_eq!(received, event);
    }

    #[test]
    fn event_emitter_multiple_subscribers() {
        let emitter = EventEmitter::new(100);
        let mut rx1 = emitter.subscribe();
        let mut rx2 = emitter.subscribe();
        assert_eq!(emitter.receiver_count(), 2);

        let event = RoomEvent::Reconnected { disconnected_at: None };
        emitter.emit(event.clone()).expect("emit must succeed");

        let r1 = rx1.try_recv().expect("rx1 must receive");
        let r2 = rx2.try_recv().expect("rx2 must receive");
        assert_eq!(r1, event);
        assert_eq!(r2, event);
    }

    #[test]
    fn event_emitter_default_capacity() {
        let emitter = EventEmitter::default();
        assert_eq!(emitter.receiver_count(), 0);
    }

    #[test]
    fn supported_room_events_has_8() {
        for evt in SUPPORTED_ROOM_EVENTS {
            // 每个 type_str 必须非空
            assert!(!evt.is_empty());
        }
    }

    // helper 抑制 unused warning
    #[allow(dead_code)]
    fn _force_track_kind_use() -> TrackKind {
        TrackKind::Video
    }
    #[allow(dead_code)]
    fn _force_livekit_error_use() -> Result<(), LiveKitError> {
        Ok(())
    }
}
