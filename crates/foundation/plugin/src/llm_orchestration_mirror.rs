//! Plugin `LlmFactory` → orchestration 镜像 trait 的适配器（O-6 归位）。
//!
//! # 为什么放在 plugin（系统最优）
//! orchestration 持有**镜像** LlmFactory trait（plugin 已依赖 orchestration 的
//! `SubagentRole`，反向依赖成环——见 `apeireth_orchestration::llm` 模块注释）。
//! 两套 trait 之间的桥，只能落在同时可见双方的 crate：
//! - `apeireth-plugin` 依赖 `apeireth-orchestration` ✓ 且拥有 plugin 侧
//!   `llm_factory` 的 trait 本体——**桥归 plugin，单一事实源**；
//! - 之前版本放在 CLI composition root，导致 orchestrator 测试（council_live）
//!   不得不复制一份 field 级适配器——"复制粘贴两处漂移"（O-6 抽象层不重复）。
//! 归位后：CLI / gateway / 测试全部消费同一实现。
//!
//! 映射语义：两套 `CompletionRequest`/`CompletionResponse`/`TokenUsage` 形状
//! 一致，field 级 1:1 搬运，0 语义转换；错误按 variant 一一映射。

use std::sync::Arc;

use async_trait::async_trait;

use apeireth_orchestration::llm as orch_llm;
use apeireth_orchestration::SubagentRole;

use crate::llm_factory as plugin_llm;

/// 桥工厂：持 plugin trait 工厂，实现 orchestration 镜像 trait。
pub struct MirrorLlmFactory {
    inner: Arc<dyn plugin_llm::LlmFactory>,
}

impl MirrorLlmFactory {
    /// 包一个 plugin 侧工厂为镜像 trait 工厂。
    pub fn new(inner: Arc<dyn plugin_llm::LlmFactory>) -> Self {
        Self { inner }
    }
}

impl std::fmt::Debug for MirrorLlmFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MirrorLlmFactory")
            .field("name", &"Arc<dyn plugin LlmFactory>")
            .finish_non_exhaustive()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_factory::{
        CompletionMessage, CompletionRequest, CompletionResponse, LlmError, NoopLlmFactory,
        TokenUsage,
    };
    use apeireth_orchestration::llm::LlmFactory as OrchLlmFactory;
    use apeireth_orchestration::llm::LlmInstance as OrchLlmInstance;

    /// 错误映射: 每个 plugin 变体 → 对应镜像变体.
    #[test]
    fn error_mapping_is_one_to_one() {
        assert!(matches!(
            map_err(plugin_llm::LlmError::Credentials("c".into())),
            orch_llm::LlmError::Credentials(_)
        ));
        assert!(matches!(
            map_err(plugin_llm::LlmError::Network("n".into())),
            orch_llm::LlmError::Network(_)
        ));
        assert!(matches!(
            map_err(plugin_llm::LlmError::RateLimited { retry_after_ms: 9 }),
            orch_llm::LlmError::RateLimited { retry_after_ms: 9 }
        ));
        assert!(matches!(
            map_err(plugin_llm::LlmError::Provider("p".into())),
            orch_llm::LlmError::Provider(_)
        ));
        assert!(matches!(
            map_err(plugin_llm::LlmError::Stream("s".into())),
            orch_llm::LlmError::Stream(_)
        ));
        assert!(matches!(
            map_err(plugin_llm::LlmError::NotImplemented("x")),
            orch_llm::LlmError::NotImplemented("x")
        ));
    }

    /// Noop 工厂经桥后: spawn 成功, complete 显式 NotImplemented (不静默).
    #[tokio::test]
    async fn noop_factory_mirrors_not_implemented() {
        let mirror = MirrorLlmFactory::new(Arc::new(NoopLlmFactory));
        let instance = mirror
            .spawn(SubagentRole::Reviewer, "m")
            .await
            .expect("noop spawn succeeds");
        let req = orch_llm::CompletionRequest {
            system_prompt: String::new(),
            messages: Vec::new(),
            temperature: 1.0,
            tools: Vec::new(),
            max_tokens: None,
        };
        let result = instance.complete(req).await;
        assert!(matches!(result, Err(orch_llm::LlmError::NotImplemented(_))));
    }

    /// 桥实例的 complete 失败透传 (Noop instance 不存在, 用错误路径验证).
    #[tokio::test]
    async fn mirror_factory_name_and_models() {
        let mirror = MirrorLlmFactory::new(Arc::new(NoopLlmFactory));
        assert_eq!(mirror.name(), "mirror");
        assert!(mirror.available_models().await.unwrap().is_empty());
    }

    // 类型形状引用 (编译期锁: 两套 CompletionRequest/Response 字段一一对应).
    #[test]
    fn conversion_shapes_compile_check() {
        let _ = |req: orch_llm::CompletionRequest| {
            let _plugin_req = plugin_llm::CompletionRequest {
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
        };
        let _ = |resp: orch_llm::CompletionResponse| {
            let _plugin_resp = CompletionResponse {
                message: CompletionMessage {
                    role: resp.message.role,
                    content: resp.message.content,
                },
                tool_calls: resp.tool_calls,
                finish_reason: resp.finish_reason,
                usage: TokenUsage {
                    prompt_tokens: resp.usage.prompt_tokens,
                    completion_tokens: resp.usage.completion_tokens,
                    total_tokens: resp.usage.total_tokens,
                },
            };
        };
        let _ = LlmError::NotImplemented("shape");
    }
}
