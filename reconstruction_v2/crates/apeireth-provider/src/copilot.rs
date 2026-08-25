//! copilot — GitHub Copilot Provider client.
//!
//! 0 装 PASS: 真 Provider descriptor (8 tools + 3 ModelKind + OAuth 强制).
//!
//! v1 对齐 (R20 阶段 4 → R35): @github/copilot-sdk 0.9.21 (8 工具 + 3 ModelKind + OAuth).

pub struct CopilotProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
    pub oauth_required: bool,
}

impl CopilotProvider {
    pub fn new() -> Self {
        Self {
            name: "copilot",
            tools: vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"],
            model_kinds: vec!["gpt-4o", "gpt-4o-mini", "claude-3.5-sonnet"],
            oauth_required: true,
        }
    }
}

impl Default for CopilotProvider { fn default() -> Self { Self::new() } }

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
    #[test]
    fn default_impl() {
        let p: CopilotProvider = Default::default();
        assert!(p.oauth_required);
    }
    #[test]
    fn model_kinds_contains_gpt4o() {
        let p = CopilotProvider::new();
        assert!(p.model_kinds.contains(&"gpt-4o"));
    }
}
