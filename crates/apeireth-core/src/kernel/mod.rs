//! Canonical primitives — the stable vocabulary every Apeireth subsystem shares.
//!
//! # What belongs here
//!
//! Identifiers, time, lifecycle, events, annotations, and the errors those
//! produce. Nothing else. The test for membership is: *would two unrelated
//! subsystems both need this in order to talk to each other?* If only one
//! subsystem needs it, it belongs to that subsystem.
//!
//! # What must never be added here
//!
//! HTTP, LLM implementations, SQLite, tools, memory engines, gateways, MCP
//! transports, companion cognition, provider implementations. Each of those has
//! an owner elsewhere; see `ARCHITECTURE.md`.
//!
//! # Why this is a submodule rather than the crate root
//!
//! `apeireth-core` presently also carries a large body of historical work —
//! memory items, philosophy gates, permission onions, a cognitive lifecycle —
//! re-exported at the crate root and depended on by 38 crates. That content does
//! not satisfy the rule above, but it cannot be evicted without breaking the
//! workspace, and doing so is tracked as a migration item rather than performed
//! here.
//!
//! Confining the canonical vocabulary to `apeireth_core::kernel` gives new code
//! an unambiguous namespace today, and keeps `pub use kernel::*` off the crate
//! root so that the legacy re-exports cannot silently shadow a primitive. When
//! the legacy content has moved to its rightful owners, this module becomes the
//! crate.
//!
//! # Determinism
//!
//! Nothing here reads the wall clock implicitly. [`Event`] and [`Timestamp`] take
//! a `&dyn Clock`, which is what lets the end-to-end runtime test assert on
//! timing without sleeping.

pub mod error;
pub mod event;
pub mod ids;
pub mod lifecycle;
pub mod metadata;
pub mod time;

pub use error::{CoreError, CoreResult};
pub use event::{Event, Topic};
pub use ids::{CapabilityId, ModelId, PluginId, RequestId, SessionId, TaskId, TraceId};
pub use lifecycle::Lifecycle;
pub use metadata::Metadata;
pub use time::{system_clock, Clock, SystemClock, Timestamp, VirtualClock};

#[cfg(test)]
mod tests {
    use super::*;

    /// The primitives have to compose without any subsystem's help; if this stops
    /// being true, something layered has leaked into core.
    #[test]
    fn the_primitives_compose_into_a_correlated_record() {
        let clock = VirtualClock::new(
            Timestamp::from_epoch_millis(1_700_000_000_000)
                .unwrap()
                .as_datetime(),
        );

        let session = SessionId::new();
        let trace = TraceId::new();
        let plugin = PluginId::new("builtin.calculator").unwrap();
        let capability = CapabilityId::new("tool.calculator").unwrap();
        let model = ModelId::new("fake-model-1").unwrap();

        let state = Lifecycle::Registered
            .transition_to(plugin.as_str(), Lifecycle::Initializing)
            .unwrap()
            .transition_to(plugin.as_str(), Lifecycle::Active)
            .unwrap();
        assert!(state.is_dispatchable());

        let event = Event::new(
            Topic::new("runtime.capability.dispatched").unwrap(),
            trace,
            &clock,
            serde_json::json!({
                "session": session.to_string(),
                "capability": capability.as_str(),
                "model": model.as_str(),
            }),
        )
        .with_metadata("plugin", plugin.as_str());

        assert_eq!(event.trace, trace);
        assert_eq!(event.metadata.get("plugin"), Some("builtin.calculator"));
        assert_eq!(capability.kind_segment(), "tool");
    }
}
