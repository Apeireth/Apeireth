//! Protocol detection from URL/path/headers.

use crate::protocol::{ProtocolHints, CompatProtocol};

/// Detect protocol from hints. Honest heuristic — not perfect for edge cases.
pub fn detect_protocol(hints: &ProtocolHints) -> CompatProtocol {
    // Path-based (most reliable)
    if let Some(path) = &hints.path {
        let p = path.to_lowercase();
        if p.contains("/v1/messages") || p.contains("/v1beta/messages") {
            return CompatProtocol::AnthropicMessages;
        }
        if p.contains("/v1/responses") {
            return CompatProtocol::OpenAIResponses;
        }
        if p.contains("/v1/chat/completions") || p.contains("/v1/completions") {
            return CompatProtocol::OpenAIChatCompletions;
        }
        if p.contains("/v1beta/models/") || p.contains(":generatecontent") {
            return CompatProtocol::Gemini;
        }
    }
    // Header-based fallback
    if hints.anthropic_version.is_some() {
        return CompatProtocol::AnthropicMessages;
    }
    if let Some(ct) = &hints.content_type {
        if ct.contains("anthropic") {
            return CompatProtocol::AnthropicMessages;
        }
    }
    CompatProtocol::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints_with_path(path: &str) -> ProtocolHints {
        ProtocolHints { path: Some(path.to_string()), ..Default::default() }
    }

    #[test]
    fn detect_anthropic_via_path() {
        let p = detect_protocol(&hints_with_path("/v1/messages"));
        assert_eq!(p, CompatProtocol::AnthropicMessages);
    }

    #[test]
    fn detect_openai_responses() {
        let p = detect_protocol(&hints_with_path("/v1/responses"));
        assert_eq!(p, CompatProtocol::OpenAIResponses);
    }

    #[test]
    fn detect_openai_chat() {
        let p = detect_protocol(&hints_with_path("/v1/chat/completions"));
        assert_eq!(p, CompatProtocol::OpenAIChatCompletions);
    }

    #[test]
    fn detect_gemini() {
        let p = detect_protocol(&hints_with_path("/v1beta/models/foo:generateContent"));
        assert_eq!(p, CompatProtocol::Gemini);
    }

    #[test]
    fn detect_unknown() {
        let p = detect_protocol(&hints_with_path("/random"));
        assert_eq!(p, CompatProtocol::Unknown);
    }

    #[test]
    fn detect_via_anthropic_version() {
        let h = ProtocolHints { anthropic_version: Some("2023-06-01".to_string()), ..Default::default() };
        let p = detect_protocol(&h);
        assert_eq!(p, CompatProtocol::AnthropicMessages);
    }
}