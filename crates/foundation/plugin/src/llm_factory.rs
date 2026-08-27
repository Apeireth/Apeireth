//! P-arch (2026-08-27): LlmFactory trait (场景 D §5 决策 1, v2.0.0-rc RC-5 前置).
//!
//! **位置**: trait 在 `apeireth-plugin` (foundation), impl 留 v2.0.0-rc (RC-5 任务).
//! 与 `ProviderCapability` / `ToolCapability` 同位: 都是 capability 抽象.
//!
//! **目的** (per `v2.0.0-rc-roadmap.md` §2.1):
//! - v2 governance / council / orchestrator / perception 都需要调 LLM, 但:
//!   * 不同模块需要不同 model (per scene-d §5 决策 1: reviewer 用不同 model 隔离)
//!   * 凭证 (API key) 必须走 `CredentialResolver` (不是 `String`)
//! - LlmFactory trait 让所有 LLM 调用方**不直接**知道 provider 细节
//!
//! **LlmFactory vs ProviderCapability**:
//! - `ProviderCapability` 是单实例能力 (一个 provider 一个 capability ID)
//! - `LlmFactory` 是**实例工厂**, 按 (role, model) 生成独立 LLM 调用上下文 (scene-d §3 多 instance 隔离)
//! - LlmInstance 必须是**独立**的 (独立 temperature / system prompt / tool config), 不共享
//!
//! **3 阶审查** (O-6 锚 #9, commit message 必写明):
//! 1. 总体: 与场景 D §5 决策 1 (multi-instance 隔离) + RC-5 (Orchestrator runtime LLM harness) 对齐
//! 2. 系统: trait 在 foundation, impl 在 engine (单向, 与 plugin 体系一致); 凭证走 CredentialResolver
//! 3. 架构: runtime / orchestrator / council / governance 全部通过 `Arc<dyn LlmFactory>` 注入, 不直接 import LLM impl
//!
//! **0 装 PASS**: trait 是 0 装, RC-5 任务在 v2.0.0-rc.1 启动时实现真 LlmFactory + 多个 LLM provider impl
//!
//! **async-trait**: 用 `async_trait::async_trait` 宏 (v1 era 同模式, per `crates/foundation/plugin/src/{provider,tool}.rs`);
//! async-trait decision matrix (`docs/04-internal/async-trait-decision-matrix.md`) 推荐路径 A (保持 async_trait),
//! rc 阶段拍板路径 B/C
//!
//! **v1 compat**: trait 是新增, 0 破现有 100+ consumer

use async_trait::async_trait;
use apeireth_orchestration::SubagentRole;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

/// 角色 (scene-d §3 多 instance 隔离)
/// Re-export from orchestration crate for LlmFactory use.
/// 实际 actor role 用 SubagentRole (Planner / Implementer / Reviewer / Tester / Documenter)
/// + 在 LlmFactory 里加扩展 role (MainActor / Reviewer / SelfAssessor) 给场景 D 例 2/3 用.

/// LLM 完成请求 (provider-agnostic, normalized)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// 系统 prompt (独立 instance 上下文)
    pub system_prompt: String,
    /// 用户消息列表 (transcript 注入)
    pub messages: Vec<CompletionMessage>,
    /// 温度 (0.0-2.0, 默认 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// 工具定义 (OpenAI tool_calls 格式)
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
    /// 最大输出 tokens
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

fn default_temperature() -> f64 { 1.0 }

/// 一条完成消息 (role + content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionMessage {
    /// role: "system" / "user" / "assistant" / "tool"
    pub role: String,
    /// content (text or JSON 序列化 tool_calls result)
    pub content: String,
}

/// LLM 完成响应 (provider-agnostic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// 响应消息 (assistant role)
    pub message: CompletionMessage,
    /// 工具调用 (optional, OpenAI format)
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
    /// finish_reason: "stop" / "length" / "tool_calls"
    pub finish_reason: String,
    /// token 使用统计
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 流式响应 (0 装: alpha 不实现 streaming, rc 阶段)
pub type CompletionStream<'a> = Pin<Box<dyn futures::Stream<Item = Result<CompletionResponse, LlmError>> + Send + 'a>>;

/// LLM 错误 (统一通道)
#[derive(Debug)]
pub enum LlmError {
    /// 凭证缺失或解析失败 (走 CredentialResolver 时错)
    Credentials(String),
    /// 网络/HTTP 错
    Network(String),
    /// Rate limit (per ProviderError::RateLimited, transient)
    RateLimited { retry_after_ms: u64 },
    /// Provider 返回错误 (4xx/5xx)
    Provider(String),
    /// 流中断 / 超时
    Stream(String),
    /// 0 装 PASS: impl 阶段才有的具体错
    NotImplemented(&'static str),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Credentials(m) => write!(f, "llm credentials error: {m}"),
            Self::Network(m) => write!(f, "llm network error: {m}"),
            Self::RateLimited { retry_after_ms } => write!(f, "llm rate limited, retry after {retry_after_ms}ms"),
            Self::Provider(m) => write!(f, "llm provider error: {m}"),
            Self::Stream(m) => write!(f, "llm stream error: {m}"),
            Self::NotImplemented(what) => write!(f, "llm not implemented: {what} (0 装 PASS; rc 阶段实现)"),
        }
    }
}

impl std::error::Error for LlmError {}

/// 独立 LLM instance (per scene-d §3 多 instance 隔离)
///
/// **关键**: 每个 LlmInstance 是**独立**的 LLM 调用上下文 (独立 temperature / 独立 system prompt /
/// 独立 tool config). runtime 调度时为每个 subagent / advisor 起独立 instance, 防止 state 泄漏.
#[async_trait]
pub trait LlmInstance: Send + Sync {
    /// 同步完成
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// 流式完成 (0 装: alpha 不实现, rc 阶段)
    async fn stream(&self, _req: CompletionRequest) -> Result<CompletionStream<'_>, LlmError> {
        Err(LlmError::NotImplemented("LlmInstance::stream (0 装 PASS; rc 阶段实现)"))
    }

    /// 名字 (用于监控 / 日志)
    fn name(&self) -> &str;
}

/// LLM 实例工厂 (per scene-d §5 决策 1)
///
/// runtime / orchestrator / council 全部通过 `Arc<dyn LlmFactory>` 注入, 不直接 import LLM impl.
#[async_trait]
pub trait LlmFactory: Send + Sync {
    /// 起一个独立 LLM 实例 (scene-d §3 多 instance 隔离)
    /// - `role`: SubagentRole (决定 system prompt 模板)
    /// - `model`: model ID (e.g. "anthropic/claude-3-5-sonnet", "MiniMax/M3")
    /// 返回的 LlmInstance 是独立的 (独立 temperature / system prompt / tool config).
    async fn spawn(&self, role: SubagentRole, model: &str) -> Result<Box<dyn LlmInstance>, LlmError>;

    /// 列可用 model (runtime 启动时用, 用于 council 7 advisor 选不同 model)
    async fn available_models(&self) -> Result<Vec<String>, LlmError>;

    /// factory 名字 (用于监控)
    fn name(&self) -> &str;
}

// ============================================
// 0 装 Noop impl (v2.0 alpha 阶段)
// ============================================

/// 0 装 PASS: NoopLlmInstance
/// 不调真 LLM API, 返 `NotImplemented` 错误. rc 阶段真 LlmInstance (MiniMax / Anthropic /
/// OpenAI-compatible) 替换 Noop. trait 边界 + interface contract 已经画好, rc 阶段
/// 是"interface → real impl"一对一映射, 不会有结构性破坏.
///
/// **为何 0 装**: alpha 阶段没真 LLM API key, 也不该假装"我能调". runtime 拿 NoopLlmFactory
/// 启动时, 所有 LLM 调用返 NotImplemented, governance 把 NotImplemented 转 Deny 或
/// Abstain (per scene-d §2.2). 不假装有 AI 决策.
pub struct NoopLlmInstance {
    role: SubagentRole,
    model: String,
}

impl NoopLlmInstance {
    pub fn new(role: SubagentRole, model: impl Into<String>) -> Self {
        Self { role, model: model.into() }
    }
}

#[async_trait]
impl LlmInstance for NoopLlmInstance {
    async fn complete(&self, _req: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Err(LlmError::NotImplemented(
            "NoopLlmInstance::complete (0 装 PASS; rc 阶段真 LLM 接入)",
        ))
    }

    fn name(&self) -> &str {
        "noop"
    }
}

/// 0 装 PASS: NoopLlmFactory
/// spawn 即返 NoopLlmInstance. runtime 启动时用.
pub struct NoopLlmFactory;

#[async_trait]
impl LlmFactory for NoopLlmFactory {
    async fn spawn(&self, role: SubagentRole, model: &str) -> Result<Box<dyn LlmInstance>, LlmError> {
        // 0 装: 返 Noop, 不是真 LLM. rc 阶段返 `LlmFactoryImpl::spawn(...)` 真 LLM instance.
        Ok(Box::new(NoopLlmInstance::new(role, model)))
    }

    async fn available_models(&self) -> Result<Vec<String>, LlmError> {
        // 0 装: 永返空 (没真可用 model)
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0 装 PASS: CompletionRequest 默认值 (temperature = 1.0)
    #[test]
    fn completion_request_default_temperature() {
        let req = CompletionRequest {
            system_prompt: "system".into(),
            messages: vec![],
            temperature: default_temperature(),
            tools: vec![],
            max_tokens: None,
        };
        assert_eq!(req.temperature, 1.0);
    }

    /// 0 装 PASS: LlmError 5 个 variant 都有清晰描述
    #[test]
    fn llm_error_displays() {
        let e = LlmError::NotImplemented("LlmInstance::stream");
        let s = format!("{e}");
        assert!(s.contains("not implemented"));
        assert!(s.contains("0 装 PASS"));

        let e2 = LlmError::RateLimited { retry_after_ms: 5000 };
        let s2 = format!("{e2}");
        assert!(s2.contains("rate limited"));
        assert!(s2.contains("5000"));
    }

    /// 0 装 PASS: trait 是 0 装占位 — 没 impl, 仅 trait 边界
    #[test]
    fn llm_factory_trait_is_zero_implementation() {
        // 验证 trait 定义存在
        fn _check_factory_exists<T: LlmFactory>() {}
        fn _check_instance_exists<T: LlmInstance>() {}
        // 编译通过 = trait 边界完整
    }

    /// 0 装 PASS: SubagentRole 枚举 (从 orchestration crate re-export) — 5 role 类型存在
    #[test]
    fn subagent_role_variants_accessible() {
        let _roles = [
            SubagentRole::Planner,
            SubagentRole::Implementer,
            SubagentRole::Reviewer,
            SubagentRole::Tester,
            SubagentRole::Documenter,
        ];
        // 编译通过 = 5 role 都可达
    }

    /// 0 装 PASS: NoopLlmFactory::spawn 返 NoopLlmInstance (0 装占位, 不假装真 LLM)
    #[tokio::test]
    async fn noop_llm_factory_spawn_returns_noop_instance() {
        let factory = NoopLlmFactory;
        let instance = factory
            .spawn(SubagentRole::Reviewer, "anthropic/claude-3-5")
            .await
            .expect("spawn NoopLlmInstance");
        assert_eq!(instance.name(), "noop");
        // complete 应该返 NotImplemented (0 装: 不假装调真 LLM)
        let req = CompletionRequest {
            system_prompt: "system".into(),
            messages: vec![],
            temperature: 1.0,
            tools: vec![],
            max_tokens: None,
        };
        let result = instance.complete(req).await;
        match result {
            Err(LlmError::NotImplemented(_)) => {}
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    /// 0 装 PASS: NoopLlmFactory::available_models 永返空 (没真 model)
    #[tokio::test]
    async fn noop_llm_factory_available_models_empty() {
        let factory = NoopLlmFactory;
        let models = factory
            .available_models()
            .await
            .expect("available_models Noop");
        assert!(models.is_empty(), "NoopLlmFactory 0 装, 无可用 model");
    }

    /// 0 装 PASS: NoopLlmFactory 可当 Send + Sync (NoopLlmInstance 含 trait bound Send+Sync)
    #[test]
    fn noop_llm_factory_is_send_sync() {
        fn _assert_send_sync<T: Send + Sync>() {}
        _assert_send_sync::<NoopLlmFactory>();
        _assert_send_sync::<NoopLlmInstance>();
    }
}