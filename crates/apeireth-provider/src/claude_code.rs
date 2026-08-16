//! `apeireth-provider::claude_code` — Anthropic Claude Code Provider client
//!
//! **R35 合并**: 1 个 apeireth-provider 主 crate, 5 module 各自 1 个 Provider struct.
//! 5 个老 crate (`apeireth-provider-claude-code` 等) 留作 shell, 仍 re-export 本 module.
//!
//! **R20 阶段 4 历史**: 1:1 翻译 @anthropic-ai/claude-agent-sdk 0.2.112 (8 工具 + 3 ModelKind + 8 TOOL_WHITELIST + 5 K-1 强校验),
//! 实际是 0.1.0 skeleton, R21+ 续真 (per task spec).
//! R35 阶段 1 不重写 R20 内容 (那是 R21 任务), 只在 5 module 各放 1 个 Provider struct stub.

#![allow(dead_code)]

/// R35: ClaudeCodeProvider struct (R20 阶段 4 1:1 字段名, R21+ 真接 @anthropic-ai/claude-agent-sdk)
pub struct ClaudeCodeProvider {
    /// provider 名, 跟 R20 阶段 4 owner 1:1
    pub name: &'static str,
    /// 8 工具 (R21+ 真接后填)
    pub tools: Vec<&'static str>,
    /// 3 ModelKind (R21+ 真接后填)
    pub model_kinds: Vec<&'static str>,
}

impl ClaudeCodeProvider {
    /// R35: 构造, name = "claude-code" (跟 R20 owner name 1:1)
    pub fn new() -> Self {
        Self {
            name: "claude-code",
            tools: vec![
                "Read",
                "Write",
                "Edit",
                "Bash",
                "Glob",
                "Grep",
                "WebSearch",
                "WebFetch",
            ],
            model_kinds: vec!["claude-opus-4-5", "claude-sonnet-4-5", "claude-haiku-4-5"],
        }
    }
}

impl Default for ClaudeCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// R35: 8 TOOL_WHITELIST (per R20 阶段 4 task spec §3)
pub const TOOL_WHITELIST: [&str; 8] = [
    "Read",
    "Write",
    "Edit",
    "Bash",
    "Glob",
    "Grep",
    "WebSearch",
    "WebFetch",
];

/// R35: 5 K-1 强校验 (per R20 阶段 4 task spec §3)
pub const K1_CHECKS: [&str; 5] = [
    "base_url_not_empty",
    "auth_token_format",
    "tool_in_whitelist",
    "args_is_object",
    "timeout_positive",
];

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
    fn claude_code_tool_whitelist_8() {
        assert_eq!(TOOL_WHITELIST.len(), 8);
        assert!(TOOL_WHITELIST.contains(&"Read"));
        assert!(TOOL_WHITELIST.contains(&"Bash"));
    }

    #[test]
    fn claude_code_k1_checks_5() {
        assert_eq!(K1_CHECKS.len(), 5);
    }
}
