//! 信令协议帧定义 + 编解码 + 传输边界 (WebRTC 信令 / 房间 / 轨道发布订阅协议客户端层).
//!
//! 线格式: 长度前缀 + UTF-8 JSON 帧体 (共用 [`crate::frame_codec`] 纪律),
//! 帧体为 `{"type": "<snake_case>", ...}` 内部标签对象。
//!
//! 握手序列 (客户端视角):
//!
//! ```text
//! client -> Hello{protocol_version, client}          (版本协商)
//! server -> Welcome{protocol_version, session_id, heartbeat_interval_ms}
//! client -> Join{room, identity, token}              (token 走脱敏包装)
//! server -> JoinAccepted{room_sid, participant_sid, participants}
//!        | JoinRejected{reason}                      (认证 / 限流 / 协议拒绝)
//! ```
//!
//! 媒体生命周期帧: `TrackPublish` / `TrackPublished` / `TrackUnpublish` /
//! `TrackSubscribe` / `TrackSubscribed` / `TrackUnsubscribe` / `TrackUnsubscribed`;
//! 参与者生命周期帧: `ParticipantJoined` / `ParticipantLeft` / `ActiveSpeakers`;
//! 数据面: `DataSend` / `DataReceived` (大消息按 `message_id` + `chunk_index` 分块);
//! 保活: `Heartbeat` / `HeartbeatAck`; 离开: `Leave` / `LeaveAck`; 拒绝: `Error`。
//!
//! 传输边界 [`SignalTransport`] 是唯一 IO 面: 协议层不直接碰网络,
//! 生产实现注入 WebSocket 传输, 测试注入脚本化 mock (零真实网络)。

use serde::{Deserialize, Serialize};

use crate::frame_codec::{encode_frame, StreamDecoder};
use crate::livekit::error::LiveKitError;
use crate::livekit::participant::{Participant, ParticipantSid};
use crate::livekit::track::{TrackKind, TrackSid, TrackSource};
use crate::redact::SecretValue;

/// 信令协议版本 (握手版本协商守门).
pub const PROTOCOL_VERSION: u32 = 1;

/// 握手 `Hello.client` 客户端标识 (稳定字面量, 便于服务端兼容分支).
pub const CLIENT_NAME: &str = "apeireth-sdk-signal";

/// 服务端建议心跳间隔的默认值 (毫秒; `Welcome` 可覆盖).
pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 5_000;

/// 心跳超时默认值 (毫秒): 连续 [`MAX_HEARTBEAT_MISSED`] 次心跳无响应判失联.
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 15_000;

/// 心跳最大连续丢失次数 (超过即进入重连状态机).
pub const MAX_HEARTBEAT_MISSED: u32 = 3;

/// 单条数据消息分块体上界 (字节; 超限必须先分块).
pub const MAX_DATA_CHUNK_BYTES: usize = 16 * 1024;

/// 单条数据消息最大分块数 (防超大消息拖垮重组缓冲).
pub const MAX_DATA_CHUNKS: u32 = 256;

// ============================================================================
// §1 帧载荷类型
// ============================================================================

/// 参与者信令描述 (握手 / 加入帧携带).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipantInfo {
    /// 参与者 SID (服务端分配).
    pub sid: ParticipantSid,
    /// 参与者 identity (客户端声明).
    pub identity: String,
    /// 显示名 (可选).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 自定义 metadata (可选).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

impl ParticipantInfo {
    /// 转成房间层参与者模型.
    pub fn into_participant(self) -> Result<Participant, LiveKitError> {
        let mut p = Participant::new(self.identity)?;
        p.set_sid(self.sid);
        if let Some(name) = self.name {
            p.set_name(name);
        }
        if let Some(metadata) = self.metadata {
            p.set_metadata(metadata);
        }
        Ok(p)
    }
}

/// 信令协议帧 (26 变体, 闭合词表).
///
/// 载荷纪律: `Join.token` 走 [`SecretValue`] 脱敏包装 —— 帧的 `Debug` /
/// `Display` 输出绝不携带明文令牌。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalFrame {
    /// 客户端握手请求 (版本协商).
    Hello {
        /// 客户端支持的协议版本.
        protocol_version: u32,
        /// 客户端标识.
        client: String,
    },
    /// 服务端握手接受.
    Welcome {
        /// 服务端选定协议版本.
        protocol_version: u32,
        /// 服务端会话 ID.
        session_id: String,
        /// 服务端建议心跳间隔 (毫秒).
        heartbeat_interval_ms: u64,
    },
    /// 加入房间请求 (携带脱敏令牌).
    Join {
        /// 房间名.
        room: String,
        /// 身份.
        identity: String,
        /// 访问令牌 (脱敏包装).
        token: SecretValue,
    },
    /// 加入成功.
    JoinAccepted {
        /// 房间 SID.
        room_sid: String,
        /// 本端参与者 SID.
        participant_sid: ParticipantSid,
        /// 房间内已有参与者.
        participants: Vec<ParticipantInfo>,
    },
    /// 加入被拒 (认证失败 / 房间满 / 限流).
    JoinRejected {
        /// 拒绝原因类别 (稳定字符串).
        reason: String,
        /// 拒绝说明.
        message: String,
    },
    /// SDP offer.
    Offer {
        /// SDP 文本.
        sdp: String,
    },
    /// SDP answer.
    Answer {
        /// SDP 文本.
        sdp: String,
    },
    /// ICE 候选 (trickle).
    IceCandidate {
        /// SDP candidate 行.
        candidate: String,
        /// 媒体标识 (可选).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sdp_mid: Option<String>,
        /// 媒体行索引 (可选).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sdp_mline_index: Option<u16>,
    },
    /// 请求发布本地轨道.
    TrackPublish {
        /// 轨道类型.
        kind: TrackKind,
        /// 轨道来源.
        source: TrackSource,
        /// 轨道显示名 (可选).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// 发布成功 (服务端分配轨道 SID).
    TrackPublished {
        /// 轨道 SID (服务端分配).
        track_sid: TrackSid,
        /// 轨道类型.
        kind: TrackKind,
        /// 轨道来源.
        source: TrackSource,
    },
    /// 撤下本地轨道.
    TrackUnpublish {
        /// 轨道 SID.
        track_sid: TrackSid,
    },
    /// 撤下完成.
    TrackUnpublished {
        /// 轨道 SID.
        track_sid: TrackSid,
    },
    /// 请求订阅远端轨道.
    TrackSubscribe {
        /// 轨道 SID.
        track_sid: TrackSid,
    },
    /// 订阅成功.
    TrackSubscribed {
        /// 轨道 SID.
        track_sid: TrackSid,
        /// 轨道所属参与者.
        participant_sid: ParticipantSid,
        /// 轨道来源.
        source: TrackSource,
    },
    /// 请求取消订阅.
    TrackUnsubscribe {
        /// 轨道 SID.
        track_sid: TrackSid,
    },
    /// 取消订阅完成.
    TrackUnsubscribed {
        /// 轨道 SID.
        track_sid: TrackSid,
        /// 轨道所属参与者.
        participant_sid: ParticipantSid,
    },
    /// 参与者加入.
    ParticipantJoined {
        /// 参与者描述.
        participant: ParticipantInfo,
    },
    /// 参与者离开.
    ParticipantLeft {
        /// 参与者 SID.
        participant_sid: ParticipantSid,
    },
    /// 活跃说话者变化.
    ActiveSpeakers {
        /// 当前说话者 SID 列表.
        speakers: Vec<ParticipantSid>,
    },
    /// 上行数据分块.
    DataSend {
        /// 消息 ID (重组键).
        message_id: String,
        /// 分块序号 (0 起).
        chunk_index: u32,
        /// 分块总数.
        chunk_count: u32,
        /// 分块载荷.
        payload: Vec<u8>,
        /// 是否可靠投递.
        reliable: bool,
    },
    /// 下行数据分块.
    DataReceived {
        /// 消息 ID (重组键).
        message_id: String,
        /// 分块序号 (0 起).
        chunk_index: u32,
        /// 分块总数.
        chunk_count: u32,
        /// 发送者 SID.
        participant_sid: ParticipantSid,
        /// 分块载荷.
        payload: Vec<u8>,
        /// 是否可靠投递.
        reliable: bool,
    },
    /// 心跳请求.
    Heartbeat {
        /// 发送时刻 (毫秒, 用于回声测 RTT).
        timestamp_ms: u64,
    },
    /// 心跳响应.
    HeartbeatAck {
        /// 回声时刻 (毫秒).
        timestamp_ms: u64,
    },
    /// 请求离开房间.
    Leave,
    /// 离开确认.
    LeaveAck,
    /// 服务端错误帧 (限流 / 认证 / 协议).
    Error {
        /// 稳定错误码 (如 `rate_limited` / `unauthorized` / `protocol`).
        code: String,
        /// 说明 (不得携带秘密).
        message: String,
        /// 是否致命 (致命 → 立即断开).
        fatal: bool,
        /// 限流建议退避时长 (毫秒, 仅限流类携带).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        retry_after_ms: Option<u64>,
    },
}

impl SignalFrame {
    /// 帧类型稳定字符串 (与 wire `type` 字段一致).
    pub fn type_str(&self) -> &'static str {
        match self {
            SignalFrame::Hello { .. } => "hello",
            SignalFrame::Welcome { .. } => "welcome",
            SignalFrame::Join { .. } => "join",
            SignalFrame::JoinAccepted { .. } => "join_accepted",
            SignalFrame::JoinRejected { .. } => "join_rejected",
            SignalFrame::Offer { .. } => "offer",
            SignalFrame::Answer { .. } => "answer",
            SignalFrame::IceCandidate { .. } => "ice_candidate",
            SignalFrame::TrackPublish { .. } => "track_publish",
            SignalFrame::TrackPublished { .. } => "track_published",
            SignalFrame::TrackUnpublish { .. } => "track_unpublish",
            SignalFrame::TrackUnpublished { .. } => "track_unpublished",
            SignalFrame::TrackSubscribe { .. } => "track_subscribe",
            SignalFrame::TrackSubscribed { .. } => "track_subscribed",
            SignalFrame::TrackUnsubscribe { .. } => "track_unsubscribe",
            SignalFrame::TrackUnsubscribed { .. } => "track_unsubscribed",
            SignalFrame::ParticipantJoined { .. } => "participant_joined",
            SignalFrame::ParticipantLeft { .. } => "participant_left",
            SignalFrame::ActiveSpeakers { .. } => "active_speakers",
            SignalFrame::DataSend { .. } => "data_send",
            SignalFrame::DataReceived { .. } => "data_received",
            SignalFrame::Heartbeat { .. } => "heartbeat",
            SignalFrame::HeartbeatAck { .. } => "heartbeat_ack",
            SignalFrame::Leave => "leave",
            SignalFrame::LeaveAck => "leave_ack",
            SignalFrame::Error { .. } => "error",
        }
    }

    /// 帧变体总数 (闭合词表规模).
    pub const COUNT: usize = 26;

    /// 是否为握手阶段帧 (Hello / Welcome / Join / JoinAccepted / JoinRejected).
    pub fn is_handshake_frame(&self) -> bool {
        matches!(
            self,
            SignalFrame::Hello { .. }
                | SignalFrame::Welcome { .. }
                | SignalFrame::Join { .. }
                | SignalFrame::JoinAccepted { .. }
                | SignalFrame::JoinRejected { .. }
        )
    }
}

// ============================================================================
// §2 编解码 (复用 crate::frame_codec 长度前缀纪律)
// ============================================================================

/// 信令帧解码器 (长度前缀 + 半包缓存 + 出错熔断).
pub type SignalDecoder = StreamDecoder<SignalFrame>;

/// 编码一帧信令 (长度前缀 + JSON 帧体).
pub fn encode_signal_frame(frame: &SignalFrame) -> Result<Vec<u8>, LiveKitError> {
    encode_frame(frame).map_err(LiveKitError::from)
}

// ============================================================================
// §3 传输边界 (生产注入 WebSocket, 测试注入 mock; 零真实网络)
// ============================================================================

/// 信令传输边界: 协议层唯一 IO 面.
///
/// 实现约定:
/// - `send` 保序, 失败必须返 [`LiveKitError::Network`] 类错误 (可重试判定交给上层);
/// - `recv` 在对端关闭时返 [`LiveKitError::Network`] 类错误, 不得静默返假帧。
#[async_trait::async_trait]
pub trait SignalTransport: Send + Sync {
    /// 发送一帧.
    async fn send(&self, frame: SignalFrame) -> Result<(), LiveKitError>;
    /// 接收下一帧.
    async fn recv(&self) -> Result<SignalFrame, LiveKitError>;
}

// ============================================================================
// §4 数据消息分块 / 重组 (大消息按 chunk 传输, 防单帧超限)
// ============================================================================

/// 把一条数据消息切成 `DataSend` 分块 (顺序 0..chunk_count).
///
/// 空载荷消息也产生 1 个分块 (chunk_count=1), 保证对端重组语义一致。
pub fn chunk_data_message(
    message_id: &str,
    payload: &[u8],
    reliable: bool,
) -> Result<Vec<SignalFrame>, LiveKitError> {
    if payload.len() > MAX_DATA_CHUNK_BYTES * MAX_DATA_CHUNKS as usize {
        return Err(LiveKitError::Protocol(format!(
            "data message too large: {} bytes (max {})",
            payload.len(),
            MAX_DATA_CHUNK_BYTES * MAX_DATA_CHUNKS as usize
        )));
    }
    let chunk_size = MAX_DATA_CHUNK_BYTES.max(1);
    let chunk_count = if payload.is_empty() {
        1
    } else {
        payload.len().div_ceil(chunk_size) as u32
    };
    let mut frames = Vec::with_capacity(chunk_count as usize);
    for chunk_index in 0..chunk_count {
        let start = chunk_index as usize * chunk_size;
        let end = ((chunk_index as usize + 1) * chunk_size).min(payload.len());
        frames.push(SignalFrame::DataSend {
            message_id: message_id.to_string(),
            chunk_index,
            chunk_count,
            payload: payload[start..end].to_vec(),
            reliable,
        });
    }
    Ok(frames)
}

/// 数据消息重组器: 按 `message_id` 收齐分块, 校验序号 / 总数一致性.
#[derive(Debug, Default)]
pub struct DataMessageAssembler {
    /// 进行中的消息: message_id → (chunk_count, 已收分块).
    pending: std::collections::HashMap<String, PendingMessage>,
}

#[derive(Debug)]
struct PendingMessage {
    chunk_count: u32,
    chunks: std::collections::HashMap<u32, Vec<u8>>,
}

impl DataMessageAssembler {
    /// 创建空重组器.
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前进行中的消息数 (背压观测用).
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 喂入一个数据分块; 收齐返回 `Some(完整载荷)`.
    ///
    /// 违例 (chunk_count 前后不一致 / 越界序号 / 重复序号) 一律
    /// [`LiveKitError::Protocol`], 不做静默修复。
    pub fn push_chunk(
        &mut self,
        message_id: &str,
        chunk_index: u32,
        chunk_count: u32,
        payload: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, LiveKitError> {
        if chunk_count == 0 || chunk_count > MAX_DATA_CHUNKS {
            return Err(LiveKitError::Protocol(format!(
                "data chunk_count {chunk_count} out of range 1..={MAX_DATA_CHUNKS}"
            )));
        }
        if chunk_index >= chunk_count {
            return Err(LiveKitError::Protocol(format!(
                "data chunk_index {chunk_index} out of range 0..{chunk_count}"
            )));
        }
        if payload.len() > MAX_DATA_CHUNK_BYTES {
            return Err(LiveKitError::Protocol(format!(
                "data chunk payload {} bytes exceeds {MAX_DATA_CHUNK_BYTES}",
                payload.len()
            )));
        }
        let entry = self
            .pending
            .entry(message_id.to_string())
            .or_insert_with(|| PendingMessage {
                chunk_count,
                chunks: std::collections::HashMap::new(),
            });
        if entry.chunk_count != chunk_count {
            return Err(LiveKitError::Protocol(format!(
                "data chunk_count mismatch for message {message_id}: {} vs {chunk_count}",
                entry.chunk_count
            )));
        }
        if entry.chunks.insert(chunk_index, payload).is_some() {
            return Err(LiveKitError::Protocol(format!(
                "duplicate data chunk {chunk_index} for message {message_id}"
            )));
        }
        if entry.chunks.len() == chunk_count as usize {
            let done = self
                .pending
                .remove(message_id)
                .expect("just verified present");
            let mut out = Vec::new();
            for i in 0..chunk_count {
                out.extend_from_slice(&done.chunks[&i]);
            }
            return Ok(Some(out));
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_frames() -> Vec<SignalFrame> {
        vec![
            SignalFrame::Hello {
                protocol_version: PROTOCOL_VERSION,
                client: CLIENT_NAME.to_string(),
            },
            SignalFrame::Welcome {
                protocol_version: PROTOCOL_VERSION,
                session_id: "sess-1".to_string(),
                heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            },
            SignalFrame::Join {
                room: "room-1".to_string(),
                identity: "user-1".to_string(),
                token: SecretValue::new("secret-token-value"),
            },
            SignalFrame::JoinAccepted {
                room_sid: "RM_1".to_string(),
                participant_sid: "PA_1".to_string(),
                participants: vec![ParticipantInfo {
                    sid: "PA_2".to_string(),
                    identity: "user-2".to_string(),
                    name: Some("User Two".to_string()),
                    metadata: Some("{\"role\":\"guest\"}".to_string()),
                }],
            },
            SignalFrame::JoinRejected {
                reason: "unauthorized".to_string(),
                message: "token rejected".to_string(),
            },
            SignalFrame::Offer {
                sdp: "v=0\\r\\no=- 1 1 IN IP4 0.0.0.0\\r\\n".to_string(),
            },
            SignalFrame::Answer {
                sdp: "v=0\\r\\no=- 2 2 IN IP4 0.0.0.0\\r\\n".to_string(),
            },
            SignalFrame::IceCandidate {
                candidate: "candidate:1 1 UDP 1 0.0.0.0 1 typ host".to_string(),
                sdp_mid: Some("0".to_string()),
                sdp_mline_index: Some(0),
            },
            SignalFrame::TrackPublish {
                kind: TrackKind::Video,
                source: TrackSource::Camera,
                name: Some("camera".to_string()),
            },
            SignalFrame::TrackPublished {
                track_sid: "TR_1".to_string(),
                kind: TrackKind::Video,
                source: TrackSource::Camera,
            },
            SignalFrame::TrackUnpublish {
                track_sid: "TR_1".to_string(),
            },
            SignalFrame::TrackUnpublished {
                track_sid: "TR_1".to_string(),
            },
            SignalFrame::TrackSubscribe {
                track_sid: "TR_2".to_string(),
            },
            SignalFrame::TrackSubscribed {
                track_sid: "TR_2".to_string(),
                participant_sid: "PA_2".to_string(),
                source: TrackSource::Microphone,
            },
            SignalFrame::TrackUnsubscribe {
                track_sid: "TR_2".to_string(),
            },
            SignalFrame::TrackUnsubscribed {
                track_sid: "TR_2".to_string(),
                participant_sid: "PA_2".to_string(),
            },
            SignalFrame::ParticipantJoined {
                participant: ParticipantInfo {
                    sid: "PA_3".to_string(),
                    identity: "user-3".to_string(),
                    name: None,
                    metadata: None,
                },
            },
            SignalFrame::ParticipantLeft {
                participant_sid: "PA_3".to_string(),
            },
            SignalFrame::ActiveSpeakers {
                speakers: vec!["PA_1".to_string(), "PA_2".to_string()],
            },
            SignalFrame::DataSend {
                message_id: "msg-1".to_string(),
                chunk_index: 0,
                chunk_count: 2,
                payload: vec![1, 2, 3],
                reliable: true,
            },
            SignalFrame::DataReceived {
                message_id: "msg-2".to_string(),
                chunk_index: 1,
                chunk_count: 2,
                participant_sid: "PA_2".to_string(),
                payload: vec![9],
                reliable: false,
            },
            SignalFrame::Heartbeat { timestamp_ms: 1700 },
            SignalFrame::HeartbeatAck { timestamp_ms: 1700 },
            SignalFrame::Leave,
            SignalFrame::LeaveAck,
            SignalFrame::Error {
                code: "rate_limited".to_string(),
                message: "too many join attempts".to_string(),
                fatal: false,
                retry_after_ms: Some(2_000),
            },
        ]
    }

    #[test]
    fn frame_table_is_closed_at_26() {
        assert_eq!(SignalFrame::COUNT, 26);
        assert_eq!(all_frames().len(), 26);
    }

    #[test]
    fn codec_roundtrip_every_frame_variant() {
        for frame in all_frames() {
            let bytes = encode_signal_frame(&frame).expect("encode");
            let mut decoder = SignalDecoder::new();
            decoder.push(&bytes);
            let decoded = decoder.next_frame().expect("decode").expect("one frame");
            assert_eq!(
                decoded,
                frame,
                "roundtrip mismatch for {}",
                frame.type_str()
            );
            // wire `type` 字段与 type_str 一致
            let json: serde_json::Value =
                serde_json::from_slice(&bytes[4..]).expect("body is json");
            assert_eq!(json["type"], frame.type_str());
        }
    }

    #[test]
    fn codec_roundtrip_streaming_batch() {
        let frames = all_frames();
        let mut bytes = Vec::new();
        for f in &frames {
            bytes.extend_from_slice(&encode_signal_frame(f).expect("encode"));
        }
        let mut decoder = SignalDecoder::new();
        decoder.push(&bytes);
        let decoded = decoder.drain_frames().expect("drain");
        assert_eq!(decoded, frames);
    }

    #[test]
    fn handshake_frames_classified() {
        let hello = SignalFrame::Hello {
            protocol_version: PROTOCOL_VERSION,
            client: CLIENT_NAME.to_string(),
        };
        assert!(hello.is_handshake_frame());
        assert!(!SignalFrame::LeaveAck.is_handshake_frame());
    }

    #[test]
    fn join_token_is_redacted_in_debug() {
        let frame = SignalFrame::Join {
            room: "room-1".to_string(),
            identity: "user-1".to_string(),
            token: SecretValue::new("secret-token-value-LEAK"),
        };
        let dbg = format!("{frame:?}");
        assert!(
            !dbg.contains("secret-token-value-LEAK"),
            "Debug 泄露令牌: {dbg}"
        );
        assert!(dbg.contains("[redacted]"), "Debug 应展示脱敏占位: {dbg}");
    }

    #[test]
    fn participant_info_converts_to_participant() {
        let info = ParticipantInfo {
            sid: "PA_9".to_string(),
            identity: "user-9".to_string(),
            name: Some("Nine".to_string()),
            metadata: Some("{}".to_string()),
        };
        let p = info.into_participant().expect("valid participant");
        assert_eq!(p.sid(), Some("PA_9"));
        assert_eq!(p.identity(), "user-9");
        assert_eq!(p.name(), Some("Nine"));
        assert_eq!(p.metadata(), Some("{}"));
    }

    #[test]
    fn data_chunking_and_reassembly_roundtrip() {
        let payload: Vec<u8> = (0..(MAX_DATA_CHUNK_BYTES * 2 + 5))
            .map(|i| i as u8)
            .collect();
        let frames = chunk_data_message("msg-42", &payload, true).expect("chunk");
        assert_eq!(frames.len(), 3);
        let mut assembler = DataMessageAssembler::new();
        let mut merged = None;
        for (i, frame) in frames.iter().enumerate() {
            match frame {
                SignalFrame::DataSend {
                    message_id,
                    chunk_index,
                    chunk_count,
                    payload: chunk,
                    reliable,
                } => {
                    assert_eq!(message_id, "msg-42");
                    assert_eq!(*chunk_index, i as u32);
                    assert_eq!(*chunk_count, 3);
                    assert!(*reliable);
                    let got = assembler
                        .push_chunk(message_id, *chunk_index, *chunk_count, chunk.clone())
                        .expect("push");
                    if i == 2 {
                        merged = Some(got.expect("last chunk completes message"));
                    } else {
                        assert!(got.is_none(), "intermediate chunk must not complete");
                    }
                }
                other => panic!("unexpected frame {other:?}"),
            }
        }
        assert_eq!(merged.expect("assembled"), payload);
        assert_eq!(assembler.pending_count(), 0);
    }

    #[test]
    fn data_empty_payload_still_yields_one_chunk() {
        let frames = chunk_data_message("msg-empty", &[], false).expect("chunk");
        assert_eq!(frames.len(), 1);
        let mut assembler = DataMessageAssembler::new();
        let got = assembler
            .push_chunk("msg-empty", 0, 1, Vec::new())
            .expect("push");
        assert_eq!(got, Some(Vec::new()));
    }

    #[test]
    fn data_reassembly_rejects_protocol_violations() {
        let mut assembler = DataMessageAssembler::new();
        // chunk_count = 0
        assert!(matches!(
            assembler.push_chunk("m", 0, 0, vec![1]),
            Err(LiveKitError::Protocol(_))
        ));
        // 序号越界
        assert!(matches!(
            assembler.push_chunk("m", 2, 2, vec![1]),
            Err(LiveKitError::Protocol(_))
        ));
        // 前后 chunk_count 不一致
        assembler.push_chunk("m2", 0, 3, vec![1]).expect("first");
        assert!(matches!(
            assembler.push_chunk("m2", 1, 2, vec![1]),
            Err(LiveKitError::Protocol(_))
        ));
        // 重复序号
        let mut a2 = DataMessageAssembler::new();
        a2.push_chunk("m3", 0, 2, vec![1]).expect("first");
        assert!(matches!(
            a2.push_chunk("m3", 0, 2, vec![2]),
            Err(LiveKitError::Protocol(_))
        ));
    }

    #[test]
    fn data_reassembly_rejects_oversized_chunk() {
        let mut assembler = DataMessageAssembler::new();
        let big = vec![0u8; MAX_DATA_CHUNK_BYTES + 1];
        assert!(matches!(
            assembler.push_chunk("big", 0, 1, big),
            Err(LiveKitError::Protocol(_))
        ));
    }

    #[test]
    fn data_chunking_rejects_oversized_message() {
        let huge = vec![0u8; MAX_DATA_CHUNK_BYTES * MAX_DATA_CHUNKS as usize + 1];
        assert!(matches!(
            chunk_data_message("huge", &huge, true),
            Err(LiveKitError::Protocol(_))
        ));
    }

    #[test]
    fn data_chunks_split_at_boundary() {
        let payload = vec![7u8; MAX_DATA_CHUNK_BYTES];
        let frames = chunk_data_message("edge", &payload, true).expect("chunk");
        assert_eq!(frames.len(), 1, "恰好一帧, 不得多切");
        let payload2 = vec![7u8; MAX_DATA_CHUNK_BYTES + 1];
        let frames2 = chunk_data_message("edge2", &payload2, true).expect("chunk");
        assert_eq!(frames2.len(), 2, "多 1 字节必须多切一帧");
    }
}
