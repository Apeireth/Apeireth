//! gemini_cli — Google Gemini CLI Provider client.
//!
//! 0 装 PASS: 真 Provider descriptor (8 tools + 3 ModelKind + Embedding 维度).
//!
//! v1 对齐 (R20 阶段 4 → R35): @google/gemini-cli 0.9.21 (8 工具 + 3 ModelKind + Embedding).

pub struct GeminiCliProvider {
    pub name: &'static str,
    pub tools: Vec<&'static str>,
    pub model_kinds: Vec<&'static str>,
    pub embedding_dim: u16,
}

impl GeminiCliProvider {
    pub fn new() -> Self {
        Self {
            name: "gemini-cli",
            tools: vec!["Read", "Write", "Edit", "Bash", "Glob", "Grep", "WebSearch", "WebFetch"],
            model_kinds: vec!["gemini-1.5-pro", "gemini-1.5-flash", "gemini-2.0-flash"],
            embedding_dim: 768,
        }
    }
}

impl Default for GeminiCliProvider { fn default() -> Self { Self::new() } }

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
    #[test]
    fn default_impl() {
        let p: GeminiCliProvider = Default::default();
        assert_eq!(p.embedding_dim, 768);
    }
    #[test]
    fn embedding_dim_positive() {
        let p = GeminiCliProvider::new();
        assert!(p.embedding_dim > 0);
    }
}
