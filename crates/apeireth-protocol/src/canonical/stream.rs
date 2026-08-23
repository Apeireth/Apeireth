//! Normalized streaming events from a provider.
//!
//! One vocabulary for OpenAI SSE deltas, Anthropic `content_block_delta`, and
//! Gemini chunks, so that consumers never branch on vendor.
//!
//! Distinct from [`crate::ws_v1::WsFrame`], which is Apeireth's own client-facing
//! WebSocket wire format. This type describes what a *provider* streams *to* the
//! runtime; `WsFrame` describes what the gateway streams to a client. Collapsing
//! them would couple every provider to the gateway's wire format.
//!
//! # No raw reasoning
//!
//! There is deliberately no `ReasoningDelta` variant. Raw chain-of-thought is not
//! part of the canonical contract; see `ARCHITECTURE.md`. Providers that emit it
//! drop it, and diagnostics use structured traces instead.

use serde::{Deserialize, Serialize};

use crate::normalized::{NormalizedFinishReason, NormalizedUsage};

/// One event in a provider's response stream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum StreamEvent {
    /// The provider accepted the request and began responding.
    Started {
        /// Provider-assigned response id.
        id: String,
        /// The model actually serving the request, which a router may have
        /// substituted for the one requested.
        model: String,
    },
    /// A fragment of assistant text.
    TextDelta {
        /// The fragment. Concatenating every `TextDelta` in order reproduces the
        /// full text; fragments are not individually meaningful and may split a
        /// UTF-8 grapheme cluster or a word.
        text: String,
    },
    /// A fragment of a tool call being assembled.
    ///
    /// Providers stream tool calls incrementally: the id and name usually arrive
    /// once, then `arguments` accumulate over many events. `index` identifies
    /// which call is being extended when several are requested in parallel.
    ToolCallDelta {
        /// Position of this call within the response's tool-call list.
        index: u32,
        /// Call id, typically present only on the first delta for this index.
        id: Option<String>,
        /// Tool name, typically present only on the first delta for this index.
        name: Option<String>,
        /// Fragment of the JSON argument string. Not valid JSON on its own.
        arguments_delta: String,
    },
    /// Token accounting, usually emitted once near the end.
    Usage {
        /// The counts.
        usage: NormalizedUsage,
    },
    /// The response ended. Terminal.
    Finished {
        /// Why it ended.
        reason: NormalizedFinishReason,
    },
    /// The stream failed. Terminal.
    Error {
        /// What went wrong.
        message: String,
    },
}

impl StreamEvent {
    /// Whether no further events will follow.
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Finished { .. } | Self::Error { .. })
    }

    /// The text fragment, when this event carries one.
    pub fn text_delta(&self) -> Option<&str> {
        match self {
            Self::TextDelta { text } => Some(text),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concatenating_text_deltas_reproduces_the_message() {
        let events = [
            StreamEvent::Started {
                id: "resp_1".into(),
                model: "fake-model-1".into(),
            },
            StreamEvent::TextDelta {
                text: "The answer ".into(),
            },
            StreamEvent::TextDelta {
                text: "is 2.".into(),
            },
            StreamEvent::Finished {
                reason: NormalizedFinishReason::Stop,
            },
        ];

        let text: String = events.iter().filter_map(StreamEvent::text_delta).collect();
        assert_eq!(text, "The answer is 2.");
    }

    #[test]
    fn only_finished_and_error_are_terminal() {
        assert!(StreamEvent::Finished {
            reason: NormalizedFinishReason::Stop
        }
        .is_terminal());
        assert!(StreamEvent::Error {
            message: "upstream reset".into()
        }
        .is_terminal());

        assert!(!StreamEvent::TextDelta { text: "x".into() }.is_terminal());
        assert!(!StreamEvent::Usage {
            usage: NormalizedUsage::default()
        }
        .is_terminal());
    }

    #[test]
    fn tool_call_deltas_carry_identity_once_and_arguments_repeatedly() {
        let deltas = [
            StreamEvent::ToolCallDelta {
                index: 0,
                id: Some("call_1".into()),
                name: Some("calculator".into()),
                arguments_delta: r#"{"expr":"#.into(),
            },
            StreamEvent::ToolCallDelta {
                index: 0,
                id: None,
                name: None,
                arguments_delta: r#""1+1"}"#.into(),
            },
        ];

        let mut args = String::new();
        for d in &deltas {
            if let StreamEvent::ToolCallDelta {
                arguments_delta, ..
            } = d
            {
                args.push_str(arguments_delta);
            }
        }
        assert_eq!(args, r#"{"expr":"1+1"}"#);
        let parsed: serde_json::Value = serde_json::from_str(&args).unwrap();
        assert_eq!(parsed["expr"], "1+1");
    }

    #[test]
    fn round_trips_through_json_with_a_tagged_representation() {
        let ev = StreamEvent::TextDelta {
            text: "hello".into(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert_eq!(json, r#"{"event":"text_delta","text":"hello"}"#);

        let back: StreamEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(ev, back);
    }
}
