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
    /// 内部错误 / 兜底.
    Internal,
}

impl ErrorCategory {
    /// 分类数量 (闭合词表规模, 测试钉死).
    pub const COUNT: usize = 9;

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
    fn category_table_is_closed_at_9() {
        assert_eq!(ErrorCategory::COUNT, 9);
        assert_eq!(ErrorCategory::ALL.len(), 9);
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
