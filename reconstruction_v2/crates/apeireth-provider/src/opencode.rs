//! opencode — OpenCode Provider client.
//!
//! 0 装 PASS: 真 Provider descriptor (8 tools + 3 ModelKind).
//!
//! v1 对齐 (R20 阶段 4 → R35): @opencode-ai/opencode 0.9.21 (8 工具 + 3 ModelKind).

pub struct OpencodeProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
}

impl OpencodeProvider {
    pub fn new() -> Self {
        Self {
            name: "opencode",
            tools: vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"],
            model_kinds: vec!["opencode-default", "claude-3.5-sonnet", "gpt-4o-mini"],
        }
    }
}

impl Default for OpencodeProvider { fn default() -> Self { Self::new() } }

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
    #[test]
    fn default_impl() {
        let p: OpencodeProvider = Default::default();
        assert_eq!(p.model_kinds.len(), 3);
    }
    #[test]
    fn model_kinds_default() {
        let p = OpencodeProvider::new();
        assert!(p.model_kinds.contains(&"opencode-default"));
    }
}
