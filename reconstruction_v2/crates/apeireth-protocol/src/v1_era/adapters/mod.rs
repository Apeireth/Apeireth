//! v1 era adapters — 4 协议 adapter 内部 stub 实现
//!
//! 与 v2 顶层 `crate::adapters::*` 命名不同 (v2 用 `OpenAiAdapter`/`AnthropicAdapter`
//! 等单名, 这里是 v1 的 4 拆命名). 提供 v1 bridge.rs 期望的最小协议适配接口:
//! - `name()` / `endpoint_path()`
//! - `adapt_request(&NormalizedRequest) -> Result<Value, ProtocolError>`
//! - `adapt_response(&Value) -> Result<NormalizedResponse, ProtocolError>`

pub mod anthropic_messages;
pub mod gemini;
pub mod openai_chat;
pub mod openai_responses;

pub use anthropic_messages::AnthropicMessagesAdapter;
pub use gemini::GeminiAdapter;
pub use openai_chat::OpenAiChatAdapter;
pub use openai_responses::OpenAiResponsesAdapter;

use crate::v1_era::error::ProtocolError;
use crate::v1_era::normalized::{NormalizedRequest, NormalizedResponse};
use serde_json::Value;

/// v1 era ProtocolAdapter trait (per-bridge ZST)
pub trait ProtocolAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn endpoint_path(&self) -> &'static str;
    fn adapt_request(&self, req: &NormalizedRequest) -> Result<Value, ProtocolError>;
    fn adapt_response(&self, raw: &Value) -> Result<NormalizedResponse, ProtocolError>;
}
