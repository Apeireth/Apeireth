//! LiveKit 参与者 (per livekit-client v0.9.21 1:1 翻译)
//!
//! 1:1 翻译 v0.9.21 商业版 `Participant` class (per livekit-client/dist/src/room/Participant.d.ts):
//! - `Participant` (per 参与者身份 + metadata + permissions)
//! - `ConnectionQuality` (per 连接质量 4 等级)
//! - `Permission` (per 5 权限位)
//!
//! **当前 skeleton 不真接 livekit-server**, participant metadata / tracks 留 R21 续真接.

use std::collections::HashMap;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::LiveKitError;
use crate::track::{TrackSid, TrackSource};

// ============================================================================
// §1 ConnectionQuality 4 等级 (per v0.9.21 商业版 ConnectionQuality enum)
// ============================================================================

/// 连接质量 (4 等级, 1:1 翻译 livekit-client v0.9.21 `ConnectionQuality` enum).
///
/// 字段对应 livekit-client v0.9.21 ConnectionQuality:
/// `Excellent` / `Good` / `Poor` / `Lost`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionQuality {
    /// 优秀 (< 50ms RTT, 0 丢包)
    Excellent,
    /// 良好 (50~150ms RTT, < 2% 丢包)
    Good,
    /// 较差 (150~300ms RTT, 2~10% 丢包)
    Poor,
    /// 丢失 (> 300ms RTT 或 > 10% 丢包)
    Lost,
    /// 未知 (尚未评估, 默认状态)
    #[default]
    Unknown,
}

impl ConnectionQuality {
    /// 4 等级 + 1 unknown = 5 variant (per livekit-client v0.9.21 实际 5 variant).
    pub const COUNT: usize = 5;
    /// 字符串 (1:1 翻译 livekit-client).
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionQuality::Excellent => "excellent",
            ConnectionQuality::Good => "good",
            ConnectionQuality::Poor => "poor",
            ConnectionQuality::Lost => "lost",
            ConnectionQuality::Unknown => "unknown",
        }
    }
    /// 解析字符串.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "excellent" => Some(ConnectionQuality::Excellent),
            "good" => Some(ConnectionQuality::Good),
            "poor" => Some(ConnectionQuality::Poor),
            "lost" => Some(ConnectionQuality::Lost),
            "unknown" => Some(ConnectionQuality::Unknown),
            _ => None,
        }
    }
}

impl std::fmt::Display for ConnectionQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 4 + 1 等级 hardcode 常量 (K-1 强校验守门).
pub const SUPPORTED_CONNECTION_QUALITIES: &[ConnectionQuality] = &[
    ConnectionQuality::Excellent,
    ConnectionQuality::Good,
    ConnectionQuality::Poor,
    ConnectionQuality::Lost,
    ConnectionQuality::Unknown,
];
const _: () = assert!(SUPPORTED_CONNECTION_QUALITIES.len() == 5);

// ============================================================================
// §2 Permission 5 权限位 (per v0.9.21 商业版 Permission enum 1:1)
// ============================================================================

/// 权限位 (5 权限, 1:1 翻译 livekit-client v0.9.21 `Permission` enum).
///
/// per v0.9.21 商业版:
/// - `CanPublish` (允许发布 tracks)
/// - `CanSubscribe` (允许订阅 tracks)
/// - `CanPublishData` (允许发布 data messages)
/// - `CanUpdateMetadata` (允许更新 metadata)
/// - `Hidden` (隐藏参与者, server-side 分配)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// 允许发布 tracks
    CanPublish,
    /// 允许订阅 tracks
    CanSubscribe,
    /// 允许发布 data messages (per `RoomEvent::DataReceived`)
    CanPublishData,
    /// 允许更新 metadata
    CanUpdateMetadata,
    /// 隐藏参与者 (server-side 分配)
    Hidden,
}

impl Permission {
    /// 5 权限 hardcode.
    pub const COUNT: usize = 5;
    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::CanPublish => "can_publish",
            Permission::CanSubscribe => "can_subscribe",
            Permission::CanPublishData => "can_publish_data",
            Permission::CanUpdateMetadata => "can_update_metadata",
            Permission::Hidden => "hidden",
        }
    }
    /// 解析.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "can_publish" => Some(Permission::CanPublish),
            "can_subscribe" => Some(Permission::CanSubscribe),
            "can_publish_data" => Some(Permission::CanPublishData),
            "can_update_metadata" => Some(Permission::CanUpdateMetadata),
            "hidden" => Some(Permission::Hidden),
            _ => None,
        }
    }
}

/// 5 权限 hardcode 常量.
pub const SUPPORTED_PERMISSIONS: &[Permission] = &[
    Permission::CanPublish,
    Permission::CanSubscribe,
    Permission::CanPublishData,
    Permission::CanUpdateMetadata,
    Permission::Hidden,
];
const _: () = assert!(SUPPORTED_PERMISSIONS.len() == 5);

// ============================================================================
// §3 Participant 主结构 (per v0.9.21 商业版 Participant class 1:1)
// ============================================================================

/// 参与者 SID (per v0.9.21 商业版 `Participant.sid`, 服务端分配).
///
/// STUB 模式: 客户端先用 `identity` 临时占位, R21 续真接时由服务端分配真 sid.
pub type ParticipantSid = String;

/// 参与者 (per v0.9.21 商业版 `Participant` class 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 Participant (估 8 fields):
/// - `sid` (per ParticipantSid, 服务端分配)
/// - `identity` (per 客户端 identity 字符串)
/// - `name` (per 显示名, 可选)
/// - `metadata` (per 自定义 metadata, 可选)
/// - `is_speaking` (per 当前是否在说话, active speakers 用)
/// - `connection_quality` (per ConnectionQuality 5 等级)
/// - `permissions` (per Vec<Permission> 5 权限位)
/// - `joined_at` (per 加入时间戳)
/// - `track_publications` (per HashMap<TrackSid, TrackPublication>, R21 续)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Participant {
    /// 参与者 SID (服务端分配, R21 续真接后才有)
    sid: Option<ParticipantSid>,
    /// 参与者 identity (per v0.9.21 商业版 `Participant.identity`)
    identity: String,
    /// 显示名 (per v0.9.21 商业版 `Participant.name`, 可选)
    name: Option<String>,
    /// 自定义 metadata (per v0.9.21 商业版 `Participant.metadata`, 可选)
    metadata: Option<String>,
    /// 是否在说话 (per v0.9.21 商业版 `Participant.isSpeaking`, 给 `ActiveSpeakersChanged` 用)
    is_speaking: bool,
    /// 连接质量 (4 + 1 等级, per `ConnectionQuality`)
    connection_quality: ConnectionQuality,
    /// 权限列表 (5 权限位, per `Permission`)
    permissions: Vec<Permission>,
    /// 加入时间戳 (SystemTime, per `joined_at`)
    joined_at: SystemTime,
    /// 已发布 track 表 (per `track_publications`, R21 续真接才填, STUB 模式空)
    track_publications: HashMap<TrackSid, TrackSource>,
}

impl Participant {
    /// 创建新参与者 (STUB 模式, identity 必填).
    pub fn new(identity: impl Into<String>) -> Result<Self, LiveKitError> {
        let id = identity.into();
        if id.is_empty() {
            return Err(LiveKitError::RoomNameEmpty); // 复用空值错误
        }
        Ok(Self {
            sid: None,
            identity: id,
            name: None,
            metadata: None,
            is_speaking: false,
            connection_quality: ConnectionQuality::Unknown,
            permissions: vec![Permission::CanSubscribe, Permission::CanPublishData],
            joined_at: SystemTime::now(),
            track_publications: HashMap::new(),
        })
    }

    /// 参与者 SID.
    pub fn sid(&self) -> Option<&str> {
        self.sid.as_deref()
    }
    /// 设置 SID (R21 续真接时由服务端分配).
    pub fn set_sid(&mut self, sid: ParticipantSid) {
        self.sid = Some(sid);
    }
    /// 参与者 identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }
    /// 显示名.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// 设置显示名.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }
    /// metadata.
    pub fn metadata(&self) -> Option<&str> {
        self.metadata.as_deref()
    }
    /// 设置 metadata.
    pub fn set_metadata(&mut self, metadata: impl Into<String>) {
        self.metadata = Some(metadata.into());
    }
    /// 是否在说话.
    pub fn is_speaking(&self) -> bool {
        self.is_speaking
    }
    /// 设置 is_speaking (per `ActiveSpeakersChanged` 事件用).
    pub fn set_speaking(&mut self, speaking: bool) {
        self.is_speaking = speaking;
    }
    /// 连接质量.
    pub fn connection_quality(&self) -> ConnectionQuality {
        self.connection_quality
    }
    /// 设置连接质量 (per `ConnectionStateChanged` 事件用).
    pub fn set_connection_quality(&mut self, q: ConnectionQuality) {
        self.connection_quality = q;
    }
    /// 权限列表.
    pub fn permissions(&self) -> &[Permission] {
        &self.permissions
    }
    /// 是否有指定权限.
    pub fn has_permission(&self, perm: Permission) -> bool {
        self.permissions.contains(&perm)
    }
    /// 加入时间戳.
    pub fn joined_at(&self) -> SystemTime {
        self.joined_at
    }
    /// 已发布 track 数.
    pub fn track_count(&self) -> usize {
        self.track_publications.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_quality_5_variants() {
        assert_eq!(SUPPORTED_CONNECTION_QUALITIES.len(), 5);
        assert_eq!(ConnectionQuality::COUNT, 5);
    }

    #[test]
    fn connection_quality_as_str() {
        assert_eq!(ConnectionQuality::Excellent.as_str(), "excellent");
        assert_eq!(ConnectionQuality::Good.as_str(), "good");
        assert_eq!(ConnectionQuality::Poor.as_str(), "poor");
        assert_eq!(ConnectionQuality::Lost.as_str(), "lost");
        assert_eq!(ConnectionQuality::Unknown.as_str(), "unknown");
    }

    #[test]
    fn connection_quality_parse_roundtrip() {
        for q in SUPPORTED_CONNECTION_QUALITIES {
            assert_eq!(ConnectionQuality::parse(q.as_str()), Some(*q));
        }
        assert_eq!(ConnectionQuality::parse("unknown"), Some(ConnectionQuality::Unknown));
        assert_eq!(ConnectionQuality::parse("bogus"), None);
    }

    #[test]
    fn connection_quality_default_is_unknown() {
        let default_q: ConnectionQuality = Default::default();
        assert_eq!(default_q, ConnectionQuality::Unknown);
    }

    #[test]
    fn permission_5_variants() {
        assert_eq!(SUPPORTED_PERMISSIONS.len(), 5);
        assert_eq!(Permission::COUNT, 5);
    }

    #[test]
    fn permission_as_str() {
        assert_eq!(Permission::CanPublish.as_str(), "can_publish");
        assert_eq!(Permission::CanSubscribe.as_str(), "can_subscribe");
        assert_eq!(Permission::CanPublishData.as_str(), "can_publish_data");
        assert_eq!(Permission::CanUpdateMetadata.as_str(), "can_update_metadata");
        assert_eq!(Permission::Hidden.as_str(), "hidden");
    }

    #[test]
    fn permission_parse_roundtrip() {
        for p in SUPPORTED_PERMISSIONS {
            assert_eq!(Permission::parse(p.as_str()), Some(*p));
        }
        assert_eq!(Permission::parse("bogus"), None);
    }

    #[test]
    fn participant_creation_valid() {
        let p = Participant::new("user-1").expect("valid identity must succeed");
        assert_eq!(p.identity(), "user-1");
        assert!(p.sid().is_none());
        assert_eq!(p.connection_quality(), ConnectionQuality::Unknown);
        assert!(!p.is_speaking());
        assert_eq!(p.track_count(), 0);
    }

    #[test]
    fn participant_creation_empty_identity() {
        let result = Participant::new("");
        assert!(matches!(result, Err(LiveKitError::RoomNameEmpty)));
    }

    #[test]
    fn participant_setters() {
        let mut p = Participant::new("user-1").unwrap();
        p.set_sid("PA_xxxxxxxxxxxxx".to_string());
        p.set_name("Alice");
        p.set_metadata("{\"role\":\"admin\"}");
        p.set_speaking(true);
        p.set_connection_quality(ConnectionQuality::Good);

        assert_eq!(p.sid(), Some("PA_xxxxxxxxxxxxx"));
        assert_eq!(p.name(), Some("Alice"));
        assert_eq!(p.metadata(), Some("{\"role\":\"admin\"}"));
        assert!(p.is_speaking());
        assert_eq!(p.connection_quality(), ConnectionQuality::Good);
    }

    #[test]
    fn participant_default_permissions() {
        let p = Participant::new("user-1").unwrap();
        // 默认 CanSubscribe + CanPublishData (STUB 模式最小权限, R21 续按 JWT grant)
        assert!(p.has_permission(Permission::CanSubscribe));
        assert!(p.has_permission(Permission::CanPublishData));
        assert!(!p.has_permission(Permission::CanPublish));
        assert!(!p.has_permission(Permission::Hidden));
    }

    #[test]
    fn participant_joined_at() {
        let p = Participant::new("user-1").unwrap();
        let now = SystemTime::now();
        let joined = p.joined_at();
        // joined_at 应在 now 之前或近似 now
        assert!(joined <= now);
    }
}
