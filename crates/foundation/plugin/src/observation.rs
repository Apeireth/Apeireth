//! Generic tool-completion observation port.
//!
//! The runtime kernel publishes exactly one fact per executed tool call at the
//! same point where it records the canonical `CapabilityCompleted` trace entry.
//! The fact carries the finished [`ToolResult`] plus stable identities; it is an
//! in-process observation channel, not a serialized event stream and not a
//! dispatch surface.
//!
//! Domain owners (nociception, recovery bookkeeping, telemetry adapters)
//! implement this trait and register it with the runtime. The kernel does not
//! name any domain, configuration flag, or tool. A runtime with no observer
//! installed performs no extra work and its events/traces are unchanged.

use apeireth_core::kernel::{CapabilityId, RequestId, SessionId, Timestamp, TraceId};
use apeireth_protocol::canonical::{ToolCall, ToolResult};

/// One completed tool-call fact.
///
/// Borrowed for the duration of the call: an observer must not retain
/// references or spawn work that outlives `observe`.
#[derive(Debug, Clone, Copy)]
pub struct ToolCompletionFact<'a> {
    /// Session whose turn dispatched the call.
    pub session: SessionId,
    /// Request identity of the turn.
    pub request: RequestId,
    /// Trace identity of the turn.
    pub trace: TraceId,
    /// Logical round inside the turn.
    pub round: u32,
    /// The exact call that completed.
    pub call: &'a ToolCall,
    /// Resolved capability identity for the call.
    pub capability: &'a CapabilityId,
    /// The finished result (success or failure).
    pub result: &'a ToolResult,
    /// Runtime clock timestamp of completion.
    pub at: Timestamp,
}

/// Observes completed tool calls.
///
/// Implementations must be cheap, must not block, and must never influence the
/// dispatch result: the runtime calls this after the canonical events are
/// already recorded and discards anything the observer sees.
pub trait ToolCompletionObserver: Send + Sync {
    /// Consume one completion fact.
    fn observe(&self, fact: &ToolCompletionFact<'_>);
}

/// Convenience adapter for a closure-owned observer.
pub struct FnToolCompletionObserver<F>(pub F);

impl<F> ToolCompletionObserver for FnToolCompletionObserver<F>
where
    F: Fn(&ToolCompletionFact<'_>) + Send + Sync,
{
    fn observe(&self, fact: &ToolCompletionFact<'_>) {
        (self.0)(fact);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apeireth_protocol::canonical::ToolCall;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_closure_observer_sees_the_exact_fact_once() {
        let seen = AtomicUsize::new(0);
        let observer = FnToolCompletionObserver(|fact: &ToolCompletionFact<'_>| {
            assert_eq!(fact.capability.as_str(), "tool.echo");
            assert_eq!(fact.call.name, "echo");
            seen.fetch_add(1, Ordering::SeqCst);
        });
        let call = ToolCall {
            id: "call_1".into(),
            name: "echo".into(),
            arguments: serde_json::json!({}),
        };
        let capability = CapabilityId::new("tool.echo").expect("capability id");
        let result = ToolResult::ok("call_1", serde_json::json!("ok"));
        let fact = ToolCompletionFact {
            session: SessionId::new(),
            request: RequestId::new(),
            trace: TraceId::new(),
            round: 1,
            call: &call,
            capability: &capability,
            result: &result,
            at: Timestamp::from_epoch_millis(1_700_000_000_000).expect("timestamp"),
        };
        observer.observe(&fact);
        assert_eq!(seen.load(Ordering::SeqCst), 1);
    }
}
