//! 信令协议族错误类型: 强类型变体 + 闭合词表归类.
//!
//! 设计纪律:
//! - 每个错误变体都归入 [`crate::error_taxonomy::ErrorCategory`] 闭合词表之一
//!   ([`LiveKitError::category`]), 调用方据此做重试 / 降级决策, 不解析文案。
//! - 凭证、令牌永不进错误文案 (日志脱敏红线): 校验失败只报规则与长度。
//! - 超时统一走 `apeireth_core::deadline` 熔合面, 由 [`LiveKitError::from_deadline`]
//!   归类; 帧编解码失败由 [`frame_codec::CodecError`] 归入协议违例。
//!
//! K-1 强校验 (4 项) 仍是第一道防御:
//! - K-1 #1: API Key 格式 (缺失 / 非法)
//! - K-1 #2: API Secret 格式 (缺失 / 非法)
//! - K-1 #3: Room Name 1..=256 字符, 限 ASCII 字母数字 + `-` + `_`
//! - K-1 #4: URL 必须 `wss://` 开头且带 host

use apeireth_core::deadline::TimeoutError;

use crate::error_taxonomy::{ClassifyError, ErrorCategory};
use crate::frame_codec::{CodecError, CodecErrorKind};

/// 信令协议族错误 (闭合变体表; 新增变体必须同步 `category()` 与词表测试).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LiveKitError {
    // ===== 调用准入 (m3 防御) =====
    /// 工具未在白名单内.
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    // ===== K-1 强校验 (4 项) =====
    /// K-1 #1: API Key 未设置.
    #[error("api key not set")]
    ApiKeyMissing,
    /// K-1 #1: API Key 格式非法 (只报规则, 不回显原值).
    #[error("api key invalid: {0}")]
    ApiKeyInvalid(String),
    /// K-1 #2: API Secret 未设置.
    #[error("api secret not set")]
    ApiSecretMissing,
    /// K-1 #2: API Secret 格式非法 (只报长度规则, 不回显原值).
    #[error("api secret invalid: {0}")]
    ApiSecretInvalid(String),
    /// K-1 #3: Room Name 为空.
    #[error("room name empty (expected 1..=256 chars)")]
    RoomNameEmpty,
    /// K-1 #3: Room Name 含非法字符或超长.
    #[error("room name invalid (only alphanumeric, `-`, `_` allowed): {0}")]
    RoomNameInvalid(String),
    /// K-1 #4: URL 不是 `wss://host` 形态.
    #[error("url must start with `wss://` and carry a host: `{0}`")]
    InvalidUrl(String),
    /// 其它入参 / 配置非法 (含超时取值过闸失败).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    // ===== 认证 =====
    /// 凭证被对端拒绝 / 过期 (握手鉴权失败).
    #[error("authentication rejected: {0}")]
    Authentication(String),

    // ===== 网络 / 传输 =====
    /// 未配置信令传输 (协议层需要注入传输实现; 不配置就假装成功是不诚实的).
    #[error("no signaling transport configured")]
    TransportUnavailable,
    /// 传输层读写失败.
    #[error("network failure: {0}")]
    Network(String),
    /// 连接建立失败 (握手前失败).
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    // ===== 协议 =====
    /// 协议违例: 帧编解码失败 / 状态机非法迁移 / 序列号违例.
    #[error("protocol violation: {0}")]
    Protocol(String),

    // ===== 限流 =====
    /// 对端限流 (携带建议退避时长).
    #[error("rate limited, retry after {retry_after_ms}ms")]
    RateLimited {
        /// 建议退避时长 (毫秒).
        retry_after_ms: u64,
    },

    // ===== 超时 =====
    /// 操作超时 (`apeireth_core::deadline` 熔合面裁决).
    #[error("operation timed out: {operation}")]
    Timeout {
        /// 超时的操作名 (稳定字面量, 如 `connect` / `handshake`).
        operation: &'static str,
    },

    // ===== 背压 =====
    /// 本地有界队列 / 发送窗口打满 (腾出窗口后可重试).
    #[error("backpressure: {0}")]
    Backpressure(String),

    // ===== 生命周期状态 =====
    /// 房间未连接 (需先 `connect`).
    #[error("room not connected: {0}")]
    RoomDisconnected(String),
    /// 轨道不存在.
    #[error("track not found: {0}")]
    TrackNotFound(String),
    /// 生命周期状态不允许该操作.
    #[error("illegal state: {0}")]
    State(String),

    // ===== 内部 =====
    /// 内部错误 (序列化失败 / 不变量破坏).
    #[error("internal error: {0}")]
    Internal(String),
}

impl LiveKitError {
    /// K-1 #1: 校验 API Key (非空 + ≥10 字符 + 限 ASCII 字母数字 / `-` / `_`).
    ///
    /// 错误消息绝不回显 key 原值, 只报规则 (日志脱敏红线)。
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
        if !api_key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Self::ApiKeyInvalid(
                "api key contains invalid chars (only alphanumeric, `-`, `_` allowed; key redacted)"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// K-1 #2: 校验 API Secret (非空 + ≥32 字符).
    pub fn validate_api_secret(api_secret: &str) -> Result<(), Self> {
        if api_secret.is_empty() {
            return Err(Self::ApiSecretMissing);
        }
        if api_secret.len() < 32 {
            return Err(Self::ApiSecretInvalid(format!(
                "api secret too short: {} chars (< 32)",
                api_secret.len()
            )));
        }
        Ok(())
    }

    /// K-1 #3: 校验 Room Name (1..=256 字符, 限 ASCII 字母数字 + `-` + `_`).
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

    /// K-1 #4: 校验 URL (`wss://` 开头且带 host).
    pub fn validate_url(url: &str) -> Result<(), Self> {
        if !url.starts_with("wss://") {
            return Err(Self::InvalidUrl(url.to_string()));
        }
        if url.len() <= "wss://".len() {
            return Err(Self::InvalidUrl(format!(
                "url missing host: `{url}` (expected `wss://host:port`)"
            )));
        }
        Ok(())
    }

    /// 把 `apeireth_core::deadline` 超时熔合面的取值错误归入闭合词表.
    ///
    /// 取值错误 (0 / 超上限 / 无界哨兵) 是入参问题 → `InvalidArgument`;
    /// 定时器无法建立 (缺异步运行时) 是内部问题 → `Internal`。
    pub fn from_deadline(err: TimeoutError, context: &'static str) -> Self {
        match err {
            TimeoutError::Zero
            | TimeoutError::AboveCap { .. }
            | TimeoutError::UnboundedSentinel => {
                Self::InvalidArgument(format!("{context}: invalid timeout value ({err})"))
            }
            TimeoutError::NoRuntime => {
                Self::Internal(format!("{context}: deadline timer unavailable ({err})"))
            }
        }
    }
}

impl From<CodecError> for LiveKitError {
    fn from(err: CodecError) -> Self {
        match err.kind {
            CodecErrorKind::Encode => {
                Self::Internal(format!("frame encode failed: {}", err.detail))
            }
            CodecErrorKind::Oversize | CodecErrorKind::Malformed | CodecErrorKind::Poisoned => {
                Self::Protocol(err.to_string())
            }
        }
    }
}

impl ClassifyError for LiveKitError {
    fn category(&self) -> ErrorCategory {
        match self {
            LiveKitError::ToolNotWhitelisted(_)
            | LiveKitError::ApiKeyInvalid(_)
            | LiveKitError::ApiSecretInvalid(_)
            | LiveKitError::RoomNameEmpty
            | LiveKitError::RoomNameInvalid(_)
            | LiveKitError::InvalidUrl(_)
            | LiveKitError::InvalidArgument(_) => ErrorCategory::Validation,
            LiveKitError::ApiKeyMissing
            | LiveKitError::ApiSecretMissing
            | LiveKitError::Authentication(_) => ErrorCategory::Authentication,
            LiveKitError::TransportUnavailable
            | LiveKitError::Network(_)
            | LiveKitError::ConnectionFailed(_) => ErrorCategory::Network,
            LiveKitError::Protocol(_) => ErrorCategory::Protocol,
            LiveKitError::RateLimited { .. } => ErrorCategory::RateLimited,
            LiveKitError::Timeout { .. } => ErrorCategory::Timeout,
            LiveKitError::Backpressure(_) => ErrorCategory::Backpressure,
            LiveKitError::RoomDisconnected(_)
            | LiveKitError::TrackNotFound(_)
            | LiveKitError::State(_) => ErrorCategory::State,
            LiveKitError::Internal(_) => ErrorCategory::Internal,
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            // 传输未注入: 重试无意义, 需先装配
            LiveKitError::TransportUnavailable => false,
            // 凭证类: 换凭证后才有意义, 原样重试只会复现
            LiveKitError::ApiKeyMissing
            | LiveKitError::ApiSecretMissing
            | LiveKitError::Authentication(_) => false,
            other => other.category().default_retryable(),
        }
    }

    fn retry_after_ms(&self) -> Option<u64> {
        match self {
            LiveKitError::RateLimited { retry_after_ms } => Some(*retry_after_ms),
            _ => None,
        }
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
    fn k1_validate_api_key_invalid_chars_never_echoes_key() {
        let bad = "APIxxxxxxxx!@#$%";
        let err = LiveKitError::validate_api_key(bad).unwrap_err();
        assert!(matches!(err, LiveKitError::ApiKeyInvalid(_)));
        let msg = err.to_string();
        assert!(!msg.contains(bad), "错误消息不得回显 key 原值: {msg}");
        assert!(msg.contains("redacted"), "错误消息应标明已脱敏: {msg}");
    }

    #[test]
    fn k1_validate_api_key_valid() {
        assert!(LiveKitError::validate_api_key("API12345678").is_ok());
        assert!(LiveKitError::validate_api_key("test-placeholder-12345").is_ok());
    }

    #[test]
    fn k1_validate_api_secret_boundaries() {
        assert!(matches!(
            LiveKitError::validate_api_secret(""),
            Err(LiveKitError::ApiSecretMissing)
        ));
        assert!(matches!(
            LiveKitError::validate_api_secret("abcdef1234567890"),
            Err(LiveKitError::ApiSecretInvalid(_))
        ));
        assert!(LiveKitError::validate_api_secret("abcdef1234567890abcdef1234567890").is_ok());
    }

    #[test]
    fn k1_validate_room_name_boundaries() {
        assert!(matches!(
            LiveKitError::validate_room_name(""),
            Err(LiveKitError::RoomNameEmpty)
        ));
        let long = "a".repeat(257);
        assert!(matches!(
            LiveKitError::validate_room_name(&long),
            Err(LiveKitError::RoomNameInvalid(_))
        ));
        assert!(matches!(
            LiveKitError::validate_room_name("room name with spaces"),
            Err(LiveKitError::RoomNameInvalid(_))
        ));
        assert!(LiveKitError::validate_room_name("my-room-1").is_ok());
        assert!(LiveKitError::validate_room_name("room_2_alpha").is_ok());
    }

    #[test]
    fn k1_validate_url_boundaries() {
        assert!(matches!(
            LiveKitError::validate_url("http://example.com"),
            Err(LiveKitError::InvalidUrl(_))
        ));
        assert!(matches!(
            LiveKitError::validate_url("ws://example.com"),
            Err(LiveKitError::InvalidUrl(_))
        ));
        assert!(matches!(
            LiveKitError::validate_url("wss://"),
            Err(LiveKitError::InvalidUrl(_))
        ));
        assert!(LiveKitError::validate_url("wss://signal.example.com").is_ok());
        assert!(LiveKitError::validate_url("wss://signal.example.com:7880").is_ok());
    }

    #[test]
    fn closed_vocabulary_maps_every_variant() {
        let cases: Vec<(LiveKitError, ErrorCategory)> = vec![
            (
                LiveKitError::ToolNotWhitelisted("t".into()),
                ErrorCategory::Validation,
            ),
            (LiveKitError::ApiKeyMissing, ErrorCategory::Authentication),
            (
                LiveKitError::ApiKeyInvalid("x".into()),
                ErrorCategory::Validation,
            ),
            (
                LiveKitError::ApiSecretMissing,
                ErrorCategory::Authentication,
            ),
            (
                LiveKitError::ApiSecretInvalid("x".into()),
                ErrorCategory::Validation,
            ),
            (LiveKitError::RoomNameEmpty, ErrorCategory::Validation),
            (
                LiveKitError::RoomNameInvalid("x".into()),
                ErrorCategory::Validation,
            ),
            (
                LiveKitError::InvalidUrl("x".into()),
                ErrorCategory::Validation,
            ),
            (
                LiveKitError::InvalidArgument("x".into()),
                ErrorCategory::Validation,
            ),
            (
                LiveKitError::Authentication("x".into()),
                ErrorCategory::Authentication,
            ),
            (LiveKitError::TransportUnavailable, ErrorCategory::Network),
            (LiveKitError::Network("x".into()), ErrorCategory::Network),
            (
                LiveKitError::ConnectionFailed("x".into()),
                ErrorCategory::Network,
            ),
            (LiveKitError::Protocol("x".into()), ErrorCategory::Protocol),
            (
                LiveKitError::RateLimited { retry_after_ms: 5 },
                ErrorCategory::RateLimited,
            ),
            (
                LiveKitError::Timeout {
                    operation: "connect",
                },
                ErrorCategory::Timeout,
            ),
            (
                LiveKitError::Backpressure("x".into()),
                ErrorCategory::Backpressure,
            ),
            (
                LiveKitError::RoomDisconnected("x".into()),
                ErrorCategory::State,
            ),
            (
                LiveKitError::TrackNotFound("x".into()),
                ErrorCategory::State,
            ),
            (LiveKitError::State("x".into()), ErrorCategory::State),
            (LiveKitError::Internal("x".into()), ErrorCategory::Internal),
        ];
        for (err, expected) in cases {
            assert_eq!(err.category(), expected, "misclassified: {err:?}");
            assert!(
                ErrorCategory::ALL.contains(&err.category()),
                "category must stay in closed vocabulary: {err:?}"
            );
        }
    }

    #[test]
    fn retryability_follows_closed_rules() {
        // 暂态可重试
        assert!(LiveKitError::Network("x".into()).is_retryable());
        assert!(LiveKitError::ConnectionFailed("x".into()).is_retryable());
        assert!(LiveKitError::Timeout {
            operation: "connect"
        }
        .is_retryable());
        assert!(LiveKitError::Backpressure("x".into()).is_retryable());
        assert!(LiveKitError::RateLimited {
            retry_after_ms: 100
        }
        .is_retryable());
        // 稳态不可重试
        assert!(!LiveKitError::TransportUnavailable.is_retryable());
        assert!(!LiveKitError::ApiKeyMissing.is_retryable());
        assert!(!LiveKitError::Authentication("x".into()).is_retryable());
        assert!(!LiveKitError::Protocol("x".into()).is_retryable());
        assert!(!LiveKitError::InvalidArgument("x".into()).is_retryable());
        assert!(!LiveKitError::RoomDisconnected("x".into()).is_retryable());
    }

    #[test]
    fn rate_limit_carries_retry_after() {
        let err = LiveKitError::RateLimited {
            retry_after_ms: 1500,
        };
        assert_eq!(err.retry_after_ms(), Some(1500));
        assert_eq!(LiveKitError::Network("x".into()).retry_after_ms(), None);
    }

    #[test]
    fn codec_error_maps_to_protocol_except_encode() {
        let oversize = CodecError::new(CodecErrorKind::Oversize, "len 999999999");
        let mapped: LiveKitError = oversize.into();
        assert_eq!(mapped.category(), ErrorCategory::Protocol);

        let malformed = CodecError::new(CodecErrorKind::Malformed, "body[4B]");
        let mapped: LiveKitError = malformed.into();
        assert_eq!(mapped.category(), ErrorCategory::Protocol);

        let encode = CodecError::new(CodecErrorKind::Encode, "serialize failed");
        let mapped: LiveKitError = encode.into();
        assert_eq!(mapped.category(), ErrorCategory::Internal);
    }

    #[test]
    fn deadline_error_maps_by_kind() {
        let bad_value: LiveKitError = LiveKitError::from_deadline(TimeoutError::Zero, "connect");
        assert_eq!(bad_value.category(), ErrorCategory::Validation);

        let no_runtime: LiveKitError =
            LiveKitError::from_deadline(TimeoutError::NoRuntime, "connect");
        assert_eq!(no_runtime.category(), ErrorCategory::Internal);

        let above_cap: LiveKitError = LiveKitError::from_deadline(
            TimeoutError::AboveCap {
                requested: 10,
                cap: 5,
            },
            "connect",
        );
        assert_eq!(above_cap.category(), ErrorCategory::Validation);
    }
}
