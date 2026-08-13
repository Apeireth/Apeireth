//! `apeireth-provider::codex` — OpenAI Codex Provider client
//!
//! **R35 合并**: 1 个 apeireth-provider 主 crate, 5 module 各自 1 个 Provider struct.
//! 5 个老 crate (`apeireth-provider-codex` 等) 留作 shell, 仍 re-export 本 module.
//!
//! **R20 阶段 4 历史**: 1:1 翻译 @openai/codex 0.9.21 (8 工具 + 4 ModelKind + 3 SandboxType).
//! R35 阶段 1: 只 1 个 Provider struct + 8 工具 + 4 ModelKind. R21+ 真接 SDK.

#![allow(missing_docs)] // R163 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
#![allow(dead_code)]

/// R35: CodexProvider struct (R20 阶段 4 1:1 字段名)
pub struct CodexProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            name: "codex",
            tools: vec![
                "Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch",
            ],
            // 4 ModelKind (per R20 阶段 4 task spec §3, codex 4 vs claude-code 3)
            model_kinds: vec!["codex", "codex-mini", "o3", "o4-mini"],
        }
    }
}

impl Default for CodexProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// R35: 3 SandboxType (per R20 阶段 4 task spec §3 codex 特有)
pub const SANDBOX_TYPES: [&str; 3] = [
    "workspace-write",
    "read-only",
    "danger-full-access",
];

#[cfg(test)]
mod codex_tests {
    use super::*;

    #[test]
    fn codex_provider_basics() {
        let p = CodexProvider::new();
        assert_eq!(p.name, "codex");
        assert_eq!(p.tools.len(), 8);
        assert_eq!(p.model_kinds.len(), 4, "codex 4 vs claude 3");
    }

    #[test]
    fn codex_sandbox_types_3() {
        assert_eq!(SANDBOX_TYPES.len(), 3);
    }
}
