//! Plugin `LlmFactory` → orchestration 镜像 trait 的适配器（生产桥）。
//!
//! 为什么存在（per `apeireth_orchestration::llm` 模块注释）: orchestration 持有
//! **镜像** LlmFactory trait（plugin 已依赖 orchestration 的 `SubagentRole`,
//! 反依赖成环）。生产桥接由 composition root 负责——本模块把 provider 的
//! plugin trait 工厂（MiniMax / OpenAI-compatible / 任意）桥到镜像 trait,
//! 供 `Council::with_factory` 消费。字段级 1:1 映射, 0 语义转换。
//! 2026-09-08 (用户旋钮批): 与 tests/council_live.rs 的测试适配器同构,
//! 但位于生产装配层（composition root 是桥的主人）。

use std::sync::Arc;

use async_trait::async_trait;

use apeireth_orchestration::llm as orch_llm;
use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory as plugin_llm;

/// 桥工厂: 持 plugin trait 工厂, 实现 orchestration 镜像 trait。
pub struct MirrorLlmFactory {
    pub inner: Arc<dyn plugin_llm::LlmFactory>,
}

#[async_trait]
impl orch_llm::LlmFactory for MirrorLlmFactory {
    async fn spawn(
        &self,
        role: SubagentRole,
        model: &str,
    ) -> Result<Box<dyn orch_llm::LlmInstance>, orch_llm::LlmError> {
        let inner = self.inner.spawn(role, model).await.map_err(map_err)?;
        Ok(Box::new(MirrorLlmInstance {
            inner,
            name: format!("mirror-{model}"),
        }))
    }

    async fn available_models(&self) -> Result<Vec<String>, orch_llm::LlmError> {
        self.inner.available_models().await.map_err(map_err)
    }

    fn name(&self) -> &str {
        "mirror"
    }
}

struct MirrorLlmInstance {
    inner: Box<dyn plugin_llm::LlmInstance>,
    name: String,
}

#[async_trait]
impl orch_llm::LlmInstance for MirrorLlmInstance {
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
