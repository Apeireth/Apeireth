//! LLM 错误类型 (统一异常分类 + retryable 区分)

use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("auth failed: {0}")]
    AuthFailed(String),

    #[error("rate limited (retry after {retry_after_ms}ms)")]
    RateLimited {
        retry_after_ms: u64,
        provider: String,
    },

    #[error("timeout after {timeout_ms}ms (provider: {provider})")]
    Timeout { timeout_ms: u64, provider: String },

    #[error("bad response from {provider}: {detail}")]
    BadResponse {
        provider: String,
        detail: String,
        status_code: Option<u16>,
    },

    #[error("network error ({provider}): {detail}")]
    Network { provider: String, detail: String },

    #[error("no provider available for model {model}")]
    NoProvider {
        model: String,
        available: Vec<String>,
    },

    #[error("config error: {0}")]
    Config(String),

    #[error("provider {provider} unhealthy after {attempts} attempts")]
    ProviderExhausted {
        provider: String,
        attempts: u32,
        #[source]
        last_error: Option<Box<LlmError>>,
    },
}

impl LlmError {
    /// 该错误是否应该触发 retry 或 fallback
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::RateLimited { .. } | LlmError::Timeout { .. } | LlmError::Network { .. }
        )
    }

    /// 推荐 backoff 时长 (毫秒)
    pub fn suggested_backoff(&self) -> Duration {
        match self {
            LlmError::RateLimited { retry_after_ms, .. } => Duration::from_millis(*retry_after_ms),
            LlmError::Timeout { .. } => Duration::from_millis(1000),
            LlmError::Network { .. } => Duration::from_millis(500),
            _ => Duration::from_millis(0),
        }
    }

    /// 该错误来自哪个 provider (用于 Router 路由日志)
    pub fn provider(&self) -> Option<&str> {
        match self {
            LlmError::AuthFailed(_) => None,
            LlmError::RateLimited { provider, .. } => Some(provider.as_str()),
            LlmError::Timeout { provider, .. } => Some(provider.as_str()),
            LlmError::BadResponse { provider, .. } => Some(provider.as_str()),
            LlmError::Network { provider, .. } => Some(provider.as_str()),
            LlmError::NoProvider { .. } => None,
            LlmError::Config(_) => None,
            LlmError::ProviderExhausted { provider, .. } => Some(provider.as_str()),
        }
    }

    /// HTTP status code (如果适用)
    pub fn status_code(&self) -> Option<u16> {
        match self {
            LlmError::BadResponse { status_code, .. } => *status_code,
            _ => None,
        }
    }
}
