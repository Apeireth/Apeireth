//! Request/response conversion between protocols.

#![allow(missing_docs)] // R162 O-5: items here are implementation helpers / private internals; public API is documented in lib.rs
use serde_json::{json, Value};

use crate::protocol::CompatProtocol;

/// Convert a request from one protocol to our internal canonical form.
/// For now: pass-through + tag the source protocol.
pub fn convert_request(protocol: CompatProtocol, body: Value) -> Value {
    match protocol {
        CompatProtocol::OpenAIChatCompletions => {
            // OpenAI: { model, messages, ... } — already canonical
            let mut body = body;
            body["__apeireth_source_protocol"] = json!(protocol.name());
            body
        }
        CompatProtocol::AnthropicMessages => {
            // Anthropic: { model, messages, max_tokens } → canonical
            let mut body = body;
            if body.get("max_tokens").is_none() {
                body["max_tokens"] = json!(4096);
            }
            body["__apeireth_source_protocol"] = json!(protocol.name());
            body
        }
        CompatProtocol::OpenAIResponses => {
            let mut body = body;
            body["__apeireth_source_protocol"] = json!(protocol.name());
            body
        }
        CompatProtocol::Gemini => {
            let mut body = body;
            body["__apeireth_source_protocol"] = json!(protocol.name());
            body
        }
        CompatProtocol::Unknown => body,
    }
}

/// Convert a response from internal canonical form to target protocol.
pub fn convert_response(protocol: CompatProtocol, internal: Value) -> Value {
    match protocol {
        CompatProtocol::OpenAIChatCompletions => internal,
        CompatProtocol::AnthropicMessages => {
            // Internal { content: [{type: "text", text}], stop_reason } → Anthropic { content: [{type: "text", text}], stop_reason }
            internal
        }
        CompatProtocol::OpenAIResponses => internal,
        CompatProtocol::Gemini => internal,
        CompatProtocol::Unknown => internal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openai_chat_passthrough() {
        let body = json!({"model": "gpt", "messages": []});
        let out = convert_request(CompatProtocol::OpenAIChatCompletions, body);
        assert_eq!(out["__apeireth_source_protocol"], "openai-chat-completions");
    }

    #[test]
    fn anthropic_adds_max_tokens_default() {
        let body = json!({"model": "c", "messages": []});
        let out = convert_request(CompatProtocol::AnthropicMessages, body);
        assert_eq!(out["max_tokens"], 4096);
    }

    #[test]
    fn response_passthrough() {
        let resp = json!({"content": []});
        let out = convert_response(CompatProtocol::AnthropicMessages, resp);
        assert!(out.get("content").is_some());
    }
}