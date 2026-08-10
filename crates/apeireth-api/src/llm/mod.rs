//! LLM 子模块 —— Apeireth 通用 API 扩展平台的 LLM 客户端
//!
//! 包含: traits / error / router / config / middleware / providers / semantic_router
//! 详见各子模块文档

#![warn(missing_docs)]

pub mod config;
pub mod error;
pub mod middleware;
pub mod providers;
pub mod router;
pub mod semantic_router;
pub mod traits;

pub use config::{LlmConfig, ProviderConfig, RouterConfig};
pub use error::LlmError;
pub use middleware::{LoggingMiddleware, MiddlewareChain, RetryMiddleware};
pub use providers::{
    anthropic_compat::{AnthropicCompatibleConfig, AnthropicCompatibleProvider},
    apeireth_api::{ApeirethApiConfig, ApeirethApiProvider},
    openai_compat::{OpenAiCompatibleConfig, OpenAiCompatibleProvider},
    scripted::{ScriptedLlmProvider, ScriptedResponse},
};
pub use router::MultiLlmRouter;
pub use semantic_router::{Route, SemanticRouter, SemanticRouterConfig};
pub use traits::{
    ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities,
    ProviderHealth, TokenUsage,
};
