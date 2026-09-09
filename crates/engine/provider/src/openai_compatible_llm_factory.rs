//! RC-5 补全 (2026-09-06): OpenAI-compatible 的 `LlmFactory` 真实现。
//!
//! 至此 9-organ / Council / Orchestrator 的 `LlmFactory` trait 口有了第二个
//! 真 backend——**DeepSeek（或任意 OpenAI-compatible 端点）**。与
//! [`crate::minimax_llm_factory`] 同构：
//! - 共享底层 `OpenAiCompatibleProviderCapability`（单 reqwest client + 单 model
//!   list + 单 credential_key），spawn 时生成独立 instance（multi-instance 隔离 +
//!   single-transport 共享，per scene-d §3）；
//! - 凭证 0 装：capability 每次 `complete` 经 `CredentialResolver` 取
//!   (`provider.openai-compatible.api_key` → env `OPENAI_API_KEY`)，工厂 0 持有 key；
//! - 0 重试 0 fallback（router 拥有 fallback）；
//! - Conversion 与错误映射复用 `llm_factory_adapters`（0 复制粘贴）。
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline;
//! 0 改 trait 边界, 0 改 protocol crate, 0 改 orchestration crate.

use std::sync::Arc;

use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::{
    CompletionRequest, CompletionResponse, LlmError, LlmFactory, LlmInstance,
};
use apeireth_plugin::{CredentialResolver, ProviderCapability};
use async_trait::async_trait;

use crate::canonical_openai_compatible::{
    OpenAiCompatibleProviderCapability, OpenAiCompatibleProviderPlugin,
};
use crate::llm_factory_adapters;

/// 工厂名字 (用于监控 / 日志; per llm_factory.rs:162 "factory 名字")。
pub const FACTORY_NAME: &str = "openai-compatible";

/// 真接 OpenAI-compatible backend (DeepSeek / 任意兼容端点) 的 LlmFactory impl。
pub struct OpenAiCompatibleLlmFactory {
    capability: Arc<OpenAiCompatibleProviderCapability>,
}

impl std::fmt::Debug for OpenAiCompatibleLlmFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 0 装诚实: 0 输出 key; capability 自身 Debug 已 0 装泄漏.
        f.debug_struct("OpenAiCompatibleLlmFactory")
            .field("name", &FACTORY_NAME)
            .finish_non_exhaustive()
    }
}

impl OpenAiCompatibleLlmFactory {
    /// 从已构造好的 capability 包成 factory（0 接 key）。
    pub fn new(capability: Arc<OpenAiCompatibleProviderCapability>) -> Self {
        Self { capability }
    }

    /// 真生产路径: 从 env 构造（`APEIRETH_OPENAI_URL` / `APEIRETH_OPENAI_MODELS`
    /// 配置端点与模型; key 走 `OPENAI_API_KEY` env，per-turn 解析）。
    pub fn from_env() -> Result<Self, FactoryError> {
        Self::from_env_with_resolver(Arc::new(crate::credentials::EnvCredentialResolver::new()))
    }

    /// 显式注入 resolver（Env / Keyring / StaticCredentials 任意实现）。
    pub fn from_env_with_resolver(
        resolver: Arc<dyn CredentialResolver>,
    ) -> Result<Self, FactoryError> {
        let plugin = OpenAiCompatibleProviderPlugin::from_env().map_err(FactoryError::Plugin)?;
        plugin.attach_resolver_for_test(resolver);
        Ok(Self::new(plugin.provider_for_test()))
    }

    /// 该 factory 可服务的 model 列表 (canonical id, 按 capability 配置顺序)。
    pub fn model_ids(&self) -> Vec<String> {
        self.capability
            .models()
            .into_iter()
            .map(|m| m.id.as_str().to_string())
            .collect()
    }

    /// 返回底层 capability (test / 高级用法; **不**应绕过 capability 直接调 HTTP)。
    pub fn capability(&self) -> &Arc<OpenAiCompatibleProviderCapability> {
        &self.capability
    }
}

/// factory 构造错 (0 装诚实: 区分来源, 0 静默)。
#[derive(Debug)]
pub enum FactoryError {
    /// plugin 构造错 (reqwest build 失败 / invalid base url / empty models)。
    Plugin(apeireth_plugin::PluginError),
}

impl std::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plugin(e) => write!(
                f,
                "openai-compatible LlmFactory 构造失败: plugin error: {e}"
            ),
        }
    }
}

impl std::error::Error for FactoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plugin(e) => Some(e),
        }
    }
}

#[async_trait]
impl LlmFactory for OpenAiCompatibleLlmFactory {
    async fn spawn(
        &self,
        role: SubagentRole,
        model: &str,
    ) -> Result<Box<dyn LlmInstance>, LlmError> {
        // 0 装诚实: model 由调用方传, 不隐式选默认; 不支持时 capability.complete
        // 返 ProviderError::BadResponse → 透传 LlmError::Provider。
        Ok(Box::new(OpenAiCompatibleLlmInstance::new(
            Arc::clone(&self.capability),
            role,
            model,
        )))
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        Ok(self.model_ids())
    }

    fn name(&self) -> &str {
        FACTORY_NAME
    }
}

/// 独立 LLM instance (per scene-d §3 多 instance 隔离), 委托给共享 capability。
pub struct OpenAiCompatibleLlmInstance {
    capability: Arc<OpenAiCompatibleProviderCapability>,
    role: SubagentRole,
    /// 模型 id (spawn 传入; canonical id 或 vendor spelling)。
    model: String,
    /// 缓存的显示名 `"openai-compatible-{model}"`。
    name_str: String,
}

impl std::fmt::Debug for OpenAiCompatibleLlmInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatibleLlmInstance")
            .field("name", &self.name_str)
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}

impl OpenAiCompatibleLlmInstance {
    /// Construct an instance bound to a specific (role, model) pair.
    pub fn new(
        capability: Arc<OpenAiCompatibleProviderCapability>,
        role: SubagentRole,
        model: impl Into<String>,
    ) -> Self {
        let model = model.into();
        let name_str = format!("{FACTORY_NAME}-{model}");
        Self {
            capability,
            role,
            model,
            name_str,
        }
    }

    /// Instance 的 subagent role (per scene-d §3 多 instance 隔离).
    pub fn role(&self) -> SubagentRole {
        self.role
    }

    /// 模型 id (spawn 传入).
    pub fn model(&self) -> &str {
        &self.model
    }
}

#[async_trait]
impl LlmInstance for OpenAiCompatibleLlmInstance {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // 0 装诚实: 0 重试, 0 fallback (router 才拥有 fallback)。
        let normalized = llm_factory_adapters::to_normalized(&self.model, &req);
        let result = self.capability.complete(&normalized).await;
        match result {
            Ok(resp) => Ok(llm_factory_adapters::from_normalized(resp)),
            Err(err) => Err(llm_factory_adapters::map_provider_error(err)),
        }
    }

    fn name(&self) -> &str {
        &self.name_str
    }
}

/// 内嵌 unit test (0 装 PASS)。
#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_plugin::llm_factory::CompletionMessage;

    /// 空 resolver slot 工厂 (0 装: 0 真接 key)。
    fn empty_factory() -> OpenAiCompatibleLlmFactory {
        // 显式 base url + models (from_env 需要 env 配置; 这里走 new 路径).
        let http = reqwest::Client::builder().build().expect("reqwest client");
        let plugin = OpenAiCompatibleProviderPlugin::new(
            "https://example.invalid/v1",
            vec!["deepseek-v4-flash".to_string()],
            http,
            60_000,
        )
        .expect("plugin builds");
        OpenAiCompatibleLlmFactory::new(plugin.provider_for_test())
    }

    #[test]
    fn factory_name_is_openai_compatible() {
        assert_eq!(empty_factory().name(), "openai-compatible");
    }

    #[test]
    fn instance_name_is_factory_dash_model() {
        let factory = empty_factory();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let instance = rt
            .block_on(factory.spawn(SubagentRole::Reviewer, "deepseek-v4-flash"))
            .expect("spawn ok");
        assert_eq!(instance.name(), "openai-compatible-deepseek-v4-flash");
    }

    #[tokio::test]
    async fn available_models_lists_capability_models() {
        let factory = empty_factory();
        let models = factory.available_models().await.expect("models");
        assert_eq!(models, vec!["deepseek-v4-flash".to_string()]);
    }

    #[tokio::test]
    async fn complete_without_resolver_fails_with_credentials_error() {
        // 0 装诚实: 0 真 HTTP, resolver slot 空 → AuthFailed → LlmError::Credentials。
        let factory = empty_factory();
        let instance = factory
            .spawn(SubagentRole::Planner, "deepseek-v4-flash")
            .await
            .expect("spawn");
        let req = CompletionRequest {
            system_prompt: "system".into(),
            messages: vec![CompletionMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: 1.0,
            tools: vec![],
            max_tokens: None,
        };
        match instance.complete(req).await {
            Err(LlmError::Credentials(_)) => {}
            other => panic!("expected LlmError::Credentials, got {other:?}"),
        }
    }

    #[test]
    fn factory_and_instance_are_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<OpenAiCompatibleLlmFactory>();
        _assert_send_sync::<OpenAiCompatibleLlmInstance>();
    }

    #[test]
    fn debug_does_not_leak_secrets() {
        let factory = empty_factory();
        let printed = format!("{factory:?}");
        assert!(!printed.contains("sk-"), "factory Debug 0 泄漏 key");
    }
}
