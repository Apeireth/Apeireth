//! R179 P0-3: thin re-export \u4ece apeireth-llm-iface (拆 memory <-> api \u7f16\u8bd1\u671f\u8fb9).
//! \u539f\u5b9e\u73b0\u5df2\u642c\u5230 `apeireth_llm_iface::traits::*`.
//! \u4e3a\u4fdd\u8bc1\u73b0\u6709 `apeireth_api::llm::{ChatMessage, LlmProvider, LlmRequest, ...}` \u8fd8\u80fd\u7528, \u4ee5 re-export \u8c03\u7528.

#![allow(missing_docs)]
pub use apeireth_llm_iface::{
    ChatMessage, ChatRole, LlmProvider, LlmRequest, LlmResponse, ProviderCapabilities,
    ProviderHealth, ProviderMetadata, TokenUsage,
};