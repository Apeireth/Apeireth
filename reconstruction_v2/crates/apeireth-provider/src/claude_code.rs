//! claude_code — Anthropic Claude Code Provider client.
//!
//! 0 装 PASS: 真 Provider descriptor (8 tools + 3 model kinds + TOOL_WHITELIST + K1_CHECKS).
//!
//! v1 对齐 (R20 阶段 4 → R35): @anthropic-ai/claude-agent-sdk 0.2.112 (8 工具 + 3 ModelKind).

pub struct ClaudeCodeProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
}

impl ClaudeCodeProvider {
    pub fn new() -> Self {
        Self {
            name: "claude-code",
            tools: vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"],
            model_kinds: vec!["claude-opus-4-5", "claude-sonnet-4-5", "claude-haiku-4-5"],
        }
    }
}

impl Default for ClaudeCodeProvider { fn default() -> Self { Self::new() } }

pub const TOOL_WHITELIST: [&str; 8] = ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"];
pub const K1_CHECKS: [&str; 5] = ["base_url_not_empty", "auth_token_format", "tool_in_whitelist", "args_is_object", "timeout_positive"];

#[cfg(test)]
mod claude_code_tests {
    use super::*;
    #[test]
    fn claude_code_provider_basics() {
        let p = ClaudeCodeProvider::new();
        assert_eq!(p.name, "claude-code");
        assert_eq!(p.tools.len(), 8);
        assert_eq!(p.model_kinds.len(), 3);
    }
    #[test]
    fn tools_match_whitelist() {
        let p = ClaudeCodeProvider::new();
        for t in &p.tools { assert!(TOOL_WHITELIST.contains(t)); }
    }
    #[test]
    fn default_impl() {
        let p: ClaudeCodeProvider = Default::default();
        assert_eq!(p.name, "claude-code");
    }
    #[test]
    fn k1_checks_count() {
        assert_eq!(K1_CHECKS.len(), 5);
    }
}
