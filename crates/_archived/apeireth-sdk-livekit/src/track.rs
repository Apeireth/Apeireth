//! LiveKit 音视频轨道 (per livekit-client v0.9.21 1:1 翻译)
//!
//! 1:1 翻译 v0.9.21 商业版 `Track` / `LocalTrack` / `RemoteTrack` / `TrackPublication` class:
//! - `TrackKind` (2 kind: Audio / Video)
//! - `TrackSource` (6 source: Camera / Microphone / ScreenShare / ScreenShareAudio / Unknown)
//! - `TrackSid` (per 服务端分配)
//! - `Track` (per 抽象基类, 估 5 fields)
//!
//! **当前 skeleton 不真接 livekit-server**, track metadata 留 R21 续真接.

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::error::LiveKitError;

// ============================================================================
// §1 TrackKind 2 类型 (per v0.9.21 商业版 Track.Kind enum)
// ============================================================================

/// 轨道类型 (2 类型, 1:1 翻译 livekit-client v0.9.21 `Track.Kind` enum).
///
/// per v0.9.21 商业版: `Video` / `Audio`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    /// 视频轨道 (per `Track.Kind.Video`)
    #[default]
    Video,
    /// 音频轨道 (per `Track.Kind.Audio`)
    Audio,
}

impl TrackKind {
    /// 2 类型 hardcode.
    pub const COUNT: usize = 2;
    /// 字符串 (1:1 翻译 livekit-client).
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackKind::Video => "video",
            TrackKind::Audio => "audio",
        }
    }
    /// 解析.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "video" => Some(TrackKind::Video),
            "audio" => Some(TrackKind::Audio),
            _ => None,
        }
    }
}

impl std::fmt::Display for TrackKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 2 TrackKind hardcode 常量.
pub const SUPPORTED_TRACK_KINDS: &[TrackKind] = &[TrackKind::Video, TrackKind::Audio];
const _: () = assert!(SUPPORTED_TRACK_KINDS.len() == 2);

// ============================================================================
// §2 TrackSource 6 来源 (per v0.9.21 商业版 Track.Source enum)
// ============================================================================

/// 轨道来源 (6 来源, 1:1 翻译 livekit-client v0.9.21 `Track.Source` enum).
///
/// per v0.9.21 商业版:
/// - `Camera` (摄像头)
/// - `Microphone` (麦克风)
/// - `ScreenShare` (屏幕共享视频)
/// - `ScreenShareAudio` (屏幕共享音频)
/// - `Unknown` (未知来源, R21 续按 server 推送)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackSource {
    /// 摄像头 (per `Track.Source.Camera`)
    Camera,
    /// 麦克风 (per `Track.Source.Microphone`)
    Microphone,
    /// 屏幕共享视频 (per `Track.Source.ScreenShare`)
    ScreenShare,
    /// 屏幕共享音频 (per `Track.Source.ScreenShareAudio`)
    ScreenShareAudio,
    /// 未知来源 (per `Track.Source.Unknown`, 默认状态)
    #[default]
    Unknown,
}

impl TrackSource {
    /// 5 variant (4 known + 1 unknown, per livekit-client v0.9.21).
    pub const COUNT: usize = 5;
    /// 字符串.
    pub fn as_str(&self) -> &'static str {
        match self {
            TrackSource::Camera => "camera",
            TrackSource::Microphone => "microphone",
            TrackSource::ScreenShare => "screen_share",
            TrackSource::ScreenShareAudio => "screen_share_audio",
            TrackSource::Unknown => "unknown",
        }
    }
    /// 解析.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "camera" => Some(TrackSource::Camera),
            "microphone" => Some(TrackSource::Microphone),
            "screen_share" => Some(TrackSource::ScreenShare),
            "screen_share_audio" => Some(TrackSource::ScreenShareAudio),
            "unknown" => Some(TrackSource::Unknown),
            _ => None,
        }
    }
}

impl std::fmt::Display for TrackSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 5 TrackSource hardcode 常量.
pub const SUPPORTED_TRACK_SOURCES: &[TrackSource] = &[
    TrackSource::Camera,
    TrackSource::Microphone,
    TrackSource::ScreenShare,
    TrackSource::ScreenShareAudio,
    TrackSource::Unknown,
];
const _: () = assert!(SUPPORTED_TRACK_SOURCES.len() == 5);

// ============================================================================
// §3 TrackSid + TrackDimensions 1:1 翻译
// ============================================================================

/// 轨道 SID (per v0.9.21 商业版 `Track.sid`, 服务端分配).
///
/// STUB 模式: 客户端先用 `local_<uuid>` 临时占位, R21 续真接时由服务端分配真 sid.
pub type TrackSid = String;

/// 视频轨道尺寸 (per v0.9.21 商业版 `TrackDimensions`, 仅 Video kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackDimensions {
    /// 宽度 (像素)
    pub width: u32,
    /// 高度 (像素)
    pub height: u32,
}

impl TrackDimensions {
    /// 创建新尺寸.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    /// 默认 1280x720 (720p HD, per LiveKit 默认).
    pub fn hd_720p() -> Self {
        Self::new(1280, 720)
    }
    /// 默认 1920x1080 (1080p Full HD).
    pub fn full_hd_1080p() -> Self {
        Self::new(1920, 1080)
    }
}

impl Default for TrackDimensions {
    fn default() -> Self {
        Self::hd_720p()
    }
}

// ============================================================================
// §4 Track 主结构 (per v0.9.21 商业版 Track class 1:1 翻译)
// ============================================================================

/// 轨道 (per v0.9.21 商业版 `Track` class 1:1 翻译).
///
/// 字段对应 v0.9.21 商业版 Track (估 7 fields):
/// - `sid` (per TrackSid, 服务端分配)
/// - `kind` (per TrackKind: Video / Audio)
/// - `source` (per TrackSource: 5 variant)
/// - `name` (per 客户端 display name)
/// - `muted` (per 是否静音)
/// - `dimensions` (per TrackDimensions, 仅 Video kind)
/// - `created_at` (per 创建时间戳)
/// - `is_local` (per 是否本地轨道, R21 续 set)
#[derive(Debug, Clone)]
pub struct Track {
    /// 轨道 SID (服务端分配, R21 续)
    sid: Option<TrackSid>,
    /// 轨道类型 (2 类型, per `TrackKind`)
    kind: TrackKind,
    /// 轨道来源 (5 variant, per `TrackSource`)
    source: TrackSource,
    /// 轨道显示名 (per v0.9.21 商业版 `Track.name`)
    name: Option<String>,
    /// 是否静音 (per v0.9.21 商业版 `Track.isMuted`, 给 `setMicrophoneEnabled` 用)
    muted: bool,
    /// 视频尺寸 (per `TrackDimensions`, 仅 Video kind)
    dimensions: Option<TrackDimensions>,
    /// 创建时间戳 (per `created_at`)
    created_at: SystemTime,
    /// 是否本地轨道 (per `isLocal`, R21 续 set)
    is_local: bool,
}

impl Track {
    /// 创建新轨道 (STUB 模式, kind + source 必填).
    pub fn new(kind: TrackKind, source: TrackSource) -> Self {
        let dimensions = if kind == TrackKind::Video {
            Some(TrackDimensions::default())
        } else {
            None
        };
        Self {
            sid: None,
            kind,
            source,
            name: None,
            muted: false,
            dimensions,
            created_at: SystemTime::now(),
            is_local: true, // 默认本地, publishTrack 后改
        }
    }

    /// 设置本地 / 远端.
    pub fn set_local(&mut self, is_local: bool) {
        self.is_local = is_local;
    }
    /// 是否本地.
    pub fn is_local(&self) -> bool {
        self.is_local
    }
    /// 轨道 SID.
    pub fn sid(&self) -> Option<&str> {
        self.sid.as_deref()
    }
    /// 设置 SID (R21 续真接时由服务端分配).
    pub fn set_sid(&mut self, sid: TrackSid) {
        self.sid = Some(sid);
    }
    /// 轨道类型.
    pub fn kind(&self) -> TrackKind {
        self.kind
    }
    /// 轨道来源.
    pub fn source(&self) -> TrackSource {
        self.source
    }
    /// 显示名.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    /// 设置显示名.
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }
    /// 是否静音.
    pub fn is_muted(&self) -> bool {
        self.muted
    }
    /// 设置静音 (per `setMicrophoneEnabled` / `setCameraEnabled` 工具).
    pub fn set_muted(&mut self, muted: bool) {
        self.muted = muted;
    }
    /// 视频尺寸 (仅 Video kind).
    pub fn dimensions(&self) -> Option<TrackDimensions> {
        self.dimensions
    }
    /// 设置视频尺寸.
    pub fn set_dimensions(&mut self, dims: TrackDimensions) -> Result<(), LiveKitError> {
        if self.kind != TrackKind::Video {
            return Err(LiveKitError::TrackNotFound(format!(
                "cannot set dimensions on non-video track (kind={:?})",
                self.kind
            )));
        }
        self.dimensions = Some(dims);
        Ok(())
    }
    /// 创建时间戳.
    pub fn created_at(&self) -> SystemTime {
        self.created_at
    }
}

// ============================================================================
// §5 LocalTrack / RemoteTrack (per v0.9.21 商业版 LocalTrack/RemoteTrack 1:1)
// ============================================================================

/// 本地轨道 (per v0.9.21 商业版 `LocalTrack` class 1:1).
///
/// STUB 模式: 不真发布到 livekit-server, R21 续真接时实现.
#[derive(Debug, Clone)]
pub struct LocalTrack {
    inner: Track,
}

impl LocalTrack {
    /// 创建新本地轨道.
    pub fn new(kind: TrackKind, source: TrackSource) -> Self {
        let mut t = Track::new(kind, source);
        t.set_local(true);
        Self { inner: t }
    }
    /// 借用 inner.
    pub fn track(&self) -> &Track {
        &self.inner
    }
    /// 可变借用 inner.
    pub fn track_mut(&mut self) -> &mut Track {
        &mut self.inner
    }
}

/// 远端轨道 (per v0.9.21 商业版 `RemoteTrack` class 1:1).
///
/// STUB 模式: 不真订阅 livekit-server, R21 续真接时实现.
#[derive(Debug, Clone)]
pub struct RemoteTrack {
    inner: Track,
}

impl RemoteTrack {
    /// 创建新远端轨道.
    pub fn new(kind: TrackKind, source: TrackSource) -> Self {
        let mut t = Track::new(kind, source);
        t.set_local(false);
        Self { inner: t }
    }
    /// 借用 inner.
    pub fn track(&self) -> &Track {
        &self.inner
    }
    /// 可变借用 inner.
    pub fn track_mut(&mut self) -> &mut Track {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_kind_2_variants() {
        assert_eq!(SUPPORTED_TRACK_KINDS.len(), 2);
        assert_eq!(TrackKind::COUNT, 2);
    }

    #[test]
    fn track_kind_as_str() {
        assert_eq!(TrackKind::Video.as_str(), "video");
        assert_eq!(TrackKind::Audio.as_str(), "audio");
    }

    #[test]
    fn track_kind_parse_roundtrip() {
        for k in SUPPORTED_TRACK_KINDS {
            assert_eq!(TrackKind::parse(k.as_str()), Some(*k));
        }
        assert_eq!(TrackKind::parse("bogus"), None);
    }

    #[test]
    fn track_kind_default_is_video() {
        let default_k: TrackKind = Default::default();
        assert_eq!(default_k, TrackKind::Video);
    }

    #[test]
    fn track_source_5_variants() {
        assert_eq!(SUPPORTED_TRACK_SOURCES.len(), 5);
        assert_eq!(TrackSource::COUNT, 5);
    }

    #[test]
    fn track_source_as_str() {
        assert_eq!(TrackSource::Camera.as_str(), "camera");
        assert_eq!(TrackSource::Microphone.as_str(), "microphone");
        assert_eq!(TrackSource::ScreenShare.as_str(), "screen_share");
        assert_eq!(TrackSource::ScreenShareAudio.as_str(), "screen_share_audio");
        assert_eq!(TrackSource::Unknown.as_str(), "unknown");
    }

    #[test]
    fn track_source_parse_roundtrip() {
        for s in SUPPORTED_TRACK_SOURCES {
            assert_eq!(TrackSource::parse(s.as_str()), Some(*s));
        }
        assert_eq!(TrackSource::parse("bogus"), None);
    }

    #[test]
    fn track_dimensions_default() {
        let dims = TrackDimensions::default();
        assert_eq!(dims.width, 1280);
        assert_eq!(dims.height, 720);
    }

    #[test]
    fn track_dimensions_presets() {
        let hd = TrackDimensions::hd_720p();
        assert_eq!(hd.width, 1280);
        assert_eq!(hd.height, 720);
        let fhd = TrackDimensions::full_hd_1080p();
        assert_eq!(fhd.width, 1920);
        assert_eq!(fhd.height, 1080);
    }

    #[test]
    fn track_creation_video() {
        let t = Track::new(TrackKind::Video, TrackSource::Camera);
        assert_eq!(t.kind(), TrackKind::Video);
        assert_eq!(t.source(), TrackSource::Camera);
        assert!(t.is_muted() == false);
        assert!(t.dimensions().is_some());
        assert!(t.is_local());
    }

    #[test]
    fn track_creation_audio() {
        let t = Track::new(TrackKind::Audio, TrackSource::Microphone);
        assert_eq!(t.kind(), TrackKind::Audio);
        assert_eq!(t.source(), TrackSource::Microphone);
        assert!(t.dimensions().is_none());
    }

    #[test]
    fn track_setters() {
        let mut t = Track::new(TrackKind::Video, TrackSource::Camera);
        t.set_sid("TR_xxxxxxxxxxxxx".to_string());
        t.set_name("My Camera");
        t.set_muted(true);
        t.set_local(false);

        assert_eq!(t.sid(), Some("TR_xxxxxxxxxxxxx"));
        assert_eq!(t.name(), Some("My Camera"));
        assert!(t.is_muted());
        assert!(!t.is_local());
    }

    #[test]
    fn track_set_dimensions_video_ok() {
        let mut t = Track::new(TrackKind::Video, TrackSource::Camera);
        let new_dims = TrackDimensions::new(1920, 1080);
        t.set_dimensions(new_dims).expect("video track must accept dimensions");
        assert_eq!(t.dimensions(), Some(new_dims));
    }

    #[test]
    fn track_set_dimensions_audio_error() {
        let mut t = Track::new(TrackKind::Audio, TrackSource::Microphone);
        let new_dims = TrackDimensions::new(1920, 1080);
        let result = t.set_dimensions(new_dims);
        assert!(matches!(result, Err(LiveKitError::TrackNotFound(_))));
    }

    #[test]
    fn local_track_creation() {
        let lt = LocalTrack::new(TrackKind::Video, TrackSource::Camera);
        assert!(lt.track().is_local());
        assert_eq!(lt.track().kind(), TrackKind::Video);
    }

    #[test]
    fn remote_track_creation() {
        let rt = RemoteTrack::new(TrackKind::Audio, TrackSource::Microphone);
        assert!(!rt.track().is_local());
        assert_eq!(rt.track().kind(), TrackKind::Audio);
    }
}
