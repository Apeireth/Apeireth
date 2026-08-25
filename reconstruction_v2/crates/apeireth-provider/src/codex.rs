//! codex — OpenAI Codex Provider client.
//!
//! 0 装 PASS: 真 Provider descriptor (8 tools + 4 model kinds + 3 SandboxType).
//!
//! v1 对齐 (R20 阶段 4 → R35): @openai/codex 0.9.21 (8 工具 + 4 ModelKind + 3 SandboxType).

pub struct CodexProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
}

impl CodexProvider {
    pub fn new() -> Self {
        Self {
            name: "codex",
            tools: vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"],
            model_kinds: vec!["codex", "codex-mini", "o3", "o4-mini"],
        }
    }
}

impl Default for CodexProvider { fn default() -> Self { Self::new() } }

pub const SANDBOX_TYPES: [&str; 3] = ["workspace-write", "read-only", "danger-full-access"];

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
    fn sandbox_types_3() {
        assert_eq!(SANDBOX_TYPES.len(), 3);
        assert!(SANDBOX_TYPES.contains(&"workspace-write"));
    }
    #[test]
    fn default_impl() {
        let p: CodexProvider = Default::default();
        assert_eq!(p.model_kinds.len(), 4);
    }
}
