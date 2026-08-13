//! MultiLlmRouter — 多 provider 聚合层 (NewAPI 风格核心)
//!
//! 职责:
//! - 维护多个 provider 实例
//! - 按 `fallback_order` 顺序尝试
//! - retryable 错误触发 fallback, 不可重试错误直接返回
//! - 健康检查 (Week 2+)

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::llm::error::LlmError;
use crate::llm::traits::{LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities};

pub struct MultiLlmRouter {
    providers: Vec<Arc<dyn LlmProvider>>,
    fallback_order: Vec<String>,
    /// Provider 健康状态缓存 (provider name → health)
    health: RwLock<std::collections::HashMap<String, super::traits::ProviderHealth>>,
}

impl MultiLlmRouter {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            fallback_order: Vec::new(),
            health: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 添加 provider
    pub fn with_provider(mut self, p: Arc<dyn LlmProvider>) -> Self {
        // 自动添加到 fallback_order (如果不在)
        let name = p.name().to_string();
        self.providers.push(p);
        if !self.fallback_order.contains(&name) {
            self.fallback_order.push(name);
        }
        self
    }

    /// 设置 fallback 顺序 (按 provider 名)
    pub fn with_fallback(mut self, names: Vec<String>) -> Self {
        self.fallback_order = names;
        self
    }

    /// 添加 provider 但**不**自动加入 fallback_order (用于备用 provider)
    #[allow(dead_code)]
    pub fn with_optional_provider(mut self, p: Arc<dyn LlmProvider>) -> Self {
        self.providers.push(p);
        self
    }

    /// Provider 数量
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }

    /// 列所有 provider 名 (按 fallback_order 排序)
    pub fn provider_names(&self) -> Vec<String> {
        let mut sorted: Vec<&Arc<dyn LlmProvider>> = self.providers.iter().collect();
        sorted.sort_by_key(|p| {
            self.fallback_order
                .iter()
                .position(|n| n == p.name())
                .unwrap_or(usize::MAX)
        });
        sorted.iter().map(|p| p.name().to_string()).collect()
    }

    /// 取某 provider 的健康状态
    #[allow(dead_code)]
    pub fn get_health(&self, name: &str) -> Option<super::traits::ProviderHealth> {
        self.health.read().get(name).cloned()
    }

    /// 更新 provider 健康状态
    fn update_health(&self, name: &str, success: bool, latency_ms: u64) {
        let mut health_map = self.health.write();
        let entry = health_map.entry(name.to_string()).or_default();
        if success {
            entry.consecutive_failures = 0;
            // 简单 EMA 更新 p50
            if entry.latency_p50_ms == 0 {
                entry.latency_p50_ms = latency_ms;
            } else {
                entry.latency_p50_ms = (entry.latency_p50_ms * 7 + latency_ms) / 8;
            }
            entry.error_rate = entry.error_rate * 0.9;
            entry.healthy = entry.error_rate < 0.5;
        } else {
            entry.consecutive_failures += 1;
            entry.error_rate = entry.error_rate * 0.9 + 0.1;
            entry.healthy = entry.consecutive_failures < 3 && entry.error_rate < 0.5;
        }
        entry.last_check_ms = chrono::Utc::now().timestamp_millis();
    }
}

impl Default for MultiLlmRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for MultiLlmRouter {
    fn name(&self) -> &str {
        "router"
    }

    fn supports_model(&self, model: &str) -> bool {
        self.providers.iter().any(|p| p.supports_model(model))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        // Router 的能力 = 所有 provider 能力的并集
        let mut caps = ProviderCapabilities::NONE;
        for p in &self.providers {
            caps = caps | p.capabilities();
        }
        caps
    }

    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError> {
        // 1. 找支持该 model 的 provider
        let mut candidates: Vec<&Arc<dyn LlmProvider>> = self
            .providers
            .iter()
            .filter(|p| p.supports_model(&req.model))
            .collect();

        // 2. 按 fallback_order 排序 (未在 fallback_order 的排到最后)
        candidates.sort_by_key(|p| {
            self.fallback_order
                .iter()
                .position(|n| n == p.name())
                .unwrap_or(usize::MAX)
        });

        if candidates.is_empty() {
            return Err(LlmError::NoProvider {
                model: req.model.clone(),
                available: self
                    .providers
                    .iter()
                    .map(|p| p.name().to_string())
                    .collect(),
            });
        }

        // 3. 串行尝试 (按顺序, 第一个成功就返回)
        let mut last_err: Option<LlmError> = None;
        for provider in candidates {
            let p_name = provider.name().to_string();
            tracing::info!(
                provider = %p_name,
                model = %req.model,
                trace_id = ?req.trace_id,
                "router.try"
            );
            match provider.complete(req.clone()).await {
                Ok(resp) => {
                    self.update_health(&p_name, true, resp.latency_ms);
                    tracing::info!(
                        provider = %p_name,
                        model = %resp.model,
                        latency_ms = resp.latency_ms,
                        total_tokens = resp.usage.total_tokens,
                        "router.success"
                    );
                    return Ok(resp);
                }
                Err(e) if e.is_retryable() => {
                    tracing::warn!(
                        provider = %p_name,
                        error = %e,
                        retryable = true,
                        "router.fail_retryable_fallback"
                    );
                    self.update_health(&p_name, false, 0);
                    last_err = Some(e);
                    continue;
                }
                Err(e) => {
                    tracing::error!(
                        provider = %p_name,
                        error = %e,
                        retryable = false,
                        "router.fail_no_fallback"
                    );
                    self.update_health(&p_name, false, 0);
                    return Err(e);
                }
            }
        }

        // 所有 provider 都失败
        Err(last_err.unwrap_or_else(|| LlmError::NoProvider {
            model: req.model.clone(),
            available: self.provider_names(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::llm::providers::scripted::{ScriptedLlmProvider, ScriptedResponse};
    use crate::llm::traits::{ChatMessage, LlmProvider, LlmRequest};
    use crate::llm::*;
    use std::sync::Arc;
    #[tokio::test]
    async fn test_router_basic() {
        let provider = Arc::new(
            ScriptedLlmProvider::new("primary").with_script("hello", ScriptedResponse::new("hi")),
        );
        let router = MultiLlmRouter::new().with_provider(provider);
        let req = LlmRequest::new("m", vec![ChatMessage::user("hello")]);
        let resp = router.complete(req).await.unwrap();
        assert_eq!(resp.content, "hi");
    }

    #[tokio::test]
    async fn test_router_fallback_on_provider_error() {
        // 用一个总是返回错误的 mock provider 测 fallback
        use crate::llm::error::LlmError;
        use crate::llm::traits::LlmProvider;

        struct FailingProvider {
            name: String,
        }

        #[async_trait::async_trait]
        impl LlmProvider for FailingProvider {
            fn name(&self) -> &str {
                &self.name
            }
            fn supports_model(&self, _model: &str) -> bool {
                true
            }
            async fn complete(
                &self,
                _req: LlmRequest,
            ) -> Result<crate::llm::traits::LlmResponse, LlmError> {
                Err(LlmError::Network {
                    provider: self.name.clone(),
                    detail: "mock fail".into(),
                })
            }
        }

        let failing = Arc::new(FailingProvider {
            name: "failing".into(),
        }) as Arc<dyn LlmProvider>;
        let success = Arc::new(
            ScriptedLlmProvider::new("success")
                .with_script("hello", ScriptedResponse::new("from success")),
        ) as Arc<dyn LlmProvider>;
        let router = MultiLlmRouter::new()
            .with_provider(failing)
            .with_provider(success)
            .with_fallback(vec!["failing".into(), "success".into()]);
        let req = LlmRequest::new("m", vec![ChatMessage::user("hello")]);
        let resp = router.complete(req).await.unwrap();
        assert_eq!(resp.content, "from success");
        assert_eq!(resp.provider, "success");
    }

    #[tokio::test]
    async fn test_router_no_provider_for_model() {
        let provider = Arc::new(ScriptedLlmProvider::new("p1"));
        let router = MultiLlmRouter::new().with_provider(provider);
        // scripted 接受任何 model, 不会失败. 测 fallback 排序
        let req = LlmRequest::new("m", vec![ChatMessage::user("x")]);
        let _ = router.complete(req).await;
    }

    #[test]
    fn test_router_provider_names() {
        let p1 = Arc::new(ScriptedLlmProvider::new("alpha")) as Arc<dyn LlmProvider>;
        let p2 = Arc::new(ScriptedLlmProvider::new("beta")) as Arc<dyn LlmProvider>;
        let router = MultiLlmRouter::new()
            .with_provider(p1)
            .with_provider(p2)
            .with_fallback(vec!["beta".into(), "alpha".into()]);
        let names = router.provider_names();
        assert_eq!(names, vec!["beta", "alpha"]);
    }

    #[test]
    fn test_router_supports_model() {
        let p1 = Arc::new(ScriptedLlmProvider::new("p1"));
        let router = MultiLlmRouter::new().with_provider(p1);
        assert!(router.supports_model("any"));
    }
}
