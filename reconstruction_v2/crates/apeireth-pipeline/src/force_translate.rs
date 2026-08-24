//! Force translate base64 image to text (VCP §6.2.2 #20).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceTranslateConfig {
    pub enabled: bool,
    pub text_only_tags: Vec<String>,
    pub replacement_tag: String,
}

impl ForceTranslateConfig {
    pub fn chat_default() -> Self {
        Self {
            enabled: true,
            text_only_tags: vec!["text-only".into(), "text".into()],
            replacement_tag: "[image stripped]".into(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct ForceTranslateStats {
    pub translated: usize,
    pub kept: usize,
}

/// Check if messages contain base64 media.
pub fn messages_contain_base64_media(text: &str) -> bool {
    text.contains("data:image/") || text.contains("data:audio/") || text.contains("base64,")
}

/// Check if model is text-only.
pub fn is_text_only_model_by_tag(tags: &[String], cfg: &ForceTranslateConfig) -> bool {
    tags.iter().any(|t| cfg.text_only_tags.contains(t))
}

/// Check if force translate is needed.
pub fn needs_force_translate(text: &str, cfg: &ForceTranslateConfig) -> bool {
    cfg.enabled && messages_contain_base64_media(text)
}

/// Force translate if needed.
pub fn force_translate_if_needed(text: &str, cfg: &ForceTranslateConfig) -> String {
    if needs_force_translate(text, cfg) {
        // Strip data URL prefixes, replace with replacement tag
        let re = regex::Regex::new(r"data:[a-z]+/[a-z0-9.+-]+;base64,[A-Za-z0-9+/=]+").unwrap();
        re.replace_all(text, cfg.replacement_tag.as_str()).to_string()
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_translate_when_disabled() {
        let mut cfg = ForceTranslateConfig::chat_default();
        cfg.enabled = false;
        let r = force_translate_if_needed("data:image/png;base64,abc", &cfg);
        assert!(r.contains("data:image/png"));
    }

    #[test]
    fn translate_strips_base64() {
        let cfg = ForceTranslateConfig::chat_default();
        let r = force_translate_if_needed("hello data:image/png;base64,abc world", &cfg);
        assert!(!r.contains("base64"));
        assert!(r.contains("[image stripped]"));
    }

    #[test]
    fn needs_translate_logic() {
        let cfg = ForceTranslateConfig::chat_default();
        assert!(needs_force_translate("data:image/png;base64,x", &cfg));
        assert!(!needs_force_translate("hello world", &cfg));
    }

    #[test]
    fn text_only_tag_check() {
        let cfg = ForceTranslateConfig::chat_default();
        assert!(is_text_only_model_by_tag(&vec!["text-only".into()], &cfg));
        assert!(!is_text_only_model_by_tag(&vec!["vision".into()], &cfg));
    }
}
