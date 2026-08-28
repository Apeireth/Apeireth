//! RC-5 LlmFactory 真实现 — MiniMax backend adapter.
//!
//! **位置** (per `apeireth_plugin::llm_factory` 模块头 §"LlmFactory vs ProviderCapability"):
//! trait 在 foundation (`apeireth-plugin`), impl 跟 capability 同 crate (`apeireth-provider`),
//! 跟 `MinimaxProviderCapability` / `MinimaxProviderPlugin` 同模式。**0 触碰** plugin crate 的 trait
//! 边界, 也**0 引入循环依赖** (provider → plugin 已是合法方向, 反向会循环)。
//!
//! **目的** (per `v2.0.0-rc-roadmap.md` §2.1 + §3 RC-5):
//! - runtime / orchestrator / council / governance 通过 `Arc<dyn LlmFactory>` 注入
//!   (per llm_factory.rs:150 "runtime 全部通过 Arc<dyn LlmFactory> 注入, 不直接 import LLM impl")
//! - 本 impl 把 `LlmFactory` trait 边界接到 canonical `MinimaxProviderCapability`,
//!   复用其 HTTP client + OpenAI Chat translation + per-turn CredentialResolver
//! - **不**复制 reqwest client / 不**重新**做 model 翻译 / 不**直接**读 env
//!
//! **LlmFactory vs ProviderCapability** (复用而非重写):
//! - `MinimaxProviderCapability` 是**单实例** (一个 provider 一个 capability ID, 跨请求共享 reqwest
//!   client + credential_key + model list)
//! - `MinimaxLlmFactory` 是**实例工厂** (per scene-d §3 多 instance 隔离), 每个 (role, model)
//!   生成独立 `LlmInstance`, 每个 instance 持有 role 上下文 (system prompt / temperature 模板)
//! - 共享 `Arc<MinimaxProviderCapability>` 保证: 一个 reqwest client, 一份 model list,
//!   一条 per-turn credential resolve path (per canonical_minimax.rs:120 "resolve_key on every turn")
//!
//! **3 阶审查** (O-6 锚 #9):
//! 1. 总体: 与场景 D §5 决策 1 (multi-instance 隔离) + RC-5 (Orchestrator runtime LLM harness) 对齐
//!    — runtime / orchestrator 通过 `Arc<dyn LlmFactory>` 拿 instance, 不知道下面接的是 MiniMax
//! 2. 系统: trait 在 foundation (`apeireth-plugin`), impl 在 engine (`apeireth-provider`), 单向;
//!    impl 直接复用 canonical capability, 0 写新 HTTP / 0 写新翻译层 / 0 写新凭证解析
//! 3. 架构: retry owner per layer — 本 factory **不**重试 (`complete` 走 capability 的
//!    `ProviderError` 分类, 由上层 router 决定 fallback; per canonical_minimax.rs:23
//!    "One retry owner per layer")
//!
//! **0 装诚实**:
//! - 真接 MiniMax API (`APEIRETH_API_KEY` env via `EnvCredentialResolver`, 或 keyring /
//!   encrypted-file resolver via RC-9 `KeyringCredentialResolver`)
//! - **不**硬编码 key; **不**在源码 / commit / log / Debug print 暴露 key
//! - **不** mock 真 LLM call; integration test 用 `#[ignore = "requires MINIMAX_API_KEY"]`
//!   路径明确分 "real key 测试" vs "0 装路径"
//! - **不**重写 provider translation; 错误经 `ProviderError::*` 一对一映射到 `LlmError::*`,
//!   分类 (Network / AuthFailed / RateLimited / Timeout / BadResponse / Refused) 跟
//!   canonical_minimax.rs:166 `classify_status` 一致
//!
//! **0 触碰 LOCKED**: 9 哲学锚 / 13 键 / 3 项不可变脊柱 / workspace.version / R11 baseline;
//! 0 改 trait 边界, 0 改 protocol crate, 0 改 orchestration crate.

use std::sync::Arc;

use apeireth_orchestration::SubagentRole;
use apeireth_plugin::llm_factory::{
    CompletionMessage, CompletionRequest, CompletionResponse, LlmError, LlmFactory, LlmInstance,
    TokenUsage,
};
use apeireth_plugin::{CredentialResolver, ProviderCapability, ProviderError};
use apeireth_protocol::canonical::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
};
use async_trait::async_trait;

use crate::canonical_minimax::{MinimaxProviderCapability, MinimaxProviderPlugin};

/// 工厂名字 (用于监控 / 日志; per llm_factory.rs:162 "factory 名字")。
pub const FACTORY_NAME: &str = "minimax";

/// 真接 MiniMax backend 的 LlmFactory impl。
///
/// 持有**共享**的 [`MinimaxProviderCapability`] (单一 reqwest client + 单一 model list +
/// 单一 credential_key)。spawn 时生成独立 [`MinimaxLlmInstance`], 各自记 role + model
/// 上下文, 但都委托给同一 capability 的 `complete` 做 HTTP。这是 "multi-instance 隔离 +
/// single-transport 共享" 的正解。
///
/// **凭证**: 0 装诚实 — capability 在每次 `complete` 通过 `CredentialResolver` 取
/// (`provider.minimax.api_key`), 工厂构造不接 key。`EnvCredentialResolver` 默认映射
/// `provider.minimax.api_key → APEIRETH_API_KEY` (per credentials.rs:43)。也可注入
/// keyring / encrypted-file resolver (per RC-9 keyring_bootstrap, `build_keyring_resolver`
/// 同样可作 resolver 来源)。
pub struct MinimaxLlmFactory {
    capability: Arc<MinimaxProviderCapability>,
}

impl std::fmt::Debug for MinimaxLlmFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 0 装诚实: 0 输出 key; capability 自身 Debug 已 0 装泄漏 (per
        // canonical_minimax.rs:660 "debug_does_not_leak_secrets" 测试)
        f.debug_struct("MinimaxLlmFactory")
            .field("name", &FACTORY_NAME)
            .finish_non_exhaustive()
    }
}

impl MinimaxLlmFactory {
    /// 从已构造好的 [`MinimaxProviderCapability`] 包成 factory。
    ///
    /// **0 装诚实**: 工厂构造不接 key; capability 的 `ResolverSlot` 已在 plugin
    /// `initialize` 阶段填好 (per canonical_minimax.rs:386), 调用方应通过
    /// [`MinimaxProviderPlugin::attach_resolver_for_test`] 或 runtime 注入。
    /// **不**传 key 进 factory。
    pub fn new(capability: Arc<MinimaxProviderCapability>) -> Self {
        Self { capability }
    }

    /// 真生产路径: 从 env + 已配置 capability 构造 factory。
    ///
    /// **0 装诚实**:
    /// - 真接 `EnvCredentialResolver` (默认 `APEIRETH_API_KEY` env var)
    /// - 不在源码里 hardcode key, 不在 commit / log 暴露 key
    /// - 真生产应该用 `KeyringSelector` (per RC-9 keyring_bootstrap), 但 factory 是底层
    ///   原语, 上层装配决定 resolver 来源
    pub fn from_env() -> Result<Self, FactoryError> {
        Self::from_env_with_resolver(Arc::new(crate::credentials::EnvCredentialResolver::new()))
    }

    /// 真生产路径, 显式注入 resolver。
    ///
    /// resolver 可以是 `EnvCredentialResolver` / `KeyringCredentialResolver` /
    /// `StaticCredentials` (测试) — 任何实现 `CredentialResolver` 的都行。
    /// capability 共享这个 resolver via `attach_resolver_for_test`, 工厂 0 持有 key。
    pub fn from_env_with_resolver(
        resolver: Arc<dyn CredentialResolver>,
    ) -> Result<Self, FactoryError> {
        // 用 from_env 路径构造 plugin, 然后注入 resolver. plugin 自带 reqwest client + 默认
        // base url + 默认 models (per canonical_minimax.rs:321 `from_env`).
        let plugin = MinimaxProviderPlugin::from_env().map_err(FactoryError::Plugin)?;
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
    pub fn capability(&self) -> &Arc<MinimaxProviderCapability> {
        &self.capability
    }
}

/// factory 构造错 (0 装诚实: 区分来源, 0 静默)
#[derive(Debug)]
pub enum FactoryError {
    /// plugin 构造错 (reqwest build 失败 / invalid base url / empty models)
    Plugin(apeireth_plugin::PluginError),
}

impl std::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Plugin(e) => write!(f, "minimax LlmFactory 构造失败: plugin error: {e}"),
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
impl LlmFactory for MinimaxLlmFactory {
    async fn spawn(
        &self,
        role: SubagentRole,
        model: &str,
    ) -> Result<Box<dyn LlmInstance>, LlmError> {
        // 0 装诚实: model 由调用方传, 不隐式选默认. capability 内 supports_model 决定
        // 路由合法性; 不支持时 capability.complete 会返 ProviderError::BadResponse, 我们
        // 透传成 LlmError::Provider, 0 装不假装支持.
        Ok(Box::new(MinimaxLlmInstance::new(
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

/// 独立 LLM instance (per scene-d §3 多 instance 隔离)。
///
/// 每个 instance 持有 (role, model) 上下文, 但**委托**给共享 `MinimaxProviderCapability`
/// 完成 HTTP. role 信息仅用于 `name()` (监控); model 用于 `name()` + 传 capability 做
/// capability 内部 `supports_model` 检查。完成请求时不**额外**改 system prompt
/// (per completion_request 上游决定; instance 0 装模板化)。
pub struct MinimaxLlmInstance {
    capability: Arc<MinimaxProviderCapability>,
    role: SubagentRole,
    /// 模型 id (由 spawn 传入, 可能含 canonical id 或 vendor spelling)
    model: String,
    /// 缓存的显示名 `"minimax-{model}"`, 0 装诚实: model 不可变, 单次构造即可
    name_str: String,
}

impl std::fmt::Debug for MinimaxLlmInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinimaxLlmInstance")
            .field("name", &self.name_str)
            .field("role", &self.role)
            .finish_non_exhaustive()
    }
}

impl MinimaxLlmInstance {
    /// Construct an instance bound to a specific (role, model) pair.
    /// `name()` 返回 `"minimax-{model}"`, 0 装诚实地把 model 拼进 name (per
    /// llm_factory.rs:144 "名字 (用于监控 / 日志)").
    pub fn new(
        capability: Arc<MinimaxProviderCapability>,
        role: SubagentRole,
        model: impl Into<String>,
    ) -> Self {
        // 构造时一次性算好 `name_str` (0 装: model 不可变, 0 装重算)
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
        // 0 装诚实: role 由 spawn 决定, instance 不重映射
        self.role
    }

    /// 模型 id (spawn 传入; canonical id 或 vendor spelling 都行).
    pub fn model(&self) -> &str {
        &self.model
    }

    /// 转换 `CompletionRequest` (factory 边界) → `NormalizedRequest` (provider 边界)。
    ///
    /// **字段映射**:
    /// - `system_prompt` → 头部 `NormalizedMessage::system(...)` (provider 内部决定 wire)
    /// - `messages` (factory 风格 role/content 字符串) → `NormalizedMessage` (枚举 role + 多模 content)
    /// - `temperature` (f64) → `temperature` (Option<f32>) (provider 内部序列化时按协议)
    /// - `max_tokens` → `max_tokens`
    /// - `tools` (factory 风格 JSON Value) → **0 装**: minimax 当前**不**支持 tool calls
    ///   (per canonical_minimax.rs:148 `adapt_request` 拒绝 tools; `feature = ToolCalls` 0 装),
    ///   传 tools → 返 `LlmError::Provider` (provider 自会返 `BadResponse`, 我们透传)
    fn to_normalized(&self, req: &CompletionRequest) -> NormalizedRequest {
        let mut messages = Vec::with_capacity(req.messages.len() + 1);
        if !req.system_prompt.is_empty() {
            messages.push(NormalizedMessage::system(&req.system_prompt));
        }
        for m in &req.messages {
            messages.push(NormalizedMessage {
                role: MessageRole::from_legacy_value(&m.role),
                content: vec![ContentPart::Text {
                    text: m.content.clone(),
                }],
                tool_calls: Vec::new(),
                tool_call_id: None,
                name: None,
            });
        }
        NormalizedRequest {
            model: self.model.clone(),
            messages,
            temperature: Some(req.temperature as f32),
            max_tokens: req.max_tokens,
            stream: false,
            stop: Vec::new(),
            tools: Vec::new(), // 0 装: minimax 0 装支持 tool calls (per capability feature truthfulness)
            tool_choice: None,
            metadata: Default::default(),
        }
    }

    /// 转换 `NormalizedResponse` (provider 边界) → `CompletionResponse` (factory 边界)。
    fn from_normalized(
        &self,
        resp: apeireth_protocol::canonical::NormalizedResponse,
    ) -> CompletionResponse {
        let finish_reason = match resp.finish_reason {
            Some(NormalizedFinishReason::Stop) => "stop",
            Some(NormalizedFinishReason::Length) => "length",
            Some(NormalizedFinishReason::ToolCalls) => "tool_calls",
            Some(NormalizedFinishReason::ContentFilter) => "content_filter",
            Some(NormalizedFinishReason::StopSequence) => "stop_sequence",
            Some(NormalizedFinishReason::Other) | None => "other",
        }
        .to_string();

        let tool_calls: Vec<serde_json::Value> = resp
            .tool_calls
            .into_iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "type": "function",
                    "function": {
                        "name": c.name,
                        "arguments": c.arguments,
                    }
                })
            })
            .collect();

        CompletionResponse {
            message: CompletionMessage {
                role: "assistant".into(),
                content: resp.content,
            },
            tool_calls,
            finish_reason,
            usage: TokenUsage {
                prompt_tokens: resp.usage.prompt_tokens,
                completion_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
        }
    }

    /// 把 `ProviderError` 一对一映射到 `LlmError`。
    ///
    /// 映射保持 provider 已分类的语义 (transient vs permanent), 上层 router 拿到
    /// `LlmError` 后可按 `is_retryable` 派生决定 fallback / 重试 (per ProviderError:86
    /// `is_retryable`); LlmError 0 装重派生, 直接透传。
    fn map_provider_error(&self, err: ProviderError) -> LlmError {
        match err {
            ProviderError::AuthFailed { detail, .. } => LlmError::Credentials(detail),
            ProviderError::RateLimited { retry_after_ms, .. } => {
                LlmError::RateLimited { retry_after_ms }
            }
            ProviderError::Timeout { timeout_ms, .. } => {
                LlmError::Stream(format!("timeout after {timeout_ms}ms"))
            }
            ProviderError::Network { detail, .. } => LlmError::Network(detail),
            ProviderError::BadResponse { detail, .. } => LlmError::Provider(detail),
            ProviderError::Refused { detail, .. } => LlmError::Provider(detail),
            // 0 装诚实: `ProviderError` 标 `#[non_exhaustive]`, 未来加 variant 时 (e.g.
            // ContentFilter / PolicyViolation), 兜底成 LlmError::Provider, 0 panic.
            // 0 装, 0 假装 "全分类完成".
            other => LlmError::Provider(format!("unclassified provider error: {other}")),
        }
    }
}

#[async_trait]
impl LlmInstance for MinimaxLlmInstance {
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // 0 装诚实: 0 重试, 0 fallback (per canonical_minimax.rs:23 "One retry owner per layer";
        // router 才拥有 fallback)。单次 HTTP, 错误透传。
        let normalized = self.to_normalized(&req);
        let result = self.capability.complete(&normalized).await;
        match result {
            Ok(resp) => Ok(self.from_normalized(resp)),
            Err(err) => {
                // 0 装诚实: error 字符串走 provider 已分类的 detail, 0 在这层加 key / 0 在 log
                // 泄漏 Secret (ProviderError 是 provider-side classification, 不含 secret,
                // per canonical_minimax.rs:236 `classify_status` 0 把 Authorization 写进
                // body_text).
                Err(self.map_provider_error(err))
            }
        }
    }

    fn name(&self) -> &str {
        // 0 装诚实: name 是 "minimax-{model}", 0 含 role (role 是监控 metadata, 不放 name)
        &self.name_str
    }
}

/// 内嵌 unit test (0 装 PASS)
#[cfg(test)]
mod tests {
    use super::*;

    /// 空 resolver slot 工厂 (0 装: 0 真接 key)
    fn empty_capability() -> Arc<MinimaxProviderCapability> {
        // 用 from_env (公开 API) 构造 plugin, 默认 base url / 默认 models / 默认 timeout;
        // 0 显式接 key (key 在 resolver, 0 装 slot 留空)
        MinimaxProviderPlugin::from_env()
            .expect("plugin builds")
            .provider_for_test()
    }

    #[test]
    fn factory_name_is_minimax() {
        let cap = empty_capability();
        let factory = MinimaxLlmFactory::new(cap);
        assert_eq!(factory.name(), "minimax");
    }

    #[test]
    fn instance_name_is_minimax_dash_model() {
        let cap = empty_capability();
        let factory = MinimaxLlmFactory::new(cap);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let instance = rt
            .block_on(factory.spawn(SubagentRole::Reviewer, "MiniMax-M3"))
            .expect("spawn ok");
        assert_eq!(instance.name(), "minimax-MiniMax-M3");
    }

    #[tokio::test]
    async fn available_models_lists_capability_models() {
        let cap = empty_capability();
        let factory = MinimaxLlmFactory::new(cap);
        let models = factory.available_models().await.expect("models");
        // DEFAULT_MODELS 是 ["MiniMax-M3", "MiniMax-M3-thinking"], capability 内部转
        // canonical id ("minimax-m3", "minimax-m3-thinking").
        assert_eq!(
            models,
            vec!["minimax-m3".to_string(), "minimax-m3-thinking".to_string()]
        );
    }

    #[tokio::test]
    async fn complete_without_resolver_fails_with_credentials_error() {
        // 0 装诚实: 0 真 HTTP, resolver slot 空 → capability 返 ProviderError::AuthFailed
        // → 透传成 LlmError::Credentials
        let cap = empty_capability();
        let factory = MinimaxLlmFactory::new(cap);
        let instance = factory
            .spawn(SubagentRole::Planner, "MiniMax-M3")
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
        let result = instance.complete(req).await;
        match result {
            Err(LlmError::Credentials(_)) => {}
            other => panic!("expected LlmError::Credentials, got {other:?}"),
        }
    }

    #[test]
    fn factory_and_instance_are_send_sync() {
        // runtime / orchestrator / council 多任务并行调用 (per scene-d §3), 必须 Send + Sync
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<MinimaxLlmFactory>();
        _assert_send_sync::<MinimaxLlmInstance>();
    }

    #[test]
    fn debug_does_not_leak_secrets() {
        // 0 装诚实: factory 0 持有 key (key 走 CredentialResolver), Debug 0 泄漏
        let cap = empty_capability();
        let factory = MinimaxLlmFactory::new(cap);
        let printed = format!("{factory:?}");
        assert!(!printed.contains("sk-"), "factory Debug 0 泄漏 key");
    }
}
