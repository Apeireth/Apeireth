//! Architecture freeze guard for the canonical gateway entry.
//!
//! The semantic proof is `canonical_entry_e2e.rs`: an Axum request travels
//! through the real router and lands in `Runtime::execute`. This guard keeps the
//! adapter source itself from drifting back into orchestration.

const CANONICAL_ENTRY: &str = include_str!("../src/canonical_entry.rs");

#[test]
fn canonical_gateway_entry_delegates_semantics_to_runtime_execute() {
    assert!(
        CANONICAL_ENTRY.contains("runtime.execute_outcome("),
        "the production gateway chat path must enter Runtime::execute_outcome"
    );
    assert!(
        CANONICAL_ENTRY.contains("pub async fn execute_chat"),
        "the transport-neutral gateway entry adapter must exist"
    );
}

#[test]
fn canonical_gateway_entry_does_not_reach_around_the_runtime() {
    for forbidden in [
        "ProviderCapability",
        "ToolCapability",
        "ToolRegistry",
        "SessionManager",
        "PluginManager",
        "ProviderRouter",
        "ToolResult",
        "ProcessExecutor",
        "ProcessRequest",
        "ProcessLimits",
    ] {
        assert!(
            !CANONICAL_ENTRY.contains(forbidden),
            "canonical_entry must stay a transport adapter and not import {forbidden:?}"
        );
    }
}
