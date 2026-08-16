//! R179 P0-3: LLM \u62bd\u8c61\u63a5\u53e3 crate
//!
//! ## \u4e3a\u4ec0\u4e48\u62c5\u8fd9\u4e2a
//! `apeireth-memory` \u539f\u672c\u5728 `llm_analysis.rs` \u91cc import `apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest}`,
//! \u8ba9 memory \u2192 api \u8d70\u4e86\u4e00\u6761\u7f16\u8bd1\u671f\u8fb9 (\u5f53\u524d\u662f\u5355\u5411, \u672a\u5fa9\u73b0 Rust \u7f16\u8bd1\u5668\u62a5\u9519,
//! \u4f46 runtime init cycle + \u672a\u6765\u53cd\u8f6c\u98ce\u9669\u90fd\u5728).
//!
//! ## \u672c crate \u5305\u542b
//! - `error::LlmError` \u2014 \u7edf\u4e00\u5f02\u5e38\u5206\u7c7b + retryable \u5224\u5b9a
//! - `traits::LlmProvider` \u2014 \u62bd\u8c61 trait
//! - `traits::{ChatMessage, ChatRole, LlmRequest, LlmResponse, TokenUsage, ProviderCapabilities, ProviderHealth, ProviderMetadata}` \u2014 \u4f34\u968f\u7c7b\u578b
//!
//! ## apeireth-api \u7684\u5173\u7cfb
//! `apeireth-api::llm::{traits, error}` \u4ee5 re-export \u4ece\u672c crate, \u5b58\u91cf\u5168\u90e8\u7ef4\u62a4.
//! \u6240\u6709\u73b0\u6709\u4ee3\u7801 `use apeireth_api::llm::LlmProvider` \u4ecd\u53ef\u7528, 0 \u7834\u574f.
//!
//! ## apeireth-memory \u7684\u5173\u7cfb
//! `apeireth-memory::llm_analysis` \u5207\u5230 `use apeireth_llm_iface::*`, \u4e0d\u518d dep `apeireth-api`.
//!
//! ## \u4e3e\u4e2a\u4f8b\u5b50
//! ```rust
//! use apeireth_llm_iface::{ChatMessage, LlmRequest, LlmProvider, LlmError};
//!
//! async fn call_llm(p: &dyn LlmProvider) -> Result<String, LlmError> {
//!     let req = LlmRequest::new("test-model", vec![ChatMessage::user("hi")]);
//!     let resp = p.complete(req).await?;
//!     Ok(resp.content)
//! }
//! ```

#![warn(missing_docs)]

pub mod error;
pub mod traits;

// 便利: \u6240\u6709\u516c\u5171\u7c7b\u578b\u4e00\u6b21 re-export
pub use error::LlmError;
pub use traits::{
    ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities,
    ProviderHealth, ProviderMetadata, TokenUsage,
};
