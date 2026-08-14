//! R269: Multi-model advisor backend for council deliberation.
//!
//! 接受多个 LlmProvider, 每次 generate 在所有 provider 上发请求, 聚合响应.
//! 默认策略 = first-non-empty (fallback chain). 也可配置为 longest-response
//! (选出最长 text, 适合"哪个 model 最有信息"对比场景).
//!
//! 公开 API:
//! - `MultiModelAdvisorBackend::new(backends)` — 构造, 接受 Vec<LlmProvider>
//! - `MultiModelAdvisorBackend::with_strategy(backends, strategy)` — 选聚合策略
//! - `AggregationStrategy` — FirstNonEmpty / Longest / ConcatAll
//!
//! 设计: 复用 MockLlmProvider trait (跟 LlmAdvisorBackend 一致), 让 council 直接接.
//! 0 引入外部 dep.

#![allow(missing_docs)]

use std::sync::Arc;

use crate::mock_llm::{MockLlmProvider, MockLlmResponse};
use apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest};

/// R269: 多模型响应聚合策略.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationStrategy {
    /// 第一个非空响应 (fallback chain). 默认.
    FirstNonEmpty,
    /// 最长 text 的响应 (假设最长=最有信息).
    Longest,
    /// 拼接所有非空响应, 用 "\n--\n" 分隔.
    ConcatAll,
}

/// R269: MultiModelAdvisorBackend — 跨多 LLM 决策聚合.
///
/// 持有 1+ 个 LlmProvider, 调 generate 时在所有 provider 上发请求,
/// 用 AggregationStrategy 聚合响应. 真 LLM 路径, 跟 LlmAdvisorBackend 一致
/// (block_on Handle::current 调 async LlmProvider).
pub struct MultiModelAdvisorBackend {
    backends: Vec<Arc<dyn LlmProvider>>,
    strategy: AggregationStrategy,
}

impl MultiModelAdvisorBackend {
    /// 构造: 默认 FirstNonEmpty 策略
    pub fn new(backends: Vec<Arc<dyn LlmProvider>>) -> Self {
        Self { backends, strategy: AggregationStrategy::FirstNonEmpty }
    }

    /// 构造: 自定义策略
    pub fn with_strategy(backends: Vec<Arc<dyn LlmProvider>>, strategy: AggregationStrategy) -> Self {
        Self { backends, strategy }
    }

    /// 拿 backend 数量
    pub fn backend_count(&self) -> usize {
        self.backends.len()
    }

    /// 拿 strategy
    pub fn strategy(&self) -> AggregationStrategy {
        self.strategy
    }

    /// 内部: 在每个 backend 上跑 LLM, 收集响应
    fn collect_responses(&self, prompt: &str, system: &str) -> Vec<MockLlmResponse> {
        let mut out = Vec::with_capacity(self.backends.len());
        for backend in &self.backends {
            let model = if backend.name() == "apeireth-api" {
                "MiniMax-M3".to_string()
            } else {
                backend.name().to_string()
            };
            let result = match tokio::runtime::Handle::try_current() {
                Ok(handle) => handle.block_on(async {
                    backend.complete(LlmRequest::new(
                        &model,
                        vec![
                            ChatMessage::system(system.to_string()),
                            ChatMessage::user(prompt.to_string()),
                        ],
                    )).await
                }),
                Err(_) => {
                    // No tokio runtime: build a one-shot runtime
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("build runtime");
                    rt.block_on(async {
                        backend.complete(LlmRequest::new(
                            &model,
                            vec![
                                ChatMessage::system(system.to_string()),
                                ChatMessage::user(prompt.to_string()),
                            ],
                        )).await
                    })
                }
            };
            match result {
                Ok(resp) => {
                    let text = resp.content.clone();
                    let triggers_hold = text.to_lowercase().contains("reject") || text.to_lowercase().contains("hold");
                    out.push(MockLlmResponse {
                        text,
                        triggers_hold,
                        confidence: 0.7,
                    });
                }
                Err(_) => {
                    // skip failed backend (graceful degradation)
                }
            }
        }
        out
    }
}

#[allow(deprecated)]
impl MockLlmProvider for MultiModelAdvisorBackend {
    fn generate(&self, prompt: &str, system: &str) -> MockLlmResponse {
        let responses = self.collect_responses(prompt, system);
        if responses.is_empty() {
            return MockLlmResponse::ok("[multi-model: all backends failed]");
        }
        match self.strategy {
            AggregationStrategy::FirstNonEmpty => {
                responses.into_iter().find(|r| !r.text.is_empty()).unwrap()
            }
            AggregationStrategy::Longest => {
                responses.into_iter().max_by_key(|r| r.text.len()).unwrap()
            }
            AggregationStrategy::ConcatAll => {
                let joined: Vec<String> = responses.iter().map(|r| r.text.clone()).filter(|s| !s.is_empty()).collect();
                let combined = joined.join("
--
");
                MockLlmResponse::ok(combined)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_api::llm::{LlmError, LlmResponse, TokenUsage};

    fn make_response(provider: &str, content: &str) -> LlmResponse {
        LlmResponse {
            content: content.to_string(),
            usage: TokenUsage::default(),
            latency_ms: 0,
            model: provider.to_string(),
            finish_reason: "stop".to_string(),
            provider: provider.to_string(),
        }
    }

    struct EchoBackend(&'static str);
    #[async_trait::async_trait]
    impl LlmProvider for EchoBackend {
        fn name(&self) -> &str { self.0 }
        fn supports_model(&self, _model: &str) -> bool { true }
        async fn complete(&self, _req: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(make_response(self.0, &format!("[echo:{}] hi", self.0)))
        }
    }

    struct FailBackend;
    #[async_trait::async_trait]
    impl LlmProvider for FailBackend {
        fn name(&self) -> &str { "fail" }
        fn supports_model(&self, _model: &str) -> bool { true }
        async fn complete(&self, _req: LlmRequest) -> Result<LlmResponse, LlmError> {
            Err(LlmError::Network { provider: "fail".to_string(), detail: "simulated failure".to_string() })
        }
    }

    struct LongBackend;
    #[async_trait::async_trait]
    impl LlmProvider for LongBackend {
        fn name(&self) -> &str { "long" }
        fn supports_model(&self, _model: &str) -> bool { true }
        async fn complete(&self, _req: LlmRequest) -> Result<LlmResponse, LlmError> {
            Ok(make_response("long", "this is a longer response with more info"))
        }
    }

    #[test]
    fn r269_01_first_non_empty_strategy_picks_first() {
        let backends: Vec<Arc<dyn LlmProvider>> = vec![
            Arc::new(EchoBackend("a")),
            Arc::new(EchoBackend("b")),
        ];
        let m = MultiModelAdvisorBackend::new(backends);
        assert_eq!(m.backend_count(), 2);
        let r = m.generate("hello", "be terse");
        assert!(r.text.contains("[echo:a]"), "got: {}", r.text);
        assert!(!r.triggers_hold);
    }

    #[test]
    fn r269_02_longest_strategy_picks_longest() {
        let backends: Vec<Arc<dyn LlmProvider>> = vec![
            Arc::new(EchoBackend("short")),
            Arc::new(LongBackend),
        ];
        let m = MultiModelAdvisorBackend::with_strategy(backends, AggregationStrategy::Longest);
        let r = m.generate("hello", "be terse");
        assert!(r.text.starts_with("this is"), "got: {}", r.text);
    }

    #[test]
    fn r269_03_concat_all_strategy_joins_with_separator() {
        let backends: Vec<Arc<dyn LlmProvider>> = vec![
            Arc::new(EchoBackend("a")),
            Arc::new(EchoBackend("b")),
        ];
        let m = MultiModelAdvisorBackend::with_strategy(backends, AggregationStrategy::ConcatAll);
        let r = m.generate("hello", "be terse");
        assert!(r.text.contains("[echo:a]"));
        assert!(r.text.contains("[echo:b]"));
        assert!(r.text.contains("\n--\n"));
    }

    #[test]
    fn r269_04_failed_backend_skipped_gracefully() {
        let backends: Vec<Arc<dyn LlmProvider>> = vec![
            Arc::new(FailBackend),
            Arc::new(EchoBackend("alive")),
        ];
        let m = MultiModelAdvisorBackend::new(backends);
        let r = m.generate("hello", "be terse");
        assert!(r.text.contains("[echo:alive]"), "got: {}", r.text);
    }

    #[test]
    fn r269_05_all_failed_returns_placeholder() {
        let backends: Vec<Arc<dyn LlmProvider>> = vec![
            Arc::new(FailBackend),
            Arc::new(FailBackend),
        ];
        let m = MultiModelAdvisorBackend::new(backends);
        let r = m.generate("hello", "be terse");
        assert!(r.text.contains("all backends failed"), "got: {}", r.text);
    }

    #[test]
    fn r269_06_strategy_accessor() {
        let backends: Vec<Arc<dyn LlmProvider>> = vec![Arc::new(EchoBackend("a"))];
        let m = MultiModelAdvisorBackend::new(backends);
        assert_eq!(m.strategy(), AggregationStrategy::FirstNonEmpty);
    }
}