//! `apeireth-provider::gemini_cli` — Google Gemini CLI Provider client
//!
//! **R35 合并**: 1 个 apeireth-provider 主 crate, 5 module 各自 1 个 Provider struct.
//!
//! **R20 阶段 4 历史**: 1:1 翻译 @google/gemini-cli 0.9.21 (8 工具 + 3 ModelKind + Embedding).
//! R35 阶段 1: 只 Provider struct stub. R21+ 真接 SDK.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#![allow(dead_code)]

/// R35: GeminiCliProvider struct (R20 阶段 4 1:1 字段名)
pub struct GeminiCliProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
    /// 3 Embedding 维度 (per R20 阶段 4 task spec §3 gemini 特有)
    pub embedding_dim: u16,
}

impl GeminiCliProvider {
    pub fn new() -> Self {
        Self {
            name: "gemini-cli",
            tools: vec![
                "Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch",
            ],
            model_kinds: vec!["gemini-1.5-pro", "gemini-1.5-flash", "gemini-2.0-flash"],
            embedding_dim: 768,
        }
    }
}

impl Default for GeminiCliProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod gemini_cli_tests {
    use super::*;

    #[test]
    fn gemini_cli_provider_basics() {
        let p = GeminiCliProvider::new();
        assert_eq!(p.name, "gemini-cli");
        assert_eq!(p.tools.len(), 8);
        assert_eq!(p.model_kinds.len(), 3);
        assert_eq!(p.embedding_dim, 768);
    }
}
