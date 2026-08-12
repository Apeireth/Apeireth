//! VCP protocol enum + detection hints.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VcpProtocol {
    OpenAIChatCompletions,
    AnthropicMessages,
    OpenAIResponses,
    Gemini,
    Unknown,
}

impl VcpProtocol {
    pub fn name(&self) -> &'static str {
        match self {
            VcpProtocol::OpenAIChatCompletions => "openai-chat-completions",
            VcpProtocol::AnthropicMessages => "anthropic-messages",
            VcpProtocol::OpenAIResponses => "openai-responses",
            VcpProtocol::Gemini => "gemini",
            VcpProtocol::Unknown => "unknown",
        }
    }
    pub fn all() -> &'static [VcpProtocol] {
        &[VcpProtocol::OpenAIChatCompletions, VcpProtocol::AnthropicMessages, VcpProtocol::OpenAIResponses, VcpProtocol::Gemini]
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
        assert_eq!(VcpProtocol::all().len(), 4);
    }

    #[test]
    fn protocol_names_unique() {
        let names: Vec<_> = VcpProtocol::all().iter().map(|p| p.name()).collect();
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