//! 可插拔中间件 (Week 1 MVP)
//!
//! Week 1 实装:
//! - `LoggingMiddleware` — tracing 日志包裹
//! - `RetryMiddleware` — retry 策略 (限 retryable 错误)
//!
//! Week 2+:
//! - `MetricsMiddleware` — 用量统计 + 指标导出
//! - `CacheMiddleware` — 同请求复用响应
//! - `RateLimitMiddleware` — 限流 (token bucket)

use std::sync::Arc;

use async_trait::async_trait;

use crate::llm::error::LlmError;
use crate::llm::traits::{LlmProvider, LlmRequest, LlmResponse};

// ============================================================
// Middleware trait
// ============================================================

#[async_trait]
pub trait Middleware: Send + Sync {
    /// 中间件名 (用于日志)
    fn name(&self) -> &str;

    /// 在调用前 / 调用后包裹 provider
    async fn wrap(
        &self,
        provider: Arc<dyn LlmProvider>,
        req: LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        provider.complete(req).await
    }
}

// ============================================================
// LoggingMiddleware
// ============================================================

pub struct LoggingMiddleware;

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    fn name(&self) -> &str {
        "logging"
    }

    async fn wrap(
        &self,
        provider: Arc<dyn LlmProvider>,
        req: LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        let start = std::time::Instant::now();
        let provider_name = provider.name().to_string();
        let model = req.model.clone();
        let messages_count = req.messages.len();
        let max_tokens = req.max_tokens;

        tracing::info!(
            middleware = "logging",
            provider = %provider_name,
            model = %model,
            messages_count,
            max_tokens,
            "request.start"
        );

        let result = provider.complete(req).await;

        let elapsed_ms = start.elapsed().as_millis() as u64;
        match &result {
            Ok(resp) => {
                tracing::info!(
                    middleware = "logging",
                    provider = %resp.provider,
                    model = %resp.model,
                    latency_ms = elapsed_ms,
                    total_tokens = resp.usage.total_tokens,
                    content_len = resp.content.len(),
                    finish_reason = %resp.finish_reason,
                    "request.success"
                );
            }
            Err(e) => {
                tracing::warn!(
                    middleware = "logging",
                    provider = %provider_name,
                    elapsed_ms,
                    error = %e,
                    "request.fail"
                );
            }
        }
        result
    }
}

// ============================================================
// RetryMiddleware
// ============================================================

pub struct RetryMiddleware {
    pub max_retries: u32,
    pub backoff_base_ms: u64,
}

impl RetryMiddleware {
    pub fn new(max_retries: u32, backoff_base_ms: u64) -> Self {
        Self {
            max_retries,
            backoff_base_ms,
        }
    }
}

impl Default for RetryMiddleware {
    fn default() -> Self {
        Self::new(3, 500)
    }
}

#[async_trait]
impl Middleware for RetryMiddleware {
    fn name(&self) -> &str {
        "retry"
    }

    async fn wrap(
        &self,
        provider: Arc<dyn LlmProvider>,
        req: LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        let mut last_err: Option<LlmError> = None;
        for attempt in 0..=self.max_retries {
            match provider.complete(req.clone()).await {
                Ok(resp) => return Ok(resp),
                Err(e) if e.is_retryable() && attempt < self.max_retries => {
                    let backoff = e.suggested_backoff();
                    let backoff_ms = (backoff.as_millis() as u64)
                        .max(self.backoff_base_ms * (1 << attempt.min(5)));
                    tracing::warn!(
                        middleware = "retry",
                        attempt,
                        backoff_ms,
                        error = %e,
                        "retry.wait"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                    last_err = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err.unwrap_or_else(|| LlmError::ProviderExhausted {
            provider: provider.name().to_string(),
            attempts: self.max_retries + 1,
            last_error: None,
        }))
    }
}

// ============================================================
// MiddlewareChain
// ============================================================

/// 多个中间件按顺序包裹 provider
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn with(mut self, m: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(m);
        self
    }

    pub fn len(&self) -> usize {
        self.middlewares.len()
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    /// 顺序应用所有中间件
    pub async fn run(
        &self,
        provider: Arc<dyn LlmProvider>,
        req: LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        let mut current: Arc<dyn LlmProvider> = provider;
        for m in &self.middlewares {
            let m_clone = m.clone();
            let p_clone = current.clone();
            // 中间件链: m1 → m2 → m3 → provider
            // 每次 wrap 把当前 provider 包进中间件
            current = Arc::new(WrappedProvider::new(p_clone, m_clone));
        }
        current.complete(req).await
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// 内部: 中间件包装的 provider
struct WrappedProvider {
    inner: Arc<dyn LlmProvider>,
    middleware: Arc<dyn Middleware>,
}

impl WrappedProvider {
    fn new(inner: Arc<dyn LlmProvider>, middleware: Arc<dyn Middleware>) -> Self {
        Self { inner, middleware }
    }
}

#[async_trait]
impl LlmProvider for WrappedProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }
    fn supports_model(&self, model: &str) -> bool {
        self.inner.supports_model(model)
    }
    fn capabilities(&self) -> super::traits::ProviderCapabilities {
        self.inner.capabilities()
    }

    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.middleware.wrap(self.inner.clone(), req).await
    }
}

#[cfg(test)]
mod tests {
    use crate::llm::middleware::Middleware;
    use crate::llm::providers::scripted::{ScriptedLlmProvider, ScriptedResponse};
    use crate::llm::traits::ChatMessage;
    use crate::llm::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_logging_middleware() {
        let provider = Arc::new(ScriptedLlmProvider::new("p")) as Arc<dyn LlmProvider>;
        let logging = Arc::new(LoggingMiddleware::new());
        let req = LlmRequest::new("m", vec![ChatMessage::user("hi")]);
        let resp = logging.wrap(provider, req).await.unwrap();
        assert_eq!(
            resp.content,
            "[ScriptedLlmProvider] 默认响应 — 无关键词命中"
        );
    }

    #[tokio::test]
    async fn test_retry_middleware_no_retry_needed() {
        let provider = Arc::new(ScriptedLlmProvider::new("p")) as Arc<dyn LlmProvider>;
        let retry = Arc::new(RetryMiddleware::default());
        let req = LlmRequest::new("m", vec![ChatMessage::user("hi")]);
        let resp = retry.wrap(provider, req).await.unwrap();
        assert!(!resp.content.is_empty());
    }

    #[tokio::test]
    async fn test_middleware_chain() {
        let provider = Arc::new(
            ScriptedLlmProvider::new("p").with_script("hi", ScriptedResponse::new("matched")),
        ) as Arc<dyn LlmProvider>;
        let chain = MiddlewareChain::new()
            .with(Arc::new(LoggingMiddleware::new()))
            .with(Arc::new(RetryMiddleware::default()));
        let req = LlmRequest::new("m", vec![ChatMessage::user("hi")]);
        let resp = chain.run(provider, req).await.unwrap();
        assert_eq!(resp.content, "matched");
        assert_eq!(chain.len(), 2);
    }
}
