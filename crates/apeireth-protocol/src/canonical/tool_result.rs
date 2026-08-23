//! The result of executing one tool call.
//!
//! [`ToolResult`] is the *value* a capability produces. [`NormalizedMessage`] with
//! [`MessageRole::Tool`] is the *message* that carries it back to the model. They
//! are separated because the runtime needs to inspect success, failure, and
//! retryability before deciding whether to feed the result to the model, retry,
//! or abort — decisions that are impossible once the outcome has been flattened
//! into a message body.

use serde::{Deserialize, Serialize};

use crate::normalized::{MessageRole, NormalizedMessage};

/// What happened when a tool call was executed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ToolOutcome {
    /// The call succeeded and produced a payload.
    Ok {
        /// Structured result. Rendered to text when handed back to a model.
        value: serde_json::Value,
    },
    /// The call failed.
    Error {
        /// Human-readable failure description. Goes back to the model, so it
        /// should say what went wrong, not merely that something did.
        message: String,
        /// Whether an identical retry could plausibly succeed. Transport
        /// timeouts are retryable; a malformed argument is not.
        retryable: bool,
    },
}

impl ToolOutcome {
    /// Whether this outcome represents success.
    pub const fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }

    /// Whether an identical retry could plausibly succeed.
    pub const fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Error {
                retryable: true,
                ..
            }
        )
    }
}

/// The outcome of one tool call, correlated back to the call that requested it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolResult {
    /// The `id` of the [`crate::normalized::ToolCall`] this answers.
    ///
    /// Providers match results to calls by this id. Getting it wrong produces a
    /// model that silently ignores the result, which is far harder to debug than
    /// an outright error.
    pub tool_call_id: String,
    /// The tool's name, when the provider's wire format requires echoing it.
    pub name: Option<String>,
    /// What happened.
    pub outcome: ToolOutcome,
}

impl ToolResult {
    /// A successful result.
    pub fn ok(tool_call_id: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            name: None,
            outcome: ToolOutcome::Ok { value },
        }
    }

    /// A failed result that a retry could plausibly fix.
    pub fn retryable_error(tool_call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            name: None,
            outcome: ToolOutcome::Error {
                message: message.into(),
                retryable: true,
            },
        }
    }

    /// A failed result that a retry cannot fix.
    pub fn permanent_error(tool_call_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            name: None,
            outcome: ToolOutcome::Error {
                message: message.into(),
                retryable: false,
            },
        }
    }

    /// Builder-style tool name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Whether the call succeeded.
    pub const fn is_ok(&self) -> bool {
        self.outcome.is_ok()
    }

    /// Render the outcome as the text a model will read.
    ///
    /// JSON strings are unwrapped rather than re-quoted: a tool returning
    /// `"42"` should reach the model as `42`, not as `"42"`.
    pub fn render(&self) -> String {
        match &self.outcome {
            ToolOutcome::Ok { value } => match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            },
            ToolOutcome::Error { message, .. } => format!("error: {message}"),
        }
    }

    /// Convert into the [`MessageRole::Tool`] message appended to the transcript.
    pub fn into_message(self) -> NormalizedMessage {
        let rendered = self.render();
        NormalizedMessage::tool_result(self.tool_call_id, self.name, rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_renders_without_re_quoting_strings() {
        let r = ToolResult::ok("call_1", serde_json::json!("2"));
        assert_eq!(r.render(), "2", "a string payload must not gain quotes");

        let r = ToolResult::ok("call_1", serde_json::json!(2));
        assert_eq!(r.render(), "2");

        let r = ToolResult::ok("call_1", serde_json::json!({ "sum": 2 }));
        assert_eq!(r.render(), r#"{"sum":2}"#);
    }

    #[test]
    fn errors_are_distinguishable_by_retryability() {
        assert!(ToolResult::retryable_error("c", "timeout")
            .outcome
            .is_retryable());
        assert!(!ToolResult::permanent_error("c", "bad argument")
            .outcome
            .is_retryable());
        assert!(!ToolResult::ok("c", serde_json::Value::Null)
            .outcome
            .is_retryable());
    }

    #[test]
    fn conversion_to_a_message_preserves_the_correlation_id() {
        let msg = ToolResult::ok("call_abc", serde_json::json!("2"))
            .with_name("calculator")
            .into_message();

        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.tool_call_id.as_deref(), Some("call_abc"));
        assert_eq!(msg.name.as_deref(), Some("calculator"));
        assert!(msg.tool_calls.is_empty(), "a result must not carry calls");
    }

    #[test]
    fn error_messages_reach_the_model_rather_than_being_swallowed() {
        let msg = ToolResult::permanent_error("call_1", "division by zero").into_message();
        let text = match &msg.content[0] {
            crate::normalized::ContentPart::Text { text } => text.clone(),
            other => panic!("expected text, got {other:?}"),
        };
        assert!(text.contains("division by zero"), "{text}");
    }

    #[test]
    fn round_trips_through_json() {
        let r = ToolResult::retryable_error("call_1", "upstream timeout").with_name("fetch");
        let back: ToolResult = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(r, back);
    }
}
