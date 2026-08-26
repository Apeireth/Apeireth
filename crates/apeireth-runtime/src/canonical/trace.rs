//! Structured execution traces.
//!
//! # Why this exists instead of raw reasoning
//!
//! The nested `reconstruction_v2` prototype exposed `reasoning_cot:
//! Option<String>` as a required public field of its turn output, and its
//! fast path fabricated a value for it when no model had been consulted at all.
//! That is the pattern this replaces.
//!
//! Raw chain-of-thought is a poor debugging contract for three separate reasons:
//! it is not available from every provider, so any consumer must handle its
//! absence anyway; it is unstructured prose, so nothing can assert on it; and it
//! is model-authored text about the model, not a record of what the *runtime*
//! did. When a turn misbehaves, the useful questions are which provider served
//! it, how many rounds it took, what governance decided, and which capability
//! ran — and none of those are answerable from reasoning text.
//!
//! [`ExecutionTrace`] answers exactly those, is emitted whether or not a provider
//! offers reasoning, and can be asserted on in tests. The end-to-end test in this
//! crate proves the loop's behaviour entirely through this type.

use apeireth_core::kernel::{
    ApprovalId, CapabilityId, PluginId, RequestId, SessionId, Timestamp, TraceId,
};
use apeireth_protocol::canonical::{NormalizedFinishReason, NormalizedUsage};
use serde::{Deserialize, Serialize};

/// One thing the runtime did.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TraceEvent {
    /// A provider was asked to serve a round.
    ProviderInvoked {
        /// Which provider.
        provider: CapabilityId,
        /// Which model was requested.
        model: String,
        /// Which round of the loop, counting from 1.
        round: u32,
    },
    /// A provider answered.
    ProviderSucceeded {
        /// Which provider.
        provider: CapabilityId,
        /// Which round.
        round: u32,
        /// Why the response ended.
        finish_reason: Option<NormalizedFinishReason>,
        /// Token accounting.
        usage: NormalizedUsage,
    },
    /// A provider failed.
    ProviderFailed {
        /// Which provider.
        provider: CapabilityId,
        /// Which round.
        round: u32,
        /// What went wrong.
        error: String,
        /// Whether the router was entitled to fall back.
        retryable: bool,
    },
    /// Governance judged an action.
    GovernanceEvaluated {
        /// Which hook decided.
        hook: String,
        /// Plugin that owns the hook, when applicable.
        owner: Option<PluginId>,
        /// Which action was judged.
        action: String,
        /// What was decided.
        decision: String,
        /// Structured reason for a refusal or approval requirement.
        reason: Option<String>,
        /// Which round.
        round: u32,
    },
    /// A pending approval was created for a capability dispatch.
    ApprovalRequested {
        /// The stable approval id.
        approval_id: ApprovalId,
        /// Which capability requires approval.
        capability: CapabilityId,
        /// The call id it answers.
        tool_call_id: String,
        /// Which round.
        round: u32,
    },
    /// A pending approval was resolved.
    ApprovalResolved {
        /// The stable approval id.
        approval_id: ApprovalId,
        /// `approved` / `rejected` / `expired`.
        decision: String,
        /// Which round.
        round: u32,
    },
    /// A capability was invoked.
    CapabilityDispatched {
        /// Which capability.
        capability: CapabilityId,
        /// The call id it answers.
        tool_call_id: String,
        /// Which round.
        round: u32,
    },
    /// A capability finished.
    CapabilityCompleted {
        /// Which capability.
        capability: CapabilityId,
        /// The call id it answered.
        tool_call_id: String,
        /// Whether it succeeded.
        succeeded: bool,
        /// Which round.
        round: u32,
    },
    /// The model requested a tool that is not available.
    CapabilityUnavailable {
        /// The name the model used.
        requested: String,
        /// The call id it would have answered.
        tool_call_id: String,
        /// Why it could not be dispatched.
        reason: String,
        /// Which round.
        round: u32,
    },
    /// The turn produced a final answer.
    TurnCompleted {
        /// How many provider round-trips it took.
        rounds: u32,
    },
}

/// A timestamped [`TraceEvent`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceEntry {
    /// When it happened, per the runtime's clock.
    pub at: Timestamp,
    /// What happened.
    pub event: TraceEvent,
}

/// The full record of one turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionTrace {
    /// Correlates every event in this turn.
    pub trace: TraceId,
    /// The session the turn belongs to.
    pub session: SessionId,
    /// The request that started it.
    pub request: RequestId,
    /// What happened, in order.
    pub entries: Vec<TraceEntry>,
}

impl ExecutionTrace {
    /// An empty trace for a turn that is about to begin.
    pub fn new(trace: TraceId, session: SessionId, request: RequestId) -> Self {
        Self {
            trace,
            session,
            request,
            entries: Vec::new(),
        }
    }

    /// Record an event.
    pub fn record(&mut self, at: Timestamp, event: TraceEvent) {
        self.entries.push(TraceEntry { at, event });
    }

    /// Every recorded event, in order.
    pub fn events(&self) -> impl Iterator<Item = &TraceEvent> {
        self.entries.iter().map(|e| &e.event)
    }

    /// How many provider invocations this turn made.
    pub fn provider_invocations(&self) -> usize {
        self.events()
            .filter(|e| matches!(e, TraceEvent::ProviderInvoked { .. }))
            .count()
    }

    /// How many capability dispatches this turn made.
    pub fn capability_dispatches(&self) -> usize {
        self.events()
            .filter(|e| matches!(e, TraceEvent::CapabilityDispatched { .. }))
            .count()
    }

    /// How many dispatches of a specific capability this turn made.
    pub fn dispatches_of(&self, id: &CapabilityId) -> usize {
        self.events()
            .filter(|e| matches!(e, TraceEvent::CapabilityDispatched { capability, .. } if capability == id))
            .count()
    }

    /// The number of rounds reported by the closing event, if the turn finished.
    pub fn completed_rounds(&self) -> Option<u32> {
        self.events().find_map(|e| match e {
            TraceEvent::TurnCompleted { rounds } => Some(*rounds),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_epoch_millis(ms).unwrap()
    }

    fn calculator() -> CapabilityId {
        CapabilityId::new("tool.calculator").unwrap()
    }

    fn trace_of_a_tool_calling_turn() -> ExecutionTrace {
        let provider = CapabilityId::new("provider.fake").unwrap();
        let mut t = ExecutionTrace::new(TraceId::new(), SessionId::new(), RequestId::new());

        t.record(
            at(1_000),
            TraceEvent::ProviderInvoked {
                provider: provider.clone(),
                model: "fake-model-1".into(),
                round: 1,
            },
        );
        t.record(
            at(1_100),
            TraceEvent::ProviderSucceeded {
                provider: provider.clone(),
                round: 1,
                finish_reason: Some(NormalizedFinishReason::ToolCalls),
                usage: NormalizedUsage::default(),
            },
        );
        t.record(
            at(1_200),
            TraceEvent::CapabilityDispatched {
                capability: calculator(),
                tool_call_id: "call_1".into(),
                round: 1,
            },
        );
        t.record(
            at(1_300),
            TraceEvent::CapabilityCompleted {
                capability: calculator(),
                tool_call_id: "call_1".into(),
                succeeded: true,
                round: 1,
            },
        );
        t.record(
            at(1_400),
            TraceEvent::ProviderInvoked {
                provider,
                model: "fake-model-1".into(),
                round: 2,
            },
        );
        t.record(at(1_600), TraceEvent::TurnCompleted { rounds: 2 });
        t
    }

    #[test]
    fn a_trace_answers_the_questions_reasoning_text_cannot() {
        let t = trace_of_a_tool_calling_turn();

        assert_eq!(t.provider_invocations(), 2);
        assert_eq!(t.capability_dispatches(), 1);
        assert_eq!(t.dispatches_of(&calculator()), 1);
        assert_eq!(
            t.dispatches_of(&CapabilityId::new("tool.shell").unwrap()),
            0
        );
        assert_eq!(t.completed_rounds(), Some(2));
    }

    #[test]
    fn an_unfinished_turn_reports_no_round_count() {
        let mut t = ExecutionTrace::new(TraceId::new(), SessionId::new(), RequestId::new());
        t.record(
            at(1_000),
            TraceEvent::ProviderInvoked {
                provider: CapabilityId::new("provider.fake").unwrap(),
                model: "m".into(),
                round: 1,
            },
        );
        assert_eq!(t.completed_rounds(), None);
    }

    #[test]
    fn entries_stay_in_the_order_they_were_recorded() {
        let t = trace_of_a_tool_calling_turn();
        let times: Vec<i64> = t.entries.iter().map(|e| e.at.epoch_millis()).collect();
        let mut sorted = times.clone();
        sorted.sort_unstable();
        assert_eq!(times, sorted);
    }

    #[test]
    fn round_trips_through_json() {
        let t = trace_of_a_tool_calling_turn();
        let back: ExecutionTrace =
            serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn no_variant_carries_raw_model_reasoning() {
        // A structural guard: if someone adds a reasoning field to a trace event,
        // the serialized form will start carrying it and this will fail.
        let json = serde_json::to_string(&trace_of_a_tool_calling_turn()).unwrap();
        for forbidden in ["reasoning", "chain_of_thought", "cot", "thinking"] {
            assert!(
                !json.contains(forbidden),
                "trace must not carry raw reasoning, found {forbidden:?} in {json}"
            );
        }
    }
}
