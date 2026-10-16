//! 语音流协议族错误类型: 强类型变体 + 闭合词表归类.
//!
//! 纪律 (与信令族一致):
//! - 每个变体归入 [`crate::error_taxonomy::ErrorCategory`] 闭合词表之一
//!   ([`VoiceError::category`]), 调用方据此做重试 / 降级决策, 不解析文案。
//! - 凭证永不进错误文案 (日志脱敏红线): 校验失败只报规则与长度。
//! - 超时统一走 `apeireth_core::deadline` 熔合面 ([`VoiceError::from_deadline`]);
//!   流式编解码失败归入协议违例。
//!
//! K-1 强校验 (6 项) 仍是第一道防御 (API Key / Audio Format / Sample Rate /
//! Bit Depth / Channels / Language)。

use apeireth_core::deadline::TimeoutError;

use crate::error_taxonomy::{ClassifyError, ErrorCategory};
use crate::frame_codec::{CodecError, CodecErrorKind};

/// 语音流协议族错误 (闭合变体表; 新增变体必须同步 `category()` 与词表测试).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VoiceError {
    // ===== K-1 强校验 (7 变体) =====
    /// K-1 #1: API Key 缺失 (空 / 全空白).
    #[error("api key is missing (K-1 #1, expected non-empty)")]
    ApiKeyMissing,
    /// K-1 #1: API Key 格式非法 (长度不足).
    #[error("api key is invalid: length={0} < 16 (K-1 #1)")]
    ApiKeyInvalid(usize),
    /// K-1 #2: Audio Format 非法.
    #[error("audio format is invalid: {0} (K-1 #2, expected wav/mp3/opus/flac)")]
    AudioFormatInvalid(String),
    /// K-1 #3: Sample Rate 越界.
    #[error("sample rate is invalid: {0} (K-1 #3, expected 8000..=48000 Hz)")]
    SampleRateInvalid(u32),
    /// K-1 #4: Bit Depth 非法.
    #[error("bit depth is invalid: {0} (K-1 #4, expected 8/16/24/32)")]
    BitDepthInvalid(u16),
    /// K-1 #5: Channels 非法.
    #[error("channels is invalid: {0} (K-1 #5, expected 1/2)")]
    ChannelsInvalid(u8),
    /// K-1 #6: Language 非法.
    #[error("language is invalid: {0} (K-1 #6, expected ISO 639-1 like en/zh-CN)")]
    LanguageInvalid(String),

    // ===== 调用准入 (m3 防御) =====
    /// 工具未在白名单内.
    #[error("tool not whitelisted: {0}")]
    ToolNotWhitelisted(String),

    // ===== 入参 / 配置 =====
    /// 其它入参 / 配置非法 (含超时取值过闸失败).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    // ===== 认证 =====
    /// 凭证被对端拒绝.
    #[error("authentication rejected: {0}")]
    Authentication(String),
    /// 访问令牌过期.
    #[error("access token expired (refresh needed)")]
    TokenExpired,

    // ===== 网络 / 传输 =====
    /// 未配置语音传输 (协议层需要注入传输实现).
    #[error("no voice transport configured")]
    TransportUnavailable,
    /// 传输层读写失败.
    #[error("network failure: {0}")]
    Network(String),

    // ===== 协议 =====
    /// 协议违例: 帧编解码失败 / 分块序号违例 / 响应形状非法.
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
        /// 超时的操作名 (稳定字面量, 如 `transcribe` / `synthesize`).
        operation: &'static str,
    },

    // ===== 背压 =====
    /// 有界队列 / 发送窗口打满 (腾出窗口后可重试).
    #[error("backpressure: {0}")]
    Backpressure(String),

    // ===== 生命周期状态 =====
    /// 生命周期状态不允许该操作.
    #[error("illegal state: {0}")]
    State(String),

    // ===== 内部 =====
    /// 内部错误 (序列化失败 / 不变量破坏).
    #[error("internal error: {0}")]
    Internal(String),
}

/// 语音流协议族错误类型别名.
pub type VoiceResult<T> = Result<T, VoiceError>;

/// 闭合变体表规模 (测试钉死).
pub const VOICE_ERROR_VARIANT_COUNT: usize = 19;

impl VoiceError {
    /// **K-1 #1**: 校验 API Key (非空 + 长度 ≥ 16).
    pub fn validate_api_key(api_key: &str) -> VoiceResult<()> {
        let trimmed = api_key.trim();
        if trimmed.is_empty() {
            return Err(VoiceError::ApiKeyMissing);
        }
        if trimmed.len() < 16 {
            return Err(VoiceError::ApiKeyInvalid(trimmed.len()));
        }
        Ok(())
    }

    /// **K-1 #2**: 校验 Audio Format (wav / mp3 / opus / flac, 大小写不敏感).
    pub fn validate_audio_format(format: &str) -> VoiceResult<()> {
        let trimmed = format.trim().to_lowercase();
        if trimmed.is_empty() {
            return Err(VoiceError::AudioFormatInvalid(format.to_string()));
        }
        match trimmed.as_str() {
            "wav" | "mp3" | "opus" | "flac" => Ok(()),
            _ => Err(VoiceError::AudioFormatInvalid(format.to_string())),
        }
    }

    /// **K-1 #3**: 校验 Sample Rate (8000..=48000 Hz).
    pub fn validate_sample_rate(sample_rate: u32) -> VoiceResult<()> {
        if !(8000..=48000).contains(&sample_rate) {
            return Err(VoiceError::SampleRateInvalid(sample_rate));
        }
        Ok(())
    }

    /// **K-1 #4**: 校验 Bit Depth (8 / 16 / 24 / 32).
    pub fn validate_bit_depth(bit_depth: u16) -> VoiceResult<()> {
        if !matches!(bit_depth, 8 | 16 | 24 | 32) {
            return Err(VoiceError::BitDepthInvalid(bit_depth));
        }
        Ok(())
    }

    /// **K-1 #5**: 校验 Channels (1 / 2).
    pub fn validate_channels(channels: u8) -> VoiceResult<()> {
        if !matches!(channels, 1 | 2) {
            return Err(VoiceError::ChannelsInvalid(channels));
        }
        Ok(())
    }

    /// **K-1 #6**: 校验 Language (ISO 639-1 简化: `xx` 或 `xx-YY`).
    pub fn validate_language(language: &str) -> VoiceResult<()> {
        let trimmed = language.trim();
        if trimmed.is_empty() {
            return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
        }
        let parts: Vec<&str> = trimmed.split('-').collect();
        if parts.is_empty() || parts[0].is_empty() || parts[0].len() > 3 {
            return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
        }
        for c in parts[0].chars() {
            if !c.is_ascii_alphabetic() {
                return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
            }
        }
        if parts.len() > 1 {
            for region in &parts[1..] {
                if region.is_empty() || region.len() > 4 {
                    return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
                }
                for c in region.chars() {
                    if !c.is_ascii_alphabetic() {
                        return Err(VoiceError::LanguageInvalid(trimmed.to_string()));
                    }
                }
            }
        }
        Ok(())
    }

    /// 把 `apeireth_core::deadline` 超时熔合面的取值错误归入闭合词表.
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

impl From<CodecError> for VoiceError {
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

impl ClassifyError for VoiceError {
    fn category(&self) -> ErrorCategory {
        match self {
            VoiceError::ApiKeyInvalid(_)
            | VoiceError::AudioFormatInvalid(_)
            | VoiceError::SampleRateInvalid(_)
            | VoiceError::BitDepthInvalid(_)
            | VoiceError::ChannelsInvalid(_)
            | VoiceError::LanguageInvalid(_)
            | VoiceError::ToolNotWhitelisted(_)
            | VoiceError::InvalidArgument(_) => ErrorCategory::Validation,
            VoiceError::ApiKeyMissing
            | VoiceError::Authentication(_)
            | VoiceError::TokenExpired => ErrorCategory::Authentication,
            VoiceError::TransportUnavailable | VoiceError::Network(_) => ErrorCategory::Network,
            VoiceError::Protocol(_) => ErrorCategory::Protocol,
            VoiceError::RateLimited { .. } => ErrorCategory::RateLimited,
            VoiceError::Timeout { .. } => ErrorCategory::Timeout,
            VoiceError::Backpressure(_) => ErrorCategory::Backpressure,
            VoiceError::State(_) => ErrorCategory::State,
            VoiceError::Internal(_) => ErrorCategory::Internal,
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            // 传输未注入: 需先装配, 重试无意义
            VoiceError::TransportUnavailable => false,
            // 凭证类: 换凭证后才有意义
            VoiceError::ApiKeyMissing
            | VoiceError::Authentication(_)
            | VoiceError::TokenExpired => false,
            other => other.category().default_retryable(),
        }
    }

    fn retry_after_ms(&self) -> Option<u64> {
        match self {
            VoiceError::RateLimited { retry_after_ms } => Some(*retry_after_ms),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// K-1 #1: API Key
    #[test]
    fn k1_api_key_boundaries() {
        assert!(VoiceError::validate_api_key("1234567890abcdef").is_ok());
        assert!(VoiceError::validate_api_key("sk-voice-abcdef1234567890xyz").is_ok());
        assert!(matches!(
            VoiceError::validate_api_key(""),
            Err(VoiceError::ApiKeyMissing)
        ));
        assert!(matches!(
            VoiceError::validate_api_key("   "),
            Err(VoiceError::ApiKeyMissing)
        ));
        assert!(matches!(
            VoiceError::validate_api_key("short"),
            Err(VoiceError::ApiKeyInvalid(5))
        ));
    }

    /// K-1 #2: Audio Format
    #[test]
    fn k1_audio_format_boundaries() {
        for fmt in ["wav", "mp3", "opus", "flac", "WAV", "Mp3"] {
            assert!(VoiceError::validate_audio_format(fmt).is_ok(), "fmt {fmt}");
        }
        for fmt in ["", "aac", "ogg"] {
            assert!(matches!(
                VoiceError::validate_audio_format(fmt),
                Err(VoiceError::AudioFormatInvalid(_))
            ));
        }
    }

    /// K-1 #3: Sample Rate
    #[test]
    fn k1_sample_rate_boundaries() {
        for rate in [8000, 16000, 44100, 48000] {
            assert!(VoiceError::validate_sample_rate(rate).is_ok());
        }
        for rate in [0, 7999, 48001] {
            assert!(matches!(
                VoiceError::validate_sample_rate(rate),
                Err(VoiceError::SampleRateInvalid(_))
            ));
        }
    }

    /// K-1 #4/#5: Bit Depth / Channels
    #[test]
    fn k1_bit_depth_and_channels_boundaries() {
        for depth in [8u16, 16, 24, 32] {
            assert!(VoiceError::validate_bit_depth(depth).is_ok());
        }
        for depth in [0u16, 4, 12, 64] {
            assert!(matches!(
                VoiceError::validate_bit_depth(depth),
                Err(VoiceError::BitDepthInvalid(_))
            ));
        }
        for ch in [1u8, 2] {
            assert!(VoiceError::validate_channels(ch).is_ok());
        }
        for ch in [0u8, 6, 8] {
            assert!(matches!(
                VoiceError::validate_channels(ch),
                Err(VoiceError::ChannelsInvalid(_))
            ));
        }
    }

    /// K-1 #6: Language
    #[test]
    fn k1_language_boundaries() {
        for lang in [
            "en", "zh", "ja", "zh-CN", "en-US", "pt-BR", "en-USA", "en-USAA",
        ] {
            assert!(VoiceError::validate_language(lang).is_ok(), "lang {lang}");
        }
        for lang in ["", "english", "e1", "en-USAXX", "zh-CN-extra", "en-USAAA"] {
            assert!(matches!(
                VoiceError::validate_language(lang),
                Err(VoiceError::LanguageInvalid(_))
            ));
        }
    }

    #[test]
    fn variant_table_is_closed_at_19() {
        assert_eq!(VOICE_ERROR_VARIANT_COUNT, 19);
    }

    #[test]
    fn closed_vocabulary_maps_every_variant() {
        let cases: Vec<(VoiceError, ErrorCategory)> = vec![
            (VoiceError::ApiKeyMissing, ErrorCategory::Authentication),
            (VoiceError::ApiKeyInvalid(3), ErrorCategory::Validation),
            (
                VoiceError::AudioFormatInvalid("x".into()),
                ErrorCategory::Validation,
            ),
            (VoiceError::SampleRateInvalid(1), ErrorCategory::Validation),
            (VoiceError::BitDepthInvalid(1), ErrorCategory::Validation),
            (VoiceError::ChannelsInvalid(1), ErrorCategory::Validation),
            (
                VoiceError::LanguageInvalid("x".into()),
                ErrorCategory::Validation,
            ),
            (
                VoiceError::ToolNotWhitelisted("x".into()),
                ErrorCategory::Validation,
            ),
            (
                VoiceError::InvalidArgument("x".into()),
                ErrorCategory::Validation,
            ),
            (
                VoiceError::Authentication("x".into()),
                ErrorCategory::Authentication,
            ),
            (VoiceError::TokenExpired, ErrorCategory::Authentication),
            (VoiceError::TransportUnavailable, ErrorCategory::Network),
            (VoiceError::Network("x".into()), ErrorCategory::Network),
            (VoiceError::Protocol("x".into()), ErrorCategory::Protocol),
            (
                VoiceError::RateLimited { retry_after_ms: 5 },
                ErrorCategory::RateLimited,
            ),
            (
                VoiceError::Timeout {
                    operation: "transcribe",
                },
                ErrorCategory::Timeout,
            ),
            (
                VoiceError::Backpressure("x".into()),
                ErrorCategory::Backpressure,
            ),
            (VoiceError::State("x".into()), ErrorCategory::State),
            (VoiceError::Internal("x".into()), ErrorCategory::Internal),
        ];
        for (err, expected) in cases {
            assert_eq!(err.category(), expected, "misclassified: {err:?}");
            assert!(ErrorCategory::ALL.contains(&err.category()));
        }
    }

    #[test]
    fn retryability_and_retry_after_follow_closed_rules() {
        assert!(VoiceError::Network("x".into()).is_retryable());
        assert!(VoiceError::Timeout { operation: "op" }.is_retryable());
        assert!(VoiceError::Backpressure("x".into()).is_retryable());
        assert!(VoiceError::RateLimited { retry_after_ms: 9 }.is_retryable());
        assert!(!VoiceError::TransportUnavailable.is_retryable());
        assert!(!VoiceError::ApiKeyMissing.is_retryable());
        assert!(!VoiceError::TokenExpired.is_retryable());
        assert!(!VoiceError::Protocol("x".into()).is_retryable());
        assert_eq!(
            VoiceError::RateLimited { retry_after_ms: 9 }.retry_after_ms(),
            Some(9)
        );
        assert_eq!(VoiceError::Network("x".into()).retry_after_ms(), None);
    }

    #[test]
    fn codec_and_deadline_errors_classify_closed() {
        let malformed: VoiceError = CodecError::new(CodecErrorKind::Malformed, "body[3B]").into();
        assert_eq!(malformed.category(), ErrorCategory::Protocol);
        let encode: VoiceError = CodecError::new(CodecErrorKind::Encode, "x").into();
        assert_eq!(encode.category(), ErrorCategory::Internal);

        let bad_timeout: VoiceError = VoiceError::from_deadline(TimeoutError::Zero, "transcribe");
        assert_eq!(bad_timeout.category(), ErrorCategory::Validation);
        let no_runtime: VoiceError =
            VoiceError::from_deadline(TimeoutError::NoRuntime, "transcribe");
        assert_eq!(no_runtime.category(), ErrorCategory::Internal);
    }
}
