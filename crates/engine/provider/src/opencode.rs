//! `apeireth-provider::opencode` — OpenCode Provider client
//!
//! **R35 合并**: 1 个 apeireth-provider 主 crate, 5 module 各自 1 个 Provider struct.
//!
//! **R20 阶段 4 历史**: 1:1 翻译 @opencode-ai/opencode 0.9.21 (8 工具 + 3 ModelKind).
//! R35 阶段 1: 只 Provider struct stub. R21+ 真接 SDK.

#![allow(missing_docs)]
// R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#![allow(dead_code)]

/// R35: OpencodeProvider struct (R20 阶段 4 1:1 字段名)
pub struct OpencodeProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
}

impl OpencodeProvider {
    pub fn new() -> Self {
        Self {
            name: "opencode",
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
            model_kinds: vec!["opencode-default", "claude-3.5-sonnet", "gpt-4o-mini"],
        }
    }
}

impl Default for OpencodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod opencode_tests {
    use super::*;

    #[test]
    fn opencode_provider_basics() {
        let p = OpencodeProvider::new();
        assert_eq!(p.name, "opencode");
        assert_eq!(p.tools.len(), 8);
        assert_eq!(p.model_kinds.len(), 3);
    }
}
