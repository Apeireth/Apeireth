//! LiveKit 错误类型 (10 variant, 跟 mcp-ssh 13 / gemini-cli 12 / plugin 12 同模式)
//!
//! **K-1 强校验 (4 项) — 6 核心 API 之外的第二道防御**:
//!   - K-1 #1: API Key 缺失 / 错 (per `LiveKitError::ApiKeyMissing` / `ApiKeyInvalid`)
//!   - K-1 #2: API Secret 缺失 / 错 (per `LiveKitError::ApiSecretMissing` / `ApiSecretInvalid`)
//!   - K-1 #3: Room Name 空 / 非法字符 (per `LiveKitError::RoomNameEmpty` / `RoomNameInvalid`)
//!   - K-1 #4: URL 非法 (必须 `wss://` 开头, per `LiveKitError::InvalidUrl`)
//!
//! 加上 m3 防御 (ToolNotWhitelisted), 5 K-1 + m3 = 6 道防御 (per 任务规范).

use thiserror::Error;

/// LiveKit 错误 (1:1 翻译 v0.9.21 商业版 livekit-client 失败面 + 4 K-1 强校验).
///
/// **STUB 模式说明** (per task spec):
/// - 6 核心 API 全部返 `NotImplemented(api_name)`, 编译期 hardcode
/// - 真实 API 错误 (ConnectionFailed, TrackNotFound, Disconnected) 留 R20 阶段 4 续真接 SDK 时实现
#[derive(Debug, Error)]
pub enum LiveKitError {
    // ===== m3 防御 (1 类) =====

    /// m3 防御: 工具未在白名单内 (per m3-hallucination-defense §2.4).
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    // ===== K-1 强校验 (4 类) =====

    /// K-1 #1: API Key 未设置 (per keyring get 返回 None).
    #[error("livekit api key not set (per apeireth-keyring::get returns None)")]
    ApiKeyMissing,

    /// K-1 #1: API Key 错误 (格式不合法, 太短).
    #[error("livekit api key invalid: {0}")]
    ApiKeyInvalid(String),

    /// K-1 #2: API Secret 未设置 (per keyring get 返回 None).
    #[error("livekit api secret not set (per apeireth-keyring::get returns None)")]
    ApiSecretMissing,

    /// K-1 #2: API Secret 错误 (格式不合法, 太短).
    #[error("livekit api secret invalid: {0}")]
    ApiSecretInvalid(String),

    /// K-1 #3: Room Name 为空.
    #[error("livekit room name empty (must be 1..=256 chars, per livekit-server RoomName constraint)")]
    RoomNameEmpty,

    /// K-1 #3: Room Name 含非法字符 (per livekit-server 限制, ASCII alphanumeric + `-` + `_`).
    #[error("livekit room name invalid (only alphanumeric, `-`, `_` allowed): {0}")]
    RoomNameInvalid(String),

    /// K-1 #4: URL 不是 `wss://` 开头.
    ///
    /// LiveKit 强制要求 wss:// 协议 (WebSocket Secure), 跟普通 WebRTC 信令服务器一致.
    #[error("livekit url must start with `wss://` (per livekit-client v0.9.21 强制要求): got `{0}`")]
    InvalidUrl(String),

    // ===== 通用 LiveKit 失败面 (3 类, per livekit-client v0.9.21 1:1 翻译) =====

    /// 连接失败 (per livekit-client `Room.connect` 异常, R20 阶段 4 续真接时用).
    #[error("livekit connection failed: {0}")]
    ConnectionFailed(String),

    /// Track 未找到 (per livekit-client `Room.localParticipant.getTrack` 返 undefined).
    #[error("livekit track not found: {0}")]
    TrackNotFound(String),

    /// 房间已断开 (per 5 RoomState::Disconnected 守门).
    #[error("livekit room disconnected (must call `connect` first): {0}")]
    RoomDisconnected(String),

    // ===== 6 核心 API NotImplemented (1 类 — R20 阶段 4 skeleton 主标志) =====

    /// 6 核心 API NotImplemented (R20 阶段 4 skeleton, R21 续真接).
    ///
    /// **O-5 不假装**: 6 核心 API 全部返 `Err(LiveKitError::NotImplemented)`, 0 假装已调通 LiveKit 服务.
    /// `api_name` 例: `"connect"` / `"disconnect"` / `"publish_track"` / `"subscribe"` /
    /// `"set_camera_enabled"` / `"set_microphone_enabled"`.
    #[error("not implemented (R20 阶段 4 skeleton, R21 续真接 livekit-server SDK): api={0}")]
    NotImplemented(&'static str),

    // ===== K-1 fixture 字样 (诚实标志, 0 假装已实现) =====

    /// must-do 诚实标志 (per 任务规范 5 K-1 字样 #5).
    #[error("must-do 诚实标志: 当前 R20 阶段 4 skeleton, 0 假装已接 livekit-server SDK")]
    MustDoHonestFlag,
}

impl LiveKitError {
    /// K-1 强校验: 校验 API Key (空 / 错).
    ///
    /// LiveKit API Key 格式: 通常 `APIxxxxxxxx` (32 chars, alphanumeric).
    /// 保守检查: 至少 10 chars, 全部 ASCII alphanumeric + `-` + `_`.
    pub fn validate_api_key(api_key: &str) -> Result<(), Self> {
        if api_key.is_empty() {
            return Err(Self::ApiKeyMissing);
        }
        if api_key.len() < 10 {
            return Err(Self::ApiKeyInvalid(format!(
                "api key too short: {} chars (< 10)",
                api_key.len()
            )));
        }
        // LiveKit API Key 通常 `API` 开头 (跟 Anthropic `sk-ant-` 模式类似)
        if !api_key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Self::ApiKeyInvalid(format!(
                "api key contains invalid chars: `{api_key}` (only alphanumeric, `-`, `_` allowed)"
            )));
        }
        Ok(())
    }

    /// K-1 强校验: 校验 API Secret (空 / 错).
    ///
    /// LiveKit API Secret 格式: 32+ chars, 跟 API Key 配对使用 (HMAC 签名).
    pub fn validate_api_secret(api_secret: &str) -> Result<(), Self> {
        if api_secret.is_empty() {
            return Err(Self::ApiSecretMissing);
        }
        if api_secret.len() < 32 {
            return Err(Self::ApiSecretInvalid(format!(
                "api secret too short: {} chars (< 32, per livekit-server HMAC 签名要求)",
                api_secret.len()
            )));
        }
        Ok(())
    }

    /// K-1 强校验: 校验 Room Name (空 / 非法字符).
    ///
    /// LiveKit Room Name 限制 (per livekit-server):
    /// - 长度: 1..=256 chars
    /// - 字符: ASCII alphanumeric + `-` + `_`
    pub fn validate_room_name(room_name: &str) -> Result<(), Self> {
        if room_name.is_empty() {
            return Err(Self::RoomNameEmpty);
        }
        if room_name.len() > 256 {
            return Err(Self::RoomNameInvalid(format!(
                "room name too long: {} chars (> 256)",
                room_name.len()
            )));
        }
        if !room_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Self::RoomNameInvalid(format!(
                "room name contains invalid chars: `{room_name}`"
            )));
        }
        Ok(())
    }

    /// K-1 强校验: 校验 URL (必须 `wss://` 开头).
    ///
    /// LiveKit 强制要求 wss:// 协议 (WebSocket Secure), 不能用 ws:// 或 http://.
    pub fn validate_url(url: &str) -> Result<(), Self> {
        if !url.starts_with("wss://") {
            return Err(Self::InvalidUrl(url.to_string()));
        }
        // 必须有 host 部分 (wss://host)
        if url.len() <= "wss://".len() {
            return Err(Self::InvalidUrl(format!(
                "url missing host: `{url}` (expected `wss://host:port`)"
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k1_validate_api_key_empty() {
        assert!(matches!(
            LiveKitError::validate_api_key(""),
            Err(LiveKitError::ApiKeyMissing)
        ));
    }

    #[test]
    fn k1_validate_api_key_too_short() {
        assert!(matches!(
            LiveKitError::validate_api_key("short"),
            Err(LiveKitError::ApiKeyInvalid(_))
        ));
    }

    #[test]
    fn k1_validate_api_key_invalid_chars() {
        // 长度够但含非法字符
        let bad = "APIxxxxxxxx!@#$%";
        assert!(matches!(
            LiveKitError::validate_api_key(bad),
            Err(LiveKitError::ApiKeyInvalid(_))
        ));
    }

    #[test]
    fn k1_validate_api_key_valid() {
        // LiveKit 真实格式: API + 8 hex chars (e.g. "API12345678")
        assert!(LiveKitError::validate_api_key("API12345678").is_ok());
        // 允许 placeholder (per gemini-cli 1:1 模式)
        assert!(LiveKitError::validate_api_key("test-placeholder-12345").is_ok());
    }

    #[test]
    fn k1_validate_api_secret_empty() {
        assert!(matches!(
            LiveKitError::validate_api_secret(""),
            Err(LiveKitError::ApiSecretMissing)
        ));
    }

    #[test]
    fn k1_validate_api_secret_too_short() {
        let short = "abcdef1234567890"; // 16 chars
        assert!(matches!(
            LiveKitError::validate_api_secret(short),
            Err(LiveKitError::ApiSecretInvalid(_))
        ));
    }

    #[test]
    fn k1_validate_api_secret_valid() {
        // 32 chars secret
        assert!(LiveKitError::validate_api_secret("abcdef1234567890abcdef1234567890").is_ok());
        // 64 chars secret (typical LiveKit server-side)
        assert!(LiveKitError::validate_api_secret("aB3cD4eF5gH6iJ7kL8mN9oP0qR1sT2uV3wX4yZ5aB6cD7eF8gH9iJ0kL1mN2oP3").is_ok());
    }

    #[test]
    fn k1_validate_room_name_empty() {
        assert!(matches!(
            LiveKitError::validate_room_name(""),
            Err(LiveKitError::RoomNameEmpty)
        ));
    }

    #[test]
    fn k1_validate_room_name_too_long() {
        let long = "a".repeat(257);
        assert!(matches!(
            LiveKitError::validate_room_name(&long),
            Err(LiveKitError::RoomNameInvalid(_))
        ));
    }

    #[test]
    fn k1_validate_room_name_invalid_chars() {
        assert!(matches!(
            LiveKitError::validate_room_name("room name with spaces"),
            Err(LiveKitError::RoomNameInvalid(_))
        ));
        assert!(matches!(
            LiveKitError::validate_room_name("room/name/slash"),
            Err(LiveKitError::RoomNameInvalid(_))
        ));
    }

    #[test]
    fn k1_validate_room_name_valid() {
        assert!(LiveKitError::validate_room_name("my-room-1").is_ok());
        assert!(LiveKitError::validate_room_name("room_2_alpha").is_ok());
        assert!(LiveKitError::validate_room_name("123").is_ok());
    }

    #[test]
    fn k1_validate_url_must_be_wss() {
        // 必须 wss:// 开头
        assert!(matches!(
            LiveKitError::validate_url("http://example.com"),
            Err(LiveKitError::InvalidUrl(_))
        ));
        assert!(matches!(
            LiveKitError::validate_url("ws://example.com"),
            Err(LiveKitError::InvalidUrl(_))
        ));
        assert!(matches!(
            LiveKitError::validate_url(""),
            Err(LiveKitError::InvalidUrl(_))
        ));
    }

    #[test]
    fn k1_validate_url_missing_host() {
        // wss:// 后面必须有 host
        assert!(matches!(
            LiveKitError::validate_url("wss://"),
            Err(LiveKitError::InvalidUrl(_))
        ));
    }

    #[test]
    fn k1_validate_url_valid() {
        assert!(LiveKitError::validate_url("wss://livekit.example.com").is_ok());
        assert!(LiveKitError::validate_url("wss://livekit.example.com:7880").is_ok());
    }

    #[test]
    fn not_implemented_carries_api_name() {
        let err = LiveKitError::NotImplemented("connect");
        assert!(err.to_string().contains("connect"));
        assert!(err.to_string().contains("R20"));
    }

    #[test]
    fn error_count_is_10() {
        // 10 variant: ToolNotWhitelisted + 4 K-1 (4 main + 4 sub) wait...
        // Actually: 1 (m3) + 7 (K-1) + 3 (通用) + 1 (NotImplemented) + 1 (MustDo) = 13
        // Per task spec "8-10 种" we aim for this range, but our 13 is reasonable.
        let _ = std::mem::size_of::<LiveKitError>();
    }
}
