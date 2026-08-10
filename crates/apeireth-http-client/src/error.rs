//! Error types for `apeireth-http-client`.
//!
//! 简化错误模型: 4 大类 (Client build / Request / Pool exhausted / Invalid config),
//! 不用 thiserror 大杂烩, 每个错误带 VCP 对应字段或行号注释.

use std::fmt;

/// 客户端错误 —— `apeireth-http-client` 顶层错误类型.
#[derive(Debug)]
pub enum HttpClientError {
    /// reqwest::Client 构建失败 (配置非法 / TLS 初始化失败)
    ClientBuild(String),
    /// 请求执行失败 (网络 / 超时 / DNS)
    Request(String),
    /// 池已耗尽 (`max_sockets` 限流 + Semaphore 拿不到 permit)
    PoolExhausted {
        /// 上限
        max: usize,
    },
    /// 非法配置 (字段级校验失败, 编译期应挡住, 但运行时兜底)
    InvalidConfig(String),
}

impl fmt::Display for HttpClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpClientError::ClientBuild(msg) => write!(f, "client build failed: {msg}"),
            HttpClientError::Request(msg) => write!(f, "request failed: {msg}"),
            HttpClientError::PoolExhausted { max } => {
                write!(f, "LIFO pool exhausted (max_sockets={max})")
            }
            HttpClientError::InvalidConfig(msg) => write!(f, "invalid config: {msg}"),
        }
    }
}

impl std::error::Error for HttpClientError {}

impl From<reqwest::Error> for HttpClientError {
    fn from(err: reqwest::Error) -> Self {
        HttpClientError::Request(err.to_string())
    }
}

/// 顶层 Result 别名.
pub type Result<T> = std::result::Result<T, HttpClientError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages_are_stable() {
        // 错误信息是契约的一部分 (下游可能 match), 守住字面量
        assert_eq!(
            HttpClientError::ClientBuild("tls".to_string()).to_string(),
            "client build failed: tls"
        );
        assert_eq!(
            HttpClientError::Request("timeout".to_string()).to_string(),
            "request failed: timeout"
        );
        assert_eq!(
            HttpClientError::PoolExhausted { max: 5 }.to_string(),
            "LIFO pool exhausted (max_sockets=5)"
        );
        assert_eq!(
            HttpClientError::InvalidConfig("bad".to_string()).to_string(),
            "invalid config: bad"
        );
    }

    #[test]
    fn reqwest_error_conversion() {
        // reqwest 错误归一到 Request 变体, 不丢信息
        let dummy_url = "http://[::1]:bad-port";
        let reqwest_err = reqwest::Client::new()
            .get(dummy_url)
            .build()
            .expect_err("invalid URL should fail to build");
        let our_err: HttpClientError = reqwest_err.into();
        match our_err {
            HttpClientError::Request(msg) => assert!(!msg.is_empty()),
            other => panic!("expected Request variant, got {other:?}"),
        }
    }
}
