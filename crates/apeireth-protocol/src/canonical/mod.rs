//! The canonical interaction contract: one vocabulary for talking to any model.
//!
//! # The contract
//!
//! | Canonical concept | Type |
//! | --- | --- |
//! | Request | [`NormalizedRequest`] |
//! | Response | [`NormalizedResponse`] |
//! | Message | [`NormalizedMessage`] |
//! | Role | [`MessageRole`] |
//! | ContentBlock | [`ContentPart`] |
//! | ToolCall | [`ToolCall`] |
//! | ToolResult | [`ToolResult`] |
//! | Usage | [`NormalizedUsage`] |
//! | FinishReason | [`NormalizedFinishReason`] |
//! | StreamEvent | [`StreamEvent`] |
//! | ModelDescriptor | [`ModelDescriptor`] |
//!
//! Most of these already existed in this crate and are re-exported here rather
//! than redefined; this module adds only the three that were missing
//! ([`ToolResult`], [`StreamEvent`], [`ModelDescriptor`]) and gives the whole set
//! one importable name.
//!
//! # What this crate owns
//!
//! Translation, in both directions, between an external protocol and the types
//! above. Nothing else. [`crate::adapter::ProtocolAdapter`] is
//! `adapt_request` / `adapt_response` over `serde_json::Value`; it performs no
//! I/O and holds no state.
//!
//! # What this crate must never own
//!
//! Credentials, retry policy, routing, fallback, provider health, connection
//! pooling, quota, model selection, or ownership of an HTTP client's lifetime.
//! Those belong to the provider layer, which the runtime composes.
//!
//! This boundary is the single most important rule in the crate, and it is easy
//! to violate by accident. An adapter signature such as
//! `execute(&self, api_key: &str, req: &Request) -> Response` looks convenient
//! and quietly hands the adapter the credential, the HTTP client, the retry
//! decision, and the connection lifetime all at once — at which point every new
//! provider re-implements all four, differently. Adapters translate; they do not
//! call.
//!
//! # No raw reasoning in the contract
//!
//! No type here carries raw chain-of-thought, and none should gain one. See
//! [`stream`] and `ARCHITECTURE.md`.

pub mod model;
pub mod stream;
pub mod tool_result;

pub use model::{ModelDescriptor, ModelFeature};
pub use stream::StreamEvent;
pub use tool_result::{ToolOutcome, ToolResult};

pub use crate::normalized::{
    ContentPart, MessageRole, NormalizedFinishReason, NormalizedMessage, NormalizedRequest,
    NormalizedResponse, NormalizedTool, NormalizedToolChoice, NormalizedUsage, ToolCall,
    ToolParameters,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Walks one tool-calling turn through the contract types alone, with no
    /// provider, runtime, or transport involved. If this ever needs a type from
    /// outside this crate, the contract has a hole in it.
    #[test]
    fn the_contract_expresses_a_full_tool_calling_turn() {
        // 1. The caller asks a question and offers a tool.
        let tool = NormalizedTool {
            name: "calculator".into(),
            description: Some("Evaluate an arithmetic expression".into()),
            parameters: ToolParameters::new(),
            strict: false,
        };
        let mut request = NormalizedRequest::new(
            "fake-model-1",
            vec![NormalizedMessage::user("calculate 1+1")],
        );
        request.tools = vec![tool];

        // 2. The model answers with a tool call rather than text.
        let call = ToolCall {
            id: "call_1".into(),
            name: "calculator".into(),
            arguments: serde_json::json!({ "expr": "1+1" }),
        };
        let first = NormalizedResponse {
            id: "resp_1".into(),
            model: "fake-model-1".into(),
            content: String::new(),
            finish_reason: Some(NormalizedFinishReason::ToolCalls),
            usage: NormalizedUsage::default(),
            tool_calls: vec![call.clone()],
            raw_metadata: serde_json::Map::new(),
        };
        assert_eq!(first.finish_reason, Some(NormalizedFinishReason::ToolCalls));

        // 3. The tool runs and its result is correlated back to the call.
        let result = ToolResult::ok(&call.id, serde_json::json!("2")).with_name(&call.name);
        assert!(result.is_ok());

        // 4. Both the call and the result join the transcript.
        request
            .messages
            .push(NormalizedMessage::assistant_with_tool_calls(
                "",
                first.tool_calls,
            ));
        request.messages.push(result.into_message());

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[1].role, MessageRole::Assistant);
        assert_eq!(request.messages[2].role, MessageRole::Tool);
        assert_eq!(
            request.messages[2].tool_call_id.as_deref(),
            Some("call_1"),
            "the result must stay correlated to the call"
        );
    }
}
