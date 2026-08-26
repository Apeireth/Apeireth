//! Canonical provider capabilities.
//!
//! Providers translate vendor wire protocols into the normalized protocol and
//! expose themselves through `ProviderCapability` plugins. Routing, sessions,
//! governance, and tool execution remain owned by the canonical runtime.

#![warn(missing_docs)]

/// Anthropic Messages API provider capability and plugin.
pub mod canonical_anthropic;
/// MiniMax OpenAI-compatible provider capability and plugin.
pub mod canonical_minimax;
/// Generic OpenAI-compatible provider capability and plugin.
pub mod canonical_openai_compatible;
/// Environment-backed credential resolution.
pub mod credentials;
/// Shared OpenAI Chat Completions wire conversion.
pub mod openai_chat;
/// Canonical provider model identifiers and vendor wire names.
pub mod provider_model;
/// Provider-local reasoning field normalization.
pub mod reasoning_adapter;
