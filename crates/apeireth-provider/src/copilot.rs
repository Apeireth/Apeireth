//! `apeireth-provider::copilot` — GitHub Copilot Provider client
//!
//! **R35 合并**: 1 个 apeireth-provider 主 crate, 5 module 各自 1 个 Provider struct.
//!
//! **R20 阶段 4 历史**: 1:1 翻译 @github/copilot-sdk 0.9.21 (8 工具 + 3 ModelKind + OAuth).
//! R35 阶段 1: 只 Provider struct stub. R21+ 真接 SDK.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#![allow(dead_code)]

/// R35: CopilotProvider struct (R20 阶段 4 1:1 字段名)
pub struct CopilotProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
    /// OAuth 走 GitHub Device Flow (per R20 阶段 4 task spec §3)
    pub oauth_required: bool,
}

impl CopilotProvider {
    pub fn new() -> Self {
        Self {
            name: "copilot",
            tools: vec![
                "Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch",
            ],
            model_kinds: vec!["gpt-4o", "gpt-4o-mini", "claude-3.5-sonnet"],
            oauth_required: true,
        }
    }
}

impl Default for CopilotProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod copilot_tests {
    use super::*;

    #[test]
    fn copilot_provider_basics() {
        let p = CopilotProvider::new();
        assert_eq!(p.name, "copilot");
        assert_eq!(p.tools.len(), 8);
        assert_eq!(p.model_kinds.len(), 3);
        assert!(p.oauth_required, "copilot 强制 OAuth");
    }
}
