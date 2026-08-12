//! VCP protocol enum + detection hints.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompatProtocol {
    OpenAIChatCompletions,
    AnthropicMessages,
    OpenAIResponses,
    Gemini,
    Unknown,
}

impl CompatProtocol {
    pub fn name(&self) -> &'static str {
        match self {
            CompatProtocol::OpenAIChatCompletions => "openai-chat-completions",
            CompatProtocol::AnthropicMessages => "anthropic-messages",
            CompatProtocol::OpenAIResponses => "openai-responses",
            CompatProtocol::Gemini => "gemini",
            CompatProtocol::Unknown => "unknown",
        }
    }
    pub fn all() -> &'static [CompatProtocol] {
        &[CompatProtocol::OpenAIChatCompletions, CompatProtocol::AnthropicMessages, CompatProtocol::OpenAIResponses, CompatProtocol::Gemini]
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProtocolHints {
    pub path: Option<String>,
    pub anthropic_version: Option<String>,
    pub content_type: Option<String>,
    pub user_agent: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_count() {
        assert_eq!(CompatProtocol::all().len(), 4);
    }

    #[test]
    fn protocol_names_unique() {
        let names: Vec<_> = CompatProtocol::all().iter().map(|p| p.name()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len());
    }

    #[test]
    fn default_hints_empty() {
        let h = ProtocolHints::default();
        assert!(h.path.is_none());
    }
}