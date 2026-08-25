//! minimax — MiniMax (minimaxi) Provider client (6th provider, R128).
//!
//! 0 装 PASS: 真 Provider descriptor (8 tools + 7 model kinds + 4 protocols).
//!
//! v1 对齐 (R128): MiniMax (formerly minimaxi) OpenAI/Anthropic-compatible API hosting `MiniMax-M3` family.

pub struct MinimaxProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
}

impl MinimaxProvider {
    pub fn new() -> Self {
        Self {
            name: "minimax",
            tools: vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"],
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

impl Default for MinimaxProvider { fn default() -> Self { Self::new() } }

pub const MINIMAX_PROTOCOLS: [&str; 4] = ["anthropic", "openai_chat", "openai_responses", "gemini"];
pub const MINIMAX_BASE_URL: &str = "https://api.minimaxi.com";
pub const TOOL_WHITELIST: [&str; 8] = ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"];

#[cfg(test)]
mod minimax_tests {
    use super::*;
    #[test]
    fn minimax_provider_basics() {
        let p = MinimaxProvider::new();
        assert_eq!(p.name, "minimax");
        assert_eq!(p.tools.len(), 8);
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
    #[test]
    fn default_impl() {
        let p: MinimaxProvider = Default::default();
        assert!(p.model_kinds.len() >= 7);
    }
}
