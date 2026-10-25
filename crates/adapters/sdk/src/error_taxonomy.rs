//! 错误闭合词表: 全 SDK 统一、有限、可枚举的错误分类面.
//!
//! 协议族 (信令 / 语音流) 的错误枚举各自保留强类型变体, 但都必须映射到
//! 本模块的 [`ErrorCategory`] 闭合词表之一 —— 调用方据此做重试 / 降级 /
//! 上报决策, 不解析错误文案。
//!
//! 闭合词表 (9 类, 新增分类必须走评审并同步本模块测试):
//!
//! | 分类 | 含义 | 典型可重试 |
//! |---|---|---|
//! | `Validation` | 入参 / 配置校验失败 (K-1 强校验面) | 否 |
//! | `Authentication` | 凭证缺失 / 被拒 / 过期 | 否 (换凭证后可) |
//! | `Network` | 传输层失败 (连接断开 / 读写失败) | 是 |
//! | `Protocol` | 帧编解码 / 状态机 / 序列号违例 | 否 |
//! | `RateLimited` | 对端限流 (携带 `retry_after_ms`) | 是 (退避后) |
//! | `Timeout` | 超时 (`apeireth_core::deadline` 熔合面) | 是 |
//! | `Backpressure` | 本地有界队列 / 窗口打满 | 是 (腾出窗口后) |
//! | `State` | 生命周期状态不允许该操作 (如未连接先发布) | 否 |
//! | `Internal` | 内部不变量破坏 / 兜底 | 否 |

use serde::{Deserialize, Serialize};

/// 错误闭合词表 (9 类).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// 入参 / 配置校验失败.
    Validation,
    /// 凭证缺失 / 被拒 / 过期.
    Authentication,
    /// 传输层失败.
    Network,
    /// 协议违例 (帧格式 / 状态机 / 序列号).
    Protocol,
    /// 对端限流.
    RateLimited,
    /// 超时.
    Timeout,
    /// 本地有界队列 / 发送窗口打满.
    Backpressure,
    /// 生命周期状态不允许该操作.
    State,
    /// **能力不支持**（该能力尚未实现 / 本客户端不提供）。
    ///
    /// 2026-10-25（owner 裁定）：此前这类情形只有两个去处 ——
    /// `Internal`（读成「服务端内部错误」，会让人去查服务端）或
    /// `RateLimited`（读成「配额用完」，会让人去升级套餐）。
    /// **两者都不是真相** ⇒ 新增本成员，让宿主能准确区分
    /// 「这个能力根本不存在」/「额度不够」/「对方在限流」。
    ///
    /// **不可重试**：重试同一个未实现的能力，结果永远是同一个。
    Unsupported,
    /// 内部错误 / 兜底.
    Internal,
}

impl ErrorCategory {
    /// 分类数量 (闭合词表规模, 测试钉死).
    pub const COUNT: usize = 10;

    /// 全部分类 (枚举序稳定).
    pub const ALL: [ErrorCategory; Self::COUNT] = [
        ErrorCategory::Validation,
        ErrorCategory::Authentication,
        ErrorCategory::Network,
        ErrorCategory::Protocol,
        ErrorCategory::RateLimited,
        ErrorCategory::Timeout,
        ErrorCategory::Backpressure,
        ErrorCategory::State,
        ErrorCategory::Unsupported,
        ErrorCategory::Internal,
    ];

    /// 稳定字符串 (日志 / 指标维度).
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCategory::Validation => "validation",
            ErrorCategory::Authentication => "authentication",
            ErrorCategory::Network => "network",
            ErrorCategory::Protocol => "protocol",
            ErrorCategory::RateLimited => "rate_limited",
            ErrorCategory::Timeout => "timeout",
            ErrorCategory::Backpressure => "backpressure",
            ErrorCategory::State => "state",
            ErrorCategory::Unsupported => "unsupported",
            ErrorCategory::Internal => "internal",
        }
    }

    /// 从字符串解析 (与 [`ErrorCategory::as_str`] 严格互逆).
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }

    /// 默认可重试判定 (分类级; 具体错误可经 `is_retryable` 收紧).
    ///
    /// `Network` / `RateLimited` / `Timeout` / `Backpressure` 为暂态, 可重试;
    /// 其余为稳态, 重试同一请求只会复现同一失败。
    pub const fn default_retryable(self) -> bool {
        matches!(
            self,
            ErrorCategory::Network
                | ErrorCategory::RateLimited
                | ErrorCategory::Timeout
                | ErrorCategory::Backpressure
        )
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 统一错误分类契约: 各协议族错误枚举实现本 trait, 挂进闭合词表.
pub trait ClassifyError {
    /// 归入闭合词表分类.
    fn category(&self) -> ErrorCategory;

    /// 是否可重试 (默认跟随分类; 具体错误可覆盖).
    fn is_retryable(&self) -> bool {
        self.category().default_retryable()
    }

    /// 限流建议等待时长 (仅限流类错误携带).
    fn retry_after_ms(&self) -> Option<u64> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_table_is_closed_at_10() {
        // 2026-10-25（owner 裁定）：加了 `Unsupported`（能力不支持）⇒ 9 → 10。
        assert_eq!(ErrorCategory::COUNT, 10);
        assert_eq!(ErrorCategory::ALL.len(), 10);
    }

    #[test]
    fn category_str_parse_roundtrip() {
        for cat in ErrorCategory::ALL {
            assert_eq!(ErrorCategory::parse(cat.as_str()), Some(cat));
        }
        assert_eq!(ErrorCategory::parse("bogus"), None);
    }

    #[test]
    fn category_default_retryable_is_transient_only() {
        for cat in ErrorCategory::ALL {
            let expect = matches!(
                cat,
                ErrorCategory::Network
                    | ErrorCategory::RateLimited
                    | ErrorCategory::Timeout
                    | ErrorCategory::Backpressure
            );
            assert_eq!(cat.default_retryable(), expect, "category {}", cat.as_str());
        }
    }

    #[test]
    fn category_display_matches_as_str() {
        assert_eq!(format!("{}", ErrorCategory::RateLimited), "rate_limited");
        assert_eq!(format!("{}", ErrorCategory::Backpressure), "backpressure");
    }
}

// ============================================================================
// 三个错误族的映射（2026-10-24 接电：此前 `ClassifyError` 零实现）
// ============================================================================
//
// `ClassifyError` 是「各协议族错误枚举 → 闭合词表」的契约。之前**没有任何
// 实现者** —— 词表本身有测试，但没有任何真实错误走进去 ⇒ 调用方拿不到
// `category` / `is_retryable`，仍然只能自己解析错误文案（正是本模块要消灭的事）。
//
// 映射原则（写在这里，便于评审推翻）：
// - **调用方做错了** → `Validation`（改参数，不重试）；
// - **凭证/授权不行** → `Authentication`（换凭证，不重试）；
// - **对面/网络暂时不行** → `Network` / `Timeout` / `RateLimited`（退避重试）；
// - **帧或状态机违例** → `Protocol`（重试无用，同样的字节还会违例）；
// - **我们自己坏了** → `Internal`（兜底桶，不重试）。
//
// 有争议的几处**显式覆盖 `is_retryable`**，见下方逐条注释。

use crate::client::SdkClientError;
use crate::error::{SdkError, SdkErrorCode};
use crate::frame_codec::{CodecError, CodecErrorKind};

impl ClassifyError for SdkClientError {
    fn category(&self) -> ErrorCategory {
        match self {
            // 白名单是**客户端配置**问题：改配置即可，重试同一个调用没用。
            Self::ToolNotWhitelisted(_) => ErrorCategory::Validation,
            Self::AuthFailed(_) => ErrorCategory::Authentication,
            Self::Network(_) => ErrorCategory::Network,
            Self::Protocol(_) => ErrorCategory::Protocol,
            Self::RateLimited(_) => ErrorCategory::RateLimited,
            // 配额耗尽归 `RateLimited`（它确实是额度限制），但**不可重试**：
            // 等一会儿不会变好，得改套餐/等计费周期。所以下面覆盖 `is_retryable`。
            Self::QuotaExceeded(_) => ErrorCategory::RateLimited,
            // `ok: false` 的业务失败：对面工具执行失败，原因未知 ⇒ 兜底桶。
            Self::ToolCallFailed(_) => ErrorCategory::Internal,
            // 500 归 `Internal`，但**覆盖为可重试**：5xx 常是服务端瞬时故障，
            // 退避后重试是标准做法（与「内部不变量破坏」不同）。
            Self::ServerInternal(_) => ErrorCategory::Internal,
            // STUB MODE 是**能力缺口**不是瞬时故障：重试永远还是 501。
            // 2026-10-25（owner 裁定）：改用新增的 `Unsupported` 分类 ⇒
            // 宿主能把它与「服务端内部错误」区分开（前者不该去查服务端）。
            Self::NotImplemented(_) => ErrorCategory::Unsupported,
            Self::Other(_) => ErrorCategory::Internal,
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::QuotaExceeded(_) => false,
            Self::ServerInternal(_) => true,
            // NotImplemented 明确不可重试（重试还是同一个 501）
            Self::NotImplemented(_) => false,
            other => other.category().default_retryable(),
        }
    }

    fn retry_after_ms(&self) -> Option<u64> {
        match self {
            // 秒 → 毫秒；饱和乘法避免溢出造出一个「永远等不到」的时间。
            Self::RateLimited(seconds) => Some(seconds.saturating_mul(1_000)),
            _ => None,
        }
    }
}

impl ClassifyError for SdkErrorCode {
    fn category(&self) -> ErrorCategory {
        match self {
            Self::Unknown => ErrorCategory::Internal,
            // envelope 缺字段 / 版本不兼容 = 协议面
            Self::InvalidEnvelope => ErrorCategory::Protocol,
            Self::VersionIncompatible => ErrorCategory::Protocol,
            // 资源不存在 = 调用方给的 id 有问题（改参数，不重试）
            Self::NotFound => ErrorCategory::Validation,
            Self::PermissionDenied => ErrorCategory::Authentication,
            Self::ToolNotApproved => ErrorCategory::Authentication,
            Self::Internal => ErrorCategory::Internal,
            Self::Other(_) => ErrorCategory::Internal,
        }
    }
}

impl ClassifyError for SdkError {
    fn category(&self) -> ErrorCategory {
        match self {
            // 编解码失败 = 协议面（同样的字节重试还会失败）
            Self::Wire(_) => ErrorCategory::Protocol,
            Self::Business { code, .. } => code.category(),
        }
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::Wire(_) => false,
            Self::Business { code, .. } => code.is_retryable(),
        }
    }
}

impl ClassifyError for CodecErrorKind {
    fn category(&self) -> ErrorCategory {
        match self {
            // 帧超限 / 结构非法 = 协议违例（重试无用）
            Self::Oversize => ErrorCategory::Protocol,
            Self::Malformed => ErrorCategory::Protocol,
            // 编码侧失败是我们自己的数据不满足序列化契约 ⇒ 兜底桶
            Self::Encode => ErrorCategory::Internal,
            // 解码器已熔断：帧边界不再可信，**必须重建连接**才能继续
            // ⇒ 生命周期状态不允许继续操作，归 `State`。
            Self::Poisoned => ErrorCategory::State,
        }
    }
}

impl ClassifyError for CodecError {
    fn category(&self) -> ErrorCategory {
        self.kind.category()
    }

    fn is_retryable(&self) -> bool {
        self.kind.is_retryable()
    }
}

/// 跨语言错误帧：把任意可分类错误压成**稳定、有限**的 JSON 形状。
///
/// # 为什么需要它
///
/// 闭合词表的价值在于**调用方不必解析错误文案**。但 JS / Kotlin / Swift 那侧
/// 拿到的是 JSON —— 没有这个形状，跨语言调用方还是只能看 `message` 字符串猜。
///
/// 字段固定五个：`code`（原始码，可空）、`category`（闭合词表，恒在场）、
/// `retryable`、`retry_after_ms`（仅限流类）与 `message`（人类可读，**只给人看**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorFrame {
    /// 原始错误码（有强类型码族时给，兜底族为空）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// 闭合词表分类（**恒在场**，调用方据此决策）。
    pub category: ErrorCategory,
    /// 是否可重试（分类默认值，具体错误可覆盖）。
    pub retryable: bool,
    /// 限流建议等待（毫秒，仅限流类错误）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    /// 人类可读说明（**不要用来自动分类**）。
    pub message: String,
}

impl ErrorFrame {
    /// 从任意可分类错误构造错误帧。
    pub fn from_classified<E>(error: &E, message: impl Into<String>) -> Self
    where
        E: ClassifyError,
    {
        Self {
            code: None,
            category: error.category(),
            retryable: error.is_retryable(),
            retry_after_ms: error.retry_after_ms(),
            message: message.into(),
        }
    }

    /// `SdkClientError` 的错误帧（带强类型码族里的原始文案）。
    pub fn from_client_error(error: &SdkClientError) -> Self {
        Self::from_classified(error, error.to_string())
    }

    /// `SdkError` 的错误帧（业务错误带 snake_case 原始码）。
    pub fn from_sdk_error(error: &SdkError) -> Self {
        let mut frame = Self::from_classified(error, error.to_string());
        if let SdkError::Business { code, .. } = error {
            frame.code = Some(code.snake_name());
        }
        frame
    }

    /// `CodecError` 的错误帧。
    pub fn from_codec_error(error: &CodecError) -> Self {
        let mut frame = Self::from_classified(error, error.to_string());
        frame.code = Some(error.kind.as_str().to_string());
        frame
    }
}

#[cfg(test)]
mod wiring_tests {
    use super::*;
    use crate::client::SdkClientError;
    use crate::error::{SdkError, SdkErrorCode};
    use crate::frame_codec::CodecErrorKind;

    /// 三个错误族的**每一个变体**都归入闭合词表 —— 没有任何一条落到「未定义」。
    ///
    /// 这条同时守住「新增变体必须想清楚分类」：加了新错误却忘了映射，
    /// 下面的样例数组会缺一项，review 时会显眼。
    #[test]
    fn every_error_variant_maps_into_the_closed_vocabulary() {
        let client_errors = [
            SdkClientError::ToolNotWhitelisted("x".into()),
            SdkClientError::AuthFailed("x".into()),
            SdkClientError::Network("x".into()),
            SdkClientError::Protocol("x".into()),
            SdkClientError::RateLimited(3),
            SdkClientError::QuotaExceeded("x".into()),
            SdkClientError::ToolCallFailed("x".into()),
            SdkClientError::ServerInternal("x".into()),
            SdkClientError::NotImplemented("x".into()),
            SdkClientError::Other("x".into()),
        ];
        for error in &client_errors {
            let category = error.category();
            assert!(
                ErrorCategory::ALL.contains(&category),
                "{error:?} 落到了词表外: {category:?}"
            );
        }

        let codes = [
            SdkErrorCode::Unknown,
            SdkErrorCode::InvalidEnvelope,
            SdkErrorCode::VersionIncompatible,
            SdkErrorCode::NotFound,
            SdkErrorCode::PermissionDenied,
            SdkErrorCode::ToolNotApproved,
            SdkErrorCode::Internal,
            SdkErrorCode::Other("x".into()),
        ];
        for code in &codes {
            assert!(ErrorCategory::ALL.contains(&code.category()), "{code:?}");
        }

        for kind in [
            CodecErrorKind::Oversize,
            CodecErrorKind::Malformed,
            CodecErrorKind::Encode,
            CodecErrorKind::Poisoned,
        ] {
            assert!(ErrorCategory::ALL.contains(&kind.category()), "{kind:?}");
        }
    }

    /// 每类错误的分类**具体值**（评审可逐行推翻，但别悄悄漂移）。
    #[test]
    fn client_error_categories_are_the_documented_ones() {
        assert_eq!(
            SdkClientError::ToolNotWhitelisted("t".into()).category(),
            ErrorCategory::Validation
        );
        assert_eq!(
            SdkClientError::AuthFailed("a".into()).category(),
            ErrorCategory::Authentication
        );
        assert_eq!(
            SdkClientError::Network("n".into()).category(),
            ErrorCategory::Network
        );
        assert_eq!(
            SdkClientError::Protocol("p".into()).category(),
            ErrorCategory::Protocol
        );
        assert_eq!(
            SdkClientError::RateLimited(1).category(),
            ErrorCategory::RateLimited
        );
        assert_eq!(
            SdkClientError::QuotaExceeded("q".into()).category(),
            ErrorCategory::RateLimited
        );
        assert_eq!(
            SdkClientError::ServerInternal("s".into()).category(),
            ErrorCategory::Internal
        );
        // 2026-10-25（owner 裁定）：能力缺口归 `Unsupported`，
        // 不再混进 `Internal`（后者读成「服务端内部错误」，会让人去查服务端）。
        assert_eq!(
            SdkClientError::NotImplemented("n".into()).category(),
            ErrorCategory::Unsupported
        );
        assert!(
            !SdkClientError::NotImplemented("n".into()).is_retryable(),
            "未实现的能力重试永远还是同一个结果 ⇒ 不可重试"
        );
        assert_eq!(
            SdkClientError::Other("o".into()).category(),
            ErrorCategory::Internal
        );
    }

    /// **重试策略的三处显式覆盖**（这是本模块最容易搞错的地方）。
    #[test]
    fn retry_policy_matches_reality_not_the_default_table() {
        // 限流：可重试 + 带建议等待（秒 → 毫秒）
        let limited = SdkClientError::RateLimited(3);
        assert!(limited.is_retryable());
        assert_eq!(limited.retry_after_ms(), Some(3_000));

        // 配额耗尽：分类是限流，但**不可重试**（等一会儿不会变好）
        let quota = SdkClientError::QuotaExceeded("q".into());
        assert_eq!(quota.category(), ErrorCategory::RateLimited);
        assert!(!quota.is_retryable(), "配额得改套餐，不是退避能解决的");
        assert_eq!(quota.retry_after_ms(), None);

        // 500：分类是 Internal（默认不可重试），但**覆盖为可重试**
        let server = SdkClientError::ServerInternal("s".into());
        assert_eq!(server.category(), ErrorCategory::Internal);
        assert!(
            server.is_retryable(),
            "5xx 常是服务端瞬时故障，退避重试是标准做法"
        );

        // STUB：重试永远是同一个 501
        assert!(!SdkClientError::NotImplemented("n".into()).is_retryable());

        // 网络错误跟随分类默认（可重试）
        assert!(SdkClientError::Network("n".into()).is_retryable());
        // 编码器熔断必须重连，重试无用
        assert!(!CodecErrorKind::Poisoned.is_retryable());
        assert_eq!(CodecErrorKind::Poisoned.category(), ErrorCategory::State);
    }

    /// 跨语言错误帧：`category` / `retryable` **恒在场**，
    /// `retry_after_ms` 只在限流时出现（不给调用方塞 `null` 噪音）。
    #[test]
    fn error_frame_is_stable_across_languages() {
        let frame = ErrorFrame::from_client_error(&SdkClientError::RateLimited(2));
        assert_eq!(frame.category, ErrorCategory::RateLimited);
        assert!(frame.retryable);
        assert_eq!(frame.retry_after_ms, Some(2_000));
        let json = serde_json::to_value(&frame).expect("serialize");
        assert_eq!(json["category"], "rate_limited", "词表用 snake_case");
        assert_eq!(json["retry_after_ms"], 2_000);
        assert_eq!(json["retryable"], true);

        // 非限流错误不出现 retry_after_ms 键
        let plain = ErrorFrame::from_client_error(&SdkClientError::AuthFailed("a".into()));
        let json = serde_json::to_value(&plain).expect("serialize");
        assert!(
            json.get("retry_after_ms").is_none(),
            "非限流不该带该字段: {json}"
        );
        assert_eq!(json["category"], "authentication");

        // 业务错误带 snake_case 原始码
        let business = ErrorFrame::from_sdk_error(&SdkError::business(
            SdkErrorCode::VersionIncompatible,
            "wire v1 vs v2",
        ));
        assert_eq!(
            business.code.as_deref(),
            Some("version_incompatible"),
            "帧里的原始码用 snake_case（与 ErrorCategory 同一命名口径）"
        );
        assert_eq!(business.category, ErrorCategory::Protocol);
        assert!(!business.retryable);
    }

    /// 编解码错误的帧带稳定的 kind 串。
    #[test]
    fn codec_error_frame_carries_the_poisoned_kind() {
        let error = crate::frame_codec::CodecError::new(CodecErrorKind::Poisoned, "byte 12");
        let frame = ErrorFrame::from_codec_error(&error);
        assert_eq!(frame.code.as_deref(), Some("poisoned"));
        assert_eq!(frame.category, ErrorCategory::State);
        assert!(!frame.retryable);
    }
}
