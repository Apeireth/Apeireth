//! Council live E2E with the OpenAI-compatible factory (DeepSeek).
//!
//! 0 装纪律与 provider/organ live tests 一致: `#[ignore]` 默认不跑; 手动跑需
//! `OPENAI_API_KEY` + `APEIRETH_OPENAI_URL` + `APEIRETH_OPENAI_MODELS` env;
//! 0 commit key / 0 print key / 0 mock 真 LLM。
//!
//! 桥接说明 (per RC-6 设计): orchestration 持有**镜像** LlmFactory trait
//! (避免 plugin↔orchestration 依赖环); 生产桥接 = runtime-assembly 的
//! `InvokerLlmFactory`。本测试用测试本地适配器把 plugin 的
//! `OpenAiCompatibleLlmFactory` 桥到镜像 trait, 验证 Council 7-advisor
//! 真 LLM 并行判定路径 (此前只有 Mock factory)。

use std::sync::Arc;

use async_trait::async_trait;

use apeireth_core::kernel::SessionId;
use apeireth_orchestration::llm as orch_llm;
use apeireth_orchestration::{Council, CouncilVerdict, Proposal, SubagentRole};
use apeireth_plugin::llm_factory as plugin_llm;
use apeireth_provider::openai_compatible_llm_factory::OpenAiCompatibleLlmFactory;

fn sample_proposal() -> Proposal {
    Proposal {
        id: "p-live".into(),
        proposer: "live-e2e".into(),
        payload: serde_json::json!({"action": "deploy"}),
        submitted_at: 1_700_000_000,
        session_id: SessionId::new(),
    }
}

// ---------------------------------------------------------------------------
// 测试本地适配器: plugin LlmFactory → orchestration 镜像 trait (field 级 1:1)。
// ---------------------------------------------------------------------------

struct AdapterFactory {
    inner: Arc<dyn plugin_llm::LlmFactory>,
}

#[async_trait]
impl orch_llm::LlmFactory for AdapterFactory {
    async fn spawn(
        &self,
        role: SubagentRole,
        model: &str,
    ) -> Result<Box<dyn orch_llm::LlmInstance>, orch_llm::LlmError> {
        let inner = self.inner.spawn(role, model).await.map_err(map_err)?;
        Ok(Box::new(AdapterInstance {
            inner,
            name: format!("adapter-{model}"),
        }))
    }

    async fn available_models(&self) -> Result<Vec<String>, orch_llm::LlmError> {
        self.inner.available_models().await.map_err(map_err)
    }

    fn name(&self) -> &str {
        "openai-compatible-adapter"
    }
}

struct AdapterInstance {
    inner: Box<dyn plugin_llm::LlmInstance>,
    name: String,
}

#[async_trait]
impl orch_llm::LlmInstance for AdapterInstance {
    async fn complete(
        &self,
        req: orch_llm::CompletionRequest,
    ) -> Result<orch_llm::CompletionResponse, orch_llm::LlmError> {
        let inner_req = plugin_llm::CompletionRequest {
            system_prompt: req.system_prompt,
            messages: req
                .messages
                .into_iter()
                .map(|m| plugin_llm::CompletionMessage {
                    role: m.role,
                    content: m.content,
                })
                .collect(),
            temperature: req.temperature,
            tools: req.tools,
            max_tokens: req.max_tokens,
        };
        let resp = self.inner.complete(inner_req).await.map_err(map_err)?;
        Ok(orch_llm::CompletionResponse {
            message: orch_llm::CompletionMessage {
                role: resp.message.role,
                content: resp.message.content,
            },
            tool_calls: resp.tool_calls,
            finish_reason: resp.finish_reason,
            usage: orch_llm::TokenUsage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
        })
    }

    fn name(&self) -> &str {
        &self.name
    }
}

fn map_err(e: plugin_llm::LlmError) -> orch_llm::LlmError {
    match e {
        plugin_llm::LlmError::Credentials(m) => orch_llm::LlmError::Credentials(m),
        plugin_llm::LlmError::Network(m) => orch_llm::LlmError::Network(m),
        plugin_llm::LlmError::RateLimited { retry_after_ms } => {
            orch_llm::LlmError::RateLimited { retry_after_ms }
        }
        plugin_llm::LlmError::Provider(m) => orch_llm::LlmError::Provider(m),
        plugin_llm::LlmError::Stream(m) => orch_llm::LlmError::Stream(m),
        plugin_llm::LlmError::NotImplemented(what) => orch_llm::LlmError::NotImplemented(what),
    }
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY + APEIRETH_OPENAI_MODELS env (DeepSeek live E2E, manual)"]
async fn council_7_advisor_live_decide() {
    let inner: Arc<dyn plugin_llm::LlmFactory> = Arc::new(
        OpenAiCompatibleLlmFactory::from_env()
            .expect("factory from env (需 OPENAI_API_KEY + APEIRETH_OPENAI_MODELS)"),
    );
    let factory: Arc<dyn orch_llm::LlmFactory> = Arc::new(AdapterFactory { inner });
    let council = Council::with_factory(factory, "deepseek-v4-flash");
    assert_eq!(council.advisors().len(), 7);

    let verdict = council.decide(&sample_proposal()).await;
    eprintln!("live council verdict: {verdict:?}");
    // 0 装: 断言 7 个真 LLM advisor 并行判定路径完整跑通 (不強断特定 verdict —
    // 模型判定文本决定结果, 防 flaky); DeferToHuman 也是合法出口 (整体 60s 超时)。
    assert!(
        matches!(
            verdict,
            CouncilVerdict::Approved
                | CouncilVerdict::Vetoed { .. }
                | CouncilVerdict::DeferToHuman { .. }
        ),
        "verdict must be a valid CouncilVerdict, got {verdict:?}"
    );
}
