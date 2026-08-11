//! `apeireth-provider::minimax` — MiniMax (minimaxi) Provider client
//!
//! **R128 (2026-08-12)**: 新增 minimax descriptor.
//! MiniMax (formerly minimaxi) is MiniMax (minimaxi) -- an OpenAI/Anthropic-compatible API
//! that hosts the `MiniMax-M3` model family. apeireth-provider treats it as a 6th provider
//! alongside the original 5 (claude-code / codex / copilot / gemini-cli / opencode).
//!
//! **R128 真接**: 实际 HTTP 调用在 `apeireth-api::llm::providers::anthropic_compat`
//! (`AnthropicCompatibleProvider`) + `openai_compat` (`OpenAiCompatibleProvider`).
//! minimax 同时支持 Anthropic Messages API 和 OpenAI Chat Completions 协议, 同 key 复用.
//!
//! **descriptor vs 真接**: apeireth-provider 给 minimax 提供配置描述符 (name + tools + model_kinds),
//! 实际 provider 实现走 apeireth-api (避免循环依赖).

#![allow(dead_code)]

/// R128: MiniMax Provider struct descriptor.
///
/// This struct is a thin descriptor: `name`, supported `tools`, and supported `model_kinds`.
/// It does **not** hold an HTTP client or live state — instantiate one per provider session
/// for configuration introspection (e.g. tool whitelist, model allow-list).
///
/// For actual HTTP calls, use `apeireth_api::llm::AnthropicCompatibleProvider` or
/// `apeireth_api::llm::OpenAiCompatibleProvider` with `base_url = "https://api.minimaxi.com"`.
pub struct MinimaxProvider {
    /// provider name, "minimax" (kebab-case to match VCP `vcptoolbox` convention).
    pub name: &'static str,
    /// 8 tools (per apeireth-core 13-key verdict + apeireth-tool-registry 8 standard tools).
    pub tools: Vec<&'static str>,
    /// 7 model kinds (per R82 LIVE benchmark 7/7 pass):
    /// `MiniMax-M3` / `MiniMax-M2.7-highspeed` / `MiniMax-M2.7` /
    /// `MiniMax-M2.5-highspeed` / `MiniMax-M2.5` / `MiniMax-M2.1-highspeed` / `MiniMax-M2.1`.
    /// Plus legacy `abab` series for back-compat.
    pub model_kinds: Vec<&'static str>,
}

impl MinimaxProvider {
    /// Construct a new MiniMax descriptor.
    pub fn new() -> Self {
        Self {
            name: "minimax",
            tools: vec![
                "Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch",
            ],
            model_kinds: vec![
                "MiniMax-M3",
                "MiniMax-M2.7-highspeed",
                "MiniMax-M2.7",
                "MiniMax-M2.5-highspeed",
                "MiniMax-M2.5",
                "MiniMax-M2.1-highspeed",
                "MiniMax-M2.1",
            ],
        }
    }
}

impl Default for MinimaxProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// R128: 4 protocols MiniMax supports (dual-protocol gateway):
/// - `anthropic` -> `POST https://api.minimaxi.com/anthropic/v1/messages` (x-api-key auth)
/// - `openai_chat` -> `POST https://api.minimaxi.com/v1/chat/completions` (Bearer auth)
/// - `openai_responses` -> `POST https://api.minimaxi.com/v1/responses`
/// - `gemini` -> (MiniMax supports Gemini-compatible path, less commonly used)
pub const MINIMAX_PROTOCOLS: [&str; 4] = [
    "anthropic",
    "openai_chat",
    "openai_responses",
    "gemini",
];

/// R128: default MiniMax base URL.
pub const MINIMAX_BASE_URL: &str = "https://api.minimaxi.com";

/// R128: MiniMax 8 standard tools whitelist (per apeireth-tool-registry convention).
pub const TOOL_WHITELIST: [&str; 8] = [
    "Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch",
];

#[cfg(test)]
mod minimax_tests {
    use super::*;

    #[test]
    fn minimax_provider_basics() {
        let p = MinimaxProvider::new();
        assert_eq!(p.name, "minimax");
        assert_eq!(p.tools.len(), 8);
        // R82 LIVE 7/7 models pass, plus legacy abab series => 7 model kinds in current descriptor
        assert!(p.model_kinds.len() >= 7, "minimax should have 7+ model kinds");
    }

    #[test]
    fn minimax_4_protocols() {
        assert_eq!(MINIMAX_PROTOCOLS.len(), 4);
        assert!(MINIMAX_PROTOCOLS.contains(&"anthropic"));
        assert!(MINIMAX_PROTOCOLS.contains(&"openai_chat"));
        assert!(MINIMAX_PROTOCOLS.contains(&"openai_responses"));
    }

    #[test]
    fn minimax_tool_whitelist_8() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
    }

    #[test]
    fn minimax_base_url_correct() {
        assert_eq!(MINIMAX_BASE_URL, "https://api.minimaxi.com");
    }
}